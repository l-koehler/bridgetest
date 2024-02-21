// assorted stuff

extern crate alloc;

mod translator;
mod utils;
mod commands;
mod settings;
mod clientbound_translator;
mod mt_definitions;

use minetest_protocol::MinetestServer;

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
    println!("{}", settings.cache);
    //start_client_handler(settings).await;
}

pub struct MTServerState {
    players: Vec<String>,
    // add other stuff i need to keep track of
}

async fn start_client_handler(settings: Config) {
    // Create/Host a Minetest Server
    utils::logger(&format!("[Minetest] Creating Server ({})...", settings::MT_SERVER_ADDR), 1);
    let mut mt_server = MinetestServer::new(settings::MT_SERVER_ADDR.parse().unwrap());
    // Define a server state with stuff to keep track of
    let mt_server_state = MTServerState {
        players: Vec::new(),
    };
    // Read the ProxyConfig

    // Wait for a client to join
    tokio::select! {
        conn = mt_server.accept() => {
            utils::logger(&format!("[Minetest] Client connected from {:?}", conn.remote_addr()), 1);
            translator::client_handler(mt_server, conn, mt_server_state).await;
        }
    }
}

fn load_config() -> Config {
    let config_path: PathBuf = dirs::config_dir().unwrap();
    let config_file_path: PathBuf = config_path.join("bridgetest.toml");
    if !Path::new(config_file_path.as_path()).exists() {
        // Create config and set defaults
        let mut data_file = File::create(config_file_path.as_path()).expect("creation failed");
        data_file.write(settings::CONF_FALLBACK.as_bytes()).expect("write failed");
    }
    let builder = Config::builder()
        .set_default("texture_pack_path", "").unwrap()
        .add_source(config::File::new(config_file_path.to_str().expect("The config file path must be UTF-8!"),
                                      config::FileFormat::Toml));
    builder.build().expect("Failed to create config!")
}
