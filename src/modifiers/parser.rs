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

use nom::branch;
use nom::bytes::complete as bytes;
use nom::character::complete as character;
use nom::combinator;
use nom::error;
use nom::multi;
use nom::sequence;
use nom::sequence::preceded;
use thiserror::Error;

pub type IResult<'e, I, T> = nom::IResult<I, T, error::VerboseError<&'e str>>;
pub type ParserResult<'e> = Result<Token, nom::Err<error::VerboseError<&'e str>>>;

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

#[derive(Debug)]
pub struct Parser {}

pub fn parse(input: &str) -> IResult<&str, Token> {
    branch::alt((
        error::context("parse-alt:parse_string", parse_string),
        error::context("parse-alt:parse_modifier", parse_modifier),
    ))(input)
}

fn parse_string(input: &str) -> IResult<&str, Token> {
    sequence::delimited(
        character::char('"'),
        bytes::escaped_transform(
            character::alphanumeric1,
            '\\',
            combinator::value("\"", bytes::tag("\"")),
        ),
        character::char('"'),
    )(input)
    .map(|s| (s.0, Token::String(s.1)))
}

fn parse_modifier(input: &str) -> IResult<&str, Token> {
    let (remaining, modifier_name) = error::context("parse_modifier", parse_modifier_name)(input)?;

    let (_, modifier) = error::context(
        "parse_modifier_select_modifier_type",
        branch::alt((
            combinator::value(Modifier::Combine, bytes::tag("combine")),
            combinator::value(Modifier::File, bytes::tag("file")),
            combinator::value(Modifier::Uppercase, bytes::tag("uppercase")),
            combinator::value(Modifier::Lowercase, bytes::tag("lowercase")),
        )),
    )(modifier_name)?;

    let (remaining, args) = parse_modifier_arguments(remaining)?;

    Ok((remaining, Token::Modifier(modifier, args)))
}

fn parse_modifier_name(input: &str) -> IResult<&str, &str> {
    error::context("parse_modifier_name", bytes::take_until1("("))(input)
}

fn parse_modifier_arguments(input: &str) -> IResult<&str, Vec<Token>> {
    error::context(
        "parse_modifier_arguments",
        // the take(1) is necessary as otherwise we fail due to trying to parse the unconsumed
        // opening parenthesis
        preceded(
            bytes::take(1usize),
            combinator::cut(sequence::terminated(
                multi::separated_list1(
                    preceded(character::space0, character::char(',')),
                    preceded(
                        character::space0,
                        branch::alt((preceded(character::char(','), parse), parse)),
                    ),
                ),
                preceded(character::space0, character::char(')')),
            )),
        ),
    )(input)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parser_parse_string_simple() {
        let unparsed = "\"string\"";

        let res = parse(unparsed);
        assert!(res.is_ok());
        let res = res.unwrap().1;

        assert_eq!(res, Token::String(String::from("string")));
    }

    #[test]
    fn test_parser_parse_string_escaped() {
        let unparsed = "\"\\\"pa55w0rd\"";

        let res = parse(unparsed);
        assert!(res.is_ok());
        let res = res.unwrap().1;

        assert_eq!(res, Token::String(String::from("\"pa55w0rd")));
    }

    #[test]
    fn test_uppercase_modifier_string_arg() {
        let unparsed = "uppercase(\"hello\")";
        let res = parse(unparsed);

        let res = res.unwrap().1;
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
        let res = parse(unparsed);

        let res = res.unwrap().1;
        assert_eq!(
            res,
            Token::Modifier(
                Modifier::Lowercase,
                vec![Token::String(From::from("hello"))]
            )
        );
    }

    #[test]
    fn test_combine_modifier_three_args() {
        let unparsed = "combine(\"hello\", \",\", \" world\")";
        let res = parse(unparsed);

        if let Err(nom::Err::Failure(e) | nom::Err::Error(e)) = res {
            let error = error::convert_error(unparsed, e);
            panic!("Failed to parse combine modifier: {error}");
        }
        let res = res.unwrap().1;
        assert_eq!(
            res,
            Token::Modifier(
                Modifier::Combine,
                vec![
                    Token::String(From::from("hello")),
                    Token::String(From::from(",")),
                    Token::String(From::from(" world"))
                ]
            )
        );
    }
}
