//! A small JSON reader for the evidence tests, written for them and sharing nothing with the
//! writer. It reads every JSON value (objects keep their keys in written order, numbers keep
//! their text), which is enough for evidence lines and for the schema files.

use std::iter::Peekable;
use std::str::Chars;

/// A parsed JSON value.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Json {
    Null,
    Bool(bool),
    /// A number as written, so integers and decimals stay distinguishable.
    Number(String),
    String(String),
    Array(Vec<Json>),
    /// Members in written order.
    Object(Vec<(String, Json)>),
}

impl Json {
    /// The member `key` of an object.
    pub(super) fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Self::Object(members) => members.iter().find(|(name, _)| name == key).map(|(_, v)| v),
            _ => None,
        }
    }
}

/// Parses one JSON document, refusing trailing text and duplicate keys.
pub(super) fn parse(text: &str) -> Result<Json, String> {
    let mut reader = Reader { chars: text.chars().peekable() };
    let value = reader.value()?;
    reader.skip_space();
    match reader.chars.next() {
        None => Ok(value),
        Some(c) => Err(format!("trailing {c:?} after the value")),
    }
}

struct Reader<'a> {
    chars: Peekable<Chars<'a>>,
}

impl Reader<'_> {
    fn skip_space(&mut self) {
        while self.chars.peek().is_some_and(|c| matches!(c, ' ' | '\t' | '\n' | '\r')) {
            self.chars.next();
        }
    }

    fn next(&mut self) -> Result<char, String> {
        self.chars.next().ok_or_else(|| "unexpected end of text".to_owned())
    }

    fn expect(&mut self, wanted: char) -> Result<(), String> {
        let found = self.next()?;
        if found == wanted {
            Ok(())
        } else {
            Err(format!("expected {wanted:?}, found {found:?}"))
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        self.skip_space();
        match self.chars.peek().copied() {
            Some('{') => self.object(),
            Some('[') => self.array(),
            Some('"') => Ok(Json::String(self.string()?)),
            Some(_) => self.word(),
            None => Err("missing value".to_owned()),
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.expect('{')?;
        let mut members: Vec<(String, Json)> = Vec::new();
        self.skip_space();
        if self.chars.peek() == Some(&'}') {
            self.chars.next();
            return Ok(Json::Object(members));
        }
        loop {
            self.skip_space();
            let key = self.string()?;
            if members.iter().any(|(name, _)| *name == key) {
                return Err(format!("duplicate key {key:?}"));
            }
            self.skip_space();
            self.expect(':')?;
            members.push((key, self.value()?));
            self.skip_space();
            match self.next()? {
                ',' => {}
                '}' => return Ok(Json::Object(members)),
                other => return Err(format!("unexpected {other:?} in an object")),
            }
        }
    }

    fn array(&mut self) -> Result<Json, String> {
        self.expect('[')?;
        let mut items = Vec::new();
        self.skip_space();
        if self.chars.peek() == Some(&']') {
            self.chars.next();
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.value()?);
            self.skip_space();
            match self.next()? {
                ',' => {}
                ']' => return Ok(Json::Array(items)),
                other => return Err(format!("unexpected {other:?} in an array")),
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut text = String::new();
        loop {
            match self.next()? {
                '"' => return Ok(text),
                '\\' => text.push(self.escape()?),
                c if c < ' ' => return Err("raw control character in a string".to_owned()),
                c => text.push(c),
            }
        }
    }

    fn escape(&mut self) -> Result<char, String> {
        Ok(match self.next()? {
            '"' => '"',
            '\\' => '\\',
            '/' => '/',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            'b' => '\u{8}',
            'f' => '\u{c}',
            'u' => {
                let mut hex = String::new();
                for _ in 0..4 {
                    hex.push(self.next()?);
                }
                let code = u32::from_str_radix(&hex, 16).map_err(|err| err.to_string())?;
                char::from_u32(code).ok_or("a surrogate escape")?
            }
            other => return Err(format!("bad escape \\{other}")),
        })
    }

    fn word(&mut self) -> Result<Json, String> {
        let mut word = String::new();
        while let Some(&c) = self.chars.peek() {
            if matches!(c, ',' | '}' | ']' | ' ' | '\t' | '\n' | '\r') {
                break;
            }
            word.push(c);
            self.chars.next();
        }
        match word.as_str() {
            "true" => Ok(Json::Bool(true)),
            "false" => Ok(Json::Bool(false)),
            "null" => Ok(Json::Null),
            number if is_json_number(number) => Ok(Json::Number(number.to_owned())),
            other => Err(format!("not a JSON value: {other:?}")),
        }
    }
}

/// The JSON number grammar: `-? (0 | [1-9][0-9]*) (\.[0-9]+)? ([eE][+-]?[0-9]+)?`.
fn is_json_number(text: &str) -> bool {
    let rest = text.strip_prefix('-').unwrap_or(text);
    let (mantissa, exponent) = match rest.find(['e', 'E']) {
        Some(at) => (&rest[..at], Some(&rest[at + 1..])),
        None => (rest, None),
    };
    let (whole, fraction) = match mantissa.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (mantissa, None),
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let whole_ok = digits(whole) && (whole == "0" || !whole.starts_with('0'));
    let exponent_ok = exponent.is_none_or(|e| digits(e.strip_prefix(['+', '-']).unwrap_or(e)));
    whole_ok && fraction.is_none_or(digits) && exponent_ok
}
