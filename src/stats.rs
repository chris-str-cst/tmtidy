use serde::Serialize;

#[derive(Debug, Default, Serialize)]
pub struct ScanStats {
    pub excluded_new: u64,
    pub skipped_existing: u64,
    pub errors: u64,
}

#[derive(Debug, Default, Serialize)]
pub struct DecayStats {
    pub candidates: u64,
    pub trashed: u64,
    pub reclaimed_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_stats_default_is_zero_and_serializes() {
        let s = ScanStats::default();
        assert_eq!(s.excluded_new, 0);
        let j = serde_json::to_value(&s).unwrap();
        assert_eq!(j["skipped_existing"], 0);
    }
}
