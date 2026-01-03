use std::cmp::Ordering;
use std::fs::File;
use std::io::{self, BufWriter, Write};

use crate::compare::compare_records;
use crate::config::Config;

/// Write records to output with optional deduplication
/// add_trailing controls whether to add delimiter after the last record
pub fn write_records<W: Write>(
    writer: W,
    records: &[Vec<u8>],
    config: &Config,
    add_trailing: bool,
) -> io::Result<()> {
    let mut writer = BufWriter::new(writer);
    let delimiter = config.record_delimiter;

    if config.unique {
        write_unique(&mut writer, records, config, delimiter, add_trailing)?;
    } else {
        write_all(&mut writer, records, delimiter, add_trailing)?;
    }

    writer.flush()
}

/// Write all records without deduplication
fn write_all<W: Write>(
    writer: &mut W,
    records: &[Vec<u8>],
    delimiter: u8,
    add_trailing: bool,
) -> io::Result<()> {
    let len = records.len();
    for (i, record) in records.iter().enumerate() {
        writer.write_all(record)?;
        // Add delimiter between records, and after last only if add_trailing
        if i < len - 1 || add_trailing {
            writer.write_all(&[delimiter])?;
        }
    }
    Ok(())
}

/// Write unique records only (first among equals by key comparison)
fn write_unique<W: Write>(
    writer: &mut W,
    records: &[Vec<u8>],
    config: &Config,
    delimiter: u8,
    add_trailing: bool,
) -> io::Result<()> {
    let mut prev: Option<&Vec<u8>> = None;
    let mut unique_records: Vec<&Vec<u8>> = Vec::new();

    for record in records {
        let is_dup = prev
            .map(|p| compare_for_unique(p, record, config) == Ordering::Equal)
            .unwrap_or(false);

        if !is_dup {
            unique_records.push(record);
            prev = Some(record);
        }
    }

    let len = unique_records.len();
    for (i, record) in unique_records.iter().enumerate() {
        writer.write_all(record)?;
        if i < len - 1 || add_trailing {
            writer.write_all(&[delimiter])?;
        }
    }

    Ok(())
}

/// Comparison for -u deduplication
///
/// Uses key comparison only (no last-resort) since -u disables last-resort.
fn compare_for_unique(a: &[u8], b: &[u8], config: &Config) -> Ordering {
    // compare_records already handles -u by disabling last-resort
    compare_records(a, b, config)
}

/// Open output file or return stdout
/// Handles "-" as stdout (GNU convention)
pub fn open_output(config: &Config) -> io::Result<Box<dyn Write>> {
    match &config.output_file {
        Some(path) if path == "-" => Ok(Box::new(io::stdout())),
        Some(path) => {
            let file = File::create(path)?;
            Ok(Box::new(file))
        }
        None => Ok(Box::new(io::stdout())),
    }
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
            debug: false,
            record_delimiter: b'\n',
            field_separator: None,
            keys: vec![],
            output_file: None,
            input_files: vec![],
        }
    }

    #[test]
    fn test_write_all() {
        let records: Vec<Vec<u8>> = vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()];
        let config = test_config();
        let mut output = Vec::new();
        write_records(&mut output, &records, &config, true).unwrap();
        assert_eq!(output, b"a\nb\nc\n");
    }

    #[test]
    fn test_write_unique() {
        let records: Vec<Vec<u8>> = vec![
            b"a".to_vec(),
            b"a".to_vec(),
            b"b".to_vec(),
            b"b".to_vec(),
            b"c".to_vec(),
        ];
        let mut config = test_config();
        config.unique = true;
        let mut output = Vec::new();
        write_records(&mut output, &records, &config, true).unwrap();
        assert_eq!(output, b"a\nb\nc\n");
    }

    #[test]
    fn test_write_unique_by_key() {
        // With -u -k1,1: lines with same first field are duplicates
        let records: Vec<Vec<u8>> = vec![
            b"a 1".to_vec(),
            b"a 2".to_vec(),
            b"b 1".to_vec(),
        ];
        let mut config = test_config();
        config.unique = true;
        config.keys = vec![KeySpec::parse("1,1").unwrap()];
        let mut output = Vec::new();
        write_records(&mut output, &records, &config, true).unwrap();
        assert_eq!(output, b"a 1\nb 1\n");
    }

    #[test]
    fn test_write_nul_delimiter() {
        let records: Vec<Vec<u8>> = vec![b"a".to_vec(), b"b".to_vec()];
        let mut config = test_config();
        config.record_delimiter = 0u8;
        let mut output = Vec::new();
        write_records(&mut output, &records, &config, true).unwrap();
        assert_eq!(output, b"a\0b\0");
    }
}
