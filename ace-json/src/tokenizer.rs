use crate::JsonError;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    BraceOpen,   // {
    BraceClose,  // }
    BracketOpen, // [
    BracketClose,// ]
    Colon,       // :
    Comma,       // ,
    String(String),
    Number(f64),
    Bool(bool),
    Null,
}

pub struct Tokenizer<'a> {
    input: &'a [char],
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    pub fn new(input: &'a [char]) -> Self {
        Self { input, pos: 0 }
    }

    pub fn current_pos(&self) -> usize {
        self.pos
    }

    pub fn next_token(&mut self) -> Result<Option<Token>, JsonError> {
        self.skip_whitespace();

        if self.pos >= self.input.len() {
            return Ok(None);
        }

        let c = self.input[self.pos];
        self.pos += 1;

        match c {
            '{' => Ok(Some(Token::BraceOpen)),
            '}' => Ok(Some(Token::BraceClose)),
            '[' => Ok(Some(Token::BracketOpen)),
            ']' => Ok(Some(Token::BracketClose)),
            ':' => Ok(Some(Token::Colon)),
            ',' => Ok(Some(Token::Comma)),
            '"' => self.read_string().map(|s| Some(Token::String(s))),
            't' => self.read_literal("rue", Token::Bool(true)),
            'f' => self.read_literal("alse", Token::Bool(false)),
            'n' => self.read_literal("ull", Token::Null),
            '-' | '0'..='9' => self.read_number(c).map(|n| Some(Token::Number(n))),
            _ => Err(JsonError::UnexpectedCharacter(c, self.pos - 1)),
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.input[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }

    fn read_string(&mut self) -> Result<String, JsonError> {
        let mut result = String::new();
        let start_pos = self.pos - 1;

        while self.pos < self.input.len() {
            let c = self.input[self.pos];
            self.pos += 1;

            if c == '"' {
                return Ok(result);
            }

            if c == '\\' {
                if self.pos >= self.input.len() {
                    return Err(JsonError::InvalidString(start_pos));
                }
                let esc = self.input[self.pos];
                self.pos += 1;
                match esc {
                    '"' => result.push('"'),
                    '\\' => result.push('\\'),
                    '/' => result.push('/'),
                    'b' => result.push('\u{0008}'),
                    'f' => result.push('\u{000C}'),
                    'n' => result.push('\n'),
                    'r' => result.push('\r'),
                    't' => result.push('\t'),
                    'u' => {
                        if self.pos + 4 > self.input.len() {
                            return Err(JsonError::InvalidString(self.pos - 2));
                        }
                        let hex: String = self.input[self.pos..self.pos + 4].iter().collect();
                        self.pos += 4;
                        if let Ok(code) = u32::from_str_radix(&hex, 16) {
                            if let Some(c) = std::char::from_u32(code) {
                                result.push(c);
                            } else {
                                return Err(JsonError::InvalidString(self.pos - 6));
                            }
                        } else {
                            return Err(JsonError::InvalidString(self.pos - 6));
                        }
                    }
                    _ => return Err(JsonError::UnexpectedCharacter(esc, self.pos - 2)),
                }
            } else {
                result.push(c);
            }
        }

        Err(JsonError::InvalidString(start_pos))
    }

    fn read_literal(&mut self, expected: &str, token: Token) -> Result<Option<Token>, JsonError> {
        let start_pos = self.pos - 1;
        for c in expected.chars() {
            if self.pos >= self.input.len() || self.input[self.pos] != c {
                return Err(JsonError::ExpectedToken(expected.to_string(), start_pos));
            }
            self.pos += 1;
        }
        Ok(Some(token))
    }

    fn read_number(&mut self, first_char: char) -> Result<f64, JsonError> {
        let start_pos = self.pos - 1;
        let mut s = String::new();
        s.push(first_char);

        while self.pos < self.input.len() {
            let c = self.input[self.pos];
            if c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E' || c == '+' || c == '-' {
                s.push(c);
                self.pos += 1;
            } else {
                break;
            }
        }

        s.parse::<f64>().map_err(|_| JsonError::InvalidNumber(start_pos))
    }
}
