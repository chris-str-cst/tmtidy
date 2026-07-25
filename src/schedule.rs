use anyhow::{bail, Context, Result};
use std::path::Path;

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
    }

    #[test]
    fn parse_every_rejects_bad_input() {
        for bad in ["", "0", "5x", "h", "-1h", "1h30m", "  "] {
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
