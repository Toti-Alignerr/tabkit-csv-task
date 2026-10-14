//! `tabkit` command line tool.
//!
//! ```text
//! tabkit json   [--delimiter C] [--no-headers] [--flexible] [--skip-errors] <FILE|->
//! tabkit count  [--delimiter C] [--no-headers] [--flexible] [--skip-errors] <FILE|->
//! tabkit select --columns NAME[,NAME...] [--delimiter C] [--skip-errors] <FILE|->
//! ```

use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::process::ExitCode;

use tabkit::{json, CsvError, Reader, ReaderOptions, Writer};

const USAGE: &str = "usage:
  tabkit json   [--delimiter C] [--no-headers] [--flexible] [--skip-errors] <FILE|->
  tabkit count  [--delimiter C] [--no-headers] [--flexible] [--skip-errors] <FILE|->
  tabkit select --columns NAME[,NAME...] [--delimiter C] [--skip-errors] <FILE|->";

struct Args {
    command: String,
    path: String,
    options: ReaderOptions,
    skip_errors: bool,
    columns: Option<Vec<String>>,
}

fn parse_args(raw: Vec<String>) -> Result<Args, String> {
    let mut iter = raw.into_iter();
    let command = iter.next().ok_or("missing command")?;
    if !matches!(command.as_str(), "json" | "count" | "select") {
        return Err(format!("unknown command {command:?}"));
    }
    let mut options = ReaderOptions::default();
    let mut skip_errors = false;
    let mut columns = None;
    let mut path = None;
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--delimiter" => {
                let value = iter.next().ok_or("--delimiter needs a value")?;
                let value = if value == "\\t" { "\t".to_string() } else { value };
                let bytes = value.as_bytes();
                if bytes.len() != 1 || !bytes[0].is_ascii() || matches!(bytes[0], b'"' | b'\r' | b'\n') {
                    return Err(format!("invalid delimiter {value:?}"));
                }
                options.delimiter = bytes[0];
            }
            "--no-headers" => options.has_headers = false,
            "--flexible" => options.flexible = true,
            "--skip-errors" => skip_errors = true,
            "--columns" => {
                let value = iter.next().ok_or("--columns needs a value")?;
                columns = Some(value.split(',').map(str::to_string).collect());
            }
            other if other.starts_with("--") => return Err(format!("unknown option {other}")),
            other => {
                if path.replace(other.to_string()).is_some() {
                    return Err("only one input file is allowed".into());
                }
            }
        }
    }
    let path = path.ok_or("missing input file")?;
    match command.as_str() {
        "select" if columns.is_none() => return Err("select needs --columns".into()),
        "select" if !options.has_headers || options.flexible => {
            return Err("select requires headers and does not support --flexible".into())
        }
        "json" if options.has_headers && options.flexible => {
            return Err("--flexible is only supported with --no-headers for json".into())
        }
        "json" | "count" if columns.is_some() => return Err("--columns is only valid for select".into()),
        _ => {}
    }
    Ok(Args { command, path, options, skip_errors, columns })
}

fn open(path: &str) -> io::Result<Box<dyn BufRead>> {
    if path == "-" {
        Ok(Box::new(BufReader::new(io::stdin())))
    } else {
        Ok(Box::new(BufReader::new(File::open(path)?)))
    }
}

/// Error policy: by default the first error aborts with exit code 1;
/// with --skip-errors errors are reported on stderr and processing continues.
fn handle(err: CsvError, skip: bool) -> Result<(), CsvError> {
    if skip {
        eprintln!("tabkit: skipped: {err}");
        Ok(())
    } else {
        Err(err)
    }
}

fn run(args: Args) -> Result<(), String> {
    let input = open(&args.path).map_err(|e| format!("{}: {e}", args.path))?;
    let mut reader = Reader::new(input, args.options.clone());
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let headers: Option<Vec<String>> = reader.headers().map_err(|e| e.to_string())?.map(|h| h.to_vec());

    match args.command.as_str() {
        "json" => {
            for result in reader {
                match result {
                    Ok(record) => {
                        let line = match &headers {
                            Some(h) => json::object(h, &record.fields),
                            None => json::array(&record.fields),
                        };
                        writeln!(out, "{line}").map_err(|e| e.to_string())?;
                    }
                    Err(err) => handle(err, args.skip_errors).map_err(|e| e.to_string())?,
                }
            }
        }
        "count" => {
            let mut count = 0u64;
            for result in reader {
                match result {
                    Ok(_) => count += 1,
                    Err(err) => handle(err, args.skip_errors).map_err(|e| e.to_string())?,
                }
            }
            writeln!(out, "{count}").map_err(|e| e.to_string())?;
        }
        "select" => {
            let headers = headers.unwrap_or_default();
            let wanted = args.columns.unwrap_or_default();
            let mut indexes = Vec::new();
            for name in &wanted {
                let index = headers.iter().position(|h| h == name).ok_or(format!("unknown column {name:?}"))?;
                indexes.push(index);
            }
            let mut writer = Writer::new(&mut out, args.options.delimiter);
            writer.write_record(&wanted).map_err(|e| e.to_string())?;
            for result in reader {
                match result {
                    Ok(record) => {
                        let row: Vec<&str> = indexes.iter().map(|&i| record.fields[i].as_str()).collect();
                        writer.write_record(&row).map_err(|e| e.to_string())?;
                    }
                    Err(err) => handle(err, args.skip_errors).map_err(|e| e.to_string())?,
                }
            }
        }
        _ => unreachable!(),
    }
    out.flush().map_err(|e| e.to_string())
}

fn main() -> ExitCode {
    let args = match parse_args(std::env::args().skip(1).collect()) {
        Ok(args) => args,
        Err(message) => {
            eprintln!("tabkit: {message}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("tabkit: {message}");
            ExitCode::from(1)
        }
    }
}
