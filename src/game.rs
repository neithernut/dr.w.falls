//! Game implementation

mod lobby;
mod waiting;
mod round;


/// Item type for game update channels
///
enum GameUpdate<U,T> {
    Update(U),
    PhaseEnd(PhaseEnd<T>),
}


/// Phase end messages
///
enum PhaseEnd<T> {
    Transition(T),
    EndOfGame,
}


/// Game phase indication
///
enum GamePhase {
    Lobby,
    Waiting,
    Round(usize),
    EndOfGame,
}


/// Player handle
///
#[derive(Default)]
struct PlayerHandle {
    data: std::sync::Arc<()>,
}

impl PlayerHandle {
    /// Generate a tag for this player handle
    ///
    fn tag(&self) -> PlayerTag {
        PlayerTag {data: std::sync::Arc::downgrade(&self.data)}
    }
}


/// Player tag
///
/// A value of this type allows identifying a player (via comparison)
///
#[derive(Clone)]
pub struct PlayerTag {
    data: std::sync::Weak<()>,
}

impl PlayerTag {
    pub fn is_alive(&self) -> bool {
        self.data.strong_count() > 0
    }
}

impl PartialEq<PlayerTag> for PlayerTag {
    fn eq(&self, other: &PlayerTag) -> bool {
        self.data.ptr_eq(&other.data)
    }
}

