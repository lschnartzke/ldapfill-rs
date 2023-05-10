use log::LevelFilter;
use serde::Deserialize;
use anyhow::Error;
use super::cli::CliArgs;

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

#[derive(Debug, Clone, Deserialize)]
pub struct LdapConfig {
    // The server to connect to
    pub server: String,
    // The user to use for connecting to the server 
    pub user: String,
    // The password to use when connecting.
    pub password: String,

    connections: usize
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
    /// Tries to create an `LdapConfig` using the provided `CliArgs`.
    /// Returns `None` if the cli args are missing one or more parameters.
    pub fn from_args(args: &CliArgs) -> Option<Self> {
        let Some(user) = args.user.clone() else { return None; };
        let Some(server) = args.server.clone() else { return None; };
        let connections = args.connections.unwrap_or(1);
        let password = match args.password {
            true => rpassword::prompt_password(format!("Password for {server}: ")).unwrap(),
            false => "".to_string()
        };

        Some(Self {
            user,
            server,
            password,
            connections
        })

    }

    /// Merges the present values of `args` with `self`, effectively overwriting 
    /// values. 
    pub fn merge_args(&mut self, args: &CliArgs) {
        if let Some(ref user) = args.user {
            self.user = user.to_owned();
        }

        if let Some(ref server) = args.server {
            self.server = server.to_owned();
        }

        if args.password {
            self.password = rpassword::prompt_password(format!("Password for {}: ", self.server)).unwrap();
        }
    }


    pub fn server(&self) -> &str {
        self.server.as_str()
    }

    pub fn user(&self) -> &str {
        self.user.as_str()
    }

    pub fn password(&self) -> &str {
        self.password.as_str()
    }

    pub fn connections(&self) -> usize {
        self.connections
    }
}

impl DefaultSettings {
    pub fn format_file(&self) -> Option<&str> {
        self.format_file.as_deref()
    }
}

fn default_log() -> LevelFilter {
    LevelFilter::Info
}
