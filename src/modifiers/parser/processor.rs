use super::ParserResult;
use super::*;

pub type ProcessorResult = Result<Token, ParserError>;

/// A simple wrapper around a string that can be used to process a string 
/// to parse it into the contained tokens.
struct Processor<'p> {
    string: &'p str,
    current_char: usize,
}

impl<'p> Processor<'p> {
    /// Creates a new processor that operates on the given string.
    pub fn new(string: &'p str) -> Self {
        Self {
            string,
            current_char: 0
        }
    }

    pub fn next_token(&mut self) -> ProcessorResult {
        
        Err(ParserError::EmptyValue)
    }


}
