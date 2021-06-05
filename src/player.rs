//! Player data and management

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::RwLock;

use tokio::task::JoinHandle;


/// Data associated to a player
///
#[derive(Debug)]
pub struct Data {
    name: String,
    addr: SocketAddr,
    score: AtomicU32,
    conn_state: RwLock<Option<ConnTaskHandle>>,
}

impl Data {
    /// Create a new player data object
    ///
    pub fn new(name: String, addr: SocketAddr, handle: ConnTaskHandle) -> Self {
        Self {name, addr, score: 0.into(), conn_state: Some(handle).into()}
    }

    /// Retrieve the player's name
    ///
    pub fn name(&self) -> &str {
        &self.name.as_ref()
    }

    /// Retrieve the address of the peer
    ///
    pub fn addr(&self) -> &SocketAddr {
        &self.addr
    }

    /// Retrieve the current total score
    ///
    pub fn score(&self) -> u32 {
        self.score.load(Ordering::Relaxed)
    }

    /// Add a given value to the player's total score
    ///
    pub fn add_score(&self, value: u32) -> u32 {
        self.score.fetch_add(value, Ordering::Release)
    }

    /// Check whether the player is still connected
    ///
    pub fn is_connected(&self) -> bool {
        // As the only code which could end up poisoning the lock are related to
        // a player's disconnect, anyway, we can just assume that they are
        // disconnected.
        self.conn_state.read().map(|state| state.is_some()).unwrap_or(false)
    }

    /// Kick the player
    ///
    /// This function kicks the player by aborting the associated connection
    /// task. The associated conneciton task handle will be returned if the
    /// player was not already disconnected.
    ///
    pub fn kick(&self) -> Option<ConnTaskHandle> {
        self.conn_state.write().ok().and_then(|mut s| s.take()).map(|h| { h.abort(); h})
    }
}


/// Task handle of connection tasks
///
pub type ConnTaskHandle = JoinHandle<()>;

