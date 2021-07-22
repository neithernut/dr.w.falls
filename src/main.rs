//! Dr. W. Falls

use std::time::Duration;

use tokio::{net, sync::watch};

#[macro_use]
extern crate clap;

#[macro_use]
extern crate quickcheck_macros;


mod console;
mod display;
mod error;
mod field;
mod game;
mod player;
mod util;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = clap_app!(dr_w_falls =>
        (@arg listen: -l --listen +takes_value "Address to listen on")
        (@arg port: -p --port +takes_value "Port to listen on")
        (@arg maxp: --max-players +takes_value "Maximum number of players allowed")
        (@arg virs: --virs +takes_value "number of viruses placed on the field at the beginning of a round")
        (@arg tick: --tick +takes_value "duration of a tick (the time a capsule moved down one tile) im ms")
        (@arg console: --gm-sock +takes_value "serve a GM console on a UNIX domain socket at this path")
    ).get_matches();


    // Collect settings
    let addr = matches
        .value_of("listen")
        .map(str::parse)
        .transpose()
        .map_err(|e| error::WrappedErr::new("Expected address to listen on", e))?
        .unwrap_or(std::net::Ipv4Addr::UNSPECIFIED.into());
    let port = matches
        .value_of("port")
        .map(str::parse)
        .transpose()
        .map_err(|e| error::WrappedErr::new("Expected address to listen on", e))?
        .unwrap_or(2020);
    let addr = std::net::SocketAddr::new(addr, port);

    let settings = console::Settings {
        accept_players: true,
        max_players: matches
            .value_of("maxp")
            .map(str::parse)
            .transpose()
            .map_err(|e| error::WrappedErr::new("Expected maximum number of players", e))?
            .unwrap_or(u8::MAX),
        virus_count: matches
            .value_of("virs")
            .map(str::parse)
            .transpose()
            .map_err(|e| error::WrappedErr::new("Expected number of viruses", e))?
            .unwrap_or(10),
        tick_duration: Duration::from_millis(matches
            .value_of("virs")
            .map(str::parse)
            .transpose()
            .map_err(|e| error::WrappedErr::new("Expected tick duration in number of ms", e))?
            .unwrap_or(200)),
    };

    let gm_sock_path = matches.value_of_os("console");


    // Setup
    let (control_sender, control_receiver) = watch::channel(settings.as_lobby_control());
    let (phase_sender, phase) = watch::channel(game::GamePhase::<rand_pcg::Pcg64Mcg>::default());
    let roster = Default::default();

    log::info!("Listening for players on {}", addr);
    let player_sock = net::TcpListener::bind(addr)
        .await
        .map_err(|e| error::WrappedErr::new("Could not listen for players", e))?;
    let gm_sock = gm_sock_path
        .map(net::UnixListener::bind)
        .transpose()
        .map_err(|e| error::WrappedErr::new("Could not open GM socket", e))?;


    // Run
    log::info!("Finished setup {}", addr);
    let gm = console::game_master(control_sender, settings, phase.clone(), Clone::clone(&roster), gm_sock);
    let game = game::run(player_sock, control_receiver, roster, phase_sender, phase);
    let sigint = tokio::signal::ctrl_c();
    tokio::select!{
        r = gm => r.map_err(Into::into),
        r = game => r.map_err(Into::into),
        r = sigint => r.map_err(Into::into),
    }
}

