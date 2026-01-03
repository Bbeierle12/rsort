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

        // Validate end_field
        if let Some(ef) = end_field {
            if ef == 0 {
                return Err(RsortError::InvalidKey(
                    "end field must be >= 1".to_string(),
                ));
            }
            if ef < start_field {
                return Err(RsortError::InvalidKey(format!(
                    "end field {} < start field {}",
                    ef, start_field
                )));
            }
        }

        // Validate character positions
        if let Some(sc) = start_char {
            if sc == 0 {
                return Err(RsortError::InvalidKey(
                    "start char must be >= 1".to_string(),
                ));
            }
        }
        if let Some(ec) = end_char {
            if ec == 0 {
                return Err(RsortError::InvalidKey(
                    "end char must be >= 1".to_string(),
                ));
            }
        }

        // Same field: validate char ordering
        if end_field == Some(start_field) {
            if let (Some(sc), Some(ec)) = (start_char, end_char) {
                if ec < sc {
                    return Err(RsortError::InvalidKey(format!(
                        "end char {} < start char {}",
                        ec, sc
                    )));
                }
            }
        }

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
/// For multi-field keys, preserves original bytes (including separators) from the record
pub fn extract_key(record: &[u8], spec: &KeySpec, field_separator: Option<u8>) -> Vec<u8> {
    let fields_with_pos = split_fields_with_positions(record, field_separator);

    // Convert to 0-indexed
    let start_idx = spec.start_field.saturating_sub(1);

    // Field doesn't exist: return empty
    if start_idx >= fields_with_pos.len() {
        return Vec::new();
    }

    let end_idx = spec
        .end_field
        .map(|f| f.saturating_sub(1).min(fields_with_pos.len().saturating_sub(1)))
        .unwrap_or(fields_with_pos.len().saturating_sub(1));

    let (first_start, first_end) = fields_with_pos[start_idx];
    let (last_start, last_end) = fields_with_pos[end_idx];

    // Apply character offsets
    let start_char_offset = spec.start_char.unwrap_or(1).saturating_sub(1);
    let byte_start = (first_start + start_char_offset).min(first_end);

    let byte_end = if let Some(ec) = spec.end_char {
        // end_char applies to the last field
        (last_start + ec).min(last_end)
    } else {
        last_end
    };

    // For single field, just slice
    if start_idx == end_idx {
        return record.get(byte_start..byte_end).unwrap_or(&[]).to_vec();
    }

    // For multiple fields, copy the entire span from record (preserving original separators)
    record.get(byte_start..byte_end).unwrap_or(&[]).to_vec()
}

/// Split record into fields, returning (start_pos, end_pos) for each
pub fn split_fields_with_positions(record: &[u8], separator: Option<u8>) -> Vec<(usize, usize)> {
    match separator {
        Some(sep) => {
            let mut fields = Vec::new();
            let mut start = 0;

            for (i, &b) in record.iter().enumerate() {
                if b == sep {
                    fields.push((start, i));
                    start = i + 1;
                }
            }
            fields.push((start, record.len()));
            fields
        }
        None => {
            // Whitespace-delimited
            let mut fields = Vec::new();
            let mut in_field = false;
            let mut start = 0;

            for (i, &b) in record.iter().enumerate() {
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
                fields.push((start, record.len()));
            }

            if fields.is_empty() {
                fields.push((0, 0));
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
