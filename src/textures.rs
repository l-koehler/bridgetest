// code to get media to the client
use base64::{Engine, engine::general_purpose};
use glam::Vec3 as v3f32;
use luanti_protocol::commands::client_to_server;
use luanti_protocol::commands::{server_to_client, server_to_client::ToClientCommand};
use luanti_protocol::types::{
    AlignStyle, DrawType, MediaAnnouncement, MediaFileData, NodeBox, NodeBoxFixed,
    TileAnimationParams, TileDef, aabb3f,
};
use serde::Deserialize;
use serde_json;
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::io::{Cursor, Read};
use std::path::PathBuf;

use crate::utils::{find_suffix_match, sanitize_model_name};
use crate::{settings, utils};

// resolves ambiguity in mapping minecraft:thing to textures
// important! only stores paths relative to the texture pack root.
// Stores models using the fake ./model/ path
#[derive(Clone, Eq, PartialEq, Hash, Debug, Deserialize)]
pub struct LuantiTexture {
    rel_path: String,
}

impl LuantiTexture {
    pub fn get_relative(&self) -> &str {
        return &self.rel_path;
    }
    pub fn get_absolute(&self, model_mode: bool) -> PathBuf {
        let relative_root;
        if model_mode {
            relative_root = dirs::data_local_dir().unwrap().join("bridgetest/models/");
            // remove "fake" model/ path
            // "desanitize" name, glob by *file_post
            let file_post = &self.rel_path.replace("./model/", "");
            return find_suffix_match(&relative_root, &file_post).unwrap();
        } else {
            relative_root = dirs::data_local_dir().unwrap().join("bridgetest/textures/");
            return relative_root.join(PathBuf::from(&self.rel_path));
        }
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
        let relative_root;
        if model_mode {
            relative_root = dirs::data_local_dir().unwrap().join("bridgetest/models/");
        } else {
            relative_root = dirs::data_local_dir().unwrap().join("bridgetest/textures/");
        }
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
    // add textures (block-thing.png)
    let texture_root = dirs::data_local_dir().unwrap().join("bridgetest/textures/");
    for texture in
        get_texture_iterator_recursive(texture_root, settings::TEXTURE_MAX_RECURSION, false)
    {
        announcement_vec.push(MediaAnnouncement {
            name: String::from(texture.to_luanti_safe()),
            sha1_base64: get_sha1_base64(&texture.get_absolute(false)),
        });
    }
    // add models (model-thing.b3d)
    let model_root = dirs::data_local_dir().unwrap().join("bridgetest/models/");
    for model in get_texture_iterator_recursive(model_root, 2, true) {
        announcement_vec.push(MediaAnnouncement {
            name: format!("model-{}", sanitize_model_name(model.to_luanti_safe())),
            sha1_base64: get_sha1_base64(&model.get_absolute(true)),
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

fn get_sha1_base64(path: &PathBuf) -> String {
    let mut file_handle;
    let metadata;
    file_handle = fs::File::open(path).unwrap();
    metadata = fs::metadata(path).expect("Unable to read File Metadata! (Check Permissions?)");
    let mut buffer = vec![0; metadata.len() as usize];
    file_handle.read_exact(&mut buffer).unwrap();
    // buffer_hash_b64 is base64encode( sha1hash( buffer ) )
    let mut hasher = Sha1::new();
    hasher.update(buffer);
    let mut buffer_hash_b64 = String::new();
    general_purpose::STANDARD.encode_string(hasher.finalize(), &mut buffer_hash_b64);
    buffer_hash_b64
}

pub fn handle_request(specbox: Box<client_to_server::RequestMediaSpec>) -> ToClientCommand {
    let client_to_server::RequestMediaSpec { files } = *specbox;
    let mut file_data: Vec<MediaFileData> = Vec::new();
    for file_name in files {
        let texture = LuantiTexture::from_luanti_safe(&file_name);
        let model_mode = texture.get_relative().starts_with("./model/");
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
                backface_culling: true,
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
struct RawBlockMapping {
    textures: HashMap<Direction, String>,
    drawtype: String,
}

pub fn load_block_mappings(
    nodebox_mapping: &HashMap<String, NodeBox>,
) -> HashMap<String, BlockMapping> {
    let data = include_bytes!("../extra_data/block_texture_map.json");
    let raw_map: HashMap<String, RawBlockMapping> = serde_json::from_slice(data).unwrap();
    let parsed_map = raw_map
        .into_iter()
        .map(|(k, v)| {
            let textures = v
                .textures
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
            };
            (k, mapped)
        })
        .collect();
    return parsed_map;
}

pub fn load_item_mappings() -> HashMap<String, LuantiTexture> {
    let data = include_bytes!("../extra_data/item_texture_map.json");
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
            min_edge: (v3f32 {
                x: cuboid[0] as f32 / sf_a - sf_b,
                y: cuboid[1] as f32 / sf_a - sf_b,
                z: cuboid[2] as f32 / sf_a - sf_b,
            }),
            max_edge: (v3f32 {
                x: cuboid[3] as f32 / sf_a - sf_b,
                y: cuboid[4] as f32 / sf_a - sf_b,
                z: cuboid[5] as f32 / sf_a - sf_b,
            }),
        })
    }
    return NodeBox::Fixed(NodeBoxFixed { fixed: ab_bounds });
}

pub fn load_nodeboxes() -> HashMap<String, NodeBox> {
    let data = include_bytes!("../extra_data/nodeboxes.json");
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

pub async fn fetch_models() {
    // ensure data dir exists
    let _ = std::fs::create_dir_all(dirs::data_local_dir().unwrap().join("bridgetest/"));
    // if the models are already downloaded, exit.
    let models_folder: PathBuf = dirs::data_local_dir().unwrap().join("bridgetest/models/");
    if models_folder.exists() {
        return;
    }
    std::fs::create_dir_all(&models_folder).unwrap();
    // attempt to get zip
    let model_url =
        "https://codeberg.org/mineclonia/mineclonia/archive/main:mods/ENTITIES/mobs_mc/models.zip";
    let resp = reqwest::get(model_url)
        .await
        .expect("Failed to request texture pack!");
    let texture_pack_data = resp.bytes().await.unwrap();
    zip_extract::extract(Cursor::new(texture_pack_data), &models_folder, true).unwrap();
}
