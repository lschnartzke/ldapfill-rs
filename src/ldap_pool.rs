//! A simple connection pool for ldap connections.
use ldap3::{Ldap, LdapConnAsync, LdapError};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::config::LdapConfig;

#[derive(Debug)]
pub struct LdapPool {
    conns: Vec<Ldap>,
    index: AtomicUsize,
}

impl LdapPool {
    /// Creates a new pool with `count` connections using `settings`. Each connection
    /// will be driven immediately and a bind operations with the provided credentials
    /// is performed. If any of the binds or connects fails, the function returns an
    /// error.
    ///
    /// # Panics
    /// Panics if `count` cannot be allocated by `Vec`.
    pub async fn new(settings: LdapConfig) -> Result<Self, LdapError> {
        let mut conns = Vec::with_capacity(settings.connections());
        let index = AtomicUsize::new(0);

        for _ in 0..settings.connections() {
            let (conn, mut ldap) = LdapConnAsync::new(settings.server()).await?;
            ldap3::drive!(conn);

            ldap.simple_bind(settings.user(), settings.password())
                .await?;

            conns.push(ldap);
        }

        Ok(LdapPool { conns, index })
    }

    /// Rotates the internal queue and returns a cloned reference to one of the
    /// available connections. The connections are shared using round-robin.
    pub fn get_conn(&self) -> Ldap {
        let index = self.index.fetch_add(1, Ordering::SeqCst) % self.conns.len();
        self.conns[index].clone()
    }
}
