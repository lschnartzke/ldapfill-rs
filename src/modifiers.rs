mod file_cache;
mod parser;
mod types;

use file_cache::FileCache;
use parser::Modifier as ModifierKind;
pub use types::*;

use self::parser::Token;

/// Base trait of a modifier. The modifier gets passed all string arguments or the
/// output of nested modifiers, in the order they are specified in the configuration.
pub trait Modifier {
    fn apply(args: Vec<String>) -> String;
}

pub type ModifierTree = Token;

impl ModifierTree {
    pub fn apply(&self) -> String {
        match *self {
            Token::String(ref s) => s.to_owned(),
            Token::Modifier(modifier, ref args) => self.apply_modifier(modifier, args),
        }
    }

    /// Traverses the ModifierTree for `Modifier::File`s and builds a file cache to use 
    /// for more efficient value generation.
    pub async fn build_file_cache(&self) -> FileCache {
        let mut files = Vec::new();
        


    }

    fn apply_modifier(&self, modifier: ModifierKind, args: &Vec<ModifierTree>) -> String {
        match modifier {
            ModifierKind::Uppercase => args
                .iter()
                .map(ModifierTree::apply)
                .collect::<Vec<String>>()
                .iter()
                .map(|s| s.to_uppercase())
                .collect::<Vec<String>>()
                .join(""),
            ModifierKind::Lowercase => args
                .iter()
                .map(ModifierTree::apply)
                .collect::<Vec<String>>()
                .iter()
                .map(|s| s.to_lowercase())
                .collect::<Vec<String>>()
                .join(""),
            ModifierKind::Combine => args
                .iter()
                .map(ModifierTree::apply)
                .collect::<Vec<String>>()
                .join(""),
            // TODO: Load random string from file cache.
            ModifierKind::File if args.len() == 1 => unimplemented!(),
            _ => panic!("invalid number of arguments for {modifier:?}: {}", args.len()),
        }
    }

}

#[cfg(test)]
mod test {
    use super::parser::*;
    use super::*;

    #[test]
    fn apply_string_modifier() {
        let modifier_tree = Token::String(String::from("Hello, world!"));

        assert_eq!("Hello, world!", modifier_tree.apply().as_str());
    }

    #[test]
    fn apply_uppercase_modifier() {
        let modifier_tree = Token::Modifier(
            ModifierKind::Uppercase,
            vec![Token::String("Hello, world!".to_string())],
        );

        assert_eq!("HELLO, WORLD!", modifier_tree.apply().as_str());
    }

    #[test]
    fn apply_lowercase_modifier() {
        let modifier_tree = Token::Modifier(
            ModifierKind::Lowercase,
            vec![Token::String(String::from("Hello, world!"))],
        );

        assert_eq!("hello, world!", modifier_tree.apply().as_str());
    }

    #[test]
    fn apply_combine_modifier() {
        let modifier_tree = Token::Modifier(
            ModifierKind::Combine,
            vec![
                Token::String(String::from("Hello")),
                Token::String(String::from(", ")),
                Token::String(String::from("world!")),
            ],
        );

        assert_eq!("Hello, world!", modifier_tree.apply().as_str())
    }
}
