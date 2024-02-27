// unsurprisingly has absolutely nothing to do with translator.rs
// i am terrible at naming things
// anyways this contains functions that TAKE data from the minecraft server
// and send it to the minetest client.

extern crate alloc;

use crate::utils;
use crate::mt_definitions;
use crate::MTServerState;

use azalea_registry::Registry;
use minetest_protocol::wire::command::ToClientCommand;
use minetest_protocol::MinetestConnection;
use minetest_protocol::wire;
use minetest_protocol::wire::types::{v3s16, MapNodesBulk, MapNode, MapBlock, NodeMetadataList};

use azalea_client::PlayerInfo;
use azalea_client::Client;
use azalea_client::chat::ChatPacket;
use azalea::inventory::ItemSlotData;

use tokio::sync::mpsc::UnboundedReceiver;
use azalea_client::Event;
use azalea_protocol::packets::game::ClientboundGamePacket;
use azalea_protocol::packets::game::clientbound_level_chunk_with_light_packet::{ClientboundLevelChunkWithLightPacket, ClientboundLevelChunkPacketData};
use azalea_protocol::packets::game::clientbound_system_chat_packet::ClientboundSystemChatPacket;
use std::sync::Arc;
use azalea_core::position::ChunkPos;

/*
 * slot_id maps to slots in the players inventory.
 */
pub async fn send_item_if_missing(slotdata: ItemSlotData, slot_id: usize) {
    let item = slotdata.kind;
    let count = slotdata.count;
    utils::logger(&format!("[Minecraft] Unimplemented InvSync Slot:{} Item:{}*{} [ID:{}]", slot_id, count, item, item.to_u32()), 2);
}

pub async fn send_message(conn: &mut MinetestConnection, message: ChatPacket) {
    let chat_packet = ToClientCommand::TCChatMessage(
        Box::new(wire::command::TCChatMessageSpec {
            version: 1, // idk what this or message_type do
            message_type: 1, // but it works, dont touch it
            sender: message.username().unwrap_or(String::from("")),
            message: message.message().to_string(),
            timestamp: chrono::Utc::now().timestamp().try_into().unwrap_or(0),
        })
    );
    let _ = conn.send(chat_packet).await;
}

pub async fn send_sys_message(conn: &mut MinetestConnection, message: &ClientboundSystemChatPacket) {
    match &message.content {
        azalea_chat::FormattedText::Text(component) => {
            let chat_packet = ToClientCommand::TCChatMessage(
                Box::new(wire::command::TCChatMessageSpec {
                    version: 1, // idk what this or message_type do
                    message_type: 1, // but it works, dont touch it
                    sender: String::from("System"),
                    message: component.text.to_string(),
                    timestamp: chrono::Utc::now().timestamp().try_into().unwrap_or(0),
                })
            );
            let _ = conn.send(chat_packet).await;
        },
        _ => (),
    }

}

pub async fn initialize_16node_chunk(x_pos:i16, y_pos:i16, z_pos:i16, conn: &mut MinetestConnection, node_arr: [MapNode; 4096]) {
    // Fills a 16^3 area with a vector of map nodes, where param0 is a MC-compatible ID.
    // remember that this is limited to 16 blocks of heigth, while a MC chunk goes from -64 to 320
    // y_pos of 0 -> actual y filled from 0 to 16
    // so call it with y values ranging from -4 to 20 in order to fill a chunk
    
    /* simplified representation of the array, for a 3^3 cube.
     * in actual use, its a 16^3 cube. each number is a minecraft blockid.
     *
     *      one "line" along the X axis
     *        |
     *      /---\  /------/------------- gets repeated for each Y, to be a 3^2 slice
     * z=2: 0,0,0, 0,0,0, 0,0,0,
     * z=1: 0,0,0, 0,0,0, 0,0,0, \___ gets repeated for each Z, to be a 3^3 cube
     * z=0: 0,0,0, 0,0,0, 0,0,0, /
     */
    utils::logger(&format!("[Minetest] S->C Initializing 16^3 nodes at {}{}{}", x_pos, y_pos, z_pos), 1);
    // TODO this does not support metadata
    let mut metadata_vec = Vec::new();
    for x in 0..15 {
        for y in 0..15 {
            for z in 0..15 {
                metadata_vec.push(mt_definitions::get_metadata_placeholder(x, y, z))
            }
        }
    }
    let addblockcommand = ToClientCommand::Blockdata(
        Box::new(wire::command::BlockdataSpec {
            pos: v3s16 { x: x_pos, y: y_pos, z: z_pos },
            block: MapBlock {
                is_underground: false,
                 day_night_diff: false,
                 generated: false,
                 lighting_complete: None,
                 nodes: MapNodesBulk {
                     nodes: node_arr,
                },
                node_metadata: NodeMetadataList {
                    metadata: metadata_vec,
                }
            },
            network_specific_version: 44
        })
    );
    let _ = conn.send(addblockcommand).await;
}

pub async fn setblock(x: i16, y: i16, z: i16, id: u16, mt_conn: &mut MinetestConnection) {
    let addblockcommand = ToClientCommand::Addnode(
        Box::new(wire::command::AddnodeSpec {
            pos: v3s16 { x, y, z },
            keep_metadata: true,
            node: MapNode {
                param0: id,
                param1: 1,
                param2: 1,
            }
        })
    );
    let _ = mt_conn.send(addblockcommand).await;
}

pub async fn add_player(player_data: PlayerInfo, conn: &mut MinetestConnection, mt_server_state: &mut MTServerState) {
    let new_user: String = player_data.profile.name.to_string();
    mt_server_state.players.push(new_user);
    let add_player_command = ToClientCommand::UpdatePlayerList(
        Box::new(wire::command::UpdatePlayerListSpec {
            typ: 0,
            players: mt_server_state.players.clone(),
        })
    );
    let _ = conn.send(add_player_command).await;
    utils::logger("[Minetest] S->C UpdatePlayerList", 1);
}

pub async fn chunkbatch(mt_conn: &mut MinetestConnection, mc_conn: &mut UnboundedReceiver<Event>, mc_client: &Client) {
    utils::logger("[Minetest] Forwarding ChunkBatch...", 1);
    // called by a ChunkBatchStart
    // first let azalea do everything until ChunkBatchFinished,
    // then move the azalea world over to the client
    loop {
        tokio::select! {
            t = mc_conn.recv() => {
                match t {
                    Some(_) => {
                        let mc_command = t.expect("[Minecraft] Failed to unwrap non-empty packet from Server!");
                        utils::show_mc_command(&mc_command);
                        match mc_command {
                            Event::Packet(packet_value) => match Arc::unwrap_or_clone(packet_value) {
                                ClientboundGamePacket::LevelChunkWithLight(packet_data) => {
                                    utils::logger("[Minecraft] S->C LevelchunkWithLight", 1);
                                    store_level_chunk(&packet_data, mc_client);
                                },
                                ClientboundGamePacket::ChunkBatchFinished(_) => {
                                    utils::logger("[Minecraft] S->C ChunkBatchFinished", 1);
                                    send_all_chunks(mt_conn, mc_client).await;
                                    return; // Done
                                },
                                _ => (),
                            },
                            _ => (),
                        }
                    },
                    None => utils::logger(&format!("[Minecraft] Recieved empty/none, skipping: {:#?}", t), 2),
                }
            }
        }
    }
}

fn store_level_chunk(packet_data: &ClientboundLevelChunkWithLightPacket, mc_client: &Client) {
    // Parse packet
    let ClientboundLevelChunkWithLightPacket {x: chunk_x_pos, z: chunk_z_pos, chunk_data: chunk_packet_data, light_data: light_packet_data} = packet_data;
    let ClientboundLevelChunkPacketData { heightmaps: chunk_heightmaps, data: chunk_data, block_entities: chunk_entities } = chunk_packet_data;
    utils::logger(&format!("[Minecraft] Server sent chunk x/z {}/{}", chunk_x_pos, chunk_z_pos), 1);
    let chunk_location: ChunkPos = ChunkPos { x: *chunk_x_pos, z: *chunk_z_pos };
    // send chunk over
    // TODO: this is terribly slow. it iterates the chunk, pushing each node one-at-a-time.
    
}

async fn send_all_chunks(mt_conn: &MinetestConnection, mc_client: &Client) {
    utils::logger("Skipping sending new chunk: NOT IMPLEMENTED", 3);
}
