use clap::Parser;

#[derive(Parser)]
#[clap(version, author, about, long_about = None)]
pub struct CliArgs {
    #[arg(short, long, default_value_t = String::from("/etc/ldapfill.toml"))]
    /// The config file to use
    pub config_file: String,
    
    /// The file specifying the format to use when generating LDAP-Entries. specifying
    /// this will override the file specified in the configuration, if any. Note that this option 
    /// is required if there is no file set in the configuration.
    #[arg(short, long)]
    pub format_file: Option<String>,

    #[arg(short, long)]
    /// The user to use when connecting to the LDAP server. Overrides the value specified in the
    /// config.
    pub user: Option<String>,

    #[arg(short, long)]
    /// Whether to use a password. If set, the user will be prompted for the login password before
    /// connecting to the server. Overrides the value specified in the config, if present.
    pub password: bool,
    
    #[arg(short = 'C', long)]
    /// If set, exports files corresponding to object classes and their 
    /// attribute values into csv files. The files will be named <objectClass>.csv.
    pub csv: bool,

    #[arg(short = 'D' , long, default_value_t = String::from("./csv"))]
    /// Set the directory to export the csv files to.
    pub csv_directory: String,

    #[arg(short, long)]
    /// The server to connect to. Overrides the default specified in the 
    /// configuration file, if present.
    pub server: Option<String>,

    /// How many (simultaneous) connections to the server should be created.
    /// The connections will be shared between worker tasks to achieve a 
    /// higher throughput.
    #[arg(short = 'n', long)]
    pub connections: Option<usize>,

    /// The base entry to use when inserting
    pub base: String


}
