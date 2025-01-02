/*
 * This file contains shared functions, for example logging
 */

use crate::settings;
use crate::MTServerState;
use crate::mt_definitions;

use azalea::inventory::ItemStack;
use minetest_protocol::CommandRef;
use minetest_protocol::CommandDirection;
use minetest_protocol::wire::types::{v3f, MapNode};
use azalea_client::Event;
use azalea::core::{aabb::AABB, position::Vec3};
use azalea::registry::{EntityKind, Registry};
use azalea_block::BlockState;
use std::path::Path;
use std::path::PathBuf;
use std::io::Read;
use rand::Rng;
use mt_definitions::EntityMetadata;

// modified version of the liang-barsky line clipping algo
// adapted to work in 3d and also to return a simple boolean indicating if the line clips at all.
pub fn liang_barsky_3d(bb: AABB, line_a: Vec3, line_b: Vec3) -> bool {
    let mut t0 = 0.0;
    let mut t1 = 1.0;

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
            let item_name = slot_data.kind.to_string().replace("minecraft:", "").to_lowercase() + ".png";
            return match basename_to_prefixed(mt_server_state, &item_name) {
                Some(name) => name,
                None => format!("block-{}", item_name)
            }
        }
    }
}

pub fn basename_to_prefixed(mt_server_state: &MTServerState, basename: &str) -> Option<String> {
    // chests are in entity-chest-(something.png) eg
    match basename {
        "chest_side.png" => {
            return Some(String::from("entity-chest-normal_left.png"))
        }
        "chest_bottom.png" | "chest_top.png" | "chest.png" => {
            return Some(String::from("entity-chest-normal.png"))
        },
        "trapped_chest_side.png" => {
            return Some(String::from("entity-chest-trapped_left.png"))
        }
        "trapped_chest_bottom.png" | "trapped_chest_top.png" | "trapped_chest.png" => {
            return Some(String::from("entity-chest-trapped.png"))
        },
        "ender_chest.png" | "ender_chest_top.png" | "ender_chest_bottom.png" | "ender_chest_side.png" => {
            return Some(String::from("entity-chest-ender.png"))
        },
        _ => ()
    };
    for item in mt_server_state.path_name_map.iter() {
        if item.0.1 == basename { // basename
            if basename.contains("chest") {
                println!("success")
            }
            return Some(String::from(item.1)) // prefixed name
        }
    }
    None
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
        param0,
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

pub fn b3d_sanitize(input_path: String) -> String {
    input_path
    .replace("amc_", "")
    .replace("extra_mobs_", "")
    .replace("mcl_boats_", "")
    .replace("mcl_bows_", "")
    .replace("mcl_chests_", "")
    .replace("mcl_minecarts_", "")
    .replace("mcl_", "")
    .replace("mobs_mc_", "")
}

pub fn get_colormap(texture: &str) -> Option<(u8, u8, u8)> {
    // use the "Plains" texture. per-biome textures dont really work in mt afaik
    // https://minecraft.fandom.com/wiki/Color#Block_and_fluid_colors - what blocks use the colormaps
    // https://minecraft.fandom.com/wiki/Block_colors                 - what colors are to be used
    let grass_group = ["block-grass_block_top.png", "block-grass_block_side_overlay.png", "block-short_grass.png", "block-tall_grass_bottom.png", "block-tall_grass_top.png", "block-fern.png", "block-large_fern_bottom.png", "block-large_fern_top.png"];
    if grass_group.contains(&texture) {
        return Some((0x91, 0xBD, 0x59))
    }
    let foliage_group = ["block-oak_leaves.png", "block-jungle_leaves.png", "block-acacia_leaves.png", "block-dark_oak_leaves.png", "block-vine.png"];
    if foliage_group.contains(&texture) {
        return Some((0x77, 0xAB, 0x2F))
    }
    let water_group = ["block-water_still.png", "block-water_flow.png"];
    if water_group.contains(&texture) {
        return Some((0x3F, 0x76, 0xE4))
    }
    let stem_group = ["block-attached_melon_stem.png", "block-attached_pumpkin_stem.png", "block-melon_stem.png", "block-pumpkin_stem.png", "pink_petals_stem.png"];
    if stem_group.contains(&texture) {
        return Some((0xE0, 0xC7, 0x1C))
    }
    // these textures are colormapped but constant for some stupid reason
    if texture == "block-birch_leaves.png" { return Some((0x80, 0xA7, 0x55)) }
    if texture == "block-spruce_leaves.png" { return Some((0x61, 0x99, 0x61)) }
    if texture == "block-lily_pad.png" { return Some((0x20, 0x80, 0x30)) }
    None
}

pub fn ask_confirm(question: &str) -> bool {
    println!("{}",question);
    let mut input = [0];
    let _ = std::io::stdin().read(&mut input);
    match char::from_u32(input[0].into()).expect("Failed to read STDIN") {
        'y' | 'Y' => return true,
        _ => return false,
    }
}

pub fn possibly_create_dir(path: &PathBuf) -> bool {
    if !Path::new(path.as_path()).exists() {
        logger(&format!("Creating directory \"{}\"", path.display()), 0);
        let _ = std::fs::create_dir(path); // TODO check if this worked
        return true;
    } else {
        logger(&format!("Found \"{}\", not creating it", path.display()), 0);
        return false;
    }
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

pub fn get_entity_model(entity: &EntityKind) -> (&str, Vec<String>) {
    match entity {
        // TODO for entitys without models choose the least stupid-looking fallback
        EntityKind::Axolotl    => ("entitymodel-axolotl.b3d" , vec![String::from("entity-axolotl-cyan.png")]),
        EntityKind::Bat        => ("entitymodel-bat.b3d"     , vec![String::from("entity-bat.png")]),
        EntityKind::Blaze      => ("entitymodel-blaze.b3d"   , vec![String::from("entity-blaze.png")]),
        
        EntityKind::AcaciaBoat => ("entitymodel-boat.b3d"    , vec![String::from("entity-boat-acacia.png")]),
        EntityKind::BirchBoat  => ("entitymodel-boat.b3d"    , vec![String::from("entity-boat-birch.png")]),
        EntityKind::BambooRaft => ("entitymodel-boat.b3d"    , vec![String::from("entity-boat-oak.png")]), // TODO
        EntityKind::CherryBoat => ("entitymodel-boat.b3d"    , vec![String::from("entity-boat-cherry.png")]),
        EntityKind::DarkOakBoat => ("entitymodel-boat.b3d"   , vec![String::from("entity-boat-darkoak.png")]),
        EntityKind::JungleBoat => ("entitymodel-boat.b3d"    , vec![String::from("entity-boat-jungle.png")]),
        EntityKind::MangroveBoat => ("entitymodel-boat.b3d"  , vec![String::from("entity-boat-mangrove.png")]),
        EntityKind::OakBoat    => ("entitymodel-boat.b3d"    , vec![String::from("entity-boat-oak.png")]),
        EntityKind::PaleOakBoat => ("entitymodel-boat.b3d"   , vec![String::from("entity-boat-birch.png")]), // TODO
        EntityKind::SpruceBoat => ("entitymodel-boat.b3d"    , vec![String::from("entity-boat-spruce.png")]),
        
        EntityKind::Cat        => ("entitymodel-cat.b3d"     , vec![String::from("entity-cat-red.png")]),
        EntityKind::CaveSpider => ("entitymodel-spider.b3d"  , vec![String::from("entity-spider-cave_spider.png")]),
        EntityKind::ChestMinecart => ("entitymodel-minecart_chest.b3d", vec![String::from("entity-minecart.png")]), // minecraft adds the chest texture, there is no separate minecart texture
        EntityKind::Chicken    => ("entitymodel-chicken.b3d" , vec![String::from("entity-chicken.png")]),
        EntityKind::Cod        => ("entitymodel-cod.b3d"     , vec![String::from("entity-fish-cod.png")]),
        EntityKind::CommandBlockMinecart => ("entitymodel-minecart_block.b3d", vec![String::from("entity-minecart.png"), String::from("block-command_block_side.png")]),
        EntityKind::Cow        => ("entitymodel-cow.b3d"     , vec![String::from("entity-cow-cow.png"), String::from("block-red_mushroom.png^[opacity:0")]), // transparent
        EntityKind::Creeper    => ("entitymodel-creeper.b3d" , vec![String::from("entity-creeper-creeper.png")]),
        EntityKind::Dolphin    => ("entitymodel-dolphin.b3d" , vec![String::from("entity-dolphin.png")]),
        EntityKind::Donkey     => ("entitymodel-horse.b3d"   , vec![String::from("entity-horse-donkey.png")]),
        EntityKind::Drowned    => ("entitymodel-zombie.b3d"  , vec![String::from("entity-zombie-zombie.png")]), // drowned is a layered texture
        EntityKind::ElderGuardian => ("entitymodel-guardian.b3d", vec![String::from("entity-guardian_elder.png")]),
        EntityKind::EndCrystal => ("entitymodel-end_crystal.b3d", vec![String::from("entity-end_crystal-end_crystal.png")]),
        EntityKind::EnderDragon => ("entitymodel-dragon.b3d" , vec![String::from("entity-enderdragon-dragon.png")]),
        EntityKind::Enderman   => ("entitymodel-enderman.b3d", vec![String::from("entity-enderman-enderman.png")]),
        EntityKind::Endermite  => ("entitymodel-endermite.b3d", vec![String::from("entity-endermite.png")]),
        EntityKind::Evoker     => ("entitymodel-evoker.b3d"  , vec![String::from("entity-illager-evoker.png")]),
        EntityKind::Fox        => ("entitymodel-cat.b3d"     , vec![String::from("entity-fox-fox.png")]),
        EntityKind::FurnaceMinecart => ("entitymodel-minecart_block.b3d", vec![String::from("entity-minecart.png"), String::from("block-furnace_side.png")]),
        EntityKind::Ghast      => ("entitymodel-ghast.b3d"   , vec![String::from("entity-ghast-ghast.png")]),
        EntityKind::GlowSquid  => ("entitymodel-glow_squid.b3d", vec![String::from("entity-squid-glow_squid.png")]),
        EntityKind::Goat       => ("entitymodel-sheepfur.b3d", vec![String::from("entity-goat-goat.png")]),
        EntityKind::Guardian   => ("entitymodel-guardian.b3d", vec![String::from("entity-guardian.png")]),
        EntityKind::Hoglin     => ("entitymodel-hoglin.b3d"  , vec![String::from("entity-hoglin-hoglin.png")]),
        EntityKind::HopperMinecart => ("entitymodel-minecart_hopper.b3d", vec![String::from("entity-minecart.png")]),
        EntityKind::Horse      => ("entitymodel-horse.b3d"   , vec![String::from("entity-horse-horse_brown.png")]),
        EntityKind::Husk       => ("entitymodel-zombie.b3d"  , vec![String::from("entity-zombie-husk.png")]),
        EntityKind::Illusioner => ("entitymodel-illusioner.b3d", vec![String::from("entity-illager-illusioner.png")]),
        EntityKind::IronGolem  => ("entitymodel-iron_golem.b3d", vec![String::from("entity-iron_golem-iron_golem.png")]),
        EntityKind::Llama      => ("entitymodel-llama.b3d"   , vec![String::from("entity-llama-creamy.png")]),
        EntityKind::MagmaCube  => ("entitymodel-magmacube.b3d", vec![String::from("entity-slime-magmacube.png")]),
        EntityKind::Minecart   => ("entitymodel-minecart.b3d", vec![String::from("entity-minecart.png")]),
        EntityKind::Mooshroom  => ("entitymodel-cow.b3d"     , vec![String::from("entity-cow-red_mooshroom.png"), String::from("block-red_mushroom.png")]),
        EntityKind::Mule       => ("entitymodel-horse.b3d"   , vec![String::from("entity-horse-mule.png")]),
        EntityKind::Ocelot     => ("entitymodel-cat.b3d"     , vec![String::from("entity-cat-ocelot.png")]),
        EntityKind::Parrot     => ("entitymodel-parrot.b3d"  , vec![String::from("entity-parrot-parrot_red_blue.png")]),
        EntityKind::Pig        => ("entitymodel-pig.b3d"     , vec![String::from("entity-pig-pig.png")]),
        EntityKind::Piglin     => ("entitymodel-sword_piglin.b3d", vec![String::from("entity-piglin-piglin.png"), String::from("item-golden_sword.png")]),
        EntityKind::PiglinBrute => ("entitymodel-sword_piglin.b3d", vec![String::from("entity-piglin-piglin_brute.png"), String::from("item-golden_axe.png")]),
        EntityKind::Pillager   => ("entitymodel-pillager.b3d", vec![String::from("entity-illager-pillager.png"), String::from("item-crossbow_arrow.png")]),
        EntityKind::PolarBear  => ("entitymodel-polarbear.b3d", vec![String::from("entity-bear-polarbear.png")]),
        EntityKind::Rabbit     => ("entitymodel-rabbit.b3d"  , vec![String::from("entity-rabbit-brown.png")]),
        EntityKind::Salmon     => ("entitymodel-salmon.b3d"  , vec![String::from("entity-fish-salmon.png")]),
        EntityKind::Sheep      => ("entitymodel-sheepfur.b3d", vec![String::from("entity-sheep-sheep_fur.png"),String::from("entity-sheep-sheep.png")]),
        EntityKind::Shulker    => ("entitymodel-shulker.b3d" , vec![String::from("entity-shulker-shulker.png")]),
        EntityKind::Silverfish => ("entitymodel-silverfish.b3d", vec![String::from("entity-silverfish.png")]),
        EntityKind::Skeleton   => ("entitymodel-skeleton.b3d", vec![String::from("entity-skeleton-skeleton.png"), String::from("bow_pulling_2.png")]),
        EntityKind::Slime      => ("entitymodel-slime.b3d"   , vec![String::from("entity-slime-slime.png")]),
        EntityKind::SnowGolem  => ("entitymodel-snowman.b3d" , vec![String::from("entity-snow_golem.png")]),
        EntityKind::SpawnerMinecart => ("entitymodel-minecart_block.b3d", vec![String::from("entity-minecart.png"), String::from("block-spawner.png")]),
        EntityKind::Spider     => ("entitymodel-spider.b3d"  , vec![String::from("entity-spider-spider.png")]),
        EntityKind::Squid      => ("entitymodel-squid.b3d"   , vec![String::from("entity-squid-squid.png")]),
        EntityKind::Stray      => ("entitymodel-stray.b3d"   , vec![String::from("entity-skeleton-stray.png")]), //TODO layered
        EntityKind::Strider    => ("entitymodel-strider.b3d" , vec![String::from("entity-strider.png")]),
        EntityKind::TntMinecart => ("entitymodel-minecart_block.b3d", vec![String::from("entity-minecart.png"), String::from("block-tnt_side.png")]),
        EntityKind::TraderLlama => ("entitymodel-llama.b3d"  , vec![String::from("entity-llama-brown.png")]),
        EntityKind::TropicalFish => ("entitymodel-tropical_fish_a.b3d", vec![String::from("entity-fish-tropical_a.png")]), // a/b textures with patterns. no way am i going to deal with that
        EntityKind::Vex        => ("entitymodel-vex.b3d"     , vec![String::from("entity-illager-vex.png")]),
        EntityKind::Villager   => ("entitymodel-villager.b3d", vec![String::from("entity-villager-villager.png")]),
        EntityKind::Vindicator => ("entitymodel-vindicator.b3d", vec![String::from("entity-illager-vindicator.png"), String::from("item-iron_axe.png")]),
        EntityKind::WanderingTrader => ("entitymodel-villager.b3d", vec![String::from("entity-wandering_trader.png")]),
        EntityKind::Warden     => ("entitymodel-iron_golem.b3d", vec![String::from("entity-warden-warden.png")]),
        EntityKind::Witch      => ("entitymodel-witch.b3d"   , vec![String::from("entity-witch.png")]),
        EntityKind::Wither     => ("entitymodel-wither.b3d"  , vec![String::from("entity-wither-wither.png")]),
        EntityKind::WitherSkeleton => ("entitymodel-witherskeleton.b3d", vec![String::from("entity-skeleton-wither_skeleton.png")]),
        EntityKind::Wolf       => ("entitymodel-wolf.b3d"    , vec![String::from("entity-wolf-wolf.png")]),
        EntityKind::Zoglin     => ("entitymodel-hoglin.b3d"  , vec![String::from("entity-hoglin-zoglin.png")]),
        EntityKind::Zombie     => ("entitymodel-zombie.b3d"  , vec![String::from("entity-zombie-zombie.png")]),
        EntityKind::ZombieHorse => ("entitymodel-horse.b3d"  , vec![String::from("entity-horse-horse_zombie.png")]),
        EntityKind::ZombieVillager => ("entitymodel-villager.b3d", vec![String::from("entity-zombie_villager-zombie_villager.png")]),
        EntityKind::ZombifiedPiglin => ("entitymodel-sword_piglin.b3d", vec![String::from("entity-piglin-zombified_piglin.png"), String::from("item-golden_sword.png")]),
        EntityKind::Player     => ("entitymodel-armor_character.b3d", vec![String::from("entity-player-wide-steve.png")]),
        _                      => ("entitymodel-pig.b3d"     , vec![String::from("entity-pig-pig.png")])
    }
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
