use crate::compare::compare_records;
use crate::config::Config;

/// Sort records according to configuration
///
/// Uses stable sort when -s or -u is specified (to preserve input order for equals).
/// Otherwise uses unstable sort (faster, no scratch allocation).
pub fn sort_records(records: &mut [Vec<u8>], config: &Config) {
    if config.use_stable_sort() {
        records.sort_by(|a, b| compare_records(a, b, config));
    } else {
        records.sort_unstable_by(|a, b| compare_records(a, b, config));
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
    fn test_basic_sort() {
        let mut records: Vec<Vec<u8>> = vec![b"c".to_vec(), b"a".to_vec(), b"b".to_vec()];
        let config = test_config();
        sort_records(&mut records, &config);
        assert_eq!(records, vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
    }

    #[test]
    fn test_numeric_sort() {
        let mut records: Vec<Vec<u8>> = vec![b"10".to_vec(), b"2".to_vec(), b"1".to_vec()];
        let mut config = test_config();
        config.numeric = true;
        sort_records(&mut records, &config);
        assert_eq!(records, vec![b"1".to_vec(), b"2".to_vec(), b"10".to_vec()]);
    }

    #[test]
    fn test_reverse_sort() {
        let mut records: Vec<Vec<u8>> = vec![b"a".to_vec(), b"c".to_vec(), b"b".to_vec()];
        let mut config = test_config();
        config.reverse = true;
        sort_records(&mut records, &config);
        assert_eq!(records, vec![b"c".to_vec(), b"b".to_vec(), b"a".to_vec()]);
    }

    #[test]
    fn test_fold_case_sort() {
        let mut records: Vec<Vec<u8>> = vec![b"B".to_vec(), b"a".to_vec(), b"A".to_vec(), b"b".to_vec()];
        let mut config = test_config();
        config.fold_case = true;
        sort_records(&mut records, &config);
        // With -f and last-resort: A < B < a < b (uppercase before lowercase)
        // Because A == a on key, last-resort sees A (0x41) < a (0x61)
        assert_eq!(
            records,
            vec![b"A".to_vec(), b"a".to_vec(), b"B".to_vec(), b"b".to_vec()]
        );
    }

    #[test]
    fn test_key_sort() {
        let mut records: Vec<Vec<u8>> = vec![
            b"x 3".to_vec(),
            b"y 1".to_vec(),
            b"z 2".to_vec(),
        ];
        let mut config = test_config();
        config.numeric = true;
        config.keys = vec![KeySpec::parse("2,2").unwrap()];
        sort_records(&mut records, &config);
        assert_eq!(
            records,
            vec![b"y 1".to_vec(), b"z 2".to_vec(), b"x 3".to_vec()]
        );
    }
}
