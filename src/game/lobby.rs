//! Implementation of the lobby phase

use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::oneshot;

use crate::player;


/// Registration request
///
#[derive(Debug)]
struct Registration {
    name: String,
    token: ConnectionToken,
    response: oneshot::Sender<RegistrationReply>,
}

impl Registration {
    /// Create a new Registration
    ///
    pub fn new(
        name: String,
        token: ConnectionToken,
        response: oneshot::Sender<RegistrationReply>
    ) -> Self {
        Self {name, token, response}
    }
}


/// Reply to a registration request
///
#[derive(Debug)]
enum RegistrationReply {
    Accepted(player::Handle),
    Denied(DenialReason),
}

impl From<player::Handle> for RegistrationReply {
    fn from(handle: player::Handle) -> Self {
        Self::Accepted(handle)
    }
}

impl From<DenialReason> for RegistrationReply {
    fn from(reason: DenialReason) -> Self {
        Self::Denied(reason)
    }
}


/// Reason for denial of a registration
///
#[derive(Copy, Clone, Debug)]
enum DenialReason {
    AcceptanceClosed,
    MaxPlayers,
    NameTaken,
    TemporaryFailure,
    PermanentFailure,
}

impl fmt::Display for DenialReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AcceptanceClosed  => write!(f, "Registration is closed"),
            Self::MaxPlayers        => write!(f, "Max number of players reached"),
            Self::NameTaken         => write!(f, "Name is already taken"),
            Self::TemporaryFailure  => write!(f, "Temporary registration failure"),
            Self::PermanentFailure  => write!(f, "Permanent registration failure"),
        }
    }
}


/// Connection token
///
#[derive(Clone, Debug)]
pub struct ConnectionToken {
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


/// Maximum allowed length for a name
///
const NAME_MAX_LEN: u16 = 16;

