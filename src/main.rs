#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate pest_derive;
#[macro_use]
extern crate log;
#[macro_use]
extern crate anyhow;

use std::collections::{HashMap, VecDeque};
use tokio::sync::{mpsc, oneshot};

use anyhow::bail;
use clap::Parser;

mod cli;
mod config;
mod csv;
mod entries;
mod error;
mod format;
mod ldap_pool;
mod modifiers;
mod types;
mod progress;

use cli::CliArgs;
use config::{Config, LdapConfig};
use entries::EntryGenerator;
use format::Format;
use ldap3::Ldap;
use ldap_pool::LdapPool;
use modifiers::file_cache::{set_file_cache, FileCache};
use crate::csv::{CsvSender, start_csv_task};

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

    env_logger::builder().filter_level(config.log()).init();

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

    let Some(mut ldap_config) = config.ldap().cloned().or_else(|| LdapConfig::from_args(args)) else {
        error!("Missing required parameters to connect to server. Check config or provide via cli (--help for more info)");
        bail!("Missing required server information");
    };

    ldap_config.merge_args(args);

    // you're not supposed to fight or work around the borrow checker,
    // but I need to hand in bachelors thesis in less than 2 months, so
    // I don't have the time to work around this properly.
    unsafe {
        GENERATORS = Some(generators);
        HIERARCHY = Some(hierarchy_weights);
    }
    let generators = unsafe { GENERATORS.as_ref().unwrap() };
    let hierarchy = unsafe { HIERARCHY.as_ref().unwrap() };

    let csv_sender = if args.csv {
        Some(start_csv_task(args.csv_directory.as_str()).await?)
    } else { None };

    if let Err(e) = fill_ldap(ldap_config, generators, hierarchy, args.base.as_str(), csv_sender).await {
        error!("Failed to fill directory: {e}");
        return Err(e);
    }

    Ok(())
}

// creates tasks to fill up the directory in parallel, task size is determined by available cpus
// and amount of entries to generate. Whatever is smaller determines the task size.
async fn fill_ldap(
    ldap_config: LdapConfig,
    generators: &'static HashMap<String, EntryGenerator>,
    hierarchy: &'static [(String, u64)],
    base: &'static str,
    csv_sender: Option<CsvSender>
) -> anyhow::Result<()> {
    let (_, count) = &hierarchy[0];
    let cpus = num_cpus::get() as u64;
    let entry_count = hierarchy.iter().map(|(_, c)| *c).product::<u64>();
    let task_count = if cpus < *count {
        cpus * 12 // for good measure
    } else {
        *count
    };

    let pool = LdapPool::new(ldap_config).await?;

    info!("Generating up to {entry_count} entries using {task_count} tasks");

    // Generate channels and tasks
    let channels: VecDeque<Sender> = {
        let mut res = VecDeque::with_capacity(task_count as usize);
        for _ in 0..task_count {
            let (tx, rx) = mpsc::unbounded_channel();
            let cc = pool.get_conn();

            let sender = csv_sender.clone();
            tokio::spawn(async move { fill_level(cc, rx, sender).await });
            res.push_back(tx);
        }
        res
    };

    let start = std::time::Instant::now();
    let mut dns: Vec<String> = vec![base.to_owned()];
    let  ptx = progress::start_progress_task(entry_count).await;

    for (object_class, count) in hierarchy.iter() {
        // performance-wise this feels terrible, but maybe the compiler can optimise this away
        // (please)
        //let mut tasks = vec![];
        let generator = &generators[object_class];

        let mut results: Vec<ResultReceiver> = vec![];
        let mut new_dns = vec![];
        for dn in dns.iter() {
            let mut count = *count;
            'outer: while count != 0 {
                let it = channels.iter();
                for tx in it {
                    let (sender, result) = oneshot::channel();
                    results.push(result);
                    count -= 1;

                    if let Err(e) = tx.send((dn.clone(), generator, sender)) {
                        warn!("Error when trying to send data to fill-task: {e}");
                    }

                    drop(ptx.send(()));

                    if count == 0 {
                        break 'outer;
                    }
                }
            }

            for result in results.drain(..results.len()) {
                let res = result.await;
                match res {
                    Ok(Ok(res)) => new_dns.push(res),
                    Ok(Err(e)) => warn!("Got result; it failed: {e}"),
                    Err(e) => warn!("Failed to receive result: {e}"),
                }
            }
        }
        dns.clear();
        dns.extend(new_dns);
    }

    let end = std::time::Instant::now();

    info!(
        "Generated {} entries in {}ms",
        hierarchy.iter().map(|(_, c)| *c).product::<u64>(),
        (end - start).as_millis()
    );

    Ok(())
}

//
async fn fill_level(mut ldap: Ldap, mut rx: Receiver, csv_sender: Option<CsvSender>) {
    while let Some((dn, generator, result_sender)) = rx.recv().await {
        let res = add_entry(&mut ldap, generator, dn, csv_sender.as_ref()).await;

        if result_sender.send(res).is_err() {
            warn!("Failed to return result, error: channel closed");
        }
    }
}
///
/// Creates one entry for the provided base level. Returns the generated entry dn.
async fn add_entry(
    ldap: &mut Ldap,
    generator: &EntryGenerator,
    base: String,
    csv_sender: Option<&CsvSender>
) -> anyhow::Result<String> {
    let (rdn, entry) = generator.generate_entry();
    let dn = format!("{rdn},{base}");

    if let Err(e) = ldap.add(dn.as_str(), entry.clone()).await?.success() {
        debug!("entry {entry:#?} caused error during add: dn: {dn} error: {e:#?}");
        warn!("Error during entry insertion: {e}");
    }

    // even if the entry failed to add, we can now test invalid entries as well, yay
    if let Some(sender) = csv_sender {
        // Ignore the result. If this fails, the writer task quit early. In that case,
        // we have different problems as we're holding a sender handle and the task 
        // should not quit unless all senders are dropped.
        drop(sender.send((generator.object_class().to_owned(), entry)));
    }

    Ok(dn)
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
