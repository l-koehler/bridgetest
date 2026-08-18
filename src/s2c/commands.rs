use crate::s2c;
use crate::state;
use crate::utils;

use luanti_protocol::LuantiConnection;

use log::*;

use azalea::events::Event;
use azalea::protocol::packets::game::ClientboundGamePacket;
use tokio::sync::mpsc::UnboundedReceiver;

pub async fn process(
    command: Event,
    luanti_conn: &mut LuantiConnection,
    mc_client: &mut azalea::Client,
    proxy_state: &mut state::ProxyState,
    mc_conn: &mut UnboundedReceiver<Event>,
) {
    let cloned_command = command.clone();
    let command_name = utils::mc_packet_name(&cloned_command);
    match command {
        Event::AddPlayer(player_data) => {
            s2c::player::add_player(player_data, luanti_conn, &mut proxy_state.player).await
        }
        Event::Chat(message) => s2c::chat::send_message(luanti_conn, message).await,
        Event::Tick => (), // our on-tick actions are handled by a separate timer
        Event::Death(_) => {
            s2c::player::death(luanti_conn, &mut proxy_state.player, &mc_client).await
        }
        Event::Packet(packet_value) => match (*packet_value).clone() {
            ClientboundGamePacket::BundleDelimiter(_) => (),

            ClientboundGamePacket::ChunkBatchStart(_) => {
                s2c::world::chunkbatch(luanti_conn, mc_conn, &mut proxy_state.player).await
            }
            ClientboundGamePacket::SystemChat(message) => {
                s2c::chat::send_sys_message(luanti_conn, &message).await
            }
            ClientboundGamePacket::PlayerPosition(playerpos_packet) => {
                s2c::player::set_player_pos(&playerpos_packet, luanti_conn, &mut proxy_state.player)
                    .await
            }
            ClientboundGamePacket::SetTime(settime_packet) => {
                s2c::world::set_time(&settime_packet, luanti_conn, &mut proxy_state.time).await
            }
            ClientboundGamePacket::SetHealth(sethealth_packet) => {
                s2c::player::set_health(&sethealth_packet, luanti_conn, &mut proxy_state.player)
                    .await
            }
            // these two are misleading. SetDefaultSpawnPosition sets the on-death respawn position,
            // Respawn (re)*SPAWNS* the player in a different dimension and is entirely unrelated to death!
            ClientboundGamePacket::SetDefaultSpawnPosition(setspawn_packet) => {
                s2c::player::set_spawn(&setspawn_packet, &mut proxy_state.player).await
            }
            ClientboundGamePacket::Respawn(respawn_packet) => {
                s2c::player::update_dimension(&respawn_packet, &mut proxy_state.player).await
            }

            ClientboundGamePacket::KeepAlive(_) => trace!("Got S2C KeepAlive packet, ignoring it."),
            ClientboundGamePacket::AddEntity(addentity_packet) => {
                s2c::entities::add_entity(
                    s2c::entities::EAddType::Entity(addentity_packet),
                    luanti_conn,
                    &mut proxy_state.entities,
                )
                .await
            }
            ClientboundGamePacket::MoveEntityPos(entitypos_packet) => {
                s2c::entities::entity_setpos(&entitypos_packet, &mut proxy_state.entities).await
            }
            ClientboundGamePacket::TeleportEntity(entitytp_packet) => {
                s2c::entities::entity_teleport(&entitytp_packet, &mut proxy_state.entities).await
            }
            ClientboundGamePacket::MoveEntityPosRot(entityposrot_packet) => {
                s2c::entities::entity_setposrot(&entityposrot_packet, &mut proxy_state.entities)
                    .await
            }
            ClientboundGamePacket::MoveEntityRot(entityrot_packet) => {
                s2c::entities::entity_setrot(&entityrot_packet, &mut proxy_state.entities).await
            }
            ClientboundGamePacket::SetEntityMotion(entitymotion_packet) => {
                s2c::entities::entity_setmotion(&entitymotion_packet, &mut proxy_state.entities)
                    .await
            }
            ClientboundGamePacket::EntityPositionSync(entitysync_packet) => {
                s2c::entities::entity_sync(&entitysync_packet, &mut proxy_state.entities)
            }
            // would need a better implementation of models and bones than this
            ClientboundGamePacket::RotateHead(_) => {
                trace!("Got S2C RotateHead packet, ignoring it.")
            }
            // should mostly not matter, server-controlled stuff
            ClientboundGamePacket::UpdateAttributes(_) => {
                trace!("Got S2C UpdateAttributes, ignoring it.")
            }
            ClientboundGamePacket::RemoveEntities(removeentity_packet) => {
                s2c::entities::remove_entity(
                    &removeentity_packet,
                    luanti_conn,
                    &mut proxy_state.entities,
                )
                .await
            }

            ClientboundGamePacket::EntityEvent(event_packet) => {
                s2c::entities::entity_event(&event_packet, luanti_conn, mc_client).await
            }
            ClientboundGamePacket::SetEntityData(data_packet) => {
                s2c::entities::set_entity_data(
                    &data_packet,
                    luanti_conn,
                    &proxy_state.entities,
                    &proxy_state.media,
                    mc_client,
                )
                .await
            }

            ClientboundGamePacket::OpenScreen(screen_packet) => {
                s2c::inventory::open_screen(&screen_packet, luanti_conn, &mut proxy_state.inventory)
                    .await
            }

            ClientboundGamePacket::BlockUpdate(blockupdate_packet) => {
                s2c::world::blockupdate(&blockupdate_packet, luanti_conn, &proxy_state.player).await
            }

            ClientboundGamePacket::SectionBlocksUpdate(sectionupdate_packet) => {
                s2c::world::section_block_update(
                    &sectionupdate_packet,
                    luanti_conn,
                    &proxy_state.player,
                    mc_client,
                )
                .await
            }
            ClientboundGamePacket::Sound(sound_packet) => {
                s2c::chat::show_sound(&sound_packet, &mut proxy_state.chat)
            }
            ClientboundGamePacket::UpdateMobEffect(mobeffect_packet) => {
                s2c::entities::update_mob_effect(
                    &mobeffect_packet,
                    &mut proxy_state.player,
                    luanti_conn,
                    mc_client,
                )
                .await
            }
            ClientboundGamePacket::RemoveMobEffect(mobeffect_packet) => {
                s2c::entities::remove_mob_effect(
                    &mobeffect_packet,
                    luanti_conn,
                    &mut proxy_state.player,
                    mc_client,
                )
                .await
            }
            // use on-tick and azalea abstractions for containers
            ClientboundGamePacket::ContainerSetSlot(_) => (),
            ClientboundGamePacket::ContainerSetData(_) => (),
            ClientboundGamePacket::ContainerSetContent(_) => (),
            ClientboundGamePacket::ContainerClose(_) => (),
            _ => warn!(
                "Got unimplemented S2C ClientboundGamePacket, dropping {}",
                command_name
            ),
        },
        _ => warn!("Got unimplemented S2C command, dropping {}", command_name),
    };
}
