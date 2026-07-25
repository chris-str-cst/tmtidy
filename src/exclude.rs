use anyhow::{Context, Result};
use std::path::Path;

pub const EXCLUDE_XATTR: &str = "com.apple.metadata:com_apple_backup_excludeItem";

// The exact value `tmutil addexclusion` (sticky/no -p) writes: a binary plist
// wrapping the string "com.apple.backupd". Setting this xattr ourselves is
// equivalent — `tmutil isexcluded` reports [Excluded] and backupd honors it at
// backup time — but instant, vs. `tmutil addexclusion` which blocks ~10s+ per
// call on some machines (backupd round-trip). Sticky = self-cleaning (goes with
// the dir on delete) and needs no sudo. See is_excluded, which reads the same key.
const EXCLUDE_PLIST: &[u8] = &[
    0x62, 0x70, 0x6C, 0x69, 0x73, 0x74, 0x30, 0x30, // bplist00
    0x5F, 0x10, 0x11, 0x63, 0x6F, 0x6D, 0x2E, 0x61, // _..com.a
    0x70, 0x70, 0x6C, 0x65, 0x2E, 0x62, 0x61, 0x63, // pple.bac
    0x6B, 0x75, 0x70, 0x64, 0x08, 0x00, 0x00, 0x00, // kupd....
    0x00, 0x00, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x1C,
];

pub fn is_excluded(path: &Path) -> bool {
    matches!(xattr::get(path, EXCLUDE_XATTR), Ok(Some(_)))
}

pub fn add_exclusion(path: &Path) -> Result<()> {
    xattr::set(path, EXCLUDE_XATTR, EXCLUDE_PLIST)
        .with_context(|| format!("setting exclude xattr on {}", path.display()))
}

// Exercised by tests; reserved as public API for a future unexclude path.
#[allow(dead_code)]
pub fn remove_exclusion(path: &Path) -> Result<()> {
    // Absent xattr = already not excluded. Check first: a missing attr raises a
    // platform-specific errno (ENOATTR/ENODATA) the xattr crate doesn't normalize.
    if !is_excluded(path) {
        return Ok(());
    }
    xattr::remove(path, EXCLUDE_XATTR)
        .with_context(|| format!("removing exclude xattr on {}", path.display()))
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

    #[test]
    fn add_then_detect_then_remove_roundtrip() {
        // Pure xattr now — no tmutil, runs anywhere the FS supports xattrs.
        let d = tempdir().unwrap();
        add_exclusion(d.path()).unwrap();
        assert!(is_excluded(d.path()));
        // Value matches exactly what `tmutil addexclusion` writes.
        assert_eq!(
            xattr::get(d.path(), EXCLUDE_XATTR).unwrap().as_deref(),
            Some(EXCLUDE_PLIST)
        );
        remove_exclusion(d.path()).unwrap();
        assert!(!is_excluded(d.path()));
        // Removing an absent xattr is a no-op success.
        remove_exclusion(d.path()).unwrap();
    }
}
