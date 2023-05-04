use std::{collections::HashMap, path::Path};
use serde::Deserialize;
use toml::Deserializer;

use crate::entries::EntryGenerator;
use crate::modifiers::parser;

pub type Fields = HashMap<String, HashMap<String, String>>;

// The file specifies how ldap entries should be built.
#[derive(Debug, Deserialize)]
pub struct Format {
    #[serde(flatten)]
    fields: Fields
}

impl Format {
    pub fn load_from_file<A: AsRef<Path>>(path: A) -> Result<Format, anyhow::Error> {
        let string = std::fs::read_to_string(path)?;

        let deserializer = Deserializer::new(string.as_str());

        Ok(Deserialize::deserialize(deserializer)?)
    }

    pub fn to_entry_generators(self) -> Result<HashMap<String, EntryGenerator>, anyhow::Error> {
        let mut generators = HashMap::new();
        
        for (object_class, attribute_map) in self.fields.iter() {
            let mut object_attributes = HashMap::new();

            for (attribute, modifier_string) in attribute_map.iter() {
                let tree = parser::parse(modifier_string)?;

                object_attributes.insert(attribute.clone(), tree);
            }

            let generator = EntryGenerator::new(object_class.clone(), object_attributes);
            
            generators.insert(object_class.clone(), generator);
        }

        Ok(generators)
    }

    pub fn fields(&self) -> &Fields {
        &self.fields
    }
}
