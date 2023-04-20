use serde::Deserialize;
use std::collections::HashMap;


#[derive(Debug, Deserialize)]
pub struct Config {
    ldap: Option<LdapConfig>,

    #[serde(flatten)]
    objects: HashMap<String, String>

}

#[derive(Debug, Deserialize)]
pub struct LdapConfig {
    // The server to connect to
    server: String,
    // The user to use for connecting to the server 
    user: String,
    // Optionally, the password to use when connecting.
    password: Option<String>
}
