use crate::s2c;
use crate::state;
use crate::utils;
use azalea::Client;
use azalea::ecs::prelude::With;
use azalea::entity::{LookDirection, Physics, Position, metadata::AbstractEntity};
use azalea::world::MinecraftEntityId;
use glam::Vec3 as v3f;
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
    // for stupid reasons, we don't use packets for this, instead on every tick
    // and whenever the player crafted something
    s2c::inventory::refresh_inv(mc_client, luanti_conn, &mut proxy_state.inventory).await;
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
    let mut ecs = mc_client.ecs.lock();
    let mut query = ecs
        .query_filtered::<(&MinecraftEntityId, &Position, &LookDirection, &Physics), With<AbstractEntity>>();
    // check each entity in the ECS
    for (&entity_id, position, look_direction, physics) in query.iter(&ecs) {
        // this lets me remove() after checking if entity_id is present without iterating again
        if proxy_state.entities.entities_update_scheduled.is_empty() {
            break;
        }
        let index_in_sched = proxy_state
            .entities
            .entities_update_scheduled
            .iter()
            .position(|n| *n == entity_id);
        if index_in_sched.is_some() {
            proxy_state
                .entities
                .entities_update_scheduled
                .remove(index_in_sched.unwrap());
            let acceleration = azalea::Vec3 {
                x: physics.x_acceleration.into(),
                y: physics.y_acceleration.into(),
                z: physics.z_acceleration.into(),
            };
            aom_vector.push(ActiveObjectMessage {
                id: *proxy_state
                    .entities
                    .entity_id_map
                    .get_by_left(&entity_id)
                    .unwrap(),
                data: types::ActiveObjectCommand::UpdatePosition(types::AOCUpdatePosition {
                    position: utils::vec3_to_v3f(position, 10),
                    velocity: utils::vec3_to_v3f(&physics.velocity, 400),
                    acceleration: utils::vec3_to_v3f(&acceleration, 10),
                    rotation: v3f {
                        x: look_direction.x_rot,
                        y: look_direction.y_rot,
                        z: 0.0,
                    },
                    // these values *might* be wrong in case of teleport packets
                    // but that's not a big problem, interpolation just affects client-side graphics a tiny bit.
                    do_interpolate: true,
                    is_end_position: false,
                    update_interval: 1.0,
                }),
            });
            if aom_vector.len() == 20 {
                chunks.push(aom_vector);
                aom_vector = Vec::new()
            }
        }
    }
    drop(ecs);
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

    // sync air supply to client
    let air_supply: azalea::entity::metadata::AirSupply = mc_client.component();
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
    let sprinting: azalea::entity::metadata::Sprinting = mc_client.component();
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
