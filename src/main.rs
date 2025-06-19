#![feature(variant_count)]
#![feature(slice_as_array)]
#![feature(slice_pattern)]
#![feature(string_remove_matches)]
#![feature(string_into_chars)]
// fuck this warning.
// sure the language doesn't need the parens, but this isn't codegolf. i need legible code
#![allow(unused_parens)]

mod handshake;
mod settings;
mod translator;
mod utils;

mod c2s;
mod s2c;
mod state;

use log::*;
use luanti_protocol::LuantiServer;

use clap::Arg;
use config::Config;
use std::fs::File;
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[tokio::main]
async fn main() {
    env_logger::init();
    let settings: Config = load_config();
    s2c::media::fetch_models(&settings).await;
    start_client_handler(settings).await;
}

async fn start_client_handler(settings: Config) {
    // Create a luanti server
    let mt_addr: IpAddr = match settings.get("net.local_only").unwrap() {
        true => Ipv4Addr::new(127, 0, 0, 1),
        false => Ipv4Addr::new(0, 0, 0, 0),
    }
    .into();
    let mt_port = settings.get_int("net.luanti_port").unwrap() as u16;
    info!("Creating Luanti server ({}:{})...", mt_addr, mt_port);
    let mut mt_server = LuantiServer::new(SocketAddr::new(mt_addr, mt_port));

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
            translator::client_handler(mt_server, conn, settings).await;
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
