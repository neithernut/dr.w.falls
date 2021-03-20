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
    score: u32,
}

impl Player {
    /// Create a new player
    ///
    pub fn new(name: String, tag: game::PlayerTag) -> Self {
        Self {name, tag, score: 0}
    }
}

