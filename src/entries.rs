use crate::modifiers::{ModifierTree, file_cache::FileCache};
use std::collections::{HashMap, HashSet};

/// The entry generator is used to generate entries of one specific object class.
#[derive(Debug)]
pub struct EntryGenerator {
    object_class: String,
    attributes: HashMap<String, ModifierTree>,
}

impl EntryGenerator {
    /// Creates a new `EntryGenerator` using the provided attributes.
    pub fn new(object_class: String, attributes: HashMap<String, ModifierTree>) -> Self {
        Self {
            object_class,
            attributes
        }
    }

    pub fn generate_entry<'e>(&'e self) -> Vec<(&'e str, HashSet<String>)> {
        let mut entry = vec![("objectclass", HashSet::from([self.object_class.clone()]))];
        
        entry.extend(
            self.attributes.iter().map(|(attribute, modifier)| (attribute.as_str(), HashSet::from([modifier.apply()])))
        );
        
        entry
    }

    pub async fn load_files(&self, cache: &mut  FileCache) -> std::io::Result<()> {
        for tree in self.attributes.values() {
            tree.load_files_into_cache(cache).await?;
        }

        Ok(())
    }
}




