// code to get media to the client
use bimap::BiHashMap;
use luanti_protocol::commands::client_to_server;
use luanti_protocol::commands::{server_to_client, server_to_client::ToClientCommand};
use luanti_protocol::types::{MediaAnnouncement, MediaFileData};
use std::path::{PathBuf, Path};
use std::fs;
use config::Config;
use std::io::{Read, Write, Cursor};
use sha1::{Sha1, Digest};
use base64::{Engine, engine::general_purpose};

use crate::{utils, MTServerState};

pub fn generate_map() -> BiHashMap<(PathBuf, String), String> {
    // generates the bimap (path,basename)<->name
    // basename is the (possibly ambiguous) name without prefix (with extension)
    let mut path_name_map: BiHashMap<(PathBuf, String), String> = BiHashMap::new();
    let textures_folder: PathBuf = dirs::data_local_dir().unwrap().join("bridgetest/textures/assets/minecraft/textures/");
    for dir_prefix in ["block", "particle", "entity", "item", "environment", "gui"] {
        scan_dir(&mut path_name_map, &textures_folder.join(dir_prefix), 3, dir_prefix);
    }
    // add models (which are included in the binary, not as external files)
    let iterator = fs::read_dir("./models/").expect("Failed to read media");
    for item in iterator {
        let name = item.as_ref().unwrap().file_name().into_string().unwrap();
        if name.ends_with(".b3d") {
            let path = item.unwrap().path();
            let basename = utils::b3d_sanitize(name);
            path_name_map.insert(
                (path, basename.clone()),
                format!("{}-{}", "entitymodel", basename)
            );
        };
    }
    path_name_map
}

pub fn scan_dir(path_name_map: &mut BiHashMap<(PathBuf, String), String>, dir: &PathBuf, recurse: u8, prefix: &str) {
    let iterator = fs::read_dir(dir).expect("Failed to read media");
    for item in iterator {
        let name = item.as_ref().unwrap().file_name().into_string().unwrap();
        if item.as_ref().unwrap().file_type().unwrap().is_dir() && recurse != 0 {
            // recurse one layer deep
            // also add the dir name to the prefix of these textures
            // to avoid "boat/birch.png" -> "entity-birch.png", when it should be "entity-boat-birch.png"
            scan_dir(path_name_map, &item.as_ref().unwrap().path(), recurse-1, &format!("{}-{}", prefix, name));
        }
        // ignore non-texture files
        if name.ends_with(".png") {
            path_name_map.insert(
                (item.as_ref().unwrap().path(), name.clone()),
                format!("{}-{}", prefix, name)
            );
        }
    }
}

pub fn get_announcement(path_name_map: &BiHashMap<(PathBuf, String), String>) -> ToClientCommand {
    let mut announcement_vec: Vec<MediaAnnouncement> = Vec::new();
    for texture in path_name_map.iter() {
        let sha1_base64 = get_sha1_base64(&texture.0.0);
        announcement_vec.push(MediaAnnouncement {
            name: texture.1.to_string(),
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

fn get_sha1_base64(path: &PathBuf) -> String {
    let mut file_handle = fs::File::open(&path).unwrap();
    let metadata = fs::metadata(&path).expect("Unable to read File Metadata! (Check Permissions?)");
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
        if !mt_server_state.path_name_map.contains_right(&file_name) {
            utils::logger(&format!("[Minetest] Client requested unknown media: {}", file_name), 3);
            continue;
        }
        let path = &mt_server_state.path_name_map.get_by_right(&file_name).unwrap().0;
        if file_name.starts_with("entitymodel") {
            // handle models separately, these are included in the binary
            let buffer = match path.file_name().unwrap().to_str().unwrap() {
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


pub async fn validate_texture_pack(settings: &Config) -> bool {
    // check (and possibly fix) the texture pack
    let texture_pack_url = settings.get_string("texture_pack_url").expect("Failed to read config!");
    let mut do_download: bool = false;
    // ensure the data folder exists
    let data_folder: PathBuf = dirs::data_local_dir().unwrap().join("bridgetest/"); // if this fails, your system got bigger issues
    utils::possibly_create_dir(&data_folder);
    // check if url.dsv exists
    if !Path::new(data_folder.join("url.dsv").as_path()).exists() {
        // url.dsv does not exist
        utils::logger("url.dsv is missing, creating it.", 1);
        let dsv_content = format!("{}:{}", chrono::Utc::now().timestamp(), texture_pack_url);
        let mut url_dsv = fs::File::create(data_folder.join("url.dsv").as_path()).expect("Creating url.dsv failed!");
        url_dsv.write_all(dsv_content.as_bytes()).expect("Writing data to url.dsv failed!");
        // we need to re-download in this case
        do_download = true;
    } else {
        // url.dsv does exist
        // example dsv_content = "1708635188:https://database.faithfulpack.net/packs/32x-Java/December%202023/Faithful%2032x%20-%201.20.4.zip"
        let dsv_content = fs::read_to_string(data_folder.join("url.dsv").as_path()).expect("Failed to read url.dsv, but it exists! (Check permissions?)");
        if !dsv_content.contains(&texture_pack_url) {
            // url.dsv does not contain our pack URL, so we need to re-download.
            utils::logger("url.dsv does exist, but contains the wrong URL. re-writing it.", 1);
            let new_dsv_content = format!("{}:{}", chrono::Utc::now().timestamp(), texture_pack_url);
            let mut url_dsv = fs::File::open(data_folder.join("url.dsv").as_path()).expect("Opening url.dsv failed!");
            url_dsv.write_all(new_dsv_content.as_bytes()).expect("Writing data to url.dsv failed!");
            do_download = true;
        } else {
            utils::logger(&format!("Found url.dsv at {}", data_folder.join("url.dsv").display()), 1)
        }
    };
    if do_download {
        if !utils::ask_confirm("No texture pack found! Download faithfulpack.net? [Y/N]: ") {
            // the user denied downloading the pack.
            let config_file_path: PathBuf = dirs::config_dir().unwrap().join("bridgetest.toml");
            utils::logger(&format!("A texture pack is needed for this program to run.
    You can change what pack will be downloaded by editing the URL in {}", config_file_path.display()), 3);
            std::process::exit(0);
        } else {
            utils::logger("Preparing texture pack -- This might take a while, depending on your internet speed.", 1);
            if Path::new(data_folder.join("textures/").as_path()).exists() {
                utils::logger("Detected old texture folder in data_dir, deleting it.", 1);
                let _ = fs::remove_dir_all(data_folder.join("textures/").as_path()); // TODO: rn assuming this works
            }
            utils::logger("Downloading textures.zip (into memory)", 1);
            let resp = reqwest::get(texture_pack_url).await.expect("Failed to request texture pack!");
            let texture_pack_data = resp.bytes().await.expect("Recieved invalid response! This might be caused by not supplying a direct download link.");
            utils::logger("Unpacking textures.zip to data_dir/textures", 1);
            zip_extract::extract(Cursor::new(texture_pack_data), data_folder.join("textures/").as_path(), true).expect("Failed to extract! Check Permissions!");
        }
    } // else the textures are already installed
    do_download
}
