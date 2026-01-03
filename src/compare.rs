use std::cmp::Ordering;

use crate::config::Config;
use crate::key::extract_key;

/// Main comparison function implementing GNU sort semantics
///
/// 1. Compare by keys (or whole line if no keys specified)
/// 2. If keys equal and last-resort enabled, compare whole line bytewise
/// 3. Last-resort ignores ALL options except -r
pub fn compare_records(a: &[u8], b: &[u8], config: &Config) -> Ordering {
    // Step 1: Compare by keys
    let key_result = compare_by_keys(a, b, config);

    if key_result != Ordering::Equal {
        return maybe_reverse(key_result, config.reverse);
    }

    // Step 2: Last-resort comparison (if enabled)
    // CRITICAL: Last-resort ignores ALL options except -r
    if config.use_last_resort() {
        let last_resort = compare_bytes_raw(a, b);
        return maybe_reverse(last_resort, config.reverse);
    }

    // Keys equal and no last-resort: preserve input order (stable sort handles this)
    Ordering::Equal
}

/// Compare by key specifications
fn compare_by_keys(a: &[u8], b: &[u8], config: &Config) -> Ordering {
    if config.keys.is_empty() {
        // No -k: compare entire line with options
        return compare_with_options(a, b, config);
    }

    for key_spec in &config.keys {
        let key_a = extract_key(a, key_spec, config.field_separator);
        let key_b = extract_key(b, key_spec, config.field_separator);

        let result = compare_with_options(&key_a, &key_b, config);
        if result != Ordering::Equal {
            return result;
        }
    }

    Ordering::Equal
}

/// Compare with -n, -f options applied
fn compare_with_options(a: &[u8], b: &[u8], config: &Config) -> Ordering {
    if config.numeric {
        compare_numeric(a, b)
    } else if config.fold_case {
        compare_fold_case(a, b)
    } else {
        compare_bytes_raw(a, b)
    }
}

/// Raw bytewise comparison using unsigned byte values
/// This is used for last-resort and default lexicographic comparison
fn compare_bytes_raw(a: &[u8], b: &[u8]) -> Ordering {
    a.cmp(b)
}

/// Numeric comparison matching GNU sort -n behavior
///
/// Parses leading numeric value (with optional sign and decimal point).
/// Non-numeric lines compare as 0.
/// Does NOT handle scientific notation (use -g for that).
pub fn compare_numeric(a: &[u8], b: &[u8]) -> Ordering {
    let num_a = parse_leading_number(a);
    let num_b = parse_leading_number(b);

    // Use partial_cmp which treats -0.0 == +0.0 (matching GNU sort behavior)
    // This allows last-resort comparison to break ties based on byte values
    // For NaN, treat as 0 (compare equal, let last-resort handle it)
    num_a.partial_cmp(&num_b).unwrap_or(Ordering::Equal)
}

/// Parse leading number from bytes (GNU sort -n compatible)
///
/// - Skips leading whitespace (space and tab only)
/// - Handles optional sign (+ or -)
/// - Handles decimal point
/// - Stops at first non-numeric character
/// - Returns 0.0 for non-numeric input
/// - Works directly on bytes without requiring valid UTF-8
fn parse_leading_number(s: &[u8]) -> f64 {
    // Skip leading whitespace (bytes)
    let mut idx = 0;
    while idx < s.len() && (s[idx] == b' ' || s[idx] == b'\t') {
        idx += 1;
    }
    if idx >= s.len() {
        return 0.0;
    }

    let s = &s[idx..];
    let mut end = 0;
    let mut has_dot = false;

    // Optional sign
    if end < s.len() && (s[end] == b'-' || s[end] == b'+') {
        end += 1;
    }

    // Digits and decimal point
    while end < s.len() {
        if s[end].is_ascii_digit() {
            end += 1;
        } else if s[end] == b'.' && !has_dot {
            has_dot = true;
            end += 1;
        } else {
            break;
        }
    }

    // Edge cases: just sign or just dot
    if end == 0 || (end == 1 && matches!(s[0], b'-' | b'+' | b'.')) {
        return 0.0;
    }

    // Convert only the numeric prefix to string (guaranteed ASCII, so always valid UTF-8)
    // SAFETY: We've verified all bytes are ASCII digits, sign, or dot
    let num_str = unsafe { std::str::from_utf8_unchecked(&s[..end]) };
    num_str.parse().unwrap_or(0.0)
}

/// Case-folded comparison (ASCII only, a-z → A-Z)
fn compare_fold_case(a: &[u8], b: &[u8]) -> Ordering {
    let fold = |&b: &u8| -> u8 {
        if b.is_ascii_lowercase() {
            b.to_ascii_uppercase()
        } else {
            b
        }
    };

    a.iter().map(fold).cmp(b.iter().map(fold))
}

/// Apply reverse if needed
#[inline]
fn maybe_reverse(ord: Ordering, reverse: bool) -> Ordering {
    if reverse {
        ord.reverse()
    } else {
        ord
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        Config {
            reverse: false,
            numeric: false,
            fold_case: false,
            unique: false,
            stable: false,
            debug: false,
            record_delimiter: b'\n',
            field_separator: None,
            keys: vec![],
            output_file: None,
            input_files: vec![],
        }
    }

    #[test]
    fn test_bytewise_comparison() {
        assert_eq!(compare_bytes_raw(b"a", b"b"), Ordering::Less);
        assert_eq!(compare_bytes_raw(b"b", b"a"), Ordering::Greater);
        assert_eq!(compare_bytes_raw(b"a", b"a"), Ordering::Equal);
    }

    #[test]
    fn test_bytewise_high_bytes() {
        // 0xFF > 0x7F
        assert_eq!(compare_bytes_raw(b"\xff", b"\x7f"), Ordering::Greater);
        // 0x00 < everything
        assert_eq!(compare_bytes_raw(b"\x00", b"a"), Ordering::Less);
    }

    #[test]
    fn test_numeric_basic() {
        assert_eq!(compare_numeric(b"1", b"2"), Ordering::Less);
        assert_eq!(compare_numeric(b"10", b"2"), Ordering::Greater);
        assert_eq!(compare_numeric(b"10", b"10"), Ordering::Equal);
    }

    #[test]
    fn test_numeric_with_decimals() {
        assert_eq!(compare_numeric(b"1.5", b"1.10"), Ordering::Greater);
        assert_eq!(compare_numeric(b"1.9", b"1.10"), Ordering::Greater);
    }

    #[test]
    fn test_numeric_leading_zeros() {
        assert_eq!(compare_numeric(b"007", b"7"), Ordering::Equal);
        assert_eq!(compare_numeric(b"08", b"8"), Ordering::Equal);
    }

    #[test]
    fn test_numeric_negative() {
        assert_eq!(compare_numeric(b"-5", b"3"), Ordering::Less);
        assert_eq!(compare_numeric(b"-5", b"-3"), Ordering::Less);
    }

    #[test]
    fn test_numeric_non_numeric() {
        // Non-numeric treated as 0
        assert_eq!(compare_numeric(b"abc", b"0"), Ordering::Equal);
        assert_eq!(compare_numeric(b"abc", b"1"), Ordering::Less);
    }

    #[test]
    fn test_numeric_empty() {
        assert_eq!(compare_numeric(b"", b"0"), Ordering::Equal);
        assert_eq!(compare_numeric(b"   ", b"0"), Ordering::Equal);
    }

    #[test]
    fn test_numeric_with_high_bytes() {
        // Lines with invalid UTF-8 should still parse numeric prefix
        // "5\xff" should parse as 5, not 0
        assert_eq!(compare_numeric(b"5\xff", b"3"), Ordering::Greater);
        assert_eq!(compare_numeric(b"5\xff", b"5"), Ordering::Equal);
        // "\xff5" has no numeric prefix, so it's 0
        assert_eq!(compare_numeric(b"\xff5", b"0"), Ordering::Equal);
    }

    #[test]
    fn test_fold_case() {
        assert_eq!(compare_fold_case(b"A", b"a"), Ordering::Equal);
        assert_eq!(compare_fold_case(b"ABC", b"abc"), Ordering::Equal);
        assert_eq!(compare_fold_case(b"a", b"B"), Ordering::Less);
    }

    #[test]
    fn test_last_resort_ignores_fold() {
        let mut config = test_config();
        config.fold_case = true;

        // 'A' (0x41) < 'a' (0x61) bytewise
        // With -f: keys are equal, so last-resort kicks in
        assert_eq!(compare_records(b"A", b"a", &config), Ordering::Less);
    }

    #[test]
    fn test_last_resort_ignores_numeric() {
        let mut config = test_config();
        config.numeric = true;

        // "010" and "10" equal numerically, but "010" < "10" bytewise
        assert_eq!(compare_records(b"010", b"10", &config), Ordering::Less);
    }

    #[test]
    fn test_last_resort_with_reverse() {
        let mut config = test_config();
        config.reverse = true;

        // With -r: both key and last-resort are reversed
        assert_eq!(compare_records(b"a", b"b", &config), Ordering::Greater);
    }

    #[test]
    fn test_stable_disables_last_resort() {
        let mut config = test_config();
        config.fold_case = true;
        config.stable = true;

        // With -s: no last-resort, so A == a
        assert_eq!(compare_records(b"A", b"a", &config), Ordering::Equal);
    }

    #[test]
    fn test_unique_disables_last_resort() {
        let mut config = test_config();
        config.fold_case = true;
        config.unique = true;

        // With -u: no last-resort, so A == a
        assert_eq!(compare_records(b"A", b"a", &config), Ordering::Equal);
    }
}
