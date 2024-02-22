// ItemDefinitions and BlockDefinitions to be sent to the minetest client
// the functions are actually more like consts but
// the "String" type cant be a constant so :shrug:

use azalea::entity::metadata::Text;
use minetest_protocol::wire::command::{ItemdefSpec, NodedefSpec, ToClientCommand};
use minetest_protocol::wire::types::{s16, Option16, v3f, SColor, SimpleSoundSpec, // generic types
    ItemdefList, ItemDef, ToolCapabilities, ToolGroupCap, ItemAlias, ItemType, // item specific
    NodeDefManager, ContentFeatures, TileDef, AlignStyle, TileAnimationParams, NodeBox, AlphaMode, DrawType // node specific (even more complicated than items qwq)
    };

use alloc::boxed::Box;
use config::Config;

use std::path::{ Path, PathBuf };
use std::fs::{ File,  remove_dir_all, read_to_string };
use std::io::{ Cursor, Write };

use crate::utils;


pub fn get_item_def_command() -> ToClientCommand{
    pub struct Defaults {
        simplesound: SimpleSoundSpec,
        itemdef: ItemDef,
        itemalias: ItemAlias,
    }

    let placeholder: Defaults = Defaults {
        simplesound: SimpleSoundSpec {
            name: String::from("[[ERROR]]"),
            gain: 1.0,
            pitch: 1.0,
            fade: 1.0,
        },
        itemdef: ItemDef {
            version: 6, // https://github.com/minetest/minetest/blob/master/src/itemdef.cpp#L192
            item_type: ItemType::None,
            name: String::from("[[ERROR]]"),
            description: String::from("A unexpected (actually very expected) error occured. The proxy was unable to map a MC item to MT"),
            inventory_image: String::from(""), // TODO: That is not an image.
            wield_image: String::from(""),
            wield_scale: v3f {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
            stack_max: 64,
            usable: false,
            liquids_pointable: false,
            tool_capabilities: Option16::None,
            groups: Vec::new(),
            node_placement_prediction: String::from(""),
            sound_place: SimpleSoundSpec {
                name: String::from("[[ERROR]]"),
                gain: 1.0,
                pitch: 1.0,
                fade: 1.0,
            },
            sound_place_failed: SimpleSoundSpec {
                name: String::from("[[ERROR]]"),
                gain: 1.0,
                pitch: 1.0,
                fade: 1.0,
            },
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
        },
        itemalias: ItemAlias {
            name: String::from(""),
            convert_to: String::from("")

        }
    };

    let item_definitions: Vec<ItemDef> = vec![placeholder.itemdef];
    let alias_definitions: Vec<ItemAlias> = vec![placeholder.itemalias];

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

pub fn get_node_def_command() -> ToClientCommand {
    let simplesound_placeholder: SimpleSoundSpec = SimpleSoundSpec {
        name: String::from("[[ERROR]]"),
        gain: 1.0,
        pitch: 1.0,
        fade: 1.0,
    };
    let tiledef_placeholder: TileDef = TileDef {
        name: String::from("[[ERROR]]"),
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
    let contentfeatures_placeholder: ContentFeatures = ContentFeatures {
        version: 13, // https://github.com/minetest/minetest/blob/master/src/nodedef.h#L313
        name: String::from("[[ERROR]]"),
        groups: vec![(String::from(""), 1)], // [(String, i16), (String, i16)]
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
    let nodedef_command = ToClientCommand::Nodedef(
        Box::new(NodedefSpec {
            node_def: NodeDefManager {
                content_features: vec![(1, contentfeatures_placeholder)]
            }
        })
    );
    return nodedef_command;
}

/*
 * data_folder               -- dir::data_local_dir/bridgetest
 * |- url.dsv                -- contains "timestamp:url", where "url" is the url of the pack currently present and "timestamp" the time of download
 * |- textures.zip           -- the file downloaded from the url
 * \- textures               -- this file, but decompressed
 *    |- pack.mcmeta
 *    |- pack.png
 *    \- other stuff         -- all the other stuff in a valid mc texturepack
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
        let mut url_dsv = File::create(data_folder.join("url.dsv").as_path()).expect("Creating url.dsv failed!");
        url_dsv.write(dsv_content.as_bytes()).expect("Writing data to url.dsv failed!");
        // we need to re-download in this case
        do_download = true;
    } else {
        // url.dsv does exist
        // example dsv_content = "1708635188:https://database.faithfulpack.net/packs/32x-Java/December%202023/Faithful%2032x%20-%201.20.4.zip"
        let dsv_content = read_to_string(data_folder.join("url.dsv").as_path()).expect("Failed to read url.dsv, but it exists! (Check permissions?)");
        if !dsv_content.contains(&texture_pack_url) {
            // url.dsv does not contain our pack URL, so we need to re-download.
            utils::logger("url.dsv does exist, but contains the wrong URL. re-writing it.", 1);
            let new_dsv_content = format!("{}:{}", chrono::Utc::now().timestamp(), texture_pack_url);
            let mut url_dsv = File::open(data_folder.join("url.dsv").as_path()).expect("Opening url.dsv failed!");
            url_dsv.write(new_dsv_content.as_bytes()).expect("Writing data to url.dsv failed!");
            do_download = true;
        }
    };
    if do_download {
        utils::logger("Preparing texture pack -- This might take a while, depending on your internet speed.", 1);
        if Path::new(data_folder.join("textures/").as_path()).exists() {
            utils::logger("Detected old texture folder in data_dir, deleting it.", 1);
            let _ = remove_dir_all(data_folder.join("textures/").as_path()); // TODO: rn assuming this works
        }
        utils::logger("Downloading textures.zip (into memory)", 1);
        let resp = reqwest::get(texture_pack_url).await.expect("Failed to request texture pack!");
        let texture_pack_data = resp.bytes().await.expect("Recieved invalid response! This might be caused by not supplying a direct download link.");
        utils::logger("Unpacking textures.zip to data_dir/textures", 1);
        zip_extract::extract(Cursor::new(texture_pack_data), &data_folder.join("textures/").as_path(), true).expect("Failed to extract! Check Permissions!");
    } // else everything is fine
    return do_download;
}

pub fn get_texture_media_command(settings: &Config) -> ToClientCommand {
    validate_texture_pack(settings);
    return todo!();
}
