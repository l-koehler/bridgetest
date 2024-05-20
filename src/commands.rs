/*
 * This file contains functions that perform specific actions
 * between the MT client and the MC server
 * For example the handshake function, which also creates and returns a
 * Minecraft client.
 */

use crate::settings;
use crate::utils;
use crate::serverbound_translator;
use crate::clientbound_translator;
use crate::MTServerState;
extern crate alloc;

use minetest_protocol::wire::command::CommandProperties;
use minetest_protocol::wire::command::ToServerCommand;
use minetest_protocol::MinetestConnection;
use minetest_protocol::wire::command::ToClientCommand;
use minetest_protocol::wire::command::HelloSpec;
use minetest_protocol::wire::command::AuthAcceptSpec;
use minetest_protocol::wire::command::InitSpec;
use minetest_protocol::wire::types;

use azalea_client::{Client, Account, inventory};

use tokio::sync::mpsc::UnboundedReceiver;
use alloc::boxed::Box;
use azalea_client::Event;
use azalea_protocol::packets::game::ClientboundGamePacket;
use config::Config;
use std::net::SocketAddr;

pub async fn mt_auto(command: ToServerCommand, mt_conn: &mut MinetestConnection, mc_client: &mut azalea::Client, mt_server_state: &mut MTServerState) {
    match command {
        ToServerCommand::Null(_) => (), // Drop NULL
        ToServerCommand::Init(_) => utils::logger("[Minetest] Client sent Init, but handshake already done!", 2),
        ToServerCommand::Init2(_) => utils::logger("[Minetest] Client sent Init2 (preferred language), this is not implemented and will be ignored.", 2),
        ToServerCommand::ModchannelJoin(_) => utils::logger("[Minetest] Client sent ModchannelJoin, this does not exist in MC", 2),
        ToServerCommand::ModchannelLeave(_) => utils::logger("[Minetest] Client sent ModchannelLeave, this does not exist in MC", 2),
        ToServerCommand::TSModchannelMsg(_) => utils::logger("[Minetest] Client sent TSModchannelMsg, this does not exist in MC", 2),
        ToServerCommand::Playerpos(specbox) => serverbound_translator::playerpos(mc_client, specbox, mt_server_state).await,
        ToServerCommand::TSChatMessage(specbox) => serverbound_translator::send_message(mc_client, specbox),
        ToServerCommand::Interact(specbox) => serverbound_translator::interact_generic(mt_conn, mc_client, specbox, mt_server_state).await,
        ToServerCommand::Playeritem(specbox) => serverbound_translator::set_mainhand(mc_client, specbox),
        ToServerCommand::InventoryAction(specbox) => serverbound_translator::inventory_generic(mt_conn, mc_client, specbox, mt_server_state).await,
        // Breaks yaw/pitch somehow, no clue why
        //ToServerCommand::Gotblocks(specbox) => serverbound_translator::gotblocks(mc_client, specbox, mt_conn, mt_server_state.current_dimension).await,
        _ => utils::logger(&format!("[Minetest] Got unimplemented command, dropping {}", command.command_name()), 2) // Drop packet if unable to match
    }
}

pub async fn mc_auto(command: azalea_client::Event, mt_conn: &mut MinetestConnection, mc_client: &mut azalea::Client, mt_server_state: &mut MTServerState, mc_conn: &mut UnboundedReceiver<Event>) {
    let cloned_command = command.clone();
    let command_name = utils::mc_packet_name(&cloned_command);
    match command {
        Event::AddPlayer(player_data) => clientbound_translator::add_player(player_data, mt_conn, mt_server_state).await,
        Event::Chat(message) => clientbound_translator::send_message(mt_conn, message).await,
        Event::Tick => on_minecraft_tick(mt_conn, mc_client, mt_server_state).await,
        Event::Death(_) => clientbound_translator::death(mt_conn, mt_server_state).await,
        Event::Packet(packet_value) => match (*packet_value).clone() {
            ClientboundGamePacket::ChunkBatchStart(_) => clientbound_translator::chunkbatch(mt_conn, mc_conn, mt_server_state, mc_client).await,
            ClientboundGamePacket::SystemChat(message) => clientbound_translator::send_sys_message(mt_conn, &message).await,
            ClientboundGamePacket::PlayerPosition(playerpos_packet) => clientbound_translator::set_player_pos(&playerpos_packet, mt_conn, mt_server_state).await,
            ClientboundGamePacket::SetTime(settime_packet) => clientbound_translator::set_time(&settime_packet, mt_conn).await,
            ClientboundGamePacket::SetHealth(sethealth_packet) => clientbound_translator::set_health(&sethealth_packet, mt_conn, mt_server_state).await,
            // these two are misleading. SetDefaultSpawnPosition sets the on-death respawn position,
            // Respawn (re)*SPAWNS* the player in a different dimension and is entirely unrelated to death!
            ClientboundGamePacket::SetDefaultSpawnPosition(setspawn_packet) => clientbound_translator::set_spawn(&setspawn_packet, mt_server_state).await,
            ClientboundGamePacket::Respawn(respawn_packet) => clientbound_translator::update_dimension(&respawn_packet, mt_server_state).await,

            ClientboundGamePacket::KeepAlive(_) => utils::logger("[Minecraft] Got KeepAlive packet, ignoring it.", 0),
            ClientboundGamePacket::AddEntity(addentity_packet) => clientbound_translator::add_entity(Some(&addentity_packet), mt_conn, mt_server_state).await,
            ClientboundGamePacket::MoveEntityPos(entitypos_packet) => clientbound_translator::entity_setpos(&entitypos_packet, mt_conn, mt_server_state).await,
            ClientboundGamePacket::TeleportEntity(entitytp_packet) => clientbound_translator::entity_teleport(&entitytp_packet, mt_conn, mt_server_state).await,
            ClientboundGamePacket::MoveEntityPosRot(entityposrot_packet) => clientbound_translator::entity_setposrot(&entityposrot_packet, mt_conn, mt_server_state).await,
            ClientboundGamePacket::MoveEntityRot(entityrot_packet) => clientbound_translator::entity_setrot(&entityrot_packet, mt_conn, mt_server_state).await,
            ClientboundGamePacket::SetEntityMotion(entitymotion_packet) => clientbound_translator::entity_setmotion(&entitymotion_packet, mt_conn, mt_server_state).await,
            ClientboundGamePacket::RemoveEntities(removeentity_packet) => clientbound_translator::remove_entity(&removeentity_packet, mt_conn, mt_server_state).await,
            ClientboundGamePacket::RotateHead(rotatehead_packet) => clientbound_translator::entity_rotatehead(&rotatehead_packet, mt_conn, mt_server_state).await,
            
            ClientboundGamePacket::EntityEvent(event_packet) => clientbound_translator::entity_event(&event_packet, mt_conn, mt_server_state).await,
            ClientboundGamePacket::SetEntityData(data_packet) => clientbound_translator::set_entity_data(&data_packet, mt_conn, mt_server_state).await,
            
            ClientboundGamePacket::OpenScreen(screen_packet) => clientbound_translator::open_screen(&screen_packet, mt_conn, mt_server_state).await,
            ClientboundGamePacket::ContainerSetContent(content_packet) => clientbound_translator::set_container_content(&content_packet, mt_conn, mt_server_state).await,
            ClientboundGamePacket::BlockEntityData(data_packet) => clientbound_translator::block_entity_data(&data_packet, mt_conn, mt_server_state).await,
            
            ClientboundGamePacket::BlockUpdate(blockupdate_packet) => clientbound_translator::blockupdate(&blockupdate_packet, mt_conn, mt_server_state).await,
            
            ClientboundGamePacket::UpdateRecipes(recipe_packet) => clientbound_translator::update_recipes(&recipe_packet, mt_server_state),
            _ => utils::logger(&format!("[Minecraft] Got unimplemented command, dropping {}", command_name), 2),
        }
        _ => utils::logger(&format!("[Minecraft] Got unimplemented command, dropping {}", command_name), 2),
    };
}

pub async fn on_minecraft_tick(mt_conn: &mut MinetestConnection, mc_client: &Client, mt_server_state: &mut MTServerState) {
    // update the MT clients position once every {settings::POS_FORCE_AFTER} ticks
    if mt_server_state.ticks_since_sync >= settings::POS_FORCE_AFTER {
        let new_position = utils::vec3_to_v3f(&mc_client.position(), 0.1);
        clientbound_translator::force_player_pos(new_position, mt_conn, mt_server_state).await;
    } else {
        mt_server_state.ticks_since_sync += 1;
    };
    // update the MT clients inventory if it changed
    // for stupid reasons, we don't use packets for this
    let mut to_update: Vec<(&str,Vec<inventory::ItemSlot>)> = vec![];
    match mc_client.menu() {
        inventory::Menu::Player(serverside_inventory) => {
            // fields of the inventory needing a update
            if serverside_inventory.craft_result != mt_server_state.mt_clientside_player_inv.craft_result {
                to_update.push(("craftpreview", vec![serverside_inventory.craft_result.clone()]));
            }
            if serverside_inventory.craft.as_slice() != mt_server_state.mt_clientside_player_inv.craft.as_slice() {
                to_update.push(("craft", serverside_inventory.craft.to_vec()))
            }
            if serverside_inventory.armor.as_slice() != mt_server_state.mt_clientside_player_inv.armor.as_slice() {
                to_update.push(("armor", serverside_inventory.armor.to_vec()))
            }
            if serverside_inventory.inventory.as_slice() != mt_server_state.mt_clientside_player_inv.inventory.as_slice() {
                to_update.push(("main", serverside_inventory.inventory.to_vec()))
            }
            if serverside_inventory.offhand != mt_server_state.mt_clientside_player_inv.offhand {
                to_update.push(("offhand", vec![serverside_inventory.offhand.clone()]))
            }
            mt_server_state.mt_clientside_player_inv = serverside_inventory;
        },
        // contents: SlotList<n>
        // different n per menu type, so incompatible types
        inventory::Menu::Generic9x1 { contents, player } => {
            to_update.push(("container", contents.to_vec()));
            to_update.push(("main", player.to_vec()))
        },
        inventory::Menu::Generic9x2 { contents, player } => {
            to_update.push(("container", contents.to_vec()));
            to_update.push(("main", player.to_vec()))
        },
        inventory::Menu::Generic9x3 { contents, player } => {
            to_update.push(("container", contents.to_vec()));
            to_update.push(("main", player.to_vec()))
        },
        inventory::Menu::Generic9x4 { contents, player } => {
            to_update.push(("container", contents.to_vec()));
            to_update.push(("main", player.to_vec()))
        },
        inventory::Menu::Generic9x5 { contents, player } => {
            to_update.push(("container", contents.to_vec()));
            to_update.push(("main", player.to_vec()))
        },
        inventory::Menu::Generic9x6 { contents, player } => {
            to_update.push(("container", contents.to_vec()));
            to_update.push(("main", player.to_vec()))
        },
        inventory::Menu::Generic3x3 { contents, player } => {
            to_update.push(("container", contents.to_vec()));
            to_update.push(("main", player.to_vec()))
        },
        inventory::Menu::Crafter3x3 { contents, player } => {
            to_update.push(("container", contents.to_vec()));
            to_update.push(("main", player.to_vec()))
        },
        _ => {
            to_update.push(("container", vec![inventory::ItemSlot::Empty; mt_server_state.container_size.into()]));
        }
    }
    if !to_update.is_empty() {
        clientbound_translator::update_inventory(mt_conn, to_update).await;
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

    let mut player_name = init_command.player_name;
    // if the name is "random", the random result only affects the MC server. the MT client will think the name is literal "random".
    mt_server_state.this_player.0 = player_name.clone();
    if player_name == "random" {
        player_name = utils::get_random_username();
        utils::logger(&format!("Using random username: {}", player_name), 1);
    }
    mt_server_state.this_player.1 = player_name.clone();
    mt_server_state.players.push(player_name.clone());

    // Send S->C Hello
    let hello_command = ToClientCommand::Hello(
        Box::new(HelloSpec {
            serialization_ver: 29, // as per https://docs.rs/minetest-protocol/0.1.4/src/minetest_protocol/wire/types.rs.html#2256-2262
            compression_mode: 1,
            proto_ver: 44,
            auth_mechs: types::AuthMechsBitset {
                legacy_password: false,
                srp: false,
                first_srp: true,
            },
            username_legacy: player_name.clone(),
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
    Client::join(&mc_account, mc_server_addr).await.expect("[Minecraft] Failed to log in!")
}
