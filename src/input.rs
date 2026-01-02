use std::io::{self, BufRead};

/// Reads records from input, splitting on the specified delimiter
pub struct RecordReader<R> {
    reader: R,
    delimiter: u8,
    buffer: Vec<u8>,
}

impl<R: BufRead> RecordReader<R> {
    pub fn new(reader: R, delimiter: u8) -> Self {
        Self {
            reader,
            delimiter,
            buffer: Vec::new(),
        }
    }

    /// Read the next record, returning None at EOF
    pub fn read_record(&mut self) -> io::Result<Option<&[u8]>> {
        self.buffer.clear();
        let bytes_read = self.reader.read_until(self.delimiter, &mut self.buffer)?;

        if bytes_read == 0 {
            return Ok(None);
        }

        // Strip delimiter if present at end
        if self.buffer.last() == Some(&self.delimiter) {
            self.buffer.pop();
        }

        Ok(Some(&self.buffer))
    }
}

/// Read all records from a reader into a Vec
pub fn read_all_records<R: BufRead>(reader: R, delimiter: u8) -> io::Result<Vec<Vec<u8>>> {
    let mut records = Vec::new();
    let mut rec_reader = RecordReader::new(reader, delimiter);

    while let Some(record) = rec_reader.read_record()? {
        records.push(record.to_vec());
    }

    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_read_records_newline() {
        let input = b"a\nb\nc\n";
        let records = read_all_records(Cursor::new(input), b'\n').unwrap();
        assert_eq!(records, vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
    }

    #[test]
    fn test_read_records_no_trailing_newline() {
        let input = b"a\nb\nc";
        let records = read_all_records(Cursor::new(input), b'\n').unwrap();
        assert_eq!(records, vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
    }

    #[test]
    fn test_read_records_nul_delimiter() {
        let input = b"a\0b\0c\0";
        let records = read_all_records(Cursor::new(input), 0u8).unwrap();
        assert_eq!(records, vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
    }

    #[test]
    fn test_empty_records() {
        let input = b"\n\na\n\n";
        let records = read_all_records(Cursor::new(input), b'\n').unwrap();
        assert_eq!(
            records,
            vec![b"".to_vec(), b"".to_vec(), b"a".to_vec(), b"".to_vec()]
        );
    }
}
