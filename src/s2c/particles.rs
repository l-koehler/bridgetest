use crate::state;
use crate::utils;
use azalea::BlockPos;
use azalea::block::BlockState;
use azalea::entity::particle::Particle;
use azalea::protocol::packets::game::ClientboundLevelParticles;
use azalea::protocol::packets::game::c_level_event::ClientboundLevelEvent;
use azalea::registry::builtin::BlockKind;
use glam::Vec3 as v3f;
use log::*;
use luanti_core::MapNode;
use luanti_protocol::{
    LuantiConnection,
    commands::server_to_client::{
        self, AddParticlespawnerCommand, Attractor, CommonParticleParams, ParticleParameters,
        ParticleTexture, ServerParticleTexture, ToClientCommand, TweenStyle, TweenedParameter,
    },
    types::{RangedParameter, TileAnimationParams},
};
use rand::RngExt;

pub async fn level_particles(packet_data: &ClientboundLevelParticles, conn: &mut LuantiConnection) {
    let ClientboundLevelParticles {
        override_limiter: _,
        always_show: _,
        pos,
        x_dist: _,
        y_dist: _,
        z_dist: _,
        max_speed: _,
        count: _,
        particle,
    } = packet_data;
    let (texture_str, size) = match (*particle) {
        Particle::AngryVillager => ("particle-angry.png", 1.0),
        // skip item lookup stuff
        Particle::Block(_)
        | Particle::BlockMarker(_)
        | Particle::FallingDust(_)
        | Particle::DustPlume
        | Particle::DustPillar
        | Particle::BlockCrumble
        | Particle::Item(_)
        | Particle::ItemSlime
        | Particle::ItemCobweb
        | Particle::ItemSnowball => ("particle-generic_7.png", 1.0),
        Particle::Bubble | Particle::CurrentDown | Particle::BubbleColumnUp => {
            ("particle-bubble.png", 1.0)
        }
        Particle::SulfurBubbles => ("particle-bubble_white.png", 1.0),
        Particle::NoxiousGas | Particle::NoxiousGasCloud => ("particle-noxious_gas_01.png", 1.0),
        Particle::Geyser(_) | Particle::GeyserPlume(_) => ("particle-geyser_plume_01.png", 1.0),
        Particle::GeyserBase(_) => ("particle-geyser_base_01.png", 1.0),
        Particle::GeyserPoof(_) => ("particle-geyser_poof_01.png", 1.0),
        Particle::Cloud => ("particle-generic_7.png", 1.0),
        Particle::CopperFireFlame => ("particle-copper_fire_flame.png", 1.0),
        Particle::Crit => ("particle-critical_hit.png", 1.0),
        Particle::DamageIndicator => ("particle-damage.png", 1.0),
        Particle::DragonBreath => ("particle-generic_5.png", 1.0),
        Particle::DrippingLava
        | Particle::DrippingWater
        | Particle::DrippingHoney
        | Particle::DrippingObsidianTear
        | Particle::DrippingDripstoneLava
        | Particle::DrippingDripstoneWater => ("particle-drip_hang.png", 1.0),
        Particle::FallingLava
        | Particle::FallingHoney
        | Particle::FallingNectar
        | Particle::FallingObsidianTear
        | Particle::FallingSporeBlossom
        | Particle::SporeBlossomAir
        | Particle::FallingDripstoneLava
        | Particle::FallingDripstoneWater => ("particle-drip_fall.png", 1.0),
        Particle::LandingLava | Particle::LandingHoney | Particle::LandingObsidianTear => {
            ("particle-drip_land.png", 1.0)
        }
        Particle::FallingWater => ("particle-drip_fall.png", 1.0),
        Particle::Dust(_) | Particle::DustColorTransition(_) => ("particle-generic_7.png", 1.0),
        Particle::Effect | Particle::EntityEffect(_) => ("particle-effect_7.png", 1.0),
        // screen overlay in vanilla, no proper particle
        Particle::ElderGuardian => ("particle-generic_0.png", 1.0),
        Particle::EnchantedHit => ("particle-enchanted_hit.png", 1.0),
        Particle::Enchant => ("particle-sga_a.png", 1.0),
        Particle::EndRod | Particle::TotemOfUndying => ("particle-glitter_7.png", 1.0),
        Particle::ExplosionEmitter | Particle::Explosion => ("particle-explosion_0.png", 1.0),
        Particle::Gust | Particle::GustEmitterLarge => ("particle-gust_0.png", 1.0),
        Particle::SmallGust | Particle::GustEmitterSmall => ("particle-small_gust_0.png", 1.0),
        Particle::SonicBoom => ("particle-sonic_boom_0.png", 1.0),
        Particle::Firework => ("particle-spark_7.png", 1.0),
        Particle::Fishing | Particle::Rain | Particle::Splash => ("particle-splash_0.png", 1.0),
        Particle::Flame | Particle::SmallFlame => ("particle-flame.png", 1.0),
        Particle::Infested => ("particle-infested.png", 1.0),
        Particle::CherryLeaves => ("particle-cherry_0.png", 1.0),
        Particle::PaleOakLeaves => ("particle-pale_oak_0.png", 1.0),
        Particle::TintedLeaves => ("particle-leaf_0.png", 1.0),
        Particle::SculkSoul => ("particle-sculk_soul_0.png", 1.0),
        Particle::SculkCharge(_) => ("particle-sculk_charge_0.png", 1.0),
        Particle::SculkChargePop => ("particle-sculk_charge_pop_0.png", 1.0),
        Particle::SoulFireFlame => ("particle-soul_fire_flame.png", 1.0),
        Particle::Soul => ("particle-soul_0.png", 1.0),
        Particle::Flash => ("particle-flash.png", 1.0),
        Particle::HappyVillager
        | Particle::Composter
        | Particle::EggCrack
        | Particle::PauseMobGrowth
        | Particle::ResetMobGrowth => ("particle-glint.png", 1.0),
        Particle::Heart => ("particle-heart.png", 1.0),
        Particle::InstantEffect | Particle::Witch => ("particle-spell_7.png", 1.0),
        Particle::Vibration(_) => ("particle-vibration.png", 1.0),
        Particle::Trail | Particle::Portal | Particle::ReversePortal => {
            ("particle-generic_0.png", 1.0)
        }
        Particle::LargeSmoke
        | Particle::Smoke
        | Particle::WhiteSmoke
        | Particle::Sneeze
        | Particle::Snowflake
        | Particle::SquidInk
        | Particle::GlowSquidInk => ("particle-generic_7.png", 1.0),
        Particle::Lava => ("particle-lava.png", 1.0),
        Particle::Mycelium
        | Particle::Dolphin
        | Particle::CrimsonSpore
        | Particle::WarpedSpore
        | Particle::WhiteAsh
        | Particle::Underwater => ("particle-generic_0.png", 1.0),
        Particle::Ash => ("particle-generic_0.png", 1.0),
        Particle::Note => ("particle-note.png", 1.0),
        Particle::Poof => ("particle-generic_7.png", 1.0),
        Particle::BubblePop => ("particle-bubble_pop_0.png", 1.0),
        Particle::Nautilus => ("particle-nautilus.png", 1.0),
        Particle::CampfireCosySmoke | Particle::CampfireSignalSmoke => {
            ("particle-big_smoke_0.png", 1.0)
        }
        Particle::Spit => ("particle-generic_7.png", 1.0),
        Particle::SweepAttack => ("particle-sweep_0.png", 1.0),
        Particle::Glow
        | Particle::WaxOn
        | Particle::WaxOff
        | Particle::ElectricSpark
        | Particle::Scrape => ("particle-glow.png", 1.0),
        Particle::Shriek(_) => ("particle-shriek.png", 1.0),
        Particle::TrialSpawnerDetection => ("particle-trial_spawner_detection_0.png", 1.0),
        Particle::TrialSpawnerDetectionOminous => {
            ("particle-trial_spawner_detection_ominous_0.png", 1.0)
        }
        Particle::VaultConnection => ("particle-vault_connection.png", 1.0),
        Particle::OminousSpawning => ("particle-ominous_spawning.png", 1.0),
        Particle::RaidOmen => ("particle-raid_omen.png", 1.0),
        Particle::TrialOmen => ("particle-trial_omen.png", 1.0),
        Particle::Firefly => ("particle-firefly.png", 1.0),
        Particle::SulfurCubeGoo => ("particle-sulfur_cube_goo.png", 1.0),
    };
    trace!(
        "Got particle of type {:?} for {:?}. Using texture {}",
        particle, pos, texture_str
    );
    let particle_params: ParticleParameters = ParticleParameters {
        // unlike entity/player positions, ParticleParameters::pos is in raw node units
        // why do the luanti devs hate consistency so much :sob:
        pos: v3f {
            x: utils::mirror_pos(pos.x as f32),
            y: utils::align_pos(pos.y as f32),
            z: utils::align_pos(pos.z as f32),
        },
        vel: v3f::ZERO,
        acc: v3f::ZERO,
        expiration_time: 1000.0,
        size: size as f32,
        base: CommonParticleParams {
            collision_detection: false,
            vertical: false,
            collision_removal: false,
            animation: TileAnimationParams::None,
            glow: 0,
            object_collision: false,
            node: MapNode::default(),
            node_tile: 0,
            texture: ServerParticleTexture {
                base: ParticleTexture::default(),
                string: String::from(texture_str),
            },
        },
        drag: v3f::ZERO,
        jitter: RangedParameter {
            min: v3f::ZERO,
            max: v3f::ZERO,
            bias: 0.0,
        },
        bounce: RangedParameter {
            min: 0.0,
            max: 0.0,
            bias: 0.0,
        },
    };
    let particle_command =
        ToClientCommand::SpawnParticle(Box::new(server_to_client::SpawnParticleCommand {
            parameters: particle_params,
        }));
    conn.send(particle_command).unwrap();
}

const LEVEL_EVENT_BLOCK_BREAK: u32 = 2001;
pub async fn level_event(
    packet_data: &ClientboundLevelEvent,
    conn: &mut LuantiConnection,
    player_state: &state::PlayerState,
    light_cache: &state::LightCache,
    media_state: &state::MediaState,
) {
    let ClientboundLevelEvent {
        event_type,
        pos,
        data,
        global_event: _,
    } = packet_data;
    match *event_type {
        LEVEL_EVENT_BLOCK_BREAK => {
            block_break_particles(pos, *data, conn, player_state, light_cache, media_state).await
        }
        _ => trace!(
            "Got unhandled S2C LevelEvent {} at {:?}, ignoring",
            event_type, pos
        ),
    }
}

// spawns a burst of node-textured particles, like block break effects
async fn block_break_particles(
    pos: &BlockPos,
    data: u32,
    conn: &mut LuantiConnection,
    player_state: &state::PlayerState,
    light_cache: &state::LightCache,
    media_state: &state::MediaState,
) {
    let Ok(block_state) = BlockState::try_from(data) else {
        warn!("Got block break particle event with invalid block state id {data}");
        return;
    };
    if block_state.is_air() {
        return;
    }
    let cave_air_glow = player_state.current_dimension == state::Dimensions::Nether;
    let light = light_cache.get_node(pos.x, pos.y, pos.z);
    let node = utils::state_to_node(block_state, cave_air_glow, light, media_state);

    // particles spawn from the block center
    let center = v3f {
        x: utils::mirror_pos(pos.x as f32 + 0.5),
        y: utils::align_pos(pos.y as f32 + 0.5),
        z: utils::align_pos(pos.z as f32 + 0.5),
    };

    let mut rng = rand::rng();
    // 10 particles, block_break does a burst in vanilla
    let mut particles = Vec::with_capacity(10);
    for _ in 0..10 {
        particles.push(ParticleParameters {
            pos: center,
            vel: v3f {
                x: rng.random_range(-1.5..1.5),
                y: rng.random_range(0.0..3.0),
                z: rng.random_range(-1.5..1.5),
            },
            acc: v3f {
                x: 0.0,
                y: -9.0,
                z: 0.0,
            },
            expiration_time: rng.random_range(0.3..0.6),
            size: rng.random_range(0.6..1.0),
            base: CommonParticleParams {
                collision_detection: true,
                vertical: false,
                collision_removal: true,
                animation: TileAnimationParams::None,
                glow: 0,
                object_collision: false,
                node,
                node_tile: 0, // picks a random tile of the node
                texture: ServerParticleTexture {
                    base: ParticleTexture::default(),
                    string: String::new(),
                },
            },
            drag: v3f::ZERO,
            jitter: RangedParameter {
                min: v3f::ZERO,
                max: v3f::ZERO,
                bias: 0.0,
            },
            bounce: RangedParameter {
                min: 0.0,
                max: 0.0,
                bias: 0.0,
            },
        });
    }
    let batch_command =
        ToClientCommand::SpawnParticleBatch(Box::new(server_to_client::SpawnParticleBatchSpec {
            particles,
        }));
    conn.send(batch_command).unwrap();
}

// per-block particle spawners
// minecraft relies on the client to spawn particles for particle-spawning blocks
// luanti instead uses particle spawners that the server defines
struct Emission {
    // 1 texture = static, else one is picked at random per particle
    textures: &'static [&'static str],
    size: f32,
    amount: u16,
    exptime: f32,
    // spawn position range relative to the block center
    pos_min: v3f,
    pos_max: v3f,
    // velocity range
    vel_min: v3f,
    vel_max: v3f,
}

// one emission uses one pool and describes one particle type, blocks can combine several emissions
struct BlockSpawnerDef {
    kinds: &'static [BlockKind],
    emissions: &'static [Emission],
}

// pools: textures that are either animated or randomly picked from
const GENERIC_POOL: &[&str] = &[
    "particle-generic_0.png",
    "particle-generic_1.png",
    "particle-generic_2.png",
    "particle-generic_3.png",
    "particle-generic_4.png",
    "particle-generic_5.png",
    "particle-generic_6.png",
    "particle-generic_7.png",
];
const SOUL_POOL: &[&str] = &[
    "particle-soul_0.png",
    "particle-soul_1.png",
    "particle-soul_2.png",
    "particle-soul_3.png",
    "particle-soul_4.png",
    "particle-soul_5.png",
    "particle-soul_6.png",
    "particle-soul_7.png",
    "particle-soul_8.png",
    "particle-soul_9.png",
    "particle-soul_10.png",
];
const BIG_SMOKE_POOL: &[&str] = &[
    "particle-big_smoke_0.png",
    "particle-big_smoke_1.png",
    "particle-big_smoke_2.png",
    "particle-big_smoke_3.png",
    "particle-big_smoke_4.png",
    "particle-big_smoke_5.png",
    "particle-big_smoke_6.png",
    "particle-big_smoke_7.png",
    "particle-big_smoke_8.png",
    "particle-big_smoke_9.png",
    "particle-big_smoke_10.png",
    "particle-big_smoke_11.png",
];
const GLITTER_POOL: &[&str] = &[
    "particle-glitter_0.png",
    "particle-glitter_1.png",
    "particle-glitter_2.png",
    "particle-glitter_3.png",
    "particle-glitter_4.png",
    "particle-glitter_5.png",
    "particle-glitter_6.png",
    "particle-glitter_7.png",
];
const PORTAL_POOL: &[&str] = &[
    "particle-generic_0.png^[colorize:#8B00E0:220",
    "particle-generic_1.png^[colorize:#8B00E0:220",
    "particle-generic_2.png^[colorize:#8B00E0:220",
    "particle-generic_3.png^[colorize:#8B00E0:220",
    "particle-generic_4.png^[colorize:#8B00E0:220",
    "particle-generic_5.png^[colorize:#8B00E0:220",
    "particle-generic_6.png^[colorize:#8B00E0:220",
    "particle-generic_7.png^[colorize:#8B00E0:220",
];

const TORCH_FLAME: Emission = Emission {
    textures: &["particle-flame.png"],
    size: 2.0,
    amount: 1,
    exptime: 0.75,
    pos_min: v3f::new(-0.05, 0.1, -0.05),
    pos_max: v3f::new(0.05, 0.2, 0.05),
    vel_min: v3f::ZERO,
    vel_max: v3f::ZERO,
};
const SOUL_TORCH_FLAME: Emission = Emission {
    textures: &["particle-soul_fire_flame.png"],
    size: 2.0,
    amount: 1,
    exptime: 0.75,
    pos_min: v3f::new(-0.05, 0.1, -0.05),
    pos_max: v3f::new(0.05, 0.2, 0.05),
    vel_min: v3f::ZERO,
    vel_max: v3f::ZERO,
};
// shared by torch and soul torch
const TORCH_SMOKE: Emission = Emission {
    textures: GENERIC_POOL,
    size: 2.0,
    amount: 1,
    exptime: 1.5,
    pos_min: v3f::new(-0.05, 0.1, -0.05),
    pos_max: v3f::new(0.05, 0.25, 0.05),
    vel_min: v3f::new(-0.05, 0.15, -0.05),
    vel_max: v3f::new(0.05, 0.3, 0.05),
};

// standalone fire block
const FIRE_FLAME: Emission = Emission {
    textures: &["particle-flame.png"],
    size: 1.6,
    amount: 6,
    exptime: 1.0,
    pos_min: v3f::new(-0.3, -0.3, -0.3),
    pos_max: v3f::new(0.3, 0.3, 0.3),
    vel_min: v3f::ZERO,
    vel_max: v3f::new(0.0, 0.2, 0.0),
};
const FIRE_SMOKE: Emission = Emission {
    textures: GENERIC_POOL,
    size: 1.2,
    amount: 1,
    exptime: 2.0,
    pos_min: v3f::new(-0.3, 0.0, -0.3),
    pos_max: v3f::new(0.3, 0.4, 0.3),
    vel_min: v3f::new(-0.05, 0.2, -0.05),
    vel_max: v3f::new(0.05, 0.4, 0.05),
};

// same thing for soul fire. soul instead of smoke particles
const SOUL_FIRE_FLAME: Emission = Emission {
    textures: &["particle-soul_fire_flame.png"],
    size: 1.6,
    amount: 6,
    exptime: 1.0,
    pos_min: v3f::new(-0.3, -0.3, -0.3),
    pos_max: v3f::new(0.3, 0.3, 0.3),
    vel_min: v3f::ZERO,
    vel_max: v3f::new(0.0, 0.2, 0.0),
};
const SOUL_WISP: Emission = Emission {
    textures: SOUL_POOL,
    size: 1.0,
    amount: 2,
    exptime: 2.0,
    pos_min: v3f::new(-0.2, 0.0, -0.2),
    pos_max: v3f::new(0.2, 0.3, 0.2),
    vel_min: v3f::new(-0.05, 0.3, -0.05),
    vel_max: v3f::new(0.05, 0.6, 0.05),
};

const CAMPFIRE_SMOKE: Emission = Emission {
    textures: BIG_SMOKE_POOL,
    size: 6.0,
    amount: 2,
    exptime: 10.0,
    // starts near the top of the campfire
    pos_min: v3f::new(-0.1, 0.5, -0.1),
    pos_max: v3f::new(0.1, 0.5, 0.1),
    // moves upward, with a little horizontal drift
    vel_min: v3f::new(-0.1, 1.0, -0.1),
    vel_max: v3f::new(0.1, 1.5, 0.1),
};

const CRYING_OBSIDIAN_TEAR: Emission = Emission {
    textures: &["particle-drip_hang.png^[colorize:#6B24AC:220"],
    size: 0.7,
    amount: 1,
    exptime: 3.0,
    pos_min: v3f::new(-0.5, -0.5, -0.5),
    pos_max: v3f::new(0.5, -0.45, 0.5),
    vel_min: v3f::new(0.0, -0.1, 0.0),
    vel_max: v3f::new(0.0, -0.05, 0.0),
};

// from center, not bothering with rotation
const END_ROD_SPARKLE: Emission = Emission {
    textures: GLITTER_POOL,
    size: 0.5,
    amount: 1,
    exptime: 1.5,
    pos_min: v3f::new(-0.05, 0.4, -0.05),
    pos_max: v3f::new(0.05, 0.5, 0.05),
    vel_min: v3f::new(-0.02, 0.0, -0.02),
    vel_max: v3f::new(0.02, 0.05, 0.02),
};

// per portal block, we dont need to know about the portal structure
const PORTAL_SWIRL: Emission = Emission {
    textures: PORTAL_POOL,
    size: 1.8,
    amount: 1,
    exptime: 2.0,
    pos_min: v3f::new(-0.4, -0.4, -0.4),
    pos_max: v3f::new(0.4, 0.4, 0.4),
    vel_min: v3f::new(-0.1, -0.1, -0.1),
    vel_max: v3f::new(0.1, 0.1, 0.1),
};

const SPAWNER_DEFS: &[BlockSpawnerDef] = &[
    BlockSpawnerDef {
        kinds: &[BlockKind::Torch, BlockKind::WallTorch],
        emissions: &[TORCH_FLAME, TORCH_SMOKE],
    },
    BlockSpawnerDef {
        kinds: &[BlockKind::SoulTorch, BlockKind::SoulWallTorch],
        emissions: &[SOUL_TORCH_FLAME, TORCH_SMOKE],
    },
    BlockSpawnerDef {
        kinds: &[BlockKind::Fire],
        emissions: &[FIRE_FLAME, FIRE_SMOKE],
    },
    BlockSpawnerDef {
        kinds: &[BlockKind::SoulFire],
        emissions: &[SOUL_FIRE_FLAME, SOUL_WISP],
    },
    BlockSpawnerDef {
        kinds: &[BlockKind::Campfire, BlockKind::SoulCampfire],
        emissions: &[CAMPFIRE_SMOKE],
    },
    BlockSpawnerDef {
        kinds: &[BlockKind::CryingObsidian],
        emissions: &[CRYING_OBSIDIAN_TEAR],
    },
    BlockSpawnerDef {
        kinds: &[BlockKind::EndRod],
        emissions: &[END_ROD_SPARKLE],
    },
    BlockSpawnerDef {
        kinds: &[BlockKind::NetherPortal],
        emissions: &[PORTAL_SWIRL],
    },
];

fn spawner_def_for_block(kind: BlockKind) -> Option<usize> {
    SPAWNER_DEFS
        .iter()
        .position(|def| def.kinds.contains(&kind))
}

// set the per-block particle spawners for a block
// also called if the block becomes air, so we delete them
pub async fn sync_block_spawner(
    pos: BlockPos,
    kind: BlockKind,
    conn: &LuantiConnection,
    particle_spawners: &mut state::ParticleSpawnerState,
) {
    let current = particle_spawners.get(&pos);
    let wanted = spawner_def_for_block(kind);
    if current
        .as_ref()
        .is_some_and(|(cur_idx, _)| Some(*cur_idx) == wanted)
    {
        return;
    }
    if current.is_none() && wanted.is_none() {
        return;
    }

    if let Some((_, ids)) = particle_spawners.remove(&pos) {
        for id in ids {
            conn.send(ToClientCommand::DeleteParticlespawner(Box::new(
                server_to_client::DeleteParticlespawnerSpec { server_id: id },
            )))
            .unwrap();
        }
    }

    let Some(def_index) = wanted else { return };
    let def = &SPAWNER_DEFS[def_index];
    let ids = particle_spawners.insert(pos, def_index, def.emissions.len());
    // spawn from the block center
    let center = v3f {
        x: utils::mirror_pos(pos.x as f32 + 0.5),
        y: utils::align_pos(pos.y as f32 + 0.5),
        z: utils::align_pos(pos.z as f32 + 0.5),
    };
    for (emission, id) in def.emissions.iter().zip(ids) {
        let command = build_spawner(emission, center, id);
        conn.send(ToClientCommand::AddParticlespawner(Box::new(command)))
            .unwrap();
    }
}

fn fixed_vec3(v: v3f) -> TweenedParameter<RangedParameter<v3f>> {
    ranged_vec3(v, v)
}

fn fixed_f32(v: f32) -> TweenedParameter<RangedParameter<f32>> {
    ranged_f32(v, v)
}

fn ranged_vec3(min: v3f, max: v3f) -> TweenedParameter<RangedParameter<v3f>> {
    let ranged = RangedParameter {
        min,
        max,
        bias: 0.0,
    };
    TweenedParameter {
        style: TweenStyle::Fwd,
        reps: 1,
        beginning: 0.0,
        start: ranged.clone(),
        end: ranged,
    }
}

fn ranged_f32(min: f32, max: f32) -> TweenedParameter<RangedParameter<f32>> {
    let ranged = RangedParameter {
        min,
        max,
        bias: 0.0,
    };
    TweenedParameter {
        style: TweenStyle::Fwd,
        reps: 1,
        beginning: 0.0,
        start: ranged.clone(),
        end: ranged,
    }
}

fn build_spawner(emission: &Emission, center: v3f, id: u32) -> AddParticlespawnerCommand {
    // single textures get written to base.texture, multiple to texpool so luanti picks at random
    let (texture_string, texpool) = match emission.textures {
        [single] => (String::from(*single), Vec::new()),
        multiple => (
            String::new(),
            multiple
                .iter()
                .map(|texture| ServerParticleTexture {
                    base: ParticleTexture::default(),
                    string: String::from(*texture),
                })
                .collect(),
        ),
    };
    AddParticlespawnerCommand {
        base: CommonParticleParams {
            collision_detection: false,
            vertical: false,
            collision_removal: false,
            animation: TileAnimationParams::None,
            glow: 0,
            object_collision: false,
            node: MapNode::default(),
            node_tile: 0,
            texture: ServerParticleTexture {
                base: ParticleTexture::default(),
                string: texture_string,
            },
        },
        amount: emission.amount,
        time: 0.0, // 0 = spawn forever, until DeleteParticlespawner
        texpool,
        pos: ranged_vec3(center + emission.pos_min, center + emission.pos_max),
        vel: ranged_vec3(emission.vel_min, emission.vel_max),
        acc: fixed_vec3(v3f::ZERO),
        drag: fixed_vec3(v3f::ZERO),
        radius: fixed_vec3(v3f::ZERO),
        jitter: fixed_vec3(v3f::ZERO),
        attractor: Attractor::None,
        exptime: fixed_f32(emission.exptime),
        size: fixed_f32(emission.size),
        bounce: fixed_f32(0.0),
        server_id: id,
        attached_id: 0,
    }
}
