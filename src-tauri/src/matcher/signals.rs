pub fn bpm_signal(bpm_a: Option<f64>, bpm_b: Option<f64>) -> f32 {
    match (bpm_a, bpm_b) {
        (Some(a), Some(b)) => {
            let diff = (a - b).abs();
            if diff <= 1.0 {
                0.15
            } else if diff > 5.0 {
                -0.10
            } else {
                0.0
            }
        }
        _ => 0.0,
    }
}

pub fn temporal_signal(mtime_a: i64, mtime_b: i64) -> f32 {
    let diff_secs = (mtime_a - mtime_b).unsigned_abs();
    let three_days = 3 * 86400_u64;
    let fourteen_days = 14 * 86400_u64;

    if diff_secs <= three_days {
        0.10
    } else if diff_secs <= fourteen_days {
        0.05
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bpm_same() {
        assert_eq!(bpm_signal(Some(128.0), Some(128.0)), 0.15);
    }

    #[test]
    fn test_bpm_close() {
        assert_eq!(bpm_signal(Some(128.0), Some(128.5)), 0.15);
    }

    #[test]
    fn test_bpm_different() {
        assert_eq!(bpm_signal(Some(128.0), Some(140.0)), -0.10);
    }

    #[test]
    fn test_bpm_null() {
        assert_eq!(bpm_signal(None, Some(128.0)), 0.0);
        assert_eq!(bpm_signal(Some(128.0), None), 0.0);
    }

    #[test]
    fn test_temporal_same_day() {
        let t = 1700000000_i64;
        assert_eq!(temporal_signal(t, t + 3600), 0.10);
    }

    #[test]
    fn test_temporal_week_apart() {
        let t = 1700000000_i64;
        assert_eq!(temporal_signal(t, t + 7 * 86400), 0.05);
    }

    #[test]
    fn test_temporal_month_apart() {
        let t = 1700000000_i64;
        assert_eq!(temporal_signal(t, t + 60 * 86400), 0.0);
    }
}
