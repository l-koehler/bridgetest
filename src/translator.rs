/*
 * This file contains the loop in which packets from the MT Client are
 * processed (fn client_handler).
 * Also, this file is badly named (as you might have noticed).
 */

use crate::MTServerState;
use crate::clientbound_translator;
use crate::commands;
use crate::mt_definitions;
use crate::on_tick;
use crate::settings;
use crate::textures;
use crate::utils; // ok this is stupid to do whatever it works (i need global variables) (for normal reasons)

use luanti_protocol::LuantiConnection;
use luanti_protocol::LuantiServer;
use luanti_protocol::commands::CommandProperties;
use luanti_protocol::commands::client_to_server::ToServerCommand;
use luanti_protocol::peer::PeerError;

use azalea_client::Event;
use config::Config;
use log::*;
use std::time::Duration;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::IntervalStream;

pub async fn client_handler(
    _mt_server: LuantiServer,
    mut mt_conn: LuantiConnection,
    mut mt_server_state: MTServerState,
    settings: Config,
) {
    /*
     * The first few packets (handshake) are outside the main loop, because
     * at this point the minecraft client isn't initialized yet.
     */
    let mut command;
    loop {
        let t = mt_conn.recv().await;
        match t {
            Err(_) => warn!("Got error from mt_conn.recv(), skipping!"),
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
    let (mut mc_client, mut mc_conn) =
        commands::handshake(command, &mut mt_conn, &mut mt_server_state, &settings).await;
    // Await a LOGIN packet
    // It verifies that the client is now in the server world
    debug!("Awaiting S2C Login confirmation...");
    loop {
        let t = mc_conn.recv().await;
        let command = t.expect("[Minecraft] Server sent disconnect while awaiting login");
        match command {
            // Recieved login packet from minecraft server
            Event::Login => break,
            _ => warn!(
                "Dropping unexpected C2S packet! Got serverbound \"{:?}\", expected \"Init\"",
                command
            ),
        }
    }

    mt_server_state.item_texture_map = textures::load_item_mappings();
    mt_server_state.nodebox_lookup = textures::load_nodeboxes();
    mt_server_state.block_texture_map =
        textures::load_block_mappings(&mt_server_state.nodebox_lookup);

    mt_conn.send(textures::get_announcement()).unwrap();

    debug!("Sending S2C Itemdef");
    mt_conn
        .send(mt_definitions::get_item_def_command(&mt_server_state).await)
        .unwrap();
    debug!("Sending S2C Nodedef");
    mt_conn
        .send(mt_definitions::get_node_def_command(&settings, &mut mt_server_state).await)
        .unwrap();

    debug!("Sending S2C MovementSpec");
    mt_conn
        .send(mt_definitions::get_movementspec(4.317))
        .unwrap();

    debug!("Sending S2C SetPriv");
    mt_conn.send(mt_definitions::get_defaultpriv()).unwrap();

    debug!("Sending S2C AddHUD (HealthBar)");
    mt_conn.send(mt_definitions::add_healthbar()).unwrap();
    debug!("Sending S2C AddHUD (FoodBar)");
    mt_conn.send(mt_definitions::add_foodbar()).unwrap();
    debug!("Sending S2C AddHUD (AirBar)");
    mt_conn.send(mt_definitions::add_airbar()).unwrap();
    debug!("Sending S2C AddHUD (Subtitles)");
    mt_conn.send(mt_definitions::add_subtitlebox()).unwrap();

    debug!("Sending S2C Formspec (Inventory)");
    mt_conn
        .send(mt_definitions::get_inventory_formspec(
            settings::PLAYER_INV_FORMSPEC,
        ))
        .unwrap();

    debug!("Sending S2C CsmRestrictions");
    mt_conn.send(mt_definitions::get_csmrestrictions()).unwrap();

    info!("Awaiting C2S ClientReady");
    loop {
        let t = mt_conn.recv().await;
        let command = t.unwrap();
        match command {
            ToServerCommand::RequestMedia(packet) => {
                mt_conn.send(textures::handle_request(packet)).unwrap();
            }
            ToServerCommand::ClientReady(_) => break,
            _ => warn!(
                "Dropping unexpected C2S packet! Got serverbound \"{}\", expected \"ClientReady\"",
                command.command_name()
            ),
        }
    }

    debug!("Sending S2C Hotbar Definition");
    mt_conn.send(mt_definitions::set_hotbar_size()).unwrap();
    mt_conn.send(mt_definitions::set_hotbar_texture()).unwrap();
    mt_conn.send(mt_definitions::set_hotbar_selected()).unwrap();

    debug!("Sending S2C Inventory Data");
    mt_conn.send(mt_definitions::empty_inventory()).unwrap();

    debug!("Sending S2C SetSky, SetSun, SetMoon, SetStars, OverrideDayNightRatio");
    for thing in mt_definitions::get_sky_stuff() {
        mt_conn.send(thing).unwrap();
    }

    debug!("Sending S2C ActiveObjectRemoveAdd (add LocalPlayer)");
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
                            error!("Client Disconnected: {:?}", err);
                        } else {
                            println!("Client Disconnected");
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
                    None => info!("Received empty C2S packet, skipping: {:#?}", t),
                }
            },
            // or run the tick function if no packets are waiting to be processed
            _ = stream.next() => {
                on_tick::server(&mut mt_conn, &mc_client, &mut mt_server_state).await;
            }
        }
    }
}
