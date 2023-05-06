use std::{collections::HashMap, path::Path};
use serde::Deserialize;
use toml::Deserializer;

use crate::entries::EntryGenerator;
use crate::modifiers::parser;

pub type Fields = HashMap<String, HashMap<String, String>>;

/// The file specifies how ldap entries should be built.
///
/// The `hierarchy` field describes how the entries will be 
/// inserted in the directory. 
///
/// The `weight` describes how many entries are to be generated for 
/// each level in the hierarchy.
#[derive(Debug, Deserialize)]
pub struct Format {
    hierarchy: Vec<String>,
    count: Vec<u64>,
    #[serde(flatten)]
    fields: Fields
}

impl Format {
    pub fn load_from_file<A: AsRef<Path>>(path: A) -> Result<Format, anyhow::Error> {
        let string = std::fs::read_to_string(path)?;

        let deserializer = Deserializer::new(string.as_str());

        let format: Format = Deserialize::deserialize(deserializer)?;

        if format.count.len() != format.hierarchy.len() {
            bail!("count and hierarchy must have the same number of elements");
        }

        for class in &format.hierarchy[..] {
            if !format.fields.contains_key(class) {
                bail!("All values of hierarchy must correspond to an object class. Could not find: {class}");
            }
        }

        Ok(format)
    }

    pub fn hierarchy_tuples(&self) -> Vec<(String, u64)> {
        self.hierarchy.iter().cloned().zip(self.count.iter().copied()).collect()
    }

    pub fn to_entry_generators(self) -> Result<HashMap<String, EntryGenerator>, anyhow::Error> {
        let mut generators = HashMap::new();
        
        for (object_class, attribute_map) in self.fields.iter() {
            let mut object_attributes = HashMap::new();
            let mut rdn_attribute: Option<String> = None;

            for (attribute, modifier_string) in attribute_map.iter() {
                if attribute == "rdn" {
                    rdn_attribute = rdn_attribute.or_else(|| Some(modifier_string.clone()));
                    continue;
                }

                let tree = parser::parse(modifier_string)?;

                object_attributes.insert(attribute.clone(), tree);
            }

            let Some(rdn_attribute) = rdn_attribute else {
                bail!("rdn attribute MUST be present for object class {object_class}");
            };
            let generator = EntryGenerator::new(object_class.clone(), rdn_attribute, object_attributes);
            
            generators.insert(object_class.clone(), generator);
        }

        Ok(generators)
    }

    pub fn fields(&self) -> &Fields {
        &self.fields
    }
}
