#[macro_use]
extern crate pest_derive;
#[macro_use]
extern crate log;
#[macro_use]
extern crate anyhow;

use std::error::Error;


use anyhow::bail;
use clap::Parser;

mod cli;
mod config;
mod entries;
mod error;
mod format;
mod modifiers;

use cli::CliArgs;
use config::{Config, LdapConfig};
use entries::EntryGenerator;
use format::Format;
use ldap3::{Ldap, LdapConnAsync};
use modifiers::file_cache::FileCache;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // TODO: Add config option for this
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();
    let args: CliArgs = CliArgs::parse();
    let cfg = args.config_file.as_str();

    info!("Trying to load config file at {cfg}");
    let config = Config::load_from_file(cfg)?;

    let Some(format_file_path) = (match config.defaults() {
        Some(defaults) => defaults.format_file().or(args.format_file.as_ref().map(String::as_str)),
        None => args.format_file.as_ref().map(String::as_str)
    }) else {
        bail!("path to format file must be specified either in the configuration or using the --format-file option");
    };

    info!("Trying to load format file at {format_file_path}");
    let format = Format::load_from_file(format_file_path)?;
    let generators = match format.to_entry_generators() {
        Ok(g) => g,
        Err(e) => {
            error!("Failed to build entry generators from format file: {e}");
            return Err(e);
        }
    };

    if let Err(e) = build_file_cache(generators.values()).await {
        error!("Failed to build file cache: {e}");
        return Err(e);
    }

    for (object_class, generator) in generators.iter() {
        info!("Generated entry: {:#?}", generator.generate_entry());
    }
    
    let ldap = connect_to_ldap_using_args_or_config(&args, &config).await?;


    Ok(())
}

async fn build_file_cache<'e, T>(generators: T) -> anyhow::Result<()>
where T: Iterator<Item = &'e EntryGenerator> {
    let mut cache = FileCache::new();

    for generator in generators {
        generator.load_files(&mut cache).await?;
    }

    
    Ok(())
}

async fn connect_to_ldap_using_args_or_config(args: &CliArgs, config: &Config) -> Result<Ldap, anyhow::Error> {
    let (server, user, password) = match config.ldap() {
        Some(LdapConfig {server, user, password}) => (server.clone(), user.clone(), password.clone()),
        None => ldap_settings_from_args(args)?
    };

    info!("Connecting to {server}...");
    let (conn, mut ldap) = LdapConnAsync::new(server.as_str()).await?;
    ldap3::drive!(conn);

    info!("Attempting to bind to the server using {user}...");
    ldap.simple_bind(user.as_str(), password.as_str()).await?;

    Ok(ldap)

}

fn ldap_settings_from_args(args: &CliArgs) -> Result<(String, String, String), anyhow::Error> {
    let Some(user) = args.user.clone() else {
        bail!("Missing required --user option");
    };

    let Some(server) = args.server.clone() else {
        bail!("Missing required --server option");
    };

    let password = match args.password {
        true => rpassword::prompt_password("Password: ")?,
        false => String::new()
    };

    Ok((server, user, password))
}
