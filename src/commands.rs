/*
 * This file contains functions that perform specific actions
 * between the MT client and the MC server
 * For example the handshake function, which also creates and returns a
 * Minecraft client.
 */

use std::net::ToSocketAddrs;

use crate::utils;
use crate::settings;
use crate::clientbound_translator;
use crate::MTServerState;
extern crate alloc;

use minetest_protocol::wire::command::ToServerCommand;
use minetest_protocol::MinetestConnection;
use minetest_protocol::wire::command::ToClientCommand;
use minetest_protocol::wire::command::HelloSpec;
use minetest_protocol::wire::command::AuthAcceptSpec;
use minetest_protocol::wire::command::InitSpec;
use minetest_protocol::wire::types;

use azalea;
use azalea_client::{Client, Account};

use tokio::sync::mpsc::UnboundedReceiver;
use alloc::boxed::Box;
use azalea_client::Event;
use azalea_protocol::packets::game::ClientboundGamePacket;
use config::Config;
use std::net::SocketAddr;

pub async fn mt_auto(command: ToServerCommand, conn: &mut MinetestConnection, mc_client: &azalea::Client) {
    match command {
        ToServerCommand::Null(_) => (), // Drop NULL
        ToServerCommand::Init(_) => utils::logger("[Minetest] Client sent Init, but handshake already done!", 2),
        ToServerCommand::Init2(_) => utils::logger("[Minetest] Client sent Init2 (preferred language), this is not implemented and will be ignored.", 2),
        ToServerCommand::ModchannelJoin(_) => utils::logger("[Minetest] Client sent ModchannelJoin, this does not exist in MC", 2),
        ToServerCommand::ModchannelLeave(_) => utils::logger("[Minetest] Client sent ModchannelLeave, this does not exist in MC", 2),
        ToServerCommand::TSModchannelMsg(_) => utils::logger("[Minetest] Client sent TSModchannelMsg, this does not exist in MC", 2),
        _ => utils::logger("[Minetest] Got unimplemented command, dropping packet!", 2) // Drop packet if unable to match
    }
}

pub async fn mc_auto(command: azalea_client::Event, mt_conn: &mut MinetestConnection, mc_client: &azalea::Client, mt_server_state: &mut MTServerState, mc_conn: &mut UnboundedReceiver<Event>) {
    let command_clone = command.clone(); //  The borrow checker is my only mortal enemy
    let command_name = utils::mc_packet_name(&command_clone);
    match command {
        Event::AddPlayer(player_data) => clientbound_translator::add_player(player_data, mt_conn, mt_server_state).await,
        Event::Packet(packet_value) => match *packet_value {
            ClientboundGamePacket::ChunkBatchStart(_) => clientbound_translator::chunkbatch(mt_conn, mc_conn, &mc_client).await,
            _ => utils::logger(&format!("[Minecraft] Got unimplemented command, dropping {}", command_name), 2),
        },
        _ => utils::logger(&format!("[Minecraft] Got unimplemented command, dropping {}", command_name), 2),
    }
}

pub async fn handshake(command: ToServerCommand, conn: &mut MinetestConnection, mt_server_state: &mut MTServerState, settings: &Config) -> (azalea::Client, UnboundedReceiver<azalea::Event>) {
    // command is guaranteed to be ToServerCommand::Init(Box<InitSpec>)
    let init_command: Box<InitSpec>;
    if let ToServerCommand::Init(extracted_box) = command {
        init_command = extracted_box;
    } else {
        utils::logger("commands::handshake() got called with a ToServerCommand that was not a C->S Init", 3);
        panic!("handshake() got called with non-init packet!")
    }
    let player_name = init_command.player_name.clone(); // Fix to not trip the borrow checker (Needed for Account::offline)
    mt_server_state.players.push(player_name.clone()); // I FUCKING [mildy dislike (affectionate)] THE BORROW CHECKER AAAAA
    // Send S->C Hello
    let hello_command = ToClientCommand::Hello(
        Box::new(HelloSpec {
            serialization_ver: 0,
            compression_mode: 0,
            proto_ver: 44,
            auth_mechs: types::AuthMechsBitset {
                legacy_password: false,
                srp: false,
                first_srp: true,
            },
            username_legacy: init_command.player_name,
        })
    );
    let _ = conn.send(hello_command).await;
    utils::logger("[Minetest] S->C Hello", 1);
    // Wait for a C->S FirstSrp
    // TODO: this is right now just assuming the response is part of the authentication
    let second_response = conn.recv().await.expect("Client disconnected during authentication!");
    utils::show_mt_command(&second_response);
    // Send S->C AuthAccept
    let auth_accept_command = ToClientCommand::AuthAccept(
        Box::new(AuthAcceptSpec {
            player_pos: types::v3f {
                // TODO: Sane defaults are impossible here
                // Teleport the player as soon as DefaultSpawnLocation is recieved or something?
                 x: 0.0,
                 y: 0.0,
                 z: 90.0,
            },
            map_seed: 0,
            recommended_send_interval: 0.1,
            sudo_auth_methods: 0,
        })
    );
    let _ = conn.send(auth_accept_command).await;
    utils::logger("[Minetest] S->C AuthAccept", 1);
    utils::logger("[Minecraft] Logging in...", 1);
    
    // TODO: Change this line to allow online accounts
    let mc_server_addr: SocketAddr = settings.get_string("mc_server_addr").expect("Failed to read config!")
                                             .parse().expect("Failed to parse mc_server_addr!");
    let mc_account: Account = Account::offline(player_name.as_str());
    let (mut mc_client, mut rx) = Client::join(&mc_account, mc_server_addr).await.expect("[Minecraft] Failed to log in!");
    return (mc_client, rx)
}
