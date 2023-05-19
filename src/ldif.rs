/// Exports generated entries as LDIF file instead of directly adding them to the server.
/// This increases reusability at the cost of possibly generating invalid entries as there 
/// is no syntax validation according to the ldif specification. (And I don't have time to 
/// read all that and test it in less than 5 weeks)

use std::path::{Path, PathBuf};
use std::io;

use tokio::fs as tfs;
use tokio::io as tio;
use tokio::sync::mpsc::unbounded_channel;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::UnboundedReceiverStream;


use crate::types::{LdifSender, LdifReceiver, LdapEntry};

pub async fn start_ldif_export_task<P: AsRef<Path>>(export_file: P) -> anyhow::Result<LdifSender> {
    let (tx, rx) = unbounded_channel();

    let file = tfs::File::open(export_file).await?;
    let writer = tio::BufWriter::new(file);

    tokio::spawn(async move { ldif_exporter(rx, writer).await });

    Ok(tx)
}

async fn ldif_exporter<O: tio::AsyncWriteExt + Unpin>(rx: LdifReceiver, mut writer: O) {
    let mut stream = UnboundedReceiverStream::new(rx);
    while let Some(entry) = stream.next().await {
        let entry_string = build_entry_string(entry);

        if let Err(e) = writer.write(entry_string.as_bytes()).await {
            debug!("LDIF write error: {e:#?}");
            warn!("Failed to write entry to file: {e}");
        }

    }
}

fn build_entry_string(entry: LdapEntry) -> String {
    let (dn, attributes) = entry;
    //              prefix                                                                      ": \n"            empty line      
    let capacity = "dn: \n".len() + dn.len() + attributes.iter().map(|(k, v)| k.len() + v.len() + 3).sum::<usize>() + 2;
    let mut entry_string = String::with_capacity(capacity);
    // build the entry String
    entry_string.push_str("dn: ");
    entry_string.push_str(dn.as_str());
    entry_string.push('\n');

    for (key, value) in attributes.iter() {
        entry_string.push_str(key.as_str());
        entry_string.push_str(": ");
        // theres always exactly one value for each generated value
        entry_string.push_str(value.iter().next().unwrap());
        entry_string.push('\n');
    }
    entry_string.push('\n');

    entry_string
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use super::*;

    #[tokio::test]
    async fn test_ldif() {
        let entry = (
            "uid=test.user,ou=users,dc=example,dc=org".to_string(),
            vec![
            ("objectClass".to_string(), HashSet::from(["inetOrgPerson".to_string()])),
            ("uid".to_string(), HashSet::from(["test.user".to_string()])),
            ("sn".to_string(), HashSet::from(["user".to_string()]))
            ]
        );

        let entry_string = build_entry_string(entry);

        assert_eq!(entry_string.as_str(), "dn: uid=test.user,ou=users,dc=example,dc=org\nobjectClass: inetOrgPerson\nuid: test.user\nsn: user\n\n");

            
    }
}
