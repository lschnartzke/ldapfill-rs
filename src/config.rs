use log::LevelFilter;
use serde::Deserialize;
use anyhow::Error;

use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_log")]
    log: LevelFilter,

    ldap: Option<LdapConfig>,
    
    defaults: Option<DefaultSettings>

}

#[derive(Debug, Deserialize)]
pub struct DefaultSettings {
    #[serde(rename(deserialize = "format-file"))]
    format_file: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LdapConfig {
    // The server to connect to
    pub server: String,
    // The user to use for connecting to the server 
    pub user: String,
    // The password to use when connecting.
    pub password: String
}

impl Config {
    pub fn load_from_file<A: AsRef<Path>>(path: A) -> Result<Config, Error> {
        let string = std::fs::read_to_string(path)?;
        
        Ok(toml::from_str(string.as_str())?)
    }

    pub fn log(&self) -> LevelFilter {
        self.log
    }

    pub fn ldap(&self) -> Option<&LdapConfig> {
       self.ldap.as_ref()
    }

    pub fn defaults(&self) -> Option<&DefaultSettings> {
        self.defaults.as_ref()
    }
}

impl LdapConfig {
    pub fn server(&self) -> &str {
        self.server.as_str()
    }

    pub fn user(&self) -> &str {
        self.user.as_str()
    }

    pub fn password(&self) -> &str {
        self.password.as_str()
    }
}

impl DefaultSettings {
    pub fn format_file(&self) -> Option<&str> {
        self.format_file.as_ref().map(String::as_str)
    }
}

fn default_log() -> LevelFilter {
    LevelFilter::Info
}
