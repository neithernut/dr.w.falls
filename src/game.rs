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

