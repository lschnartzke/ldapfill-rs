pub(crate) mod file_cache;
pub(crate) mod parser;
mod types;

use file_cache::FileCache;
use parser::Modifier as ModifierKind;
use std::io;
use std::{path::PathBuf, str::FromStr};

pub use types::*;

use self::file_cache::get_file_cache;
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

    /// Traverses the modifier tree and collects all file arguments. Then adds all found
    /// files to the passed file cache.
    pub(crate) async fn load_files_into_cache(&self, cache: &mut FileCache) -> io::Result<()> {
        let args = self.collect_file_arguments();

        for arg in args {
            cache.load_file(PathBuf::from(arg)).await?;
        }

        Ok(())
    }

    /// Collects all arguments to `ModifierKind::File`s. Panics if the
    /// argument is not a string.
    pub fn collect_file_arguments(&self) -> Vec<&str> {
        let mut res = vec![];

        match *self {
            Token::String(_) => (),
            Token::Modifier(modifier, ref args) => match modifier {
                ModifierKind::File if args.len() == 1 => {
                    if let Token::String(s) = &args[0] {
                        res.push(s.as_str())
                    } else {
                        panic!("ModifierKind::File only accepts String arguments, got {args:#?} instead")
                    }
                }
                ModifierKind::Uppercase | ModifierKind::Combine | ModifierKind::Lowercase => res
                    .extend(
                        args.iter()
                            .flat_map(ModifierTree::collect_file_arguments)
                            .collect::<Vec<&str>>(),
                    ),

                ModifierKind::File => {
                    panic!("`ModifierKind::File` expects exactly one string argument")
                }
            },
        }

        res
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
            ModifierKind::File if args.len() == 1 => {
                // Not ideal, but I don't have time right now
                let buf = PathBuf::from(
                    args.iter()
                        .map(ModifierTree::apply)
                        .collect::<Vec<String>>()[0]
                        .as_str(),
                );
                get_file_cache().get_string(&buf).to_owned()
            }
            _ => panic!(
                "invalid number of arguments for {modifier:?}: {}",
                args.len()
            ),
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

    #[test]
    fn collect_file_modifier_arguments_simple_string() {
        let tree = Token::Modifier(
            ModifierKind::File,
            vec![Token::String(String::from("lastname.txt"))],
        );

        let args = tree.collect_file_arguments();

        assert_eq!(args, vec!["lastname.txt"]);
    }

    #[test]
    fn collect_file_modifier_arguments_nested_tree() {
        let tree = Token::Modifier(
            ModifierKind::Combine,
            vec![
                Token::Modifier(
                    ModifierKind::File,
                    vec![Token::String(String::from("firstname.txt"))],
                ),
                Token::String(String::from(" ")),
                Token::Modifier(
                    ModifierKind::File,
                    vec![Token::String(String::from("lastname.txt"))],
                ),
            ],
        );

        let args = tree.collect_file_arguments();

        assert_eq!(args, vec!["firstname.txt", "lastname.txt"]);
    }

    #[test]
    fn collect_file_modifier_arguments_deepish_nested() {
        let tree = Token::Modifier(
            ModifierKind::Combine,
            vec![
                Token::Modifier(
                    ModifierKind::Lowercase,
                    vec![Token::Modifier(
                        ModifierKind::File,
                        vec![Token::String(String::from("firstname.txt"))],
                    )],
                ),
                Token::String(String::from(".")),
                Token::Modifier(
                    ModifierKind::Lowercase,
                    vec![Token::Modifier(
                        ModifierKind::File,
                        vec![Token::String(String::from("lastname.txt"))],
                    )],
                ),
                Token::String(String::from("@")),
                Token::Modifier(
                    ModifierKind::Lowercase,
                    vec![Token::Modifier(
                        ModifierKind::File,
                        vec![Token::String(String::from("company.txt"))],
                    )],
                ),
                Token::String(String::from(".com")),
            ],
        );

        let args = tree.collect_file_arguments();

        assert_eq!(args, vec!["firstname.txt", "lastname.txt", "company.txt"]);
    }

    #[test]
    #[should_panic]
    fn collect_file_modifier_arguments_invalid_count() {
        let tree = Token::Modifier(
            ModifierKind::File,
            vec![
                Token::String(String::from("Hello")),
                Token::String(String::from(", world!")),
            ],
        );

        // panic
        tree.collect_file_arguments();
    }

    #[test]
    #[should_panic]
    fn collect_file_modifier_arguments_invalid_type() {
        let tree = Token::Modifier(
            ModifierKind::File,
            vec![Token::Modifier(
                ModifierKind::Uppercase,
                vec![Token::String(String::from("hello.txt"))],
            )],
        );

        // panic
        tree.collect_file_arguments();
    }

    #[test]
    fn collect_file_modifier_arguments_empty_list() {
        let tree = Token::Modifier(
            ModifierKind::Uppercase,
            vec![Token::String(String::from("lastname"))],
        );

        let args = tree.collect_file_arguments();

        assert_eq!(args, Vec::<&str>::new());
    }
}
