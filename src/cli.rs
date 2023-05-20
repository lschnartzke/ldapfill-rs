use clap::{Parser, Subcommand};

use crate::csv::CsvSender;

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
    
    #[arg(short = 'C', long)]
    /// If set, exports files corresponding to object classes and their 
    /// attribute values into csv files. The files will be named <objectClass>.csv.
    pub csv: bool,

    #[arg(short = 'D' , long, default_value_t = String::from("./csv"))]
    /// Set the directory to export the csv files to.
    pub csv_directory: String,

    /// The base entry to use when inserting
    pub base: String,

    #[command(subcommand)]
    pub cmd: MainCommand
}

#[derive(Debug, Clone, Subcommand)]
pub enum MainCommand {
    /// Export generated entries into an ldif file.
    Export {
        /// The file to export to
        #[arg(long)]
        file: String
    },
    /// Directly add the generated entries to a running server
    Insert {
        #[arg(short, long)]
        server: Option<String>,
        #[arg(short, long)]
        user: Option<String>,
        #[arg(short, long)]
        password: bool,

        #[arg(short = 'n', long, default_value_t = 1)]
        connections: usize
    }
}

impl CliArgs {
    pub async fn csv_sender(&self) -> anyhow::Result<Option<CsvSender>> {
        if self.csv {
            Ok(Some(crate::csv::start_csv_task(self.csv_directory.as_str()).await?))
        } else {
            Ok(None)
        }
    }
}
