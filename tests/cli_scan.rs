// End-to-end: build a temp tree + config, run `tmtidy scan`, assert exit 0
// and (on macOS with tmutil) the target dir becomes excluded.
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn scan_excludes_target_dir() {
    let d = tempdir().unwrap();
    let proj = d.path().join("proj");
    fs::create_dir_all(proj.join("target")).unwrap();
    fs::write(proj.join("Cargo.toml"), "").unwrap();

    let cfg_path = d.path().join("config.yaml");
    fs::write(&cfg_path, format!(
        "roots:\n  - path: {}\n", d.path().display()
    )).unwrap();

    let bin = env!("CARGO_BIN_EXE_tmtidy");
    let out = Command::new(bin)
        .args(["--config", cfg_path.to_str().unwrap(), "scan"])
        .output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));

    #[cfg(target_os = "macos")]
    if Command::new("tmutil").arg("version").output().is_ok() {
        assert!(xattr::get(proj.join("target"), tmtidy_xattr()).unwrap().is_some());
    }
}

#[cfg(target_os = "macos")]
fn tmtidy_xattr() -> &'static str { "com.apple.metadata:com_apple_backup_excludeItem" }
