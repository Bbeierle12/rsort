//! Differential tests comparing rsort output against GNU sort
//!
//! All tests run with LC_ALL=C LANG=C to ensure bytewise collation.
//! On Windows, uses WSL to access GNU sort.
//! On Unix, uses native sort command.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// Check if we're running on Windows
fn is_windows() -> bool {
    cfg!(target_os = "windows")
}

/// Shell-escape an argument for bash
fn shell_escape(arg: &str) -> String {
    // Use single quotes and escape any single quotes in the argument
    format!("'{}'", arg.replace('\'', "'\\''"))
}

/// Run GNU sort command and return its stdout
fn run_gnu_sort(input: &[u8], args: &[&str]) -> Vec<u8> {
    if is_windows() {
        // Use WSL to run GNU sort on Windows
        // Must use 'env' to set LC_ALL inside WSL (Windows env vars don't pass through)
        let escaped_args: Vec<String> = args.iter().map(|a| shell_escape(a)).collect();
        let sort_args = escaped_args.join(" ");
        let shell_cmd = format!("LC_ALL=C LANG=C sort {}", sort_args);

        let mut cmd = Command::new("wsl")
            .args(["bash", "-c", &shell_cmd])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn wsl sort - is WSL installed?");

        if let Some(mut stdin) = cmd.stdin.take() {
            stdin.write_all(input).expect("failed to write stdin");
        }

        cmd.wait_with_output().expect("failed to wait").stdout
    } else {
        // Use native sort on Unix
        let mut cmd = Command::new("sort")
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .spawn()
            .expect("failed to spawn sort");

        if let Some(mut stdin) = cmd.stdin.take() {
            stdin.write_all(input).expect("failed to write stdin");
        }

        cmd.wait_with_output().expect("failed to wait").stdout
    }
}

/// Run rsort and return its stdout
fn run_rsort(input: &[u8], args: &[&str]) -> Vec<u8> {
    let rsort_path = if Path::new("./target/release/rsort.exe").exists() {
        "./target/release/rsort.exe"
    } else if Path::new("./target/release/rsort").exists() {
        "./target/release/rsort"
    } else if Path::new("./target/debug/rsort.exe").exists() {
        "./target/debug/rsort.exe"
    } else {
        "./target/debug/rsort"
    };

    let mut cmd = Command::new(rsort_path)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn rsort at {}: {}", rsort_path, e));

    if let Some(mut stdin) = cmd.stdin.take() {
        stdin.write_all(input).expect("failed to write stdin");
    }

    cmd.wait_with_output().expect("failed to wait").stdout
}

/// Compare rsort output against GNU sort
/// Returns true if outputs match byte-for-byte
fn compare_with_gnu(input: &[u8], args: &[&str]) -> bool {
    let gnu = run_gnu_sort(input, args);
    let rsort = run_rsort(input, args);

    if gnu != rsort {
        eprintln!("========== MISMATCH ==========");
        eprintln!("Args: {:?}", args);
        eprintln!("Input ({} bytes): {:?}", input.len(), String::from_utf8_lossy(input));
        eprintln!("GNU   ({} bytes): {:?}", gnu.len(), String::from_utf8_lossy(&gnu));
        eprintln!("rsort ({} bytes): {:?}", rsort.len(), String::from_utf8_lossy(&rsort));
        eprintln!("==============================");
        false
    } else {
        true
    }
}

// ============================================================
// Basic Functionality Tests
// ============================================================

#[test]
fn test_basic_sort() {
    assert!(compare_with_gnu(b"c\nb\na\n", &[]));
}

#[test]
fn test_already_sorted() {
    assert!(compare_with_gnu(b"a\nb\nc\n", &[]));
}

#[test]
fn test_reverse_sorted() {
    assert!(compare_with_gnu(b"c\nb\na\n", &[]));
}

#[test]
fn test_single_line() {
    assert!(compare_with_gnu(b"hello\n", &[]));
}

#[test]
fn test_empty_input() {
    assert!(compare_with_gnu(b"", &[]));
}

// ============================================================
// Reverse (-r) Tests
// ============================================================

#[test]
fn test_reverse() {
    assert!(compare_with_gnu(b"a\nb\nc\n", &["-r"]));
}

#[test]
fn test_reverse_with_duplicates() {
    assert!(compare_with_gnu(b"a\nb\na\nc\nb\n", &["-r"]));
}

// ============================================================
// Numeric (-n) Tests
// ============================================================

#[test]
fn test_numeric() {
    assert!(compare_with_gnu(b"10\n2\n1\n", &["-n"]));
}

#[test]
fn test_numeric_negative() {
    assert!(compare_with_gnu(b"5\n-3\n0\n-10\n3\n", &["-n"]));
}

#[test]
fn test_numeric_leading_zeros() {
    assert!(compare_with_gnu(b"007\n7\n08\n8\n", &["-n"]));
}

#[test]
fn test_numeric_negative_zero() {
    assert!(compare_with_gnu(b"-0\n0\n+0\n", &["-n"]));
}

#[test]
fn test_numeric_decimal() {
    assert!(compare_with_gnu(b"1.5\n1.10\n1.9\n", &["-n"]));
}

#[test]
fn test_numeric_scientific_not_supported() {
    // -n does NOT parse scientific notation (use -g for that)
    assert!(compare_with_gnu(b"1e5\n100\n1000\n", &["-n"]));
}

#[test]
fn test_numeric_empty_is_zero() {
    // Empty/blank lines are treated as 0 in numeric sort
    assert!(compare_with_gnu(b"\n5\n-3\n\n", &["-n"]));
}

#[test]
fn test_numeric_non_numeric_prefix() {
    // Non-numeric text sorts as 0
    assert!(compare_with_gnu(b"abc\n5\nxyz\n0\n", &["-n"]));
}

#[test]
fn test_numeric_whitespace_prefix() {
    // Leading whitespace is ignored
    assert!(compare_with_gnu(b"  10\n2\n   1\n", &["-n"]));
}

#[test]
fn test_numeric_reverse() {
    assert!(compare_with_gnu(b"1\n10\n2\n", &["-n", "-r"]));
}

// ============================================================
// Case Folding (-f) Tests
// ============================================================

#[test]
fn test_fold_case() {
    assert!(compare_with_gnu(b"B\na\nA\nb\n", &["-f"]));
}

#[test]
fn test_fold_case_mixed() {
    assert!(compare_with_gnu(b"Apple\napple\nAPPLE\nBanana\nbanana\n", &["-f"]));
}

#[test]
fn test_fold_case_reverse() {
    assert!(compare_with_gnu(b"a\nA\nb\nB\n", &["-f", "-r"]));
}

// ============================================================
// Unique (-u) Tests
// ============================================================

#[test]
fn test_unique() {
    assert!(compare_with_gnu(b"a\na\nb\n", &["-u"]));
}

#[test]
fn test_unique_by_key_not_line() {
    // -u with -k: dedupe by KEY, not by whole line
    assert!(compare_with_gnu(b"a 1\na 2\nb 1\n", &["-u", "-k1,1"]));
}

#[test]
fn test_unique_preserves_first() {
    // "first" means first in sorted order
    assert!(compare_with_gnu(b"B\nb\nA\na\n", &["-u", "-f"]));
}

#[test]
fn test_unique_with_numeric() {
    // Numeric unique: "007" and "7" are equal, one survives
    assert!(compare_with_gnu(b"7\n007\n7\n", &["-u", "-n"]));
}

#[test]
fn test_unique_consecutive_only() {
    // Test that -u removes consecutive duplicates after sort
    assert!(compare_with_gnu(b"a\nb\na\nb\na\n", &["-u"]));
}

// ============================================================
// Stable Sort (-s) Tests
// ============================================================

#[test]
fn test_stable() {
    assert!(compare_with_gnu(b"b 1\na 2\nb 2\na 1\n", &["-s", "-k1,1"]));
}

#[test]
fn test_stable_preserves_input_order() {
    // With -s, equal-key lines preserve their input order
    assert!(compare_with_gnu(b"b X\na Y\nb Z\na W\n", &["-s", "-k1,1"]));
}

#[test]
fn test_stable_with_fold() {
    // -s -f: case-equal lines preserve input order
    assert!(compare_with_gnu(b"B\na\nb\nA\n", &["-s", "-f"]));
}

#[test]
fn test_stable_with_numeric() {
    assert!(compare_with_gnu(b"10 a\n10 b\n2 c\n2 d\n", &["-s", "-k1,1", "-n"]));
}

// ============================================================
// Key Specification (-k) Tests
// ============================================================

#[test]
fn test_key_single_field() {
    assert!(compare_with_gnu(b"x 3\ny 1\nz 2\n", &["-k2,2", "-n"]));
}

#[test]
fn test_key_field_range() {
    assert!(compare_with_gnu(b"a b c\nd e f\ng h i\n", &["-k2,3"]));
}

#[test]
fn test_key_field_beyond_end() {
    // Key references field that doesn't exist: treated as empty
    assert!(compare_with_gnu(b"a\na b\na b c\n", &["-k3,3"]));
}

#[test]
fn test_key_char_beyond_end() {
    // Character position beyond field length: treated as empty
    assert!(compare_with_gnu(b"ab\nabc\nabcd\n", &["-k1.10,1.20"]));
}

#[test]
fn test_key_multiple_keys() {
    // Multiple -k: compared in order, first difference wins
    assert!(compare_with_gnu(b"a 2\na 1\nb 1\n", &["-k1,1", "-k2,2", "-n"]));
}

#[test]
fn test_key_overlapping() {
    // Overlapping key ranges
    assert!(compare_with_gnu(b"abc\nbcd\nabc\n", &["-k1.1,1.2", "-k1.2,1.3"]));
}

#[test]
fn test_key_char_positions() {
    // -k1.2,1.4 means characters 2 through 4 of field 1
    assert!(compare_with_gnu(b"xabc\nxdef\nxghi\n", &["-k1.2,1.4"]));
}

// ============================================================
// Field Delimiter (-t) Tests
// ============================================================

#[test]
fn test_delimiter_colon() {
    assert!(compare_with_gnu(b"a:3\nb:1\nc:2\n", &["-t:", "-k2,2", "-n"]));
}

#[test]
fn test_delimiter_tab() {
    assert!(compare_with_gnu(b"a\t3\nb\t1\nc\t2\n", &["-t\t", "-k2,2", "-n"]));
}

#[test]
fn test_delimiter_consecutive() {
    // Consecutive delimiters create empty fields
    assert!(compare_with_gnu(b"a::c\na:b:c\n", &["-t:", "-k2,2"]));
}

#[test]
fn test_delimiter_at_end() {
    // Trailing delimiter
    assert!(compare_with_gnu(b"a:b:\na:b:c\n", &["-t:", "-k3,3"]));
}

#[test]
fn test_delimiter_at_start() {
    // Leading delimiter creates empty first field
    assert!(compare_with_gnu(b":a:b\nc:d:e\n", &["-t:", "-k1,1"]));
}

// ============================================================
// NUL Delimiter (-z) Tests
// ============================================================

#[test]
fn test_zero_terminated() {
    assert!(compare_with_gnu(b"c\0b\0a\0", &["-z"]));
}

#[test]
fn test_zero_terminated_with_newlines() {
    // With -z, newlines are regular characters
    assert!(compare_with_gnu(b"c\nd\0a\nb\0", &["-z"]));
}

#[test]
fn test_zero_terminated_numeric() {
    assert!(compare_with_gnu(b"10\x002\x001\x00", &["-z", "-n"]));
}

// ============================================================
// Last-Resort Comparison Edge Cases
// ============================================================

#[test]
fn test_last_resort_ignores_numeric() {
    // "10" and "010" are equal numerically, but differ in last-resort (bytewise)
    // "010" < "10" because '0' (0x30) < '1' (0x31)
    assert!(compare_with_gnu(b"10\n010\n", &["-n"]));
}

#[test]
fn test_last_resort_ignores_fold() {
    // "A" and "a" are equal with -f, but last-resort sees 'A' (0x41) < 'a' (0x61)
    assert!(compare_with_gnu(b"a\nA\n", &["-f"]));
    assert!(compare_with_gnu(b"A\na\n", &["-f"]));
}

#[test]
fn test_last_resort_with_reverse() {
    // -r applies to both key comparison AND last-resort
    assert!(compare_with_gnu(b"a\nA\n", &["-f", "-r"]));
    assert!(compare_with_gnu(b"A\na\n", &["-f", "-r"]));
}

#[test]
fn test_last_resort_numeric_ties() {
    // Multiple lines with same numeric value but different text
    assert!(compare_with_gnu(b"1 apple\n1 banana\n01 cherry\n", &["-n", "-k1,1"]));
}

// ============================================================
// Empty and Whitespace Edge Cases
// ============================================================

#[test]
fn test_empty_lines() {
    assert!(compare_with_gnu(b"\n\na\n\nb\n\n", &[]));
}

#[test]
fn test_only_empty_lines() {
    assert!(compare_with_gnu(b"\n\n\n", &[]));
}

#[test]
fn test_whitespace_only_lines() {
    assert!(compare_with_gnu(b"   \n\t\n  \t  \n", &[]));
}

#[test]
fn test_mixed_empty_and_content() {
    assert!(compare_with_gnu(b"\na\n\nb\n\nc\n\n", &[]));
}

// ============================================================
// Binary/High-Byte Edge Cases
// ============================================================

#[test]
fn test_binary_bytes() {
    // Bytes 0x80-0xFF should sort correctly (unsigned comparison)
    assert!(compare_with_gnu(b"\xff\n\x80\n\x00\n\x7f\n", &[]));
}

#[test]
fn test_embedded_nul() {
    // NUL bytes in lines (not as delimiter)
    assert!(compare_with_gnu(b"a\x00z\na\x00a\nb\n", &[]));
}

#[test]
fn test_high_bytes_order() {
    // Verify unsigned byte ordering: 0x7F < 0x80 < 0xFF
    assert!(compare_with_gnu(b"\x7f\n\x80\n\xff\n", &[]));
}

#[test]
fn test_all_byte_values() {
    // Test sorting with various byte values
    let mut input = Vec::new();
    for b in [0x00, 0x01, 0x7e, 0x7f, 0x80, 0x81, 0xfe, 0xff] {
        input.push(b);
        input.push(b'\n');
    }
    assert!(compare_with_gnu(&input, &[]));
}

// ============================================================
// No Trailing Newline Edge Cases
// ============================================================

#[test]
fn test_no_trailing_newline() {
    assert!(compare_with_gnu(b"b\na", &[]));
}

#[test]
fn test_no_trailing_newline_single() {
    assert!(compare_with_gnu(b"hello", &[]));
}

#[test]
fn test_no_trailing_newline_empty() {
    assert!(compare_with_gnu(b"", &[]));
}

// ============================================================
// Reverse with Other Options
// ============================================================

#[test]
fn test_reverse_stable() {
    assert!(compare_with_gnu(b"a\nb\na\n", &["-r", "-s"]));
}

#[test]
fn test_reverse_unique() {
    assert!(compare_with_gnu(b"a\nb\na\nb\n", &["-r", "-u"]));
}

#[test]
fn test_reverse_numeric_unique() {
    assert!(compare_with_gnu(b"1\n2\n1\n2\n", &["-r", "-n", "-u"]));
}

// ============================================================
// Long Line Edge Cases
// ============================================================

#[test]
fn test_very_long_line() {
    let long = "x".repeat(100000);
    let input = format!("{}\nshort\n", long);
    assert!(compare_with_gnu(input.as_bytes(), &[]));
}

#[test]
fn test_many_short_lines() {
    let mut input = String::new();
    for i in 0..1000 {
        input.push_str(&format!("line{}\n", i));
    }
    assert!(compare_with_gnu(input.as_bytes(), &[]));
}

// ============================================================
// Complex Combined Flag Tests
// ============================================================

#[test]
fn test_numeric_reverse_unique() {
    assert!(compare_with_gnu(b"1\n2\n1\n3\n2\n", &["-n", "-r", "-u"]));
}

#[test]
fn test_fold_stable_key() {
    assert!(compare_with_gnu(
        b"Apple 1\napple 2\nBANANA 3\nbanana 4\n",
        &["-f", "-s", "-k1,1"]
    ));
}

#[test]
fn test_numeric_key_unique() {
    assert!(compare_with_gnu(
        b"x 10\ny 2\nz 10\nw 2\n",
        &["-n", "-k2,2", "-u"]
    ));
}

#[test]
fn test_delimiter_numeric_reverse() {
    assert!(compare_with_gnu(b"a:10\nb:2\nc:1\n", &["-t:", "-k2,2", "-n", "-r"]));
}

// ============================================================
// Special Character Tests
// ============================================================

#[test]
fn test_spaces_in_data() {
    assert!(compare_with_gnu(b"  a\n a\na\n", &[]));
}

#[test]
fn test_tabs_in_data() {
    assert!(compare_with_gnu(b"\ta\n\t\ta\na\n", &[]));
}

#[test]
fn test_mixed_whitespace() {
    assert!(compare_with_gnu(b" \ta\n\t a\n  a\n", &[]));
}

// ============================================================
// Duplicate Handling
// ============================================================

#[test]
fn test_all_duplicates() {
    assert!(compare_with_gnu(b"a\na\na\na\n", &[]));
}

#[test]
fn test_all_duplicates_unique() {
    assert!(compare_with_gnu(b"a\na\na\na\n", &["-u"]));
}

#[test]
fn test_alternating_duplicates() {
    assert!(compare_with_gnu(b"a\nb\na\nb\na\nb\n", &[]));
}

#[test]
fn test_alternating_duplicates_unique() {
    assert!(compare_with_gnu(b"a\nb\na\nb\na\nb\n", &["-u"]));
}
