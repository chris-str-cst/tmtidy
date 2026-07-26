use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::default_config_path;
use crate::logging::logfile_path;

/// The starter config written by `tmtidy init` — the documented example,
/// baked into the binary so `init` needs no external files.
const EXAMPLE_CONFIG: &str = include_str!("../config.example.yaml");

/// `~/.local/state/tmtidy` — where run/scan logs live. Derived from the log
/// file path so the two never drift apart.
fn state_dir() -> PathBuf {
    logfile_path()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default()
}

/// Write `contents` to `path`, creating parent dirs. Returns `true` if written,
/// `false` if the file already existed and `force` is not set (left untouched).
fn write_config(path: &Path, contents: &str, force: bool) -> Result<bool> {
    if path.exists() && !force {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating config dir {}", parent.display()))?;
    }
    fs::write(path, contents).with_context(|| format!("writing config {}", path.display()))?;
    Ok(true)
}

/// Create the config dir + starter config and the state dir, then report the
/// resolved paths. Existing config is left untouched unless `force`.
pub fn init(force: bool) -> Result<()> {
    let cfg = default_config_path();
    let state = state_dir();

    let written = write_config(&cfg, EXAMPLE_CONFIG, force)?;
    fs::create_dir_all(&state)
        .with_context(|| format!("creating state dir {}", state.display()))?;

    println!("tmtidy init:");
    println!(
        "  config: {}  {}",
        cfg.display(),
        if written {
            "(created)"
        } else {
            "(exists — skipped; use --force to overwrite)"
        }
    );
    println!("  state:  {}  (ready)", state.display());
    println!("  logs:   tmtidy.log, scan.log (created on first run)");
    if written {
        println!("\nEdit the `roots:` list, then run `tmtidy scan --dry-run`.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_config_when_missing_and_creates_parent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/config.yaml");
        let wrote = write_config(&path, "hello", false).unwrap();
        assert!(wrote);
        assert_eq!(fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn skips_existing_without_force() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        fs::write(&path, "original").unwrap();
        let wrote = write_config(&path, "new", false).unwrap();
        assert!(!wrote);
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
    }

    #[test]
    fn overwrites_existing_with_force() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        fs::write(&path, "original").unwrap();
        let wrote = write_config(&path, "new", true).unwrap();
        assert!(wrote);
        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
    }

    #[test]
    fn baked_example_config_is_valid_and_nonempty() {
        assert!(EXAMPLE_CONFIG.contains("roots:"));
        // Must parse as the real config, so `init` never writes a broken file.
        crate::config::Config::from_yaml_str(EXAMPLE_CONFIG).unwrap();
    }
}
