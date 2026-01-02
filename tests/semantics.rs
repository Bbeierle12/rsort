//! Tests for comparison contract semantics
//!
//! These tests verify the core comparison behavior independent of GNU sort.
//! They document and enforce the expected semantics of rsort's comparison contract.

use rsort::compare::compare_records;
use rsort::config::Config;
use rsort::key::KeySpec;
use std::cmp::Ordering;

/// Create a default test configuration
fn default_config() -> Config {
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

// ============================================================
// Last-Resort Comparison Contract
// ============================================================

#[test]
fn test_last_resort_is_bytewise() {
    let config = default_config();
    // 'A' (0x41) < 'a' (0x61) bytewise
    assert_eq!(compare_records(b"A", b"a", &config), Ordering::Less);
}

#[test]
fn test_last_resort_uses_unsigned_bytes() {
    let config = default_config();
    // 0x7F < 0x80 (unsigned comparison)
    assert_eq!(compare_records(b"\x7f", b"\x80", &config), Ordering::Less);
    // 0xFF is the largest byte value
    assert_eq!(compare_records(b"\xff", b"\x00", &config), Ordering::Greater);
}

#[test]
fn test_last_resort_ignores_fold_case() {
    let mut config = default_config();
    config.fold_case = true;

    // With -f: A == a on keys, but last-resort sees A < a
    assert_eq!(compare_records(b"A", b"a", &config), Ordering::Less);
}

#[test]
fn test_last_resort_ignores_numeric() {
    let mut config = default_config();
    config.numeric = true;

    // "10" and "010" equal numerically, but "010" < "10" bytewise
    // '0' (0x30) < '1' (0x31)
    assert_eq!(compare_records(b"010", b"10", &config), Ordering::Less);
}

#[test]
fn test_last_resort_applies_reverse() {
    let mut config = default_config();
    config.reverse = true;

    // With -r: last-resort is also reversed
    // Normally A < a, but with -r: A > a
    assert_eq!(compare_records(b"A", b"a", &config), Ordering::Greater);
}

#[test]
fn test_last_resort_reverse_with_fold() {
    let mut config = default_config();
    config.fold_case = true;
    config.reverse = true;

    // With -f -r: keys compare equal (A == a folded), then reversed
    // Last-resort would be A < a, but reversed becomes A > a
    assert_eq!(compare_records(b"A", b"a", &config), Ordering::Greater);
}

// ============================================================
// Last-Resort Disabling
// ============================================================

#[test]
fn test_stable_disables_last_resort() {
    let mut config = default_config();
    config.fold_case = true;
    config.stable = true;

    // With -s -f: A == a (no last-resort to break tie)
    assert_eq!(compare_records(b"A", b"a", &config), Ordering::Equal);
}

#[test]
fn test_unique_disables_last_resort() {
    let mut config = default_config();
    config.fold_case = true;
    config.unique = true;

    // With -u -f: A == a (no last-resort)
    assert_eq!(compare_records(b"A", b"a", &config), Ordering::Equal);
}

#[test]
fn test_stable_numeric_no_last_resort() {
    let mut config = default_config();
    config.numeric = true;
    config.stable = true;

    // With -s -n: "10" == "010" (numerically equal, no last-resort)
    assert_eq!(compare_records(b"010", b"10", &config), Ordering::Equal);
}

// ============================================================
// Numeric Comparison Semantics
// ============================================================

#[test]
fn test_numeric_basic_order() {
    let mut config = default_config();
    config.numeric = true;

    assert_eq!(compare_records(b"1", b"2", &config), Ordering::Less);
    assert_eq!(compare_records(b"10", b"2", &config), Ordering::Greater);
}

#[test]
fn test_numeric_negative_numbers() {
    let mut config = default_config();
    config.numeric = true;

    assert_eq!(compare_records(b"-5", b"0", &config), Ordering::Less);
    assert_eq!(compare_records(b"-5", b"-3", &config), Ordering::Less);
    assert_eq!(compare_records(b"-5", b"-10", &config), Ordering::Greater);
}

#[test]
fn test_numeric_decimals() {
    let mut config = default_config();
    config.numeric = true;

    assert_eq!(compare_records(b"1.5", b"1.6", &config), Ordering::Less);
    assert_eq!(compare_records(b"1.10", b"1.5", &config), Ordering::Less); // 1.10 < 1.5
}

#[test]
fn test_numeric_leading_whitespace() {
    let mut config = default_config();
    config.numeric = true;
    config.stable = true; // Disable last-resort to test numeric comparison only

    // Leading whitespace is ignored
    assert_eq!(compare_records(b"  5", b"5", &config), Ordering::Equal);
    assert_eq!(compare_records(b"\t10", b"10", &config), Ordering::Equal);
}

#[test]
fn test_numeric_non_numeric_is_zero() {
    let mut config = default_config();
    config.numeric = true;
    config.stable = true; // Disable last-resort to test numeric comparison only

    // Non-numeric text compares as 0
    assert_eq!(compare_records(b"abc", b"0", &config), Ordering::Equal);
    assert_eq!(compare_records(b"xyz", b"1", &config), Ordering::Less);
}

#[test]
fn test_numeric_empty_is_zero() {
    let mut config = default_config();
    config.numeric = true;
    config.stable = true; // Disable last-resort to test numeric comparison only

    assert_eq!(compare_records(b"", b"0", &config), Ordering::Equal);
    assert_eq!(compare_records(b"   ", b"0", &config), Ordering::Equal);
}

// ============================================================
// Case Folding Semantics
// ============================================================

#[test]
fn test_fold_case_ascii_only() {
    let mut config = default_config();
    config.fold_case = true;

    // ASCII letters are folded
    assert_eq!(compare_records(b"ABC", b"abc", &config), Ordering::Less);
    // Last-resort breaks the tie: A < a
}

#[test]
fn test_fold_case_mixed() {
    let mut config = default_config();
    config.fold_case = true;

    // "Apple" and "apple" compare equal on keys, A < a in last-resort
    assert_eq!(compare_records(b"Apple", b"apple", &config), Ordering::Less);
}

#[test]
fn test_fold_case_non_ascii_unchanged() {
    let mut config = default_config();
    config.fold_case = true;
    config.stable = true; // Disable last-resort for clear testing

    // High bytes (>127) are not affected by case folding
    assert_eq!(compare_records(b"\xe0", b"\xc0", &config), Ordering::Greater);
}

// ============================================================
// Key Extraction Semantics
// ============================================================

#[test]
fn test_key_extracts_correct_field() {
    let mut config = default_config();
    config.numeric = true;
    config.keys = vec![KeySpec::parse("2,2").unwrap()];

    // Sort by second field numerically
    // "a 10" vs "b 2": key is "10" vs "2", so "a 10" > "b 2"
    assert_eq!(
        compare_records(b"a 10", b"b 2", &config),
        Ordering::Greater
    );
}

#[test]
fn test_key_missing_field_is_empty() {
    let mut config = default_config();
    config.stable = true; // Disable last-resort
    config.keys = vec![KeySpec::parse("5,5").unwrap()];

    // Field 5 doesn't exist, so both keys are empty
    assert_eq!(compare_records(b"a b c", b"x y z", &config), Ordering::Equal);
}

#[test]
fn test_multiple_keys_first_wins() {
    let mut config = default_config();
    config.stable = true;
    config.keys = vec![
        KeySpec::parse("1,1").unwrap(),
        KeySpec::parse("2,2").unwrap(),
    ];

    // First keys differ, so second key doesn't matter
    assert_eq!(compare_records(b"a 1", b"b 2", &config), Ordering::Less);
}

#[test]
fn test_multiple_keys_fallback() {
    let mut config = default_config();
    config.stable = true;
    config.numeric = true;
    config.keys = vec![
        KeySpec::parse("1,1").unwrap(),
        KeySpec::parse("2,2").unwrap(),
    ];

    // First keys equal, fall back to second key
    assert_eq!(compare_records(b"a 10", b"a 2", &config), Ordering::Greater);
}

// ============================================================
// Field Separator Semantics
// ============================================================

#[test]
fn test_custom_separator() {
    let mut config = default_config();
    config.numeric = true;
    config.field_separator = Some(b':');
    config.keys = vec![KeySpec::parse("2,2").unwrap()];

    // With -t:, fields are split on ':'
    assert_eq!(
        compare_records(b"a:10", b"b:2", &config),
        Ordering::Greater
    );
}

#[test]
fn test_consecutive_separators() {
    let mut config = default_config();
    config.stable = true;
    config.field_separator = Some(b':');
    config.keys = vec![KeySpec::parse("2,2").unwrap()];

    // "a::c" has empty field 2, "a:b:c" has "b" as field 2
    // Empty string < "b"
    assert_eq!(compare_records(b"a::c", b"a:b:c", &config), Ordering::Less);
}

// ============================================================
// Complete Comparison Flow
// ============================================================

#[test]
fn test_full_flow_with_last_resort() {
    let config = default_config();

    // Direct bytewise comparison, last-resort enabled
    assert_eq!(compare_records(b"apple", b"banana", &config), Ordering::Less);
    assert_eq!(
        compare_records(b"APPLE", b"apple", &config),
        Ordering::Less
    );
}

#[test]
fn test_full_flow_numeric_last_resort() {
    let mut config = default_config();
    config.numeric = true;

    // "1" and "01" are numerically equal (both = 1)
    // Last-resort: "01" < "1" bytewise
    assert_eq!(compare_records(b"01", b"1", &config), Ordering::Less);
}

#[test]
fn test_full_flow_key_then_last_resort() {
    let mut config = default_config();
    config.keys = vec![KeySpec::parse("1,1").unwrap()];

    // Keys "a" and "a" are equal
    // Last-resort compares full lines: "a x" < "a y"
    assert_eq!(compare_records(b"a x", b"a y", &config), Ordering::Less);
}

// ============================================================
// Edge Cases
// ============================================================

#[test]
fn test_empty_vs_empty() {
    let config = default_config();
    assert_eq!(compare_records(b"", b"", &config), Ordering::Equal);
}

#[test]
fn test_empty_vs_nonempty() {
    let config = default_config();
    // Empty string comes before any non-empty string
    assert_eq!(compare_records(b"", b"a", &config), Ordering::Less);
}

#[test]
fn test_prefix_comparison() {
    let config = default_config();
    // "ab" < "abc" (prefix is less)
    assert_eq!(compare_records(b"ab", b"abc", &config), Ordering::Less);
}

#[test]
fn test_same_length_different_content() {
    let config = default_config();
    assert_eq!(compare_records(b"abc", b"abd", &config), Ordering::Less);
}
