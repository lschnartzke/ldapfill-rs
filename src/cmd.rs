use indicatif::{ProgressBar, ProgressStyle};
use tokio_stream::{wrappers::{ReceiverStream, UnboundedReceiverStream}, StreamExt};

use crate::{cli::{CliArgs, MainCommand}, entries::EntryGenerator, config::LdapConfig, ldap_pool::LdapPool};
use std::collections::HashMap;


static mut GENERATORS: Option<HashMap<String, EntryGenerator>> = None;
static mut HIERARCHY: Option<Vec<(String, u64)>> = None;


pub fn set_generators(h: HashMap<String,EntryGenerator>) {
    unsafe {
        GENERATORS = Some(h);
    }
}

pub fn set_hierarchy(h: Vec<(String, u64)>) {
    unsafe {
        HIERARCHY = Some(h);
    }
}

pub fn get_generators() -> &'static HashMap<String, EntryGenerator> {
    unsafe {
        GENERATORS.as_ref().expect("GENERATORS must be set before calling get_generators")
    }
}

pub fn get_hierarchy() -> &'static Vec<(String, u64)> {
    unsafe {
        HIERARCHY.as_ref().expect("HIERARCHY must be set before calling get_hierarchy")
    }
}


pub async fn export_cmd(args: &CliArgs) -> anyhow::Result<()> {
    let ldif_file = match args.cmd {
        MainCommand::Export { ref file } => file.as_str(),
        _ => unreachable!()
    };
    let count = get_hierarchy().iter().map(|(_, c)| c).product::<u64>();

    let style = ProgressStyle::with_template("{wide_bar} [{pos}/{len}] ({percent}%) {msg} [{elapsed}/{eta}]").expect("valid style");
    let bar = ProgressBar::new(count);
    bar.set_style(style);

    // Create the export file and generate the entries
    let csv_sender = args.csv_sender().await?;
    let ldif_sender = crate::ldif::start_ldif_export_task(ldif_file).await?;
    let entry_receiver = crate::entries::entry_generator_task(args.base.clone(), get_generators(), get_hierarchy());

    let mut entry_stream = ReceiverStream::new(entry_receiver);
    while let Some(entry) = entry_stream.next().await {
        if let Some(ref sender) = csv_sender {
            sender.send(entry.clone()).expect("csv_task to be running");
        }

        ldif_sender.send(entry).expect("ldif_task to be running");

        bar.inc(1);
    }

    bar.finish();

    Ok(())
}

pub async fn insert_cmd(args: &CliArgs) -> anyhow::Result<()> {
    let count = get_hierarchy().iter().map(|(_, c)| c).product::<u64>();

    let Some(ldap_config) = LdapConfig::from_args(args) else { bail!("user, server, password required") };
    let pool = LdapPool::new(ldap_config).await?;

    let style = ProgressStyle::with_template("{wide_bar} [{pos}/{len}] ({percent}%) {msg} [{elapsed}/{eta}]").expect("Valid style");
    let bar = ProgressBar::new(count);
    bar.set_style(style);

    let csv_sender = args.csv_sender().await?;
    let entry_receiver = crate::entries::entry_generator_task(args.base.clone(), get_generators(), get_hierarchy());
    let (entry_sender, result_receiver) = crate::entries::insert_entries_task(pool);

    // handle the progress bar in its own task 
    let bar_task = tokio::spawn(async move {
        let mut result_stream = UnboundedReceiverStream::new(result_receiver);

        while let Some(res) = result_stream.next().await {
            bar.inc(1);

            if let Err(e) = res {
                bar.println(format!("Error: {e}"));
            }
        }
    });

    let mut entry_stream = ReceiverStream::new(entry_receiver);
    while let Some(entry) = entry_stream.next().await {
        if let Some(ref csv_sender) = csv_sender {
            csv_sender.send(entry.clone()).unwrap();
        }

        entry_sender.send(entry).await.unwrap();
    }
    

    Ok(bar_task.await?)
}
