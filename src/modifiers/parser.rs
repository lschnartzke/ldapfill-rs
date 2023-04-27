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

use pest::{error::Error, iterators::Pair, iterators::Pairs, Parser};
use thiserror::Error;

mod processor;

pub type ParserResult<'e> = Result<Token, ParserError>;
pub type PestResult<'r> = Result<Pairs<'r, Rule>, Error<Rule>>;

/// Individual entities that can be encountered when parsing config files
#[derive(Debug, PartialEq, Eq)]
pub enum Token {
    Modifier(Modifier, Vec<Token>),
    String(String),
}

// A list of modifiers that can be encountered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modifier {
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

pub fn parse(input: &str) -> ParserResult {
    let mut res = CfgParser::parse(Rule::line, input).expect("Valid input");

    let res = res.next().expect("at least one pair");
    #[cfg(test)]
    println!("{res:#?}");

    let mut token = build_token_tree_from_pair(res);
    assert!(token.len() == 1);

    Ok(token.pop().expect("exactly one token"))
}

fn build_token_tree_from_pair(pair: Pair<Rule>) -> Vec<Token> {
    let mut res = vec![];
    let rule = pair.as_rule();
    println!("build_token_tree_from_pairs(): rule: {rule:?}, pair: {pair:#?}");

    match rule {
        Rule::line => {
            let mut inner_pair = pair.into_inner();
            res.extend(build_token_tree_from_pair(inner_pair.next().expect(
                "line MUST always contain either string or modifier (check grammar)",
            )));
        }
        Rule::modifier => {
            let mut inner_pair = pair.into_inner();
            let modifier_name_pair = inner_pair
                .next()
                .expect("modifier name MUST be present (check grammar)");
            let modifier = Modifier::from_str(modifier_name_pair.as_span().as_str())
                .expect("modifier should be checked by grammar (check grammar)");
            let modifier_args_pair = inner_pair
                .next()
                .expect("modifier must contain MODIFIER_ARGS (check grammar)");
            let args = build_token_tree_from_pair(modifier_args_pair);
            res.push(Token::Modifier(modifier, args));
        }
        Rule::modifier_name => {
            unreachable!("Rule::modifier_name should be handled by Rule::modifier branch")
        }
        Rule::modifier_args => {
            // inner_pair contains a list of all arguments
            let inner_pair = pair.into_inner();
            
            // loop over the pairs and collect the arguments 
            for arg in inner_pair {
                res.extend(build_token_tree_from_pair(arg));
            }
        }
        Rule::string => res.push(Token::String(
            pair.into_inner()
                .next()
                .expect("string MUST always contain STRING_CONTENT")
                .as_span()
                .as_str()
                .to_string(),
        )),
        Rule::char | Rule::string_content => unreachable!(),
    }
    res
}
impl FromStr for Modifier {
    type Err = ParserError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "uppercase" => Ok(Modifier::Uppercase),
            "lowercase" => Ok(Modifier::Lowercase),
            "file" => Ok(Modifier::File),
            "combine" => Ok(Modifier::Combine),
            s => Err(ParserError::UnknownModifier(s.to_string())),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parser_parse_string_simple() {
        let unparsed = "\"string\"";

        let res = parse(unparsed).expect("valid token");

        assert_eq!(res, Token::String(From::from("string")));
    }

    #[test]
    fn test_parser_parse_string_escaped() {
        let unparsed = "\"\\\"pa55w0rd\"";
        let res = parse(unparsed).expect("valid token");

        assert_eq!(res, Token::String(From::from("\\\"pa55w0rd")));
    }

    #[test]
    fn test_uppercase_modifier_string_arg() {
        let unparsed = "uppercase(\"hello\")";
        let res = parse(unparsed).expect("valid token");

        assert_eq!(
            res,
            Token::Modifier(
                Modifier::Uppercase,
                vec![Token::String(From::from("hello"))]
            )
        );
    }

    #[test]
    fn test_lowercase_modifier_string_arg() {
        let unparsed = "lowercase(\"hello\")";
        let res = parse(unparsed).expect("valid token");

        assert_eq!(
            res,
            Token::Modifier(
                Modifier::Lowercase,
                vec![Token::String(From::from("hello"))]
            )
        );
    }

    #[test]
    fn test_nested_modifiers() {
        let unparsed =
            "combine(uppercase(file(\"firstname.txt\")), \".\", lowercase(file(\"lastname.txt\")))";
        let res = parse(unparsed).expect("valid token");

        assert_eq!(res, Token::Modifier(Modifier::Combine, vec![
            Token::Modifier(Modifier::Uppercase, vec![
                Token::Modifier(Modifier::File, vec![Token::String("firstname.txt".to_string())])
            ]),

            Token::String(".".to_string()),

            Token::Modifier(Modifier::Lowercase, vec![
                            Token::Modifier(Modifier::File, vec![Token::String("lastname.txt".to_string())])
            ])
        ]));
    }

    #[test]
    fn test_combine_modifier_three_args() {
        let unparsed = "combine(\"hello\", \",\", \" world\")";
        let res = parse(unparsed).expect("valid token");

        assert_eq!(res, Token::Modifier(
                Modifier::Combine, vec![
                    Token::String("hello".to_string()),
                    Token::String(",".to_string()),
                    Token::String(" world".to_string())
                ]
                ));
    }

    #[test]
    fn test_parse_uppercase_modifier_with_string_arument() {
        let raw = "uppercase(\"test\")";
        let res = parse(raw).expect("valid token");


        assert_eq!(res, Token::Modifier(Modifier::Uppercase, vec![Token::String(String::from("test"))]))
    }
}
