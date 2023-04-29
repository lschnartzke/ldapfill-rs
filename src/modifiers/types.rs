

/// Simple modifier that returns a fixed string everytime it is applied
#[derive(Debug)]
pub struct StringModifier {
    string: String
}

/// Converts the provided string to uppercase representation.
#[derive(Debug)]
pub struct UppercaseModifier;

/// Converts the provided string(s) to lowercase representation
#[derive(Debug)]
pub struct LowercaseModifier;

/// Combines the provided arguments into a single string
#[derive(Debug)]
pub struct CombineModifier;

/// Reads the provided file into memory and returns a random **line** when
/// the modifier is applied. Lines might be reused.
#[derive(Debug)]
pub struct FileModifier {

}
