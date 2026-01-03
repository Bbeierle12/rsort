use std::io::{self, BufRead};

/// Reads records from input, splitting on the specified delimiter
pub struct RecordReader<R> {
    reader: R,
    delimiter: u8,
    buffer: Vec<u8>,
    last_had_delimiter: bool,
}

impl<R: BufRead> RecordReader<R> {
    pub fn new(reader: R, delimiter: u8) -> Self {
        Self {
            reader,
            delimiter,
            buffer: Vec::new(),
            last_had_delimiter: true,
        }
    }

    /// Read the next record, returning None at EOF
    pub fn read_record(&mut self) -> io::Result<Option<&[u8]>> {
        self.buffer.clear();
        let bytes_read = self.reader.read_until(self.delimiter, &mut self.buffer)?;

        if bytes_read == 0 {
            return Ok(None);
        }

        // Track and strip delimiter if present at end
        self.last_had_delimiter = self.buffer.last() == Some(&self.delimiter);
        if self.last_had_delimiter {
            self.buffer.pop();
        }

        Ok(Some(&self.buffer))
    }

    /// Returns whether the last read record had a trailing delimiter
    pub fn last_had_delimiter(&self) -> bool {
        self.last_had_delimiter
    }
}

/// Read all records from a reader into a Vec
/// Returns (records, had_trailing_delimiter)
/// Note: GNU sort always adds trailing delimiter, so we always return true
pub fn read_all_records<R: BufRead>(reader: R, delimiter: u8) -> io::Result<(Vec<Vec<u8>>, bool)> {
    let mut records = Vec::new();
    let mut rec_reader = RecordReader::new(reader, delimiter);

    while let Some(record) = rec_reader.read_record()? {
        records.push(record.to_vec());
    }

    // GNU sort always adds trailing delimiter to output
    Ok((records, true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_read_records_newline() {
        let input = b"a\nb\nc\n";
        let (records, _) = read_all_records(Cursor::new(input), b'\n').unwrap();
        assert_eq!(records, vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
    }

    #[test]
    fn test_read_records_no_trailing_newline() {
        let input = b"a\nb\nc";
        let (records, _) = read_all_records(Cursor::new(input), b'\n').unwrap();
        assert_eq!(records, vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
    }

    #[test]
    fn test_read_records_nul_delimiter() {
        let input = b"a\0b\0c\0";
        let (records, _) = read_all_records(Cursor::new(input), 0u8).unwrap();
        assert_eq!(records, vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
    }

    #[test]
    fn test_empty_records() {
        let input = b"\n\na\n\n";
        let (records, _) = read_all_records(Cursor::new(input), b'\n').unwrap();
        assert_eq!(
            records,
            vec![b"".to_vec(), b"".to_vec(), b"a".to_vec(), b"".to_vec()]
        );
    }
}
