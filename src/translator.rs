/*
 * This file contains the loop in which packets from the MT Client are
 * processed (fn client_handler).
 * Also, this file is badly named (as you might have noticed).
 */

use crate::utils;
use crate::commands;
use crate::MTServerState; // ok this is stupid to do whatever it works

use minetest_protocol::peer::peer::PeerError;
use minetest_protocol::wire::command::CommandProperties;
use minetest_protocol::wire::command::ToServerCommand;
use minetest_protocol::MinetestConnection;
use minetest_protocol::MinetestServer;

use azalea;
use azalea_client;
use azalea_protocol::packets::game::ClientboundGamePacket;

use tokio::sync::mpsc::UnboundedReceiver;

pub async fn client_handler(_mt_server: MinetestServer, mut mt_conn: MinetestConnection, mut mt_server_state: MTServerState) {
    println!("[Debug] async translator::client_handler()");
    /*
     * The first few packets (handshake) are outside the main loop, because
     * at this point the minecraft client isn't initialized yet.
     */
    let mut command;
    loop {
        let t = mt_conn.recv().await;
        match t {
            Err(_) => utils::logger("[Minetest] got Error from conn.recv(), skipping!", 2),
            Ok(_t) => {
                command = _t; // Cannot use _t directly, _t is valid only in the scope of the match
                match command {
                    ToServerCommand::Init(_) => break,
                    _ => utils::logger(&format!("[Minetest] Dropping unexpected packet! Got serverbound \"{}\", expected \"Init\"", command.command_name()), 2),
                }
            }
        };

    }
    let (mc_client, mut mc_conn) = commands::handshake(command, &mut mt_conn, &mut mt_server_state).await;
    // Await a LOGIN packet
    // It verifies that the client is now in the server world
    println!("[Minecraft] Awaiting S->C Login confirmation...");
    loop {
        let t = mc_conn.recv().await;
        let command = t.expect("[Minecraft] Server sent disconnect while awaiting login");
        if utils::mc_packet_name(&command) == "Login" {
            // Recieved login packet from minecraft server
            break;
        }
        println!("[Minetest] Dropping unexpected packet! Got serverbound \"{}\", expected \"Init\"", utils::mc_packet_name(&command));
    }

    println!("[Debug] Authenticated with both client and server.");
    /*
     * Main Loop.
     * At this point, both the minetest client and the minecraft server
     * are connected.
     * mt_conn refers to the connection to the minetest client
     * mc_client and mc_conn refer to the minecraft client and its connection
     */
    loop {
        tokio::select! {
            // recieve data over the MinetestConnection
            t = mt_conn.recv() => {
                // Check if the client disconnected
                match t {
                    Ok(_) => (),
                    Err(err) => {
                        let show_err = if let Some(err) = err.downcast_ref::<PeerError>() {
                            match err {
                                PeerError::PeerSentDisconnect => false,
                                _ => true,
                            }
                        } else {
                            true
                        };
                        if show_err {
                            println!("[Minetest] Client Disconnected: {:?}", err)
                        } else {
                            println!("[Minetest] Client Disconnected")
                        }
                        break; // Exit the client handler on client disconnect
                    }
                }
                let mt_command = t.unwrap();
                utils::show_mt_command(&mt_command);
                commands::mt_auto(mt_command, &mut mt_conn, &mc_client).await;
            },
            // or the minecraft connection
            t = mc_conn.recv() => {
                // t: azalea_client::Event
                match t {
                    Some(_) => {
                        let mc_command = t.expect("[Minecraft] Failed to unwrap non-empty packet from Server!");
                        utils::show_mc_command(&mc_command);
                        commands::mc_auto(mc_command, &mut mt_conn, &mc_client, &mut mt_server_state).await;
                    },
                    // This should NOT happen, why does it happen thousands of times per second?? TODO!
                    None => utils::logger(&format!("[Minecraft] Recieved empty/none, skipping: {:#?}", t), 2),
                }
            }
        }
    }
}
