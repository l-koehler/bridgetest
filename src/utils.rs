/*
 * This file contains shared functions, for example logging
 */

use crate::settings;
use crate::MTServerState;
use crate::mt_definitions;
use crate::textures;

use azalea::inventory::ItemStack;
use luanti_core::ContentId;
use minecraft_data_rs::models::version::Version;
use minecraft_data_rs::{api, Api};
use luanti_protocol::CommandRef;
use luanti_protocol::CommandDirection;
use luanti_core::MapNode;
use azalea_client::Event;
use azalea::core::{aabb::AABB, position::Vec3};
use azalea::registry::{EntityKind, Registry};
use azalea_block::BlockState;
use rand::Rng;
use mt_definitions::EntityMetadata;
use textures::LuantiTexture;
use std::path::PathBuf;

use glam::Vec3 as v3f;

// modified version of the liang-barsky line clipping algo
// adapted to work in 3d and also to return a simple boolean indicating if the line clips at all.
// also makes the bounding box a little higher to account for some weird graphics
pub fn liang_barsky_3d(mut bb: AABB, line_a: Vec3, line_b: Vec3) -> bool {
    let mut t0 = 0.0;
    let mut t1 = 1.0;

    bb.max_y += 1.0;
    bb.min_y -= 0.5;

    let dx = line_b.x - line_a.x;
    let dy = line_b.y - line_a.y;
    let dz = line_b.z - line_a.z;


    let clipping_edges = [
        (-dx, line_a.x - bb.min_x.min(bb.max_x)),
        ( dx, bb.max_x.max(bb.min_x) - line_a.x),
        (-dy, line_a.y - bb.min_y.min(bb.max_y)),
        ( dy, bb.max_y.max(bb.min_y) - line_a.y),
        (-dz, line_a.z - bb.min_z.min(bb.max_z)),
        ( dz, bb.max_z.max(bb.min_z) - line_a.z),
    ];
    for &(p, q) in &clipping_edges {
        if p == 0.0 && q < 0.0 {
            return false;
        }
        let r = q / p;
        if p < 0.0 {
            if r > t1 {
                return false;
            }
            if r > t0 {
                t0 = r;
            }
        } else if p > 0.0 {
            if r < t0 {
                return false;
            }
            if r < t1 {
                t1 = r;
            }
        }
    }
    t0 < t1
}

pub fn normalize_angle(angle: f32) -> f32 {
    let mut normalized_angle = angle % 360.0;
    if normalized_angle < 0.0 {
        normalized_angle += 360.0;
    }
    normalized_angle
}

pub fn allocate_id(serverside_id: u32, mt_server_state: &mut MTServerState) -> u16 {
    // pick smallest range
    let i_smallest_range = mt_server_state.c_alloc_id_ranges
        .iter()
        .enumerate()
        .min_by_key(|&(_, &(start, end))| end - start)
        .map(|(index, _)| index)
        .expect("Client exhausted all available entity IDs!");
    // pick new ID
    let clientside_id: u16 = mt_server_state.c_alloc_id_ranges[i_smallest_range].0;
    // resize range
    if mt_server_state.c_alloc_id_ranges[i_smallest_range].0 == mt_server_state.c_alloc_id_ranges[i_smallest_range].1 {
        mt_server_state.c_alloc_id_ranges.remove(i_smallest_range);
    } else {
        mt_server_state.c_alloc_id_ranges[i_smallest_range].0 += 1;
    }
    // add ID pair to maps, return
    mt_server_state.entity_id_map.insert(serverside_id, clientside_id);
    mt_server_state.entity_meta_map.insert(serverside_id, EntityMetadata::default());
    return clientside_id;
}

pub fn free_id(serverside_id: u32, mt_server_state: &mut MTServerState) {
    // remove from maps
    let id_pair = mt_server_state.entity_id_map.remove_by_left(&serverside_id);
    mt_server_state.entity_meta_map.remove(&serverside_id);
    mt_server_state.entities_update_scheduled.retain(|x| *x != serverside_id); // may be scheduled several times
    // add new range and re-optimize the ranges
    match id_pair {
        Some((_, clientside_id)) => {
            mt_server_state.c_alloc_id_ranges.push((clientside_id, clientside_id));
            defrag_ranges(mt_server_state);
        },
        None => ()
    }
}

fn defrag_ranges(mt_server_state: &mut MTServerState) {
    mt_server_state.c_alloc_id_ranges.sort_by_key(|r| r.0);
    let mut index_lim = mt_server_state.c_alloc_id_ranges.len()-1;
    let mut p = mt_server_state.c_alloc_id_ranges[0];
    let mut r_index: usize = 1;
    loop {
        if r_index > index_lim {
            break;
        }
        let r = mt_server_state.c_alloc_id_ranges[r_index];
        if r.0 == p.1+1 {
            mt_server_state.c_alloc_id_ranges[r_index-1].1 = r.1;
            mt_server_state.c_alloc_id_ranges.remove(r_index);
            index_lim -= 1;
            p = (p.0, r.1);
        } else {
            p = r;
            r_index += 1;
        }
    }
}

pub fn texture_from_itemstack(item: &ItemStack, mt_server_state: &MTServerState) -> String {
    match item {
        ItemStack::Empty => String::from("air.png"),
        ItemStack::Present(slot_data) => {
            let item_name = slot_data.kind.to_string();
            let inventory_image: String;
            if item_name.ends_with("_spawn_egg") {
                inventory_image = mt_server_state.item_texture_map.get("minecraft:template_spawn_egg").unwrap().clone().to_luanti_safe();
            } else {
                if mt_server_state.item_texture_map.contains_key(&item_name) {
                    inventory_image = mt_server_state.item_texture_map.get(&item_name).unwrap().clone().to_luanti_safe();
                } else {
                    inventory_image = mt_server_state.block_texture_map.get(&item_name).unwrap().clone().to_safe_cube();
                }
            }
            return inventory_image;
        }
    }
}

pub fn state_to_node(state: BlockState, cave_air_glow: bool) -> MapNode {
    let mut param0: u16;
    let param1: u8;
    let param2: u8 = 0;
    param0 = azalea::registry::Block::try_from(state).unwrap().to_u32() as u16 + 128;
    
    // param1: transparency i think
    if state.is_air() {
        param0 = 126;
        param1 = 0xEE;
    } else if (azalea::registry::Block::try_from(state).unwrap() == azalea::registry::Block::CaveAir) && cave_air_glow {
        param0 = 120; // custom node: glowing_air, used in nether
        param1 = 0xEE;
    } else {
        param1 = 0x00;
    }
    
    MapNode {
        content_id: ContentId(param0),
        param1,
        param2,
    }
}

pub fn vec3_to_v3f(input_vector: &Vec3, scale: f64) -> v3f {
    // loss of precision, f64 -> f32
    let Vec3 { x: xf64, y: yf64, z: zf64 } = input_vector;
    v3f {
        x: (*xf64/scale) as f32,
        y: (*yf64/scale) as f32,
        z: (*zf64/scale) as f32
    }
}

pub fn get_colormap(texture: &LuantiTexture) -> Option<(u8, u8, u8)> {
    // use the "Plains" texture. per-biome textures dont really work in mt afaik
    // https://minecraft.fandom.com/wiki/Color#Block_and_fluid_colors - what blocks use the colormaps
    // https://minecraft.fandom.com/wiki/Block_colors                 - what colors are to be used
    let r_texture = texture.to_luanti_safe();
    let name = r_texture.as_str();
    let grass_group = ["block-grass_block_top.png", "block-grass_block_side_overlay.png", "block-short_grass.png", "block-tall_grass_bottom.png", "block-tall_grass_top.png", "block-fern.png", "block-large_fern_bottom.png", "block-large_fern_top.png"];
    if grass_group.contains(&name) {
        return Some((0x91, 0xBD, 0x59))
    }
    let foliage_group = ["block-oak_leaves.png", "block-jungle_leaves.png", "block-acacia_leaves.png", "block-dark_oak_leaves.png", "block-vine.png"];
    if foliage_group.contains(&name) {
        return Some((0x77, 0xAB, 0x2F))
    }
    let water_group = ["block-water_still.png", "block-water_flow.png"];
    if water_group.contains(&name) {
        return Some((0x3F, 0x76, 0xE4))
    }
    let stem_group = ["block-attached_melon_stem.png", "block-attached_pumpkin_stem.png", "block-melon_stem.png", "block-pumpkin_stem.png", "pink_petals_stem.png"];
    if stem_group.contains(&name) {
        return Some((0xE0, 0xC7, 0x1C))
    }
    // these textures are colormapped but constant for some stupid reason
    if name == "block-birch_leaves.png" { return Some((0x80, 0xA7, 0x55)) }
    if name == "block-spruce_leaves.png" { return Some((0x61, 0x99, 0x61)) }
    if name == "block-lily_pad.png" { return Some((0x20, 0x80, 0x30)) }
    None
}

pub fn show_mt_command(command: &dyn CommandRef) {
    let dir = match command.direction() {
        CommandDirection::ToClient => "S->C",
        CommandDirection::ToServer => "C->S",
    };
    logger(&format!("[Minetest] {} {}", dir, command.command_name()), 0);
    //println!("{} {:#?}", dir, command); // overly verbose
}

pub fn logger(text: &str, level: i8) {
    /*
     * Level 0: Debug - Everything that makes some sense to have
     * Level 1: Stats - Status updates and other sort-of-useful stuff
     * Level 2: Error - Packet got dropped or something
     * Level 3: Fatal - Cannot recover, will panic or drop connections
     */
    if settings::DROP_LOG_BELOW <= level {
        if level == 0 {
            println!("\x1b[0;37m[{:?}] [DEBUG] {}\x1b[0m", chrono::Utc::now().timestamp(), text)
        } else if level == 1{
            println!("[{:?}] [STATS] {}", chrono::Utc::now().timestamp(), text)
        } else if level == 2 {
            println!("\x1b[1;33m[{:?}] [ERROR] {}\x1b[0m", chrono::Utc::now().timestamp(), text)
        } else {
            println!("\x1b[0;31m[{:?}] [FATAL] {}\x1b[0m", chrono::Utc::now().timestamp(), text)
        }
    }


}

pub fn show_mc_command(command: &Event) {
    match command {
        Event::Tick => (), // Don't log ticks, these happen far too often for that
        _ => logger(&format!("[Minecraft] S->C {}", mc_packet_name(command)), 1)
    }
}

pub fn get_random_username() -> String {
    let hs_name = String::from(settings::HS_NAMES[rand::thread_rng().gen_range(0..26)]);
    format!("{}{:0>3}", hs_name, rand::thread_rng().gen_range(0..1000))
}

pub fn find_suffix_match(dir: &PathBuf, suffix: &str) -> Option<PathBuf> {
    for entry in std::fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(suffix) {
                    return Some(path);
                }
            }
        }
    }
    None
}

pub fn get_entity_model(entity: &EntityKind) -> (&str, Vec<String>) {
    match entity {
        // TODO for entitys without models choose the least stupid-looking fallback
        EntityKind::Axolotl    => ("model-axolotl.b3d" , vec![String::from("entity-axolotl-cyan.png")]),
        EntityKind::Bat        => ("model-bat.b3d"     , vec![String::from("entity-bat.png")]),
        EntityKind::Blaze      => ("model-blaze.b3d"   , vec![String::from("entity-blaze.png")]),
        
        EntityKind::AcaciaBoat => ("model-boat.b3d"    , vec![String::from("entity-boat-acacia.png")]),
        EntityKind::BirchBoat  => ("model-boat.b3d"    , vec![String::from("entity-boat-birch.png")]),
        EntityKind::BambooRaft => ("model-boat.b3d"    , vec![String::from("entity-boat-oak.png")]), // TODO
        EntityKind::CherryBoat => ("model-boat.b3d"    , vec![String::from("entity-boat-cherry.png")]),
        EntityKind::DarkOakBoat => ("model-boat.b3d"   , vec![String::from("entity-boat-darkoak.png")]),
        EntityKind::JungleBoat => ("model-boat.b3d"    , vec![String::from("entity-boat-jungle.png")]),
        EntityKind::MangroveBoat => ("model-boat.b3d"  , vec![String::from("entity-boat-mangrove.png")]),
        EntityKind::OakBoat    => ("model-boat.b3d"    , vec![String::from("entity-boat-oak.png")]),
        EntityKind::PaleOakBoat => ("model-boat.b3d"   , vec![String::from("entity-boat-birch.png")]), // TODO
        EntityKind::SpruceBoat => ("model-boat.b3d"    , vec![String::from("entity-boat-spruce.png")]),
        
        EntityKind::Cat        => ("model-cat.b3d"     , vec![String::from("entity-cat-red.png")]),
        EntityKind::CaveSpider => ("model-spider.b3d"  , vec![String::from("entity-spider-cave_spider.png")]),
        EntityKind::ChestMinecart => ("model-minecart_chest.b3d", vec![String::from("entity-minecart.png")]), // minecraft adds the chest texture, there is no separate minecart texture
        EntityKind::Chicken    => ("model-chicken.b3d" , vec![String::from("entity-chicken.png")]),
        EntityKind::Cod        => ("model-cod.b3d"     , vec![String::from("entity-fish-cod.png")]),
        EntityKind::CommandBlockMinecart => ("model-minecart_block.b3d", vec![String::from("entity-minecart.png"), String::from("block-command_block_side.png")]),
        EntityKind::Cow        => ("model-cow.b3d"     , vec![String::from("entity-cow-cow.png"), String::from("block-red_mushroom.png^[opacity:0")]), // transparent
        EntityKind::Creeper    => ("model-creeper.b3d" , vec![String::from("entity-creeper-creeper.png")]),
        EntityKind::Dolphin    => ("model-dolphin.b3d" , vec![String::from("entity-dolphin.png")]),
        EntityKind::Donkey     => ("model-horse.b3d"   , vec![String::from("entity-horse-donkey.png")]),
        EntityKind::Drowned    => ("model-zombie.b3d"  , vec![String::from("entity-zombie-zombie.png")]), // drowned is a layered texture
        EntityKind::ElderGuardian => ("model-guardian.b3d", vec![String::from("entity-guardian_elder.png")]),
        EntityKind::EndCrystal => ("model-end_crystal.b3d", vec![String::from("entity-end_crystal-end_crystal.png")]),
        EntityKind::EnderDragon => ("model-dragon.b3d" , vec![String::from("entity-enderdragon-dragon.png")]),
        EntityKind::Enderman   => ("model-enderman.b3d", vec![String::from("entity-enderman-enderman.png")]),
        EntityKind::Endermite  => ("model-endermite.b3d", vec![String::from("entity-endermite.png")]),
        EntityKind::Evoker     => ("model-evoker.b3d"  , vec![String::from("entity-illager-evoker.png")]),
        EntityKind::Fox        => ("model-cat.b3d"     , vec![String::from("entity-fox-fox.png")]),
        EntityKind::FurnaceMinecart => ("model-minecart_block.b3d", vec![String::from("entity-minecart.png"), String::from("block-furnace_side.png")]),
        EntityKind::Ghast      => ("model-ghast.b3d"   , vec![String::from("entity-ghast-ghast.png")]),
        EntityKind::GlowSquid  => ("model-glow_squid.b3d", vec![String::from("entity-squid-glow_squid.png")]),
        EntityKind::Goat       => ("model-sheepfur.b3d", vec![String::from("entity-goat-goat.png")]),
        EntityKind::Guardian   => ("model-guardian.b3d", vec![String::from("entity-guardian.png")]),
        EntityKind::Hoglin     => ("model-hoglin.b3d"  , vec![String::from("entity-hoglin-hoglin.png")]),
        EntityKind::HopperMinecart => ("model-minecart_hopper.b3d", vec![String::from("entity-minecart.png")]),
        EntityKind::Horse      => ("model-horse.b3d"   , vec![String::from("entity-horse-horse_brown.png")]),
        EntityKind::Husk       => ("model-zombie.b3d"  , vec![String::from("entity-zombie-husk.png")]),
        EntityKind::Illusioner => ("model-illusioner.b3d", vec![String::from("entity-illager-illusioner.png")]),
        EntityKind::IronGolem  => ("model-iron_golem.b3d", vec![String::from("entity-iron_golem-iron_golem.png")]),
        EntityKind::Llama      => ("model-llama.b3d"   , vec![String::from("entity-llama-creamy.png")]),
        EntityKind::MagmaCube  => ("model-magmacube.b3d", vec![String::from("entity-slime-magmacube.png")]),
        EntityKind::Minecart   => ("model-minecart.b3d", vec![String::from("entity-minecart.png")]),
        EntityKind::Mooshroom  => ("model-cow.b3d"     , vec![String::from("entity-cow-red_mooshroom.png"), String::from("block-red_mushroom.png")]),
        EntityKind::Mule       => ("model-horse.b3d"   , vec![String::from("entity-horse-mule.png")]),
        EntityKind::Ocelot     => ("model-cat.b3d"     , vec![String::from("entity-cat-ocelot.png")]),
        EntityKind::Parrot     => ("model-parrot.b3d"  , vec![String::from("entity-parrot-parrot_red_blue.png")]),
        EntityKind::Pig        => ("model-pig.b3d"     , vec![String::from("entity-pig-pig.png")]),
        EntityKind::Piglin     => ("model-sword_piglin.b3d", vec![String::from("entity-piglin-piglin.png"), String::from("item-golden_sword.png")]),
        EntityKind::PiglinBrute => ("model-sword_piglin.b3d", vec![String::from("entity-piglin-piglin_brute.png"), String::from("item-golden_axe.png")]),
        EntityKind::Pillager   => ("model-pillager.b3d", vec![String::from("entity-illager-pillager.png"), String::from("item-crossbow_arrow.png")]),
        EntityKind::PolarBear  => ("model-polarbear.b3d", vec![String::from("entity-bear-polarbear.png")]),
        EntityKind::Rabbit     => ("model-rabbit.b3d"  , vec![String::from("entity-rabbit-brown.png")]),
        EntityKind::Salmon     => ("model-salmon.b3d"  , vec![String::from("entity-fish-salmon.png")]),
        EntityKind::Sheep      => ("model-sheepfur.b3d", vec![String::from("entity-sheep-sheep_fur.png"),String::from("entity-sheep-sheep.png")]),
        EntityKind::Shulker    => ("model-shulker.b3d" , vec![String::from("entity-shulker-shulker.png")]),
        EntityKind::Silverfish => ("model-silverfish.b3d", vec![String::from("entity-silverfish.png")]),
        EntityKind::Skeleton   => ("model-skeleton.b3d", vec![String::from("entity-skeleton-skeleton.png"), String::from("bow_pulling_2.png")]),
        EntityKind::Slime      => ("model-slime.b3d"   , vec![String::from("entity-slime-slime.png")]),
        EntityKind::SnowGolem  => ("model-snowman.b3d" , vec![String::from("entity-snow_golem.png")]),
        EntityKind::SpawnerMinecart => ("model-minecart_block.b3d", vec![String::from("entity-minecart.png"), String::from("block-spawner.png")]),
        EntityKind::Spider     => ("model-spider.b3d"  , vec![String::from("entity-spider-spider.png")]),
        EntityKind::Squid      => ("model-squid.b3d"   , vec![String::from("entity-squid-squid.png")]),
        EntityKind::Stray      => ("model-stray.b3d"   , vec![String::from("entity-skeleton-stray.png")]), //TODO layered
        EntityKind::Strider    => ("model-strider.b3d" , vec![String::from("entity-strider.png")]),
        EntityKind::TntMinecart => ("model-minecart_block.b3d", vec![String::from("entity-minecart.png"), String::from("block-tnt_side.png")]),
        EntityKind::TraderLlama => ("model-llama.b3d"  , vec![String::from("entity-llama-brown.png")]),
        EntityKind::TropicalFish => ("model-tropical_fish_a.b3d", vec![String::from("entity-fish-tropical_a.png")]), // a/b textures with patterns. no way am i going to deal with that
        EntityKind::Vex        => ("model-vex.b3d"     , vec![String::from("entity-illager-vex.png")]),
        EntityKind::Villager   => ("model-villager.b3d", vec![String::from("entity-villager-villager.png")]),
        EntityKind::Vindicator => ("model-vindicator.b3d", vec![String::from("entity-illager-vindicator.png"), String::from("item-iron_axe.png")]),
        EntityKind::WanderingTrader => ("model-villager.b3d", vec![String::from("entity-wandering_trader.png")]),
        EntityKind::Warden     => ("model-iron_golem.b3d", vec![String::from("entity-warden-warden.png")]),
        EntityKind::Witch      => ("model-witch.b3d"   , vec![String::from("entity-witch.png")]),
        EntityKind::Wither     => ("model-wither.b3d"  , vec![String::from("entity-wither-wither.png")]),
        EntityKind::WitherSkeleton => ("model-witherskeleton.b3d", vec![String::from("entity-skeleton-wither_skeleton.png")]),
        EntityKind::Wolf       => ("model-wolf.b3d"    , vec![String::from("entity-wolf-wolf.png")]),
        EntityKind::Zoglin     => ("model-hoglin.b3d"  , vec![String::from("entity-hoglin-zoglin.png")]),
        EntityKind::Zombie     => ("model-zombie.b3d"  , vec![String::from("entity-zombie-zombie.png")]),
        EntityKind::ZombieHorse => ("model-horse.b3d"  , vec![String::from("entity-horse-horse_zombie.png")]),
        EntityKind::ZombieVillager => ("model-villager.b3d", vec![String::from("entity-zombie_villager-zombie_villager.png")]),
        EntityKind::ZombifiedPiglin => ("model-sword_piglin.b3d", vec![String::from("entity-piglin-zombified_piglin.png"), String::from("item-golden_sword.png")]),
        EntityKind::Player     => ("model-armor_character.b3d", vec![String::from("entity-player-wide-steve.png")]),
        _                      => ("model-pig.b3d"     , vec![String::from("entity-pig-pig.png")])
    }
}

pub fn sanitize_model_name(mut name: String) -> String {
    let prefixes = ["mobs_mc_", "extra_mobs_"];
    for prefix in prefixes {
        name.remove_matches(prefix);
    };
    return name;
}

// these can't be printed any sane way, so have this nonsense
pub fn mc_packet_name(command: &Event) -> &str {
    match command {
        Event::Init => "Init",
        Event::Login => "Login",
        Event::Chat(_) => "Chat",
        Event::Tick => "Tick",
        Event::Packet(packet_value) => match **packet_value {
            // There are 117 possible cases here and most of them do not matter
            // Can't be bothered to keep this up to date tbh
            _ => "GamePacket (no detail)",
        },
        Event::AddPlayer(_) => "AddPlayer",
        Event::RemovePlayer(_) => "RemovePlayer",
        Event::UpdatePlayer(_) => "UpdatePlayer",
        Event::Death(_) => "Death",
        Event::KeepAlive(_) => "KeepAlive",
        Event::Disconnect(_) => "Disconnect",
    }
}

// select data API (from https://github.com/PrismarineJS/minecraft-data) based on azalea version
// Basically Api::latest() but compatible with azalea

pub fn compatible_data_api() -> Api {
    let Ok(versions) = api::versions() else {
        panic!("Failed to retrieve minecraft data versions!");
    };
    assert!(versions.len() != 0);
    let azalea_ver = azalea::protocol::packets::PROTOCOL_VERSION;
    let mut closest_match: Option<Version> = None;
    for version in versions {
        let closest_match_proto = match closest_match {
            Some(ref v)  => v.version,
            None => 0
        };
        if azalea_ver >= version.version && version.version > closest_match_proto {
            closest_match = Some(version);
        };
    };
    return Api::new(closest_match.expect("Found no version possibly matching azalea!"))
}