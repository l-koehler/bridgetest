// code to get media to the client
use luanti_protocol::commands::client_to_server;
use luanti_protocol::commands::{server_to_client, server_to_client::ToClientCommand};
use luanti_protocol::types::{MediaAnnouncement, MediaFileData};
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use std::io::Read;
use sha1::{Sha1, Digest};
use base64::{Engine, engine::general_purpose};
use serde_json;

use crate::{utils, MTServerState, mt_definitions::TextureBlob};

pub fn generate_map() -> HashMap<String, TextureBlob> {
    // minecraft:thing -> TextureBlob::*
    let mut path_name_map: HashMap<String, TextureBlob> = HashMap::new();
    // paths relative to "bridgetest/textures/assets/minecraft/textures/"
    let item_mapping = include_bytes!("../extra_data/item_texture_map.json");
    let block_mapping = include_bytes!("../extra_data/block_texture_map.json");
    let item_mapping_json: serde_json::Value = serde_json::from_slice(item_mapping).unwrap();
    let block_mapping_json: serde_json::Value = serde_json::from_slice(block_mapping).unwrap();
    for mapping in block_mapping_json.as_object().unwrap().iter() {
        path_name_map.insert(
            String::from(mapping.0),
            TextureBlob::Block(mapping.1.as_str().unwrap().to_owned())
        );
    };
    for mapping in item_mapping_json.as_object().unwrap().iter() {
        let texture: TextureBlob;
        if path_name_map.contains_key(mapping.0) {
            let old = path_name_map.get(mapping.0).unwrap();
            texture = TextureBlob::BlockItem(
                String::from(old.get_texture()),
                mapping.1.as_str().unwrap().to_owned()
            )
        } else {
            texture = TextureBlob::Item(mapping.1.as_str().unwrap().to_owned())
        }
        path_name_map.insert(
            String::from(mapping.0),
            texture
        );
    };
    // air.png is provided by the minetest engine, we don't have to send it
    path_name_map.insert(
        String::from("minecraft:air"), TextureBlob::Block(String::from("air.png"))
    );
    path_name_map.insert(
        String::from("minecraft:void_air"), TextureBlob::Block(String::from("air.png"))
    );
    path_name_map.insert(
        String::from("minecraft:cave_air"), TextureBlob::Block(String::from("air.png"))
    );
    return path_name_map
}

pub fn get_announcement(path_name_map: &HashMap<String, TextureBlob>) -> ToClientCommand {
    let mut announcement_vec: Vec<MediaAnnouncement> = Vec::new();
    for texture in path_name_map.iter() {
        // engine provides air.png
        // skip last 3 entries (air, void/cave air)
        if *texture.1 == TextureBlob::Block(String::from("air.png")) {
            continue;
        }
        let sha1_base64 = get_sha1_base64(&PathBuf::from(&texture.1.get_texture()), true);
        announcement_vec.push(MediaAnnouncement {
            name: texture.0.to_string(),
            sha1_base64
        });
    }
    ToClientCommand::AnnounceMedia(
        Box::new(server_to_client::AnnounceMediaSpec {
            files: announcement_vec,
            remote_servers: String::from("") // IDK what this means or does, but it works if left alone. (meee :3)
        })
    )
}

fn get_sha1_base64(path: &PathBuf, is_rel_texture: bool) -> String {
    let mut file_handle;
    let metadata;
    if is_rel_texture {
        file_handle = fs::File::open(utils::make_abs_path(path)).unwrap();
        metadata = fs::metadata(utils::make_abs_path(path)).expect("Unable to read File Metadata! (Check Permissions?)");
    } else {
        file_handle = fs::File::open(path).unwrap();
        metadata = fs::metadata(path).expect("Unable to read File Metadata! (Check Permissions?)");
    }
    let mut buffer = vec![0; metadata.len() as usize];
    file_handle.read_exact(&mut buffer).expect("File Metadata lied about File Size. This should NOT happen, what the hell is wrong with your device?");
    // buffer_hash_b64 is base64encode( sha1hash( buffer ) )
    let mut hasher = Sha1::new();
    hasher.update(buffer);
    let mut buffer_hash_b64 = String::new();
    general_purpose::STANDARD.encode_string(hasher.finalize(), &mut buffer_hash_b64);
    buffer_hash_b64
}

pub fn handle_request(mt_server_state: &MTServerState, specbox: Box<client_to_server::RequestMediaSpec>) -> ToClientCommand {
    let client_to_server::RequestMediaSpec { files } = *specbox;
    let mut file_data: Vec<MediaFileData> = Vec::new();
    for file_name in files {
        let path = &mt_server_state.path_name_map.get(&file_name).unwrap().get_texture();
        if file_name.starts_with("model:") {
            // handle models separately, these are included in the binary
            // remove the "./model/" prefix to the path
            let buffer = match path.split_at_checked(8).unwrap().1 {
                "extra_mobs_cod.b3d" => include_bytes!("../models/extra_mobs_cod.b3d").to_vec(),
                "extra_mobs_dolphin.b3d" => include_bytes!("../models/extra_mobs_dolphin.b3d").to_vec(),
                "extra_mobs_glow_squid.b3d" => include_bytes!("../models/extra_mobs_glow_squid.b3d").to_vec(),
                "extra_mobs_hoglin.b3d" => include_bytes!("../models/extra_mobs_hoglin.b3d").to_vec(),
                "extra_mobs_piglin.b3d" => include_bytes!("../models/extra_mobs_piglin.b3d").to_vec(),
                "extra_mobs_salmon.b3d" => include_bytes!("../models/extra_mobs_salmon.b3d").to_vec(),
                "extra_mobs_strider.b3d" => include_bytes!("../models/extra_mobs_strider.b3d").to_vec(),
                "extra_mobs_sword_piglin.b3d" => include_bytes!("../models/extra_mobs_sword_piglin.b3d").to_vec(),
                "extra_mobs_tropical_fish_a.b3d" => include_bytes!("../models/extra_mobs_tropical_fish_a.b3d").to_vec(),
                "extra_mobs_tropical_fish_b.b3d" => include_bytes!("../models/extra_mobs_tropical_fish_b.b3d").to_vec(),
                "mobs_mc_axolotl.b3d" => include_bytes!("../models/mobs_mc_axolotl.b3d").to_vec(),
                "mobs_mc_bat.b3d" => include_bytes!("../models/mobs_mc_bat.b3d").to_vec(),
                "mobs_mc_blaze.b3d" => include_bytes!("../models/mobs_mc_blaze.b3d").to_vec(),
                "mobs_mc_cat.b3d" => include_bytes!("../models/mobs_mc_cat.b3d").to_vec(),
                "mobs_mc_chicken.b3d" => include_bytes!("../models/mobs_mc_chicken.b3d").to_vec(),
                "mobs_mc_cow.b3d" => include_bytes!("../models/mobs_mc_cow.b3d").to_vec(),
                "mobs_mc_creeper.b3d" => include_bytes!("../models/mobs_mc_creeper.b3d").to_vec(),
                "mobs_mc_dragon.b3d" => include_bytes!("../models/mobs_mc_dragon.b3d").to_vec(),
                "mobs_mc_enderman.b3d" => include_bytes!("../models/mobs_mc_enderman.b3d").to_vec(),
                "mobs_mc_endermite.b3d" => include_bytes!("../models/mobs_mc_endermite.b3d").to_vec(),
                "mobs_mc_evoker.b3d" => include_bytes!("../models/mobs_mc_evoker.b3d").to_vec(),
                "mobs_mc_ghast.b3d" => include_bytes!("../models/mobs_mc_ghast.b3d").to_vec(),
                "mobs_mc_guardian.b3d" => include_bytes!("../models/mobs_mc_guardian.b3d").to_vec(),
                "mobs_mc_horse.b3d" => include_bytes!("../models/mobs_mc_horse.b3d").to_vec(),
                "mobs_mc_illusioner.b3d" => include_bytes!("../models/mobs_mc_illusioner.b3d").to_vec(),
                "mobs_mc_iron_golem.b3d" => include_bytes!("../models/mobs_mc_iron_golem.b3d").to_vec(),
                "mobs_mc_llama.b3d" => include_bytes!("../models/mobs_mc_llama.b3d").to_vec(),
                "mobs_mc_magmacube.b3d" => include_bytes!("../models/mobs_mc_magmacube.b3d").to_vec(),
                "mobs_mc_parrot.b3d" => include_bytes!("../models/mobs_mc_parrot.b3d").to_vec(),
                "mobs_mc_pig.b3d" => include_bytes!("../models/mobs_mc_pig.b3d").to_vec(),
                "mobs_mc_pillager.b3d" => include_bytes!("../models/mobs_mc_pillager.b3d").to_vec(),
                "mobs_mc_polarbear.b3d" => include_bytes!("../models/mobs_mc_polarbear.b3d").to_vec(),
                "mobs_mc_rabbit.b3d" => include_bytes!("../models/mobs_mc_rabbit.b3d").to_vec(),
                "mobs_mc_sheepfur.b3d" => include_bytes!("../models/mobs_mc_sheepfur.b3d").to_vec(),
                "mobs_mc_shulker.b3d" => include_bytes!("../models/mobs_mc_shulker.b3d").to_vec(),
                "mobs_mc_silverfish.b3d" => include_bytes!("../models/mobs_mc_silverfish.b3d").to_vec(),
                "mobs_mc_skeleton.b3d" => include_bytes!("../models/mobs_mc_skeleton.b3d").to_vec(),
                "mobs_mc_slime.b3d" => include_bytes!("../models/mobs_mc_slime.b3d").to_vec(),
                "mobs_mc_snowman.b3d" => include_bytes!("../models/mobs_mc_snowman.b3d").to_vec(),
                "mobs_mc_spider.b3d" => include_bytes!("../models/mobs_mc_spider.b3d").to_vec(),
                "mobs_mc_squid.b3d" => include_bytes!("../models/mobs_mc_squid.b3d").to_vec(),
                "mobs_mc_stray.b3d" => include_bytes!("../models/mobs_mc_stray.b3d").to_vec(),
                "mobs_mc_vex.b3d" => include_bytes!("../models/mobs_mc_vex.b3d").to_vec(),
                "mobs_mc_villager.b3d" => include_bytes!("../models/mobs_mc_villager.b3d").to_vec(),
                "mobs_mc_villager_zombie.b3d" => include_bytes!("../models/mobs_mc_villager_zombie.b3d").to_vec(),
                "mobs_mc_vindicator.b3d" => include_bytes!("../models/mobs_mc_vindicator.b3d").to_vec(),
                "mobs_mc_witch.b3d" => include_bytes!("../models/mobs_mc_witch.b3d").to_vec(),
                "mobs_mc_wither.b3d" => include_bytes!("../models/mobs_mc_wither.b3d").to_vec(),
                "mobs_mc_witherskeleton.b3d" => include_bytes!("../models/mobs_mc_witherskeleton.b3d").to_vec(),
                "mobs_mc_wolf.b3d" => include_bytes!("../models/mobs_mc_wolf.b3d").to_vec(),
                "mobs_mc_zombie.b3d" => include_bytes!("../models/mobs_mc_zombie.b3d").to_vec(),
                _ => panic!("Model file was not included at compile time!")
            };
            file_data.push(MediaFileData {
                name: file_name,
                data: buffer
            })
        } else {
            let mut file_handle = fs::File::open(&path).unwrap();
            let metadata = fs::metadata(&path).expect("Unable to read File Metadata! (Check Permissions?)");
            let mut buffer = vec![0; metadata.len() as usize];
            file_handle.read_exact(&mut buffer).expect("File Metadata lied about File Size. This should NOT happen, what the hell is wrong with your device?");
            file_data.push(MediaFileData {
                name: file_name,
                data: buffer
            })
        }
    }
    ToClientCommand::Media(
        Box::new(server_to_client::MediaSpec {
            num_bunches: 1,
            bunch_index: 0,
            files: file_data
        })
    )
}
