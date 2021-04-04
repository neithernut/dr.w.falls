//! Implementation of the lobby phase

use std::sync::Arc;

use tokio::sync::oneshot;

use crate::display;


/// Local type for game updates
///
type GameUpdate<S, R> = super::GameUpdate<Arc<Vec<ScoreBoardEntry>>, (S, R)>;


/// Local type for phase end
///
type PhaseEnd<S, R> = super::PhaseEnd<(S, R)>;


/// Registration request
///
struct Registration {
    name: String,
    token: ConnectionToken,
    response: oneshot::Sender<RegistrationReply>
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
enum RegistrationReply {
    Accepted(super::PlayerHandle),
    Denied(DenialReason)
}

impl From<DenialReason> for RegistrationReply {
    fn from(reason: DenialReason) -> Self {
        Self::Denied(reason)
    }
}


/// Reason for denial of a registration
///
enum DenialReason {
    AcceptanceClosed,
    MaxPlayers,
    NameTaken,
    RosterAccess,
}


/// Connection token
///
#[derive(Clone)]
struct ConnectionToken {
    data: Arc<()>,
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


/// Score board entry for the lobby phase
///
#[derive(PartialEq)]
struct ScoreBoardEntry {
    name: String,
    tag: super::PlayerTag,
}

impl display::ScoreBoardEntry for ScoreBoardEntry {
    type Tag = super::PlayerTag;

    type Extra = &'static str;

    fn name(&self) -> &str {
        self.name.as_ref()
    }

    fn tag(&self) -> Self::Tag {
        self.tag.clone()
    }

    fn score(&self) -> u32 {
        0
    }

    fn extra(&self) -> Self::Extra {
        ""
    }

    fn active(&self) -> bool {
        self.tag.is_alive()
    }
}


/// Maximum allowed length for a name
///
const NAME_MAX_LEN: u16 = 16;

