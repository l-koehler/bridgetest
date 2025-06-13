#![feature(variant_count)]
#![feature(slice_as_array)]
#![feature(slice_pattern)]
#![feature(string_remove_matches)]
// fuck this warning.
// sure the language doesn't need the parens, but this isn't codegolf. i need legible code
#![allow(unused_parens)]

mod clientbound_translator;
mod commands;
mod mt_definitions;
mod on_tick;
mod serverbound_translator;
mod settings;
mod textures;
mod translator;
mod utils;

use azalea::container::ContainerHandle;
use azalea::world::MinecraftEntityId;
use azalea_client::inventory;
use log::*;
use luanti_protocol::LuantiServer;
use luanti_protocol::types::NodeBox;
use mt_definitions::{Dimensions, EntityMetadata};
use textures::{BlockMapping, LuantiTexture};

use bimap::BiMap;
use clap::Arg;
use config::Config;
use dirs;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[tokio::main]
async fn main() {
    env_logger::init();
    let settings: Config = load_config();
    textures::fetch_models(&settings).await;
    start_client_handler(settings).await;
}

#[derive(Clone)]
pub struct MTServerState {
    // things the server should keep track of
    // mostly used to prevent sending useless/redundant packets
    // and for everything else
    // qwq this thing sucks...
    players: Vec<String>,               // names of all players
    this_player: (String, String), // the proxied player (0: clientside name, 1: name passed to the mc server)
    mt_clientside_pos: (f32, f32, f32), // used to tolerate slight position differences, resulting in far smoother movement
    client_rotation: (f32, f32),        // yaw/pitch
    mt_clientside_player_inv: inventory::Player,
    mt_last_known_health: u16, // used to determine if a HP change should trigger a damage effect flash
    mc_last_air_supply: u32,   // used to determine if the air supply bar should change
    respawn_pos: (f32, f32, f32),
    current_dimension: Dimensions,
    is_sneaking: bool,
    mt_current_speed: f32,
    has_moved_since_sync: bool,
    keys_pressed: u32,
    // 32 bit server-side ID <-> 16 bit client-side ID
    entity_id_map: BiMap<MinecraftEntityId, u16>,
    // allocatable (free) ID ranges on the client
    // adjacent free ranges are joined on entity removal, range is inclusive on both sides
    // adding a entity will pick the lowest ID of the smallest range to prevent fragmentation
    // starts with 0 non-allocatable because the player doesn't properly get a server-side ID
    c_alloc_id_ranges: Vec<(u16, u16)>,
    // position/velocity in ECS-format in case a entity scheduled for update causes a ECS miss
    // mapped by the server-side ID
    // also EntityKind for some other stuff
    entity_meta_map: HashMap<MinecraftEntityId, EntityMetadata>,
    // entities that will be updated in the next tick
    // used to prevent flooding the client with thousands of packets
    // side effect: we only iterate the ECS once
    entities_update_scheduled: Vec<MinecraftEntityId>,
    // never read, only used to not drop the handle
    inventory_handle: Option<Arc<Mutex<ContainerHandle>>>,
    container_id: Option<i32>,
    // used to not attack on every left click, only on ones that aren't breaking blocks
    next_click_no_attack: bool,
    // used to only attack on the rising edge, not constantly
    previous_dig_held: bool,
    // maps "minecraft:item"
    item_texture_map: HashMap<String, LuantiTexture>,
    // maps "minecraft:block"
    block_texture_map: HashMap<String, BlockMapping>,
    // maps NB_abc123
    nodebox_lookup: HashMap<String, NodeBox>,
    subtitles: Vec<(String, Instant)>,
    prev_subtitle_string: String,
}

async fn start_client_handler(settings: Config) {
    // Create/Host a Minetest Server
    let mt_addr: IpAddr = match settings.get("net.local_only").unwrap() {
        true => Ipv4Addr::new(127, 0, 0, 1),
        false => Ipv4Addr::new(0, 0, 0, 0),
    }
    .into();
    let mt_port = settings.get_int("net.luanti_port").unwrap() as u16;
    info!("Creating Luanti server ({}:{})...", mt_addr, mt_port);
    let mut mt_server = LuantiServer::new(SocketAddr::new(mt_addr, mt_port));
    // Define a server state with stuff to keep track of
    // Sane defaults aren't possible, all this will be overwritten before getting read anyways
    let mt_server_state = MTServerState {
        players: Vec::new(),
        this_player: (String::from(""), String::from("")),
        mt_clientside_pos: (0.0, 0.0, 0.0),
        client_rotation: (0.0, 0.0),
        mt_clientside_player_inv: inventory::Player {
            craft_result: inventory::ItemStack::default(),
            craft: inventory::SlotList::default(),
            armor: inventory::SlotList::default(),
            inventory: inventory::SlotList::default(),
            offhand: inventory::ItemStack::default(),
        },
        mt_last_known_health: 0,
        mc_last_air_supply: 0,
        respawn_pos: (0.0, 0.0, 0.0),
        current_dimension: Dimensions::Overworld,
        is_sneaking: false,
        mt_current_speed: 4.317,
        has_moved_since_sync: false,
        keys_pressed: 0,
        entity_id_map: BiMap::new(),
        c_alloc_id_ranges: vec![(2, u16::MAX)], // 0 reserved for player, 1 causes issues
        entity_meta_map: HashMap::new(),
        entities_update_scheduled: Vec::new(),
        inventory_handle: None,
        container_id: None,
        next_click_no_attack: false,
        previous_dig_held: false,
        item_texture_map: HashMap::new(),
        block_texture_map: HashMap::new(),
        nodebox_lookup: HashMap::new(),
        subtitles: Vec::new(),
        prev_subtitle_string: String::from(""),
    };

    // Wait for a client to join
    // also print some relevant info before that
    println!(
        "Luanti server listening ({}:{}), will proxy to Minecraft ({})",
        mt_addr,
        mt_port,
        settings.get_string("net.mc_server").unwrap()
    );
    println!("Waiting for client to connect...");
    tokio::select! {
        conn = mt_server.accept() => {
            println!("Client connected ({})", conn.remote_addr());
            translator::client_handler(mt_server, conn, mt_server_state, settings).await;
            // The infinite loop of the client handler has returned, presumably due to a disconnect.
            // exit after this.
            debug!("Client Handler returned, exiting.");
        }
    }
}

fn load_config() -> Config {
    let command_line_matches = clap::Command::new("bridgetest")
        .version("0.1.0")
        .about("Proxy between a Luanti client and a Minecraft 1.21.5 server")
        .arg(
            Arg::new("server")
                .short('s')
                .long("server")
                .value_name("SERVER IP:PORT")
                .help("IP address and port (as IP:port) of the minecraft server")
                .value_hint(clap::ValueHint::Other),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("LUANTI PORT")
                .help("Port to listen on for a luanti client")
                .value_parser(clap::value_parser!(i64))
                .value_hint(clap::ValueHint::Other),
        )
        .arg(
            Arg::new("local")
                .long("local-only")
                .help("Make the proxy accessible only over the local loopback address")
                .value_name("LOCAL-ONLY")
                .value_parser(clap::builder::BoolishValueParser::new())
                .value_hint(clap::ValueHint::Other),
        )
        .arg(
            Arg::new("account")
                .long("account")
                .value_name("E-MAIL")
                .help("If set, use that microsoft account. Overrides address set in config file.")
                .value_hint(clap::ValueHint::EmailAddress),
        )
        .get_matches();
    let config_path: PathBuf = dirs::config_dir().unwrap();
    let config_file_path: PathBuf = config_path.join("bridgetest.toml");
    info!("Using config file at {:?}", config_file_path);
    if !Path::new(config_file_path.as_path()).exists() {
        // Create config and set defaults
        warn!(
            "Config file not found, writing defaults ({:?})",
            config_file_path
        );
        let mut data_file =
            File::create(config_file_path.as_path()).expect("Creating config file failed!");
        data_file
            .write_all(settings::CONF_FALLBACK.as_bytes())
            .expect("Writing defaults to config file failed!");
    }
    let mut builder = Config::builder().add_source(config::File::new(
        config_file_path.to_str().unwrap(),
        config::FileFormat::Toml,
    ));
    if let Some(server) = command_line_matches.get_one::<String>("server") {
        if SocketAddr::from_str(&server).is_err() {
            println!("Invalid server address! (must be IP:port like 127.0.0.1:25565)");
            std::process::exit(1);
        }
        builder = builder
            .set_override("net.mc_server", server.clone())
            .unwrap()
    }
    // use i64 here for compat with the toml file
    if let Some(port) = command_line_matches.get_one::<i64>("port") {
        if *port > u16::MAX.into() || *port < 0 {
            println!("Invalid port number! (must be between 0 and {})", u16::MAX);
            std::process::exit(1);
        }
        builder = builder.set_override("net.luanti_port", *port).unwrap()
    }
    if let Some(local) = command_line_matches.get_one::<bool>("local") {
        builder = builder.set_override("net.local_only", *local).unwrap()
    }
    if let Some(email) = command_line_matches.get_one::<String>("account") {
        builder = builder.set_override("auth.online_mode", true).unwrap();
        builder = builder
            .set_override("auth.microsoft_email", email.clone())
            .unwrap()
    }
    builder.build().expect("Failed to create config!")
}
