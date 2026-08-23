/*
 * This file contains shared functions, for example logging
 */

use crate::s2c;
use crate::settings;
use crate::state;

use azalea::BlockPos;
use azalea::Client;
use azalea::block::BlockState;
use azalea::core::{aabb::Aabb, bitset::BitSet, position::Vec3};
use azalea::events::Event;
use azalea::inventory::ItemStack;
use azalea::registry::Registry;
use azalea::registry::builtin::{BlockKind, EntityKind};
use log::*;
use luanti_core::ContentId;
use luanti_core::MapNode;
use minecraft_data_rs::models::version::Version;
use minecraft_data_rs::{Api, api};
use rand::RngExt;
use s2c::media::LuantiTexture;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::LazyLock;

use glam::Vec3 as v3f;

// modified version of the liang-barsky line clipping algo
// adapted to work in 3d and also to return a simple boolean indicating if the line clips at all.
// also makes the bounding box a little higher to account for some weird graphics
pub fn liang_barsky_3d(mut bb: Aabb, line_a: Vec3, line_b: Vec3) -> bool {
    let mut t0 = 0.0;
    let mut t1 = 1.0;

    bb.max.y += 1.0;
    bb.min.y -= 0.5;

    let dx = line_b.x - line_a.x;
    let dy = line_b.y - line_a.y;
    let dz = line_b.z - line_a.z;

    let clipping_edges = [
        (-dx, line_a.x - bb.min.x.min(bb.max.x)),
        (dx, bb.max.x.max(bb.min.x) - line_a.x),
        (-dy, line_a.y - bb.min.y.min(bb.max.y)),
        (dy, bb.max.y.max(bb.min.y) - line_a.y),
        (-dz, line_a.z - bb.min.z.min(bb.max.z)),
        (dz, bb.max.z.max(bb.min.z) - line_a.z),
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

// translate between luanti/minecraft
// the two programs disagree on X handedness, so a bunch of stuff needs converting

// MC uses block corners, luanti uses centers
// irrelevant for the map, but entities need this converted
// also do handedness change
pub fn mirror_pos(x: f32) -> f32 {
    -x - 0.5
}

// block-grid alignment
pub fn align_pos(v: f32) -> f32 {
    v - 0.5
}

/// mirror an integer block/chunk-index X coordinate, staying aligned to 16-node chunk boundaries
pub fn mirror_block_pos(x: i32) -> i32 {
    -x - 1
}

/// mirror a direction-like (velocity, accel)
pub fn mirror_vec(x: f32) -> f32 {
    -x
}

/// mirror a yaw angle in degrees
pub fn mirror_yaw(yaw: f32) -> f32 {
    -yaw
}

pub fn allocate_id(serverside_id: u32, entity_state: &mut state::EntityState) -> u16 {
    // pick smallest range
    let i_smallest_range = entity_state
        .c_alloc_id_ranges
        .iter()
        .enumerate()
        .min_by_key(|&(_, &(start, end))| end - start)
        .map(|(index, _)| index)
        .expect("Client exhausted all available entity IDs!");
    // pick new ID
    let clientside_id: u16 = entity_state.c_alloc_id_ranges[i_smallest_range].0;
    // resize range
    if entity_state.c_alloc_id_ranges[i_smallest_range].0
        == entity_state.c_alloc_id_ranges[i_smallest_range].1
    {
        entity_state.c_alloc_id_ranges.remove(i_smallest_range);
    } else {
        entity_state.c_alloc_id_ranges[i_smallest_range].0 += 1;
    }
    // add ID pair to maps, return
    entity_state
        .entity_id_map
        .insert(serverside_id.into(), clientside_id);
    return clientside_id;
}

pub fn free_id(serverside_id: u32, entity_state: &mut state::EntityState) {
    // remove from maps
    let id_pair = entity_state
        .entity_id_map
        .remove_by_left(&serverside_id.into());
    entity_state
        .entities_update_scheduled
        .retain(|x| *x != serverside_id.into()); // may be scheduled several times
    // HeadYaw lives on the ECS entity itself (see state::HeadYaw), azalea drops it
    // along with everything else when it despawns the entity for us
    // add new range and re-optimize the ranges
    match id_pair {
        Some((_, clientside_id)) => {
            entity_state
                .c_alloc_id_ranges
                .push((clientside_id, clientside_id));
            defrag_ranges(entity_state);
        }
        None => (),
    }
    // drop any leftover appearance bookkeeping for the freed entity
    entity_state
        .appearance_update_scheduled
        .retain(|x| *x != serverside_id.into());
    entity_state.entity_appearance.remove(&serverside_id.into());
}

fn defrag_ranges(entity_state: &mut state::EntityState) {
    entity_state.c_alloc_id_ranges.sort_by_key(|r| r.0);
    let mut index_lim = entity_state.c_alloc_id_ranges.len() - 1;
    let mut p = entity_state.c_alloc_id_ranges[0];
    let mut r_index: usize = 1;
    loop {
        if r_index > index_lim {
            break;
        }
        let r = entity_state.c_alloc_id_ranges[r_index];
        if r.0 == p.1 + 1 {
            entity_state.c_alloc_id_ranges[r_index - 1].1 = r.1;
            entity_state.c_alloc_id_ranges.remove(r_index);
            index_lim -= 1;
            p = (p.0, r.1);
        } else {
            p = r;
            r_index += 1;
        }
    }
}

pub fn texture_from_itemstack(item: &ItemStack, media_state: &state::MediaState) -> String {
    match item {
        ItemStack::Empty => String::from("air.png"),
        ItemStack::Present(slot_data) => {
            let item_name = slot_data.kind.to_string();
            let inventory_image: String;
            if media_state.item_texture_map.contains_key(&item_name) {
                inventory_image = media_state
                    .item_texture_map
                    .get(&item_name)
                    .unwrap()
                    .clone()
                    .to_luanti_safe();
            } else {
                inventory_image = media_state
                    .block_texture_map
                    .get(&item_name)
                    .unwrap()
                    .clone()
                    .to_safe_cube();
            }
            return inventory_image;
        }
    }
}

pub fn state_to_node(state: BlockState, cave_air_glow: bool) -> MapNode {
    let mut param0: u16;
    let param1: u8;
    let param2: u8 = 0;
    param0 = BlockKind::try_from(state).unwrap().to_u32() as u16 + 128;

    // param1 (CPT_LIGHT): lower nibble = day/sky light, upper nibble = block light
    if state.is_air() {
        param0 = 126;
        param1 = 0xEE;
    } else if (BlockKind::try_from(state).unwrap() == BlockKind::CaveAir) && cave_air_glow {
        param0 = 120; // custom node: glowing_air, used in nether
        param1 = 0xEE;
    } else {
        // no light context available on this code path; assume full sky light so the
        // client's sky ray-march (and lighting) works. blockupdate/section_block_update
        // could refine this later.
        param1 = 0xEE;
    }

    MapNode {
        content_id: ContentId(param0),
        param1,
        param2,
    }
}

/// Decode the MC "Section Light" layer into per-section light levels (0..15).
///
/// `layers` holds one 2048-byte layer per set bit in `y_mask` (in ascending bit order,
/// starting from the lowest value). Each layer stores the light for a single 16-high
/// chunk section: two X-values per byte, with y and z as the outer loops.
///
/// `y_mask` has one bit per world section plus 2: bit 0 covers the section *one below*
/// the min world height and the topmost bit covers the section *one above* the max.
/// Thus mask bit `b` corresponds to world section index `(b - 1)`. Sections outside
/// `[min_y, max_y)` are left all-zero.
///
/// Returns one `[u8; 4096]` per 16-high section (index = x + y*16 + z*256), matching
/// the node layout used by initialize_16node_chunk.
pub fn decode_light_layers(
    layers: &[Box<[u8]>],
    y_mask: &BitSet,
    _min_y: i32,
    _max_y: i32, // exclusive
    num_sections: usize,
) -> Vec<[u8; 4096]> {
    let mut out: Vec<[u8; 4096]> = vec![[0u8; 4096]; num_sections];

    // Walk set bits in ascending order; the i-th set bit maps to layers[i].
    for (layer_idx, bit) in y_mask.iter_ones().enumerate() {
        if layer_idx >= layers.len() || layers[layer_idx].len() != 2048 {
            break;
        }
        let data: &[u8] = &layers[layer_idx];
        // mask bit `bit` -> world section index (bit - 1)
        let section_index = (bit as i32) - 1;
        if section_index < 0 || section_index >= num_sections as i32 {
            continue;
        }
        let local_section = section_index as usize;
        let levels = &mut out[local_section];
        for y in 0..16usize {
            for z in 0..16usize {
                for x in (0..16usize).step_by(2) {
                    let byte = data[((y * 16 + z) * 8) + (x / 2)];
                    let i_lo = x + y * 16 + z * 256;
                    let i_hi = (x + 1) + y * 16 + z * 256;
                    levels[i_lo] = byte & 0xF;
                    levels[i_hi] = (byte >> 4) & 0xF;
                }
            }
        }
    }
    out
}

pub fn vec3_to_v3f(input_vector: &Vec3, scale: i32) -> v3f {
    // loss of precision, f64 -> f32
    let Vec3 {
        x: xf64,
        y: yf64,
        z: zf64,
    } = input_vector;
    v3f {
        x: (*xf64 * scale as f64) as f32,
        y: (*yf64 * scale as f64) as f32,
        z: (*zf64 * scale as f64) as f32,
    }
}

pub fn get_colormap(texture: &LuantiTexture) -> Option<(u8, u8, u8)> {
    // use the "Plains" texture. per-biome textures dont really work in mt afaik
    // https://minecraft.fandom.com/wiki/Color#Block_and_fluid_colors - what blocks use the colormaps
    // https://minecraft.fandom.com/wiki/Block_colors                 - what colors are to be used
    let r_texture = texture.to_luanti_safe();
    let name = r_texture.as_str();
    let grass_group = [
        "block-grass_block_top.png",
        "block-grass_block_side_overlay.png",
        "block-short_grass.png",
        "block-tall_grass_bottom.png",
        "block-tall_grass_top.png",
        "block-fern.png",
        "block-large_fern_bottom.png",
        "block-large_fern_top.png",
    ];
    if grass_group.contains(&name) {
        return Some((0x91, 0xBD, 0x59));
    }
    let foliage_group = [
        "block-oak_leaves.png",
        "block-jungle_leaves.png",
        "block-acacia_leaves.png",
        "block-dark_oak_leaves.png",
        "block-vine.png",
    ];
    if foliage_group.contains(&name) {
        return Some((0x77, 0xAB, 0x2F));
    }
    let water_group = ["block-water_still.png", "block-water_flow.png"];
    if water_group.contains(&name) {
        return Some((0x3F, 0x76, 0xE4));
    }
    let stem_group = [
        "block-attached_melon_stem.png",
        "block-attached_pumpkin_stem.png",
        "block-melon_stem.png",
        "block-pumpkin_stem.png",
        "pink_petals_stem.png",
    ];
    if stem_group.contains(&name) {
        return Some((0xE0, 0xC7, 0x1C));
    }
    // these textures are colormapped but constant for some stupid reason
    if name == "block-birch_leaves.png" {
        return Some((0x80, 0xA7, 0x55));
    }
    if name == "block-spruce_leaves.png" {
        return Some((0x61, 0x99, 0x61));
    }
    if name == "block-lily_pad.png" {
        return Some((0x20, 0x80, 0x30));
    }
    None
}

pub fn get_block_at(mc_client: &mut Client, pos: &BlockPos) -> Option<BlockKind> {
    let world_lock = mc_client.world().unwrap();
    let world = world_lock.read();
    let state = world.get_block_state(*pos);
    if let Some(state_u) = state {
        return Some(BlockKind::from(state_u));
    } else {
        return None;
    }
}

pub fn get_random_username() -> String {
    let hs_name = String::from(settings::HS_NAMES[rand::rng().random_range(0..26)]);
    format!("{}{:0>3}", hs_name, rand::rng().random_range(0..1000))
}

pub fn mc_packet_name(command: &Event) -> String {
    return String::from(match command {
        Event::Init => "Init",
        Event::Login => "Login",
        Event::Spawn => "Spawn",
        Event::Chat(_) => "Chat",
        Event::Tick => "Tick",
        // There are 117 possible cases here
        // pattern matching would get really boring
        Event::Packet(packet) => {
            let s = format!("{:?}", **packet);
            return s
                .split('(')
                .next() // for data variants
                .or_else(|| s.split_whitespace().next()) // for unit variants
                .unwrap()
                .to_owned();
        }
        Event::AddPlayer(_) => "AddPlayer",
        Event::RemovePlayer(_) => "RemovePlayer",
        Event::UpdatePlayer(_) => "UpdatePlayer",
        Event::Death(_) => "Death",
        Event::KeepAlive(_) => "KeepAlive",
        Event::Disconnect(_) => "Disconnect",
        _ => "Unknown", // should be exhaustive idk what the compiler wants here
    });
}

// select data API (from https://github.com/PrismarineJS/minecraft-data) based on azalea version
// Basically Api::latest() but compatible with azalea

pub fn compatible_data_api() -> Api {
    return Api::latest().expect("Found no Rust data version!");
    //FIXME Api::latest good enough for now, this otherwise somehow manages to return a broken api
    let Ok(versions) = api::versions() else {
        error!("Failed to retrieve minecraft data versions!");
        std::process::exit(1)
    };
    assert!(versions.len() != 0);
    let azalea_ver = azalea::protocol::packets::PROTOCOL_VERSION;
    let mut closest_match: Option<Version> = None;
    for version in versions {
        let closest_match_proto = match closest_match {
            Some(ref v) => v.version,
            None => 0,
        };
        if azalea_ver >= version.version && version.version > closest_match_proto {
            closest_match = Some(version);
        };
    }
    return Api::new(closest_match.expect("Found no version possibly matching azalea!"));
}


// Helpers for extra_data/entity_info.json
#[derive(Debug, Clone, Deserialize)]
pub enum SwivelAxis {
    Y,
    Z,
}

// entity_info allows offsetting head yaw
fn default_head_phase() -> f32 {
    0.0
}
// and body yaw
fn default_body_phase() -> f32 {
    0.0
}

#[derive(Debug, Clone, Deserialize)]
pub struct SwivelInfo {
    pub bone: String,
    pub axis: SwivelAxis,
    #[serde(default = "default_head_phase")]
    pub phase: f32,
}

// what an entity visual is made of
// required on all the entries in entity_info
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VisualKind {
    /// a plain texture
    Texture,
    /// 6 textures in a cube
    Block,
    /// a .b3d model with texture slots
    Model,
}

// all optional here, as variants compose on the base
// does mean that invalid base entries need to be caught, but still cool to use one type
//
// Textures are a list of texture slots, with each slot defining a base texture and any number of overlays
// (or an empty base texture, to just compose overlays onto the real base (for variants (hey cool i love nesting parens)))
#[derive(Debug, Deserialize)]
pub(crate) struct RawEntityInfo {
    #[serde(rename = "type")]
    pub(crate) visual: Option<VisualKind>,
    pub(crate) model: Option<String>,
    pub(crate) textures: Option<Vec<Vec<String>>>,
    pub(crate) size: Option<[f32; 3]>,
    head_swivel: Option<SwivelInfo>,
    #[serde(default = "default_body_phase")]
    body_phase: f32,
}

// split in two files purely for readability
// entity_info holds basic info per entity kind, entity_variants has variants (surprisingly)
static ENTITY_INFO: LazyLock<HashMap<String, RawEntityInfo>> = LazyLock::new(|| {
    let base_data = include_str!("../extra_data/entity_info.json");
    let variants_data = include_str!("../extra_data/entity_variants.json");
    let mut info: HashMap<String, RawEntityInfo> =
        serde_json::from_str(base_data).expect("extra_data/entity_info.json is invalid");
    let variants: HashMap<String, RawEntityInfo> =
        serde_json::from_str(variants_data).expect("extra_data/entity_variants.json is invalid");
    let (base_len, variants_len) = (info.len(), variants.len());
    info.extend(variants);
    assert_eq!(
        info.len(),
        base_len + variants_len,
        "extra_data/entity_info.json and entity_variants.json have overlapping keys"
    );
    info
});

fn entity_info(entity: EntityKind) -> &'static RawEntityInfo {
    ENTITY_INFO.get(entity.to_str()).unwrap_or_else(|| {
        ENTITY_INFO
            .get("_default")
            .expect("extra_data/entity_info.json missing a \"_default\" entry")
    })
}

// raw lookup by literal key (bare or "kind+variant")
// shouldn't be used to do anything but implement the fallback-having lookup
pub(crate) fn entity_info_by_key(key: &str) -> Option<&'static RawEntityInfo> {
    ENTITY_INFO.get(key)
}

pub fn get_head_swivel(entity: EntityKind) -> Option<SwivelInfo> {
    entity_info(entity).head_swivel.clone()
}

pub fn get_body_phase(entity: EntityKind) -> f32 {
    entity_info(entity).body_phase
}
