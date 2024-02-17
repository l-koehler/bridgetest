/*
 * This file contains the loop in which packets from the MT Client are
 * processed (fn client_handler).
 * Also, this file is badly named (as you might have noticed).
 */

use crate::utils;
use crate::commands;
use crate::packet_handler;

use minetest_protocol::peer::peer::PeerError;
use minetest_protocol::wire::command::CommandProperties;
use minetest_protocol::wire::command::ToServerCommand;
use minetest_protocol::MinetestConnection;
use minetest_protocol::MinetestServer;

use azalea;

use tokio::sync::mpsc::UnboundedReceiver;

pub async fn client_handler(_mt_server: MinetestServer, mut conn: MinetestConnection) {
    println!("[Debug] async translator::client_handler()");
    /*
     * The first few packets (handshake) are outside the main loop, because
     * at this point the minecraft client isn't initialized yet.
     */
    let mut command;
    loop {
        let t = conn.recv().await;
        command = t.expect("[Minetest] Client sent disconnect during handshake!");
        if command.command_name() == "Init" {
            // Recieved init packet from minetest client
            break;
        } else {
            // for some reason the first packet was NOT init
            println!("[Minetest] Dropping unexpected packet! Got serverbound \"{}\", expected \"Init\"", command.command_name());
        }
    }
    let (mc_client, rx) = commands::handshake(command, &mut conn).await;
    //mc_client.chat("Hello, world!");

    println!("[Debug] Authenticated with both client and server.");
    /*
     * Main Loop.
     * At this point, both the minetest client and the minecraft server
     * are connected.
     * conn refers to the connection to the minetest client
     * mc_client and rx refer to the minecraft client and its connection
     */
    loop {
        tokio::select! {
            // recieve data over the MinetestConnection
            t = conn.recv() => {
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
                let command = t.unwrap();
                utils::show_mt_command(&command);
                packet_handler::auto(command, &mut conn).await;
            },
            // or recieve data from minecraft TODO
        }
    }
}
