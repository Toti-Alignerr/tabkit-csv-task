//! CSV writer.

use std::io::{self, Write};

/// Writes records as CSV lines terminated by `\n`.
pub struct Writer<W: Write> {
    inner: W,
    delimiter: u8,
}

impl<W: Write> Writer<W> {
    pub fn new(inner: W, delimiter: u8) -> Self {
        Writer { inner, delimiter }
    }

    pub fn write_record<S: AsRef<str>>(&mut self, fields: &[S]) -> io::Result<()> {
        let delimiter = (self.delimiter as char).to_string();
        let line: Vec<&str> = fields.iter().map(|f| f.as_ref()).collect();
        self.inner.write_all(line.join(&delimiter).as_bytes())?;
        self.inner.write_all(b"\n")
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }

    pub fn into_inner(self) -> W {
        self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_plain_records() {
        let mut writer = Writer::new(Vec::new(), b',');
        writer.write_record(&["a", "b c", ""]).unwrap();
        writer.write_record(&["1", "2", "3"]).unwrap();
        assert_eq!(String::from_utf8(writer.into_inner()).unwrap(), "a,b c,\n1,2,3\n");
    }

    #[test]
    fn uses_delimiter() {
        let mut writer = Writer::new(Vec::new(), b'\t');
        writer.write_record(&["x", "y"]).unwrap();
        assert_eq!(writer.into_inner(), b"x\ty\n");
    }
}
