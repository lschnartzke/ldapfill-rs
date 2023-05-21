//! Common types across the program

use std::collections::HashSet;

use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver, Sender, Receiver};

pub type LdapEntry = (String, Vec<(String, HashSet<String>)>);
pub type EntrySender = Sender<LdapEntry>;
pub type EntryReceiver = Receiver<LdapEntry>;
pub type LdifSender = UnboundedSender<LdapEntry>;
pub type LdifReceiver = UnboundedReceiver<LdapEntry>;
