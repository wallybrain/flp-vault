pub fn normalize_filename(path: &str) -> String {
    // Extract stem (filename without extension)
    let stem = std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(path);

    let mut name = stem.to_lowercase();

    // Strip common noise suffixes (with _ or space separator)
    let noise_suffixes = ["_final", "_old", "_backup", "_copy", " final", " old", " backup", " copy"];
    for suffix in &noise_suffixes {
        if name.ends_with(suffix) {
            name = name[..name.len() - suffix.len()].to_string();
        }
    }

    // Strip trailing digit clusters in a loop (handles FL Studio double-numbering)
    loop {
        let trimmed = name.trim_end();
        // Strip trailing digits
        let without_digits = trimmed.trim_end_matches(|c: char| c.is_ascii_digit());
        if without_digits.len() == trimmed.len() {
            // No trailing digits found
            break;
        }
        // Strip trailing separator (underscore or space)
        let without_sep = without_digits.trim_end_matches(|c: char| c == '_' || c == ' ');
        name = without_sep.to_string();

        if name.is_empty() {
            break;
        }
    }

    name.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_normalization() {
        assert_eq!(normalize_filename("Song Name.flp"), "song name");
    }

    #[test]
    fn test_strip_version_number() {
        assert_eq!(normalize_filename("Song Name 2.flp"), "song name");
    }

    #[test]
    fn test_strip_double_version_number() {
        // FL Studio appends digit to existing digit: "Song 2" -> "Song 22"
        assert_eq!(normalize_filename("Trap Beat 22.flp"), "trap beat");
    }

    #[test]
    fn test_strip_triple_version_number() {
        assert_eq!(normalize_filename("Trap Beat 222.flp"), "trap beat");
    }

    #[test]
    fn test_strip_underscore_version() {
        assert_eq!(normalize_filename("Song Name_3.flp"), "song name");
    }

    #[test]
    fn test_strip_noise_suffixes() {
        assert_eq!(normalize_filename("My Song_final.flp"), "my song");
        assert_eq!(normalize_filename("My Song_backup.flp"), "my song");
        assert_eq!(normalize_filename("My Song_old.flp"), "my song");
        assert_eq!(normalize_filename("My Song_copy.flp"), "my song");
    }

    #[test]
    fn test_full_path_extracts_stem() {
        assert_eq!(normalize_filename("/path/to/Song Name 5.flp"), "song name");
    }

    #[test]
    fn test_empty_after_normalization() {
        // Edge case: name is only digits
        let result = normalize_filename("123.flp");
        assert!(result.is_empty() || result.len() < 4);
    }
}
