// unsurprisingly has absolutely nothing to do with translator.rs
// i am terrible at naming things
// anyways this contains functions that TAKE data from the minecraft server
// and send it to the minetest client.

extern crate alloc;

use crate::mt_definitions::EntityResendableData;
use crate::settings;
use crate::utils;
use crate::mt_definitions;
use crate::commands;
use crate::MTServerState;
use azalea::BlockPos;
use azalea_core::delta::PositionDelta8;
use azalea_core::position::ChunkBlockPos;
use azalea_entity::{EntityDataValue, EntityDataItem};
use minetest_protocol::wire::types::ItemStackMetadata;
use minetest_protocol::wire::types::ObjectProperties;
use mt_definitions::{HeartDisplay, FoodDisplay, Dimensions};
use minetest_protocol::peer::peer::PeerError;

use azalea_registry::EntityKind;
use minetest_protocol::wire::command::ToClientCommand;
use minetest_protocol::wire::types::HudStat;
use minetest_protocol::MinetestConnection;
use minetest_protocol::wire;
use minetest_protocol::wire::types::{v3s16, v3f, v2f, MapNodesBulk, MapNode, MapBlock, NodeMetadataList, AddedObject, GenericInitData, ActiveObjectCommand, SColor, aabb3f, v2s16, InventoryEntry, InventoryList, ItemStackUpdate, ItemStack };

use azalea_client::{PlayerInfo, Client, inventory};
use azalea_client::chat::ChatPacket;

use tokio::sync::mpsc::UnboundedReceiver;
use azalea_client::Event;
use azalea_protocol::packets::game::{ClientboundGamePacket,
    clientbound_player_position_packet::ClientboundPlayerPositionPacket,
    clientbound_set_time_packet::ClientboundSetTimePacket,
    clientbound_set_health_packet::ClientboundSetHealthPacket,
    clientbound_set_default_spawn_position_packet::ClientboundSetDefaultSpawnPositionPacket,
    clientbound_respawn_packet::ClientboundRespawnPacket,
    clientbound_add_entity_packet::ClientboundAddEntityPacket,
    clientbound_move_entity_pos_packet::ClientboundMoveEntityPosPacket,
    clientbound_teleport_entity_packet::ClientboundTeleportEntityPacket,
    clientbound_move_entity_pos_rot_packet::ClientboundMoveEntityPosRotPacket,
    clientbound_move_entity_rot_packet::ClientboundMoveEntityRotPacket,
    clientbound_remove_entities_packet::ClientboundRemoveEntitiesPacket,
    clientbound_set_entity_motion_packet::ClientboundSetEntityMotionPacket,
    clientbound_rotate_head_packet::ClientboundRotateHeadPacket,
    clientbound_block_update_packet::ClientboundBlockUpdatePacket,
    clientbound_entity_event_packet::ClientboundEntityEventPacket,
    clientbound_set_entity_data_packet::ClientboundSetEntityDataPacket,
    clientbound_block_destruction_packet::ClientboundBlockDestructionPacket,
    clientbound_block_entity_data_packet::ClientboundBlockEntityDataPacket,
    clientbound_open_screen_packet::ClientboundOpenScreenPacket,
    clientbound_sound_packet::ClientboundSoundPacket,
};

use azalea_protocol::packets::common::CommonPlayerSpawnInfo;
use azalea_core::resource_location::ResourceLocation;
use azalea_protocol::packets::game::clientbound_level_chunk_with_light_packet::{ClientboundLevelChunkWithLightPacket, ClientboundLevelChunkPacketData};
use azalea_protocol::packets::game::clientbound_system_chat_packet::ClientboundSystemChatPacket;
use std::sync::Arc;
use std::io::Cursor;
use azalea_world::chunk_storage;
use azalea_block::BlockState;
use std::time::Instant;

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
    // FIXME: entirely broken, no clue why.
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
                y: respawn_pos.1,MinecraftEntityId
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
    if !hearth_texture.is_empty() {
        let set_bar_texture = ToClientCommand::Hudchange(
            Box::new(wire::command::HudchangeSpec {
                server_id: settings::HEALTHBAR_ID,
                stat: HudStat::Text(String::from(hearth_texture))
            })
        );
        let _ = conn.send(set_bar_texture).await;
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
    if !food_texture.is_empty() {
        let set_bar_texture = ToClientCommand::Hudchange(
            Box::new(wire::command::HudchangeSpec {
                server_id: settings::FOODBAR_ID,
                stat: HudStat::Text(String::from(food_texture))
            })
        );
        let _ = conn.send(set_bar_texture).await;
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
    // if the bar is full, don't display it
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
    let ClientboundPlayerPositionPacket {
        x, y, z, y_rot, x_rot, relative_arguments: _, id: _
    } = source_packet;
    let dest_x = (*x as f32) * 10.0;
    let dest_y = (*y as f32) * 10.0;
    let dest_z = (*z as f32) * 10.0;
    let setpos_packet = ToClientCommand::MovePlayer(
        Box::new(wire::command::MovePlayerSpec {
            pos: v3f {x: dest_x, y: dest_y, z: dest_z},
            pitch: *x_rot, yaw: *y_rot,
        })
    );
    let _ = conn.send(setpos_packet).await;
    mt_server_state.mt_clientside_pos = (dest_x, dest_y, dest_z);
    mt_server_state.client_rotation = (*y_rot, *x_rot);
}

pub async fn sync_client_pos(mc_client: &Client, conn: &mut MinetestConnection, mt_server_state: &mut MTServerState) {
    let vec_serverpos = mc_client.position();
    let serverpos = (vec_serverpos.x as f32, vec_serverpos.y as f32, vec_serverpos.z as f32);
    let clientpos = mt_server_state.mt_clientside_pos;
    // we count height as half, otherwise jumping is noticeably broken
    let x_y_euclid_diff: f32 = {
        ((serverpos.0 - clientpos.0).abs().powi(2) +
        (serverpos.2 - clientpos.2).abs().powi(2)).sqrt()
    };
    let distance = {
        (x_y_euclid_diff.powi(2) +
        ((serverpos.1 - clientpos.1).abs()/2.0).powi(2)).sqrt()
    };

    if distance > settings::POS_DIFF_TOLERANCE {
        utils::logger(&format!("[Minetest] Re-Syncing Player Position: {} difference.", distance), 1);
        let setpos_packet = ToClientCommand::MovePlayer(
            Box::new(wire::command::MovePlayerSpec {
                pos: v3f { x: serverpos.0*10.0, y: serverpos.1*10.0, z: serverpos.2*10.0 },
                pitch: mt_server_state.client_rotation.1,
                yaw: mt_server_state.client_rotation.0
            })
        );
        let _ = conn.send(setpos_packet).await;
        mt_server_state.mt_clientside_pos = serverpos;
    }
}

pub async fn update_inventory(conn: &mut MinetestConnection, to_change: Vec<(&str, Vec<inventory::ItemSlot>)>) {
    let mut entries: Vec<InventoryEntry> = vec![];
    let mut changed_fields: Vec<&str> = vec![];
    for field in to_change {
        changed_fields.push(field.0);
        let mut field_items: Vec<ItemStackUpdate> = vec![];
        for item in field.1 {
            match item {
                inventory::ItemSlot::Present(ref slot_data) => {
                    field_items.push(ItemStackUpdate::Item(
                        ItemStack {
                            name: slot_data.kind.to_string(),
                            count: slot_data.count as u16,
                            wear: 0,
                            metadata: ItemStackMetadata {
                                string_vars: vec![]
                            }
                        }
                    ));
                },
                inventory::ItemSlot::Empty => {
                    field_items.push(ItemStackUpdate::Empty)
                }
            }
        };
        entries.push(InventoryEntry::Update {
            0: InventoryList {
                name: String::from(field.0),
                width: 0, // idk what this does
                items: field_items
            }
        });
    }
    // send keep to unchanged fields (not doing that deletes the associated UI element)
    let unchanged_fields: Vec<&str> = settings::ALL_INV_FIELDS.into_iter().filter(|item| !changed_fields.contains(item)).collect();
    for field in unchanged_fields {
        entries.push(InventoryEntry::KeepList(String::from(field)))
    }
    let update_inventory_packet = ToClientCommand::Inventory(
        Box::new(wire::command::InventorySpec {
            inventory: wire::types::Inventory {
                entries
            }
        })
    );
    let _ = conn.send(update_inventory_packet).await;
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
    if let azalea_chat::FormattedText::Text(component) = &message.content {
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
    }
}


pub async fn initialize_16node_chunk(x_pos:i16, y_pos:i16, z_pos:i16, conn: &MinetestConnection, state_arr: [BlockState; 4096], cave_air_glow: bool) {
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
    for state_arr_i in 0..4095 {
        state = state_arr[state_arr_i];        
        nodes[state_arr_i] = utils::state_to_node(state, cave_air_glow)
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

pub async fn chunkbatch(mt_conn: &mut MinetestConnection, mc_conn: &mut UnboundedReceiver<Event>, mt_server_state: &mut MTServerState, mc_client: &mut Client) {
    utils::logger("[Minetest] Forwarding ChunkBatch...", 1);
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
                                    utils::logger("[Minecraft] S->C LevelchunkWithLight", 1);
                                    send_level_chunk(&packet_data, mt_conn, mt_server_state).await;
                                },
                                ClientboundGamePacket::ChunkBatchFinished(_) => {
                                    utils::logger("[Minecraft] S->C ChunkBatchFinished", 1);
                                    return; // Done
                                },
                                _ => (),
                            }
                        }
                    },
                    None => utils::logger(&format!("[Minecraft] Recieved empty/none, skipping: {:#?}", t), 2),
                }
            },
            t = mt_conn.recv() => {
                utils::logger("[Minetest] Got Packet while handling ChunkBatch, processing it. Possibly dropping Chunks.", 2);
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
                commands::mt_auto(mt_command, mt_conn, mc_client, mt_server_state).await;
            }
        }
    }
}

pub async fn send_level_chunk(packet_data: &ClientboundLevelChunkWithLightPacket, mt_conn: &mut MinetestConnection, mt_server_state: &mut MTServerState) {
    let y_bounds = mt_definitions::get_y_bounds(&mt_server_state.current_dimension);
    let is_nether = matches!(mt_server_state.current_dimension, Dimensions::Nether);
    // Parse packet
    let ClientboundLevelChunkWithLightPacket {x: chunk_x_pos, z: chunk_z_pos, chunk_data: chunk_packet_data, light_data: _} = packet_data;
    let ClientboundLevelChunkPacketData { heightmaps: chunk_heightmaps, data: chunk_data, block_entities } = chunk_packet_data;
    utils::logger(&format!("[Minecraft] Server sent chunk x/z {}/{}", chunk_x_pos, chunk_z_pos), 1);
    //let chunk_location: ChunkPos = ChunkPos { x: *chunk_x_pos, z: *chunk_z_pos }; // unused
    // send chunk to the MT client
    let mut nodearr: [BlockState; 4096] = [BlockState{id:125};4096];
    // for each y level (mc chunks go from top to bottom, while mt chunks are 16 nodes high)
    let mut chunk_data_cursor = Cursor::new(chunk_data.as_slice());
    let dimension_height: u16 = i16::abs_diff(y_bounds.0, y_bounds.1);
    let mc_chunk: chunk_storage::Chunk = chunk_storage::Chunk::read_with_dimension_height(&mut chunk_data_cursor, dimension_height.into(), y_bounds.0.into(), chunk_heightmaps)
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
        // for each block in the 16^3 chunke
        for z in 0..16 {
            for y in 0..16 {
                for x in 0..16 {
                    current_state = section.get(azalea_core::position::ChunkSectionBlockPos { x: x as u8, y: y as u8, z: z as u8});
                    // index ranges from 0 (0/0/0) to 4095 (15/15/15), as described in initialize_16node_chunk()
                    nodearr[x+(y*16)+(z*256)] = current_state;
                }
            }
        }
        initialize_16node_chunk(*chunk_x_pos as i16, chunk_y_pos, *chunk_z_pos as i16, mt_conn, nodearr, is_nether).await;
        chunk_y_pos += 1;
    }
    for block_entity in block_entities {
        let chunk_pos = ChunkBlockPos {
            x: block_entity.packed_xz >> 4,
            y: (block_entity.y % dimension_height) as i32, // TODO breaks with neg y
            z: block_entity.packed_xz & 15
        };
        let pos: (i32, i32, i32) = (
            chunk_pos.x as i32 + ((chunk_x_pos*16) as i32),
            chunk_pos.y,
            chunk_pos.z as i32 + ((chunk_z_pos*16) as i32)
        );
        utils::logger(&format!("[Minecraft] Registring Block Entity at {:?}", pos), 1);
        if mt_server_state.container_map.insert(pos, block_entity.kind) != None {
            utils::logger(&format!("[Minecraft] Overwriting Block Entity at {:?}", pos), 2);
        }
    }
}

// if no packet is passed, add the player using data from the server state
pub async fn add_entity(optional_packet: Option<&ClientboundAddEntityPacket>, conn: &mut MinetestConnection, mt_server_state: &mut MTServerState) {
    let is_player: bool;
    let name: String;
    let id: u16;
    let position: v3f;
    let velocity: v3f;
    let mesh: &str;
    let textures: Vec<String>;
    let visual: String;
    let entity_kind: EntityKind;
    match optional_packet {
        Some(packet_data) => {
            // use a network packet
            let ClientboundAddEntityPacket {
                id: serverside_id,
                uuid,
                entity_type, // TODO: textures and models depend on this thing
                position: vec_pos,
                x_rot: _, y_rot: _, y_head_rot: _, data: _, x_vel, y_vel, z_vel } = packet_data;
            is_player = false;
            name = format!("UUID-{}", uuid);
            id = *serverside_id as u16 + 1; // ensure 0 is always "free" for the local player, because the actual ID can't be known
            position = utils::vec3_to_v3f(vec_pos, 0.1);
            velocity = v3f::new(
                *x_vel as f32/400.0,
                *y_vel as f32/400.0,
                *z_vel as f32/400.0
            );
            entity_kind = *entity_type;
            if *entity_type == EntityKind::Item {
                visual = String::from("sprite");
                mesh = "";
                // what item it is can't be known at this time, leave empty so
                // a "texture modifier" sent later will just set the texture
                textures = vec![String::from("")];
            } else {
                visual = String::from("mesh");
                (mesh, textures) = utils::get_entity_model(entity_type);
            }
            
        },
        None => {
            // use the mt_server_state and lucky guesses
            is_player = true;
            visual = String::from("mesh");
            name = mt_server_state.this_player.0.clone();
            id = 0; // ensured to be "free"
            position = v3f{x: 0.0, y: 0.0, z: 0.0}; // player will be moved somewhere else later
            mesh = "entitymodel-villager.b3d"; // TODO
            textures = vec![String::from("entity-player-slim-steve.png")];
            entity_kind = EntityKind::Player;
            velocity = v3f::new(0.0, 0.0, 0.0);
        }
    };
    
    let entitydata = EntityResendableData {
        position,
        rotation: v3f::new(0.0, 0.0, 0.0),
        velocity,
        acceleration: v3f::new(0.0, 0.0, 0.0),
        entity_kind
    };
    let insert_successful = mt_server_state.entity_id_pos_map.insert_checked(id.into(), entitydata);
    if !insert_successful {
        utils::logger(&format!("[Minetest] Failed to insert position for (adjusted) entity ID {}: ID already present, dropping the packet!", id), 2);
        return
    }
    
    let added_object: AddedObject = AddedObject {
        id,
        typ: 101, // idk
        init_data: GenericInitData {
            version: 1, // used a packet sniffer, idk if there are other versions
            name,
            is_player, // possibly a lie, but thats not the clients problem anyways
            id,
            position,
            rotation: v3f{x: 0.0, y: 0.0, z: 0.0},
            hp: 100, // entity deaths handled by server
            messages: vec![
                ActiveObjectCommand::SetProperties(
                    wire::types::AOCSetProperties {
                        newprops: ObjectProperties {
                            version: 4,
                            hp_max: 100,
                            physical: true,
                            _unused: 0,
                            collision_box: aabb3f {
                                min_edge: v3f {
                                    x: -0.5,
                                    y: -0.5,
                                    z: -0.5,
                                },
                                max_edge: v3f {
                                    x: 0.5,
                                    y: 0.5,
                                    z: 0.5,
                                },
                            },
                            selection_box: aabb3f {
                                min_edge: v3f {
                                    x: -0.5,
                                    y: -0.5,
                                    z: -0.5,
                                },
                                max_edge: v3f {
                                    x: 0.5,
                                    y: 0.5,
                                    z: 0.5,
                                },
                            },
                            pointable: false,
                            visual,
                            visual_size: v3f {
                                x: 1.0,
                                y: 1.0,
                                z: 1.0,
                            },
                            textures,
                            spritediv: v2s16 {
                                x: 1,
                                y: 1,
                            },
                            initial_sprite_basepos: v2s16 {
                                x: 0,
                                y: 0,
                            },
                            is_visible: true,
                            makes_footstep_sound: true,
                            automatic_rotate: 0.0,
                            mesh: String::from(mesh), // it didnt have sane defaults qwq
                            colors: vec![
                                SColor {
                                    r: 255,
                                    g: 255,
                                    b: 255,
                                    a: 255,
                                },
                            ],
                            collide_with_objects: false,
                            stepheight: 0.0,
                            automatic_face_movement_dir: false,
                            automatic_face_movement_dir_offset: 0.0,
                            backface_culling: true,
                            nametag: String::from(""), // type_str,
                            nametag_color: SColor {
                                r: 255,
                                g: 255,
                                b: 255,
                                a: 255,
                            },
                            automatic_face_movement_max_rotation_per_sec: 360.0,
                            infotext: String::from(""),
                            wield_item: String::from(""),
                            glow: 0,
                            breath_max: 0,
                            eye_height: 1.625,
                            zoom_fov: 0.0,
                            use_texture_alpha: false,
                            damage_texture_modifier: Some(String::from("^[brighten")),
                            shaded: Some(true),
                            show_on_minimap: Some(false),
                            nametag_bgcolor: None,
                            rotate_selectionbox: Some(false)
                        }
                    },
                ),
                ActiveObjectCommand::SetTextureMod(
                    wire::types::AOCSetTextureMod {
                        modifier: String::from("")
                    }
                ),
                ActiveObjectCommand::SetAnimation(
                    wire::types::AOCSetAnimation {
                        range: v2f { x: 0.0, y: 0.0 },
                        speed: 0.0,
                        blend: 0.0,
                        no_loop: false
                    }
                ),
                ActiveObjectCommand::UpdateArmorGroups(
                    wire::types::AOCUpdateArmorGroups {
                        ratings: vec![
                            (String::from("immortal"), 1)
                        ]
                    }
                ),
                ActiveObjectCommand::AttachTo(
                    wire::types::AOCAttachTo {
                        parent_id: 0,
                        bone: String::from(""),
                        position: v3f { x: 0.0, y: 0.0, z: 0.0 },
                        rotation: v3f { x: 0.0, y: 0.0, z: 0.0 },
                        force_visible: false
                    }
                )
            ]
        }
    };
    
    let clientbound_addentity = ToClientCommand::ActiveObjectRemoveAdd(
        Box::new(wire::command::ActiveObjectRemoveAddSpec {
            removed_object_ids: vec![],
            added_objects: vec![added_object],
        })
    );
    let _ = conn.send(clientbound_addentity).await;
}

pub async fn remove_entity(packet_data: &ClientboundRemoveEntitiesPacket, conn: &mut MinetestConnection, mt_server_state: &mut MTServerState) {
    let ClientboundRemoveEntitiesPacket { entity_ids } = packet_data;
    let mut adjusted_id: u16;
    let mut entity_ids_adjusted: Vec<u16> = vec![];
    for entity_id in entity_ids {
        adjusted_id = *entity_id as u16 + 1;
        if mt_server_state.entity_id_pos_map.contains_key(adjusted_id.into()) {
            mt_server_state.entity_id_pos_map.remove(adjusted_id.into());
            entity_ids_adjusted.push(adjusted_id);
        } else {
            utils::logger(&format!("[Minetest] Failed to remove entity with (adjusted) entity ID {}: ID not present. Skipping its removal!", adjusted_id), 2);
        }
    }
    if !entity_ids_adjusted.is_empty() {
        let clientbound_removeentity = ToClientCommand::ActiveObjectRemoveAdd(
            Box::new(wire::command::ActiveObjectRemoveAddSpec {
                removed_object_ids: entity_ids_adjusted,
                added_objects: vec![],
            })
        );
        let _ = conn.send(clientbound_removeentity).await;
    } else {
        utils::logger("[Minetest] Skipping RemoveEntitiesPacket, no entities to remove!", 2);
    }
}

pub async fn entity_setpos(packet_data: &ClientboundMoveEntityPosPacket, conn: &mut MinetestConnection, mt_server_state: &mut MTServerState) {
    let ClientboundMoveEntityPosPacket { entity_id, delta, on_ground: _ } = packet_data;
    let PositionDelta8 {xa, ya, za} = *delta;
    // delta: offset from the current position

    // force conversion u32->u16->u64, because the overflow behavior will differ with u32->u64
    let adjusted_id = *entity_id as u16 + 1;
    if !mt_server_state.entity_id_pos_map.contains_key(adjusted_id.into()) {
        utils::logger(&format!("[Minetest] Failed to update data for (adjusted) entity ID {}: ID not yet present, dropping the packet!", adjusted_id), 2);
        return
    }
    let entitydata = mt_server_state.entity_id_pos_map.get_mut(adjusted_id.into()).unwrap();
    let EntityResendableData {
        position: old_position,
        rotation,
        velocity,
        acceleration,
        entity_kind
    } = entitydata.clone();

    let position = v3f {
        x: old_position.x + xa as f32/4096.0,
        y: old_position.y + ya as f32/4096.0,
        z: old_position.z + za as f32/4096.0
    };
    *entitydata = EntityResendableData {
        position, rotation,
        velocity,
        acceleration, entity_kind
    };
    send_entity_data(adjusted_id, entitydata, conn).await;
}

pub async fn entity_teleport(packet_data: &ClientboundTeleportEntityPacket, conn: &mut MinetestConnection, mt_server_state: &mut MTServerState) {
    let ClientboundTeleportEntityPacket { id: entity_id, position, y_rot, x_rot, on_ground: _ } = packet_data;
    let adjusted_id = *entity_id as u16 + 1;
    if !mt_server_state.entity_id_pos_map.contains_key(adjusted_id.into()) {
        utils::logger(&format!("[Minetest] Failed to update data for (adjusted) entity ID {}: ID not yet present, dropping the packet!", adjusted_id), 2);
        return
    }
    let entitydata = mt_server_state.entity_id_pos_map.get_mut(adjusted_id.into()).unwrap();
    let EntityResendableData {
        position: _,
        rotation: old_rotation,
        velocity,
        acceleration,
        entity_kind
    } = entitydata.clone();
    *entitydata = EntityResendableData {
        position: utils::vec3_to_v3f(position, 0.1),
        rotation: v3f { x: *x_rot as f32, y: *y_rot as f32, z: old_rotation.z },
        velocity, acceleration, entity_kind
    };
    send_entity_data(adjusted_id, entitydata, conn).await;
}

pub async fn entity_setposrot(packet_data: &ClientboundMoveEntityPosRotPacket, conn: &mut MinetestConnection, mt_server_state: &mut MTServerState) {
    let ClientboundMoveEntityPosRotPacket { entity_id, delta, y_rot, x_rot, on_ground: _ } = packet_data;
    let PositionDelta8 {xa, ya, za} = *delta;
    let adjusted_id = *entity_id as u16 + 1;
    if !mt_server_state.entity_id_pos_map.contains_key(adjusted_id.into()) {
        utils::logger(&format!("[Minetest] Failed to update data for (adjusted) entity ID {}: ID not yet present, dropping the packet!", adjusted_id), 2);
        return
    }
    let entitydata = mt_server_state.entity_id_pos_map.get_mut(adjusted_id.into()).unwrap();
    let EntityResendableData {
        position: old_position,
        rotation: old_rotation,
        velocity,
        acceleration,
        entity_kind,
    } = entitydata.clone();

    let position = v3f {
        x: old_position.x + xa as f32/4096.0,
        y: old_position.y + ya as f32/4096.0,
        z: old_position.z + za as f32/4096.0
    };
    if entity_kind != EntityKind::Player {
        println!("Moving Chicken to (p+r): {:?}", position);
    }
    *entitydata = EntityResendableData {
        position,
        rotation: v3f { x: *x_rot as f32, y: *y_rot as f32, z: old_rotation.z },
        velocity,
        acceleration,
        entity_kind
    };
    send_entity_data(adjusted_id, entitydata, conn).await;
}

pub async fn entity_setrot(packet_data: &ClientboundMoveEntityRotPacket, conn: &mut MinetestConnection, mt_server_state: &mut MTServerState) {
    let ClientboundMoveEntityRotPacket { entity_id, y_rot, x_rot, on_ground: _ } = packet_data;
    let adjusted_id = *entity_id as u16 + 1;
    if !mt_server_state.entity_id_pos_map.contains_key(adjusted_id.into()) {
        utils::logger(&format!("[Minetest] Failed to update data for (adjusted) entity ID {}: ID not yet present, dropping the packet!", adjusted_id), 2);
        return
    }
    let entitydata = mt_server_state.entity_id_pos_map.get_mut(adjusted_id.into()).unwrap();
    let EntityResendableData {
        position,
        rotation: old_rotation,
        velocity,
        acceleration,
        entity_kind
    } = entitydata.clone();

    *entitydata = EntityResendableData {
        rotation: v3f { x: *x_rot as f32, y: *y_rot as f32, z: old_rotation.z },
        position, velocity, acceleration, entity_kind
    };
    send_entity_data(adjusted_id, entitydata, conn).await;
}

pub async fn entity_setmotion(packet_data: &ClientboundSetEntityMotionPacket, conn: &mut MinetestConnection, mt_server_state: &mut MTServerState) {
    let ClientboundSetEntityMotionPacket { id, xa, ya, za } = packet_data;
    let adjusted_id = *id as u16 + 1;
    if !mt_server_state.entity_id_pos_map.contains_key(adjusted_id.into()) {
        utils::logger(&format!("[Minetest] Failed to update data for (adjusted) entity ID {}: ID not yet present, dropping the packet!", adjusted_id), 2);
        return
    }
    let entitydata = mt_server_state.entity_id_pos_map.get_mut(adjusted_id.into()).unwrap();
    let EntityResendableData {
        position,
        rotation,
        velocity: _,
        acceleration,
        entity_kind
    } = entitydata.clone();

    // unit: (1/8000)block/50ms -> block/second
    let delta = v3f {
        x: *xa as f32/400.0,
        y: *ya as f32/400.0,
        z: *za as f32/400.0
    };
    *entitydata = EntityResendableData {
        rotation, position,
        velocity: delta,
        acceleration, entity_kind
    };
    send_entity_data(adjusted_id, entitydata, conn).await;
}

pub async fn entity_rotatehead(packet_data: &ClientboundRotateHeadPacket, conn: &mut MinetestConnection, mt_server_state: &MTServerState) {
    // TODO
}

pub async fn entity_event(packet_data: &ClientboundEntityEventPacket, conn: &mut MinetestConnection, mt_server_state: &MTServerState) {
    return; // TODO finish this fn
    let ClientboundEntityEventPacket { entity_id, event_id } = packet_data;
    let adjusted_id = *entity_id as u16 + 1;
    if !mt_server_state.entity_id_pos_map.contains_key(adjusted_id.into()) {
        utils::logger(&format!("[Minetest] Failed to get entity kind for (adjusted) entity ID {}: ID not yet present, dropping the packet!", adjusted_id), 2);
        return
    }
    let entitydata = mt_server_state.entity_id_pos_map.get_mut(adjusted_id.into()).unwrap();
    let entity_kind = entitydata.entity_kind;
    let bad_id_for_entity = format!("[Minecraft] Got entity event with ID {} referring to a entity of type {}, this event isn't implemented for that entity.", adjusted_id, entity_kind);
    // https://wiki.vg/Entity_statuses
    match event_id {
        0 => (), // Tipped Arrow particles
        1 => {
            match entity_kind {
                EntityKind::Rabbit => (), // Rabbit Jump animation
                EntityKind::SpawnerMinecart => (), // Reset cooldown to 200 ticks
                _ => utils::logger(&bad_id_for_entity, 2)
            }
        }
        3 => {
            match entity_kind {
                EntityKind::Egg => (), // Display "ironcrack" particles at own location
                EntityKind::Snowball => (), // Display "snowballpoof" particles at own location
                _ => () // Death sound & animation
            }
        }
        4 => {
            match entity_kind {
                EntityKind::EvokerFangs => (), // Attack animation and sound
                EntityKind::Hoglin => (), // Attack animation and sound
                EntityKind::IronGolem => (), // Attack animation and sound
                EntityKind::Ravager => (), // Attack animation for 10 ticks
                EntityKind::Zoglin => (), // Attack animation and sound
                _ => utils::logger(&bad_id_for_entity, 2)
            }
        }
        6 => (), // Taming Fail particles (smoke)
        7 => (), // Taming Success particles (heart)
        8 => (), // Wolf shaking water animation
        9 => (), // Item usage finished (e.g. eating done)
        10 => {
            match entity_kind {
                EntityKind::Sheep => (), // Sheep eating grass animation
                EntityKind::TntMinecart => (), // Ignite TntMinecart
                _ => utils::logger(&bad_id_for_entity, 2)
            }
        }
        11 => (), // Iron golem holding flower for 20 seconds animation
        12 => (), // villager mating heart particles
        13 => (), // villager angry particles
        14 => (), // villager happy particles
        15 => (), // spawn 10 to 45 "witchMagic" particles
        16 => (), // play zombieVillagerCure sound
        17 => (), // trigger firework explosion
        18 => (), // spawn heart particles
        19 => (), // reset rotation
        20 => (), // spawn explosion particles
        //TODO finish this (after implementing the particle system)
        _ => utils::logger(&format!("[Minecraft] Got unsupported Entity Event (Event ID: {}, Entity ID: {})", event_id, adjusted_id), 2),
    }
}

pub async fn set_entity_data(packet_data: &ClientboundSetEntityDataPacket, conn: &mut MinetestConnection, mt_server_state: &MTServerState) {
    // Currently, the only data that will actually be used is EntityDataValue::ItemStack in EntityKind::Item
    // Everything else gets dropped.
    let ClientboundSetEntityDataPacket { id, packed_items } = packet_data;
    let adjusted_id = *id as u16 + 1;

    if !mt_server_state.entity_id_pos_map.contains_key(adjusted_id.into()) {
        utils::logger(&format!("[Minetest] Failed to update data for (adjusted) entity ID {}: ID not yet present, dropping the packet!", adjusted_id), 2);
        return
    }
    let entity_kind = mt_server_state.entity_id_pos_map.get(adjusted_id.into()).unwrap().entity_kind;
    
    let mut metadata_item: &EntityDataItem;
    for i in 0..packed_items.len() {
        metadata_item = &packed_items[i];
        let EntityDataItem { index: _, value } = metadata_item;
        match value {
            EntityDataValue::ItemStack(data) => {
                match entity_kind {
                    EntityKind::Item => set_entity_texture(adjusted_id, utils::texture_from_itemslot(data, mt_server_state), conn).await,
                    _ => utils::logger("[Minecraft] Server sent SetEntityData with ItemStack, but this is only implemented for dropped items! Dropping this EntityDataItem.", 2)
                }
            },
            _ => utils::logger(&format!("[Minecraft] Server sent SetEntityData with unsupported EntityDataValue ({:?})! Dropping this EntityDataItem.", value), 2),
        }
    }
}

pub async fn send_entity_data(id: u16, entitydata: &EntityResendableData, conn: &mut MinetestConnection) {
    let clientbound_moveentity = ToClientCommand::ActiveObjectMessages(
        Box::new(wire::command::ActiveObjectMessagesSpec{
            objects: vec![wire::types::ActiveObjectMessage{
                id,
                data: wire::types::ActiveObjectCommand::UpdatePosition(
                    wire::types::AOCUpdatePosition {
                        position: entitydata.position,
                        velocity: entitydata.velocity,
                        acceleration: entitydata.acceleration,
                        rotation: entitydata.rotation,
                        do_interpolate: false,
                        is_end_position: true,
                        update_interval: 1.0
                    }
                )
            }]
        })
    );
    let _ = conn.send(clientbound_moveentity).await;
}

//FIXME
// causes a segfault, wait for upstream (either minetest_protocol or minetest) to fix this.
// until then, send one packet per entity
pub async fn _send_multi_entity_data(data: Vec<(u16, EntityResendableData)>, conn: &mut MinetestConnection) {
    let mut objects: Vec<wire::types::ActiveObjectMessage> = Vec::new();
    for (id, entitydata) in data {
        objects.push(wire::types::ActiveObjectMessage{
            id,
            data: wire::types::ActiveObjectCommand::UpdatePosition(
                wire::types::AOCUpdatePosition {
                    position: entitydata.position,
                    velocity: entitydata.velocity,
                    acceleration: entitydata.acceleration,
                    rotation: entitydata.rotation,
                    do_interpolate: false,
                    is_end_position: true,
                    update_interval: 1.0
                }
            )
        })
    }
    let clientbound_moveentity = ToClientCommand::ActiveObjectMessages(
        Box::new(wire::command::ActiveObjectMessagesSpec {
            objects
        })
    );
    let _ = conn.send(clientbound_moveentity).await;
}

async fn set_entity_texture(id: u16, texture: String, conn: &MinetestConnection) {
    /*
     * Strictly speaking, this does not *set* a texture.
     * It only works when the previous texture was "".
     * Currently, it *should* only be called when that's the case,
     * but that won't stay so forever (or even always hold true
     * currently, I don't know what MC does). FIXME (later)
     */
    let update_texture_packet = ToClientCommand::ActiveObjectMessages(
        Box::new(wire::command::ActiveObjectMessagesSpec {
            objects: vec![wire::types::ActiveObjectMessage{
                id,
                data: wire::types::ActiveObjectCommand::SetTextureMod(
                    wire::types::AOCSetTextureMod {
                        modifier: texture
                   }
                )
            }]
        })
    );
    let _ = conn.send(update_texture_packet).await;
}

// block placement/destruction
pub async fn blockupdate(packet_data: &ClientboundBlockUpdatePacket, conn: &mut MinetestConnection, mt_server_state: &MTServerState) {
    let ClientboundBlockUpdatePacket { pos, block_state } = packet_data;
    let cave_air_glow = mt_server_state.current_dimension == Dimensions::Nether;
    let BlockPos { x, y, z } = pos;
    let addnodecommand = ToClientCommand::Addnode(
        Box::new(wire::command::AddnodeSpec {
            pos: v3s16 { x: *x as i16, y: *y as i16, z: *z as i16 },
            node: utils::state_to_node(*block_state, cave_air_glow),
            keep_metadata: false
        })
    );
    let _ = conn.send(addnodecommand).await;
}

// block destruction overlay stuff
pub async fn destruction_overlay(packet_data: &ClientboundBlockDestructionPacket, conn: &mut MinetestConnection) {
    // TODO finish this thing as soon as i figure out how to send overlays
    let ClientboundBlockDestructionPacket { id: _, pos, progress } = packet_data;
    let new_overlay = match progress {
        0 => "block-destroy_stage_0.png",
        1 => "block-destroy_stage_1.png",
        2 => "block-destroy_stage_2.png",
        3 => "block-destroy_stage_3.png",
        4 => "block-destroy_stage_4.png",
        5 => "block-destroy_stage_5.png",
        6 => "block-destroy_stage_6.png",
        7 => "block-destroy_stage_7.png",
        8 => "block-destroy_stage_8.png",
        9 => "block-destroy_stage_9.png",
        _ => ""
    };
}

pub async fn open_screen(packet_data: &ClientboundOpenScreenPacket, conn: &mut MinetestConnection, mt_server_state: &mut MTServerState) {
    let ClientboundOpenScreenPacket { container_id: _, menu_type, title } = packet_data;
    let form_spec = mt_definitions::get_container_formspec(menu_type, &title.to_string());
    utils::logger("[Minetest] Showing Formspec for opened container", 1);
    let formspec_command = ToClientCommand::ShowFormspec(
        Box::new(wire::command::ShowFormspecSpec {
            form_spec,
            form_name: String::from("current-container-form")
        })
    );
    let _ = conn.send(formspec_command).await;
    // Update container size (unused, kept in case it does turn out to be needed)
    // mt_server_state.container_size = match menu_type {
    //     MenuKind::Generic9x1 => 9,
    //     MenuKind::Generic9x2 => 18,
    //     MenuKind::Generic9x3 => 27,
    //     MenuKind::Generic9x4 => 36,
    //     MenuKind::Generic9x5 => 45,
    //     MenuKind::Generic9x6 => 54,
    //     MenuKind::Generic3x3 => 9,
    //     MenuKind::Crafter3x3 => 10,
    //     MenuKind::Anvil => 3,
    //     MenuKind::Beacon => 1,
    //     MenuKind::BlastFurnace => 3,
    //     MenuKind::BrewingStand => 5,
    //     MenuKind::Crafting => 10,
    //     MenuKind::Enchantment => 2,
    //     MenuKind::Furnace => 3,
    //     MenuKind::Grindstone => 2,
    //     MenuKind::Hopper => 5,
    //     MenuKind::Lectern => 1,
    //     MenuKind::Loom => 4,
    //     MenuKind::Merchant => 3,
    //     MenuKind::ShulkerBox => 36,
    //     MenuKind::Smithing => 4,
    //     MenuKind::Smoker => 3,
    //     MenuKind::CartographyTable => 3,
    //     MenuKind::Stonecutter => 2
    // }
}


pub async fn block_entity_data(packet_data: &ClientboundBlockEntityDataPacket, conn: &mut MinetestConnection, mt_server_state: &mut MTServerState) {
    let ClientboundBlockEntityDataPacket { pos, block_entity_type, tag: _ } = packet_data;
    if mt_server_state.container_map.insert((pos.x, pos.y, pos.z), *block_entity_type) != None {
        utils::logger(&format!("[Minecraft] Overwriting Block Entity at {:?}", pos), 2);
    }
    // TODO: Add the tag to the block metadata if it is relevant to the client
}

pub async fn refresh_inv(mc_client: &Client, mt_conn: &mut MinetestConnection, mt_server_state: &mut MTServerState) {
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
                // we need to shift the inventory that is sent to the client
                // because the hotbar for some reason isnt the first (or even last!) row in the sent data
                let mut sent_data = serverside_inventory.inventory.to_vec();
                // if we ever use indexes on "main" that were sent by the minetest client,
                // we first need to fix these: serverside = (clientside - 9) % 36
                sent_data.rotate_right(9);
                to_update.push(("main", sent_data));
            }
            if serverside_inventory.offhand != mt_server_state.mt_clientside_player_inv.offhand {
                to_update.push(("offhand", vec![serverside_inventory.offhand.clone()]))
            }
            mt_server_state.mt_clientside_player_inv = serverside_inventory;
        },
        // contents: SlotList<n>
        // different n per menu type, so incompatible types
        // my apologies to anyone having to read this
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
        inventory::Menu::Anvil { first, second, result, player } => {
            to_update.push(("container", vec![first, second, result]));
            to_update.push(("main", player.to_vec()))
        },
        inventory::Menu::Beacon { payment, player } => {
            to_update.push(("container", vec![payment]));
            to_update.push(("main", player.to_vec()))
        },
        inventory::Menu::BlastFurnace { ingredient, fuel, result, player } => {
            to_update.push(("container", vec![ingredient, fuel, result]));
            to_update.push(("main", player.to_vec()))
        },
        inventory::Menu::BrewingStand { bottles, ingredient, fuel, player } => {
            let item_vec = [bottles.to_vec(), vec![ingredient, fuel]].concat();
            to_update.push(("container", item_vec));
            to_update.push(("main", player.to_vec()))
        },
        inventory::Menu::Crafting { result, grid, player } => {
            let item_vec = [grid.to_vec(), vec![result]].concat();
            to_update.push(("container", item_vec));
            to_update.push(("main", player.to_vec()))
        },
        inventory::Menu::Enchantment { item, lapis, player } => {
            to_update.push(("container", vec![item, lapis]));
            to_update.push(("main", player.to_vec()))
        },
        inventory::Menu::Grindstone { input, additional, result, player } => {
            to_update.push(("container", vec![input, additional, result]));
            to_update.push(("main", player.to_vec()))
        },
        inventory::Menu::Hopper { contents, player } => {
            to_update.push(("container", contents.to_vec()));
            to_update.push(("main", player.to_vec()))
        },
        inventory::Menu::Lectern { book, player } => {
            to_update.push(("container", vec![book]));
            to_update.push(("main", player.to_vec()))
        },
        inventory::Menu::Loom { banner, dye, pattern, result, player } => {
            to_update.push(("container", vec![banner, dye, pattern, result]));
            to_update.push(("main", player.to_vec()))
        },
        inventory::Menu::Merchant { payments, result, player } => {
            let item_vec = [payments.to_vec(), vec![result]].concat();
            to_update.push(("container", item_vec));
            to_update.push(("main", player.to_vec()))
        },
        inventory::Menu::ShulkerBox { contents, player } => {
            to_update.push(("container", contents.to_vec()));
            to_update.push(("main", player.to_vec()))
        },
        inventory::Menu::Smithing { template, base, additional, result, player } => {
            to_update.push(("container", vec![template, base, additional, result]));
            to_update.push(("main", player.to_vec()))
        },
        inventory::Menu::Smoker { ingredient, fuel, result, player } => {
            to_update.push(("container", vec![ingredient, fuel, result]));
            to_update.push(("main", player.to_vec()))
        },
        inventory::Menu::CartographyTable { map, additional, result, player } => {
            to_update.push(("container", vec![map, additional, result]));
            to_update.push(("main", player.to_vec()))
        },
        inventory::Menu::Stonecutter { input, result, player } => {
            to_update.push(("container", vec![input, result]));
            to_update.push(("main", player.to_vec()))
        },
        inventory::Menu::Furnace { ingredient, fuel, result, player } => {
            to_update.push(("container", vec![ingredient, fuel, result]));
            to_update.push(("main", player.to_vec()))
        }
    }
    if !to_update.is_empty() {
        update_inventory(mt_conn, to_update).await;
    }
}

pub async fn show_sound(packet_data: &ClientboundSoundPacket, conn: &mut MinetestConnection, mt_server_state: &mut MTServerState) {
    let ClientboundSoundPacket { sound, source: _, x: _, y: _, z: _, volume: _, pitch: _, seed: _ } = packet_data;
    utils::logger(&format!("[Minetest] New Subtitle: {:?}", sound), 1);
    mt_server_state.subtitles.push((format!("{:?}", sound), Instant::now()));
    /*
    let formatted_str = mt_server_state.subtitles.join("\n");
    let subtitle_update_command = ToClientCommand::Hudchange(
        Box::new(wire::command::HudchangeSpec {
            server_id: settings::SUBTITLE_ID,
            stat: HudStat::Text(formatted_str),
        })
    );
    let _ = conn.send(subtitle_update_command).await;*/
}
