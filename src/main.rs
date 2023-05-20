#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate pest_derive;
#[macro_use]
extern crate log;
#[macro_use]
extern crate anyhow;

use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};

use anyhow::bail;
use clap::Parser;

mod cli;
mod cmd;
mod config;
mod csv;
mod entries;
mod error;
mod format;
mod ldap_pool;
mod modifiers;
mod types;
mod progress;
mod ldif;

use cli::CliArgs;
use cli::MainCommand;
use config::{Config, LdapConfig};
use entries::EntryGenerator;
use format::Format;
use ldap_pool::LdapPool;
use modifiers::file_cache::{set_file_cache, FileCache};
use crate::{csv::{CsvSender, start_csv_task}, progress::ProgressMessage};

static mut GENERATORS: Option<HashMap<String, EntryGenerator>> = None;
static mut HIERARCHY: Option<Vec<(String, u64)>> = None;
lazy_static! {
    static ref ARGS: CliArgs = CliArgs::parse();
}

pub type ResultReceiver = oneshot::Receiver<anyhow::Result<String>>;
pub type ResultSender = oneshot::Sender<anyhow::Result<String>>;
pub type Receiver = mpsc::UnboundedReceiver<(String, &'static EntryGenerator, ResultSender)>;
pub type Sender = mpsc::UnboundedSender<(String, &'static EntryGenerator, ResultSender)>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // TODO: Add config option for this
    let args = &ARGS;
    let cfg = args.config_file.as_str();

    let config = Config::load_from_file(cfg)?;

    let Some(format_file_path) = (match config.defaults() {
        Some(defaults) => args.format_file.as_deref().or(defaults.format_file()),
        None => args.format_file.as_deref(),
    }) else {
        bail!("path to format file must be specified either in the configuration or using the --format-file option");
    };

    info!("Trying to load format file at {format_file_path}");
    let format = Format::load_from_file(format_file_path)?;
    let hierarchy_weights = format.hierarchy_tuples();
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

    // you're not supposed to fight or work around the borrow checker,
    // but I need to hand in a bachelors thesis in less than 2 months, so
    // I don't have the time to work around this properly.
    unsafe {
        GENERATORS = Some(generators.clone());
        HIERARCHY = Some(hierarchy_weights.clone());
    }
    
    cmd::set_hierarchy(hierarchy_weights);
    cmd::set_generators(generators);

    let res = match args.cmd {
        MainCommand::Export { .. } => cmd::export_cmd(&args).await,
        MainCommand::Insert { .. } => cmd::insert_cmd(&args).await
    };

    res
}

async fn build_file_cache<'e, T>(generators: T) -> anyhow::Result<()>
where
    T: Iterator<Item = &'e EntryGenerator>,
{
    let mut cache = FileCache::new();

    for generator in generators {
        generator.load_files(&mut cache).await?;
    }

    set_file_cache(cache);

    Ok(())
}
