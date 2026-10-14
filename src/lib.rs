//! tabkit: streaming CSV reading and writing, plus a small conversion CLI.
//!
//! ```
//! use tabkit::{Reader, ReaderOptions};
//!
//! let data = "name,qty\napple,3\npear,5\n";
//! let mut reader = Reader::new(data.as_bytes(), ReaderOptions::default());
//! assert_eq!(reader.headers().unwrap().unwrap(), ["name", "qty"]);
//! let rows: Vec<_> = reader.map(|r| r.unwrap().fields).collect();
//! assert_eq!(rows, vec![vec!["apple", "3"], vec!["pear", "5"]]);
//! ```

pub mod error;
pub mod json;
pub mod reader;
pub mod writer;

pub use error::{CsvError, ErrorKind};
pub use reader::{Reader, ReaderOptions, Record};
pub use writer::Writer;
