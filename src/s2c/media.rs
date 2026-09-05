// code to get media to the client
use crate::{settings, utils};
use glam::Vec3 as v3f;
use log::*;
use luanti_protocol::commands::client_to_server;
use luanti_protocol::commands::{server_to_client, server_to_client::ToClientCommand};
use luanti_protocol::types::{
    AlignStyle, DrawType, MediaAnnouncement, MediaFileData, NodeBox, NodeBoxFixed,
    TileAnimationParams, TileDef, aabb3f,
};
use serde::Deserialize;
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::io::{Cursor, Read};
use std::path::PathBuf;
use zip::read::root_dir_common_filter;

// Helpers for our various stored data
fn data_root() -> PathBuf {
    dirs::data_local_dir().unwrap().join("bridgetest/")
}
fn texture_root() -> PathBuf {
    data_root().join("textures/")
}
fn assets_root() -> PathBuf {
    data_root().join("bridgetest_assets/")
}
fn asset_model_root() -> PathBuf {
    assets_root().join("models/")
}
fn asset_texture_root() -> PathBuf {
    assets_root().join("textures/")
}

// resolves ambiguity in mapping minecraft:thing to textures
// important! only stores paths relative to the texture pack root (or the model root, for models).
#[derive(Clone, Eq, PartialEq, Hash, Debug, Deserialize)]
pub struct LuantiTexture {
    rel_path: String,
}

impl LuantiTexture {
    pub fn get_relative(&self) -> &str {
        return &self.rel_path;
    }
    // models always in bridgetest_assets
    // textures can be either, prefer texture pack if available
    pub fn get_absolute(&self, model_mode: bool) -> PathBuf {
        if model_mode {
            return asset_model_root().join(PathBuf::from(&self.rel_path));
        }
        let in_pack = texture_root().join(PathBuf::from(&self.rel_path));
        if in_pack.exists() {
            return in_pack;
        }
        return asset_texture_root().join(PathBuf::from(&self.rel_path));
    }
    // ./block/thing.png -> block-thing.png
    // we need to keep the extension, luanti relies on that for file type
    pub fn to_luanti_safe(&self) -> String {
        return self.get_relative().replace("./", "").replace("/", "-");
    }
    pub fn from_luanti_safe(safe_texture: &str) -> LuantiTexture {
        let rel_path = format!("./{}", safe_texture.replace("-", "/"));
        return LuantiTexture { rel_path };
    }
    pub fn from_string(rpath: &str) -> LuantiTexture {
        LuantiTexture {
            rel_path: String::from(rpath),
        }
    }
    pub fn from_absolute(apath: PathBuf, model_mode: bool) -> LuantiTexture {
        let relative_root = if model_mode {
            asset_model_root()
        } else if apath.starts_with(texture_root()) {
            texture_root()
        } else {
            asset_texture_root()
        };
        let rel_path = apath
            .strip_prefix(relative_root)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        LuantiTexture { rel_path }
    }
}

pub fn get_announcement() -> ToClientCommand {
    let mut announcement_vec: Vec<MediaAnnouncement> = Vec::new();
    // add textures
    for root in [texture_root(), asset_texture_root()] {
        for texture in get_texture_iterator_recursive(root, settings::TEXTURE_MAX_RECURSION, false)
        {
            announcement_vec.push(MediaAnnouncement {
                name: String::from(texture.to_luanti_safe()),
                sha1: get_sha1(&texture.get_absolute(false)),
            });
        }
    }
    // add models
    for model in get_texture_iterator_recursive(asset_model_root(), 2, true) {
        announcement_vec.push(MediaAnnouncement {
            name: model.to_luanti_safe(),
            sha1: get_sha1(&model.get_absolute(true)),
        });
    }
    ToClientCommand::AnnounceMedia(Box::new(server_to_client::AnnounceMediaSpec {
        files: announcement_vec,
        remote_servers: String::from(""),
    }))
}

pub fn get_texture_iterator_recursive(
    path: PathBuf,
    limit: u8,
    model_mode: bool,
) -> Vec<LuantiTexture> {
    let mut ret: Vec<LuantiTexture> = Vec::new();
    if limit == 0 {
        return ret;
    };
    for entry in path.read_dir().unwrap() {
        let entry_u = entry.unwrap();
        if entry_u.file_type().unwrap().is_dir() {
            ret.extend(get_texture_iterator_recursive(
                entry_u.path(),
                limit - 1,
                model_mode,
            ));
        };
        if !model_mode && entry_u.path().extension() != Some(&OsStr::new("png")) {
            continue;
        };
        if model_mode && entry_u.path().extension() != Some(&OsStr::new("b3d")) {
            continue;
        }
        ret.push(LuantiTexture::from_absolute(entry_u.path(), model_mode));
    }
    return ret;
}

fn get_sha1(path: &PathBuf) -> [u8; 20] {
    let mut file_handle;
    let metadata;
    file_handle = fs::File::open(path).unwrap();
    metadata = fs::metadata(path).expect("Unable to read File Metadata! (Check Permissions?)");
    let mut buffer = vec![0; metadata.len() as usize];
    file_handle.read_exact(&mut buffer).unwrap();
    let mut hasher = Sha1::new();
    hasher.update(buffer);
    hasher.finalize().into()
}

pub fn handle_request(specbox: Box<client_to_server::RequestMediaSpec>) -> ToClientCommand {
    let client_to_server::RequestMediaSpec { files } = *specbox;
    let mut file_data: Vec<MediaFileData> = Vec::new();
    for file_name in files {
        let texture = LuantiTexture::from_luanti_safe(&file_name);
        let model_mode = file_name.ends_with(".b3d");
        let path = texture.get_absolute(model_mode);
        let mut file_handle = fs::File::open(&path).unwrap();
        let metadata =
            fs::metadata(&path).expect("Unable to read File Metadata! (Check Permissions?)");
        let mut buffer = vec![0; metadata.len() as usize];
        file_handle.read_exact(&mut buffer).unwrap();
        file_data.push(MediaFileData {
            name: file_name,
            data: buffer,
        })
    }
    ToClientCommand::Media(Box::new(server_to_client::MediaSpec {
        num_bunches: 1,
        bunch_index: 0,
        files: file_data,
    }))
}

// parse block texture map
#[derive(Debug, Deserialize, Eq, PartialEq, Hash, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Up = 0,
    Down,
    North,
    South,
    East,
    West,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "lowercase", tag = "drawtype", content = "nodebox")]
pub enum RawDrawType {
    Full,
    Air,
    Flower,
    Fire,
    Liquid,
    #[serde(rename = "NB_")]
    NodeBox(String),
}
impl RawDrawType {
    pub fn compile(&self, nodebox_mapping: &HashMap<String, NodeBox>) -> (DrawType, NodeBox) {
        let dt = match self {
            RawDrawType::Air => DrawType::AirLike,
            RawDrawType::Fire => DrawType::FireLike,
            RawDrawType::Flower => DrawType::PlantLike,
            RawDrawType::Full => DrawType::Normal,
            RawDrawType::Liquid => DrawType::Liquid,
            RawDrawType::NodeBox(_) => DrawType::NodeBox,
        };
        if let RawDrawType::NodeBox(a) = self {
            return (dt, nodebox_mapping.get(a).unwrap().clone());
        } else {
            return (dt, NodeBox::Regular);
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlockMapping {
    textures: HashMap<Direction, LuantiTexture>,
    pub drawtype: DrawType,
    pub nodebox: NodeBox,
    // true if any face texture has alpha transparency
    pub cutout: bool,
}

// gets the variant whose keys "prop=val" pairs are all satisfied by the state_key pairs
// (a stored key may omit properties Mojang's own blockstate JSON never mentions)
// prefer the matching key with the most pairs (-> specific). Falls back to the protpertyless variant
pub fn lookup_block_mapping<'a>(
    block_texture_map: &'a HashMap<String, HashMap<String, BlockMapping>>,
    block_name: &str,
    state_key: &str,
) -> Option<&'a BlockMapping> {
    let variants = block_texture_map.get(block_name)?;
    let state_pairs: Vec<&str> = state_key.split(',').filter(|s| !s.is_empty()).collect();
    variants
        .iter()
        .filter_map(|(key, mapping)| {
            let pairs: Vec<&str> = key.split(',').filter(|s| !s.is_empty()).collect();
            pairs
                .iter()
                .all(|pair| state_pairs.contains(pair))
                .then_some((pairs.len(), mapping))
        })
        .max_by_key(|(specificity, _)| *specificity)
        .map(|(_, mapping)| mapping)
        .or_else(|| variants.get(""))
        .or_else(|| variants.values().next())
}

impl BlockMapping {
    pub fn get_tiledefs(&self, animation: &TileAnimationParams) -> [TileDef; 6] {
        let mut ret_vec: Vec<TileDef> = Vec::new();
        for i in 0..=5 {
            let direction = match i {
                0 => Direction::Up,
                1 => Direction::Down,
                2 => Direction::North,
                3 => Direction::South,
                4 => Direction::East,
                5 => Direction::West,
                _ => unreachable!(),
            };
            let texture = self.textures.get(&direction).unwrap();
            ret_vec.push(TileDef {
                name: texture.to_luanti_safe(),
                animation: animation.clone(),
                // PlantLike can't have backface culling or it'll cull itself
                // (not entirely, but it's an X shape and one plane culls the other)
                backface_culling: self.drawtype != DrawType::PlantLike,
                tileable_horizontal: false,
                tileable_vertical: false,
                color_rgb: utils::get_colormap(texture),
                scale: 0,
                align_style: AlignStyle::Node,
            })
        }
        let ret: [TileDef; 6] = ret_vec.as_array().unwrap().clone();
        return ret;
    }
    // the 6 face textures in the same order get_tiledefs uses,
    // used for active objects with "cube" visual
    pub fn to_entity_textures(&self) -> [String; 6] {
        [
            Direction::Up,
            Direction::Down,
            Direction::North,
            Direction::South,
            Direction::East,
            Direction::West,
        ]
        .map(|d| self.textures.get(&d).unwrap().to_luanti_safe())
    }
    pub fn to_safe_cube(&self) -> String {
        return format!(
            "[inventorycube{{{}{{{}{{{}",
            self.textures.get(&Direction::Up).unwrap().to_luanti_safe(),
            self.textures
                .get(&Direction::North)
                .unwrap()
                .to_luanti_safe(),
            self.textures
                .get(&Direction::East)
                .unwrap()
                .to_luanti_safe()
        );
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawTextures {
    // Uniform is just a shorter version for having all Faces be identical
    Uniform(String),
    PerFace(HashMap<Direction, String>),
}

impl RawTextures {
    fn into_map(self) -> HashMap<Direction, String> {
        match self {
            RawTextures::PerFace(map) => map,
            RawTextures::Uniform(tex) => [
                Direction::Up,
                Direction::Down,
                Direction::North,
                Direction::South,
                Direction::East,
                Direction::West,
            ]
            .into_iter()
            .map(|dir| (dir, tex.clone()))
            .collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawBlockMapping {
    textures: RawTextures,
    drawtype: String,
    #[serde(default)]
    cutout: bool,
}

// Every BlockState of a block gets its own entry, keyed by
// "prop1=val1,prop2=val2" like in Minecraft blockstate JSON (alphabetic)
// See utils::variant_key_from_state where the key is built
pub fn load_block_mappings(
    nodebox_mapping: &HashMap<String, NodeBox>,
) -> HashMap<String, HashMap<String, BlockMapping>> {
    let data = include_bytes!("../../extra_data/block_texture_map.json");
    let raw_map: HashMap<String, HashMap<String, RawBlockMapping>> =
        serde_json::from_slice(data).unwrap();
    raw_map
        .into_iter()
        .map(|(block_name, variants)| {
            let compiled_variants = variants
                .into_iter()
                .map(|(state_key, v)| {
                    let textures = v
                        .textures
                        .into_map()
                        .into_iter()
                        .map(|(dir, tex)| (dir, LuantiTexture::from_string(&tex)))
                        .collect();
                    let drawtype = match v.drawtype.as_str() {
                        "full" => RawDrawType::Full,
                        "air" => RawDrawType::Air,
                        "flower" => RawDrawType::Flower,
                        "fire" => RawDrawType::Fire,
                        "liquid" => RawDrawType::Liquid,
                        _ if v.drawtype.starts_with("NB_") => RawDrawType::NodeBox(v.drawtype),
                        _ => unreachable!(),
                    };
                    let (drawtype, nodebox) = drawtype.compile(nodebox_mapping);
                    let mapped = BlockMapping {
                        textures,
                        drawtype,
                        nodebox,
                        cutout: v.cutout,
                    };
                    (state_key, mapped)
                })
                .collect();
            (block_name, compiled_variants)
        })
        .collect()
}

// See extra_data/block_info.json
// Edge cases that cant easily be read from the minecraft client jar
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BlockInfo {
    #[serde(default)]
    pub light_source: u8,
    // used when the state has the "lit" property true
    pub lit_light_source: Option<u8>,
    #[serde(default)]
    pub waving: bool,
    #[serde(default)]
    pub climbable: bool,
    // full-cube block with seethrough (also slime and some other)
    #[serde(default)]
    pub glasslike: bool,
    // full-cube block with alpha
    #[serde(default)]
    pub cutout: bool,
}

pub fn load_block_info() -> HashMap<String, BlockInfo> {
    let data = include_bytes!("../../extra_data/block_info.json");
    serde_json::from_slice(data).expect("extra_data/block_info.json is invalid")
}

pub fn load_item_mappings() -> HashMap<String, LuantiTexture> {
    let data = include_bytes!("../../extra_data/item_texture_map.json");
    let raw_map: HashMap<String, String> = serde_json::from_slice(data).unwrap();
    let parsed_map = raw_map
        .into_iter()
        .map(|(k, v)| (k, LuantiTexture::from_string(&v)))
        .collect();
    return parsed_map;
}

// magic value
// setting this to 1 exactly causes really weird texture issues (see for yourself if you must).
// this value is the closest to 1 that works (really. 1.0009 fails).
// this adds some mostly invisible inaccuracies, but that's fine
// i wasted 6 hours trying to "fix" this.
// just do not touch it. the luanti codebase contains The Horrors™
pub const NB_SCALE_FACTOR: f32 = 1.001;
fn generate_nodebox(cuboids: Vec<[i8; 6]>) -> NodeBox {
    let mut ab_bounds: Vec<aabb3f> = Vec::new();
    for cuboid in cuboids {
        let sf_a: f32 = 1.6 * NB_SCALE_FACTOR;
        let sf_b: f32 = 5.0 / NB_SCALE_FACTOR;
        ab_bounds.push(aabb3f {
            min_edge: (v3f {
                x: cuboid[0] as f32 / sf_a - sf_b,
                y: cuboid[1] as f32 / sf_a - sf_b,
                z: cuboid[2] as f32 / sf_a - sf_b,
            }),
            max_edge: (v3f {
                x: cuboid[3] as f32 / sf_a - sf_b,
                y: cuboid[4] as f32 / sf_a - sf_b,
                z: cuboid[5] as f32 / sf_a - sf_b,
            }),
        })
    }
    return NodeBox::Fixed(NodeBoxFixed { fixed: ab_bounds });
}

pub fn load_nodeboxes() -> HashMap<String, NodeBox> {
    let data = include_bytes!("../../extra_data/nodeboxes.json");
    let raw_map: HashMap<String, Vec<[i8; 6]>> = serde_json::from_slice(data).unwrap();
    let parsed_map = raw_map
        .into_iter()
        .map(|(k, v)| (k, generate_nodebox(v)))
        .collect();
    return parsed_map;
}

pub fn get_empty_tiledefs() -> [TileDef; 6] {
    let td = TileDef {
        name: String::from(""),
        animation: TileAnimationParams::None,
        backface_culling: false,
        tileable_horizontal: true,
        tileable_vertical: true,
        color_rgb: None,
        scale: 1,
        align_style: AlignStyle::Node,
    };
    return [
        td.clone(),
        td.clone(),
        td.clone(),
        td.clone(),
        td.clone(),
        td,
    ];
}

// fetches entity/boat models and textures from the bridgetest_assets repo
pub async fn fetch_media() {
    let assets_dir = assets_root();
    let _ = std::fs::create_dir_all(&assets_dir);
    let version_file = assets_dir.join("version.txt");

    let installed_version: i32 = fs::read_to_string(&version_file)
        .ok()
        .and_then(|version| version.trim().parse().ok())
        .unwrap_or(0);
    // backwards compatibility assumed, don't downgrade
    if installed_version >= settings::BRIDGETEST_ASSETS_VER {
        debug!(
            "Not downloading bridgetest_assets (have {}, need {})",
            installed_version,
            settings::BRIDGETEST_ASSETS_VER
        );
        return;
    }

    warn!(
        "bridgetest_assets missing/outdated (have {}, need {}), downloading ({})",
        installed_version,
        settings::BRIDGETEST_ASSETS_VER,
        settings::BRIDGETEST_ASSETS_URL
    );
    let resp = reqwest::get(settings::BRIDGETEST_ASSETS_URL)
        .await
        .unwrap_or_else(|_| {
            error!("Failed to get bridgetest_assets. Check network conenction?");
            std::process::exit(1)
        });
    let archive_data = Cursor::new(resp.bytes().await.unwrap());
    debug!("Extracting downloaded zip file...");
    let archive = zip::ZipArchive::new(archive_data);
    archive
        .expect("Could not decompress media, file not a valid zip archive?")
        .extract_unwrapped_root_dir(&assets_dir, root_dir_common_filter)
        .expect("Could not decompress media!");

    let found_models = get_texture_iterator_recursive(asset_model_root(), 2, true);
    let found_textures = get_texture_iterator_recursive(
        asset_texture_root(),
        settings::TEXTURE_MAX_RECURSION,
        false,
    );
    info!(
        "bridgetest_assets downloaded! ({} models, {} textures available)",
        found_models.len(),
        found_textures.len()
    );
}
