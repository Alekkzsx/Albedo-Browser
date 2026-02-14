use cssparser::{Parser, ParseError, CowRcStr, SourceLocation, ParserState, Token, DeclarationParser};
use crate::engine::style::selector_impl::Helper;
use selectors::parser::SelectorList;

pub struct RuleParser;

pub type StyleRule = crate::engine::style::Rule;

impl<'i> cssparser::QualifiedRuleParser<'i> for RuleParser {
    type Prelude = SelectorList<Helper>;
    type QualifiedRule = crate::engine::style::Rule;
    type Error = ();

    fn parse_prelude<'t>(
        &mut self,
        parser: &mut Parser<'i, 't>,
    ) -> Result<Self::Prelude, ParseError<'i, Self::Error>> {
        let selectors = SelectorList::parse(&SelectorParserImpl, parser, selectors::parser::ParseRelative::No)
             .map_err(|_| parser.new_custom_error(()))?;
        Ok(selectors)
    }

    fn parse_block<'t>(
        &mut self,
        _prelude: Self::Prelude,
        _start: &ParserState,
        parser: &mut Parser<'i, 't>,
    ) -> Result<Self::QualifiedRule, ParseError<'i, Self::Error>> {
        let declarations = parse_declarations(parser);
        
        Ok(crate::engine::style::Rule {
            selectors: _prelude,
            declarations,
        })
    }
}

impl<'i> cssparser::AtRuleParser<'i> for RuleParser {
    type Prelude = ();
    type AtRule = StyleRule;
    type Error = ();
    
     fn parse_prelude<'t>(
        &mut self,
        _name: CowRcStr<'i>,
        parser: &mut Parser<'i, 't>,
    ) -> Result<Self::Prelude, ParseError<'i, Self::Error>> {
        Err(parser.new_custom_error(()))
    }

     fn parse_block<'t>(
        &mut self,
        _prelude: Self::Prelude,
        _start: &ParserState,
        _parser: &mut Parser<'i, 't>,
    ) -> Result<Self::AtRule, ParseError<'i, Self::Error>> {
        Err(_parser.new_custom_error(()))
    }
}

pub struct PropertyParser;

impl<'i> cssparser::DeclarationParser<'i> for PropertyParser {
    type Declaration = crate::engine::style::Declaration;
    type Error = ();

    fn parse_value<'t>(
        &mut self,
        _name: CowRcStr<'i>,
        parser: &mut Parser<'i, 't>,
        _parser_state: &ParserState,
    ) -> Result<Self::Declaration, ParseError<'i, Self::Error>> {
        let mut value = String::new();
        while let Ok(token) = parser.next_including_whitespace() {
            match token {
                Token::Semicolon => break,
                Token::Ident(s) => value.push_str(s),
                Token::Dimension { value: v, unit, .. } => value.push_str(&format!("{}{}", v, unit)),
                Token::Number { value: v, .. } => value.push_str(&v.to_string()),
                Token::Percentage { unit_value, .. } => value.push_str(&format!("{}%", unit_value * 100.0)),
                Token::Hash(s) | Token::IDHash(s) => value.push_str(&format!("#{}", s)),
                Token::QuotedString(s) => value.push_str(&format!("\"{}\"", s)),
                Token::WhiteSpace(s) => value.push_str(s),
                _ => {} 
            }
        }
        
        Ok(crate::engine::style::Declaration {
            name: _name.to_string(),
            value: value.trim().to_string(),
        })
    }
}

pub struct SelectorParserImpl;

impl<'i> selectors::parser::Parser<'i> for SelectorParserImpl {
    type Impl = Helper;
    type Error = selectors::parser::SelectorParseErrorKind<'i>;

    fn parse_non_ts_pseudo_class(
        &self,
        _location: SourceLocation,
        _name: CowRcStr<'i>,
    ) -> Result<<Self::Impl as selectors::SelectorImpl>::NonTSPseudoClass, ParseError<'i, Self::Error>> {
        Err(_location.new_custom_error(selectors::parser::SelectorParseErrorKind::UnexpectedIdent("".into())))
    }
    
    fn parse_non_ts_functional_pseudo_class<'t>(
        &self,
        _name: CowRcStr<'i>,
        parser: &mut Parser<'i, 't>,
        _bool_arg: bool,
    ) -> Result<<Self::Impl as selectors::SelectorImpl>::NonTSPseudoClass, ParseError<'i, Self::Error>> {
         Err(parser.new_custom_error(selectors::parser::SelectorParseErrorKind::UnexpectedIdent("".into())))
    }
    
    fn parse_pseudo_element(
        &self,
        _location: SourceLocation,
        _name: CowRcStr<'i>,
    ) -> Result<<Self::Impl as selectors::SelectorImpl>::PseudoElement, ParseError<'i, Self::Error>> {
         Err(_location.new_custom_error(selectors::parser::SelectorParseErrorKind::UnexpectedIdent("".into())))
    }
    
    fn parse_slotted(&self) -> bool { false }
    fn parse_part(&self) -> bool { false }
    fn parse_is_and_where(&self) -> bool { false }
    fn parse_host(&self) -> bool { false }
    fn parse_parent_selector(&self) -> bool { false }
}

pub fn parse_declarations<'i, 't>(parser: &mut Parser<'i, 't>) -> Vec<crate::engine::style::Declaration> {
    let mut declarations = Vec::new();
    let mut property_parser = PropertyParser;
    
    while let Ok(token) = parser.next() {
        match token {
            Token::Ident(name) => {
                let name_cow = name.clone();
                if let Ok(Token::Colon) = parser.next() {
                    let state = parser.state();
                    if let Ok(decl) = property_parser.parse_value(name_cow, parser, &state) {
                        declarations.push(decl);
                    }
                }
            }
            Token::Comment(_) | Token::WhiteSpace(_) | Token::Semicolon => continue,
            _ => {}
        }
    }
    declarations
}
