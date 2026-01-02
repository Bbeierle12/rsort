use crate::error::{Result, RsortError};

/// Parsed key specification from -k argument
#[derive(Clone, Debug, Default)]
pub struct KeySpec {
    /// Starting field (1-indexed)
    pub start_field: usize,
    /// Starting character within field (1-indexed, optional)
    pub start_char: Option<usize>,
    /// Ending field (1-indexed, optional - defaults to end of line)
    pub end_field: Option<usize>,
    /// Ending character within field (1-indexed, optional)
    pub end_char: Option<usize>,
    // Future: per-key modifiers (b, d, f, i, n, r)
}

impl KeySpec {
    /// Parse key specification like "1", "1,2", "2.3,2.5", "1,1"
    ///
    /// Format: FIELD[.CHAR][,FIELD[.CHAR]]
    pub fn parse(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split(',').collect();

        if parts.is_empty() || parts.len() > 2 {
            return Err(RsortError::InvalidKey(s.to_string()));
        }

        let (start_field, start_char) = parse_field_char(parts[0])?;

        if start_field == 0 {
            return Err(RsortError::InvalidKey(
                "field number must be >= 1".to_string(),
            ));
        }

        let (end_field, end_char) = if parts.len() > 1 {
            let (f, c) = parse_field_char(parts[1])?;
            (Some(f), c)
        } else {
            (None, None)
        };

        Ok(KeySpec {
            start_field,
            start_char,
            end_field,
            end_char,
        })
    }
}

/// Parse "FIELD" or "FIELD.CHAR" into (field, optional_char)
fn parse_field_char(s: &str) -> Result<(usize, Option<usize>)> {
    // Strip any trailing modifier letters (for future compatibility)
    let s = s.trim_end_matches(|c: char| c.is_ascii_alphabetic());

    let parts: Vec<&str> = s.split('.').collect();

    if parts.is_empty() || parts.len() > 2 {
        return Err(RsortError::InvalidKey(s.to_string()));
    }

    let field: usize = parts[0]
        .parse()
        .map_err(|_| RsortError::InvalidKey(format!("invalid field number: {}", parts[0])))?;

    let char_pos = if parts.len() > 1 {
        Some(
            parts[1]
                .parse()
                .map_err(|_| RsortError::InvalidKey(format!("invalid char position: {}", parts[1])))?,
        )
    } else {
        None
    };

    Ok((field, char_pos))
}

/// Extract key bytes from a record based on KeySpec
pub fn extract_key(record: &[u8], spec: &KeySpec, field_separator: Option<u8>) -> Vec<u8> {
    let fields = split_fields(record, field_separator);

    // Convert to 0-indexed
    let start_idx = spec.start_field.saturating_sub(1);

    // Field doesn't exist: return empty
    if start_idx >= fields.len() {
        return Vec::new();
    }

    let end_idx = spec
        .end_field
        .map(|f| f.saturating_sub(1).min(fields.len().saturating_sub(1)))
        .unwrap_or(fields.len().saturating_sub(1));

    // Single field extraction with character positions
    if start_idx == end_idx {
        let field = fields[start_idx];
        let start_char = spec.start_char.unwrap_or(1).saturating_sub(1);
        let end_char = spec.end_char.unwrap_or(field.len());

        return field
            .get(start_char..end_char.min(field.len()))
            .unwrap_or(&[])
            .to_vec();
    }

    // Multiple fields: concatenate with separator
    let mut result = Vec::new();
    for i in start_idx..=end_idx {
        if i < fields.len() {
            if !result.is_empty() {
                // Use space as separator for whitespace-delimited, else use actual separator
                let sep = field_separator.unwrap_or(b' ');
                result.push(sep);
            }

            let field = fields[i];

            // Apply char positions only to first/last fields
            let start = if i == start_idx {
                spec.start_char.unwrap_or(1).saturating_sub(1)
            } else {
                0
            };
            let end = if i == end_idx {
                spec.end_char.unwrap_or(field.len()).min(field.len())
            } else {
                field.len()
            };

            if let Some(slice) = field.get(start..end) {
                result.extend_from_slice(slice);
            }
        }
    }

    result
}

/// Split record into fields based on separator
fn split_fields(record: &[u8], separator: Option<u8>) -> Vec<&[u8]> {
    match separator {
        Some(sep) => record.split(|&b| b == sep).collect(),
        None => {
            // Default: split on runs of whitespace (space or tab)
            // Leading whitespace is included in first field
            let mut fields = Vec::new();
            let mut in_field = false;
            let mut start = 0;

            for (i, &b) in record.iter().enumerate() {
                let is_space = b == b' ' || b == b'\t';

                if is_space && in_field {
                    fields.push(&record[start..i]);
                    in_field = false;
                } else if !is_space && !in_field {
                    start = i;
                    in_field = true;
                }
            }

            if in_field {
                fields.push(&record[start..]);
            }

            // Handle empty record
            if fields.is_empty() {
                fields.push(&record[0..0]);
            }

            fields
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_field() {
        let spec = KeySpec::parse("2").unwrap();
        assert_eq!(spec.start_field, 2);
        assert_eq!(spec.start_char, None);
        assert_eq!(spec.end_field, None);
    }

    #[test]
    fn test_parse_field_range() {
        let spec = KeySpec::parse("1,2").unwrap();
        assert_eq!(spec.start_field, 1);
        assert_eq!(spec.end_field, Some(2));
    }

    #[test]
    fn test_parse_with_chars() {
        let spec = KeySpec::parse("2.3,2.5").unwrap();
        assert_eq!(spec.start_field, 2);
        assert_eq!(spec.start_char, Some(3));
        assert_eq!(spec.end_field, Some(2));
        assert_eq!(spec.end_char, Some(5));
    }

    #[test]
    fn test_extract_key_whitespace() {
        let record = b"apple banana cherry";
        let spec = KeySpec::parse("2,2").unwrap();
        let key = extract_key(record, &spec, None);
        assert_eq!(key, b"banana");
    }

    #[test]
    fn test_extract_key_delimiter() {
        let record = b"a:b:c";
        let spec = KeySpec::parse("2,2").unwrap();
        let key = extract_key(record, &spec, Some(b':'));
        assert_eq!(key, b"b");
    }

    #[test]
    fn test_extract_key_beyond_end() {
        let record = b"a b";
        let spec = KeySpec::parse("5,5").unwrap();
        let key = extract_key(record, &spec, None);
        assert_eq!(key, b"");
    }

    #[test]
    fn test_extract_key_char_range() {
        let record = b"abcdef";
        let spec = KeySpec::parse("1.2,1.4").unwrap();
        let key = extract_key(record, &spec, None);
        assert_eq!(key, b"bcd");
    }
}
