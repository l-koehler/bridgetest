use minetest_protocol::wire::types::v3f;
use minetest_protocol::MinetestConnection;
use azalea_client::Client;
use crate::{mt_definitions, utils};
use azalea::ecs::prelude::With;
use azalea::entity::{metadata::AbstractEntity, LookDirection, Position, Physics};
use azalea::world::MinecraftEntityId;

use crate::MTServerState;
use crate::clientbound_translator;
use crate::mt_definitions::V3F_ZERO;
use std::time::{Duration, Instant};
use minetest_protocol::wire::command::ToClientCommand;
use minetest_protocol::wire;
use crate::settings;

pub async fn server(mt_conn: &mut MinetestConnection, mc_client: &Client, mt_server_state: &mut MTServerState) {
    if mt_server_state.has_moved_since_sync {
        clientbound_translator::sync_client_pos(mc_client, mt_conn, mt_server_state).await;
        mt_server_state.has_moved_since_sync = false;
    }
    // update the MT clients inventory if it changed
    // for stupid reasons, we don't use packets for this, instead on every tick
    // and whenever the player crafted something
    clientbound_translator::refresh_inv(mc_client, mt_conn, mt_server_state).await;
    // update subtitles, removing any older than 1.5 seconds
    let cutoff = Instant::now() - Duration::from_millis(1500);
    mt_server_state.subtitles.retain(|x| x.1 > cutoff);
    let mut formatted_str = String::from("");
    for (text, _) in mt_server_state.subtitles.clone() {
        formatted_str = format!("{}\n{}", formatted_str, text);
    };
    if formatted_str != mt_server_state.prev_subtitle_string {
        // if the subtitle actually changed, update the client
        mt_server_state.prev_subtitle_string = formatted_str.clone();
        let subtitle_update_command = ToClientCommand::Hudchange(
            Box::new(wire::command::HudchangeSpec {
                server_id: settings::SUBTITLE_ID,
                stat: wire::types::HudStat::Text(formatted_str),
            })
        );
        let _ = mt_conn.send(subtitle_update_command).await;
    }
    
    // update all entities that moved this tick
    mt_server_state.entities_update_scheduled.sort();
    mt_server_state.entities_update_scheduled.dedup();
    let mut chunks: Vec<Vec<wire::types::ActiveObjectMessage>> = Vec::new();
    let mut aom_vector: Vec<wire::types::ActiveObjectMessage> = Vec::new();
    let mut ecs = mc_client.ecs.lock();
    let mut query = ecs
        .query_filtered::<(&MinecraftEntityId, &Position, &LookDirection, &Physics), With<AbstractEntity>>();
    // check each entity in the ECS
    for (&entity_id, position, look_direction, physics) in query.iter(&ecs) {
        // this lets me remove() after checking if entity_id is present without iterating again
        if mt_server_state.entities_update_scheduled.is_empty() {break;}
        let index_in_sched = mt_server_state.entities_update_scheduled.iter().position(|n| *n == entity_id.0);
        if index_in_sched.is_some() {
            mt_server_state.entities_update_scheduled.remove(index_in_sched.unwrap());
            aom_vector.push(wire::types::ActiveObjectMessage{
                id: *mt_server_state.entity_id_map.get_by_left(&entity_id.0).unwrap(),
                data: wire::types::ActiveObjectCommand::UpdatePosition(
                    wire::types::AOCUpdatePosition {
                        position: utils::vec3_to_v3f(position, 0.1),
                        velocity: utils::vec3_to_v3f(&physics.velocity, 0.0025),
                        acceleration: V3F_ZERO,
                        rotation: v3f {
                            x: look_direction.x_rot,
                            y: look_direction.y_rot,
                            z: 0.0
                        },
                        do_interpolate: false,
                        is_end_position: false,
                        update_interval: 1.0
                    }
                )
            });
            if aom_vector.len() == 20 {
                chunks.push(aom_vector);
                aom_vector = Vec::new()
            }
        }
    };
    drop(ecs); // we need to drop the ECS as soon a possible to not cause locks
    // for each entity not in the ECS (weird unloading bs can happen)
    for serverside_id in mt_server_state.entities_update_scheduled.drain(..) {
        let clientside_id = mt_server_state.entity_id_map.get_by_left(&serverside_id).unwrap();
        let meta_entry = mt_server_state.entity_meta_map.get(&serverside_id).unwrap();
        let position: v3f = utils::vec3_to_v3f(&meta_entry.position, 0.1);
        let velocity: v3f = utils::vec3_to_v3f(&meta_entry.velocity, 0.0025);
        let r: (f32, f32) = (
            meta_entry.rotation.0 as f32,
            meta_entry.rotation.1 as f32
        );
        aom_vector.push(wire::types::ActiveObjectMessage{
            id: *clientside_id,
            data: wire::types::ActiveObjectCommand::UpdatePosition(
                wire::types::AOCUpdatePosition {
                    position,
                    velocity,
                    acceleration: V3F_ZERO,
                    rotation: v3f {
                        x: r.0,
                        y: r.1,
                        z: 0.0
                    },
                    do_interpolate: false,
                    is_end_position: true,
                    update_interval: 1.0
                }
            )
        });
        if aom_vector.len() == 20 {
            chunks.push(aom_vector);
            aom_vector = Vec::new();
        }
    }
    // sending all updates at once can exceed minetests packet processing budget
    // send at most 20/packet
    for aom_vector in chunks {
        let clientbound_moveentity = ToClientCommand::ActiveObjectMessages(
            Box::new(wire::command::ActiveObjectMessagesSpec{
                objects: aom_vector
            })
        );
        let _ = mt_conn.send(clientbound_moveentity).await;
    }

    // sync air supply to client
    let air_supply: azalea::entity::metadata::AirSupply = mc_client.component();
    // format of air_supply: 0 - 299
    // 0 -> 0 bubbles displayed
    // 299 -> 20 bubbles
    let approx_bubble_count: u32 = {
        air_supply.abs() as f32 / 14.95
    }.round() as u32;
    if approx_bubble_count != mt_server_state.mc_last_air_supply {
        clientbound_translator::edit_airbar(approx_bubble_count, mt_conn, mt_server_state.mc_last_air_supply).await;
        mt_server_state.mc_last_air_supply = approx_bubble_count;
    };

    // check for sprinting/sneaking, change client movement speed if needed
    let sprinting: azalea::entity::metadata::Sprinting = mc_client.component();
    if sprinting.0 && mt_server_state.is_sneaking {mt_server_state.is_sneaking = false}
    // TODO: soul sand, cobwebs etc may also change player speed
    let current_speed: f32 = match (sprinting.0, mt_server_state.is_sneaking) {
        (false, false) => 4.317,
        (false, true) => 1.295,
        (true, false) => 5.612,
        (true, true) => {
            mt_server_state.is_sneaking = false;
            5.612
        }
    };
    if current_speed != mt_server_state.mt_current_speed {
        mt_server_state.mt_current_speed = current_speed;
        let _ = mt_conn.send(mt_definitions::get_movementspec(current_speed)).await;
    }
}
