use anyhow::{Context, Result};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn logfile_path() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".local/state/tmtidy/tmtidy.log")
}

pub fn append_run(record: &serde_json::Value) -> Result<()> {
    append_run_to(&logfile_path(), record)
}

pub fn append_run_to(path: &Path, record: &serde_json::Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating log dir {}", parent.display()))?;
    }
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
}
