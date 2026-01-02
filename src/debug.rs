use std::io::{self, Write};

use crate::config::Config;
use crate::key::extract_key;

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
                // Find where the key appears in the line
                if let Some(pos) = find_key_position(line, &key, key_spec, config) {
                    let indent = " ".repeat(pos);
                    let underline = "_".repeat(key.len().max(1));
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

/// Find the column position of a key within the line
fn find_key_position(
    line: &[u8],
    _key: &[u8],
    key_spec: &crate::key::KeySpec,
    config: &Config,
) -> Option<usize> {
    // Calculate position based on field/char spec
    let fields = split_fields_with_positions(line, config.field_separator);

    let field_idx = key_spec.start_field.saturating_sub(1);
    if field_idx >= fields.len() {
        return None;
    }

    let (field_start, _field_end) = fields[field_idx];
    let char_offset = key_spec.start_char.unwrap_or(1).saturating_sub(1);

    Some(field_start + char_offset)
}

/// Split line into fields, returning (start_pos, end_pos) for each
fn split_fields_with_positions(line: &[u8], separator: Option<u8>) -> Vec<(usize, usize)> {
    match separator {
        Some(sep) => {
            let mut fields = Vec::new();
            let mut start = 0;

            for (i, &b) in line.iter().enumerate() {
                if b == sep {
                    fields.push((start, i));
                    start = i + 1;
                }
            }
            fields.push((start, line.len()));
            fields
        }
        None => {
            // Whitespace-delimited
            let mut fields = Vec::new();
            let mut in_field = false;
            let mut start = 0;

            for (i, &b) in line.iter().enumerate() {
                let is_space = b == b' ' || b == b'\t';

                if is_space && in_field {
                    fields.push((start, i));
                    in_field = false;
                } else if !is_space && !in_field {
                    start = i;
                    in_field = true;
                }
            }

            if in_field {
                fields.push((start, line.len()));
            }

            if fields.is_empty() {
                fields.push((0, 0));
            }

            fields
        }
    }
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
        let mut config = test_config();
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
