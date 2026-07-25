use anyhow::{bail, Context, Result};

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
}
