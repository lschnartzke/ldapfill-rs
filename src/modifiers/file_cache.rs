//! To avoid reading files multiple times and to improve perfomance,
//! we use a global FileCache, which reads all required `Modifier::File`s
//! into memory (line by line) and returns a random line from a specified
//! file every time a file modifier is applied.
//!
//! Every file modifier, internally, has a reference to the global FileCache.

use std::{collections::HashMap, path::PathBuf};
use std::io;
use tokio::fs;
use tokio::io::{self as tio, AsyncBufReadExt};
use rand::{thread_rng, Rng};

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

    /// Loads an entire file into memory, allowing other functions to access random values from the
    /// file. The file's lines are stored in a hash map with the path being the key.
    pub async fn load_file(&mut self, path: PathBuf) -> Result<(), io::Error> {
        let file = fs::File::open(&path).await?;
        let reader = tio::BufReader::new(file);
        let mut line_reader = reader.lines();
        let mut lines = vec![];
        
        while let Some(line) = line_reader.next_line().await? {
            lines.push(line);
        }

        self.cache.insert(path, lines);

        Ok(())
    }

    /// Returns a random line of the specified `file`.
    ///
    /// # Panics
    /// Will panic if the file is not present in the cache.
    pub fn get_string(&self, file: &PathBuf) -> &'_ str {
        let index = thread_rng().gen_range(0..self.cache[file].len());

        self.cache[file][index].as_str()
    }
} 


