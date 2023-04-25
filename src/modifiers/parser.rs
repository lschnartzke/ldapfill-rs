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
use pest::Parser;

pub type ParserResult<'e> = Result<Token, ParserError>;

/// Individual entities that can be encountered when parsing config files
#[derive(Debug, PartialEq, Eq)]
pub enum Token {
    Modifier(Modifier, Vec<Token>),
    String(String),
}

// A list of modifiers that can be encountered.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Modifier {
    Combine,
    Uppercase,
    Lowercase,
    File,
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
    IllegalCharacter(char, usize),
}

#[derive(Debug, Parser)]
#[grammar = "../modifier.pest"]
pub struct CfgParser;

pub fn parse(input: &str) -> Token {
    let res = CfgParser::parse(Rule::line, input).expect("Valid input");

    // TODO: Convert parsed tree into tokens/modifiers
    for record in res.into_iter() {
        println!("{record:#?}");
    }

    panic!("Nope");
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parser_parse_string_simple() {
        let unparsed = "\"string\"";

        let res = parse(unparsed);
        
        assert_eq!(res, Token::String(From::from("string")));
    }

    #[test]
    fn test_parser_parse_string_escaped() {
        let unparsed = "\"\\\"pa55w0rd\"";


    }

    #[test]
    fn test_uppercase_modifier_string_arg() {
        let unparsed = "uppercase(\"hello\")";
        let res = parse(unparsed);

    }

    #[test]
    fn test_lowercase_modifier_string_arg() {
        let unparsed = "lowercase(\"hello\")";
        let res = parse(unparsed);

    }

    #[test]
    fn test_combine_modifier_three_args() {
        let unparsed = "combine(\"hello\", \",\", \" world\")";
        let res = parse(unparsed);

    }
}

