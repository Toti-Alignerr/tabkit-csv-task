//! Streaming CSV reader.

use std::io::BufRead;

use crate::error::{CsvError, ErrorKind};

/// Reader configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReaderOptions {
    /// Field separator. Must be an ASCII byte other than `"`, `\r` or `\n`.
    pub delimiter: u8,
    /// Whether the first record is a header row.
    pub has_headers: bool,
    /// Allow records with differing field counts.
    pub flexible: bool,
}

impl Default for ReaderOptions {
    fn default() -> Self {
        ReaderOptions { delimiter: b',', has_headers: true, flexible: false }
    }
}

/// One data record and the 1-based input line it starts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub fields: Vec<String>,
    pub line: u64,
}

/// Reads CSV records from any [`BufRead`], one record at a time.
///
/// Iterating yields data records only; call [`Reader::headers`] to get the
/// header row when `has_headers` is set.
pub struct Reader<R> {
    inner: R,
    options: ReaderOptions,
    line: u64,
    headers: Option<Vec<String>>,
    headers_read: bool,
}

impl<R: BufRead> Reader<R> {
    pub fn new(inner: R, options: ReaderOptions) -> Self {
        assert!(
            options.delimiter.is_ascii() && !matches!(options.delimiter, b'"' | b'\r' | b'\n'),
            "invalid delimiter"
        );
        Reader { inner, options, line: 0, headers: None, headers_read: false }
    }

    pub fn from_reader(inner: R) -> Self {
        Self::new(inner, ReaderOptions::default())
    }

    pub fn options(&self) -> &ReaderOptions {
        &self.options
    }

    /// Returns the header row, reading it if necessary. `Ok(None)` when the
    /// reader has no headers or the input is empty.
    pub fn headers(&mut self) -> Result<Option<&[String]>, CsvError> {
        if !self.headers_read {
            self.headers_read = true;
            if self.options.has_headers {
                if let Some((_, fields)) = self.read_record()? {
                    self.headers = Some(fields);
                }
            }
        }
        Ok(self.headers.as_deref())
    }

    /// Reads the next non-empty physical line, without its terminator.
    fn read_line(&mut self) -> Result<Option<(u64, String)>, CsvError> {
        loop {
            let mut buf = Vec::new();
            let read = self
                .inner
                .read_until(b'\n', &mut buf)
                .map_err(|e| CsvError::new(ErrorKind::Io(e.to_string()), self.line + 1))?;
            if read == 0 {
                return Ok(None);
            }
            self.line += 1;
            while matches!(buf.last(), Some(b'\n') | Some(b'\r')) {
                buf.pop();
            }
            if buf.is_empty() {
                continue;
            }
            let text = String::from_utf8(buf).map_err(|_| CsvError::new(ErrorKind::InvalidUtf8, self.line))?;
            return Ok(Some((self.line, text)));
        }
    }

    fn split(&self, line: &str) -> Vec<String> {
        line.split(self.options.delimiter as char)
            .map(|field| {
                if field.len() >= 2 && field.starts_with('"') && field.ends_with('"') {
                    field[1..field.len() - 1].replace("\"\"", "\"")
                } else {
                    field.to_string()
                }
            })
            .collect()
    }

    fn read_record(&mut self) -> Result<Option<(u64, Vec<String>)>, CsvError> {
        Ok(self.read_line()?.map(|(line, text)| (line, self.split(&text))))
    }
}

impl<R: BufRead> Iterator for Reader<R> {
    type Item = Result<Record, CsvError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Err(err) = self.headers() {
            return Some(Err(err));
        }
        let (line, fields) = match self.read_record() {
            Ok(Some(record)) => record,
            Ok(None) => return None,
            Err(err) => return Some(Err(err)),
        };
        if !self.options.flexible {
            if let Some(headers) = &self.headers {
                if headers.len() != fields.len() {
                    let kind = ErrorKind::FieldCount { expected: headers.len(), found: fields.len() };
                    return Some(Err(CsvError::new(kind, line)));
                }
            }
        }
        Some(Ok(Record { fields, line }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(input: &str, options: ReaderOptions) -> Vec<Vec<String>> {
        Reader::new(input.as_bytes(), options).map(|r| r.unwrap().fields).collect()
    }

    #[test]
    fn reads_headers_and_records() {
        let mut reader = Reader::from_reader("a,b\n1,2\n3,4\n".as_bytes());
        assert_eq!(reader.headers().unwrap().unwrap(), ["a", "b"]);
        let records: Vec<Record> = reader.map(Result::unwrap).collect();
        assert_eq!(records[0], Record { fields: vec!["1".into(), "2".into()], line: 2 });
        assert_eq!(records[1].line, 3);
    }

    #[test]
    fn without_headers_every_line_is_a_record() {
        let options = ReaderOptions { has_headers: false, ..Default::default() };
        assert_eq!(rows("x,y\n1,2", options), vec![vec!["x", "y"], vec!["1", "2"]]);
    }

    #[test]
    fn handles_crlf_and_blank_lines() {
        let options = ReaderOptions { has_headers: false, ..Default::default() };
        assert_eq!(rows("a,b\r\n\r\nc,d\r\n", options), vec![vec!["a", "b"], vec!["c", "d"]]);
    }

    #[test]
    fn simple_quoted_fields() {
        let options = ReaderOptions { has_headers: false, ..Default::default() };
        assert_eq!(rows("\"a\",\"say \"\"hi\"\"\",c\n", options), vec![vec!["a", "say \"hi\"", "c"]]);
    }

    #[test]
    fn custom_delimiter_and_spaces_preserved() {
        let options = ReaderOptions { delimiter: b';', has_headers: false, ..Default::default() };
        assert_eq!(rows(" a ; b,c \n", options), vec![vec![" a ", " b,c "]]);
    }

    #[test]
    fn field_count_mismatch_with_headers() {
        let mut reader = Reader::from_reader("a,b\n1,2\n3\n4,5\n".as_bytes());
        let results: Vec<_> = reader.by_ref().collect();
        assert!(results[0].is_ok());
        assert_eq!(
            results[1].clone().unwrap_err(),
            CsvError::new(ErrorKind::FieldCount { expected: 2, found: 1 }, 3)
        );
        assert_eq!(results[2].clone().unwrap().fields, vec!["4", "5"]);
    }

    #[test]
    fn flexible_allows_ragged_rows() {
        let options = ReaderOptions { flexible: true, ..Default::default() };
        assert_eq!(rows("a,b\n1\n1,2,3\n", options), vec![vec!["1"], vec!["1", "2", "3"]]);
    }

    #[test]
    fn empty_input() {
        let mut reader = Reader::from_reader("".as_bytes());
        assert_eq!(reader.headers().unwrap(), None);
        assert!(reader.next().is_none());
    }

    #[test]
    fn invalid_utf8_is_reported() {
        let options = ReaderOptions { has_headers: false, ..Default::default() };
        let bytes: &[u8] = b"ok\n\xff\xfe\n";
        let results: Vec<_> = Reader::new(bytes, options).collect();
        assert_eq!(results[1].clone().unwrap_err(), CsvError::new(ErrorKind::InvalidUtf8, 2));
    }
}
