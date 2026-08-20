//! # Parser de Cabeçalhos HTTP Estruturados RFC 8941 / RFC 9651 (Zero-Allocation)
//!
//! A especificação RFC 8941 (Structured Fields for HTTP) padroniza a tipagem e sintaxe de cabeçalhos de rede
//! (`Dictionary`, `List`, `Item`, `Integer`, `Decimal`, `String`, `Token`, `Byte Sequence`, `Boolean`).
//! Este parser opera diretamente sobre as fatias de bytes (`&'a [u8]`) do buffer de I/O de rede sem alocações temporárias.

use std::fmt;

#[derive(Debug, PartialEq, Clone)]
pub enum BareItem<'a> {
    Integer(i64),
    Decimal {
        integer_part: i64,
        fractional_part: i16,
        precision: u8,
    },
    String(&'a str),
    Token(&'a str),
    ByteSeq(&'a [u8]),
    Boolean(bool),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Parameter<'a> {
    pub key: &'a str,
    pub value: BareItem<'a>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ParameterizedItem<'a> {
    pub item: BareItem<'a>,
    pub parameters: Vec<Parameter<'a>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SfError {
    UnexpectedEnd,
    InvalidCharacter(char),
    IntegerOverflow,
    DecimalOverflow,
    SyntaxError(&'static str),
}

impl fmt::Display for SfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for SfError {}

pub struct SfCursor<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> SfCursor<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self { input, pos: 0 }
    }

    #[inline(always)]
    pub fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    #[inline(always)]
    pub fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    #[inline(always)]
    pub fn advance(&mut self) {
        if self.pos < self.input.len() {
            self.pos += 1;
        }
    }

    pub fn skip_sp(&mut self) {
        while let Some(b) = self.peek() {
            if b == b' ' {
                self.advance();
            } else {
                break;
            }
        }
    }

    pub fn skip_ow_ws(&mut self) {
        while let Some(b) = self.peek() {
            if b == b' ' || b == b'\t' {
                self.advance();
            } else {
                break;
            }
        }
    }

    pub fn parse_key(&mut self) -> Result<&'a str, SfError> {
        let start = self.pos;
        match self.peek() {
            Some(b) if b.is_ascii_lowercase() || b == b'*' => self.advance(),
            Some(c) => return Err(SfError::InvalidCharacter(c as char)),
            None => return Err(SfError::UnexpectedEnd),
        }

        while let Some(b) = self.peek() {
            if b.is_ascii_lowercase()
                || b.is_ascii_digit()
                || b == b'_'
                || b == b'-'
                || b == b'.'
                || b == b'*'
            {
                self.advance();
            } else {
                break;
            }
        }

        let slice = &self.input[start..self.pos];
        std::str::from_utf8(slice).map_err(|_| SfError::SyntaxError("Invalid UTF-8 in key"))
    }

    pub fn parse_bare_item(&mut self) -> Result<BareItem<'a>, SfError> {
        match self.peek() {
            Some(b'?') => self.parse_boolean(),
            Some(b'"') => self.parse_string(),
            Some(b':') => self.parse_byte_sequence(),
            Some(b) if b == b'-' || b.is_ascii_digit() => self.parse_number(),
            Some(b) if b.is_ascii_alphabetic() || b == b'*' => self.parse_token(),
            Some(c) => Err(SfError::InvalidCharacter(c as char)),
            None => Err(SfError::UnexpectedEnd),
        }
    }

    fn parse_boolean(&mut self) -> Result<BareItem<'a>, SfError> {
        self.advance();
        match self.peek() {
            Some(b'1') => {
                self.advance();
                Ok(BareItem::Boolean(true))
            }
            Some(b'0') => {
                self.advance();
                Ok(BareItem::Boolean(false))
            }
            Some(c) => Err(SfError::InvalidCharacter(c as char)),
            None => Err(SfError::UnexpectedEnd),
        }
    }

    fn parse_string(&mut self) -> Result<BareItem<'a>, SfError> {
        self.advance();
        let start = self.pos;

        while let Some(b) = self.peek() {
            if b == b'"' {
                let slice = &self.input[start..self.pos];
                self.advance();
                let s = std::str::from_utf8(slice)
                    .map_err(|_| SfError::SyntaxError("Invalid UTF-8 in string"))?;
                return Ok(BareItem::String(s));
            } else if b < 0x20 || b > 0x7E {
                return Err(SfError::InvalidCharacter(b as char));
            } else {
                self.advance();
            }
        }
        Err(SfError::UnexpectedEnd)
    }

    fn parse_token(&mut self) -> Result<BareItem<'a>, SfError> {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if b.is_ascii_alphabetic()
                || b.is_ascii_digit()
                || b == b'_'
                || b == b'-'
                || b == b'.'
                || b == b':'
                || b == b'/'
                || b == b'*'
            {
                self.advance();
            } else {
                break;
            }
        }
        let slice = &self.input[start..self.pos];
        let s = std::str::from_utf8(slice)
            .map_err(|_| SfError::SyntaxError("Invalid UTF-8 in token"))?;
        Ok(BareItem::Token(s))
    }

    fn parse_byte_sequence(&mut self) -> Result<BareItem<'a>, SfError> {
        self.advance();
        let start = self.pos;
        while let Some(b) = self.peek() {
            if b == b':' {
                let slice = &self.input[start..self.pos];
                self.advance();
                return Ok(BareItem::ByteSeq(slice));
            } else if b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=' {
                self.advance();
            } else {
                return Err(SfError::InvalidCharacter(b as char));
            }
        }
        Err(SfError::UnexpectedEnd)
    }

    fn parse_number(&mut self) -> Result<BareItem<'a>, SfError> {
        let mut is_negative = false;
        let mut integer_val: i64 = 0;
        let mut is_decimal = false;
        let mut fractional_val: i16 = 0;
        let mut fractional_digits = 0;
        let mut digits_count = 0;

        if self.peek() == Some(b'-') {
            is_negative = true;
            self.advance();
        }

        while let Some(b) = self.peek() {
            if b.is_ascii_digit() {
                digits_count += 1;
                if is_decimal {
                    if fractional_digits >= 3 {
                        return Err(SfError::DecimalOverflow);
                    }
                    fractional_digits += 1;
                    fractional_val = fractional_val * 10 + (b - b'0') as i16;
                } else {
                    if digits_count > 15 {
                        return Err(SfError::IntegerOverflow);
                    }
                    integer_val = integer_val
                        .checked_mul(10)
                        .and_then(|v| v.checked_add((b - b'0') as i64))
                        .ok_or(SfError::IntegerOverflow)?;
                }
                self.advance();
            } else if b == b'.' && !is_decimal {
                if digits_count == 0 || digits_count > 12 {
                    return Err(SfError::DecimalOverflow);
                }
                is_decimal = true;
                self.advance();
            } else {
                break;
            }
        }

        if is_negative {
            integer_val = -integer_val;
            fractional_val = -fractional_val;
        }

        if is_decimal {
            if fractional_digits == 0 {
                return Err(SfError::SyntaxError("Trailing decimal point"));
            }
            Ok(BareItem::Decimal {
                integer_part: integer_val,
                fractional_part: fractional_val,
                precision: fractional_digits,
            })
        } else {
            Ok(BareItem::Integer(integer_val))
        }
    }

    pub fn parse_parameters(&mut self) -> Result<Vec<Parameter<'a>>, SfError> {
        let mut params = Vec::new();
        while let Some(b';') = self.peek() {
            self.advance();
            self.skip_sp();
            let key = self.parse_key()?;
            let value = if let Some(b'=') = self.peek() {
                self.advance();
                self.parse_bare_item()?
            } else {
                BareItem::Boolean(true)
            };
            params.push(Parameter { key, value });
        }
        Ok(params)
    }

    pub fn parse_item(&mut self) -> Result<ParameterizedItem<'a>, SfError> {
        let item = self.parse_bare_item()?;
        let parameters = self.parse_parameters()?;
        Ok(ParameterizedItem { item, parameters })
    }

    pub fn parse_list(&mut self) -> Result<Vec<ParameterizedItem<'a>>, SfError> {
        let mut list = Vec::new();
        if self.is_eof() {
            return Ok(list);
        }

        loop {
            self.skip_ow_ws();
            let item = self.parse_item()?;
            list.push(item);
            self.skip_ow_ws();

            if self.is_eof() {
                break;
            }

            match self.peek() {
                Some(b',') => {
                    self.advance();
                    self.skip_ow_ws();
                }
                Some(c) => return Err(SfError::InvalidCharacter(c as char)),
                None => break,
            }
        }
        Ok(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_structured_header_list() {
        let header = b"max-age=604800; report-to=\"default\", enforce; v=?1";
        let mut cursor = SfCursor::new(header);
        let list = cursor.parse_list().unwrap();

        assert_eq!(list.len(), 2);
        assert_eq!(list[0].item, BareItem::Token("max-age"));
        assert_eq!(list[0].parameters[0].key, "report-to");
        assert_eq!(list[0].parameters[0].value, BareItem::String("default"));
        assert_eq!(list[1].item, BareItem::Token("enforce"));
        assert_eq!(list[1].parameters[0].key, "v");
        assert_eq!(list[1].parameters[0].value, BareItem::Boolean(true));
    }
}
