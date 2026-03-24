use anyhow::{Result, bail};

use crate::model::{
    DEFAULT_BATCH_SIZE, DEFAULT_MAX_CHECKS, DEFAULT_REQUEST_INTERVAL_MS, DEFAULT_RESULTS,
    MAX_NAME_LENGTH, MIN_NAME_LENGTH, SearchOptions,
};

pub const ALLOWED_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_";

pub fn is_valid_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

pub fn is_valid_name(name: &str) -> bool {
    let len = name.chars().count();
    (usize::from(MIN_NAME_LENGTH)..=usize::from(MAX_NAME_LENGTH)).contains(&len)
        && name.chars().all(is_valid_name_char)
}

pub fn validate_prefix(prefix: &str) -> Result<()> {
    if !prefix.chars().all(is_valid_name_char) {
        bail!("prefix may only contain A-Z, a-z, 0-9, and _");
    }

    Ok(())
}

pub fn validate_search_options(
    length: u8,
    prefix: &str,
    results: usize,
    max_checks: usize,
) -> Result<SearchOptions> {
    if !(MIN_NAME_LENGTH..=MAX_NAME_LENGTH).contains(&length) {
        bail!(
            "length must be between {} and {}",
            MIN_NAME_LENGTH,
            MAX_NAME_LENGTH
        );
    }

    validate_prefix(prefix)?;

    let prefix_len = prefix.chars().count();
    if prefix_len > usize::from(length) {
        bail!("prefix cannot be longer than the target length");
    }

    if results == 0 {
        bail!("results must be at least 1");
    }

    if max_checks == 0 {
        bail!("max-checks must be at least 1");
    }

    Ok(SearchOptions {
        length,
        prefix: prefix.to_string(),
        results,
        max_checks,
        batch_size: DEFAULT_BATCH_SIZE.min(max_checks),
        request_interval: std::time::Duration::from_millis(DEFAULT_REQUEST_INTERVAL_MS),
    })
}

pub fn default_search_options() -> SearchOptions {
    SearchOptions {
        length: 4,
        prefix: String::new(),
        results: DEFAULT_RESULTS,
        max_checks: DEFAULT_MAX_CHECKS,
        batch_size: DEFAULT_BATCH_SIZE,
        request_interval: std::time::Duration::from_millis(DEFAULT_REQUEST_INTERVAL_MS),
    }
}

#[cfg(test)]
mod tests {
    use super::{default_search_options, is_valid_name, validate_search_options};

    #[test]
    fn accepts_basic_search_options() {
        let options = validate_search_options(4, "e", 5, 20).expect("valid options");
        assert_eq!(options.length, 4);
        assert_eq!(options.prefix, "e");
    }

    #[test]
    fn rejects_too_long_prefix() {
        let error = validate_search_options(4, "hello", 5, 20).expect_err("invalid prefix");
        assert!(error.to_string().contains("prefix cannot be longer"));
    }

    #[test]
    fn validates_names() {
        assert!(is_valid_name("e123"));
        assert!(!is_valid_name("ab"));
        assert!(!is_valid_name("abcdefghijk"));
        assert!(!is_valid_name("bad-name"));
    }

    #[test]
    fn exposes_defaults() {
        let defaults = default_search_options();
        assert_eq!(defaults.length, 4);
        assert_eq!(defaults.results, 20);
    }
}
