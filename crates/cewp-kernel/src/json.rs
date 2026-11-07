//! Clean, self-contained JSON parser and AST in pure Rust std.
//!
//! Capable of parsing `transitions.json`, `governance_policy.json`, and `mesh_story_trace.json`.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Object(entries) => {
                for (k, v) in entries {
                    if k == key {
                        return Some(v);
                    }
                }
                None
            }
            _ => None,
        }
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut JsonValue> {
        match self {
            JsonValue::Object(entries) => {
                for (k, v) in entries {
                    if k == key {
                        return Some(v);
                    }
                }
                None
            }
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            JsonValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            JsonValue::Number(n) if *n >= 0.0 => Some(*n as u64),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            JsonValue::Number(n) => Some(*n as i64),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            JsonValue::Array(a) => Some(a.as_slice()),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&[(String, JsonValue)]> {
        match self {
            JsonValue::Object(o) => Some(o.as_slice()),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, JsonValue::Null)
    }

    pub fn to_json_string(&self) -> String {
        let mut out = String::new();
        self.write_json(&mut out);
        out
    }

    fn write_json(&self, out: &mut String) {
        match self {
            JsonValue::Null => out.push_str("null"),
            JsonValue::Bool(true) => out.push_str("true"),
            JsonValue::Bool(false) => out.push_str("false"),
            JsonValue::Number(n) => {
                if n.fract() == 0.0 && *n >= (i64::MIN as f64) && *n <= (i64::MAX as f64) {
                    out.push_str(&format!("{}", *n as i64));
                } else {
                    out.push_str(&format!("{}", n));
                }
            }
            JsonValue::String(s) => {
                out.push('"');
                for ch in s.chars() {
                    match ch {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\x08' => out.push_str("\\b"),
                        '\x0C' => out.push_str("\\f"),
                        '\n' => out.push_str("\\n"),
                        '\r' => out.push_str("\\r"),
                        '\t' => out.push_str("\\t"),
                        c if (c as u32) < 0x20 => {
                            out.push_str(&format!("\\u{:04x}", c as u32));
                        }
                        c => out.push(c),
                    }
                }
                out.push('"');
            }
            JsonValue::Array(arr) => {
                out.push('[');
                for (i, v) in arr.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    v.write_json(out);
                }
                out.push(']');
            }
            JsonValue::Object(entries) => {
                out.push('{');
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push('"');
                    out.push_str(k);
                    out.push_str("\":");
                    v.write_json(out);
                }
                out.push('}');
            }
        }
    }
}

impl fmt::Display for JsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_json_string())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub msg: String,
    pub offset: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON parse error at offset {}: {}", self.offset, self.msg)
    }
}

impl std::error::Error for ParseError {}

pub fn parse(input: &str) -> Result<JsonValue, ParseError> {
    let mut parser = Parser {
        bytes: input.as_bytes(),
        pos: 0,
    };
    let val = parser.parse_value()?;
    parser.skip_whitespace();
    if parser.pos != parser.bytes.len() {
        return Err(ParseError {
            msg: format!("Trailing data at offset {}", parser.pos),
            offset: parser.pos,
        });
    }
    Ok(val)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn skip_whitespace(&mut self) {
        while self.pos < self.bytes.len() && matches!(self.bytes[self.pos], b' ' | b'\t' | b'\n' | b'\r') {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        if self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            self.pos += 1;
            Some(b)
        } else {
            None
        }
    }

    fn parse_value(&mut self) -> Result<JsonValue, ParseError> {
        self.skip_whitespace();
        let b = self.peek().ok_or_else(|| ParseError {
            msg: "Unexpected end of input".to_string(),
            offset: self.pos,
        })?;

        match b {
            b'n' => self.parse_null(),
            b't' | b'f' => self.parse_bool(),
            b'"' => self.parse_string().map(JsonValue::String),
            b'[' => self.parse_array(),
            b'{' => self.parse_object(),
            b'-' | b'0'..=b'9' => self.parse_number(),
            other => Err(ParseError {
                msg: format!("Unexpected byte '{}'", other as char),
                offset: self.pos,
            }),
        }
    }

    fn parse_null(&mut self) -> Result<JsonValue, ParseError> {
        if self.pos + 4 <= self.bytes.len() && &self.bytes[self.pos..self.pos + 4] == b"null" {
            self.pos += 4;
            Ok(JsonValue::Null)
        } else {
            Err(ParseError {
                msg: "Expected 'null'".to_string(),
                offset: self.pos,
            })
        }
    }

    fn parse_bool(&mut self) -> Result<JsonValue, ParseError> {
        if self.pos + 4 <= self.bytes.len() && &self.bytes[self.pos..self.pos + 4] == b"true" {
            self.pos += 4;
            Ok(JsonValue::Bool(true))
        } else if self.pos + 5 <= self.bytes.len() && &self.bytes[self.pos..self.pos + 5] == b"false" {
            self.pos += 5;
            Ok(JsonValue::Bool(false))
        } else {
            Err(ParseError {
                msg: "Expected boolean".to_string(),
                offset: self.pos,
            })
        }
    }

    fn parse_string(&mut self) -> Result<String, ParseError> {
        if self.advance() != Some(b'"') {
            return Err(ParseError {
                msg: "Expected '\"'".to_string(),
                offset: self.pos,
            });
        }

        let mut s = String::new();
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            self.pos += 1;
            match b {
                b'"' => return Ok(s),
                b'\\' => {
                    let esc = self.advance().ok_or_else(|| ParseError {
                        msg: "Unterminated string escape".to_string(),
                        offset: self.pos,
                    })?;
                    match esc {
                        b'"' => s.push('"'),
                        b'\\' => s.push('\\'),
                        b'/' => s.push('/'),
                        b'b' => s.push('\x08'),
                        b'f' => s.push('\x0C'),
                        b'n' => s.push('\n'),
                        b'r' => s.push('\r'),
                        b't' => s.push('\t'),
                        b'u' => {
                            let cp = self.parse_hex_u16()?;
                            if (0xD800..=0xDBFF).contains(&cp) {
                                // High surrogate, check for low surrogate \uDC00..=0xDFFF
                                if self.pos + 6 <= self.bytes.len()
                                    && self.bytes[self.pos] == b'\\'
                                    && self.bytes[self.pos + 1] == b'u'
                                {
                                    self.pos += 2;
                                    let low = self.parse_hex_u16()?;
                                    if (0xDC00..=0xDFFF).contains(&low) {
                                        let full_cp = 0x10000 + (((cp as u32 - 0xD800) << 10) | (low as u32 - 0xDC00));
                                        if let Some(ch) = char::from_u32(full_cp) {
                                            s.push(ch);
                                        }
                                        continue;
                                    }
                                }
                            }
                            if let Some(ch) = char::from_u32(cp as u32) {
                                s.push(ch);
                            }
                        }
                        other => {
                            return Err(ParseError {
                                msg: format!("Unknown string escape: \\{}", other as char),
                                offset: self.pos,
                            });
                        }
                    }
                }
                _ => {
                    // Collect UTF-8 sequence
                    let len = if b & 0x80 == 0 {
                        1
                    } else if b & 0xE0 == 0xC0 {
                        2
                    } else if b & 0xF0 == 0xE0 {
                        3
                    } else if b & 0xF8 == 0xF0 {
                        4
                    } else {
                        1
                    };
                    let start = self.pos - 1;
                    let end = (start + len).min(self.bytes.len());
                    self.pos = end;
                    if let Ok(slice_str) = std::str::from_utf8(&self.bytes[start..end]) {
                        s.push_str(slice_str);
                    }
                }
            }
        }

        Err(ParseError {
            msg: "Unterminated string".to_string(),
            offset: self.pos,
        })
    }

    fn parse_hex_u16(&mut self) -> Result<u16, ParseError> {
        if self.pos + 4 > self.bytes.len() {
            return Err(ParseError {
                msg: "Incomplete \\u hex escape".to_string(),
                offset: self.pos,
            });
        }
        let hex_slice = &self.bytes[self.pos..self.pos + 4];
        self.pos += 4;
        let hex_str = std::str::from_utf8(hex_slice).map_err(|_| ParseError {
            msg: "Invalid UTF-8 in hex escape".to_string(),
            offset: self.pos,
        })?;
        u16::from_str_radix(hex_str, 16).map_err(|_| ParseError {
            msg: format!("Invalid hex digits in \\u escape: {}", hex_str),
            offset: self.pos,
        })
    }

    fn parse_number(&mut self) -> Result<JsonValue, ParseError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        while let Some(b) = self.peek() {
            if b.is_ascii_digit() || b == b'.' || b == b'e' || b == b'E' || b == b'+' || b == b'-' {
                self.pos += 1;
            } else {
                break;
            }
        }
        let s = std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|_| ParseError {
            msg: "Invalid UTF-8 in number".to_string(),
            offset: start,
        })?;
        let n: f64 = s.parse().map_err(|_| ParseError {
            msg: format!("Invalid number literal: '{}'", s),
            offset: start,
        })?;
        Ok(JsonValue::Number(n))
    }

    fn parse_array(&mut self) -> Result<JsonValue, ParseError> {
        if self.advance() != Some(b'[') {
            return Err(ParseError {
                msg: "Expected '['".to_string(),
                offset: self.pos,
            });
        }
        let mut arr = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(JsonValue::Array(arr));
        }

        loop {
            let val = self.parse_value()?;
            arr.push(val);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                    self.skip_whitespace();
                    if self.peek() == Some(b']') {
                        // Allow trailing comma
                        self.pos += 1;
                        break;
                    }
                }
                Some(b']') => {
                    self.pos += 1;
                    break;
                }
                other => {
                    return Err(ParseError {
                        msg: format!("Expected ',' or ']' in array, got {:?}", other.map(|c| c as char)),
                        offset: self.pos,
                    });
                }
            }
        }
        Ok(JsonValue::Array(arr))
    }

    fn parse_object(&mut self) -> Result<JsonValue, ParseError> {
        if self.advance() != Some(b'{') {
            return Err(ParseError {
                msg: "Expected '{'".to_string(),
                offset: self.pos,
            });
        }
        let mut entries = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(JsonValue::Object(entries));
        }

        loop {
            self.skip_whitespace();
            if self.peek() != Some(b'"') {
                return Err(ParseError {
                    msg: "Expected string key in object".to_string(),
                    offset: self.pos,
                });
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.advance() != Some(b':') {
                return Err(ParseError {
                    msg: "Expected ':' after object key".to_string(),
                    offset: self.pos,
                });
            }
            let val = self.parse_value()?;
            entries.push((key, val));
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                    self.skip_whitespace();
                    if self.peek() == Some(b'}') {
                        // Allow trailing comma
                        self.pos += 1;
                        break;
                    }
                }
                Some(b'}') => {
                    self.pos += 1;
                    break;
                }
                other => {
                    return Err(ParseError {
                        msg: format!("Expected ',' or '}}' in object, got {:?}", other.map(|c| c as char)),
                        offset: self.pos,
                    });
                }
            }
        }
        Ok(JsonValue::Object(entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basics() {
        let v = parse("{\"a\": 1, \"b\": [true, false, null, \"hello\\nworld\"]}").unwrap();
        assert_eq!(v.get("a").and_then(|x| x.as_i64()), Some(1));
        let b = v.get("b").and_then(|x| x.as_array()).unwrap();
        assert_eq!(b.len(), 4);
        assert_eq!(b[0].as_bool(), Some(true));
        assert_eq!(b[1].as_bool(), Some(false));
        assert!(b[2].is_null());
        assert_eq!(b[3].as_str(), Some("hello\nworld"));
    }
}
