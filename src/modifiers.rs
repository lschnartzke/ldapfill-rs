mod parser;
mod types;
mod file_cache;

use parser::Modifier as ModifierKind;
pub use types::*;

/// Base trait of a modifier. The modifier gets passed all string arguments or the 
/// output of nested modifiers, in the order they are specified in the configuration.
pub trait Modifier {
    fn apply(args: Vec<String>) -> String;

}

#[derive(Debug)]
struct ModifierTreeNode {
    modifier: ModifierKind,
    args: Vec<ModifierTreeNode>
}

#[derive(Debug)]
pub struct ModifierTree {
    root: ModifierTreeNode
}
