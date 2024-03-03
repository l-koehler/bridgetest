// assorted stuff

extern crate alloc;

mod translator;
mod utils;
mod commands;
mod settings;
mod clientbound_translator;
mod serverbound_translator;
mod mt_definitions;

use minetest_protocol::MinetestServer;
use mt_definitions::Dimensions;

use alloc::vec::Vec;
use config::Config;
use std::path::PathBuf;
use std::path::Path;
use std::fs::File;
use std::io::Write;
use dirs;

#[tokio::main]
async fn main() {
    let settings: Config = load_config();
    mt_definitions::validate_texture_pack(&settings).await;
    start_client_handler(settings).await;
}

pub struct MTServerState {
    // things the server should keep track of
    players: Vec<String>, // names of all players
    mt_clientside_pos: (f32, f32, f32), // used to tolerate slight position differences, resulting in far smoother movement
    mt_last_known_health: u16, // used to determine if a HP change should trigger a damage effect flash
    respawn_pos: (f32, f32, f32),
    current_dimension: Dimensions
}

async fn start_client_handler(settings: Config) {
    // Create/Host a Minetest Server
    let mt_server_addr: String = settings.get_string("mt_server_addr").expect("Failed to read config!");
    utils::logger(&format!("[Minetest] Creating Server ({})...", mt_server_addr), 1);
    let mut mt_server = MinetestServer::new(mt_server_addr.parse().unwrap());
    // Define a server state with stuff to keep track of
    // Sane defaults aren't possible, all this will be overwritten before getting read anyways
    let mt_server_state = MTServerState {
        players: Vec::new(),
        mt_clientside_pos: (0.0, 0.0, 0.0),
        mt_last_known_health: 0,
        respawn_pos: (0.0, 0.0, 0.0),
        current_dimension: Dimensions::Overworld
    };

    // Wait for a client to join
    tokio::select! {
        conn = mt_server.accept() => {
            utils::logger(&format!("[Minetest] Client connected from {:?}", conn.remote_addr()), 1);
            translator::client_handler(mt_server, conn, mt_server_state, settings).await;
            // The infinite loop of the client handler has returned
            // presumably due to a disconnect
        }
    }
}

fn load_config() -> Config {
    let config_path: PathBuf = dirs::config_dir().unwrap();
    let config_file_path: PathBuf = config_path.join("bridgetest.toml");
    if !Path::new(config_file_path.as_path()).exists() {
        // Create config and set defaults
        let mut data_file = File::create(config_file_path.as_path()).expect("Creating config file failed!");
        data_file.write(settings::CONF_FALLBACK.as_bytes()).expect("Writing defaults to config file failed!");
    }
    let builder = Config::builder()
        .set_default("texture_pack_path", "").unwrap()
        .add_source(config::File::new(config_file_path.to_str().expect("The config file path must be UTF-8! IDK what you did to your system that it is not."),
                                      config::FileFormat::Toml));
    builder.build().expect("Failed to create config!")
}
