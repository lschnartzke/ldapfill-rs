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

static mut FILE_CACHE: Option<FileCache> = None;

/// A FileCache holds a given number of text files in memory, stored on a line-by-line basis.
/// File modifiers may request data from a given file, in which case a random line will be 
/// returned. There is no guarantee, that a line will only be used once, though big enough
/// files are unlikely to produce duplicates.
#[derive(Debug)]
pub struct FileCache {
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

        if lines.is_empty() {
            panic!("File must not be empty: {path:?}");
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

/// Sets the global file cache to use when retrieving values for modifiers.
///
/// # Panics
/// Panics if you try to swap the file cache.
pub fn set_file_cache(cache: FileCache) {
    unsafe {
        match FILE_CACHE {
            Some(_) => panic!("CANNOT SET FILE_CACHE TWICE"),
            None => FILE_CACHE = Some(cache)
        }
    }
}

/// Returns a reference to the global file cache
///
/// # Panics
/// Panics if the file cache has not been set
pub fn get_file_cache() -> &'static FileCache {
    unsafe {
        match FILE_CACHE {
            Some(ref c) => c,
            None => panic!("get_file_cache() MUST be called AFTER set_file_cache")
        }
    }
}


impl Default for FileCache {
    fn default() -> Self {
        FileCache::new()
    }
}
