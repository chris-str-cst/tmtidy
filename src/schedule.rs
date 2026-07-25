use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Parse a single-unit duration (`<int><s|m|h|d>`) into seconds.
/// Rejects empty, zero, missing number/unit, unknown unit, and multi-unit input.
pub fn parse_every(s: &str) -> Result<u64> {
    let s = s.trim();
    if s.is_empty() {
        bail!("--every: empty value");
    }
    let split = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    let (num, unit) = s.split_at(split);
    if num.is_empty() {
        bail!("--every: missing number in {:?}", s);
    }
    let n: u64 = num.parse().with_context(|| format!("--every: bad number in {:?}", s))?;
    let mult = match unit {
        "s" => 1,
        "m" => 60,
        "h" => 3600,
        "d" => 86400,
        "" => bail!("--every: missing unit (use s/m/h/d) in {:?}", s),
        other => bail!("--every: unknown unit {:?} (use s/m/h/d)", other),
    };
    let secs = n.checked_mul(mult).context("--every: value too large")?;
    if secs == 0 {
        bail!("--every must be greater than 0");
    }
    if secs < 60 {
        bail!("--every must be at least 60s (1 minute)");
    }
    Ok(secs)
}

pub const LABEL: &str = "com.tmtidy.scan";

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Render the LaunchAgent plist XML. Pure — no filesystem access.
pub fn render_plist(exe: &Path, config: Option<&Path>, interval_secs: u64, log: &Path) -> String {
    let mut args = format!(
        "\t\t<string>{}</string>\n\t\t<string>scan</string>\n",
        xml_escape(&exe.display().to_string())
    );
    if let Some(cfg) = config {
        args.push_str(&format!(
            "\t\t<string>--config</string>\n\t\t<string>{}</string>\n",
            xml_escape(&cfg.display().to_string())
        ));
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Label</key>
	<string>{label}</string>
	<key>ProgramArguments</key>
	<array>
{args}	</array>
	<key>StartInterval</key>
	<integer>{interval}</integer>
	<key>RunAtLoad</key>
	<true/>
	<key>StandardOutPath</key>
	<string>{log}</string>
	<key>StandardErrorPath</key>
	<string>{log}</string>
	<key>ProcessType</key>
	<string>Background</string>
	<key>LowPriorityIO</key>
	<true/>
	<key>Nice</key>
	<integer>5</integer>
</dict>
</plist>
"#,
        label = LABEL,
        args = args,
        interval = interval_secs,
        log = xml_escape(&log.display().to_string()),
    )
}

pub fn plist_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join("Library/LaunchAgents")
        .join(format!("{}.plist", LABEL))
}

pub fn scan_log_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".local/state/tmtidy/scan.log")
}

fn ensure_macos() -> Result<()> {
    if cfg!(target_os = "macos") {
        Ok(())
    } else {
        bail!("scheduling is macOS-only (uses launchd)")
    }
}

fn uid() -> Result<String> {
    let out = Command::new("id").arg("-u").output().context("running `id -u`")?;
    if !out.status.success() {
        bail!("`id -u` failed");
    }
    Ok(String::from_utf8(out.stdout).context("parsing uid")?.trim().to_string())
}

fn domain_target() -> Result<String> {
    Ok(format!("gui/{}", uid()?))
}

fn is_loaded() -> bool {
    let target = match domain_target() {
        Ok(t) => t,
        Err(_) => return false,
    };
    Command::new("launchctl")
        .arg("print")
        .arg(format!("{}/{}", target, LABEL))
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn bootstrap(plist: &Path) -> Result<()> {
    let target = domain_target()?;
    let out = Command::new("launchctl")
        .arg("bootstrap")
        .arg(&target)
        .arg(plist)
        .output()
        .context("launchctl bootstrap")?;
    if !out.status.success() {
        bail!(
            "launchctl bootstrap failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

/// Unload the agent. A not-loaded agent is treated as benign success.
fn bootout() -> Result<()> {
    let target = domain_target()?;
    let out = Command::new("launchctl")
        .arg("bootout")
        .arg(format!("{}/{}", target, LABEL))
        .output()
        .context("launchctl bootout")?;
    if out.status.success() {
        return Ok(());
    }
    let err = String::from_utf8_lossy(&out.stderr);
    // ENOENT-equivalents: agent wasn't loaded. Not an error.
    if err.contains("No such process") || err.contains("Could not find") {
        return Ok(());
    }
    bail!("launchctl bootout failed: {}", err.trim());
}

pub fn install(config: Option<&Path>, interval_secs: u64) -> Result<()> {
    ensure_macos()?;
    let exe = std::env::current_exe()
        .context("resolving current executable")?
        .canonicalize()
        .context("canonicalizing executable path")?;
    let log = scan_log_path();
    if let Some(p) = log.parent() {
        std::fs::create_dir_all(p).ok();
    }
    let plist = plist_path();
    if let Some(p) = plist.parent() {
        std::fs::create_dir_all(p).with_context(|| format!("creating {}", p.display()))?;
    }
    let xml = render_plist(&exe, config, interval_secs, &log);
    std::fs::write(&plist, xml).with_context(|| format!("writing {}", plist.display()))?;
    // Reload cleanly: drop any prior instance, then load the fresh plist.
    let _ = bootout();
    bootstrap(&plist)?;
    println!(
        "scheduled: {} runs `scan` every {}s (+ at load)",
        LABEL, interval_secs
    );
    println!("binary: {}", exe.display());
    println!("plist:  {}", plist.display());
    Ok(())
}

pub fn uninstall() -> Result<()> {
    ensure_macos()?;
    let _ = bootout();
    let plist = plist_path();
    if plist.exists() {
        std::fs::remove_file(&plist).with_context(|| format!("removing {}", plist.display()))?;
    }
    println!("unscheduled: {} removed", LABEL);
    Ok(())
}

pub fn disable() -> Result<()> {
    ensure_macos()?;
    bootout()?;
    println!("disabled: {} stopped (plist kept — run `schedule enable` to resume)", LABEL);
    Ok(())
}

pub fn enable() -> Result<()> {
    ensure_macos()?;
    let plist = plist_path();
    if !plist.exists() {
        bail!("no schedule installed; run `tmtidy schedule install` first");
    }
    let _ = bootout();
    bootstrap(&plist)?;
    println!("enabled: {} loaded", LABEL);
    Ok(())
}

/// Last `scan` record from the run log, formatted `ts stats`, if any.
fn last_scan_run() -> Option<String> {
    let content = std::fs::read_to_string(crate::logging::logfile_path()).ok()?;
    let line = content
        .lines()
        .rev()
        .find(|l| l.contains("\"command\":\"scan\""))?;
    match serde_json::from_str::<serde_json::Value>(line) {
        Ok(v) => Some(format!(
            "{} {}",
            v.get("ts").and_then(|t| t.as_str()).unwrap_or("?"),
            v.get("stats").map(|s| s.to_string()).unwrap_or_default()
        )),
        Err(_) => Some(line.to_string()),
    }
}

pub fn status() -> Result<()> {
    ensure_macos()?;
    let plist = plist_path();
    println!(
        "plist:    {} [{}]",
        plist.display(),
        if plist.exists() { "present" } else { "absent" }
    );
    println!("loaded:   {}", if is_loaded() { "yes" } else { "no" });
    println!("scan log: {}", scan_log_path().display());
    match last_scan_run() {
        Some(s) => println!("last run: {}", s),
        None => println!("last run: (none yet)"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_every_accepts_units() {
        assert_eq!(parse_every("3600s").unwrap(), 3600);
        assert_eq!(parse_every("30m").unwrap(), 1800);
        assert_eq!(parse_every("1h").unwrap(), 3600);
        assert_eq!(parse_every("24h").unwrap(), 86400);
        assert_eq!(parse_every("1d").unwrap(), 86400);
        assert_eq!(parse_every("60s").unwrap(), 60);
        assert_eq!(parse_every("1m").unwrap(), 60);
    }

    #[test]
    fn parse_every_rejects_bad_input() {
        for bad in ["", "0", "5x", "h", "-1h", "1h30m", "  ", "1s", "30s", "59s"] {
            assert!(parse_every(bad).is_err(), "expected {:?} to be rejected", bad);
        }
    }

    #[test]
    fn render_plist_has_core_fields() {
        let out = render_plist(
            Path::new("/usr/local/bin/tmtidy"),
            None,
            3600,
            Path::new("/Users/x/.local/state/tmtidy/scan.log"),
        );
        assert!(out.contains("<string>com.tmtidy.scan</string>"));
        assert!(out.contains("<string>/usr/local/bin/tmtidy</string>"));
        assert!(out.contains("<string>scan</string>"));
        assert!(out.contains("<key>StartInterval</key>"));
        assert!(out.contains("<integer>3600</integer>"));
        assert!(out.contains("<key>RunAtLoad</key>"));
        assert!(out.contains("<string>/Users/x/.local/state/tmtidy/scan.log</string>"));
        // No config passed -> no --config arg.
        assert!(!out.contains("--config"));
    }

    #[test]
    fn render_plist_includes_config_when_present() {
        let out = render_plist(
            Path::new("/usr/local/bin/tmtidy"),
            Some(Path::new("/Users/x/.config/tmtidy/config.yaml")),
            1800,
            Path::new("/tmp/scan.log"),
        );
        assert!(out.contains("<string>--config</string>"));
        assert!(out.contains("<string>/Users/x/.config/tmtidy/config.yaml</string>"));
        assert!(out.contains("<integer>1800</integer>"));
    }
}
