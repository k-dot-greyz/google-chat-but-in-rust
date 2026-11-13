//! A small JSON codec. Objects keep their key order. Duplicate keys are errors.
//! `<`, `>`, and `&` are escaped so a value can sit in HTML without becoming markup.

use std::fmt::Write as _;

const MAX_DEPTH: usize = 32;
const MAX_STRING: usize = 65_536;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Number {
    Int(i64),
    /// A JSON number that is not a bare integer in i64 range. The text is the literal.
    Raw(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonError {
    pub message: String,
    pub position: usize,
}

impl JsonError {
    fn new(position: usize, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            position,
        }
    }
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        match self {
            Value::Object(pairs) => pairs.iter_mut().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Number(Number::Int(n)) => Some(*n),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Value>> {
        match self {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&[(String, Value)]> {
        match self {
            Value::Object(pairs) => Some(pairs),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn to_json(&self) -> String {
        let mut out = String::new();
        write_value(self, &mut out);
        out
    }
}

pub fn parse(input: &str) -> Result<Value, JsonError> {
    let mut parser = Parser {
        src: input.as_bytes(),
        index: 0,
        depth: 0,
    };
    parser.skip();
    if parser.eof() {
        return Err(JsonError::new(0, "expected a JSON value"));
    }
    let value = parser.value()?;
    parser.skip();
    if !parser.eof() {
        return Err(JsonError::new(
            parser.index,
            "trailing data after JSON value",
        ));
    }
    Ok(value)
}

struct Parser<'a> {
    src: &'a [u8],
    index: usize,
    depth: usize,
}

impl<'a> Parser<'a> {
    fn eof(&self) -> bool {
        self.index >= self.src.len()
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.index).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.index += 1;
        Some(byte)
    }

    fn skip(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.index += 1;
        }
    }

    fn value(&mut self) -> Result<Value, JsonError> {
        self.skip();
        match self.peek() {
            Some(b'n') => self.literal(b"null", Value::Null),
            Some(b't') => self.literal(b"true", Value::Bool(true)),
            Some(b'f') => self.literal(b"false", Value::Bool(false)),
            Some(b'"') => Ok(Value::String(self.string()?)),
            Some(b'[') => self.array(),
            Some(b'{') => self.object(),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => Err(JsonError::new(self.index, "expected a JSON value")),
            None => Err(JsonError::new(self.index, "unexpected end of JSON")),
        }
    }

    fn literal(&mut self, word: &[u8], value: Value) -> Result<Value, JsonError> {
        let start = self.index;
        for expected in word {
            if self.bump() != Some(*expected) {
                return Err(JsonError::new(start, "invalid literal"));
            }
        }
        Ok(value)
    }

    fn enter(&mut self) -> Result<(), JsonError> {
        if self.depth >= MAX_DEPTH {
            return Err(JsonError::new(self.index, "JSON nesting is deeper than 32"));
        }
        self.depth += 1;
        Ok(())
    }

    fn leave(&mut self) {
        self.depth -= 1;
    }

    fn array(&mut self) -> Result<Value, JsonError> {
        self.enter()?;
        self.bump();
        let mut items = Vec::new();
        loop {
            self.skip();
            if self.peek() == Some(b']') {
                self.bump();
                self.leave();
                return Ok(Value::Array(items));
            }
            if !items.is_empty() {
                if self.bump() != Some(b',') {
                    return Err(JsonError::new(self.index, "expected a comma in array"));
                }
                self.skip();
                if self.peek() == Some(b']') {
                    return Err(JsonError::new(self.index, "trailing comma in array"));
                }
            }
            items.push(self.value()?);
        }
    }

    fn object(&mut self) -> Result<Value, JsonError> {
        self.enter()?;
        self.bump();
        let mut pairs = Vec::new();
        loop {
            self.skip();
            if self.peek() == Some(b'}') {
                self.bump();
                self.leave();
                return Ok(Value::Object(pairs));
            }
            if !pairs.is_empty() {
                if self.bump() != Some(b',') {
                    return Err(JsonError::new(self.index, "expected a comma in object"));
                }
                self.skip();
                if self.peek() == Some(b'}') {
                    return Err(JsonError::new(self.index, "trailing comma in object"));
                }
            }
            self.skip();
            if self.peek() != Some(b'"') {
                return Err(JsonError::new(self.index, "expected an object key"));
            }
            let key = self.string()?;
            self.skip();
            if self.bump() != Some(b':') {
                return Err(JsonError::new(self.index, "expected a colon"));
            }
            let value = self.value()?;
            if pairs.iter().any(|(existing, _)| existing == &key) {
                return Err(JsonError::new(self.index, format!("duplicate key `{key}`")));
            }
            pairs.push((key, value));
        }
    }

    fn string(&mut self) -> Result<String, JsonError> {
        let start = self.index;
        if self.bump() != Some(b'"') {
            return Err(JsonError::new(start, "expected a string"));
        }
        let mut out = String::new();
        loop {
            if out.len() > MAX_STRING {
                return Err(JsonError::new(self.index, "string exceeds 64 KiB"));
            }
            match self.bump() {
                Some(b'"') => return Ok(out),
                Some(b'\\') => out.push(self.escape()?),
                Some(b) if b < 0x20 => {
                    return Err(JsonError::new(
                        self.index - 1,
                        "unescaped control character",
                    ));
                }
                Some(b) => {
                    self.index -= 1;
                    let ch = self.utf8_char()?;
                    out.push(ch);
                    let _ = b;
                }
                None => return Err(JsonError::new(start, "unterminated string")),
            }
        }
    }

    fn utf8_char(&mut self) -> Result<char, JsonError> {
        let start = self.index;
        let first = self
            .bump()
            .ok_or_else(|| JsonError::new(start, "bad utf-8"))?;
        let width = if first < 0x80 {
            1
        } else if first & 0xE0 == 0xC0 {
            2
        } else if first & 0xF0 == 0xE0 {
            3
        } else if first & 0xF8 == 0xF0 {
            4
        } else {
            return Err(JsonError::new(start, "bad utf-8"));
        };
        let mut buf = [first, 0, 0, 0];
        for slot in buf.iter_mut().take(width).skip(1) {
            let byte = self
                .bump()
                .ok_or_else(|| JsonError::new(start, "bad utf-8"))?;
            if byte & 0xC0 != 0x80 {
                return Err(JsonError::new(start, "bad utf-8"));
            }
            *slot = byte;
        }
        let text =
            std::str::from_utf8(&buf[..width]).map_err(|_| JsonError::new(start, "bad utf-8"))?;
        text.chars()
            .next()
            .ok_or_else(|| JsonError::new(start, "bad utf-8"))
    }

    fn escape(&mut self) -> Result<char, JsonError> {
        let start = self.index.saturating_sub(1);
        match self.bump() {
            Some(b'"') => Ok('"'),
            Some(b'\\') => Ok('\\'),
            Some(b'/') => Ok('/'),
            Some(b'b') => Ok('\u{0008}'),
            Some(b'f') => Ok('\u{000c}'),
            Some(b'n') => Ok('\n'),
            Some(b'r') => Ok('\r'),
            Some(b't') => Ok('\t'),
            Some(b'u') => self.unicode_escape(),
            _ => Err(JsonError::new(start, "invalid escape")),
        }
    }

    fn unicode_escape(&mut self) -> Result<char, JsonError> {
        let start = self.index.saturating_sub(2);
        let unit = self.hex4()?;
        if (0xD800..=0xDBFF).contains(&unit) {
            if self.bump() != Some(b'\\') || self.bump() != Some(b'u') {
                return Err(JsonError::new(start, "lonely surrogate"));
            }
            let low = self.hex4()?;
            if !(0xDC00..=0xDFFF).contains(&low) {
                return Err(JsonError::new(start, "lonely surrogate"));
            }
            let cp = 0x10000 + (((unit as u32 - 0xD800) << 10) | (low as u32 - 0xDC00));
            return char::from_u32(cp).ok_or_else(|| JsonError::new(start, "invalid code point"));
        }
        if (0xDC00..=0xDFFF).contains(&unit) {
            return Err(JsonError::new(start, "lonely surrogate"));
        }
        char::from_u32(unit as u32).ok_or_else(|| JsonError::new(start, "invalid code point"))
    }

    fn hex4(&mut self) -> Result<u16, JsonError> {
        let start = self.index;
        let mut value = 0u16;
        for _ in 0..4 {
            let byte = self
                .bump()
                .ok_or_else(|| JsonError::new(start, "short unicode escape"))?;
            let digit = match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => byte - b'A' + 10,
                _ => return Err(JsonError::new(start, "bad unicode escape")),
            };
            value = (value << 4) | u16::from(digit);
        }
        Ok(value)
    }

    fn number(&mut self) -> Result<Value, JsonError> {
        let start = self.index;
        if self.peek() == Some(b'-') {
            self.bump();
        }
        match self.peek() {
            Some(b'0') => {
                self.bump();
            }
            Some(b'1'..=b'9') => {
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.bump();
                }
            }
            _ => return Err(JsonError::new(start, "invalid number")),
        }
        let mut fractional = false;
        if self.peek() == Some(b'.') {
            fractional = true;
            self.bump();
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(JsonError::new(start, "invalid number"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.bump();
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            fractional = true;
            self.bump();
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.bump();
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(JsonError::new(start, "invalid number"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.bump();
            }
        }
        let raw = std::str::from_utf8(&self.src[start..self.index])
            .map_err(|_| JsonError::new(start, "invalid number"))?;
        if !fractional && let Ok(integer) = raw.parse::<i64>() {
            return Ok(Value::Number(Number::Int(integer)));
        }
        Ok(Value::Number(Number::Raw(raw.to_string())))
    }
}

fn write_value(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(Number::Int(n)) => {
            let _ = write!(out, "{n}");
        }
        Value::Number(Number::Raw(raw)) => out.push_str(raw),
        Value::String(text) => write_string(text, out),
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_value(item, out);
            }
            out.push(']');
        }
        Value::Object(pairs) => {
            out.push('{');
            for (index, (key, item)) in pairs.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_string(key, out);
                out.push(':');
                write_value(item, out);
            }
            out.push('}');
        }
    }
}

fn write_string(text: &str, out: &mut String) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{code:04x}", code = c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_object_keeps_key_order() {
        let raw = r#"{"b":1,"a":[true,null,"</script>"],"c":{"d":false}}"#;
        let value = parse(raw).unwrap();
        let again = parse(&value.to_json()).unwrap();
        assert_eq!(value, again);
        let Value::Object(pairs) = &value else {
            panic!("object");
        };
        assert_eq!(pairs[0].0, "b");
        assert_eq!(pairs[1].0, "a");
        assert!(value.to_json().contains("\\u003c"));
        assert!(!value.to_json().contains('<'));
    }

    #[test]
    fn rejects_duplicate_keys_trailing_commas_and_leading_zeros() {
        assert!(parse(r#"{"a":1,"a":2}"#).is_err());
        assert!(parse("[1,]").is_err());
        assert!(parse("01").is_err());
        assert!(parse("-01").is_err());
        assert!(parse("{\"a\":1} trailing").is_err());
    }

    #[test]
    fn rejects_deep_nesting_and_bad_escapes() {
        let deep = format!("{}{}", "[".repeat(40), "]".repeat(40));
        assert!(parse(&deep).unwrap_err().message.contains("32"));
        assert!(parse("\"\\q\"").is_err());
        assert!(parse("\"\\uD800\"").is_err());
        assert!(parse("\"unterminated").is_err());
        let smile = parse("\"\\uD83D\\uDE00\"").unwrap();
        assert_eq!(smile.as_str(), Some("😀"));
    }

    #[test]
    fn integers_are_integers_and_exponents_stay_raw() {
        assert_eq!(parse("42").unwrap().as_i64(), Some(42));
        assert_eq!(parse("-7").unwrap().as_i64(), Some(-7));
        assert!(parse("1e2").unwrap().as_i64().is_none());
        assert!(parse("1.0").unwrap().as_i64().is_none());
    }
}
