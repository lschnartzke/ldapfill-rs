//! Parser for text lines that specify modifiers.
//!
//! The parser expects a top-level modifier to be present when parsing.
//!
//! The parser will create a tree, that will be processed breadth-first from bottom to top.
//!
//! For example, the modifier:
//!
//! combine(uppercase(file("firstname.txt")), ".", combine(uppercase(file("lastname.txt")), "-",
//! lowercase(file("country.txt"))))
//!
//! Will result in the following tree:
//!
//! combine: 
//! |-- uppercase:
//! ---| file:
//! |  |---- "firstname.txt"
//! |
//! |-- "."
//! |
//! |-- combine:
//! |---|-- uppercase:
//!     |  |-- file:
//!     |      | -- "lastname.txt"
//!     |-- "-"
//!     |
//!     | lowercase:
//!       | -- file:
//!            | -- "country.txt"
//!
//!
//! During evaluation, the file contents will be loaded into memory first, then, for each
//! invocation, a random will be returned. The upper-lowercase modifiers will format the value
//! accordingly and the combine-modifiers will chain all parameters together.

use std::str::FromStr;

use thiserror::Error;

pub type ParserResult = Result<(Token, usize), ParserError>;

/// Individual entities that can be encountered when parsing config files
#[derive(Debug, PartialEq, Eq)]
pub enum Token {
    Modifier(Modifier, Vec<Token>),
    String(String)
}

// A list of modifiers that can be encountered.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Modifier {
    Combine,
    Uppercase,
    Lowercase,
    File
}

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("unclosed string beginning at {0}")]
    UnclosedString(usize),
    #[error("Encountered unknown modifier name: {0}")]
    UnknownModifier(String),
    #[error("Value cannot be empty")]
    EmptyValue,
    #[error("unmatches parenthesis")]
    UnmatchesParenthesis,
    #[error("illegal character {0} at {1}")]
    IllegalCharacter(char, usize)
}

#[derive(Debug)]
pub struct Parser {

}

impl Parser {
    pub fn new() -> Self {
        Self {}
    }

    /// Parse the provided string into a token tree or return the error that occured.
    pub fn parse(&self, raw: &str) -> ParserResult {
        return match raw.char_indices().next() {
            None => Err(ParserError::EmptyValue),
            Some((_, '\'' | '"')) => self.parse_string(&raw[1..], 1),
            Some(_) => self.parse_modifier(raw, 0)
        }
    }


    /// Parse a modifier. This function is called when we encountered a opening parenthesis after
    /// parsing an unquoted literal string. The first character of the `str` is immediately 
    /// after the opening parenthesis of the modifier.
    /// Returns the parsed modifier, or error, and the total amount of processed characters. 
    fn parse_modifier(&self, str: &str, start_offset: usize) -> ParserResult {
        let mut modifier_name = String::new();
        let mut index = 0;
        let mut parsed_chars = 0;
        for (i, ch) in str.char_indices() {
            index = i;
            parsed_chars = i;
            // parsed name fully, can parse sub-tokens or modifiers
            if ch == '(' {
                break;
            } else if !ch.is_ascii_alphabetic() {
                return Err(ParserError::IllegalCharacter(ch, start_offset + parsed_chars));
            } else {
                modifier_name.push(ch);
            }
        }

        let modifier = Modifier::from_str(modifier_name.as_str())?;

        // parse string or nested modifier
        let arguments: Vec<Token> = {
            
        };
        

        Err(ParserError::EmptyValue)
    }

    /// Parse a raw string. Called immediately after a quote is encountered. Returns a String token 
    /// containing all characters except the last non-escaped quote.
    fn parse_string(&self, str: &str, start_offset: usize) -> ParserResult {
        let mut string = String::new();
        let mut escape_next = false;
        let mut i = 0;
        for (i, ch) in str.char_indices() {
            if ch == '\\' && !escape_next {
                escape_next = true;
                continue;
            } else if ch == '"' {
                if escape_next {
                    string.push(ch);
                } else {
                    return Ok((Token::String(string), start_offset + i));
                }
            } else {
                string.push(ch);
            }

            escape_next = false;
        }
    
        Err(ParserError::UnclosedString(start_offset + i))
    }
}

impl FromStr for Modifier {
    type Err = ParserError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "lowercase" => Ok(Modifier::Lowercase),
            "uppercase" => Ok(Modifier::Uppercase),
            "file" => Ok(Modifier::File),
            "combine" => Ok(Modifier::Combine),
            _ => Err(ParserError::UnknownModifier(s.to_owned()))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parser_parse_string_simple() {
        let unparsed = "\"string\"";
        let mut parser = Parser::new();

        let res = parser.parse(unparsed);
        assert!(res.is_ok());
        let res = res.unwrap();

        assert_eq!(res.0, Token::String(String::from("string")));
    }

    #[test]
    fn test_parser_parse_string_escaped() {
        let unparsed = "\"\\\"pa55w0rd\"";

        let mut parser = Parser::new();
        let res = parser.parse(unparsed);

        assert!(res.is_ok());
        let res = res.unwrap();

        assert_eq!(res.0, Token::String(String::from("\"pa55w0rd")));
    }
}
