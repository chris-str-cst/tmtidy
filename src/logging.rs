use anyhow::{Context, Result};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Hard cap per log file. When a log exceeds this, its oldest entries are
/// dropped in place so it never grows past ~this size. No archive is kept.
pub const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024; // 5 MiB

pub fn logfile_path() -> PathBuf {
    crate::paths::state_home().join("tmtidy/tmtidy.log")
}

/// Keep the log at or under `max_bytes` by discarding the OLDEST entries in
/// place: read the file, retain the most recent whole lines that fit within
/// `max_bytes`, and rewrite it. No rotation/archive file is created. A missing
/// file, or one already within the cap, is a no-op.
pub fn cap_log(path: &Path, max_bytes: u64) -> Result<()> {
    let len = match fs::metadata(path) {
        Ok(m) => m.len(),
        Err(_) => return Ok(()),
    };
    if len <= max_bytes {
        return Ok(());
    }
    let data = fs::read(path).with_context(|| format!("reading log {}", path.display()))?;
    // Keep the last `max_bytes`, then advance past the next newline so the
    // retained content starts on a whole line (drops the partial oldest line).
    let start = data.len().saturating_sub(max_bytes as usize);
    let keep_from = match data[start..].iter().position(|&b| b == b'\n') {
        Some(i) => start + i + 1,
        None => start,
    };
    fs::write(path, &data[keep_from..])
        .with_context(|| format!("capping log {}", path.display()))?;
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
    cap_log(path, MAX_LOG_BYTES)?;
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
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

    /// The old rotation kept a `<path>.1`; the cap must never create one.
    fn dot1(path: &Path) -> PathBuf {
        let mut s = path.to_path_buf().into_os_string();
        s.push(".1");
        PathBuf::from(s)
    }

    #[test]
    fn cap_noop_when_missing_or_small() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.log");
        // Missing file: no error, nothing created.
        cap_log(&path, 100).unwrap();
        assert!(!path.exists());
        // Under cap: left untouched.
        fs::write(&path, vec![b'x'; 50]).unwrap();
        cap_log(&path, 100).unwrap();
        assert_eq!(fs::metadata(&path).unwrap().len(), 50);
        assert!(!dot1(&path).exists());
    }

    #[test]
    fn cap_keeps_recent_tail_and_drops_oldest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.log");
        let mut content = String::new();
        for i in 0..10 {
            content.push_str(&format!("line{}\n", i)); // 6 bytes each, 60 total
        }
        fs::write(&path, &content).unwrap();
        cap_log(&path, 18).unwrap();
        let kept = fs::read_to_string(&path).unwrap();
        assert!(fs::metadata(&path).unwrap().len() <= 18); // never exceeds cap
        assert!(kept.ends_with("line9\n")); // newest retained
        assert!(!kept.contains("line0")); // oldest dropped
        assert!(kept.starts_with("line")); // retained content starts whole
        assert!(!dot1(&path).exists()); // no archive kept
    }

    #[test]
    fn append_caps_then_writes_no_archive() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.log");
        // Seed just over the cap, then append one record.
        fs::write(&path, vec![b'x'; MAX_LOG_BYTES as usize + 10]).unwrap();
        append_run_to(&path, &json!({"a": 1})).unwrap();
        assert!(!dot1(&path).exists()); // no rotation file
        let len = fs::metadata(&path).unwrap().len();
        assert!(len <= MAX_LOG_BYTES + 64); // capped, plus the one small line
        let s = fs::read_to_string(&path).unwrap();
        assert!(s.trim_end().ends_with("{\"a\":1}")); // newest record present
    }
}
