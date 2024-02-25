// ItemDefinitions and BlockDefinitions to be sent to the minetest client
// the functions are actually more like consts but
// the "String" type cant be a constant so :shrug:

use azalea::core::particle;
use azalea::entity::metadata::Text;
use minetest_protocol::wire::command::{AnnounceMediaSpec, MediaSpec, ItemdefSpec, NodedefSpec, ToClientCommand};
use minetest_protocol::wire::types::{s16, v3f, AlignStyle, AlphaMode, ContentFeatures, DrawType, ItemAlias, ItemDef, ItemType, ItemdefList, MediaAnnouncement, MediaFileData, NodeBox, NodeBoxWallmounted, NodeDefManager, Option16, SColor, SimpleSoundSpec, TileAnimationParams, TileDef, ToolCapabilities, ToolGroupCap // the fool i was, thinking items were bad,,
    };

use alloc::boxed::Box;
use config::Config;

use std::ffi::OsString;
use std::path::{ Path, PathBuf };
use std::fs;
use std::io::{ Cursor, Write, Read, copy };

use crate::utils;
use sha1::{Sha1, Digest};
use base64::{Engine as _, engine::general_purpose};
use serde_json;

pub async fn get_item_def_command(settings: &Config) -> ToClientCommand {
    // ensure arcticdata_items exists
    let data_folder: PathBuf = dirs::data_local_dir().unwrap().join("bridgetest/");
    if !Path::new(data_folder.join("arcticdata_items.json").as_path()).exists() {
        let data_url = settings.get_string("arcticdata_items").expect("Failed to read config!");
        utils::logger(&format!("arcticdata_items.json missing, downloading it from {}.", data_url), 2);
        let resp = reqwest::get(data_url).await.expect("Failed to request texture pack!");
        let arctic_items_data = resp.text().await.expect("Recieved invalid response! This might be caused by not supplying a direct download link.");
        let mut json_file = fs::File::create(data_folder.join("arcticdata_items.json").as_path()).expect("Creating arcticdata_items.json failed!");
        json_file.write(arctic_items_data.as_bytes()).expect("Writing data to arcticdata_items.json failed!");
    }
    // parse arcticdata_blocks.json
    let arcticdata_items: std::collections::HashMap<String, serde_json::Value> = 
    serde_json::from_str(&fs::read_to_string(data_folder.join("arcticdata_items.json"))
    .expect("Failed to read arcticdata_items.json"))
    .expect("Failed to parse arcticdata_items.json!");
    
    let mut mc_name: String;
    let mut texture_name: String;
    let mut stacklimit: i16;
    let mut item_definitions: Vec<ItemDef> = Vec::new();
    for item in arcticdata_items {
        mc_name = item.0;
        texture_name = format!("item-{}.png", mc_name.replace("minecraft:", ""));
        stacklimit = item.1.get("maxStackSize").expect("Found a item without Stack Size!").as_u64().unwrap().try_into().unwrap(); // serde only offers as_u64, cant read u16 from file directly (qwq)
        println!("{} MAPPED -> {}", mc_name, texture_name);
        item_definitions.push(generate_itemdef(&mc_name, "TODO remove this :3", stacklimit, &texture_name));
    }
    
    let alias_definitions: Vec<ItemAlias> = vec![ItemAlias {name: String::from(""), convert_to: String::from("")}];

    let itemdef_command = ToClientCommand::Itemdef(
        Box::new(ItemdefSpec {
            item_def: ItemdefList {
                itemdef_manager_version: 0, // https://github.com/minetest/minetest/blob/master/src/itemdef.cpp#L616
                 defs: item_definitions,
                 aliases: alias_definitions
            }
        })
    );
    return itemdef_command;
}

// TODO: just like the nodedef thing, this uses bad defaults and needs to get the whole JSON to get rid of these
pub fn generate_itemdef(name: &str, description: &str, stacklimit: i16, inventory_image: &str) -> ItemDef {
    let simplesound_placeholder: SimpleSoundSpec = SimpleSoundSpec {
        name: String::from("[[ERROR]]"),
        gain: 1.0,
        pitch: 1.0,
        fade: 1.0,
    };
    ItemDef {
        version: 6, // https://github.com/minetest/minetest/blob/master/src/itemdef.cpp#L192
        item_type: ItemType::None,
        name: String::from(name),
        description: String::from(description),
        inventory_image: String::from(inventory_image),
        wield_image: String::from(inventory_image), // TODO what is a wield image doing and can i just decide to ignore it?
        wield_scale: v3f {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
        stack_max: stacklimit,
        usable: false,
        liquids_pointable: false,
        tool_capabilities: Option16::None,
        groups: Vec::new(),
        node_placement_prediction: String::from(""),
        sound_place: simplesound_placeholder.clone(),
        sound_place_failed: simplesound_placeholder,
        range: 5.0,
        palette_image: String::from(""),
        color: SColor {
            r: 100,
            g: 70,
            b: 85,
            a: 20,
        },
        inventory_overlay: String::from(""),
        wield_overlay: String::from(""),
        short_description: Some(String::from("Proxy fucked up, sorry!")),
        place_param2: None,
        sound_use: None,
        sound_use_air: None
    }
}

pub async fn get_node_def_command(settings: &Config) -> ToClientCommand {
    // ensure arcticdata_blocks exists
    let data_folder: PathBuf = dirs::data_local_dir().unwrap().join("bridgetest/");
    if !Path::new(data_folder.join("arcticdata_blocks.json").as_path()).exists() {
        let data_url = settings.get_string("arcticdata_blocks").expect("Failed to read config!");
        utils::logger(&format!("arcticdata_blocks.json missing, downloading it from {}.", data_url), 2);
        let resp = reqwest::get(data_url).await.expect("Failed to request texture pack!");
        let arctic_block_data = resp.text().await.expect("Recieved invalid response! This might be caused by not supplying a direct download link.");
        let mut json_file = fs::File::create(data_folder.join("arcticdata_blocks.json").as_path()).expect("Creating arcticdata_blocks.json failed!");
        json_file.write(arctic_block_data.as_bytes()).expect("Writing data to arcticdata_blocks.json failed!");
    }
    // parse arcticdata_blocks.json
    let arcticdata_blocks: std::collections::HashMap<String, serde_json::Value> = 
    serde_json::from_str(&fs::read_to_string(data_folder.join("arcticdata_blocks.json"))
    .expect("Failed to read arcticdata_blocks.json"))
    .expect("Failed to parse arcticdata_blocks.json!");
    
    let mut mc_name: String;
    let mut texture_name: String;
    let mut id: u16;
    let mut content_features: Vec<(u16, ContentFeatures)> = Vec::new();
    for block in arcticdata_blocks {
        mc_name = block.0;
        texture_name = format!("block-{}.png", mc_name.replace("minecraft:", ""));
        id = block.1.get("id").expect("Found a block without ID!").as_u64().unwrap().try_into().unwrap(); // serde only offers as_u64, cant read u16 from file directly (qwq)
        println!("{} MAPPED -> {}", mc_name, texture_name);
        content_features.push(generate_contentfeature(id, &mc_name, &texture_name));
    }
    let nodedef_command = ToClientCommand::Nodedef(
        Box::new(NodedefSpec {
            node_def: NodeDefManager {
                content_features: content_features,
            }
        })
    );
    return nodedef_command;
}

// TODO: This uses a bunch of only somewhat sane defaults. Add more options or just pass the JSON.
pub fn generate_contentfeature(id: u16, name: &str, texture_name: &str) -> (u16, ContentFeatures) {
    let simplesound_placeholder: SimpleSoundSpec = SimpleSoundSpec {
        name: String::from("[[ERROR]]"),
        gain: 1.0,
        pitch: 1.0,
        fade: 1.0,
    };
    let tiledef_placeholder: TileDef = TileDef {
        name: String::from(texture_name),
        animation: TileAnimationParams::None,
        backface_culling: false,
        tileable_horizontal: false,
        tileable_vertical: false,
        color_rgb: None,
        scale: 1,
        align_style: AlignStyle::Node
    };
    // like [tiledef_placeholder; 6] if it were slow qwq
    let tiledef_sides = [tiledef_placeholder.clone(), tiledef_placeholder.clone(), tiledef_placeholder.clone(), tiledef_placeholder.clone(), tiledef_placeholder.clone(), tiledef_placeholder.clone()];
    let contentfeatures: ContentFeatures = ContentFeatures {
        version: 13, // https://github.com/minetest/minetest/blob/master/src/nodedef.h#L313
        name: String::from(name),
        groups: vec![(String::from(""), 1)], // [(String, i16), (String, i16)], IDK what this does
        param_type: 0,
        param_type_2: 0,
        drawtype: DrawType::Normal,
        mesh: String::from(""),
        visual_scale: 1.0,
        unused_six: 6, // unused? idk what does this even do
        tiledef: tiledef_sides.clone(),
        tiledef_overlay: tiledef_sides.clone(),
        // unexplained in the minetest-protocol crate
        tiledef_special: tiledef_sides.to_vec(),
        alpha_for_legacy: 20,
        red: 100,
        green: 70,
        blue: 85,
        palette_name: String::from(""),
        waving: 0,
        connect_sides: 0,
        connects_to_ids: Vec::new(),
        post_effect_color: SColor {
            r: 100,
            g: 70,
            b: 85,
            a: 20,
        },
        leveled: 0,
        light_propagates: 0,
        sunlight_propagates: 0,
        light_source: 0,
        is_ground_content: false,
        walkable: true,
        pointable: true,
        diggable: true,
        climbable: false,
        buildable_to: true,
        rightclickable: false,
        damage_per_second: 0,
        liquid_type_bc: 0,
        liquid_alternative_flowing: String::from(""),
        liquid_alternative_source: String::from(""),
        liquid_viscosity: 0,
        liquid_renewable: false,
        liquid_range: 0,
        drowning: 0,
        floodable: false,
        node_box: NodeBox::Regular,
        selection_box: NodeBox::Regular,
        collision_box: NodeBox::Regular,
        sound_footstep: simplesound_placeholder.clone(),
        sound_dig: simplesound_placeholder.clone(),
        sound_dug: simplesound_placeholder.clone(),
        legacy_facedir_simple: false,
        legacy_wallmounted: false,
        node_dig_prediction: None,
        leveled_max: None,
        alpha: None,
        move_resistance: None,
        liquid_move_physics: None
    };
    return (id, contentfeatures)
}

/*
 * Texture pack sender/generators:
 * validate_texture_pack()
 * get_mediafilevecs()
 * texture_vec_iterator()
 * get_texture_media_commands()
 * Folder Structure
 * data_folder               -- dir::data_local_dir/bridgetest
 * |- url.dsv                -- contains "timestamp:url", where "url" is the url of the pack currently present and "timestamp" the time of download
 * \- textures               -- a valid minecraft texturepack, uncompressed
 *    |- pack.mcmeta
 *    |- pack.png
 *    \- assets
 *       \- minecraft
 *          \- textures
 *             |- block
 *             |  \- a bunch of PNGs (block-name.png)
 *             |- item
 *             |  \- a bunch of PNGs (item-name.png)
 *             |- entity
 *             |  \- a bunch of PNGs (entity-name.png)
 *             |
 *             \~ a bunch of other folders this program does not care about
 */

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
        url_dsv.write(dsv_content.as_bytes()).expect("Writing data to url.dsv failed!");
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
            url_dsv.write(new_dsv_content.as_bytes()).expect("Writing data to url.dsv failed!");
            do_download = true;
        } else {
            utils::logger(&format!("Found url.dsv at {}", data_folder.join("url.dsv").display()), 1)
        }
    };
    if do_download {
        if !utils::ask_confirm("No texture pack found! Download faithfulpack.net? [Y/N]: ") {
            // the user denied downloading the pack.
            let config_file_path: PathBuf = dirs::config_dir().unwrap().join("bridgetest.toml");
            println!("A texture pack is needed for this program to run.
    You can change what pack will be downloaded by editing the URL in {}", config_file_path.display());
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
            zip_extract::extract(Cursor::new(texture_pack_data), &data_folder.join("textures/").as_path(), true).expect("Failed to extract! Check Permissions!");
        }
    } // else the textures are already installed
    return do_download;
}

pub fn get_mediafilevecs(filename: PathBuf, name: &str) -> (MediaFileData, MediaAnnouncement) {
    let mut texture_file = fs::File::open(&filename).unwrap();
    let metadata = fs::metadata(&filename).expect("Unable to read File Metadata! (Check Permissions?)");
    let mut buffer = vec![0; metadata.len() as usize];
    texture_file.read(&mut buffer).expect("File Metadata lied about File Size. This should NOT happen, what the hell is wrong with your device?");
    // buffer: Vec<u8> with the png's content.
    let filedata = MediaFileData {
        name: String::from(name),
        data: buffer.clone()
    };
    // buffer_hash_b64 is base64encode( sha1hash( buffer ) )
    let mut hasher = Sha1::new();
    hasher.update(buffer);
    let mut buffer_hash_b64 = String::new();
    general_purpose::STANDARD.encode_string(hasher.finalize(), &mut buffer_hash_b64);
    let fileannounce = MediaAnnouncement {
        name: String::from(name),
        sha1_base64: buffer_hash_b64,
    };
    return (filedata, fileannounce);
}

fn texture_vec_iterator(texture_vec: &mut Vec<(PathBuf, String)>, iterator: fs::ReadDir, prefix: &str) {
    let mut name: String;
    let mut path: PathBuf;
    for item in iterator {
        name = item.as_ref().unwrap().file_name().into_string().unwrap();
        if name.ends_with("png") {
            path = item.as_ref().unwrap().path();
            texture_vec.push((path, format!("{}-{}", prefix, name)));
        };
    }
}

pub async fn get_texture_media_commands(settings: &Config) -> (ToClientCommand, ToClientCommand, ToClientCommand, ToClientCommand, ToClientCommand) {
    // TODO: This is *very* inefficient. not that bad, its only run once each start, but still..
    // returns (announcemedia, media)
    // ensure a texture pack exists
    validate_texture_pack(settings).await;
    // foreach texture, generate announce and send specs
    // TODO: This currently will have every texture loaded into RAM at the same time
    let textures_folder: PathBuf = dirs::data_local_dir().unwrap().join("bridgetest/textures/assets/minecraft/textures/");
    let block_textures = fs::read_dir(textures_folder.join("block/")).unwrap();
    let particle_textures = fs::read_dir(textures_folder.join("particle/")).unwrap();
    let entity_textures = fs::read_dir(textures_folder.join("entity/")).unwrap();
    let item_textures = fs::read_dir(textures_folder.join("item/")).unwrap();
    // iterate over each
    let mut block_texture_vec: Vec<(PathBuf, String)> = Vec::new();
    let mut particle_texture_vec: Vec<(PathBuf, String)> = Vec::new();
    let mut entity_texture_vec: Vec<(PathBuf, String)> = Vec::new();
    let mut item_texture_vec: Vec<(PathBuf, String)> = Vec::new();
    texture_vec_iterator(&mut block_texture_vec, block_textures, "block");
    texture_vec_iterator(&mut particle_texture_vec, particle_textures, "particle");
    texture_vec_iterator(&mut entity_texture_vec, entity_textures, "entity");
    texture_vec_iterator(&mut item_texture_vec, item_textures, "item");
    // texture_vec = [("/path/to/allay.png", "entity-allay"), ("/path/to/cactus_bottom.png", "block-cactus_bottom"), ...]
    // call get_mediafilevecs on each entry tuple in *_texture_vec
    let mut announcement_vec: Vec<MediaAnnouncement> = Vec::new();
    let mut block_file_vec: Vec<MediaFileData> = Vec::new();
    let mut particle_file_vec: Vec<MediaFileData> = Vec::new();
    let mut entity_file_vec: Vec<MediaFileData> = Vec::new();
    let mut item_file_vec: Vec<MediaFileData> = Vec::new();
    let mut mediafilevecs;
    for path_name_tuple in block_texture_vec {
        mediafilevecs = get_mediafilevecs(path_name_tuple.0, &path_name_tuple.1);
        announcement_vec.push(mediafilevecs.1);
        block_file_vec.push(mediafilevecs.0);
    }
    for path_name_tuple in particle_texture_vec {
        mediafilevecs = get_mediafilevecs(path_name_tuple.0, &path_name_tuple.1);
        announcement_vec.push(mediafilevecs.1);
        particle_file_vec.push(mediafilevecs.0);
    }
    for path_name_tuple in entity_texture_vec {
        mediafilevecs = get_mediafilevecs(path_name_tuple.0, &path_name_tuple.1);
        announcement_vec.push(mediafilevecs.1);
        entity_file_vec.push(mediafilevecs.0);
    }
    for path_name_tuple in item_texture_vec {
        mediafilevecs = get_mediafilevecs(path_name_tuple.0, &path_name_tuple.1);
        announcement_vec.push(mediafilevecs.1);
        item_file_vec.push(mediafilevecs.0);
    }
    let announcemedia = ToClientCommand::AnnounceMedia(
        Box::new(AnnounceMediaSpec {
            files: announcement_vec,
            remote_servers: String::from("") // IDK what this means or does, but it works if left alone. (meee :3)
        })
    );
    // split texture packets across 4 packets
    let block_media_packet = ToClientCommand::Media(
        Box::new(MediaSpec {
            num_bunches: 4,
            bunch_index: 1,
            files: block_file_vec.clone()
        })
    );
    let particle_media_packet = ToClientCommand::Media(
        Box::new(MediaSpec {
            num_bunches: 4,
            bunch_index: 2,
            files: particle_file_vec.clone()
        })
    );
    let entity_media_packet = ToClientCommand::Media(
        Box::new(MediaSpec {
            num_bunches: 4,
            bunch_index: 3,
            files: entity_file_vec.clone()
        })
    );
    let item_media_packet = ToClientCommand::Media(
        Box::new(MediaSpec {
            num_bunches: 4,
            bunch_index: 4,
            files: item_file_vec
        })
    );
    return (announcemedia, block_media_packet, particle_media_packet, entity_media_packet, item_media_packet);
}
