use crate::matcher::signals::{bpm_signal, temporal_signal};

pub fn compute_confidence(
    norm_a: &str,
    norm_b: &str,
    bpm_a: Option<f64>,
    bpm_b: Option<f64>,
    mtime_a: i64,
    mtime_b: i64,
) -> f32 {
    let trigram_score = if norm_a.len() < 4 || norm_b.len() < 4 {
        // Short name: require exact match
        if norm_a == norm_b {
            1.0_f32
        } else {
            0.0_f32
        }
    } else {
        trigram::similarity(norm_a, norm_b) as f32
    };

    let score = trigram_score + bpm_signal(bpm_a, bpm_b) + temporal_signal(mtime_a, mtime_b);
    score.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_names_high_confidence() {
        let score = compute_confidence(
            "acid bass line",
            "acid bass line",
            Some(128.0),
            Some(128.0),
            1700000000,
            1700000000,
        );
        assert!(score > 0.9);
    }

    #[test]
    fn test_different_names_low_confidence() {
        let score = compute_confidence(
            "acid bass line",
            "funky groove master",
            Some(128.0),
            Some(90.0),
            1700000000,
            1700000000,
        );
        assert!(score < 0.4);
    }

    #[test]
    fn test_short_name_exact_match() {
        let score = compute_confidence("hi", "hi", Some(128.0), Some(128.0), 1700000000, 1700000000);
        assert!(score > 0.9);
    }

    #[test]
    fn test_short_name_no_match() {
        let score = compute_confidence("hi", "ho", None, None, 1700000000, 1700000000);
        assert!(score < 0.3);
    }

    #[test]
    fn test_confidence_clamped_to_one() {
        // Even with all boosts, should not exceed 1.0
        let score = compute_confidence(
            "test name",
            "test name",
            Some(128.0),
            Some(128.0),
            1700000000,
            1700000000,
        );
        assert!(score <= 1.0);
    }
}
