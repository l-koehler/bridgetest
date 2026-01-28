// unsurprisingly has absolutely nothing to do with translator.rs
// i am terrible at naming things
// anyways this contains functions that TAKE data from the minecraft server
// and send it to the minetest client.

extern crate alloc;

use crate::settings;
use crate::state;
use azalea::entity::MobEffectData;
use state::world::Dimensions;

use azalea::BlockPos;
use azalea::registry::builtin::MobEffect;
use glam::Vec3 as v3f;
use log::*;

use luanti_protocol::LuantiConnection;
use luanti_protocol::commands::server_to_client;
use luanti_protocol::commands::server_to_client::ToClientCommand;

use azalea::Client;
use azalea::player::PlayerInfo;

use azalea::protocol::packets::game::{
    c_player_position::ClientboundPlayerPosition, c_respawn::ClientboundRespawn,
    c_set_default_spawn_position::ClientboundSetDefaultSpawnPosition,
    c_set_health::ClientboundSetHealth,
};

use azalea::protocol::packets::common::CommonPlayerSpawnInfo;
use std::time::Instant;

use crate::s2c;
use s2c::defs::{FoodDisplay, HeartDisplay};

pub async fn update_dimension(
    source_packet: &ClientboundRespawn,
    player_state: &mut state::PlayerState,
) {
    let ClientboundRespawn {
        common: player_spawn_info,
        data_to_keep: _,
    } = source_packet;
    let CommonPlayerSpawnInfo {
        dimension_type: _,
        dimension,
        seed: _,
        sea_level: _,
        game_type: _,
        previous_game_type: _,
        is_debug: _,
        is_flat: _,
        last_death_location: _,
        portal_cooldown: _,
    } = player_spawn_info;
    if dimension.namespace() != "minecraft" {
        player_state.current_dimension = Dimensions::Custom;
    } else {
        player_state.current_dimension = match dimension.path() {
            "overworld" => Dimensions::Overworld,
            "the_nether" => Dimensions::Nether,
            "the_end" => Dimensions::End,
            _ => Dimensions::Custom,
        };
    }
    info!(
        "Client changed dimension: {}:{}",
        dimension.namespace(),
        dimension.path()
    )
}

pub async fn set_spawn(
    source_packet: &ClientboundSetDefaultSpawnPosition,
    player_state: &mut state::PlayerState,
) {
    let ClientboundSetDefaultSpawnPosition { global_pos, .. } = source_packet;
    let BlockPos { x, y, z } = global_pos.pos;
    let dest_x = x as f32;
    let dest_y = y as f32;
    let dest_z = z as f32;
    player_state.respawn_pos = (dest_x, dest_y, dest_z);
}

pub async fn add_player(
    player_data: PlayerInfo,
    conn: &mut LuantiConnection,
    player_state: &mut state::PlayerState,
) {
    let new_user: String = player_data.profile.name.to_string();
    //FIXME use ECS
    player_state.players.push(new_user);
    let add_player_command =
        ToClientCommand::UpdatePlayerList(Box::new(server_to_client::UpdatePlayerListSpec {
            typ: 0,
            players: player_state.players.clone(),
        }));
    debug!("Sending S2C UpdatePlayerList");
    conn.send(add_player_command).unwrap();
}

pub async fn death(
    conn: &LuantiConnection,
    player_state: &mut state::PlayerState,
    mc_client: &Client,
) {
    let respawn_pos = player_state.respawn_pos;

    let deathscreen = ToClientCommand::Deathscreen(Box::new(server_to_client::DeathscreenSpec {
        set_camera_point_target: false,
        camera_point_target: v3f::ZERO,
    }));

    // this event is basically the click on the "respawn" button
    // needed to update position
    mc_client
        .ecs
        .write()
        .write_message(azalea::respawn::PerformRespawnEvent {
            entity: mc_client.entity,
        });
    let setpos_packet = ToClientCommand::MovePlayer(Box::new(server_to_client::MovePlayerSpec {
        pos: v3f {
            x: mc_client.position().x as f32,
            y: mc_client.position().y as f32,
            z: mc_client.position().z as f32,
        },
        pitch: 0.0,
        yaw: 0.0,
    }));
    conn.send(setpos_packet).unwrap();
    player_state.mt_clientside_pos = (
        respawn_pos.0 * 10.0,
        respawn_pos.1 * 10.0,
        respawn_pos.2 * 10.0,
    );
    conn.send(deathscreen).unwrap();

    set_health(
        &ClientboundSetHealth {
            health: 20.0,
            food: 20,
            saturation: 0.0,
        },
        conn,
        player_state,
    )
    .await;
}

pub async fn edit_healthbar(mode: HeartDisplay, num: u32, conn: &LuantiConnection) {
    // num is from 0 to 20
    // above 20: no change will be made to the number of hearts
    let heart_texture: &str = match mode {
        HeartDisplay::Absorb => "gui-sprites-hud-heart-absorbing_full.png",
        HeartDisplay::Frozen => "gui-sprites-hud-heart-frozen_full.png",
        HeartDisplay::Normal => "gui-sprites-hud-heart-full.png",
        HeartDisplay::Poison => "gui-sprites-hud-heart-poisoned_full.png",
        HeartDisplay::Wither => "gui-sprites-hud-heart-withered_full.png",
        HeartDisplay::HardcoreAbsorb => "gui-sprites-hud-heart-absorbing_hardcore_full.png",
        HeartDisplay::HardcoreFrozen => "gui-sprites-hud-heart-frozen_hardcore_full.png",
        HeartDisplay::HardcoreNormal => "gui-sprites-hud-heart-hardcore_full.png",
        HeartDisplay::HardcorePoison => "gui-sprites-hud-heart-poisoned_hardcore_full.png",
        HeartDisplay::HardcoreWither => "gui-sprites-hud-heart-withered_hardcore_full.png",
        HeartDisplay::Vehicle => "gui-sprites-hud-heart-vehicle_full.png",
        HeartDisplay::NoChange => "",
    };
    if !heart_texture.is_empty() {
        let set_bar_texture =
            ToClientCommand::Hudchange(Box::new(server_to_client::HudchangeCommand {
                server_id: s2c::defs::HEALTHBAR_ID,
                stat: server_to_client::HudStat::Text(String::from(heart_texture)),
            }));
        conn.send(set_bar_texture).unwrap();
    }
    if num < 20 {
        let set_bar_number =
            ToClientCommand::Hudchange(Box::new(server_to_client::HudchangeCommand {
                server_id: s2c::defs::HEALTHBAR_ID,
                stat: server_to_client::HudStat::Number(num),
            }));
        conn.send(set_bar_number).unwrap();
    }
}

pub async fn edit_foodbar(mode: FoodDisplay, num: u32, conn: &LuantiConnection) {
    let food_texture: &str = match mode {
        FoodDisplay::Normal => "gui-sprites-hud-food_full.png",
        FoodDisplay::Hunger => "gui-sprites-hud-food_full_hunger.png",
        FoodDisplay::NoChange => "",
    };
    if !food_texture.is_empty() {
        let set_bar_texture =
            ToClientCommand::Hudchange(Box::new(server_to_client::HudchangeCommand {
                server_id: s2c::defs::FOODBAR_ID,
                stat: server_to_client::HudStat::Text(String::from(food_texture)),
            }));
        conn.send(set_bar_texture).unwrap();
    }
    if num < 21 {
        let set_bar_number =
            ToClientCommand::Hudchange(Box::new(server_to_client::HudchangeCommand {
                server_id: s2c::defs::FOODBAR_ID,
                stat: server_to_client::HudStat::Number(num),
            }));
        conn.send(set_bar_number).unwrap();
    }
}

pub async fn edit_airbar(num: u32, conn: &LuantiConnection, prev_num: u32) {
    // num is count of half bubbles (between 0 and 20)
    // we reformat it to look good despite formspec
    let number = num - (num % 2);
    let item = num + (num % 2);
    let p_item = prev_num + (prev_num % 2);
    let set_bar_number: ToClientCommand =
        ToClientCommand::Hudchange(Box::new(server_to_client::HudchangeCommand {
            server_id: s2c::defs::AIRBAR_ID,
            stat: server_to_client::HudStat::Number(number),
        }));
    if item != p_item {
        // item count only needs to get updated every other change
        let set_bar_item: ToClientCommand =
            ToClientCommand::Hudchange(Box::new(server_to_client::HudchangeCommand {
                server_id: s2c::defs::AIRBAR_ID,
                stat: server_to_client::HudStat::Item(item),
            }));
        conn.send(set_bar_item).unwrap();
    };
    conn.send(set_bar_number).unwrap();
}

pub async fn update_effects(
    luanti_conn: &mut LuantiConnection,
    effects: &Vec<(MobEffect, Instant, MobEffectData)>,
) {
    let mut y_offset = 0;
    let mut combined_texture = format!("[combine:24x{}", effects.len() * 30);
    if effects.is_empty() {
        combined_texture = String::from("");
    }
    // we don't dedup the list itself, but refuse to draw the same icon twice
    // there _could_ be the same effect twice with different levels.
    // vanilla servers won't do that, but i can't prove it won't happen
    let mut prevent_dup: Vec<MobEffect> = Vec::new();
    for effect_t in effects {
        let (effect, _, data) = effect_t;
        let flags = &data.flags;
        if prevent_dup.contains(effect) {
            warn!("Found duplicate effects in player_state.client_effects!");
            continue;
        } else {
            prevent_dup.push(*effect);
        }
        // 0x01: is ambient effect
        // 0x02: show particles
        // 0x04: show icon
        // 0x08: some darkness-specific thing
        if !flags.show_icon {
            continue;
        };
        let frame_icon = match flags.ambient {
            false => "gui-sprites-hud-effect_background.png",
            true => "gui-sprites-hud-effect_background_ambient.png",
        };
        let mut effect_icon = format!("{:?}", effect).replace("MobEffect::", "");
        effect_icon = effect_icon
            .into_chars()
            .map(|c| {
                if c.is_uppercase() {
                    format!("_{}", c.to_lowercase())
                } else {
                    c.to_string()
                }
            })
            .collect();
        effect_icon.remove(0);
        let texture = format!(
            ":0,{}=({}^mob_effect-{}.png)",
            y_offset, frame_icon, effect_icon
        );
        combined_texture.push_str(&texture);
        y_offset += 30; // 6px padding
    }
    trace!("New effect texture: {}", combined_texture);
    let upd_texture = ToClientCommand::Hudchange(Box::new(server_to_client::HudchangeCommand {
        server_id: s2c::defs::EFFECTS_ID,
        stat: server_to_client::HudStat::Text(combined_texture),
    }));
    luanti_conn.send(upd_texture).unwrap();
}

pub async fn set_health(
    source_packet: &ClientboundSetHealth,
    conn: &LuantiConnection,
    player_state: &mut state::PlayerState,
) {
    let ClientboundSetHealth {
        health,
        food,
        saturation: _,
    } = source_packet;
    // health: 0..20
    let new_health: u16 = *health as u16;
    let mut damage_effect: Option<bool> = None;
    if player_state.mt_last_known_health > new_health {
        // health dropped since last time this was run
        damage_effect = Some(true);
    }
    player_state.mt_last_known_health = new_health;

    let sethealth_packet = ToClientCommand::Hp(Box::new(
        luanti_protocol::commands::server_to_client::HpSpec {
            hp: new_health,
            damage_effect,
        },
    ));
    conn.send(sethealth_packet).unwrap();
    edit_healthbar(HeartDisplay::NoChange, new_health.into(), conn).await;
    edit_foodbar(FoodDisplay::NoChange, *food, conn).await;
}

pub async fn set_player_pos(
    source_packet: &ClientboundPlayerPosition,
    conn: &LuantiConnection,
    player_state: &mut state::PlayerState,
) {
    let ClientboundPlayerPosition {
        id: _,
        change,
        relative: _,
    } = source_packet;

    let dest_x = change.pos.x as f32 * 10.0;
    let dest_y = change.pos.y as f32 * 10.0;
    let dest_z = change.pos.z as f32 * 10.0;

    let setpos_packet = ToClientCommand::MovePlayer(Box::new(server_to_client::MovePlayerSpec {
        pos: v3f {
            x: dest_x,
            y: dest_y,
            z: dest_z,
        },
        pitch: change.look_direction.x_rot(),
        yaw: change.look_direction.y_rot(),
    }));
    conn.send(setpos_packet).unwrap();
    player_state.mt_clientside_pos = (dest_x, dest_y, dest_z);
    player_state.client_rotation = (change.look_direction.y_rot(), change.look_direction.x_rot());
}

pub async fn sync_client_pos(
    mc_client: &Client,
    conn: &mut LuantiConnection,
    player_state: &mut state::PlayerState,
) {
    let vec_serverpos = mc_client.position();
    // some collision box weirdness on block edges
    // -0.5 fixes it, don't touch
    let serverpos = (
        vec_serverpos.x as f32 - 0.5,
        vec_serverpos.y as f32,
        vec_serverpos.z as f32 - 0.5,
    );
    let clientpos = player_state.mt_clientside_pos;
    // we count height as half, otherwise jumping is noticeably broken
    let x_y_euclid_diff: f32 = {
        ((serverpos.0 - clientpos.0).abs().powi(2) + (serverpos.2 - clientpos.2).abs().powi(2))
            .sqrt()
    };
    let distance =
        { (x_y_euclid_diff.powi(2) + ((serverpos.1 - clientpos.1).abs() / 2.0).powi(2)).sqrt() };

    if distance > settings::POS_DIFF_TOLERANCE {
        trace!("Re-Syncing Player Position: {} difference", distance);
        let setpos_packet =
            ToClientCommand::MovePlayer(Box::new(server_to_client::MovePlayerSpec {
                pos: v3f {
                    x: serverpos.0 * 10.0,
                    y: serverpos.1 * 10.0,
                    z: serverpos.2 * 10.0,
                },
                pitch: player_state.client_rotation.1,
                yaw: player_state.client_rotation.0,
            }));
        conn.send(setpos_packet).unwrap();
        player_state.mt_clientside_pos = serverpos;
    }
}
