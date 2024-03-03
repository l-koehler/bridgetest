// unsurprisingly has absolutely nothing to do with translator.rs
// i am terrible at naming things
// anyways this contains functions that TAKE data from the minecraft server
// and send it to the minetest client.

extern crate alloc;

use crate::settings;
use crate::utils;
use crate::mt_definitions;
use crate::MTServerState;
use azalea::BlockPos;
use mt_definitions::{HeartDisplay, FoodDisplay, Dimensions};

use azalea_registry::Registry;
use minetest_protocol::wire::command::ToClientCommand;
use minetest_protocol::wire::types::HudStat;
use minetest_protocol::MinetestConnection;
use minetest_protocol::wire;
use minetest_protocol::wire::types::{v3s16, v3f, MapNodesBulk, MapNode, MapBlock, NodeMetadataList};

use azalea_client::PlayerInfo;
use azalea_client::chat::ChatPacket;

use tokio::sync::mpsc::UnboundedReceiver;
use azalea_client::Event;
use azalea_protocol::packets::game::{ClientboundGamePacket,
    clientbound_player_position_packet::ClientboundPlayerPositionPacket,
    clientbound_set_time_packet::ClientboundSetTimePacket,
    clientbound_set_health_packet::ClientboundSetHealthPacket,
    clientbound_set_default_spawn_position_packet::ClientboundSetDefaultSpawnPositionPacket,
    clientbound_respawn_packet::ClientboundRespawnPacket,
};
use azalea_protocol::packets::common::CommonPlayerSpawnInfo;
use azalea_core::resource_location::ResourceLocation;
use azalea_protocol::packets::game::clientbound_level_chunk_with_light_packet::{ClientboundLevelChunkWithLightPacket, ClientboundLevelChunkPacketData};
use azalea_protocol::packets::game::clientbound_system_chat_packet::ClientboundSystemChatPacket;
use std::sync::Arc;
use std::io::Cursor;
use azalea_world::chunk_storage;
use azalea_block::BlockState;

pub async fn update_dimension(source_packet: &ClientboundRespawnPacket, mt_server_state: &mut MTServerState) {
    let ClientboundRespawnPacket { common: player_spawn_info, data_to_keep: _ } = source_packet;
    let CommonPlayerSpawnInfo {
        dimension_type: _,
        dimension,
        seed: _,
        game_type: _,
        previous_game_type: _,
        is_debug: _,
        is_flat: _,
        last_death_location: _,
        portal_cooldown: _
    } = player_spawn_info;
    let ResourceLocation { namespace, path } = dimension;
    if namespace != "minecraft" {
        mt_server_state.current_dimension = Dimensions::Custom;
    } else {
        mt_server_state.current_dimension = match path.as_str() {
            "overworld" => Dimensions::Overworld,
            "the_nether" => Dimensions::Nether,
            "the_end" => Dimensions::End,
            _ => Dimensions::Custom
        };
    }
    utils::logger(&format!("[Minetest] New Dimension: {}:{}", namespace, path), 1)
}

pub async fn set_spawn(source_packet: &ClientboundSetDefaultSpawnPositionPacket, mt_server_state: &mut MTServerState) {
    let ClientboundSetDefaultSpawnPositionPacket { pos, angle: _ } = source_packet;
    let BlockPos {x, y, z} = pos;
    let dest_x = *x as f32;
    let dest_y = *y as f32;
    let dest_z = *z as f32;
    mt_server_state.respawn_pos = (dest_x, dest_y, dest_z);
}

pub async fn death(conn: &MinetestConnection, mt_server_state: &mut MTServerState) {
    let respawn_pos = mt_server_state.respawn_pos;
    let setpos_packet = ToClientCommand::MovePlayer(
        Box::new(wire::command::MovePlayerSpec {
            pos: v3f {x: respawn_pos.0, y: respawn_pos.1, z: respawn_pos.2},
            pitch: 0.0,
            yaw: 0.0
        })
    );
    let _ = conn.send(setpos_packet).await;
/*
    let deathscreen = ToClientCommand::Deathscreen(
        Box::new(wire::command::DeathscreenSpec {
            set_camera_point_target: true,
            camera_point_target: v3f {
                x: respawn_pos.0,
                y: respawn_pos.1,
                z: respawn_pos.2
            }
        })
    );

    let _ = conn.send(deathscreen).await;*/

    set_health(&ClientboundSetHealthPacket { health: 20.0, food: 20, saturation: 0.0 }, conn, mt_server_state).await;
}

pub async fn edit_healthbar(mode: HeartDisplay, num: u32, conn: &MinetestConnection) {
    // num is from 0 to 20
    // above 20: no change will be made to the number of hearts
    let hearth_texture: &str = match mode {
        HeartDisplay::Absorb => "heart-absorbing_full.png",
        HeartDisplay::Frozen => "heart-frozen_full.png",
        HeartDisplay::Normal => "heart-full.png",
        HeartDisplay::Poison => "heart-poisoned_full.png",
        HeartDisplay::Wither => "heart-withered_full.png",
        HeartDisplay::HardcoreAbsorb => "heart-absorbing_hardcore_full.png",
        HeartDisplay::HardcoreFrozen => "heart-frozen_hardcore_full.png",
        HeartDisplay::HardcoreNormal => "heart-hardcore_full.png",
        HeartDisplay::HardcorePoison => "heart-poisoned_hardcore_full.png",
        HeartDisplay::HardcoreWither => "heart-withered_hardcore_full.png",
        HeartDisplay::Vehicle => "heart-vehicle_full.png",
        HeartDisplay::NoChange => ""
    };
    if hearth_texture != "" {
        let set_bar_texture = ToClientCommand::Hudchange(
            Box::new(wire::command::HudchangeSpec {
                server_id: settings::HEALTHBAR_ID,
                stat: HudStat::Text(String::from(hearth_texture))
            })
        );
        let _ = conn.send(set_bar_texture);
    }
    if num < 21 {
        let set_bar_number = ToClientCommand::Hudchange(
            Box::new(wire::command::HudchangeSpec {
                server_id: settings::HEALTHBAR_ID,
                stat: HudStat::Number(num)
            })
        );
        let _ = conn.send(set_bar_number).await;
    }
}

pub async fn edit_foodbar(mode: FoodDisplay, num: u32, conn: &MinetestConnection) {
    let food_texture: &str = match mode {
        FoodDisplay::Normal => "hud-food_full.png",
        FoodDisplay::Hunger => "hud-food_full_hunger.png",
        FoodDisplay::NoChange => ""
    };
    if food_texture != "" {
        let set_bar_texture = ToClientCommand::Hudchange(
            Box::new(wire::command::HudchangeSpec {
                server_id: settings::FOODBAR_ID,
                stat: HudStat::Text(String::from(food_texture))
            })
        );
        let _ = conn.send(set_bar_texture);
    }
    if num < 21 {
        let set_bar_number = ToClientCommand::Hudchange(
            Box::new(wire::command::HudchangeSpec {
                server_id: settings::FOODBAR_ID,
                stat: HudStat::Number(num)
            })
        );
        let _ = conn.send(set_bar_number).await;
    }
}

pub async fn edit_airbar(num: u32, conn: &MinetestConnection) {
    // num 0..20, bar invisible if air is full.
    let mut bubble_count: u32 = num;
    if num > 19 {
        bubble_count = 0;
    }

    let set_bar_number = ToClientCommand::Hudchange(
        Box::new(wire::command::HudchangeSpec {
            server_id: settings::AIRBAR_ID,
            stat: HudStat::Number(bubble_count)
        })
    );
    let _ = conn.send(set_bar_number).await;
}

pub async fn set_health(source_packet: &ClientboundSetHealthPacket, conn: &MinetestConnection, mt_server_state: &mut MTServerState) {
    let ClientboundSetHealthPacket { health, food, saturation:_ } = source_packet;
    // health: 0..20
    let new_health: u16 = *health as u16;
    let mut damage_effect: Option<bool> = None;
    if mt_server_state.mt_last_known_health > new_health {
        // health dropped since last time this was run
        damage_effect = Some(true);
    }
    mt_server_state.mt_last_known_health = new_health;

    let sethealth_packet = ToClientCommand::Hp(
        Box::new(wire::command::HpSpec {
            hp: new_health,
            damage_effect,
        })
    );
    let _ = conn.send(sethealth_packet).await;
    edit_healthbar(HeartDisplay::NoChange, new_health.into(), conn).await;
    edit_foodbar(FoodDisplay::NoChange, *food, conn).await;
}

pub async fn set_time(source_packet: &ClientboundSetTimePacket, conn: &MinetestConnection) {
    let ClientboundSetTimePacket { game_time: _, day_time } = source_packet;
    // day_time seems to be the world age in ticks, so mod 24000 is the age of the day
    // age of the day is 0..23999
    // where 0 is 06:00, 6000 is 12:00, 12000 is 18:00, 18000 is 24:00 and 23999 is 05:59

    let mt_time: u16 = (*day_time % 24000) as u16;
    utils::logger(&format!("[Minetest] S->C TimeOfDay: {}", mt_time), 0);
    let settime_packet = ToClientCommand::TimeOfDay(
        Box::new(wire::command::TimeOfDaySpec {
            time_of_day: mt_time,
            time_speed: Some(72.0) // time does pass, but we move it forward manually by resending this packet
        })
    );
    let _ = conn.send(settime_packet).await;
}

pub async fn set_player_pos(source_packet: &ClientboundPlayerPositionPacket, conn: &MinetestConnection, mt_server_state: &mut MTServerState) {
    // y_rot: yaw
    // x_rot: pitch
    // source: https://en.wikipedia.org/wiki/Aircraft_principal_axes
    let ClientboundPlayerPositionPacket {x: source_x, y: source_y, z: source_z, y_rot: source_yaw, x_rot: source_pitch, relative_arguments: _, id: _} = source_packet;
    let dest_x = (*source_x as f32) * 10.0;
    let dest_y = (*source_y as f32) * 10.0;
    let dest_z = (*source_z as f32) * 10.0;

    let abs_diff = (dest_x - mt_server_state.mt_clientside_pos.0).abs() +
                   (dest_y - mt_server_state.mt_clientside_pos.1).abs() +
                   (dest_z - mt_server_state.mt_clientside_pos.2).abs();
    
    println!("dest: {}, state: {}, diff: {}", dest_x, mt_server_state.mt_clientside_pos.0, (dest_x - mt_server_state.mt_clientside_pos.0).abs());
    if abs_diff > 0.1 {
        let setpos_packet = ToClientCommand::MovePlayer(
            Box::new(wire::command::MovePlayerSpec {
                pos: v3f {x: dest_x, y: dest_y, z: dest_z},
                pitch: *source_pitch,
                yaw: *source_yaw
            })
        );
        let _ = conn.send(setpos_packet).await;
        mt_server_state.mt_clientside_pos = (dest_x, dest_y, dest_z);
    }
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

pub async fn initialize_16node_chunk(x_pos:i16, y_pos:i16, z_pos:i16, conn: &mut MinetestConnection, state_arr: [BlockState; 4096]) {
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
    utils::logger(&format!("[Minetest] S->C Initializing 16^3 nodes at {}/{}/{}", x_pos, y_pos, z_pos), 0);
    // TODO this does not support actual metadata
    let mut metadata_vec = Vec::new();
    // subcoordinates within the chunk
    for sub_z in 0..15 {
        for sub_y in 0..15 {
            for sub_x in 0..15 {
                metadata_vec.push(mt_definitions::get_metadata_placeholder(sub_x, sub_y, sub_z)) //(x_pos*16+sub_x) as u16, (y_pos*16+sub_y) as u16, (z_pos*16+sub_z) as u16)
            }
        }
    }
    
    let mut nodes: [MapNode; 4096] = [MapNode{ param0: 126, param1: 0, param2: 0 }; 4096];
    let mut state: BlockState;
    let mut param0: u16;
    let mut param1: u8;
    let mut param2: u8;
    for state_arr_i in 0..4095 {
        state = state_arr[state_arr_i];
        param0 = azalea_registry::Block::try_from(state).unwrap().to_u32() as u16 + 128;
        param2 = 0;
        
        // param1: transparency i think
        if state.is_air() {
            param0 = 126;
            param1 = 0xEE;
        } else {
            param1 = 0x00;
        }
        
        nodes[state_arr_i] = MapNode {
            param0,
            param1,
            param2,
        }
    }
    
    let addblockcommand = ToClientCommand::Blockdata(
        Box::new(wire::command::BlockdataSpec {
            pos: v3s16 { x: x_pos, y: y_pos, z: z_pos },
            block: MapBlock {
                 is_underground: (y_pos <= 4), // below 64, likely?
                 day_night_diff: false,
                 generated: false, // server does not tell us that
                 lighting_complete: Some(65535),
                 nodes: MapNodesBulk {
                     nodes
                },
                node_metadata: NodeMetadataList {
                    metadata: vec![]
                }
            },
            network_specific_version: 2 // what does this meeeean qwq
        })
    );
    //println!("{:#?}", addblockcommand);
    //panic!("done here");
    let _ = conn.send(addblockcommand).await;
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

pub async fn chunkbatch(mt_conn: &mut MinetestConnection, mc_conn: &mut UnboundedReceiver<Event>, mt_server_state: &MTServerState) {
    utils::logger("[Minetest] Forwarding ChunkBatch...", 1);
    // called by a ChunkBatchStart
    // first let azalea do everything until ChunkBatchFinished,
    // then move the azalea world over to the client
    let y_bounds = mt_definitions::get_y_bounds(&mt_server_state.current_dimension);
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
                                    send_level_chunk(&packet_data, mt_conn, &y_bounds).await;
                                },
                                ClientboundGamePacket::ChunkBatchFinished(_) => {
                                    utils::logger("[Minecraft] S->C ChunkBatchFinished", 1);
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

pub async fn send_level_chunk(packet_data: &ClientboundLevelChunkWithLightPacket, mt_conn: &mut MinetestConnection, y_bounds: &(i16, i16)) {
    // Parse packet
    let ClientboundLevelChunkWithLightPacket {x: chunk_x_pos, z: chunk_z_pos, chunk_data: chunk_packet_data, light_data: _} = packet_data;
    let ClientboundLevelChunkPacketData { heightmaps: chunk_heightmaps, data: chunk_data, block_entities: _ } = chunk_packet_data;
    utils::logger(&format!("[Minecraft] Server sent chunk x/z {}/{}", chunk_x_pos, chunk_z_pos), 0);
    //let chunk_location: ChunkPos = ChunkPos { x: *chunk_x_pos, z: *chunk_z_pos }; // unused
    // send chunk to the MT client
    let mut nodearr: [BlockState; 4096] = [BlockState{id:125};4096];
    // for each y level (mc chunks go from top to bottom, while mt chunks are 16 nodes high)
    let mut chunk_data_cursor = Cursor::new(chunk_data.as_slice());
    let dimension_height = i16::abs_diff(y_bounds.0, y_bounds.1).into();
    let mc_chunk: chunk_storage::Chunk = chunk_storage::Chunk::read_with_dimension_height(&mut chunk_data_cursor, dimension_height, y_bounds.0.into(), chunk_heightmaps)
    .expect("Failed to parse chunk!");
    let chunk_storage::Chunk { sections, heightmaps: _ } = &mc_chunk; // heightmaps get ignored, these are just chunk_heightmaps
    
    let mut current_state: BlockState;
    /*
     * Default (engine-reserved) Nodes according to src/mapnode.h
     * 125: Unknown (A solid walkable node with the texture unknown_node.png.)
     * 126: Air (The common material through which the player can walk and which is transparent to light)
     * 127: Ignored (The stuff unloaded chunks are considered to consist of)
     */

    let mut chunk_y_pos = y_bounds.0/16;
    for section in sections { // foreach possible section height (-4 .. 20)
        // for each block in the 16^3 chunk
        for z in 0..16 {
            for y in 0..16 {
                for x in 0..16 {
                    current_state = section.get(azalea_core::position::ChunkSectionBlockPos { x: x as u8, y: y as u8, z: z as u8});
                    // index ranges from 0 (0/0/0) to 4095 (15/15/15), as described in initialize_16node_chunk()
                    nodearr[x+(y*16)+(z*256)] = current_state;
                }
            }
        }
        initialize_16node_chunk(*chunk_x_pos as i16, chunk_y_pos, *chunk_z_pos as i16, mt_conn, nodearr).await;
        chunk_y_pos += 1;
    }
}
