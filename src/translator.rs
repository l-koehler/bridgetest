use crate::c2s;
use crate::handshake;
use crate::s2c;
use crate::state;
use crate::state::world::Dimensions;

use azalea::world::WorldName;
use luanti_protocol::LuantiConnection;
use luanti_protocol::LuantiServer;
use luanti_protocol::commands::CommandProperties;
use luanti_protocol::commands::client_to_server::ToServerCommand;
use luanti_protocol::peer::PeerError;

use config::Config;
use log::*;
use std::time::Duration;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::IntervalStream;

pub async fn client_handler(
    _mt_server: LuantiServer,
    mut luanti_conn: LuantiConnection,
    settings: Config,
) {
    // initialize global state
    let mut proxy_state = state::ProxyState::default();
    proxy_state.media.item_texture_map = s2c::media::load_item_mappings();
    proxy_state.media.nodebox_lookup = s2c::media::load_nodeboxes();
    proxy_state.media.block_texture_map =
        s2c::media::load_block_mappings(&proxy_state.media.nodebox_lookup);

    /*
     * The first few packets (handshake) are outside the main loop, because
     * at this point the minecraft client isn't initialized yet.
     */
    let (mut mc_client, mut mc_conn, player_name) =
        handshake::handshake(&mut luanti_conn, &settings).await;
    debug!("Sending S2C ActiveObjectRemoveAdd (add LocalPlayer)");
    s2c::defs::register_media(&mut luanti_conn);

    s2c::defs::register_items(&mut luanti_conn, &proxy_state.media).await;
    s2c::defs::register_nodes(&mut luanti_conn, &mut proxy_state.media, &settings).await;
    s2c::defs::register_rules(&mut luanti_conn);

    info!("Awaiting C2S ClientReady");
    loop {
        let t = luanti_conn.recv().await;
        let command = t.unwrap();
        match command {
            ToServerCommand::RequestMedia(packet) => {
                luanti_conn
                    .send(s2c::media::handle_request(packet))
                    .unwrap();
            }
            ToServerCommand::ClientReady(_) => {
                debug!("Got C2S ClientReady");
                break;
            }
            _ => warn!(
                "Dropping unexpected C2S packet! Got serverbound \"{}\", expected \"ClientReady\"",
                command.command_name()
            ),
        }
    }

    s2c::defs::register_misc_late(&mut luanti_conn);
    s2c::entities::add_entity(
        s2c::entities::EAddType::Player(player_name),
        &mut luanti_conn,
        &mut proxy_state.entities,
    )
    .await;
    // set dimension before parsing chunks
    let worldname = mc_client.query_self::<&WorldName, _>(|r| r.0.clone());
    proxy_state.player.current_dimension = match worldname.path() {
        "the_end" => Dimensions::End,
        "overworld" => Dimensions::Overworld,
        "nether" => Dimensions::Nether,
        _ => {
            warn!("Got unknown dimension: {:?}", worldname.path());
            Dimensions::Custom
        }
    };
    /*
     * Main Loop.
     * At this point, both the luanti client and the minecraft server
     * are connected.
     * luanti_conn refers to the connection to the luanti client
     * mc_client and mc_conn refer to the minecraft client and a event receiver
     * we also run a tick function every 50ms
     */
    let mut stream = IntervalStream::new(tokio::time::interval(Duration::from_millis(50)));
    loop {
        tokio::select! {
            // recieve data over the LuantiConnection
            t = luanti_conn.recv() => {
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
                            error!("Client Disconnected: {:?}", err);
                        } else {
                            println!("Client Disconnected");
                        }
                        // Exit the client handler on client disconnect
                        break;
                    }
                }
                c2s::process(t.unwrap(), &mut luanti_conn, &mut mc_client, &mut proxy_state).await;
            },
            t = mc_conn.recv() => {
                s2c::process(t.unwrap(), &mut luanti_conn, &mut mc_client, &mut proxy_state, &mut mc_conn).await;
            },
            _ = stream.next() => {
                s2c::tick(&mut luanti_conn, &mut mc_client, &mut proxy_state).await;
                c2s::tick(&mut luanti_conn, &mut mc_client, &mut proxy_state).await;
            }
        }
    }
}
