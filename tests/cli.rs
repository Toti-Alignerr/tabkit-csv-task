use std::io::Write;
use std::process::{Command, Output, Stdio};

fn tabkit(args: &[&str], stdin: Option<&str>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tabkit"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tabkit");
    if let Some(input) = stdin {
        child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    }
    drop(child.stdin.take());
    child.wait_with_output().unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

#[test]
fn json_with_headers_from_file() {
    let out = tabkit(&["json", "tests/fixtures/inventory.csv"], None);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let lines: Vec<String> = stdout(&out).lines().map(str::to_string).collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], r#"{"sku":"A-1","name":"Widget","qty":"12","location":"Shelf 3"}"#);
    assert_eq!(lines[2], r#"{"sku":"C-2","name":"Sprocket","qty":"0","location":"Shelf 1"}"#);
}

#[test]
fn json_without_headers_from_stdin() {
    let out = tabkit(&["json", "--no-headers", "--delimiter", ";", "-"], Some("a;b\n1;2\n"));
    assert!(out.status.success());
    assert_eq!(stdout(&out), "[\"a\",\"b\"]\n[\"1\",\"2\"]\n");
}

#[test]
fn count_records() {
    let out = tabkit(&["count", "-"], Some("h1,h2\nx,y\n\nz,w\n"));
    assert!(out.status.success());
    assert_eq!(stdout(&out), "2\n");
    let out = tabkit(&["count", "--no-headers", "--delimiter", "\\t", "-"], Some("a\tb\nc\td\n"));
    assert_eq!(stdout(&out), "2\n");
}

#[test]
fn select_columns() {
    let out = tabkit(&["select", "--columns", "qty,sku", "-"], Some("sku,name,qty\nA,x,1\nB,y,2\n"));
    assert!(out.status.success());
    assert_eq!(stdout(&out), "qty,sku\n1,A\n2,B\n");
}

#[test]
fn errors_abort_unless_skipped() {
    let input = "a,b\n1,2\n3\n4,5\n";
    let out = tabkit(&["count", "-"], Some(input));
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&out.stderr).trim(), "tabkit: line 3: expected 2 fields, found 1");

    let out = tabkit(&["count", "--skip-errors", "-"], Some(input));
    assert!(out.status.success());
    assert_eq!(stdout(&out), "2\n");
    assert!(String::from_utf8_lossy(&out.stderr).contains("skipped: line 3"));
}

#[test]
fn usage_errors() {
    for args in [
        vec!["frobnicate", "x.csv"],
        vec!["json"],
        vec!["json", "--delimiter", "ab", "-"],
        vec!["json", "--flexible", "-"],
        vec!["select", "-"],
        vec!["select", "--columns", "a", "--no-headers", "-"],
        vec!["count", "--bogus", "-"],
    ] {
        let out = tabkit(&args, Some(""));
        assert_eq!(out.status.code(), Some(2), "{args:?}");
    }
    let out = tabkit(&["select", "--columns", "nope", "-"], Some("a,b\n1,2\n"));
    assert_eq!(out.status.code(), Some(1));
    let out = tabkit(&["json", "missing-file.csv"], None);
    assert_eq!(out.status.code(), Some(1));
}
