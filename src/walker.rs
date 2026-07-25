use crate::config::Root;
use std::collections::HashSet;
use std::path::PathBuf;
use walkdir::WalkDir;

pub fn walk_root(
    root: &Root,
    prune_names: &HashSet<String>,
    ignore: &HashSet<PathBuf>,
) -> Vec<PathBuf> {
    let mut out = Vec::new();
    // walkdir's filter_entry semantics: when the predicate returns false for a
    // dir entry, that entry is NOT yielded, and it is NOT descended into
    // (walkdir calls skip_current_dir internally). So pruned dirs (e.g.
    // `node_modules`) and ignored paths never appear in `out` at all, and
    // nothing under them is visited. This is fine for consumers: rules::match_dir
    // inspects the PARENT dir for a marker (e.g. proj containing a
    // node_modules subdir), and the parent is never itself a pruned/ignored
    // name, so it is always yielded regardless of what's pruned below it.
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
    for entry in walker.flatten() {
        // No `!ignore.contains(...)` re-check here: filter_entry already
        // guarantees every yielded entry's path is absent from `ignore`.
        if entry.file_type().is_dir() {
            out.push(entry.path().to_path_buf());
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

        let dirs = walk_root(&root, &prune, &ignore);

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
}
