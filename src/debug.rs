use std::io::{self, Write};

use crate::config::Config;
use crate::key::{extract_key, split_fields_with_positions};

/// Debug output for a single line showing key spans
///
/// Format matches GNU sort --debug:
/// - Print the original line
/// - Print underscores marking each key's span
pub fn debug_line<W: Write>(
    writer: &mut W,
    line: &[u8],
    config: &Config,
) -> io::Result<()> {
    // Print original line
    writer.write_all(line)?;
    writeln!(writer)?;

    if config.keys.is_empty() {
        // No -k: whole line is the key
        let underline = "_".repeat(line.len().max(1));
        writeln!(writer, "{}", underline)?;
    } else {
        // Show each key's span
        for key_spec in &config.keys {
            let key = extract_key(line, key_spec, config.field_separator);

            if key.is_empty() {
                writeln!(writer, "^ no match for key")?;
            } else {
                // Find the full span of the key in the original line
                if let Some((start, end)) = find_key_span(line, key_spec, config) {
                    let indent = " ".repeat(start);
                    let underline = "_".repeat((end - start).max(1));
                    writeln!(writer, "{}{}", indent, underline)?;
                } else {
                    // Key extracted but position unclear
                    let underline = "_".repeat(key.len());
                    writeln!(writer, "{}", underline)?;
                }
            }
        }
    }

    Ok(())
}

/// Find the byte span (start, end) of a key within the line
fn find_key_span(
    line: &[u8],
    key_spec: &crate::key::KeySpec,
    config: &Config,
) -> Option<(usize, usize)> {
    let fields = split_fields_with_positions(line, config.field_separator);

    let start_idx = key_spec.start_field.saturating_sub(1);
    if start_idx >= fields.len() {
        return None;
    }

    let end_idx = key_spec
        .end_field
        .map(|f| f.saturating_sub(1).min(fields.len().saturating_sub(1)))
        .unwrap_or(fields.len().saturating_sub(1));

    let (first_start, first_end) = fields[start_idx];
    let (last_start, last_end) = fields[end_idx];

    // Apply character offsets
    let start_char_offset = key_spec.start_char.unwrap_or(1).saturating_sub(1);
    let byte_start = (first_start + start_char_offset).min(first_end);

    let byte_end = if let Some(ec) = key_spec.end_char {
        (last_start + ec).min(last_end)
    } else {
        last_end
    };

    Some((byte_start, byte_end))
}

/// Emit debug output for all input lines during the read phase
pub fn debug_input<W: Write>(
    writer: &mut W,
    records: &[Vec<u8>],
    config: &Config,
) -> io::Result<()> {
    for record in records {
        debug_line(writer, record, config)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::KeySpec;

    fn test_config() -> Config {
        Config {
            reverse: false,
            numeric: false,
            fold_case: false,
            unique: false,
            stable: false,
            debug: true,
            record_delimiter: b'\n',
            field_separator: None,
            keys: vec![],
            output_file: None,
            input_files: vec![],
        }
    }

    #[test]
    fn test_debug_whole_line() {
        let config = test_config();
        let mut output = Vec::new();
        debug_line(&mut output, b"hello", &config).unwrap();
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("hello"));
        assert!(output_str.contains("_____"));
    }

    #[test]
    fn test_debug_with_key() {
        let mut config = test_config();
        config.keys = vec![KeySpec::parse("2,2").unwrap()];
        let mut output = Vec::new();
        debug_line(&mut output, b"foo bar baz", &config).unwrap();
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("foo bar baz"));
        // Should have underscores for "bar"
        assert!(output_str.contains("___"));
    }
}
