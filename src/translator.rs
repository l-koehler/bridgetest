/*
 * This file contains the loop in which packets from the MT Client are
 * processed (fn client_handler).
 * Also, this file is badly named (as you might have noticed).
 */

use crate::mt_definitions;
use crate::utils;
use crate::commands;
use crate::clientbound_translator;
use crate::MTServerState; // ok this is stupid to do whatever it works (i need global variables) (for normal reasons)

use minetest_protocol::peer::peer::PeerError;
use minetest_protocol::wire::command::CommandProperties;
use minetest_protocol::wire::command::ToServerCommand;
use minetest_protocol::MinetestConnection;
use minetest_protocol::MinetestServer;

use azalea_client::Event;
use config::Config;

pub async fn client_handler(_mt_server: MinetestServer, mut mt_conn: MinetestConnection, mut mt_server_state: MTServerState, settings: Config) {
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
    let (mc_client, mut mc_conn) = commands::handshake(command, &mut mt_conn, &mut mt_server_state, &settings).await;
    // Await a LOGIN packet
    // It verifies that the client is now in the server world
    utils::logger("[Minecraft] Awaiting S->C Login confirmation...", 1);
    loop {
        let t = mc_conn.recv().await;
        let command = t.expect("[Minecraft] Server sent disconnect while awaiting login");
        match command {
            // Recieved login packet from minecraft server
            Event::Login => break,
            _ => utils::logger(&format!("[Minetest] Dropping unexpected packet! Got serverbound \"{}\", expected \"Init\"", utils::mc_packet_name(&command)), 1),
        }
    }

    utils::logger("Authenticated with both client and server.", 1);
    
    let media_packets = mt_definitions::get_texture_media_commands(&settings).await;
    utils::logger("[Minetest] S->C MediaAnnouncement", 1); // This will cause cached media to load, breaking stuff (idc)
    let _ = mt_conn.send(media_packets.0).await;
    utils::logger("[Minetest] S->C Media (Blocks)", 1);
    let _ = mt_conn.send(media_packets.1).await;
    // utils::logger("[Minetest] S->C Media (Particle)", 1);
    // let _ = mt_conn.send(media_packets.2).await;
    // utils::logger("[Minetest] S->C Media (Entity)", 1);
    // let _ = mt_conn.send(media_packets.3).await;
    // utils::logger("[Minetest] S->C Media (Item)", 1);
    // let _ = mt_conn.send(media_packets.4).await;
    // utils::logger("[Minetest] S->C Media (Misc)", 1);
    // let _ = mt_conn.send(media_packets.5).await;
    // utils::logger("[Minetest] S->C Inventory Formspec", 1);
    // let _ = mt_conn.send(mt_definitions::get_inventory_formspec()).await;

    utils::logger("[Minetest] S->C Itemdef", 1);
    let _ = mt_conn.send(mt_definitions::get_item_def_command(&settings).await).await;
    utils::logger("[Minetest] S->C Nodedef", 1);
    let _ = mt_conn.send(mt_definitions::get_node_def_command(&settings).await).await;
    
    utils::logger("[Minetest] S->C SetSun", 1);
    let _ = mt_conn.send(mt_definitions::get_sun_def_command()).await;
    
    utils::logger("[Minetest] S->C SetLighting", 1);
    let _ = mt_conn.send(mt_definitions::get_lighting_def_command()).await;

    utils::logger("[Minetest] S->C Movement", 1);
    let _ = mt_conn.send(mt_definitions::get_movementspec()).await;

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
                            utils::logger(&format!("[Minetest] Client Disconnected: {:?}", err), 1)
                        } else {
                            utils::logger("[Minetest] Client Disconnected", 1)
                        }
                        break; // Exit the client handler on client disconnect
                    }
                }
                let mt_command = t.expect("[Minetest] Failed to unwrap Ok(_) packet from Client!");
                utils::show_mt_command(&mt_command);
                commands::mt_auto(mt_command, &mut mt_conn, &mc_client, &mt_server_state).await;
            },
            // or the minecraft connection
            t = mc_conn.recv() => {
                match t {
                    Some(_) => {
                        let mc_command = t.expect("[Minecraft] Failed to unwrap non-empty packet from Server!");
                        utils::show_mc_command(&mc_command);
                        commands::mc_auto(mc_command, &mut mt_conn, &mc_client, &mut mt_server_state, &mut mc_conn).await;
                    },
                    None => utils::logger(&format!("[Minecraft] Recieved empty/none, skipping: {:#?}", t), 2),
                }
            }
        }
    }
}
