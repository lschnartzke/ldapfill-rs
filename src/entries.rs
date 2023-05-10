use crate::modifiers::{file_cache::FileCache, ModifierTree};
use std::collections::{HashMap, HashSet};

/// The entry generator is used to generate entries of one specific object class.
#[derive(Debug)]
pub struct EntryGenerator {
    object_class: String,
    rdn_attribute: String,
    attributes: HashMap<String, ModifierTree>,
}

impl EntryGenerator {
    /// Creates a new `EntryGenerator` using the provided attributes.
    pub fn new(
        object_class: String,
        rdn_attribute: String,
        attributes: HashMap<String, ModifierTree>,
    ) -> Self {
        Self {
            object_class,
            rdn_attribute,
            attributes,
        }
    }

    pub fn object_class(&self) -> &str {
        self.object_class.as_str()
    }

    pub fn generate_entry(&self) -> (String, Vec<(String, HashSet<String>)>) {
        let mut entry = vec![("objectclass".to_string(), HashSet::from([self.object_class.clone()]))];
        let mut rdn: Option<String> = None;

        for (attribute, modifier) in self.attributes.iter() {
            let (key, value) = (attribute.as_str(), modifier.apply());
            if key == self.rdn_attribute {
                rdn = rdn.or_else(||Some(value.clone()));
            }

            entry.push((key.to_owned(), HashSet::from([value])));
        }

        (format!("{}={}", self.rdn_attribute, rdn.unwrap()), entry)
    }

    pub async fn load_files(&self, cache: &mut FileCache) -> std::io::Result<()> {
        for tree in self.attributes.values() {
            tree.load_files_into_cache(cache).await?;
        }

        Ok(())
    }
}
