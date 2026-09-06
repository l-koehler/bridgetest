use azalea::core::entity_id::MinecraftEntityId;
use log::*;
use luanti_protocol::types::ObjectProperties;
use std::time::Duration;

use glam::I16Vec2 as v2i16;
use glam::Vec2 as v2f;
use glam::Vec3 as v3f;

use azalea::registry::builtin::{EntityKind, MobEffect};
use luanti_protocol::LuantiConnection;
use luanti_protocol::commands::server_to_client;
use luanti_protocol::commands::server_to_client::ToClientCommand;
use luanti_protocol::types::{ActiveObjectCommand, AddedObject, GenericInitData, SColor, aabb3f};

use azalea::Client;

use azalea::protocol::packets::game::{
    c_add_entity::ClientboundAddEntity, c_entity_event::ClientboundEntityEvent,
    c_entity_position_sync::ClientboundEntityPositionSync,
    c_move_entity_pos::ClientboundMoveEntityPos,
    c_move_entity_pos_rot::ClientboundMoveEntityPosRot,
    c_move_entity_rot::ClientboundMoveEntityRot, c_remove_entities::ClientboundRemoveEntities,
    c_remove_mob_effect::ClientboundRemoveMobEffect, c_rotate_head::ClientboundRotateHead,
    c_set_entity_data::ClientboundSetEntityData, c_set_entity_motion::ClientboundSetEntityMotion,
    c_teleport_entity::ClientboundTeleportEntity, c_update_mob_effect::ClientboundUpdateMobEffect,
};

use std::time::Instant;

use crate::s2c;
use crate::s2c::entity_variants;
use crate::state;
use crate::utils::{self, get_head_swivel};

pub enum EAddType {
    Entity(ClientboundAddEntity),
    Player(String),
}

// shared by the initial spawn (add_entity) and later updates (new metadata)
fn build_object_properties(
    visual: String,
    mesh: String,
    textures: Vec<String>,
    size: [f32; 3],
) -> ObjectProperties {
    ObjectProperties {
        version: 4,
        hp_max: 100,
        // luanti physics are inaccurate,
        // but at least prevent mobs from sinking in the ground
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
            x: size[0],
            y: size[1],
            z: size[2],
        },
        textures,
        spritediv: v2i16 { x: 1, y: 1 },
        initial_sprite_basepos: v2i16 { x: 0, y: 0 },
        is_visible: true,
        makes_footstep_sound: true,
        automatic_rotate: 0.0,
        mesh,
        colors: vec![SColor::new(255, 255, 255, 255)],
        collide_with_objects: true,
        stepheight: 0.0,
        automatic_face_movement_dir: false,
        automatic_face_movement_dir_offset: 0.0,
        backface_culling: true,
        nametag: String::from(""),
        nametag_color: SColor::new(255, 255, 255, 255),
        automatic_face_movement_max_rotation_per_sec: 360.0,
        infotext: String::from(""),
        wield_item: String::from(""),
        glow: 0,
        breath_max: 0,
        eye_height: 1.625,
        zoom_fov: 0.0,
        use_texture_alpha: false,
        damage_texture_modifier: Some(String::from("^[colorize:#FF2244:128")),
        shaded: Some(true),
        show_on_minimap: Some(false),
        nametag_bgcolor: None,
        rotate_selectionbox: Some(false),
    }
}

// resend the full set of properties with a new visual/mesh/textures/size
pub fn appearance_update_message(
    client_id: u16,
    visual: String,
    mesh: String,
    textures: Vec<String>,
    size: [f32; 3],
) -> server_to_client::ActiveObjectMessage {
    server_to_client::ActiveObjectMessage {
        id: client_id,
        data: ActiveObjectCommand::SetProperties(luanti_protocol::types::AOCSetProperties {
            newprops: build_object_properties(visual, mesh, textures, size),
        }),
    }
}

// if no ClientboundAddEntity is given, add the player
pub async fn add_entity(
    optional_packet: EAddType,
    conn: &mut LuantiConnection,
    entity_state: &mut state::EntityState,
    mc_client: &Client,
    media_state: &state::MediaState,
) {
    let is_player: bool;
    let name: String;
    let c_id: u16;
    let position: v3f;
    let rotation: v3f;
    let mesh: String;
    let mut textures: Vec<String>;
    let visual: String;
    let size: [f32; 3];
    match optional_packet {
        EAddType::Entity(packet_data) => {
            // use a network packet
            let ClientboundAddEntity {
                id: serverside_id,
                uuid,
                entity_type, // TODO: textures and models depend on this thing
                position: vec_pos,
                x_rot,
                y_rot,
                y_head_rot,
                data,
                ..
            } = packet_data;
            is_player = false;
            name = format!("UUID-{}", uuid);
            c_id = utils::allocate_id(serverside_id.0 as u32, entity_state);

            // mirror_pos/align_pos operate on raw block-unit coordinates, so
            // they're applied before the *10 wire scale.
            position = v3f {
                x: utils::mirror_pos(vec_pos.x as f32) * 10.0,
                y: utils::align_pos(vec_pos.y as f32) * 10.0,
                z: utils::align_pos(vec_pos.z as f32) * 10.0,
            };
            // rotation behavior depends on the model bones
            if let Some(_) = get_head_swivel(entity_type) {
                // body only gets yaw, no pitch allowed
                rotation = v3f {
                    x: x_rot as f32 * (360.0 / 256.0),
                    y: 0.0,
                    z: 0.0,
                };
                insert_y_head_rot(&serverside_id, &y_head_rot, mc_client);
            } else {
                // vehicles etc, no rotatable head
                // can have a body pitch
                rotation = v3f {
                    x: x_rot as f32 * (360.0 / 256.0),
                    y: utils::mirror_yaw(y_rot as f32 * (360.0 / 256.0)),
                    z: 0.0,
                };
            }

            // per-instance metadata (variant/baby/...) hasn't arrived yet,
            // start with the default variant info.
            (visual, mesh, textures, size) = entity_variants::get_entity_model(
                entity_type,
                entity_variants::ExtraData::default(),
            );

            // falling blocks carry their block state in the spawn packet
            if entity_type == EntityKind::FallingBlock
                && let Some(real_textures) =
                    entity_variants::falling_block_textures(data, media_state)
            {
                textures = real_textures.to_vec();
            }
        }
        EAddType::Player(p_name) => {
            is_player = true;
            name = p_name;
            visual = String::from("mesh");
            c_id = 0;
            position = v3f::ZERO; // player will be moved somewhere else later
            rotation = v3f::ZERO;
            mesh = String::from("villager.b3d"); // TODO we have no model
            textures = vec![String::from("villager.png")];
            size = [1.0, 1.0, 1.0];
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
            rotation,
            hp: 100, // entity deaths handled by server
            messages: vec![
                ActiveObjectCommand::SetProperties(luanti_protocol::types::AOCSetProperties {
                    newprops: build_object_properties(visual, mesh, textures, size),
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
    let ClientboundTeleportEntity { id, .. } = packet_data;
    entity_state.entities_update_scheduled.push(*id);
}

pub async fn entity_setposrot(
    packet_data: &ClientboundMoveEntityPosRot,
    entity_state: &mut state::EntityState,
) {
    let ClientboundMoveEntityPosRot { entity_id, .. } = packet_data;
    entity_state.entities_update_scheduled.push(*entity_id);
}

pub async fn entity_setrot(
    packet_data: &ClientboundMoveEntityRot,
    entity_state: &mut state::EntityState,
) {
    let ClientboundMoveEntityRot { entity_id, .. } = packet_data;
    entity_state.entities_update_scheduled.push(*entity_id);
}

pub async fn entity_rotate_head(
    packet_data: &ClientboundRotateHead,
    entity_state: &mut state::EntityState,
    mc_client: &Client,
) {
    let ClientboundRotateHead {
        entity_id,
        y_head_rot,
    } = packet_data;
    insert_y_head_rot(entity_id, y_head_rot, mc_client);
    entity_state.entities_update_scheduled.push(*entity_id);
}

// Head yaw does not get stored by the ECS by-default
fn insert_y_head_rot(entity_id: &MinecraftEntityId, y_head_rot: &i8, mc_client: &Client) {
    // stash it on the ECS entity itself (state::HeadYaw) rather than a side
    // table, so tick() reads head and body rotation the same way
    match mc_client.entity_by_minecraft_id(*entity_id) {
        Ok(Some(entity_ref)) => {
            let mut ecs = mc_client.ecs.write();
            if let Ok(mut entity_mut) = ecs.get_entity_mut(entity_ref.id()) {
                // Convert from 1/256turn to 1/360turn (degrees)
                entity_mut.insert(state::HeadYaw(*y_head_rot as f32 * (360.0 / 256.0)));
            }
        }
        _ => warn!(
            "Got head rotation for unknown entity {:?}, ignoring it",
            entity_id
        ),
    }
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

pub async fn set_entity_data(
    packet_data: &ClientboundSetEntityData,
    entity_state: &mut state::EntityState,
) {
    let ClientboundSetEntityData { id, .. } = packet_data;
    entity_state.appearance_update_scheduled.push(*id);
}

// trigger damage flash, prevent actually client-side killing the entity by healing it in the same command
// duration (luanti content_cao.cpp): 50ms + 50ms per hp lost, max 1s
pub async fn damage_flash(
    entity_id: &MinecraftEntityId,
    entity_state: &state::EntityState,
    conn: &mut LuantiConnection,
) {
    let Some(&clientside_id) = entity_state.entity_id_map.get_by_left(entity_id) else {
        warn!("Got damage event for unknown entity {:?}, skipping!", entity_id);
        return;
    };
    let sethealth_punch = |hp| server_to_client::ActiveObjectMessage {
        id: clientside_id,
        data: ActiveObjectCommand::Punched(luanti_protocol::types::AOCPunched { hp }),
    };
    let clientbound_punched = ToClientCommand::ActiveObjectMessages(Box::new(
        server_to_client::ActiveObjectMessagesCommand {
            objects: vec![sethealth_punch(95), sethealth_punch(100)],
        },
    ));
    conn.send(clientbound_punched).unwrap();
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
    let Ok(Some(entity)) = mc_client.entity_by_minecraft_id(*entity_id) else {
        warn!("Got S2C EntityEvent for unknown ID, skipping!");
        return;
    };
    let entity_kind = mc_client
        .get_entity_component::<azalea::entity::EntityKindComponent>(entity.id())
        .unwrap()
        .0;

    let bad_id_for_entity = format!(
        "Got entity event for entity ID {} referring to a entity of type {}, this event isn't implemented for that entity.",
        entity_id, entity_kind
    );
    // https://wiki.vg/Entity_statuses
    match event_id {
        0 => (), // Tipped Arrow particles
        // obsolete since 1.19.4, replaced by ClientboundDamageEvent
        //2 => damage_flash(entity_id, entity_state, conn).await,
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

pub async fn update_mob_effect(
    packet_data: &ClientboundUpdateMobEffect,
    player_state: &mut state::PlayerState,
    conn: &mut LuantiConnection,
    mc_client: &Client,
) {
    let ClientboundUpdateMobEffect {
        entity_id,
        mob_effect,
        data,
    } = packet_data;
    // if player is affected, we may need to update the formspecs
    if (*entity_id == *mc_client.component::<MinecraftEntityId>().unwrap()) {
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
                    mc_client.hunger().unwrap().food,
                    conn,
                )
                .await
            }
            _ => (),
        }
        let duration_ms = Duration::from_millis((data.duration * 50).try_into().unwrap());
        let expires_at = Instant::now().checked_add(duration_ms).unwrap();
        player_state
            .client_effects
            .push((*mob_effect, expires_at, data.clone()));
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
    if (*entity_id == *mc_client.component::<MinecraftEntityId>().unwrap()) {
        match effect {
            MobEffect::Wither | MobEffect::Poison | MobEffect::Absorption => {
                let health: u32 = player_state.mt_last_known_health.into();
                s2c::player::edit_healthbar(s2c::defs::HeartDisplay::Normal, health, conn).await
            }
            MobEffect::Hunger => {
                s2c::player::edit_foodbar(
                    s2c::defs::FoodDisplay::Normal,
                    mc_client.hunger().unwrap().food,
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
