//! Implementation of the lobby phase

use std::sync::Arc;

use tokio::sync::oneshot;

use crate::display;


/// Registration request
///
struct Registration {
    name: String,
    token: ConnectionToken,
    response: oneshot::Sender<RegistrationReply>
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

