use crate::config::Config;
use crate::exclude::is_excluded;
use crate::rules::match_dir;
use crate::walker::walk_root;
use anyhow::Result;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

#[derive(Debug, Serialize)]
pub struct Candidate {
    pub path: PathBuf,
    pub rule: String,
    pub size_bytes: u64,
    pub age_days: u64,
    pub trashed: bool,
}

pub fn dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

pub fn age_days(path: &Path, now: SystemTime) -> u64 {
    let mtime = match std::fs::metadata(path).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return 0,
    };
    match now.duration_since(mtime) {
        Ok(d) => d.as_secs() / 86_400,
        Err(_) => 0, // mtime in the future
    }
}

fn under_any(path: &Path, roots: &HashSet<PathBuf>) -> bool {
    roots.iter().any(|r| path.starts_with(r))
}

pub fn find_candidates(cfg: &Config, now: SystemTime) -> Vec<Candidate> {
    find_candidates_with(cfg, now, &is_excluded)
}

pub fn find_candidates_with(
    cfg: &Config,
    now: SystemTime,
    is_excluded_fn: &dyn Fn(&Path) -> bool,
) -> Vec<Candidate> {
    let prune = cfg.target_names();
    let ignore: HashSet<PathBuf> = cfg.ignore.iter().cloned().collect();
    let excl_paths: HashSet<PathBuf> = cfg.decay.exclude_paths.iter().cloned().collect();
    let excl_rules: HashSet<String> = cfg.decay.exclude_rules.iter().cloned().collect();
    let min_bytes = cfg.decay.min_size_mb * 1024 * 1024;

    let mut out = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();

    for root in &cfg.roots {
        for dir in walk_root(root, &prune, &ignore) {
            for m in match_dir(&dir, &cfg.rules) {
                // (a) rule not excluded
                if excl_rules.contains(&m.rule) { continue; }
                // (e) not under ignore/exclude_paths
                if under_any(&m.path, &ignore) || under_any(&m.path, &excl_paths) { continue; }
                // (b) currently excluded
                if !is_excluded_fn(&m.path) { continue; }
                // (c) old enough
                let age = age_days(&m.path, now);
                if age < cfg.decay.max_age_days { continue; }
                // (d) big enough
                let size = dir_size(&m.path);
                if size < min_bytes { continue; }
                if !seen.insert(m.path.clone()) { continue; }
                out.push(Candidate { path: m.path, rule: m.rule, size_bytes: size, age_days: age, trashed: false });
            }
        }
    }
    out
}

pub fn trash_path(path: &Path) -> Result<()> {
    trash::delete(path).map_err(|e| anyhow::anyhow!("trash {}: {}", path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DecayConfig, Root, Rule};
    use std::fs;
    use std::time::{Duration, SystemTime};
    use tempfile::tempdir;
    use filetime::{set_file_mtime, FileTime}; // add filetime as dev-dependency

    fn base_cfg(root: &std::path::Path) -> Config {
        Config {
            roots: vec![Root { path: root.to_path_buf(), max_depth: 8 }],
            rules: vec![Rule { name: "rust".into(), markers: vec!["Cargo.toml".into()], targets: vec!["target".into()] }],
            decay: DecayConfig { max_age_days: 30, min_size_mb: 0, ..Default::default() },
            ignore: vec![],
        }
    }

    fn make_project(root: &std::path::Path, name: &str) -> std::path::PathBuf {
        let proj = root.join(name);
        fs::create_dir_all(proj.join("target")).unwrap();
        fs::write(proj.join("Cargo.toml"), "").unwrap();
        fs::write(proj.join("target/artifact.bin"), vec![0u8; 2048]).unwrap();
        proj.join("target")
    }

    fn age(path: &std::path::Path, days: u64) {
        let t = SystemTime::now() - Duration::from_secs(days * 86400);
        set_file_mtime(path, FileTime::from_system_time(t)).unwrap();
    }

    #[test]
    fn selects_old_excluded_target_only() {
        let d = tempdir().unwrap();
        let cfg = base_cfg(d.path());
        let target = make_project(d.path(), "old");
        age(&target, 100);
        // Not excluded yet → no candidate (simulate is_excluded via marker file).
        // Use a test-only exclusion marker: create the xattr is unreliable in CI,
        // so this test asserts the age/rule filtering by pre-seeding is_excluded.
        // See test-support note below.
        let cands = find_candidates_with(&cfg, SystemTime::now(), &|p| p == target);
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].path, target);
        assert!(cands[0].age_days >= 99);
    }

    #[test]
    fn rejects_recent_dir() {
        let d = tempdir().unwrap();
        let cfg = base_cfg(d.path());
        let target = make_project(d.path(), "fresh");
        age(&target, 2);
        let cands = find_candidates_with(&cfg, SystemTime::now(), &|p| p == target);
        assert!(cands.is_empty());
    }

    #[test]
    fn respects_min_size_and_exclude_rules() {
        let d = tempdir().unwrap();
        let mut cfg = base_cfg(d.path());
        cfg.decay.min_size_mb = 1; // 1 MB floor; artifact is 2 KB
        let target = make_project(d.path(), "small");
        age(&target, 100);
        assert!(find_candidates_with(&cfg, SystemTime::now(), &|p| p == target).is_empty());

        cfg.decay.min_size_mb = 0;
        cfg.decay.exclude_rules = vec!["rust".into()];
        assert!(find_candidates_with(&cfg, SystemTime::now(), &|p| p == target).is_empty());
    }

    /// Spec §8 safety invariant: decay only ever trashes a dir that matches a
    /// rule's target name. An old, "excluded" subdir whose name is not any
    /// configured target must never be selected, even though it satisfies
    /// every other criterion (old + excluded). This holds because
    /// `match_dir` only yields `dir.join(target)` paths for configured
    /// targets — a non-target sibling is never considered a candidate.
    #[test]
    fn non_target_named_dir_is_never_a_candidate_even_if_old_and_excluded() {
        let d = tempdir().unwrap();
        let cfg = base_cfg(d.path()); // rule "rust": Cargo.toml -> target
        let proj = d.path().join("proj");
        let not_a_target = proj.join("notatarget");
        fs::create_dir_all(&not_a_target).unwrap();
        fs::write(proj.join("Cargo.toml"), "").unwrap();
        fs::write(not_a_target.join("artifact.bin"), vec![0u8; 2048]).unwrap();
        age(&not_a_target, 100);

        // is_excluded_fn unconditionally reports this dir as excluded, to
        // isolate the assertion to the target-matching invariant.
        let cands = find_candidates_with(&cfg, SystemTime::now(), &|p| p == not_a_target);
        assert!(cands.is_empty());
    }
}
