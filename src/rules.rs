use crate::config::Rule;
use std::path::{Path, PathBuf};

pub struct Match {
    pub path: PathBuf,
    pub rule: String,
}

pub fn match_dir(dir: &Path, rules: &[Rule]) -> Vec<Match> {
    let mut out = Vec::new();
    for rule in rules {
        let has_marker = rule.markers.iter().any(|m| dir.join(m).is_file());
        if !has_marker {
            continue;
        }
        for target in &rule.targets {
            let p = dir.join(target);
            if p.is_dir() {
                out.push(Match { path: p, rule: rule.name.clone() });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Rule;
    use std::fs;
    use tempfile::tempdir;

    fn rule(name: &str, markers: &[&str], targets: &[&str]) -> Rule {
        Rule { name: name.into(),
            markers: markers.iter().map(|s| s.to_string()).collect(),
            targets: targets.iter().map(|s| s.to_string()).collect() }
    }

    #[test]
    fn matches_when_marker_present_and_target_dir_exists() {
        let d = tempdir().unwrap();
        fs::write(d.path().join("Cargo.toml"), "").unwrap();
        fs::create_dir(d.path().join("target")).unwrap();
        let rules = vec![rule("rust", &["Cargo.toml"], &["target"])];
        let m = match_dir(d.path(), &rules);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].path, d.path().join("target"));
        assert_eq!(m[0].rule, "rust");
    }

    #[test]
    fn no_match_without_marker() {
        let d = tempdir().unwrap();
        fs::create_dir(d.path().join("target")).unwrap(); // target but no Cargo.toml
        let rules = vec![rule("rust", &["Cargo.toml"], &["target"])];
        assert!(match_dir(d.path(), &rules).is_empty());
    }

    #[test]
    fn no_match_when_target_missing_or_is_file() {
        let d = tempdir().unwrap();
        fs::write(d.path().join("Cargo.toml"), "").unwrap();
        fs::write(d.path().join("target"), "").unwrap(); // a FILE named target
        let rules = vec![rule("rust", &["Cargo.toml"], &["target"])];
        assert!(match_dir(d.path(), &rules).is_empty());
    }
}
