/*
 * This file contains the loop in which packets from the MT Client are
 * processed (fn client_handler).
 * Also, this file is badly named (as you might have noticed).
 */

use crate::clientbound_translator;
use crate::mt_definitions;
use crate::textures;
use crate::utils;
use crate::commands;
use crate::MTServerState; // ok this is stupid to do whatever it works (i need global variables) (for normal reasons)
use crate::settings;
use crate::on_tick;

use luanti_protocol::peer::PeerError;
use luanti_protocol::commands::CommandProperties;
use luanti_protocol::commands::client_to_server::ToServerCommand;
use luanti_protocol::LuantiConnection;
use luanti_protocol::LuantiServer;

use azalea_client::Event;
use config::Config;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;
use std::time::Duration;

pub async fn client_handler(_mt_server: LuantiServer, mut mt_conn: LuantiConnection, mut mt_server_state: MTServerState, settings: Config) {
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
    let (mut mc_client, mut mc_conn) = commands::handshake(command, &mut mt_conn, &mut mt_server_state, &settings).await;
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
    
    // let media_packets = mt_definitions::get_texture_media_commands(&settings, &mut mt_server_state).await;
    // let packet_names = ["MediaAnnouncement", "Media (Blocks)", "Media (Particle)", "Media (Entity)", "Media (Item)", "Media (Other)"];
    // for index in 0..media_packets.len() {
    //     utils::logger(&format!("[Minetest] S->C {}", packet_names[index]), 1);
    //     let _ = mt_conn.send(media_packets[index].clone()).await;
    // }
    mt_server_state.item_texture_map = textures::load_item_mappings();
    mt_server_state.nodebox_lookup = textures::load_nodeboxes();
    mt_server_state.block_texture_map = textures::load_block_mappings(&mt_server_state.nodebox_lookup);

    mt_conn.send(textures::get_announcement()).unwrap();

    utils::logger("[Minetest] S->C Itemdef", 1);
    mt_conn.send(mt_definitions::get_item_def_command(&mt_server_state).await).unwrap();
    utils::logger("[Minetest] S->C Nodedef", 1);
    mt_conn.send(mt_definitions::get_node_def_command(&settings, &mut mt_server_state).await).unwrap();
    
    utils::logger("[Minetest] S->C Movement", 1);
    mt_conn.send(mt_definitions::get_movementspec(4.317)).unwrap();
    
    utils::logger("[Minetest] S->C SetPriv", 1);
    mt_conn.send(mt_definitions::get_defaultpriv()).unwrap();
    
    utils::logger("[Minetest] S->C AddHud Healthbar", 1);
    mt_conn.send(mt_definitions::add_healthbar()).unwrap();
    utils::logger("[Minetest] S->C AddHud Foodbar", 1);
    mt_conn.send(mt_definitions::add_foodbar()).unwrap();
    utils::logger("[Minetest] S->C AddHud Airbar", 1);
    mt_conn.send(mt_definitions::add_airbar()).unwrap();
    utils::logger("[Minetest] S->C AddHud Subtitles", 1);
    mt_conn.send(mt_definitions::add_subtitlebox()).unwrap();
    
    utils::logger("[Minetest] S->C Formspec", 1);
    mt_conn.send(mt_definitions::get_inventory_formspec(settings::PLAYER_INV_FORMSPEC)).unwrap();
    
    utils::logger("[Minetest] S->C CsmRestrictions", 1);
    mt_conn.send(mt_definitions::get_csmrestrictions()).unwrap();
    
    utils::logger("Awaiting ClientReady", 1);
    loop {
        let t = mt_conn.recv().await;
        let command = t.unwrap();
        match command {
            ToServerCommand::RequestMedia(packet) => {
                mt_conn.send(textures::handle_request( packet)).unwrap();
            },
            ToServerCommand::ClientReady(_) => break,
            _ => utils::logger(&format!("[Minetest] Dropping unexpected packet! Got serverbound \"{}\", expected \"ClientReady\"!", command.command_name()), 2)
        }
    }
    
    utils::logger("[Minetest] S->C Hotbar Definition", 1);
    mt_conn.send(mt_definitions::set_hotbar_size()).unwrap();
    mt_conn.send(mt_definitions::set_hotbar_texture()).unwrap();
    mt_conn.send(mt_definitions::set_hotbar_selected()).unwrap();
    
    utils::logger("[Minetest] S->C Inventory", 1);
    mt_conn.send(mt_definitions::empty_inventory()).unwrap();
    
    utils::logger("[Minetest] S->C SetSky, SetSun, SetMoon, SetStars, OverrideDayNightRatio ", 1);
    for thing in mt_definitions::get_sky_stuff() {
        mt_conn.send(thing).unwrap();
    }
    
    utils::logger("[Minetest] S->C ActiveObjectRemoveAdd LocalPlayer", 1);
    clientbound_translator::add_entity(None, &mut mt_conn, &mut mt_server_state).await;
    /*
     * Main Loop.
     * At this point, both the minetest client and the minecraft server
     * are connected.
     * mt_conn refers to the connection to the minetest client
     * mc_client and mc_conn refer to the minecraft client and its connection
     * we also run a tick function every 50ms
     */
    let mut stream = IntervalStream::new(tokio::time::interval(Duration::from_millis(50)));
    loop {
        tokio::select! {
            // recieve data over the LuantiConnection
            t = mt_conn.recv() => {
                // Check if the client disconnected
                match t {
                    Ok(_) => (),
                    Err(err) => {
                        let show_err = if let Some(err) = err.downcast_ref::<PeerError>() {
                            !matches!(err, PeerError::PeerSentDisconnect)
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
                commands::mt_auto(mt_command, &mut mt_conn, &mut mc_client, &mut mt_server_state).await;
            },
            // or the minecraft connection
            t = mc_conn.recv() => {
                match t {
                    Some(_) => {
                        let mc_command = t.expect("[Minecraft] Failed to unwrap non-empty packet from Server!");
                        utils::show_mc_command(&mc_command);
                        commands::mc_auto(mc_command, &mut mt_conn, &mut mc_client, &mut mt_server_state, &mut mc_conn).await;
                    },
                    None => utils::logger(&format!("[Minecraft] Recieved empty/none, skipping: {:#?}", t), 2),
                }
            },
            // or run the tick function if no packets are waiting to be processed
            _ = stream.next() => {
                on_tick::server(&mut mt_conn, &mc_client, &mut mt_server_state).await;
            }
        }
    }
}
