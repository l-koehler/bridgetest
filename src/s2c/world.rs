use crate::state;
use crate::utils;
use state::world::Dimensions;

use azalea::BlockPos;
use azalea::core::position::ChunkSectionBlockPos;
use core::slice::SlicePattern;
use log::*;

use glam::I16Vec3 as v3i16;
use luanti_core::ContentId;
use luanti_core::MapNode;
use luanti_protocol::LuantiConnection;
use luanti_protocol::commands::server_to_client;
use luanti_protocol::commands::server_to_client::ToClientCommand;
use luanti_protocol::types::{MapNodesBulk, NodeMetadataList, TransferrableMapBlock};

use azalea_client::Client;

use azalea::protocol::packets::game::{ClientboundGamePacket, c_section_blocks_update::*};
use azalea_client::Event;
use tokio::sync::mpsc::UnboundedReceiver;

use azalea::protocol::packets::game::{
    c_block_update::ClientboundBlockUpdate,
    c_level_chunk_with_light::{ClientboundLevelChunkPacketData, ClientboundLevelChunkWithLight},
    c_set_time::ClientboundSetTime,
};
use azalea::world::chunk_storage;
use azalea_block::BlockState;
use std::io::Cursor;
use std::sync::Arc;

pub async fn initialize_16node_chunk(
    x_pos: i16,
    y_pos: i16,
    z_pos: i16,
    conn: &LuantiConnection,
    state_arr: [BlockState; 4096],
    cave_air_glow: bool,
) {
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
    trace!(
        "Sending S2C Blockdata (16^3 nodes at {}|{}|{})",
        x_pos, y_pos, z_pos
    );

    let mut nodes: [MapNode; 4096] = [MapNode {
        content_id: ContentId::AIR,
        param1: 0,
        param2: 0,
    }; 4096];
    let mut state: BlockState;
    for state_arr_i in 0..4095 {
        state = state_arr[state_arr_i];
        nodes[state_arr_i] = utils::state_to_node(state, cave_air_glow)
    }

    let addblockcommand = ToClientCommand::Blockdata(Box::new(server_to_client::BlockdataSpec {
        pos: v3i16 {
            x: x_pos,
            y: y_pos,
            z: z_pos,
        },
        block: TransferrableMapBlock {
            is_underground: (y_pos <= 4), // below 64, likely?
            day_night_differs: false,
            generated: false, // server does not tell us that
            lighting_complete: Some(u16::MAX),
            nodes: MapNodesBulk { nodes },
            node_metadata: NodeMetadataList { metadata: vec![] },
        },
        network_specific_version: 2, // what does this meeeean qwq
    }));
    conn.send(addblockcommand).unwrap();
}

pub async fn chunkbatch(
    luanti_conn: &mut LuantiConnection,
    mc_conn: &mut UnboundedReceiver<Event>,
    player_state: &mut state::PlayerState,
) {
    debug!("Forwarding S2C ChunkBatch");
    loop {
        tokio::select! {
            t = mc_conn.recv() => {
                match t {
                    Some(_) => {
                        let mc_command = t.expect("[Minecraft] Failed to unwrap non-empty packet from Server!");
                        utils::show_mc_command(&mc_command);
                        if let Event::Packet(packet_value) = mc_command {
                            match Arc::unwrap_or_clone(packet_value) {
                                ClientboundGamePacket::LevelChunkWithLight(packet_data) => {
                                    trace!("Forwarding S2C LevelchunkWithLight");
                                    send_level_chunk(&packet_data, luanti_conn, player_state).await;
                                },
                                ClientboundGamePacket::ChunkBatchFinished(_) => {
                                    debug!("Got S2C ChunkBatchFinished");
                                    return; // Done
                                },
                                _ => warn!("Got unexpected S2C packet during ChunkBatch"),
                            }
                        }
                    },
                    None => trace!("Received empty packet, skipping: {:#?}", t),
                }
            }
        }
    }
}

pub async fn send_level_chunk(
    packet_data: &ClientboundLevelChunkWithLight,
    luanti_conn: &mut LuantiConnection,
    player_state: &mut state::PlayerState,
) {
    let y_bounds = player_state.current_dimension.get_y_bounds();
    let is_nether = matches!(player_state.current_dimension, Dimensions::Nether);
    // Parse packet
    let ClientboundLevelChunkWithLight {
        x: chunk_x_pos,
        z: chunk_z_pos,
        chunk_data: chunk_packet_data,
        light_data: _,
    } = packet_data;
    let ClientboundLevelChunkPacketData {
        heightmaps: chunk_heightmaps,
        data: chunk_data,
        block_entities: _,
    } = chunk_packet_data;

    //let chunk_location: ChunkPos = ChunkPos { x: *chunk_x_pos, z: *chunk_z_pos }; // unused
    // send chunk to the MT client
    let mut nodearr: [BlockState; 4096] = [BlockState { id: 125 }; 4096];
    // for each y level (mc chunks go from top to bottom, while mt chunks are 16 nodes high)
    let mut chunk_data_cursor = Cursor::new(chunk_data.as_slice());
    let dimension_height: u16 = i16::abs_diff(y_bounds.0, y_bounds.1);
    let mc_chunk: chunk_storage::Chunk = chunk_storage::Chunk::read_with_dimension_height(
        &mut chunk_data_cursor,
        dimension_height.into(),
        y_bounds.0.into(),
        chunk_heightmaps,
    )
    .expect("Failed to parse chunk!");
    let chunk_storage::Chunk {
        sections,
        heightmaps: _,
    } = &mc_chunk; // heightmaps get ignored, these are just chunk_heightmaps

    let mut current_state: BlockState;
    /*
     * Default (engine-reserved) Nodes according to src/mapnode.h
     * 125: Unknown (A solid walkable node with the texture unknown_node.png.)
     * 126: Air (The common material through which the player can walk and which is transparent to light)
     * 127: Ignored (The stuff unloaded chunks are considered to consist of)
     */

    let mut chunk_y_pos = y_bounds.0 / 16;
    for section in sections {
        // foreach possible section height (-4 .. 20)
        // for each block in the 16^3 chunke
        for z in 0..16 {
            for y in 0..16 {
                for x in 0..16 {
                    current_state = section.get(azalea::core::position::ChunkSectionBlockPos {
                        x: x as u8,
                        y: y as u8,
                        z: z as u8,
                    });
                    // index ranges from 0 (0/0/0) to 4095 (15/15/15), as described in initialize_16node_chunk()
                    nodearr[x + (y * 16) + (z * 256)] = current_state;
                }
            }
        }
        initialize_16node_chunk(
            *chunk_x_pos as i16,
            chunk_y_pos,
            *chunk_z_pos as i16,
            luanti_conn,
            nodearr,
            is_nether,
        )
        .await;
        chunk_y_pos += 1;
    }
}

pub async fn section_block_update(
    packet: &ClientboundSectionBlocksUpdate,
    conn: &mut LuantiConnection,
    player_state: &state::PlayerState,
    mc_client: &Client,
) {
    let ClientboundSectionBlocksUpdate {
        section_pos,
        states,
    } = packet;
    // the section we need to update is smaller than the entire array
    let mut nodearr: [BlockState; 4096] = [BlockState { id: 125 }; 4096];
    let world_lock = mc_client.world();
    let world = world_lock.read();
    for z in 0..16 {
        for y in 0..16 {
            for x in 0..16 {
                let cs_pos = ChunkSectionBlockPos {
                    x: x as u8,
                    y: y as u8,
                    z: z as u8,
                };
                let state;
                if let Some(bstate) = states.into_iter().find(|i| i.pos == cs_pos) {
                    state = bstate.state;
                } else {
                    let block_pos = BlockPos {
                        x: (section_pos.x * 16) + x as i32,
                        y: (section_pos.y * 16) + y as i32,
                        z: (section_pos.z * 16) + z as i32,
                    };
                    state = world.get_block_state(&block_pos).unwrap();
                }
                nodearr[x + (y * 16) + (z * 256)] = state;
            }
        }
    }
    initialize_16node_chunk(
        section_pos.x as i16,
        section_pos.y as i16,
        section_pos.z as i16,
        conn,
        nodearr,
        player_state.current_dimension == Dimensions::Nether,
    )
    .await;
}

pub async fn set_time(source_packet: &ClientboundSetTime, conn: &LuantiConnection) {
    let ClientboundSetTime {
        game_time: _,
        day_time,
        tick_day_time: _,
    } = source_packet; // likely wrong to ignore tick_day_time FIXME
    // day_time seems to be the world age in ticks, so mod 24000 is the age of the day
    // age of the day is 0..23999
    // where 0 is 06:00, 6000 is 12:00, 12000 is 18:00, 18000 is 24:00 and 23999 is 05:59
    // minecraft uses morning as 0, minetest uses midnight. accounted by -6000

    let mt_time: u16 = (*day_time - 6000 % 24000) as u16;
    trace!(
        "Sending S2C TimeOfDay: {} (server time was {})",
        mt_time, day_time
    );
    let settime_packet = ToClientCommand::TimeOfDay(Box::new(server_to_client::TimeOfDaySpec {
        time_of_day: mt_time,
        time_speed: Some(1.0), // time does pass, but we move it forward manually by resending this packet
    }));
    conn.send(settime_packet).unwrap();
}

// block placement/destruction
pub async fn blockupdate(
    packet_data: &ClientboundBlockUpdate,
    conn: &mut LuantiConnection,
    player_state: &state::PlayerState,
) {
    let ClientboundBlockUpdate { pos, block_state } = packet_data;
    let cave_air_glow = player_state.current_dimension == Dimensions::Nether;
    let BlockPos { x, y, z } = pos;
    let addnodecommand = ToClientCommand::Addnode(Box::new(server_to_client::AddnodeSpec {
        pos: v3i16 {
            x: *x as i16,
            y: *y as i16,
            z: *z as i16,
        },
        node: utils::state_to_node(*block_state, cave_air_glow),
        keep_metadata: false,
    }));
    conn.send(addnodecommand).unwrap();
}
