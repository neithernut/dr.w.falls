//! Player data and management

use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock};

use tokio::task::JoinHandle;
use tokio::sync::mpsc;


#[cfg(test)]
mod tests;


/// Convenience type for player roster
///
pub type Roster = Vec<Tag>;


/// A player handle
///
/// Instances of this are meant to be held by a player, or rather a task
/// associated to the player exclusively. It allows creating tags for player
/// identification and notifications via a provided channel when dropped.
///
#[derive(Debug)]
pub struct Handle {
    data: Arc<Data>,
    notifier: mpsc::UnboundedSender<Tag>
}

impl Handle {
    /// Create a new player handle
    ///
    /// When dropped, the handle will send its tag via the `notifier` channel.
    ///
    pub fn new(data: Arc<Data>, notifier: mpsc::UnboundedSender<Tag>) -> Self {
        Self {data, notifier}
    }

    /// Create a tag for this player
    ///
    pub fn tag(&self) -> Tag {
        Tag {data: self.data.clone()}
    }
}

impl PartialEq<Tag> for Handle {
    fn eq(&self, other: &Tag) -> bool {
        Arc::ptr_eq(&self.data, &other.data)
    }
}

impl AsRef<Data> for Handle {
    fn as_ref(&self) -> &Data {
        self.data.as_ref()
    }
}

impl Deref for Handle {
    type Target = Data;

    fn deref(&self) -> &Self::Target {
        self.data.deref()
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        use crate::error::{DebugErr, TryExt};

        self.conn_state
            .write()
            .map_err(|e| DebugErr::new("Could not acquire connection state lock", e))
            .or_warn(format!("Could not clear connection state for player: {}", self.name()))
            .and_then(|mut s| s.take())
            .or_err(format!("Player already disconnected: {}", self.name()));

        self.notifier.send(self.tag()).or_warn("Could not send disconnection notification");
    }
}


/// Tag identifying a specific player
///
/// In addition to identification, a tag also allows accessing the player data.
///
#[derive(Clone, Debug)]
pub struct Tag {
    data: Arc<Data>,
}

impl Eq for Tag {}

impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        self.eq(&other.data)
    }
}

impl PartialEq<Arc<Data>> for Tag {
    fn eq(&self, other: &Arc<Data>) -> bool {
        Arc::ptr_eq(&self.data, other)
    }
}

impl std::hash::Hash for Tag {
    fn hash<H>(&self, state: &mut H)
        where H: std::hash::Hasher
    {
        Arc::as_ptr(&self.data).hash(state)
    }
}

impl AsRef<Data> for Tag {
    fn as_ref(&self) -> &Data {
        self.data.as_ref()
    }
}

impl Deref for Tag {
    type Target = Data;

    fn deref(&self) -> &Self::Target {
        self.data.deref()
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for Tag {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        use quickcheck::Arbitrary;

        Tag {data: Arc::new(Data {
            name: tests::Name::arbitrary(g).into(),
            addr: Arbitrary::arbitrary(g),
            score: u32::arbitrary(g).into(),
            conn_state: None.into(),
        })}
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        let res = (tests::Name(self.name.clone()), self.addr, self.score.load(Ordering::Relaxed))
            .shrink()
            .map(|(n, addr, s)| Tag {
                data: Arc::new(Data {name: n.into(), addr, score: s.into(), conn_state: None.into()}),
            });
        Box::new(res)
    }
}


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


/// Maximum allowed length for a player name
///
pub const MAX_PLAYER_NAME_LEN: usize = 16;

