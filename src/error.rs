use std::fmt;

/// What went wrong while reading CSV input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// A quoted field was still open at the end of the input.
    UnterminatedQuote,
    /// A `"` appeared where RFC 4180 does not allow one: inside an unquoted
    /// field, or after the closing quote of a quoted field.
    InvalidQuote,
    /// A record had a different number of fields than expected.
    FieldCount { expected: usize, found: usize },
    /// The input was not valid UTF-8.
    InvalidUtf8,
    /// The underlying reader failed.
    Io(String),
}

/// An error with the 1-based input line it refers to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvError {
    pub kind: ErrorKind,
    pub line: u64,
}

impl CsvError {
    pub fn new(kind: ErrorKind, line: u64) -> Self {
        CsvError { kind, line }
    }
}

impl fmt::Display for CsvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: ", self.line)?;
        match &self.kind {
            ErrorKind::UnterminatedQuote => write!(f, "unterminated quoted field"),
            ErrorKind::InvalidQuote => write!(f, "unexpected quote character"),
            ErrorKind::FieldCount { expected, found } => {
                write!(f, "expected {expected} fields, found {found}")
            }
            ErrorKind::InvalidUtf8 => write!(f, "invalid UTF-8"),
            ErrorKind::Io(message) => write!(f, "I/O error: {message}"),
        }
    }
}

impl std::error::Error for CsvError {}
