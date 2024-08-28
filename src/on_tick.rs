use minetest_protocol::MinetestConnection;
use azalea_client::Client;
use crate::MTServerState;
use crate::clientbound_translator;
use crate::utils;
use crate::mt_definitions::EntityResendableData;
use minetest_protocol::wire::types::v3f;
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
}
