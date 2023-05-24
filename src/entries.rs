use crate::progress;
use crate::types::{EntryReceiver, LdapEntry, EntrySender};
use crate::{LdapConfig, LdapPool, ProgressMessage, Receiver, ResultReceiver, Sender};

use crate::csv::CsvSender;
use crate::modifiers::{file_cache::FileCache, ModifierTree};
use ldap3::Ldap;
use tokio_stream::StreamExt;
use std::collections::{HashMap, HashSet, VecDeque};

use tokio::sync::{mpsc, oneshot};

/// The entry generator is used to generate entries of one specific object class.
#[derive(Debug, Clone)]
pub struct EntryGenerator {
    object_class: String,
    rdn_attribute: String,
    attributes: HashMap<String, ModifierTree>,
}

impl EntryGenerator {
    /// Creates a new `EntryGenerator` using the provided attributes.
    pub fn new(
        object_class: String,
        rdn_attribute: String,
        attributes: HashMap<String, ModifierTree>,
    ) -> Self {
        Self {
            object_class,
            rdn_attribute,
            attributes,
        }
    }

    pub fn object_class(&self) -> &str {
        self.object_class.as_str()
    }

    pub fn generate_entry(&self) -> (String, Vec<(String, HashSet<String>)>) {
        let mut entry = vec![(
            "objectclass".to_string(),
            HashSet::from([self.object_class.clone()]),
        )];
        let mut rdn: Option<String> = None;

        for (attribute, modifier) in self.attributes.iter() {
            let (key, value) = (attribute.as_str(), modifier.apply());
            if key == self.rdn_attribute {
                rdn = rdn.or_else(|| Some(value.clone()));
            }

            entry.push((key.to_owned(), HashSet::from([value])));
        }

        (format!("{}={}", self.rdn_attribute, rdn.unwrap()), entry)
    }

    pub async fn load_files(&self, cache: &mut FileCache) -> std::io::Result<()> {
        for tree in self.attributes.values() {
            tree.load_files_into_cache(cache).await?;
        }

        Ok(())
    }
}

/// Starts a new task that will generate entries as specified by the provided
/// `hierarchy` using `generators`. The entries are not validated. All generated
/// entries will be sent to the returned `EntryReceiver`.
pub fn entry_generator_task(
    base: String,
    generators: &'static HashMap<String, EntryGenerator>,
    hierarchy: &'static [(String, u64)],
) -> EntryReceiver {
    let (tx, rx) = mpsc::channel(500_000);

    tokio::spawn(async move {
        let mut dns = vec![base];
        for (object_class, count) in hierarchy.iter() {
            let generator = &generators[object_class];

            let mut new_dns = vec![];
            for dn in dns.iter() {
                let count = *count;
                for _ in 0..count {
                    let entry = generate_entry(dn, generator);
                    new_dns.push(entry.0.clone());

                    tx.send(entry).await.unwrap();
                }
            }
            dns.clear();
            dns.extend(new_dns);
        }
    });

    rx
}

fn generate_entry(base: &str, generator: &EntryGenerator) -> LdapEntry {
    let (rdn, entry) = generator.generate_entry();
    let dn = format!("{rdn},{base}");

    (dn, entry)
}

pub fn insert_entries_task(pool: LdapPool) -> (EntrySender, mpsc::UnboundedReceiver<Result<(), Box<dyn std::error::Error + Send>>>) {
    let (entry_tx, entry_rx) = mpsc::channel::<LdapEntry>(500_000);
    let (result_tx, result_rx) = mpsc::unbounded_channel::<Result<(), Box<dyn std::error::Error + Send>>>();

    // for now only use one connection, even if we have more. 
    let mut conn = pool.get_conn();

    tokio::spawn(async move {
        let (rx, tx) = (entry_rx, result_tx);
        
        let mut stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        while let Some((dn, attributes)) = stream.next().await {
            match conn.add(&dn, attributes).await {
                Ok(_) => tx.send(Ok(())).unwrap(),
                Err(e) => tx.send(Err(Box::new(e))).unwrap()
            }
        }
    });
    
    (entry_tx, result_rx)
}

// creates tasks to fill up the directory in parallel, task size is determined by available cpus
// and amount of entries to generate. Whatever is smaller determines the task size.
pub async fn fill_ldap(
    ldap_config: LdapConfig,
    generators: &'static HashMap<String, EntryGenerator>,
    hierarchy: &'static [(String, u64)],
    base: &'static str,
    csv_sender: Option<CsvSender>,
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
    let ptx = progress::start_progress_task(entry_count).await;

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

                    let mut message = ProgressMessage::Progress;
                    if let Err(e) = tx.send((dn.clone(), generator, sender)) {
                        //message = ProgressMessage::ProgressWithMessage(format!("Error when trying to send data to fill-task: {e}"));
                    }

                    drop(ptx.send(message));

                    if count == 0 {
                        break 'outer;
                    }
                }
            }

            for result in results.drain(..results.len()) {
                let res = result.await;
                match res {
                    Ok(Ok(res)) => new_dns.push(res),
                    Ok(Err(e)) => drop(ptx.send(ProgressMessage::Message(format!(
                        "Got result; it failed: {e}"
                    )))),
                    Err(e) => drop(ptx.send(ProgressMessage::Message(format!(
                        "Failed to receive result: {e}"
                    )))),
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
    csv_sender: Option<&CsvSender>,
) -> anyhow::Result<String> {
    let (rdn, entry) = generator.generate_entry();
    let dn = format!("{rdn},{base}");

    if let Err(e) = ldap.add(dn.as_str(), entry.clone()).await?.success() {
        debug!("entry {entry:#?} caused error during add: dn: {dn} error: {e:#?}");
        warn!("Error during entry insertion: {e}");
        bail!("{e}")
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
