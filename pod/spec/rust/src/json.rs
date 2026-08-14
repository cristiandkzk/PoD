//! Parser estricto del subconjunto de JSON aceptado — SPEC.md §3 y §7.1 paso 1.
//!
//! Escrito contra SPEC.md. No usa ninguna biblioteca de JSON: el formato prohíbe cosas que
//! los parsers de propósito general aceptan en silencio (floats, claves duplicadas, `null`),
//! y necesita reportar el código de error exacto de SPEC §7.

use std::collections::BTreeMap;

use crate::error::SpecError;

pub const MAX_DEPTH: usize = 8;
pub const MAX_STRING_LEN: usize = 256;

/// Los `Object` usan `BTreeMap`: su orden de iteración es el orden ascendente por bytes
/// UTF-8 de la clave, que es exactamente el orden canónico de SPEC §4.1.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Str(String),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Str(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }
}

fn is_string_char(c: char) -> bool {
    c.is_ascii_uppercase()
        || c.is_ascii_lowercase()
        || c.is_ascii_digit()
        || matches!(c, '-' | '_' | '.' | ':' | '/' | '@' | '+' | '=')
}

/// `^[a-z][a-z0-9_]{0,63}$` — SPEC §3.1.
fn is_key(s: &str) -> bool {
    let b = s.as_bytes();
    if b.is_empty() || b.len() > 64 {
        return false;
    }
    if !b[0].is_ascii_lowercase() {
        return false;
    }
    b[1..]
        .iter()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'_')
}

/// `^-?(0|[1-9][0-9]*)$` — SPEC §3.3.
fn is_int_token(s: &str) -> bool {
    let b = s.as_bytes();
    let digits = if b.first() == Some(&b'-') { &b[1..] } else { b };
    if digits.is_empty() {
        return false;
    }
    if digits[0] == b'0' {
        return digits.len() == 1;
    }
    digits.iter().all(|c| c.is_ascii_digit())
}

pub fn parse(data: &[u8]) -> Result<Value, SpecError> {
    let text = std::str::from_utf8(data).map_err(|_| SpecError::new("E_SYNTAX", "$"))?;
    if text.starts_with('\u{FEFF}') {
        return Err(SpecError::new("E_SYNTAX", "$"));
    }
    let mut p = Parser {
        s: text.chars().collect(),
        i: 0,
    };
    let value = p.value(0, "$")?;
    p.skip_ws();
    if p.i != p.s.len() {
        return Err(SpecError::new("E_SYNTAX", "$"));
    }
    match value {
        Value::Object(_) => Ok(value),
        _ => Err(SpecError::new("E_NOT_OBJECT", "$")),
    }
}

struct Parser {
    s: Vec<char>,
    i: usize,
}

impl Parser {
    fn skip_ws(&mut self) {
        while self.i < self.s.len() && matches!(self.s[self.i], ' ' | '\t' | '\n' | '\r') {
            self.i += 1;
        }
    }

    fn peek(&self, path: &str) -> Result<char, SpecError> {
        self.s
            .get(self.i)
            .copied()
            .ok_or_else(|| SpecError::new("E_SYNTAX", path))
    }

    fn literal(&mut self, word: &str) -> bool {
        let w: Vec<char> = word.chars().collect();
        if self.i + w.len() <= self.s.len() && self.s[self.i..self.i + w.len()] == w[..] {
            self.i += w.len();
            return true;
        }
        false
    }

    fn looking_at(&self, word: &str) -> bool {
        let w: Vec<char> = word.chars().collect();
        self.i + w.len() <= self.s.len() && self.s[self.i..self.i + w.len()] == w[..]
    }

    fn value(&mut self, depth: usize, path: &str) -> Result<Value, SpecError> {
        self.skip_ws();
        let c = self.peek(path)?;
        match c {
            '{' => self.object(depth + 1, path),
            '[' => self.array(depth + 1, path),
            '"' => {
                let text = self.string(path)?;
                let len = text.chars().count();
                if len < 1 || len > MAX_STRING_LEN {
                    return Err(SpecError::new("E_STRING_CHARSET", path));
                }
                if !text.chars().all(is_string_char) {
                    return Err(SpecError::new("E_STRING_CHARSET", path));
                }
                Ok(Value::Str(text))
            }
            't' => {
                if self.literal("true") {
                    Ok(Value::Bool(true))
                } else {
                    Err(SpecError::new("E_SYNTAX", path))
                }
            }
            'f' => {
                if self.literal("false") {
                    Ok(Value::Bool(false))
                } else {
                    Err(SpecError::new("E_SYNTAX", path))
                }
            }
            'n' => {
                if self.literal("null") {
                    Err(SpecError::new("E_NULL", path))
                } else {
                    Err(SpecError::new("E_SYNTAX", path))
                }
            }
            'N' => {
                if self.literal("NaN") {
                    Err(SpecError::new("E_FLOAT", path))
                } else {
                    Err(SpecError::new("E_SYNTAX", path))
                }
            }
            'I' => {
                if self.literal("Infinity") {
                    Err(SpecError::new("E_FLOAT", path))
                } else {
                    Err(SpecError::new("E_SYNTAX", path))
                }
            }
            '-' if self.looking_at("-Infinity") => Err(SpecError::new("E_FLOAT", path)),
            '-' => self.number(path),
            c if c.is_ascii_digit() => self.number(path),
            _ => Err(SpecError::new("E_SYNTAX", path)),
        }
    }

    fn object(&mut self, depth: usize, path: &str) -> Result<Value, SpecError> {
        if depth > MAX_DEPTH {
            return Err(SpecError::new("E_DEPTH", path));
        }
        self.i += 1; // '{'
        let mut out: BTreeMap<String, Value> = BTreeMap::new();
        self.skip_ws();
        if self.peek(path)? == '}' {
            self.i += 1;
            return Ok(Value::Object(out));
        }
        loop {
            self.skip_ws();
            if self.peek(path)? != '"' {
                return Err(SpecError::new("E_SYNTAX", path));
            }
            let key = self.string(path)?;
            let sub = format!("{path}.{key}");
            if !is_key(&key) {
                return Err(SpecError::new("E_KEY_CHARSET", sub));
            }
            if out.contains_key(&key) {
                return Err(SpecError::new("E_DUP_KEY", sub));
            }
            self.skip_ws();
            if self.peek(path)? != ':' {
                return Err(SpecError::new("E_SYNTAX", path));
            }
            self.i += 1;
            let value = self.value(depth, &sub)?;
            out.insert(key, value);
            self.skip_ws();
            match self.peek(path)? {
                ',' => {
                    self.i += 1;
                }
                '}' => {
                    self.i += 1;
                    return Ok(Value::Object(out));
                }
                _ => return Err(SpecError::new("E_SYNTAX", path)),
            }
        }
    }

    fn array(&mut self, depth: usize, path: &str) -> Result<Value, SpecError> {
        if depth > MAX_DEPTH {
            return Err(SpecError::new("E_DEPTH", path));
        }
        self.i += 1; // '['
        let mut out: Vec<Value> = Vec::new();
        self.skip_ws();
        if self.peek(path)? == ']' {
            return Err(SpecError::new("E_EMPTY_ARRAY", path));
        }
        loop {
            let sub = format!("{path}[{}]", out.len());
            out.push(self.value(depth, &sub)?);
            self.skip_ws();
            match self.peek(path)? {
                ',' => {
                    self.i += 1;
                }
                ']' => {
                    self.i += 1;
                    return Ok(Value::Array(out));
                }
                _ => return Err(SpecError::new("E_SYNTAX", path)),
            }
        }
    }

    /// Decodifica los escapes pero no valida el conjunto de caracteres: el código difiere
    /// entre clave y valor, así que eso lo decide quien llama (SPEC §7.1).
    fn string(&mut self, path: &str) -> Result<String, SpecError> {
        self.i += 1; // '"'
        let mut out = String::new();
        loop {
            let c = self.peek(path)?;
            if c == '"' {
                self.i += 1;
                return Ok(out);
            }
            if c == '\\' {
                self.i += 1;
                let e = self.peek(path)?;
                let decoded = match e {
                    '"' => '"',
                    '\\' => '\\',
                    '/' => '/',
                    'b' => '\u{0008}',
                    'f' => '\u{000C}',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    'u' => {
                        out.push(self.unicode_escape(path)?);
                        continue;
                    }
                    _ => return Err(SpecError::new("E_SYNTAX", path)),
                };
                out.push(decoded);
                self.i += 1;
                continue;
            }
            if (c as u32) < 0x20 {
                return Err(SpecError::new("E_SYNTAX", path));
            }
            out.push(c);
            self.i += 1;
        }
    }

    fn unicode_escape(&mut self, path: &str) -> Result<char, SpecError> {
        let mut cp = self.hex4(path)? as u32;
        if (0xD800..=0xDBFF).contains(&cp) {
            if !self.looking_at("\\u") {
                return Err(SpecError::new("E_SYNTAX", path));
            }
            self.i += 1; // '\\'
            let low = self.hex4(path)? as u32;
            if !(0xDC00..=0xDFFF).contains(&low) {
                return Err(SpecError::new("E_SYNTAX", path));
            }
            cp = 0x10000 + ((cp - 0xD800) << 10) + (low - 0xDC00);
        } else if (0xDC00..=0xDFFF).contains(&cp) {
            return Err(SpecError::new("E_SYNTAX", path));
        }
        char::from_u32(cp).ok_or_else(|| SpecError::new("E_SYNTAX", path))
    }

    fn hex4(&mut self, path: &str) -> Result<u16, SpecError> {
        self.i += 1; // 'u'
        if self.i + 4 > self.s.len() {
            return Err(SpecError::new("E_SYNTAX", path));
        }
        let mut acc: u16 = 0;
        for k in 0..4 {
            let d = self.s[self.i + k]
                .to_digit(16)
                .ok_or_else(|| SpecError::new("E_SYNTAX", path))?;
            acc = acc * 16 + d as u16;
        }
        self.i += 4;
        Ok(acc)
    }

    fn number(&mut self, path: &str) -> Result<Value, SpecError> {
        let start = self.i;
        while self.i < self.s.len()
            && matches!(self.s[self.i], '+' | '-' | '.' | 'e' | 'E' | '0'..='9')
        {
            self.i += 1;
        }
        let token: String = self.s[start..self.i].iter().collect();
        if token.is_empty() {
            return Err(SpecError::new("E_SYNTAX", path));
        }
        if token.contains('.') || token.contains('e') || token.contains('E') {
            return Err(SpecError::new("E_FLOAT", path));
        }
        if token == "-0" {
            return Err(SpecError::new("E_INT_FORMAT", path));
        }
        if !is_int_token(&token) {
            return Err(SpecError::new("E_INT_FORMAT", path));
        }
        token
            .parse::<i64>()
            .map(Value::Int)
            .map_err(|_| SpecError::new("E_INT_RANGE", path))
    }
}
