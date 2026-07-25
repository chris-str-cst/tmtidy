use anyhow::{Context, Result};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Per-log size cap. Beyond this, the log is rotated to `<path>.1`.
pub const MAX_LOG_BYTES: u64 = 1_048_576; // 1 MiB

pub fn logfile_path() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".local/state/tmtidy/tmtidy.log")
}

fn rotated_path(path: &Path) -> PathBuf {
    let mut s = path.to_path_buf().into_os_string();
    s.push(".1");
    PathBuf::from(s)
}

/// If `path` is larger than `max_bytes`, rename it to `<path>.1` (replacing any
/// prior rotation) so the next write starts fresh. Missing file is a no-op.
/// Keeps at most two generations (~2×cap on disk).
pub fn rotate_if_needed(path: &Path, max_bytes: u64) -> Result<()> {
    let len = match fs::metadata(path) {
        Ok(m) => m.len(),
        Err(_) => return Ok(()),
    };
    if len <= max_bytes {
        return Ok(());
    }
    fs::rename(path, rotated_path(path))
        .with_context(|| format!("rotating log {}", path.display()))?;
    Ok(())
}

pub fn append_run(record: &serde_json::Value) -> Result<()> {
    append_run_to(&logfile_path(), record)
}

pub fn append_run_to(path: &Path, record: &serde_json::Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating log dir {}", parent.display()))?;
    }
    rotate_if_needed(path, MAX_LOG_BYTES)?;
    let mut f = OpenOptions::new().create(true).append(true).open(path)
        .with_context(|| format!("opening log {}", path.display()))?;
    let line = serde_json::to_string(record)?;
    writeln!(f, "{}", line)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn append_writes_one_json_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub/tmtidy.log");
        append_run_to(&path, &json!({"a": 1})).unwrap();
        append_run_to(&path, &json!({"a": 2})).unwrap();
        let contents = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("\"a\":1"));
    }

    fn dot1(path: &Path) -> PathBuf {
        let mut s = path.to_path_buf().into_os_string();
        s.push(".1");
        PathBuf::from(s)
    }

    #[test]
    fn rotate_noop_when_missing_or_small() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.log");
        // Missing file: no error, nothing created.
        rotate_if_needed(&path, 100).unwrap();
        assert!(!path.exists());
        // Under cap: left in place, no rotation.
        std::fs::write(&path, vec![b'x'; 50]).unwrap();
        rotate_if_needed(&path, 100).unwrap();
        assert!(path.exists());
        assert!(!dot1(&path).exists());
    }

    #[test]
    fn rotate_moves_oversize_to_dot1() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.log");
        std::fs::write(&path, vec![b'x'; 200]).unwrap();
        rotate_if_needed(&path, 100).unwrap();
        // Original path is now gone (fresh slot); old content moved to .1.
        assert!(!path.exists());
        assert_eq!(std::fs::metadata(dot1(&path)).unwrap().len(), 200);
    }

    #[test]
    fn append_rotates_then_writes_fresh() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.log");
        std::fs::write(&path, vec![b'x'; MAX_LOG_BYTES as usize + 1]).unwrap();
        append_run_to(&path, &json!({"a": 1})).unwrap();
        // Oversize content rotated away; new log holds only the fresh line.
        assert!(dot1(&path).exists());
        let fresh = std::fs::read_to_string(&path).unwrap();
        assert_eq!(fresh.lines().count(), 1);
        assert!(fresh.contains("\"a\":1"));
    }
}
