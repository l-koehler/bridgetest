use crate::utils;
use azalea_client::Event;
use azalea_client::{Account, Client};
use config::Config;
use glam::Vec3 as v3f;
use log::*;
use luanti_protocol::LuantiConnection;
use luanti_protocol::commands::CommandProperties;
use luanti_protocol::commands::{client_to_server, client_to_server::ToServerCommand};
use luanti_protocol::commands::{server_to_client, server_to_client::ToClientCommand};
use luanti_protocol::types;
use std::net::SocketAddr;
use tokio::sync::mpsc::UnboundedReceiver;

pub async fn handshake(
    luanti_conn: &mut LuantiConnection,
    settings: &Config,
) -> (azalea::Client, UnboundedReceiver<azalea::Event>, String) {
    let mut command;
    loop {
        let t = luanti_conn.recv().await;
        match t {
            Err(_) => warn!("Got error from luanti_conn.recv(), skipping!"),
            Ok(_t) => {
                command = _t; // Cannot use _t directly, _t is valid only in the scope of the match
                match command {
                    ToServerCommand::Init(_) => break,
                    _ => warn!(
                        "Dropping unexpected C2S packet! Got serverbound \"{}\", expected \"Init\"",
                        command.command_name()
                    ),
                }
            }
        };
    }

    // command is guaranteed to be ToServerCommand::Init(Box<InitSpec>)
    let init_command: Box<client_to_server::InitSpec>;
    if let ToServerCommand::Init(extracted_box) = command {
        init_command = extracted_box;
    } else {
        error!("commands::handshake() got called with a ToServerCommand that was not a C->S Init");
        std::process::exit(1);
    }

    let mut player_name = init_command.user_name.clone();

    if player_name == "random" {
        if settings.get_bool("auth.allow_random_user").unwrap()
            && !settings.get_bool("auth.online_mode").unwrap()
        {
            player_name = utils::get_random_username();
            info!("Using random username: {}", player_name);
        } else {
            warn!("Using literal username \"random\", random usernames are disabled!");
        }
    }

    // Send S->C Hello
    let hello_command = ToClientCommand::Hello(Box::new(server_to_client::HelloSpec {
        serialization_version: 29, // as per https://docs.rs/minetest-protocol/0.1.4/src/luanti_protocol/wire/types.rs.html#2256-2262
        compression_mode: 1,
        protocol_version: 44,
        auth_mechs: types::AuthMechsBitset {
            legacy_password: false,
            srp: false,
            first_srp: true,
        },
        username_legacy: init_command.user_name.clone(),
    }));
    debug!("Sending S2C Hello");
    luanti_conn.send(hello_command).unwrap();
    // Wait for a C->S FirstSrp
    // TODO: this is right now just assuming the response is part of the authentication
    let _firstsrp = luanti_conn
        .recv()
        .await
        .expect("Client disconnected during authentication!");
    // Send S->C AuthAccept
    let auth_accept_command =
        ToClientCommand::AuthAccept(Box::new(server_to_client::AuthAcceptSpec {
            player_pos: v3f {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            map_seed: 0,
            recommended_send_interval: 0.1,
            sudo_auth_methods: 0,
        }));
    debug!("Sending S2C AuthAccept");
    luanti_conn.send(auth_accept_command).unwrap();
    info!("Connecting to Minecraft server");

    let mc_server_addr: SocketAddr = settings
        .get_string("net.mc_server")
        .unwrap()
        .parse()
        .unwrap();
    let mc_account: Account = match settings.get_bool("auth.online_mode").unwrap() {
        true => {
            let email = settings.get_string("auth.microsoft_email").unwrap();
            if !email.contains('@') {
                println!("Bad email! Use --account or set your E-Mail in the config file.");
                std::process::exit(1)
            }
            Account::microsoft(&email).await.unwrap_or_else(|_| {
                error!("Microsoft auth failed!");
                std::process::exit(1)
            })
        }
        false => Account::offline(player_name.as_str()),
    };

    let (client, mut mc_conn) = Client::join(&mc_account, mc_server_addr)
        .await
        .expect("Failed to log in!");

    debug!("Awaiting S2C Login confirmation...");
    loop {
        let t = mc_conn.recv().await;
        let command = t.expect("Minecraft Server sent disconnect while awaiting login");
        match command {
            // Recieved login packet from minecraft server
            Event::Login => break,
            _ => warn!(
                "Dropping unexpected S2C packet! Got clientbound \"{}\", expected \"Init\"",
                utils::mc_packet_name(&command)
            ),
        }
    }
    return (client, mc_conn, init_command.user_name);
}
