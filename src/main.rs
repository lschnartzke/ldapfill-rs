#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate pest_derive;
#[macro_use]
extern crate log;
#[macro_use]
extern crate anyhow;

use std::{collections::{HashMap, VecDeque}, time::Duration};
use tokio::sync::{mpsc, oneshot};

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
use modifiers::file_cache::{set_file_cache, FileCache};

static mut GENERATORS: Option<HashMap<String, EntryGenerator>> = None;
static mut HIERARCHY: Option<Vec<(String, u64)>> = None;
lazy_static! {
    static ref ARGS: CliArgs = CliArgs::parse();
}

pub type ResultReceiver = oneshot::Receiver<anyhow::Result<String>>;
pub type ResultSender = oneshot::Sender<anyhow::Result<String>>;
pub type Receiver = mpsc::UnboundedReceiver<(String, &'static EntryGenerator, ResultSender )>;
pub type Sender   = mpsc::UnboundedSender<(String, &'static EntryGenerator, ResultSender)>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // TODO: Add config option for this
    let args = &ARGS;
    let cfg = args.config_file.as_str();

    let config = Config::load_from_file(cfg)?;

    env_logger::builder().filter_level(config.log()).init();

    let Some(format_file_path) = (match config.defaults() {
        Some(defaults) => defaults.format_file().or(args.format_file.as_deref()),
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

    let ldap = connect_to_ldap_using_args_or_config(args, &config).await?;

    // you're not supposed to fight or work around the borrow checker,
    // but I need to hand in bachelors thesis in less than 2 months, so
    // I don't have the time to work around this properly.
    unsafe {
        GENERATORS = Some(generators);
        HIERARCHY = Some(hierarchy_weights);
    }
    let generators = unsafe { GENERATORS.as_ref().unwrap() };
    let hierarchy = unsafe { HIERARCHY.as_ref().unwrap() };

    if let Err(e) = fill_ldap(ldap, generators, hierarchy, args.base.as_str()).await {
        error!("Failed to fill directory: {e}");
        return Err(e);
    }

    Ok(())
}

// creates tasks to fill up the directory in parallel, task size is determined by available cpus
// and amount of entries to generate. Whatever is smaller determines the task size.
async fn fill_ldap(
    conn: Ldap,
    generators: &'static HashMap<String, EntryGenerator>,
    hierarchy: &'static [(String, u64)],
    base: &'static str,
) -> anyhow::Result<()> {
    let (_, count) = &hierarchy[0];
    let cpus = num_cpus::get() as u64;
    let task_count = if cpus < *count {
        cpus * 12 // for good measure
    } else {
        *count
    };

    info!("Generating entries using {task_count} tasks");

    // Generate channels and tasks
    let channels: VecDeque<Sender> = {
        let mut res = VecDeque::with_capacity(task_count as usize);
        for _ in 0..task_count {
            let (tx, rx) = mpsc::unbounded_channel();
            let cc = conn.clone();
            
            tokio::spawn(async move { fill_level(cc, rx).await });
            res.push_back(tx);
        }
        res
    };

    let start = std::time::Instant::now();
    let mut dns: Vec<String> = vec![base.to_owned()];

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

                    if count == 0 {
                        break 'outer;
                    }
                }
            }

             for result in results.drain(..results.len()){
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
/*
        for dn in dns.iter() {
            for _ in 0..count {
                let cc = conn.clone();
                let dn = dn.clone();
                let task =
                    tokio::spawn(async move { fill_level(cc, generator, dn).await });
                tasks.push(task);
            }
        }
        dns.clear();

        for task in tasks {
            match task.await? {
                Ok(dn) => dns.push(dn),
                Err(e) => {
                    debug!("task error: {e:#?}");
                    error!("Got task error when filling directory: {e}");
                }
            }

        }
*/
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
async fn fill_level(
    mut ldap: Ldap,
    mut rx: Receiver
    ) {
    
    while let Some((dn, generator, result_sender)) = rx.recv().await {
        let res = add_entry(&mut ldap, generator, dn).await;

        if let Err(_) = result_sender.send(res) {
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
) -> anyhow::Result<String> {
    let (rdn, entry) = generator.generate_entry();
    let dn = format!("{rdn},{base}");

    if let Err(e) = ldap.add(dn.as_str(), entry.clone()).await?.success() {
        debug!("entry {entry:#?} caused error during add: dn: {dn} error: {e:#?}");
        warn!("Error during entry insertion: {e}");
    }

    Ok(dn)
}

// ATM this function is creating more objects than it should, as it using the entry count of the
// current hierarchy level. However, this count also specifies the amount of tasks to use in total,
// creating count^2 entries, if possible.
async fn fill_task(
    mut conn: Ldap,
    generators: &HashMap<String, EntryGenerator>,
    hierarchy: &Vec<(String, u64)>,
    base: &str,
) -> anyhow::Result<()> {
    debug!("Fill task");
    let base = base.to_string();
    let mut level_rdns: Vec<String> = vec![];
    for (class, count) in hierarchy {
        debug!("Generating entry for class {class}, {count} entries in total");
        let generator = &generators[class];

        for _ in 0..*count {
            let (rdn, entry) = generator.generate_entry();
            let entry_dn = format!("{},{}", rdn, base);

            debug!("Inserting entry {entry:#?} at {entry_dn}");

            if let Err(e) = conn.add(entry_dn.as_ref(), entry).await?.success() {
                warn!("Failed to insert entry at {base}: {e}");
            }
            level_rdns.push(entry_dn);
        }
    }

    Ok(())
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

async fn connect_to_ldap_using_args_or_config(
    args: &CliArgs,
    config: &Config,
) -> Result<Ldap, anyhow::Error> {
    let (server, user, password) = match config.ldap() {
        Some(LdapConfig {
            server,
            user,
            password,
        }) => (server.clone(), user.clone(), password.clone()),
        None => ldap_settings_from_args(args)?,
    };

    info!("Connecting to {server}...");
    let (conn, mut ldap) = LdapConnAsync::new(server.as_str()).await?;
    tokio::spawn(async move {
        if let Err(e) = conn.drive().await {
            debug!("LDAP conn error: {e:#?}");
            error!("LDAP Connection exited with error: {e}");
            panic!("{e}");
        }
    });

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
        false => String::new(),
    };

    Ok((server, user, password))
}
