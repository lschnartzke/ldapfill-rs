//! To avoid reading files multiple times and to improve perfomance,
//! we use a global FileCache, which reads all required `Modifier::File`s
//! into memory (line by line) and returns a random line from a specified
//! file every time a file modifier is applied.
//!
//! Every file modifier, internally, has a reference to the global FileCache.

use std::{collections::HashMap, path::PathBuf};

/// A FileCache holds a given number of text files in memory, stored on a line-by-line basis.
/// File modifiers may request data from a given file, in which case a random line will be 
/// returned. There is no guarantee, that a line will only be used once, though big enough
/// files are unlikely to produce duplicates.
#[derive(Debug)]
pub(crate) struct FileCache {
    cache: HashMap<PathBuf, Vec<String>>
}

impl FileCache {
    pub fn new() -> Self {
        Self {
            cache: Default::default()
        }
    }
}
