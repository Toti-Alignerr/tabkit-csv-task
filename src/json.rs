//! Minimal JSON encoding for string tables.

/// Encodes `value` as a JSON string literal, including the quotes.
pub fn string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Encodes a list of strings as a JSON array.
pub fn array<S: AsRef<str>>(values: &[S]) -> String {
    let items: Vec<String> = values.iter().map(|v| string(v.as_ref())).collect();
    format!("[{}]", items.join(","))
}

/// Encodes key/value pairs as a JSON object, preserving order.
pub fn object<K: AsRef<str>, V: AsRef<str>>(keys: &[K], values: &[V]) -> String {
    let items: Vec<String> = keys
        .iter()
        .zip(values)
        .map(|(k, v)| format!("{}:{}", string(k.as_ref()), string(v.as_ref())))
        .collect();
    format!("{{{}}}", items.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_special_characters() {
        assert_eq!(string("a\"b\\c\nd\te\u{1}"), "\"a\\\"b\\\\c\\nd\\te\\u0001\"");
        assert_eq!(string("ñandú 🦤"), "\"ñandú 🦤\"");
    }

    #[test]
    fn builds_arrays_and_objects() {
        assert_eq!(array(&["a", "b"]), "[\"a\",\"b\"]");
        assert_eq!(object(&["k1", "k2"], &["v1", "v2"]), "{\"k1\":\"v1\",\"k2\":\"v2\"}");
    }
}
