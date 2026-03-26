use super::tokenizer::{Token, Tokenizer};
use super::value::JsonValue;
use super::JsonError;

pub fn parse(input: &str) -> Result<JsonValue, JsonError> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokenizer = Tokenizer::new(&chars);
    let mut parser = Parser::new(&mut tokenizer)?;
    let result = parser.parse_value()?;
    
    if tokenizer.next_token()?.is_some() {
        return Err(JsonError::UnexpectedCharacter(chars[tokenizer.current_pos()-1], tokenizer.current_pos()-1));
    }
    
    Ok(result)
}

struct Parser<'a, 'b> {
    tokenizer: &'a mut Tokenizer<'b>,
    lookahead: Option<Token>,
}

impl<'a, 'b> Parser<'a, 'b> {
    fn new(tokenizer: &'a mut Tokenizer<'b>) -> Result<Self, JsonError> {
        let first = tokenizer.next_token()?;
        Ok(Self { tokenizer, lookahead: first })
    }

    fn consume(&mut self) -> Result<Token, JsonError> {
        let current = self.lookahead.take().ok_or(JsonError::UnexpectedEndOfInput)?;
        self.lookahead = self.tokenizer.next_token()?;
        Ok(current)
    }

    fn peek(&self) -> Option<&Token> {
        self.lookahead.as_ref()
    }

    fn parse_value(&mut self) -> Result<JsonValue, JsonError> {
        match self.peek() {
            Some(Token::BraceOpen) => self.parse_object(),
            Some(Token::BracketOpen) => self.parse_array(),
            Some(Token::String(_)) => {
                if let Token::String(s) = self.consume()? {
                    Ok(JsonValue::String(s))
                } else {
                    unreachable!()
                }
            }
            Some(Token::Number(_)) => {
                if let Token::Number(n) = self.consume()? {
                    Ok(JsonValue::Number(n))
                } else {
                    unreachable!()
                }
            }
            Some(Token::Bool(_)) => {
                if let Token::Bool(b) = self.consume()? {
                    Ok(JsonValue::Bool(b))
                } else {
                    unreachable!()
                }
            }
            Some(Token::Null) => {
                self.consume()?;
                Ok(JsonValue::Null)
            }
            _ => {
                let pos = self.tokenizer.current_pos();
                Err(JsonError::ExpectedToken("value".to_string(), pos))
            }
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, JsonError> {
        self.consume()?; 
        let mut map = std::collections::HashMap::new();

        if let Some(Token::BraceClose) = self.peek() {
            self.consume()?;
            return Ok(JsonValue::Object(map));
        }

        loop {
            let key = match self.consume()? {
                Token::String(s) => s,
                _ => return Err(JsonError::ExpectedToken("string key".to_string(), self.tokenizer.current_pos())),
            };

            match self.consume()? {
                Token::Colon => (),
                _ => return Err(JsonError::ExpectedToken(":".to_string(), self.tokenizer.current_pos())),
            }

            let value = self.parse_value()?;
            map.insert(key, value);

            match self.peek() {
                Some(Token::Comma) => {
                    self.consume()?;
                }
                Some(Token::BraceClose) => {
                    self.consume()?;
                    break;
                }
                _ => return Err(JsonError::ExpectedToken(", or }".to_string(), self.tokenizer.current_pos())),
            }
        }

        Ok(JsonValue::Object(map))
    }

    fn parse_array(&mut self) -> Result<JsonValue, JsonError> {
        self.consume()?; 
        let mut vec = Vec::new();

        if let Some(Token::BracketClose) = self.peek() {
            self.consume()?;
            return Ok(JsonValue::Array(vec));
        }

        loop {
            let value = self.parse_value()?;
            vec.push(value);

            match self.peek() {
                Some(Token::Comma) => {
                    self.consume()?;
                }
                Some(Token::BracketClose) => {
                    self.consume()?;
                    break;
                }
                _ => return Err(JsonError::ExpectedToken(", or ]".to_string(), self.tokenizer.current_pos())),
            }
        }

        Ok(JsonValue::Array(vec))
    }
}
