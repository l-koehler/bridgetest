use crate::state;
use crate::utils;
use azalea::BlockPos;
use azalea::block::BlockState;
use azalea::entity::particle::Particle;
use azalea::protocol::packets::game::ClientboundLevelParticles;
use azalea::protocol::packets::game::c_level_event::ClientboundLevelEvent;
use glam::Vec3 as v3f;
use log::*;
use luanti_core::MapNode;
use luanti_protocol::{
    LuantiConnection,
    commands::server_to_client::{
        self, CommonParticleParams, ParticleParameters, ParticleTexture, ServerParticleTexture,
        ToClientCommand,
    },
    types::RangedParameter,
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
            animation: luanti_protocol::types::TileAnimationParams::None,
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
                animation: luanti_protocol::types::TileAnimationParams::None,
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
