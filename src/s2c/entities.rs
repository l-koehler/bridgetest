use azalea::entity::{EntityDataItem, EntityDataValue};
use azalea::registry::MobEffect;
use azalea::world::MinecraftEntityId;
use log::*;
use luanti_protocol::types::ObjectProperties;
use std::time::Duration;

use glam::I16Vec2 as v2i16;
use glam::Vec2 as v2f;
use glam::Vec3 as v3f;

use azalea::registry::EntityKind;
use luanti_protocol::LuantiConnection;
use luanti_protocol::commands::server_to_client;
use luanti_protocol::commands::server_to_client::ToClientCommand;
use luanti_protocol::types::{ActiveObjectCommand, AddedObject, GenericInitData, SColor, aabb3f};

use azalea_client::Client;

use azalea::protocol::packets::game::{
    c_add_entity::ClientboundAddEntity, c_entity_event::ClientboundEntityEvent,
    c_entity_position_sync::ClientboundEntityPositionSync,
    c_move_entity_pos::ClientboundMoveEntityPos,
    c_move_entity_pos_rot::ClientboundMoveEntityPosRot,
    c_move_entity_rot::ClientboundMoveEntityRot, c_remove_entities::ClientboundRemoveEntities,
    c_remove_mob_effect::ClientboundRemoveMobEffect, c_set_entity_data::ClientboundSetEntityData,
    c_set_entity_motion::ClientboundSetEntityMotion, c_teleport_entity::ClientboundTeleportEntity,
    c_update_mob_effect::ClientboundUpdateMobEffect,
};

use std::time::Instant;

use crate::s2c;
use crate::state;
use crate::utils;

pub enum EAddType {
    Entity(ClientboundAddEntity),
    Player(String),
}

// if no packet is passed, add the player using data from the server state
pub async fn add_entity(
    optional_packet: EAddType,
    conn: &mut LuantiConnection,
    entity_state: &mut state::EntityState,
) {
    let is_player: bool;
    let name: String;
    let c_id: u16;
    let position: v3f;
    let mesh: String;
    let textures: Vec<String>;
    let visual: String;
    match optional_packet {
        EAddType::Entity(packet_data) => {
            // use a network packet
            let ClientboundAddEntity {
                id: serverside_id,
                uuid,
                entity_type, // TODO: textures and models depend on this thing
                position: vec_pos,
                x_rot: _,
                y_rot: _,
                y_head_rot: _,
                data: _,
                velocity: _,
            } = packet_data;
            is_player = false;
            name = format!("UUID-{}", uuid);
            c_id = utils::allocate_id(serverside_id.0 as u32, entity_state);
            position = utils::vec3_to_v3f(&vec_pos, 10);
            if entity_type.clone() == EntityKind::Item {
                visual = String::from("sprite");
                mesh = String::new();
                // what item it is can't be known at this time, leave empty so
                // a "texture modifier" sent later will just set the texture
                textures = vec![String::from("")];
            } else {
                visual = String::from("mesh");
                (mesh, textures) = utils::get_entity_model(entity_type);
            }
        }
        EAddType::Player(p_name) => {
            is_player = true;
            name = p_name;
            visual = String::from("mesh");
            c_id = 0; // ensured to be "free" by the allocatable range starting at 1
            position = v3f {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }; // player will be moved somewhere else later
            mesh = String::from("model-villager.b3d"); // TODO
            textures = vec![String::from("entity-player-slim-steve.png")];
        }
    };

    let added_object: AddedObject = AddedObject {
        id: c_id,
        typ: 101, // idk
        init_data: GenericInitData {
            version: 1, // used a packet sniffer, idk if there are other versions
            name,
            is_player, // possibly a lie, but thats not the clients problem anyways
            id: c_id,
            position,
            rotation: v3f {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            hp: 100, // entity deaths handled by server
            messages: vec![
                ActiveObjectCommand::SetProperties(luanti_protocol::types::AOCSetProperties {
                    newprops: ObjectProperties {
                        version: 4,
                        hp_max: 100,
                        physical: true,
                        _unused: 0,
                        // player hitbox
                        // entity hits are calculated by the proxy anyways
                        collision_box: aabb3f {
                            min_edge: v3f {
                                x: -0.3,
                                y: 0.0,
                                z: -0.3,
                            },
                            max_edge: v3f {
                                x: 0.3,
                                y: 1.8,
                                z: 0.3,
                            },
                        },
                        selection_box: aabb3f {
                            min_edge: v3f {
                                x: -0.3,
                                y: 0.0,
                                z: -0.3,
                            },
                            max_edge: v3f {
                                x: 0.3,
                                y: 1.8,
                                z: 0.3,
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
                        spritediv: v2i16 { x: 1, y: 1 },
                        initial_sprite_basepos: v2i16 { x: 0, y: 0 },
                        is_visible: true,
                        makes_footstep_sound: true,
                        automatic_rotate: 0.0,
                        mesh: String::from(mesh),
                        colors: vec![SColor::new(255, 255, 255, 255)],
                        collide_with_objects: false,
                        stepheight: 0.0,
                        automatic_face_movement_dir: false,
                        automatic_face_movement_dir_offset: 0.0,
                        backface_culling: true,
                        nametag: String::from(""), // type_str,
                        nametag_color: SColor::new(255, 255, 255, 255),
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
                        rotate_selectionbox: Some(false),
                    },
                }),
                ActiveObjectCommand::SetTextureMod(luanti_protocol::types::AOCSetTextureMod {
                    modifier: String::from(""),
                }),
                ActiveObjectCommand::SetAnimation(luanti_protocol::types::AOCSetAnimation {
                    range: v2f { x: 0.0, y: 0.0 },
                    speed: 0.0,
                    blend: 0.0,
                    no_loop: false,
                }),
                ActiveObjectCommand::UpdateArmorGroups(
                    luanti_protocol::types::AOCUpdateArmorGroups {
                        ratings: vec![(String::from("immortal"), 1)],
                    },
                ),
                ActiveObjectCommand::AttachTo(luanti_protocol::types::AOCAttachTo {
                    parent_id: 0,
                    bone: String::from(""),
                    position: v3f {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    rotation: v3f {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    force_visible: false,
                }),
            ],
        },
    };

    let clientbound_addentity = ToClientCommand::ActiveObjectRemoveAdd(Box::new(
        server_to_client::ActiveObjectRemoveAddSpec {
            removed_object_ids: vec![],
            added_objects: vec![added_object],
        },
    ));
    conn.send(clientbound_addentity).unwrap();
}

pub async fn remove_entity(
    packet_data: &ClientboundRemoveEntities,
    conn: &mut LuantiConnection,
    entity_state: &mut state::EntityState,
) {
    let ClientboundRemoveEntities { entity_ids } = packet_data;
    let mut entity_ids_adjusted: Vec<u16> = vec![];
    for entity_id in entity_ids {
        let Some(clientside_id) = entity_state.entity_id_map.get_by_left(entity_id) else {
            warn!("Got S2C RemoveEntity with unknown ID, skipping!");
            continue;
        };
        entity_ids_adjusted.push(*clientside_id);
        utils::free_id(entity_id.0 as u32, entity_state);
    }
    if !entity_ids_adjusted.is_empty() {
        let clientbound_removeentity = ToClientCommand::ActiveObjectRemoveAdd(Box::new(
            server_to_client::ActiveObjectRemoveAddSpec {
                removed_object_ids: entity_ids_adjusted,
                added_objects: vec![],
            },
        ));
        conn.send(clientbound_removeentity).unwrap();
    } else {
        info!("Got S2C RemoveEntity without entity IDs to remove");
    }
}

pub async fn entity_setpos(
    packet_data: &ClientboundMoveEntityPos,
    entity_state: &mut state::EntityState,
) {
    let ClientboundMoveEntityPos {
        entity_id,
        delta: _,
        on_ground: _,
    } = packet_data;
    entity_state.entities_update_scheduled.push(*entity_id);
}

pub async fn entity_teleport(
    packet_data: &ClientboundTeleportEntity,
    entity_state: &mut state::EntityState,
) {
    let ClientboundTeleportEntity {
        id,
        change: _,
        relatives: _,
        on_ground: _,
    } = packet_data;
    entity_state.entities_update_scheduled.push(*id);
}

pub async fn entity_setposrot(
    packet_data: &ClientboundMoveEntityPosRot,
    entity_state: &mut state::EntityState,
) {
    let ClientboundMoveEntityPosRot {
        entity_id,
        delta: _,
        y_rot: _,
        x_rot: _,
        on_ground: _,
    } = packet_data;
    entity_state.entities_update_scheduled.push(*entity_id);
}

pub async fn entity_setrot(
    packet_data: &ClientboundMoveEntityRot,
    entity_state: &mut state::EntityState,
) {
    let ClientboundMoveEntityRot {
        entity_id,
        y_rot: _,
        x_rot: _,
        on_ground: _,
    } = packet_data;
    entity_state.entities_update_scheduled.push(*entity_id);
}

pub async fn entity_setmotion(
    packet_data: &ClientboundSetEntityMotion,
    entity_state: &mut state::EntityState,
) {
    let ClientboundSetEntityMotion { id, delta: _ } = packet_data;
    entity_state.entities_update_scheduled.push(*id);
}

pub fn entity_sync(
    packet_data: &ClientboundEntityPositionSync,
    entity_state: &mut state::EntityState,
) {
    let ClientboundEntityPositionSync {
        id,
        values: _,
        on_ground: _,
    } = packet_data;
    entity_state.entities_update_scheduled.push(*id);
}

pub async fn entity_event(
    packet_data: &ClientboundEntityEvent,
    _conn: &mut LuantiConnection,
    mc_client: &Client,
) {
    let ClientboundEntityEvent {
        entity_id,
        event_id,
    } = packet_data;
    let Some(entity) = mc_client.ecs_entity_by_minecraft_entity(*entity_id) else {
        warn!("Got S2C EntityEvent for unknown ID, skipping!");
        return;
    };
    let entity_kind = mc_client
        .get_entity_component::<azalea::entity::EntityKind>(entity)
        .unwrap()
        .0;

    let bad_id_for_entity = format!(
        "Got entity event for entity ID {} referring to a entity of type {}, this event isn't implemented for that entity.",
        entity_id, entity_kind
    );
    // https://wiki.vg/Entity_statuses
    match event_id {
        0 => (), // Tipped Arrow particles
        1 => {
            match entity_kind {
                EntityKind::Rabbit => (),          // Rabbit Jump animation
                EntityKind::SpawnerMinecart => (), // Reset cooldown to 200 ticks, only relevant to server
                _ => warn!("{}", &bad_id_for_entity),
            }
        }
        3 => {
            match entity_kind {
                EntityKind::Egg => (),      // Display "ironcrack" particles at own location
                EntityKind::Snowball => (), // Display "snowballpoof" particles at own location
                _ => (),                    // Death sound & animation
            }
        }
        4 => {
            match entity_kind {
                EntityKind::EvokerFangs => (), // Attack animation and sound
                EntityKind::Hoglin => (),      // Attack animation and sound
                EntityKind::IronGolem => (),   // Attack animation and sound
                EntityKind::Ravager => (),     // Attack animation for 10 ticks
                EntityKind::Zoglin => (),      // Attack animation and sound
                _ => warn!("{}", &bad_id_for_entity),
            }
        }
        6 => (), // Taming Fail particles (smoke)
        7 => (), // Taming Success particles (heart)
        8 => (), // Wolf shaking water animation
        9 => (), // Item usage finished (e.g. eating done)
        10 => {
            match entity_kind {
                EntityKind::Sheep => (),       // Sheep eating grass animation
                EntityKind::TntMinecart => (), // Ignite TntMinecart
                _ => warn!("{}", &bad_id_for_entity),
            }
        }
        11 => (),      // Iron golem holding flower for 20 seconds animation
        12 => (),      // villager mating heart particles
        13 => (),      // villager angry particles
        14 => (),      // villager happy particles
        15 => (),      // spawn 10 to 45 "witchMagic" particles
        16 => (),      // play zombieVillagerCure sound
        17 => (),      // trigger firework explosion
        18 => (),      // spawn heart particles
        19 => (),      // reset rotation
        20 => (),      // spawn explosion particles
        21 => (),      // guardian attack sound effect
        22 | 23 => (), // enable/disable reduced debug screen info (TODO basic_debug flag in minetest)
        24..29 => (),  // OP permission level 0..4
        29 | 30 => (), // shield block / break sounds
        47..53 => (),  // equipment break sound (mainhand, offhand, head..feet slot)
        _ => warn!(
            "Got S2C unsupported Entity Event (Event ID: {}, Entity ID: {})",
            event_id, entity_id
        ),
    }
}

pub async fn set_entity_data(
    packet_data: &ClientboundSetEntityData,
    conn: &mut LuantiConnection,
    entity_state: &state::EntityState,
    media_state: &state::MediaState,
    mc_client: &Client,
) {
    // Currently, the only data that will actually be used is EntityDataValue::ItemStack in EntityKind::Item
    // Everything else gets dropped.
    let ClientboundSetEntityData { id, packed_items } = packet_data;

    let Some(clientside_id) = entity_state.entity_id_map.get_by_left(id) else {
        warn!("Got S2C SetEntityData for unknown ID, skipping!");
        return;
    };

    let Some(entity) = mc_client.ecs_entity_by_minecraft_entity(*id) else {
        warn!("Got S2C SetEntityData for unknown ID, skipping!");
        return;
    };
    let entity_kind = mc_client
        .get_entity_component::<azalea::entity::EntityKind>(entity)
        .unwrap()
        .0;

    let mut metadata_item: &EntityDataItem;
    for i in 0..packed_items.len() {
        metadata_item = &packed_items[i];
        let EntityDataItem { index: _, value } = metadata_item;
        match value {
            EntityDataValue::ItemStack(data) => match entity_kind {
                EntityKind::Item => {
                    set_entity_texture(
                        *clientside_id,
                        utils::texture_from_itemstack(data, media_state),
                        conn,
                    )
                    .await
                }
                _ => info!(
                    "Got S2C SetEntityData with ItemStack, but this is only implemented for dropped items! Dropping this EntityDataItem"
                ),
            },
            _ => info!(
                "Got S2C SetEntityData with unsupported EntityDataValue ({:?})! Dropping this EntityDataItem",
                value
            ),
        }
    }
}

async fn set_entity_texture(id: u16, texture: String, conn: &LuantiConnection) {
    /*
     * Strictly speaking, this does not *set* a texture.
     * It only works when the previous texture was "".
     * Currently, it *should* only be called when that's the case,
     * but that won't stay so forever (or even always hold true
     * currently, I don't know what MC does). FIXME: (later)
     */
    let update_texture_packet = ToClientCommand::ActiveObjectMessages(Box::new(
        server_to_client::ActiveObjectMessagesCommand {
            objects: vec![server_to_client::ActiveObjectMessage {
                id,
                data: luanti_protocol::types::ActiveObjectCommand::SetTextureMod(
                    luanti_protocol::types::AOCSetTextureMod { modifier: texture },
                ),
            }],
        },
    ));
    conn.send(update_texture_packet).unwrap();
}

pub async fn update_mob_effect(
    packet_data: &ClientboundUpdateMobEffect,
    player_state: &mut state::PlayerState,
    conn: &mut LuantiConnection,
    mc_client: &Client,
) {
    let ClientboundUpdateMobEffect {
        entity_id,
        mob_effect,
        effect_amplifier: _,
        effect_duration_ticks,
        flags,
    } = packet_data;
    // if player is affected, we may need to update the formspecs
    if (*entity_id == mc_client.get_component::<MinecraftEntityId>().unwrap()) {
        let health: u32 = player_state.mt_last_known_health.into();
        match mob_effect {
            MobEffect::Wither => {
                s2c::player::edit_healthbar(s2c::defs::HeartDisplay::Wither, health, conn).await
            }
            MobEffect::Poison => {
                s2c::player::edit_healthbar(s2c::defs::HeartDisplay::Poison, health, conn).await
            }
            MobEffect::Absorption => {
                s2c::player::edit_healthbar(s2c::defs::HeartDisplay::Absorb, health, conn).await
            }
            MobEffect::Hunger => {
                s2c::player::edit_foodbar(
                    s2c::defs::FoodDisplay::Hunger,
                    mc_client.hunger().food,
                    conn,
                )
                .await
            }
            _ => (),
        }
        let duration_ms = Duration::from_millis((effect_duration_ticks * 50).into());
        let expires_at = Instant::now().checked_add(duration_ms).unwrap();
        player_state
            .client_effects
            .push((*mob_effect, expires_at, *flags));
        // update effects immediately, don't wait up to a second for the tick
        s2c::player::update_effects(conn, &player_state.client_effects).await;
    }

    // also spawn particles at the mob
    //TODO
}

pub async fn remove_mob_effect(
    packet_data: &ClientboundRemoveMobEffect,
    conn: &mut LuantiConnection,
    player_state: &mut state::PlayerState,
    mc_client: &Client,
) {
    let ClientboundRemoveMobEffect { entity_id, effect } = packet_data;
    if (*entity_id == mc_client.get_component::<MinecraftEntityId>().unwrap()) {
        match effect {
            MobEffect::Wither | MobEffect::Poison | MobEffect::Absorption => {
                let health: u32 = player_state.mt_last_known_health.into();
                s2c::player::edit_healthbar(s2c::defs::HeartDisplay::Normal, health, conn).await
            }
            MobEffect::Hunger => {
                s2c::player::edit_foodbar(
                    s2c::defs::FoodDisplay::Normal,
                    mc_client.hunger().food,
                    conn,
                )
                .await
            }
            _ => (),
        }
        // remove effect from state and update HUD
        player_state.client_effects.retain(|i| i.0 != *effect);
        s2c::player::update_effects(conn, &player_state.client_effects).await;
    }
}
