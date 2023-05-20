//! Common types across the program

use std::collections::HashSet;

use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver};

pub type LdapEntry = (String, Vec<(String, HashSet<String>)>);
pub type EntrySender = UnboundedSender<LdapEntry>;
pub type EntryReceiver = UnboundedReceiver<LdapEntry>;
pub type LdifSender = UnboundedSender<LdapEntry>;
pub type LdifReceiver = UnboundedReceiver<LdapEntry>;
