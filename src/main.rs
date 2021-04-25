mod console;
mod display;
mod game;
mod gameplay;
mod util;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}


/// Roster of players
///
type Roster = Vec<Player>;


/// Representation of a single player within the roster
///
struct Player {
    name: String,
    tag: game::PlayerTag,
    peer: std::net::SocketAddr,
    task: game::ConnTaskHandle,
    score: u32,
}

impl Player {
    /// Create a new player
    ///
    pub fn new(
        name: String,
        tag: game::PlayerTag,
        peer: std::net::SocketAddr,
        task: game::ConnTaskHandle,
    ) -> Self {
        Self {name, tag, peer, task, score: 0}
    }
}

