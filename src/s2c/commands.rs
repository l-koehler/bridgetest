use crate::s2c;
use crate::state;
use crate::utils;

use luanti_protocol::LuantiConnection;

use log::*;

use azalea::events::Event;
use azalea::protocol::packets::game::ClientboundGamePacket;
use std::sync::Arc;

pub async fn process(
    command: Event,
    luanti_conn: &mut LuantiConnection,
    mc_client: &mut azalea::Client,
    proxy_state: &mut state::ProxyState,
) {
    match command {
        Event::AddPlayer(player_data) => {
            s2c::player::add_player(player_data, luanti_conn, &mut proxy_state.player).await
        }
        Event::Chat(message) => s2c::chat::send_message(luanti_conn, message).await,
        Event::Tick => (), // our on-tick actions are handled by a separate timer
        Event::Death(_) => {
            s2c::player::death(luanti_conn, &mut proxy_state.player, &mc_client).await
        }
        Event::Packet(packet_value) => match Arc::unwrap_or_clone(packet_value) {
            ClientboundGamePacket::BundleDelimiter(_) => (),

            ClientboundGamePacket::ChunkBatchStart(_) => {
                s2c::world::chunk_batch_start(&mut proxy_state.chunk_batch)
            }
            ClientboundGamePacket::ChunkBatchFinished(_) => {
                s2c::world::chunk_batch_finished(&mut proxy_state.chunk_batch)
            }
            ClientboundGamePacket::LevelChunkWithLight(chunk_packet) => {
                s2c::world::send_level_chunk(
                    &chunk_packet,
                    luanti_conn,
                    &mut proxy_state.player,
                    &mut proxy_state.light,
                    &proxy_state.media,
                    &mut proxy_state.particles,
                )
                .await
            }
            ClientboundGamePacket::LightUpdate(lightupdate_packet) => {
                s2c::world::light_update(
                    &lightupdate_packet,
                    luanti_conn,
                    &proxy_state.player,
                    mc_client,
                    &mut proxy_state.light,
                    &proxy_state.media,
                    &mut proxy_state.particles,
                )
                .await
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
            // Respawn (re)*SPAWNS* the player in a dimension and is not only sent on death!
            ClientboundGamePacket::SetDefaultSpawnPosition(setspawn_packet) => {
                s2c::player::set_spawn(&setspawn_packet, &mut proxy_state.player).await
            }
            ClientboundGamePacket::Respawn(respawn_packet) => {
                s2c::player::update_dimension(
                    &respawn_packet,
                    &mut proxy_state.player,
                    luanti_conn,
                    &mut proxy_state.light,
                    &mut proxy_state.particles,
                )
                .await
            }

            ClientboundGamePacket::KeepAlive(_) => trace!("Got S2C KeepAlive packet, ignoring it."),
            ClientboundGamePacket::AddEntity(addentity_packet) => {
                s2c::entities::add_entity(
                    s2c::entities::EAddType::Entity(addentity_packet),
                    luanti_conn,
                    &mut proxy_state.entities,
                    mc_client,
                    &proxy_state.media,
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
            ClientboundGamePacket::RotateHead(rotatehead_packet) => {
                s2c::entities::entity_rotate_head(
                    &rotatehead_packet,
                    &mut proxy_state.entities,
                    mc_client,
                )
                .await
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
            ClientboundGamePacket::DamageEvent(damage_packet) => {
                s2c::entities::damage_flash(
                    &damage_packet.entity_id,
                    &proxy_state.entities,
                    luanti_conn,
                )
                .await
            }
            ClientboundGamePacket::SetEntityData(data_packet) => {
                s2c::entities::set_entity_data(&data_packet, &mut proxy_state.entities).await
            }

            ClientboundGamePacket::OpenScreen(screen_packet) => {
                s2c::inventory::open_screen(&screen_packet, luanti_conn, &mut proxy_state.inventory)
                    .await
            }

            ClientboundGamePacket::BlockUpdate(blockupdate_packet) => {
                s2c::world::blockupdate(
                    &blockupdate_packet,
                    luanti_conn,
                    &proxy_state.player,
                    &proxy_state.light,
                    &proxy_state.media,
                    &mut proxy_state.particles,
                )
                .await
            }

            ClientboundGamePacket::SectionBlocksUpdate(sectionupdate_packet) => {
                s2c::world::section_block_update(
                    &sectionupdate_packet,
                    luanti_conn,
                    &proxy_state.player,
                    mc_client,
                    &mut proxy_state.light,
                    &proxy_state.media,
                    &mut proxy_state.particles,
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
            ClientboundGamePacket::ContainerSetSlot(_)
            | ClientboundGamePacket::ContainerSetData(_)
            | ClientboundGamePacket::ContainerSetContent(_) => {
                s2c::inventory::refresh_inv(
                    mc_client,
                    luanti_conn,
                    &mut proxy_state.inventory,
                    false,
                )
                .await
            }
            ClientboundGamePacket::ContainerClose(close_packet) => {
                s2c::inventory::server_closed_container(
                    &close_packet,
                    luanti_conn,
                    &mut proxy_state.inventory,
                )
                .await
            }
            ClientboundGamePacket::LevelParticles(particle_packet) => {
                s2c::particles::level_particles(&particle_packet, luanti_conn).await
            }
            ClientboundGamePacket::LevelEvent(levelevent_packet) => {
                s2c::particles::level_event(
                    &levelevent_packet,
                    luanti_conn,
                    &proxy_state.player,
                    &proxy_state.light,
                    &proxy_state.media,
                )
                .await
            }
            other => warn!(
                "Got unimplemented S2C ClientboundGamePacket, dropping {}",
                utils::mc_game_packet_name(&other)
            ),
        },
        other => warn!(
            "Got unimplemented S2C command, dropping {}",
            utils::mc_packet_name(&other)
        ),
    };
}
