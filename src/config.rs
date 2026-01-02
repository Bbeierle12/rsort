use crate::cli::Args;
use crate::error::Result;
use crate::key::KeySpec;

/// Runtime configuration derived from CLI arguments
#[derive(Clone, Debug)]
pub struct Config {
    pub reverse: bool,
    pub numeric: bool,
    pub fold_case: bool,
    pub unique: bool,
    pub stable: bool,
    pub debug: bool,
    pub record_delimiter: u8,
    pub field_separator: Option<u8>,
    pub keys: Vec<KeySpec>,
    pub output_file: Option<String>,
    pub input_files: Vec<String>,
}

impl Config {
    /// Build configuration from parsed CLI arguments
    pub fn from_args(args: &Args) -> Result<Self> {
        let keys: Result<Vec<KeySpec>> = args
            .keys
            .iter()
            .map(|s| KeySpec::parse(s))
            .collect();

        Ok(Config {
            reverse: args.reverse,
            numeric: args.numeric,
            fold_case: args.fold_case,
            unique: args.unique,
            stable: args.stable,
            debug: args.debug,
            record_delimiter: args.record_delimiter(),
            field_separator: args.field_separator()?,
            keys: keys?,
            output_file: args.output.clone(),
            input_files: args.files.clone(),
        })
    }

    /// Whether last-resort comparison is enabled
    pub fn use_last_resort(&self) -> bool {
        !self.stable && !self.unique
    }

    /// Whether to use stable sort algorithm
    pub fn use_stable_sort(&self) -> bool {
        self.stable || self.unique
    }
}
