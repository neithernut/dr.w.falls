//! Implementation of the lobby phase

use std::net::SocketAddr;
use std::sync::Arc;


/// Connection token
///
#[derive(Clone, Debug)]
struct ConnectionToken {
    data: Arc<SocketAddr>,
}

impl From<SocketAddr> for ConnectionToken {
    fn from(addr: SocketAddr) -> Self {
        Self {data: Arc::new(addr)}
    }
}

impl Eq for ConnectionToken {}

impl PartialEq for ConnectionToken {
    fn eq(&self, other: &ConnectionToken) -> bool {
        Arc::ptr_eq(&self.data, &other.data)
    }
}

impl std::hash::Hash for ConnectionToken {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.data).hash(state)
    }
}

impl AsRef<SocketAddr> for ConnectionToken {
    fn as_ref(&self) -> &SocketAddr {
        &self.data
    }
}

