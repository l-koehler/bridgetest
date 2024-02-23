// unsurprisingly has absolutely nothing to do with translator.rs
// i am terrible at naming things
// anyways this contains functions that TAKE data from the minecraft server
// and send it to the minetest client.

extern crate alloc;

use crate::utils;
use crate::MTServerState;

use minetest_protocol::wire::command::ToClientCommand;
use minetest_protocol::MinetestConnection;
use minetest_protocol::wire;

use azalea_client::PlayerInfo;
use azalea_client::Client;

use tokio::sync::mpsc::UnboundedReceiver;
use azalea_client::Event;
use azalea_protocol::packets::game::ClientboundGamePacket;
use azalea_protocol::packets::game::clientbound_level_chunk_with_light_packet::{ClientboundLevelChunkWithLightPacket, ClientboundLevelChunkPacketData};
use std::sync::Arc;
use azalea_core::position::ChunkPos;

use std::io::Cursor;

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
    // // Update world
    // utils::logger("Skipping World Update: NOT IMPLEMENTED", 3);
    // let world = mc_client.partial_world();
    // let world_full = mc_client.world();
    // let mut chunkstorage = &mut world_full.read().chunks;
    // world.read().chunks.replace_with_packet_data(&chunk_location, &mut Cursor::new(&chunk_data), chunk_heightmaps, chunkstorage);
}

async fn send_all_chunks(mt_conn: &MinetestConnection, mt_client: &Client) {
    utils::logger("Skipping sending new chunk: NOT IMPLEMENTED", 3);
    
}

