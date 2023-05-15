//! Provides functions that allow to export generated entries as a csv file.
//!
//! Entries written to a csv file whose name is equivalent to the object class.
//! Users can specify to which directory csv files will be written.
//!
//! When an object class is first written, the csv writer will iterate through all fields,
//! write a header and retain the order of the fields for subsequent writes.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use tokio::fs as tfs;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task;
use tokio::time::Instant;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt;

use crate::types::LdapEntry;

pub type CsvSender = UnboundedSender<LdapEntry>;
pub type CsvReceiver = UnboundedReceiver<LdapEntry>;
pub type Writer = csv::Writer<std::fs::File>;

/// Starts the csv export task. This function checks if the `target_dir` exists and tries to
/// create it if it doesen't. It starts the export task on a background task and returns the sender
/// handle that allows sending ldap entries to serialize to the task. When the last sender has been
/// dropped, the task will stop.
pub async fn start_csv_task<P: AsRef<Path>>(target_dir: P) -> anyhow::Result<CsvSender> {
    let (sender, receiver) = mpsc::unbounded_channel();
    let path = target_dir.as_ref().to_path_buf();

    // create directory if it does not exist
    if !path.exists() {
        tfs::create_dir_all(path.as_path()).await?;
    }

    tokio::spawn(async move { csv_exporter(path, receiver).await });

    Ok(sender)
}

async fn csv_exporter(export_path: PathBuf, receiver: CsvReceiver) {
    match csv_exporter_inner(export_path, receiver).await {
        Ok(_) => (),
        Err(e) => error!("Failed to export csv: {e}"),
    }
}

async fn csv_exporter_inner(export_path: PathBuf, receiver: CsvReceiver) -> anyhow::Result<()> {
    let mut stream = UnboundedReceiverStream::new(receiver);
    // the list of writers, with the associated object class
    let mut writers: Vec<(String, Writer)> = Vec::new();
    // keep track of classes and in which order to serialize them
    let mut object_classes: HashMap<String, Vec<String>> = HashMap::new();
    let mut last_flush = Instant::now();

    while let Some((object_class, attributes)) = stream.next().await {
        if !object_classes.contains_key(object_class.as_str()) {
            handle_new_object_class(&mut object_classes, object_class.as_str(), &attributes);
        }

        // get the writer
        let writer = if let Some(w) = writers
            .iter_mut()
            .find(|(c, _)| *c == object_class)
            .map(|(_, w)| w)
        {
            w
        } else {
            let file = format!("{object_class}.csv");
            match task::block_in_place(|| open_new_writer(export_path.join(file))) {
                Ok(mut w) => {
                    let order = &object_classes[&object_class];

                    if let Err(e) = w.write_record(order) {
                        warn!("Failed to write header row for {object_class}: {e}");
                    }

                    writers.push((object_class.clone(), w));
                    writers
                        .iter_mut()
                        .find(|(c, _)| *c == object_class)
                        .map(|(_, w)| w)
                        .unwrap()
                }
                Err(_) => {
                    // ignore the error and try again next time we encounter this class
                    continue;
                }
            }
        };

        let order = &object_classes[&object_class];
        let mut record: Vec<&str> = Vec::with_capacity(order.len());

        // put reference in the correct `order` for the csv file
        // we do this by iterating through the attributes of the determined order,
        // look for the current attribute in the list of attributes of the received entry
        // and pushing the value into the record Vec.
        for attribute in order.iter() {
            // O(n), I guess
            // There must always be at least one entry
            let value = attributes
                .iter()
                .find(|(k, _)| *k == *attribute)
                .map(|(_, v)| v.iter().next().unwrap())
                .unwrap();
            record.push(value);
        }

        if let Err(e) = task::block_in_place(|| writer.write_record(record)) {
            warn!("Failed to write csv record: {e}");
        }

        // flush every 5 seconds to minimize data loss if we only write few objects during a long
        // operation (e.g. generating 200k entries, only 20 of which are at the top level and
        // don't trigger a flush by the Writer on their own.)
        let now = Instant::now();
        if (now - last_flush).as_secs() >= 5 {
            task::block_in_place(|| {
                writers.iter_mut().for_each(|(s, w)| {
                    if let Err(e) = w.flush() {
                        warn!("Failed to flush {s} writer: {e}");
                    }
                })
            });

            last_flush = now;
        }
    }

    Ok(())
}

fn handle_new_object_class(
    class_map: &mut HashMap<String, Vec<String>>,
    new_class: &str,
    new_attributes: &[(String, HashSet<String>)],
) {
    let order: Vec<String> = new_attributes
        .iter()
        .map(|(s, _)| s)
        .map(String::to_owned)
        .collect();
    class_map.insert(new_class.to_owned(), order);
}

fn open_new_writer(file: PathBuf) -> anyhow::Result<Writer> {
    let file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(file)?;

    let writer = csv::WriterBuilder::new().from_writer(file);

    Ok(writer)
}
