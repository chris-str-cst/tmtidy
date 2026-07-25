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
    let walker = WalkDir::new(&root.path)
        .max_depth(root.max_depth)
        .into_iter()
        .filter_entry(|e| {
            // Prune descent into ignored paths.
            if ignore.contains(e.path()) {
                return false;
            }
            // Prune descent into a target dir (but the entry above still records it).
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
        if entry.file_type().is_dir() && !ignore.contains(entry.path()) {
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
        assert!(dirs.contains(&base.join("proj")));
        assert!(dirs.contains(&base.join("proj/src")));
        // node_modules itself is yielded (as a candidate) but not descended into:
        assert!(!dirs.contains(&base.join("proj/node_modules/deep")));
        // ignored subtree fully skipped:
        assert!(!dirs.contains(&base.join("skip")));
        assert!(!dirs.contains(&base.join("skip/inner")));
    }
}
