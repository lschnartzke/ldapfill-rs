//! Common types across the program

use std::collections::HashSet;

pub type LdapEntry = (String, Vec<(String, HashSet<String>)>);
