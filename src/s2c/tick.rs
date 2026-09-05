use crate::s2c;
use crate::s2c::entity_variants;
use crate::state;
use crate::utils;
use azalea::Client;
use azalea::core::entity_id::MinecraftEntityId;
use azalea::ecs::prelude::With;
use azalea::entity::{EntityKindComponent, LookDirection, Physics, Position, metadata};
use glam::Vec3 as v3f;
use log::*;
use luanti_protocol::LuantiConnection;
use luanti_protocol::commands::server_to_client::{self, ActiveObjectMessage, ToClientCommand};
use luanti_protocol::types;
use std::time::{Duration, Instant};

pub async fn tick(
    luanti_conn: &mut LuantiConnection,
    mc_client: &mut Client,
    proxy_state: &mut state::ProxyState,
) {
    if proxy_state.player.has_moved_since_sync {
        s2c::player::sync_client_pos(mc_client, luanti_conn, &mut proxy_state.player).await;
        proxy_state.player.has_moved_since_sync = false;
    }
    // update the MT clients inventory if it changed
    // for stupid reasons, we don't use packets for this, instead run this on every tick
    // and whenever the player crafted something
    s2c::inventory::refresh_inv(mc_client, luanti_conn, &mut proxy_state.inventory, false).await;
    // update subtitles, removing any older than 1.5 seconds
    let cutoff = Instant::now() - Duration::from_millis(1500);
    proxy_state.chat.subtitles.retain(|x| x.1 > cutoff);
    let mut formatted_str = String::from("");
    for (text, _) in proxy_state.chat.subtitles.clone() {
        formatted_str = format!("{}\n{}", formatted_str, text);
    }
    if formatted_str != proxy_state.chat.prev_subtitle_string {
        // if the subtitle actually changed, update the client
        proxy_state.chat.prev_subtitle_string = formatted_str.clone();
        let subtitle_update_command =
            ToClientCommand::Hudchange(Box::new(server_to_client::HudchangeCommand {
                server_id: s2c::defs::SUBTITLE_ID,
                stat: server_to_client::HudStat::Text(formatted_str),
            }));
        luanti_conn.send(subtitle_update_command).unwrap();
    }

    // update all entities that moved this tick
    proxy_state.entities.entities_update_scheduled.dedup();
    let mut chunks: Vec<Vec<ActiveObjectMessage>> = Vec::new();
    let mut aom_vector: Vec<ActiveObjectMessage> = Vec::new();
    let mut ecs = (*mc_client.ecs).write();
    let mut query = ecs.query_filtered::<(
        &MinecraftEntityId,
        &Position,
        &LookDirection,
        &Physics,
        &EntityKindComponent,
        Option<&state::HeadYaw>,
    ), With<metadata::AbstractEntity>>();
    // check each entity in the ECS
    for (&entity_id, position, look_direction, physics, entity_kind, head_yaw) in query.iter(&ecs) {
        if proxy_state
            .entities
            .entities_update_scheduled
            .contains(&entity_id)
        {
            let acceleration = azalea::Vec3 {
                x: physics.x_acceleration.into(),
                y: physics.y_acceleration.into(),
                z: physics.z_acceleration.into(),
            };
            let Some(clientside_id) = proxy_state.entities.entity_id_map.get_by_left(&entity_id)
            else {
                warn!("Tried to update entity without clientside ID!");
                continue;
            };
            let body_yaw = look_direction.y_rot();
            let head_pitch = look_direction.x_rot();
            // update body rotation and pos/vel/acc
            aom_vector.push(ActiveObjectMessage {
                id: *clientside_id,
                data: types::ActiveObjectCommand::UpdatePosition(types::AOCUpdatePosition {
                    position: v3f {
                        x: utils::mirror_pos(position.x as f32) * 10.0,
                        y: utils::align_pos(position.y as f32) * 10.0,
                        z: utils::align_pos(position.z as f32) * 10.0,
                    },
                    velocity: v3f {
                        // blocks-per-tick to wireblocks-per-second (20Hz ticks * luanti wire format 10x)
                        x: utils::mirror_vec(physics.velocity.x as f32) * 200.0,
                        ..utils::vec3_to_v3f(&physics.velocity, 200)
                    },
                    acceleration: v3f {
                        x: utils::mirror_vec(acceleration.x as f32) * 10.0,
                        ..utils::vec3_to_v3f(&acceleration, 10)
                    },
                    rotation: v3f {
                        x: 0.0,
                        y: utils::mirror_yaw(body_yaw) + utils::get_body_phase(entity_kind.0),
                        z: 0.0,
                    },
                    do_interpolate: true,
                    is_end_position: false,
                    update_interval: 0.1,
                }),
            });
            // if the entity has a rotatable head, also update that
            if let Some(swivel) = utils::get_head_swivel(entity_kind.0) {
                let head_yaw = head_yaw.map(|h| h.0).unwrap_or(body_yaw);
                // get smallest angular difference
                let head_yaw = ((head_yaw - body_yaw + 180.0) % 360.0) - 180.0;
                trace!(
                    "head swivel for {:?}: bone={} pitch={:.1} yaw={:.1} (body_yaw={:.1} head_yaw={:.1})",
                    entity_id, swivel.bone, head_pitch, head_yaw, body_yaw, head_yaw
                );
                let bone_rotation = match swivel.axis {
                    utils::SwivelAxis::Y => v3f {
                        x: head_pitch,
                        y: head_yaw + swivel.phase,
                        z: 0.0,
                    },
                    utils::SwivelAxis::Z => v3f {
                        x: head_pitch,
                        y: 0.0,
                        z: -head_yaw + swivel.phase,
                    },
                };
                aom_vector.push(ActiveObjectMessage {
                    id: *clientside_id,
                    data: types::ActiveObjectCommand::SetBonePosition(types::AOCSetBonePosition {
                        bone: swivel.bone,
                        // relative: (0,0,0) to ignore
                        position: v3f {
                            x: 0.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        rotation: bone_rotation,
                        scale: Some(v3f {
                            x: 1.0,
                            y: 1.0,
                            z: 1.0,
                        }),
                        // interpolate rotation over 1 tick
                        position_interp_duration: Some(0.05),
                        rotation_interp_duration: Some(0.0),
                        scale_interp_duration: Some(0.0),
                        // position relative (to easily ignore it)
                        // rotation absolute
                        absolute_flags: Some(0b010),
                    }),
                });
            }
            if aom_vector.len() >= 20 {
                chunks.push(aom_vector);
                aom_vector = Vec::new()
            }
        }
    }
    drop(ecs);

    // re-check the model/texture of every entity whose metadata changed this tick
    proxy_state.entities.appearance_update_scheduled.dedup();
    for entity_id in proxy_state.entities.appearance_update_scheduled.clone() {
        let Some(&clientside_id) = proxy_state.entities.entity_id_map.get_by_left(&entity_id)
        else {
            continue;
        };
        let Ok(Some(entity)) = mc_client.entity_by_minecraft_id(entity_id) else {
            continue;
        };
        let Some(entity_kind) = mc_client
            .get_entity_component::<EntityKindComponent>(entity.id())
            .map(|c| c.0)
        else {
            continue;
        };
        let extra =
            entity_variants::extract(mc_client, entity.id(), entity_kind, &proxy_state.media);
        let (visual, mesh, textures, size) = entity_variants::get_entity_model(entity_kind, extra);
        let unchanged = proxy_state
            .entities
            .entity_appearance
            .get(&entity_id)
            .is_some_and(|(cached_mesh, cached_textures, cached_size)| {
                *cached_mesh == mesh && *cached_textures == textures && *cached_size == size
            });
        if unchanged {
            continue;
        }
        proxy_state
            .entities
            .entity_appearance
            .insert(entity_id, (mesh.clone(), textures.clone(), size));
        aom_vector.push(s2c::entities::appearance_update_message(
            clientside_id,
            visual,
            mesh,
            textures,
            size,
        ));
        if aom_vector.len() >= 20 {
            chunks.push(aom_vector);
            aom_vector = Vec::new();
        }
    }
    proxy_state.entities.appearance_update_scheduled.clear();

    if !aom_vector.is_empty() {
        chunks.push(aom_vector);
    };
    // sending all updates at once can exceed minetests packet processing budget
    // send at most 20/packet
    for aom_vector in chunks {
        let clientbound_moveentity = ToClientCommand::ActiveObjectMessages(Box::new(
            luanti_protocol::commands::server_to_client::ActiveObjectMessagesCommand {
                objects: aom_vector,
            },
        ));
        luanti_conn.send(clientbound_moveentity).unwrap();
    }
    proxy_state.entities.entities_update_scheduled.clear();

    // sync air supply to client
    let air_supply = mc_client
        .component::<metadata::AirSupply>()
        .unwrap()
        .clone();
    // format of air_supply: 0 - 299
    // 0 -> 0 bubbles displayed
    // 299 -> 20 bubbles
    let approx_bubble_count: u32 = { air_supply.abs() as f32 / 14.95 }.round() as u32;
    if approx_bubble_count != proxy_state.player.mc_last_air_supply {
        s2c::player::edit_airbar(
            approx_bubble_count,
            luanti_conn,
            proxy_state.player.mc_last_air_supply,
        )
        .await;
        proxy_state.player.mc_last_air_supply = approx_bubble_count;
    };

    // check for sprinting/sneaking, change client movement speed if needed
    let sprinting = *mc_client.component::<metadata::Sprinting>().unwrap();
    if sprinting.0 && proxy_state.player.is_sneaking {
        proxy_state.player.is_sneaking = false
    }
    // TODO: soul sand, cobwebs etc may also change player speed
    let current_speed: f32 = match (sprinting.0, proxy_state.player.is_sneaking) {
        (false, false) => 4.317,
        (false, true) => 1.295,
        (true, false) => 5.612,
        (true, true) => {
            proxy_state.player.is_sneaking = false;
            5.612
        }
    };
    if current_speed != proxy_state.player.mt_max_speed {
        proxy_state.player.mt_max_speed = current_speed;
        luanti_conn
            .send(s2c::defs::get_movementspec(current_speed))
            .unwrap();
    }
}
