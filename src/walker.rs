use crate::config::Root;
use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::PathBuf;
use walkdir::WalkDir;

/// Dirs found, plus paths we couldn't read due to EPERM. On macOS a
/// PermissionDenied on a dir you own signals TCC (Full Disk Access) gating —
/// e.g. ~/Documents, ~/Desktop. Callers surface `denied` as an FDA hint.
#[derive(Default)]
pub struct WalkOutcome {
    pub dirs: Vec<PathBuf>,
    pub denied: Vec<PathBuf>,
}

pub fn walk_root(
    root: &Root,
    prune_names: &HashSet<String>,
    ignore: &HashSet<PathBuf>,
) -> WalkOutcome {
    let mut out = WalkOutcome::default();
    // walkdir's filter_entry semantics: when the predicate returns false for a
    // dir entry, that entry is NOT yielded, and it is NOT descended into
    // (walkdir calls skip_current_dir internally). So pruned dirs (e.g.
    // `node_modules`) and ignored paths never appear in `out` at all, and
    // nothing under them is visited. This is fine for consumers: rules::match_dir
    // inspects the PARENT dir for a marker (e.g. proj containing a
    // node_modules subdir), and the parent is normally not itself a
    // pruned/ignored name, so it is yielded regardless of what's pruned below
    // it. Known limitation: if a real directory is itself named like a
    // target (e.g. a project literally named `build` or `dist`), it gets
    // pruned and is never inspected for markers either. Rare layout;
    // acceptable for v1.
    let walker = WalkDir::new(&root.path)
        .max_depth(root.max_depth)
        .into_iter()
        .filter_entry(|e| {
            // Don't descend into ignored paths.
            if ignore.contains(e.path()) {
                return false;
            }
            // Don't descend into a pruned target dir's subtree.
            if e.depth() > 0 {
                if let Some(name) = e.file_name().to_str() {
                    if prune_names.contains(name) {
                        return false;
                    }
                }
            }
            true
        });
    for result in walker {
        match result {
            // No `!ignore.contains(...)` re-check here: filter_entry already
            // guarantees every yielded entry's path is absent from `ignore`.
            Ok(entry) => {
                if entry.file_type().is_dir() {
                    out.dirs.push(entry.path().to_path_buf());
                }
            }
            Err(err) => {
                // EPERM reading a dir we own => TCC/Full Disk Access denial.
                // Anything else (a race delete, etc.) is swallowed as before.
                let denied = err
                    .io_error()
                    .map(|e| e.kind() == ErrorKind::PermissionDenied)
                    .unwrap_or(false);
                if denied {
                    if let Some(p) = err.path() {
                        out.denied.push(p.to_path_buf());
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Root;
    use std::collections::HashSet;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn yields_dirs_but_prunes_target_and_ignore() {
        let d = tempdir().unwrap();
        let base = d.path();
        fs::create_dir_all(base.join("proj/node_modules/deep")).unwrap();
        fs::create_dir_all(base.join("proj/src")).unwrap();
        fs::create_dir_all(base.join("skip/inner")).unwrap();

        let root = Root { path: base.to_path_buf(), max_depth: 8 };
        let prune: HashSet<String> = ["node_modules".to_string()].into_iter().collect();
        let ignore: HashSet<PathBuf> = [base.join("skip")].into_iter().collect();

        let dirs = walk_root(&root, &prune, &ignore).dirs;

        // Load-bearing: the parent dir that holds the marker is yielded.
        // rules::match_dir(dir) inspects `dir` for a marker and, on match,
        // checks `dir.join(target)` — so `proj` must appear for node_modules
        // pruning to be detectable at all.
        assert!(dirs.contains(&base.join("proj")));
        assert!(dirs.contains(&base.join("proj/src")));

        // Nothing under the pruned dir is descended into.
        assert!(!dirs.contains(&base.join("proj/node_modules/deep")));
        // node_modules itself is NOT yielded either: walkdir's filter_entry
        // returning false skips yielding the entry and skips descending into
        // it. Irrelevant to consumers, since match_dir inspects the parent
        // (`proj`), never the target dir itself.
        assert!(!dirs.contains(&base.join("proj/node_modules")));

        // Ignored subtree fully absent.
        assert!(!dirs.contains(&base.join("skip")));
        assert!(!dirs.contains(&base.join("skip/inner")));
    }

    #[test]
    #[cfg(unix)]
    fn unreadable_dir_is_reported_as_denied() {
        use std::os::unix::fs::PermissionsExt;
        let d = tempdir().unwrap();
        let base = d.path();
        let locked = base.join("locked");
        fs::create_dir(&locked).unwrap();
        // chmod 000: owner (non-root) can no longer list it -> EPERM/EACCES,
        // which walkdir surfaces as ErrorKind::PermissionDenied. Same kind TCC
        // raises on a protected dir without Full Disk Access.
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();

        let root = Root { path: base.to_path_buf(), max_depth: 8 };
        let out = walk_root(&root, &HashSet::new(), &HashSet::new());

        // Restore perms so tempdir cleanup can remove it.
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();

        assert!(
            out.denied.iter().any(|p| p == &locked),
            "expected {:?} in denied, got {:?}",
            locked,
            out.denied
        );
    }
}
