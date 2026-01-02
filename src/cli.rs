use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "rsort", about = "Sort lines of text")]
pub struct Args {
    /// Reverse the result of comparisons
    #[arg(short = 'r', long)]
    pub reverse: bool,

    /// Compare according to numerical value
    #[arg(short = 'n', long = "numeric-sort")]
    pub numeric: bool,

    /// Fold lower case to upper case characters
    #[arg(short = 'f', long = "ignore-case")]
    pub fold_case: bool,

    /// Output only unique lines
    #[arg(short = 'u', long)]
    pub unique: bool,

    /// Stabilize sort by disabling last-resort comparison
    #[arg(short = 's', long)]
    pub stable: bool,

    /// Write result to FILE instead of stdout
    #[arg(short = 'o', long, value_name = "FILE")]
    pub output: Option<String>,

    /// Use SEP as field separator
    #[arg(short = 't', long = "field-separator", value_name = "SEP")]
    pub delimiter: Option<String>,

    /// Sort by key specification
    #[arg(short = 'k', long = "key", value_name = "KEYDEF")]
    pub keys: Vec<String>,

    /// Use NUL as line delimiter
    #[arg(short = 'z', long = "zero-terminated")]
    pub zero_terminated: bool,

    /// Annotate the part of the line used to sort
    #[arg(long)]
    pub debug: bool,

    /// Input files
    #[arg(value_name = "FILE")]
    pub files: Vec<String>,
}

impl Args {
    /// Parse -t argument, handling '\0' escape for NUL byte
    pub fn field_separator(&self) -> crate::error::Result<Option<u8>> {
        match &self.delimiter {
            None => Ok(None),
            Some(s) => {
                if s == "\\0" || s == "\0" {
                    Ok(Some(0u8))
                } else if s.len() == 1 {
                    Ok(Some(s.as_bytes()[0]))
                } else if s.starts_with('\\') && s.len() == 2 {
                    // Handle other escapes like \t
                    match s.chars().nth(1) {
                        Some('t') => Ok(Some(b'\t')),
                        Some('n') => Ok(Some(b'\n')),
                        Some('\\') => Ok(Some(b'\\')),
                        Some('0') => Ok(Some(0u8)),
                        _ => Err(crate::error::RsortError::InvalidDelimiter),
                    }
                } else {
                    Err(crate::error::RsortError::InvalidDelimiter)
                }
            }
        }
    }

    /// Get the record delimiter (newline or NUL)
    pub fn record_delimiter(&self) -> u8 {
        if self.zero_terminated {
            0u8
        } else {
            b'\n'
        }
    }
}
