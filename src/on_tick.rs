use minetest_protocol::MinetestConnection;
use azalea_client::Client;
use crate::MTServerState;
use crate::clientbound_translator;
use crate::utils;
use crate::mt_definitions::EntityResendableData;
use minetest_protocol::wire::types::v3f;

pub async fn server(mt_conn: &mut MinetestConnection, mc_client: &Client, mt_server_state: &mut MTServerState) {
    if mt_server_state.has_moved_since_sync {
        clientbound_translator::sync_client_pos(mc_client, mt_conn, mt_server_state).await;
        mt_server_state.has_moved_since_sync = false;
    }
    // update the MT clients inventory if it changed
    // for stupid reasons, we don't use packets for this, instead on every tick
    // and whenever the player crafted something
    clientbound_translator::refresh_inv(mc_client, mt_conn, mt_server_state).await;
    // move every entity according to a position delta
    let thing = mt_server_state.entity_velocity_tracker.clone().into_iter();
    let mut data: Vec<(u16, EntityResendableData)> = Vec::new();
    for entity in thing {
        let id = entity.0;
        let delta = entity.1;
        if !mt_server_state.entity_id_pos_map.contains_key(id.into()){
            utils::logger(&format!("[Minetest] Failed to update position for (adjusted) entity ID {}: ID not yet present, dropping the packet!", id), 2);
            return
        }
        let entitydata = mt_server_state.entity_id_pos_map.get(id.into()).unwrap();
        let EntityResendableData {
            position: old_position,
            rotation,
            velocity,
            acceleration,
            entity_kind
        } = entitydata.clone();

        // MT: velocity as floats nodes/second
        // MC: velocity as int diff*4096
        let position = v3f {
            x: old_position.x + delta.x,
            y: old_position.y + delta.y,
            z: old_position.z + delta.z
        };
        let new_data = EntityResendableData {
            position, rotation,
            velocity,
            acceleration, entity_kind
        };
        let _ = mt_server_state.entity_id_pos_map.insert(id.into(), new_data.clone());
        data.push((id, new_data));
    }
    clientbound_translator::send_multi_entity_data(data, mt_conn).await;
}
