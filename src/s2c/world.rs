use crate::s2c;
use crate::state;
use crate::utils;
use azalea::world::Chunk;
use state::world::Dimensions;

use azalea::BlockPos;
use azalea::core::position::ChunkSectionBlockPos;
use azalea::registry::builtin::BlockKind;
use core::slice::SlicePattern;
use log::*;

use glam::I16Vec3 as v3i16;
use luanti_core::ContentId;
use luanti_core::MapNode;
use luanti_protocol::LuantiConnection;
use luanti_protocol::commands::server_to_client;
use luanti_protocol::commands::server_to_client::ToClientCommand;
use luanti_protocol::types::{MapNodesBulk, NodeMetadataList, TransferrableMapBlock};

use azalea::Client;

use azalea::protocol::packets::game::c_section_blocks_update::*;

use azalea::block::BlockState;
use azalea::protocol::packets::game::{
    c_block_update::ClientboundBlockUpdate,
    c_level_chunk_with_light::{ClientboundLevelChunkPacketData, ClientboundLevelChunkWithLight},
    c_light_update::{ClientboundLightUpdate, ClientboundLightUpdatePacketData},
    c_set_time::ClientboundSetTime,
};
use azalea::registry::DataRegistry;

use std::io::Cursor;

pub fn build_node_array(
    state_arr: [BlockState; 4096],
    cave_air_glow: bool,
    sky_levels: &[u8; 4096],
    block_levels: &[u8; 4096],
    media_state: &state::MediaState,
) -> [MapNode; 4096] {
    let mut nodes: [MapNode; 4096] = [MapNode {
        content_id: ContentId::AIR,
        param1: 0,
        param2: 0,
    }; 4096];
    let mut state: BlockState;
    for state_arr_i in 0..4096 {
        state = state_arr[state_arr_i];
        let light = (sky_levels[state_arr_i], block_levels[state_arr_i]);
        let node = utils::state_to_node(state, cave_air_glow, light, media_state);
        // minecraft and luanti disagree on x handedness
        // the node order within each x-row has to be reversed
        let x = state_arr_i % 16;
        let y = (state_arr_i / 16) % 16;
        let z = state_arr_i / 256;
        let mirrored_i = (15 - x) + y * 16 + z * 256;
        nodes[mirrored_i] = node
    }
    nodes
}

pub async fn initialize_16node_chunk(
    x_pos: i16,
    y_pos: i16,
    z_pos: i16,
    conn: &LuantiConnection,
    state_arr: [BlockState; 4096],
    cave_air_glow: bool,
    sky_levels: Option<&[u8; 4096]>,
    block_levels: Option<&[u8; 4096]>,
    light_cache: &mut state::LightCache,
    media_state: &state::MediaState,
    particle_spawners: &mut state::ParticleSpawnerState,
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

    // light data gets cached for later block updates
    let (sky, block) = match (sky_levels, block_levels) {
        (Some(sky), Some(block)) => {
            light_cache.store(x_pos, y_pos, z_pos, *sky, *block);
            (*sky, *block)
        }
        _ => light_cache.get_section(x_pos, y_pos, z_pos),
    };

    let nodes = build_node_array(state_arr, cave_air_glow, &sky, &block, media_state);

    let addblockcommand = ToClientCommand::Blockdata(Box::new(server_to_client::BlockdataSpec {
        pos: v3i16 {
            // see above, we also mirror the nodes inside the chunk
            x: utils::mirror_block_pos(x_pos as i32) as i16,
            y: y_pos,
            z: z_pos,
        },
        block: TransferrableMapBlock {
            is_underground: (y_pos <= 4), // below 64, likely?
            day_night_differs: y_pos > 4,
            generated: false, // server does not tell us that
            lighting_complete: Some(u16::MAX),
            nodes: MapNodesBulk { nodes },
            node_metadata: NodeMetadataList { metadata: vec![] },
        },
        network_specific_version: 2, // what does this meeeean qwq
    }));
    conn.send(addblockcommand).unwrap();

    // update spawners
    for state_arr_i in 0..4096 {
        let x = state_arr_i % 16;
        let y = (state_arr_i / 16) % 16;
        let z = state_arr_i / 256;
        let block_pos = BlockPos {
            x: x_pos as i32 * 16 + x as i32,
            y: y_pos as i32 * 16 + y as i32,
            z: z_pos as i32 * 16 + z as i32,
        };
        let kind = BlockKind::from(state_arr[state_arr_i]);
        s2c::particles::sync_block_spawner(block_pos, kind, conn, particle_spawners).await;
    }
}

pub fn chunk_batch_start(batch_state: &mut state::ChunkBatchState) {
    if batch_state.active {
        warn!("Got S2C ChunkBatchStart while already inside a chunk batch");
    }
    debug!("Started S2C ChunkBatch");
    batch_state.active = true;
}

pub fn chunk_batch_finished(batch_state: &mut state::ChunkBatchState) {
    if !batch_state.active {
        warn!("Got S2C ChunkBatchFinished without a matching ChunkBatchStart");
    }
    debug!("Got S2C ChunkBatchFinished");
    batch_state.active = false;
}

struct SectionLight {
    sky: Vec<[u8; 4096]>,
    sky_touched: Vec<bool>,
    block: Vec<[u8; 4096]>,
    block_touched: Vec<bool>,
    // chunk_y_pos of the lowest section
    base_chunk_y: i16,
}

fn decode_section_light(
    y_bounds: (i16, i16),
    light_data: &ClientboundLightUpdatePacketData,
) -> SectionLight {
    let min_y = y_bounds.0 as i32;
    let max_y = y_bounds.1 as i32 + 1; // exclusive
    let num_sections = (((max_y - min_y) / 16) as usize).max(1);

    let (sky, sky_touched) = utils::decode_light_layers(
        &light_data.sky_updates,
        &light_data.sky_y_mask,
        &light_data.empty_sky_y_mask,
        num_sections,
    );
    let (block, block_touched) = utils::decode_light_layers(
        &light_data.block_updates,
        &light_data.block_y_mask,
        &light_data.empty_block_y_mask,
        num_sections,
    );

    // all luanti light data is relative to the aligned origin (min_y floored to a multiple if 16)
    let base_section = -((-min_y) / 16) * 16;
    let base_chunk_y = (base_section / 16) as i16;

    SectionLight {
        sky,
        sky_touched,
        block,
        block_touched,
        base_chunk_y,
    }
}

pub async fn send_level_chunk(
    packet_data: &ClientboundLevelChunkWithLight,
    luanti_conn: &mut LuantiConnection,
    player_state: &mut state::PlayerState,
    light_cache: &mut state::LightCache,
    media_state: &state::MediaState,
    particle_spawners: &mut state::ParticleSpawnerState,
) {
    let y_bounds = player_state.current_dimension.get_y_bounds();
    let is_nether = matches!(player_state.current_dimension, Dimensions::Nether);
    // Parse packet
    let ClientboundLevelChunkWithLight {
        x: chunk_x_pos,
        z: chunk_z_pos,
        chunk_data: chunk_packet_data,
        light_data,
    } = packet_data;
    let ClientboundLevelChunkPacketData {
        heightmaps: chunk_heightmaps,
        data: chunk_data,
        block_entities: _,
    } = chunk_packet_data;

    //let chunk_location: ChunkPos = ChunkPos { x: *chunk_x_pos, z: *chunk_z_pos }; // unused
    // send chunk to the MT client
    let mut nodearr: [BlockState; 4096] = [BlockState::AIR; 4096];
    // for each y level (mc chunks go from top to bottom, while mt chunks are 16 nodes high)
    let mut chunk_data_cursor = Cursor::new(chunk_data.as_slice());
    let dimension_height: u16 = i16::abs_diff(y_bounds.0, y_bounds.1);
    let mc_chunk: Chunk = Chunk::read_with_dimension_height(
        &mut chunk_data_cursor,
        dimension_height.into(),
        y_bounds.0.into(),
        chunk_heightmaps,
    )
    .expect("Failed to parse chunk!");
    let Chunk {
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

    // Decode Minecraft light data into per-section day/block light levels.
    // Sky light goes in Luanti's day bank, block light in the night bank so that
    // darkness at night and in caves works via Luantis getLightBlend()
    let section_light = decode_section_light(y_bounds, light_data);
    let mut chunk_y_pos: i16 = section_light.base_chunk_y;
    let mut section_index = 0usize;
    for section in sections {
        // foreach possible section height (-4 .. 20)
        // for each block in the 16^3 chunke
        for z in 0..16 {
            for y in 0..16 {
                for x in 0..16 {
                    current_state =
                        section
                            .states
                            .get(azalea::core::position::ChunkSectionBlockPos {
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
            Some(&section_light.sky[section_index]),
            Some(&section_light.block[section_index]),
            light_cache,
            media_state,
            particle_spawners,
        )
        .await;
        chunk_y_pos += 1;
        section_index += 1;
    }
}

pub async fn section_block_update(
    packet: &ClientboundSectionBlocksUpdate,
    conn: &mut LuantiConnection,
    player_state: &state::PlayerState,
    mc_client: &Client,
    light_cache: &mut state::LightCache,
    media_state: &state::MediaState,
    particle_spawners: &mut state::ParticleSpawnerState,
) {
    let ClientboundSectionBlocksUpdate {
        section_pos,
        states,
    } = packet;
    // the section we need to update is smaller than the entire array
    let mut nodearr: [BlockState; 4096] = [BlockState::AIR; 4096];
    let world_lock = mc_client.world().unwrap();
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
                    state = world.get_block_state(block_pos).unwrap_or_default();
                }
                nodearr[x + (y * 16) + (z * 256)] = state;
            }
        }
    }
    // this packet has no light data, so send cached values
    initialize_16node_chunk(
        section_pos.x as i16,
        section_pos.y as i16,
        section_pos.z as i16,
        conn,
        nodearr,
        player_state.current_dimension == Dimensions::Nether,
        None,
        None,
        light_cache,
        media_state,
        particle_spawners,
    )
    .await;
}

pub async fn set_time(
    source_packet: &ClientboundSetTime,
    conn: &LuantiConnection,
    time_state: &mut state::TimeState,
) {
    let ClientboundSetTime {
        game_time,
        clock_updates,
    } = source_packet;

    // use overworld world clock specifically (ID 0)
    if let Some((_clock_id, clock)) = clock_updates.iter().find(|(id, _)| id.protocol_id() == 0) {
        time_state.clock_total = clock.total_ticks;
        time_state.anchor_game_time = *game_time;
    }

    let elapsed = game_time.saturating_sub(time_state.anchor_game_time);
    let phase = (time_state.clock_total + elapsed) % 24000;

    // Minecraft: 0 = sunrise, 6000 = noon, 12000 = sunset, 18000 = midnight
    // Luanti   : 0 = midnight, 6000 = sunrise, 12000 = noon, 18000 = sunset
    // -> luanti_tod = mc_phase + 6000 (mod 24000).
    let mt_time: u16 = ((phase + 6000) % 24000) as u16;

    debug!(
        "Sending S2C TimeOfDay: {} (mc_phase {}, game_time {}, {} clock updates)",
        mt_time,
        phase,
        game_time,
        clock_updates.len()
    );
    let settime_packet = ToClientCommand::TimeOfDay(Box::new(server_to_client::TimeOfDaySpec {
        time_of_day: mt_time,
        time_speed: Some(72.0),
    }));
    conn.send(settime_packet).unwrap();
}

// block placement/destruction
pub async fn blockupdate(
    packet_data: &ClientboundBlockUpdate,
    conn: &mut LuantiConnection,
    player_state: &state::PlayerState,
    light_cache: &state::LightCache,
    media_state: &state::MediaState,
    particle_spawners: &mut state::ParticleSpawnerState,
) {
    let ClientboundBlockUpdate { pos, block_state } = packet_data;
    let cave_air_glow = player_state.current_dimension == Dimensions::Nether;
    let BlockPos { x, y, z } = pos;
    // like section_block_update, reuse cached
    let light = light_cache.get_node(*x, *y, *z);
    let addnodecommand = ToClientCommand::Addnode(Box::new(server_to_client::AddnodeSpec {
        pos: v3i16 {
            x: utils::mirror_block_pos(*x) as i16,
            y: *y as i16,
            z: *z as i16,
        },
        node: utils::state_to_node(*block_state, cave_air_glow, light, media_state),
        keep_metadata: false,
    }));
    conn.send(addnodecommand).unwrap();
    s2c::particles::sync_block_spawner(
        *pos,
        BlockKind::from(*block_state),
        conn,
        particle_spawners,
    )
    .await;
}

pub async fn light_update(
    packet: &ClientboundLightUpdate,
    conn: &mut LuantiConnection,
    player_state: &state::PlayerState,
    mc_client: &Client,
    light_cache: &mut state::LightCache,
    media_state: &state::MediaState,
    particle_spawners: &mut state::ParticleSpawnerState,
) {
    let ClientboundLightUpdate {
        x: chunk_x_pos,
        z: chunk_z_pos,
        light_data,
    } = packet;
    let y_bounds = player_state.current_dimension.get_y_bounds();
    let is_nether = player_state.current_dimension == Dimensions::Nether;

    let section_light = decode_section_light(y_bounds, light_data);
    let num_sections = section_light.sky.len();

    // ignore/keep sections missing in both masks
    let mut resolved: Vec<Option<([u8; 4096], [u8; 4096])>> = Vec::with_capacity(num_sections);
    for i in 0..num_sections {
        let touched = section_light.sky_touched[i] || section_light.block_touched[i];
        if !touched {
            resolved.push(None);
            continue;
        }
        let cached = light_cache.get_section(
            *chunk_x_pos as i16,
            section_light.base_chunk_y + i as i16,
            *chunk_z_pos as i16,
        );
        let sky = if section_light.sky_touched[i] {
            section_light.sky[i]
        } else {
            cached.0
        };
        let block = if section_light.block_touched[i] {
            section_light.block[i]
        } else {
            cached.1
        };
        resolved.push(Some((sky, block)));
    }

    // collect block states
    let mut section_nodes: Vec<(usize, [BlockState; 4096])> = Vec::new();
    {
        let world_lock = mc_client.world().unwrap();
        let world = world_lock.read();
        for (section_index, r) in resolved.iter().enumerate() {
            if r.is_none() {
                continue;
            }
            let section_base_y = (section_light.base_chunk_y as i32 + section_index as i32) * 16;
            let mut nodearr: [BlockState; 4096] = [BlockState::AIR; 4096];
            for z in 0..16 {
                for y in 0..16 {
                    for x in 0..16 {
                        let block_pos = BlockPos {
                            x: (*chunk_x_pos * 16) + x as i32,
                            y: section_base_y + y as i32,
                            z: (*chunk_z_pos * 16) + z as i32,
                        };
                        nodearr[x + (y * 16) + (z * 256)] =
                            world.get_block_state(block_pos).unwrap_or_default();
                    }
                }
            }
            section_nodes.push((section_index, nodearr));
        }
    }

    for (section_index, nodearr) in section_nodes {
        let (sky, block) = resolved[section_index].as_ref().unwrap();
        initialize_16node_chunk(
            *chunk_x_pos as i16,
            section_light.base_chunk_y + section_index as i16,
            *chunk_z_pos as i16,
            conn,
            nodearr,
            is_nether,
            Some(sky),
            Some(block),
            light_cache,
            media_state,
            particle_spawners,
        )
        .await;
    }
}
