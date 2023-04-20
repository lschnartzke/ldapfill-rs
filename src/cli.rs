use clap::Parser;

#[derive(Parser)]
#[clap(version, author, about, long_about = None)]
pub struct CliArgs {
    #[arg(short, long, default_value_t = String::from("/etc/ldapfill.toml"))]
    /// The config file to use
    config_file: String,

    #[arg(short, long)]
    /// The user to use when connecting to the LDAP server. Overrides the value specified in the
    /// config.
    user: Option<String>,

    #[arg(short, long)]
    /// Whether to use a password. If set, the user will be prompted for the login password before
    /// connecting to the server. Overrides the value specified in the config, if present.
    password: bool
}
