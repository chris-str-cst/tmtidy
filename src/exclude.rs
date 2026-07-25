use anyhow::{bail, Result};
use std::path::Path;
use std::process::Command;

pub const EXCLUDE_XATTR: &str = "com.apple.metadata:com_apple_backup_excludeItem";

pub fn is_excluded(path: &Path) -> bool {
    match xattr::get(path, EXCLUDE_XATTR) {
        Ok(Some(_)) => true,
        _ => false,
    }
}

fn run_tmutil(verb: &str, path: &Path) -> Result<()> {
    let status = Command::new("tmutil").arg(verb).arg(path).status();
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => bail!("tmutil {} {} exited with {}", verb, path.display(), s),
        Err(e) => bail!("failed to run tmutil {}: {}", verb, e),
    }
}

pub fn add_exclusion(path: &Path) -> Result<()> {
    run_tmutil("addexclusion", path)
}

// Exercised by tests; reserved as public API for a future unexclude path.
#[allow(dead_code)]
pub fn remove_exclusion(path: &Path) -> Result<()> {
    run_tmutil("removeexclusion", path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn not_excluded_by_default() {
        let d = tempdir().unwrap();
        assert!(!is_excluded(d.path()));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn add_then_detect_then_remove_roundtrip() {
        // Skip if tmutil is unavailable (e.g. sandboxed CI).
        if std::process::Command::new("tmutil").arg("version").output().is_err() {
            eprintln!("skipping: tmutil unavailable");
            return;
        }
        let d = tempdir().unwrap();
        add_exclusion(d.path()).unwrap();
        assert!(is_excluded(d.path()));
        remove_exclusion(d.path()).unwrap();
        assert!(!is_excluded(d.path()));
    }
}
