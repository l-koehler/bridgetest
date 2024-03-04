// ItemDefinitions and BlockDefinitions to be sent to the minetest client
// the functions are actually more like consts but
// the "String" type cant be a constant so :shrug:

use minetest_protocol::wire::command::{AnnounceMediaSpec, DetachedInventorySpec, InventoryFormspecSpec, ItemdefSpec, MediaSpec, MovementSpec, NodedefSpec, PrivilegesSpec, HudaddSpec, ToClientCommand , CsmRestrictionFlagsSpec, InventorySpec};
use minetest_protocol::wire::types::{ v2f, v3f, v2s32, AlignStyle, BlockPos, ContentFeatures, DrawType, Inventory, ItemAlias, ItemDef, ItemType, ItemdefList, MediaAnnouncement, MediaFileData, NodeBox, NodeDefManager, NodeMetadata, Option16, SColor, SimpleSoundSpec, SunParams, TileAnimationParams, TileDef, InventoryEntry, InventoryList, ItemStackUpdate
}; // AAAAAA

use alloc::boxed::Box;
use config::Config;

use std::path::{ Path, PathBuf };
use std::fs;
use std::io::{ Cursor, Write, Read };

use crate::utils;
use crate::settings;
use sha1::{Sha1, Digest};
use base64::{Engine as _, engine::general_purpose};
use serde_json;

use azalea_registry::{self, Block};

pub enum HeartDisplay {
    Absorb,
    Frozen,
    Normal,
    Poison,
    Wither,
    
    HardcoreAbsorb,
    HardcoreFrozen,
    HardcoreNormal,
    HardcorePoison,
    HardcoreWither,
    
    Vehicle,
    NoChange // special value: do not change the heart texture
}

pub enum FoodDisplay {
    Normal,
    Hunger,
    
    NoChange
}

pub enum Dimensions {
    Overworld,
    Nether,
    End,
    Custom // assumes overworld height
}

pub const fn get_y_bounds(dimension: &Dimensions) -> (i16, i16) {
    match dimension {
        Dimensions::Nether => (0, 255), // worldgen limit is 128, but players can go above that
        Dimensions::End => (0, 255),
        Dimensions::Overworld => (-64, 320),
        Dimensions::Custom => (-64, 320)
    }
}

pub fn empty_inventory() -> ToClientCommand {
    let inventory_command = ToClientCommand::Inventory(
        Box::new(InventorySpec {
            inventory: Inventory {
                entries: vec![
                    InventoryEntry::Update {
                        0: InventoryList {
                            name: String::from("main"),
                            width: 0,
                            items: vec![ItemStackUpdate::Empty; 32]
                        }
                    },
                    InventoryEntry::Update {
                        0: InventoryList {
                            name: String::from("craft"),
                            width: 3,
                            items: vec![ItemStackUpdate::Empty; 32]
                        }
                    },
                    InventoryEntry::Update {
                        0: InventoryList {
                            name: String::from("craftpreview"),
                            width: 0,
                            items: vec![ItemStackUpdate::Empty]
                        }
                    },
                    InventoryEntry::Update {
                        0: InventoryList {
                            name: String::from("craftresult"),
                            width: 0,
                            items: vec![ItemStackUpdate::Empty]
                        }
                    },
                ]
            }
        })
    );
    inventory_command
}

pub fn add_healthbar() -> ToClientCommand {
    let hudadd_command = ToClientCommand::Hudadd(
        Box::new(HudaddSpec {
            server_id: settings::HEALTHBAR_ID,
            typ: 2,
            pos: v2f {
                x: 0.5,
                y: 1.0
            },
            name: String::from(""),
            scale: v2f {
                x: 0.0,
                y: 0.0
            },
            text: String::from("heart-full.png"),
            number: 20,
            item: 20,
            dir: 0,
            align: v2f {
                x: 0.0,
                y: 0.0
            },
            offset: v2f {
                x: -265.0,
                y: -88.0
            },
            world_pos: Some(
                v3f {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
            ),
            size: Some(
                v2s32 {
                    x: 24,
                    y: 24,
                },
            ),
            z_index: Some(0),
            text2: Some(
                String::from("heart-container.png"),
            ),
            style: Some(0)
        })
    );
    hudadd_command
}

pub fn add_foodbar() -> ToClientCommand {
    let hudadd_command = ToClientCommand::Hudadd(
        Box::new(HudaddSpec {
            server_id: settings::FOODBAR_ID,
            typ: 2,
            pos: v2f {
                x: 0.5,
                y: 1.0
            },
            name: String::from(""),
            scale: v2f {
                x: 0.0,
                y: 0.0
            },
            text: String::from("hud-food_full.png"),
            number: 20,
            item: 20,
            dir: 0,
            align: v2f {
                x: 0.0,
                y: 0.0
            },
            offset: v2f {
                x: 45.0,
                y: -88.0
            },
            world_pos: Some(
                v3f {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
            ),
            size: Some(
                v2s32 {
                    x: 24,
                    y: 24,
                },
            ),
            z_index: Some(0),
            text2: Some(
                String::from("hud-food_empty.png"),
            ),
            style: Some(0)
        })
    );
    hudadd_command
}

pub fn add_airbar() -> ToClientCommand {
    let hudadd_command = ToClientCommand::Hudadd(
        Box::new(HudaddSpec {
            server_id: settings::AIRBAR_ID,
            typ: 2,
            pos: v2f {
                x: 0.5,
                y: 1.0
            },
            name: String::from(""),
            scale: v2f {
                x: 0.0,
                y: 0.0
            },
            text: String::from("hud-air.png"),
            number: 0, // default to not show this element
            item: 20,
            dir: 0,
            align: v2f {
                x: 0.0,
                y: 0.0
            },
            offset: v2f {
                x: -265.0,
                y: -116.0
            },
            world_pos: Some(
                v3f {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
            ),
            size: Some(
                v2s32 {
                    x: 24,
                    y: 24,
                },
            ),
            z_index: Some(0),
            text2: None,
            style: Some(0)
        })
    );
    hudadd_command
}

pub fn get_defaultpriv() -> ToClientCommand {
    let priv_command = ToClientCommand::Privileges(
        Box::new(PrivilegesSpec {
            privileges: vec![
                String::from("interact"),
                String::from("shout"),
            ]
        })
    );
    priv_command
}

pub fn get_movementspec() -> ToClientCommand {
    let movement_command = ToClientCommand::Movement(
        Box::new(MovementSpec {
            acceleration_default: 3.0,
            acceleration_air: 2.0,
            acceleration_fast: 10.0,
            speed_walk: 4.0,
            speed_crouch: 1.35,
            speed_fast: 20.0,
            speed_climb: 3.0,
            speed_jump: 8.125, // roughly 1.25
            liquid_fluidity: 1.0,
            liquid_fluidity_smooth: 0.5,
            liquid_sink: 10.0,
            gravity: 10.25,
        })
    );
    movement_command
}

pub fn get_inventory_formspec() -> ToClientCommand {
    let formspec_command = ToClientCommand::InventoryFormspec(
        Box::new(InventoryFormspecSpec{
            formspec: String::from(settings::INV_FORMSPEC),
        })
    );
    formspec_command
}

pub fn get_csmrestrictions() -> ToClientCommand {
    let csm_command = ToClientCommand::CsmRestrictionFlags(
        Box::new(CsmRestrictionFlagsSpec {
            csm_restriction_flags: 0,
            csm_restriction_noderange: 0
        })
    );
    csm_command
}

pub const fn get_metadata_placeholder(x_pos: u16, y_pos: u16, z_pos: u16) -> (BlockPos, NodeMetadata) {
    let blockpos = BlockPos {
        raw: (16*z_pos + y_pos)*16 + x_pos,
    };
    let metadata = NodeMetadata {
        stringvars: vec![],
        inventory: Inventory {
            entries: vec![]
        }
    };
    (blockpos, metadata)
}

// item def stuff

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
    let mut item_definitions: Vec<ItemDef> = Vec::new();
    for item in arcticdata_items {
        mc_name = item.0;
        texture_name = format!("item-{}.png", mc_name.replace("minecraft:", ""));
        utils::logger(&format!("[Itemdefs] Mapped {} to the texture {}", mc_name, texture_name), 0);
        item_definitions.push(generate_itemdef(&mc_name, item.1, &texture_name));
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

pub fn generate_itemdef(name: &str, item: serde_json::Value, inventory_image: &str) -> ItemDef {
    let stack_max: i16 = item.get("maxStackSize").unwrap().as_i64().unwrap_or(0).try_into().unwrap();
    let block_id: String = item.get("blockId").unwrap().to_string();
    let max_durability: i64 = item.get("maxDamage").unwrap().as_i64().unwrap_or(0);
    let is_edible: bool = item.get("edible").unwrap().as_bool().unwrap_or(false);

    let mut item_type: ItemType = ItemType::Craft;
    if block_id != "minecraft:air" {
        item_type = ItemType::Node;
    } else if max_durability != 0 {
        item_type = ItemType::Tool;
    }
    
    let simplesound_placeholder: SimpleSoundSpec = SimpleSoundSpec {
        name: String::from("[[ERROR]]"),
        gain: 1.0,
        pitch: 1.0,
        fade: 1.0,
    };
    ItemDef {
        version: 6, // https://github.com/minetest/minetest/blob/master/src/itemdef.cpp#L192
        item_type: item_type.clone(),
        name: String::from(name),
        description: String::from(""),
        inventory_image: String::from(inventory_image),
        wield_image: String::from(inventory_image), // TODO what is a wield image doing and can i just decide to ignore it?
        wield_scale: v3f {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
        stack_max,
        usable: (item_type == ItemType::Node || item_type == ItemType::Tool || is_edible),
        liquids_pointable: false,
        tool_capabilities: Option16::None,
        groups: Vec::new(),
        node_placement_prediction: block_id, // air if the item is not a node
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

// node def stuff

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
    let mut texture_base_name: String;
    let mut id: u16;
    let mut content_features: Vec<(u16, ContentFeatures)> = Vec::new();
    for block in arcticdata_blocks {
        mc_name = block.0;
        texture_base_name = mc_name.replace("minecraft:", "").replace(".png", "");
        id = block.1.get("id").expect("Found a block without ID!").as_u64().unwrap() as u16;
        // +128 because the MT engine has some builtin nodes below that.
        // generate_contentfeature ignores that and recieves the regular id,
        // everything else must adjust for this offset.
        let texture_pack_res: u16 = settings.get_int("texture_pack_res").expect("Failed to read config!")
        .try_into().expect("Texture pack resolutions above u16 are not supported. What are you even doing?");
        content_features.push((id+128, generate_contentfeature(id, &mc_name, block.1, texture_base_name, texture_pack_res)));
    }
    
    // add a special block without MC equivalent: glowing_air. this block will replace cave_air in the nether.
    // because the minetest engine has no concept of dimensions, it is impossible to tell it to make air glow in the nether.
    let tiledef = TileDef {
        name: String::from("block-air.png"),
        animation: TileAnimationParams::None,
        backface_culling: true,
        tileable_horizontal: false,
        tileable_vertical: false,
        color_rgb: None,
        scale: 0,
        align_style: AlignStyle::Node
    };
    let simplesound_placeholder: SimpleSoundSpec = SimpleSoundSpec {
        name: String::from("[[ERROR]]"),
        gain: 1.0,
        pitch: 1.0,
        fade: 1.0,
    };
    let tiledef_sides = [tiledef.clone(), tiledef.clone(), tiledef.clone(), tiledef.clone(), tiledef.clone(), tiledef.clone()];
    content_features.push((120, ContentFeatures {
        version: 13,
        name: String::from("glowing_air"),
        groups: vec![(String::from(""), 1)],
        param_type: 0,
        param_type_2: 0,
        drawtype: DrawType::AirLike,
        mesh: String::from(""),
        visual_scale: 1.0,
        unused_six: 6,
        tiledef: tiledef_sides.clone(),
        tiledef_overlay: tiledef_sides.clone(),
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
        light_propagates: 15,
        sunlight_propagates: 15,
        light_source: 15,
        is_ground_content: false,
        walkable: false,
        pointable: false,
        diggable: false,
        climbable: false,
        buildable_to: false,
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
    }));
    
    let nodedef_command = ToClientCommand::Nodedef(
        Box::new(NodedefSpec {
            node_def: NodeDefManager {
                content_features,
            }
        })
    );
    return nodedef_command;
}

pub fn generate_contentfeature(id: u16, name: &str, block: serde_json::Value, mut texture_base_name: String, texture_pack_res: u16) -> ContentFeatures {
    // If *every* possible state is solid, then walkable=true
    // for light stuff, use the "brightest" state
    // for everything else, do other stuff idk look at the code
    let this_block: azalea_registry::Block = (id as u32).try_into().expect("Got invalid ID!");
    let mut walkable = true;
    let mut light_source = 0;
    let mut sunlight_propagates = 0;
    let mut liquid_range = 0;
    let mut liquid_viscosity = 0;
    let mut liquid_renewable = true;
    let mut animation = TileAnimationParams::None;
    for state in block.get("states").unwrap().as_array().unwrap() {
        if !state.get("solid").unwrap().as_bool().unwrap() {
            walkable = false;
        };
        if (state.get("lightEmission").unwrap().as_u64().unwrap() as u8) > light_source {
            light_source = state.get("lightEmission").unwrap().as_u64().unwrap() as u8;
        }
        if state.get("propagatesSkylightDown").unwrap().as_bool().unwrap() {
            sunlight_propagates = 15;
        }
    }
    // liquid stuff
    if this_block == Block::Water {
        liquid_renewable = true;
        liquid_viscosity = 0; // determines how much the liquid slows the player down
        liquid_range = 7;
        texture_base_name.push_str("_still"); // water.png does not exist, mc uses water_still.png and water_flow.png
    } else if this_block == Block::Lava {
        liquid_renewable = false;
        liquid_viscosity = 1;
        liquid_range = 4;
        texture_base_name.push_str("_still");
    }
    // animated textures
    if [Block::Water, Block::Lava, Block::Seagrass, Block::TallSeagrass, Block::NetherPortal, Block::EndPortal, Block::MagmaBlock].contains(&this_block) {
        animation = TileAnimationParams::VerticalFrames { aspect_w: texture_pack_res, aspect_h: texture_pack_res, length: 2.0 }
    }
    // some blocks just use the texture of other blocks
    if ![Block::BambooFence, Block::BambooFenceGate].contains(&this_block) {
        texture_base_name = texture_base_name.replace("_fence", "_planks");
    } else {
        texture_base_name = texture_base_name.replace("_carpet", "_block");
    }

    
    // drawtype is a little complicated, there isn't a field in the json for that.
    /*
     * PlantLike: Texture rendered along both diagonal horizontal lines.
     * Normal: Texture rendered on each of the 6 faces.
     * Liquid: Like Normal, but transparency added + shader stuff
     * other stuff - idk too lazy to type, use common sense
     */
    // azalea_registry::blockkind would be ideal, but is unused and thus unusable in azalea.
    // so i need to make this ugly thing matching each block with a non-normal drawtype
    let drawtype = match this_block {
        Block::Water       => DrawType::Liquid,
        Block::Lava        => DrawType::Liquid,
        
        Block::Air         => DrawType::AirLike,
        Block::CaveAir     => DrawType::AirLike,
        Block::VoidAir     => DrawType::AirLike,
        
        Block::Torch       => DrawType::TorchLike,
        Block::SoulTorch   => DrawType::TorchLike,
        
        Block::Dandelion   => DrawType::PlantLike,
        Block::Poppy       => DrawType::PlantLike,
        Block::BlueOrchid  => DrawType::PlantLike,
        Block::Allium      => DrawType::PlantLike,
        Block::AzureBluet  => DrawType::PlantLike,
        Block::RedTulip    => DrawType::PlantLike,
        Block::OrangeTulip => DrawType::PlantLike,
        Block::WhiteTulip  => DrawType::PlantLike,
        Block::PinkTulip   => DrawType::PlantLike,
        Block::OxeyeDaisy  => DrawType::PlantLike,
        Block::Cornflower  => DrawType::PlantLike,
        Block::LilyOfTheValley => DrawType::PlantLike,
        Block::Torchflower => DrawType::PlantLike,
        Block::BambooSapling => DrawType::PlantLike,
        Block::Bamboo      => DrawType::PlantLike,
        Block::DeadBush    => DrawType::PlantLike,
        Block::ShortGrass  => DrawType::PlantLike,
        Block::TallGrass   => DrawType::PlantLike,
        Block::Fern        => DrawType::PlantLike,
        Block::LargeFern   => DrawType::PlantLike,
        Block::HangingRoots => DrawType::PlantLike,
        Block::SweetBerryBush => DrawType::PlantLike,
        Block::Seagrass    => DrawType::PlantLike,
        Block::TallSeagrass => DrawType::PlantLike,
        Block::PointedDripstone => DrawType::PlantLike, // totally a plant, whatever
        
        Block::LilyPad     => DrawType::SignLike, // is flat without param2
        Block::MossCarpet  => DrawType::SignLike,
        Block::WhiteCarpet => DrawType::SignLike,
        Block::LightGrayCarpet => DrawType::SignLike,
        Block::GrayCarpet  => DrawType::SignLike,
        Block::BlackCarpet => DrawType::SignLike,
        Block::BrownCarpet => DrawType::SignLike,
        Block::RedCarpet   => DrawType::SignLike,
        Block::OrangeCarpet => DrawType::SignLike,
        Block::YellowCarpet => DrawType::SignLike,
        Block::LimeCarpet  => DrawType::SignLike,
        Block::CyanCarpet  => DrawType::SignLike,
        Block::LightBlueCarpet => DrawType::SignLike,
        Block::BlueCarpet  => DrawType::SignLike,
        Block::PurpleCarpet => DrawType::SignLike,
        Block::MagentaCarpet => DrawType::SignLike,
        Block::PinkCarpet  => DrawType::SignLike,
        
        Block::Rail        => DrawType::RailLike,
        Block::PoweredRail => DrawType::RailLike,
        Block::DetectorRail => DrawType::RailLike,
        Block::ActivatorRail => DrawType::RailLike,
        
        Block::OakSign     => DrawType::SignLike, // TODO send param2 for these nodes
        Block::SpruceSign  => DrawType::SignLike,
        Block::BirchSign   => DrawType::SignLike,
        Block::JungleSign  => DrawType::SignLike,
        Block::AcaciaSign  => DrawType::SignLike,
        Block::DarkOakSign => DrawType::SignLike,
        Block::MangroveSign => DrawType::SignLike,
        Block::CherrySign  => DrawType::SignLike,
        Block::BambooSign  => DrawType::SignLike,
        Block::CrimsonSign => DrawType::SignLike,
        Block::WarpedSign  => DrawType::SignLike,
        
        Block::OakFence    => DrawType::FenceLike,
        Block::SpruceFence => DrawType::FenceLike,
        Block::BirchFence  => DrawType::FenceLike,
        Block::JungleFence => DrawType::FenceLike,
        Block::AcaciaFence => DrawType::FenceLike,
        Block::DarkOakFence => DrawType::FenceLike,
        Block::MangroveFence => DrawType::FenceLike,
        Block::CherryFence => DrawType::FenceLike,
        Block::BambooFence => DrawType::FenceLike,
        Block::CrimsonFence => DrawType::FenceLike,
        Block::WarpedFence => DrawType::FenceLike,
        
        Block::Fire        => DrawType::FireLike,
        Block::SoulFire    => DrawType::FireLike,
        
        Block::MangroveRoots => DrawType::GlassLike,
        Block::Vine        => DrawType::GlassLike,
        
        _ => DrawType::Normal,
    };
    
    let simplesound_placeholder: SimpleSoundSpec = SimpleSoundSpec {
        name: String::from("[[ERROR]]"),
        gain: 1.0,
        pitch: 1.0,
        fade: 1.0,
    };
    
    // texture stuff
    // texture_base_name is the basename.
    // if {texture_base_name}_top.png exists then use it etc, default to fallbacks
    fn get_tiledef(texture: &str, animation: &TileAnimationParams) -> TileDef {
        TileDef {
            name: String::from(texture),
            animation: animation.clone(),
            backface_culling: true,
            tileable_horizontal: false,
            tileable_vertical: false,
            color_rgb: utils::get_colormap(texture),
            scale: 0,
            align_style: AlignStyle::Node
        }
    }
    let texture_folder: PathBuf = dirs::data_local_dir().unwrap().join("bridgetest/textures/assets/minecraft/textures/block/");
    let texture_fallback_name = &format!("block-{}.png", texture_base_name);
    let mut tiledef_sides: [TileDef; 6] = [get_tiledef(texture_fallback_name, &animation), get_tiledef(texture_fallback_name, &animation), get_tiledef(texture_fallback_name, &animation), get_tiledef(texture_fallback_name, &animation), get_tiledef(texture_fallback_name, &animation), get_tiledef(texture_fallback_name, &animation)];
    
    // TODO: This breaks tall blocks (doors etc) which use _top and _bottom
    if Path::new(texture_folder.join(format!("{}_top.png", texture_base_name)).as_path()).exists() {
        tiledef_sides[0] = get_tiledef(&format!("block-{}_top.png", texture_base_name), &animation);
    }
    if Path::new(texture_folder.join(format!("{}_bottom.png", texture_base_name)).as_path()).exists() {
        tiledef_sides[1] = get_tiledef(&format!("block-{}_bottom.png", texture_base_name), &animation);
    }
    if Path::new(texture_folder.join(format!("{}_side.png", texture_base_name)).as_path()).exists() {
        tiledef_sides[2] = get_tiledef(&format!("block-{}_side.png", texture_base_name), &animation);
        tiledef_sides[3] = get_tiledef(&format!("block-{}_side.png", texture_base_name), &animation);
        tiledef_sides[4] = get_tiledef(&format!("block-{}_side.png", texture_base_name), &animation);
        tiledef_sides[5] = get_tiledef(&format!("block-{}_side.png", texture_base_name), &animation);
    }
    
    let contentfeatures: ContentFeatures = ContentFeatures {
        version: 13, // https://github.com/minetest/minetest/blob/master/src/nodedef.h#L313
        name: String::from(name),
        groups: vec![(String::from(""), 1)], // [(String, i16), (String, i16)], IDK what this does
        param_type: 0,
        param_type_2: 0,
        drawtype,
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
        light_propagates: sunlight_propagates,
        sunlight_propagates,
        light_source, // TODO test the effect of this
        is_ground_content: false,
        walkable,
        pointable: true,
        diggable: true,
        climbable: false,
        buildable_to: true,
        rightclickable: false,
        damage_per_second: 0,
        liquid_type_bc: 0,
        liquid_alternative_flowing: String::from(""),
        liquid_alternative_source: String::from(""),
        liquid_viscosity,
        liquid_renewable,
        liquid_range,
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
    contentfeatures
}

/*
 * Texture pack sender/generators:
 * validate_texture_pack()
 * get_mediafilevecs()
 * texture_vec_iterator()
 * get_texture_media_commands()
 * alternate_exists()
 * 
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

fn texture_vec_iterator(texture_vec: &mut Vec<(PathBuf, String)>, media_folder: PathBuf, prefix: &str) {
    let mut name: String;
    let mut path: PathBuf;
    let iterator = fs::read_dir(&media_folder).expect("Failed to read media");
    for item in iterator {
        name = item.as_ref().unwrap().file_name().into_string().unwrap();
        if name.ends_with(".png") || name.ends_with(".b3d") {
            path = item.as_ref().unwrap().path();
            texture_vec.push((path, format!("{}-{}", prefix, name)));
        };
    }
}

pub async fn get_texture_media_commands(settings: &Config) -> (ToClientCommand, ToClientCommand, ToClientCommand, ToClientCommand, ToClientCommand, ToClientCommand) {
    // TODO: This is *very* inefficient. not that bad, its only run once each start, but still..
    // returns (announcemedia, media)
    // ensure a texture pack exists
    validate_texture_pack(settings).await;
    // foreach texture, generate announce and send specs
    // TODO: This currently will have every texture loaded into RAM at the same time
    let textures_folder: PathBuf = dirs::data_local_dir().unwrap().join("bridgetest/textures/assets/minecraft/textures/");
    let models_folder: PathBuf = dirs::data_local_dir().unwrap().join("bridgetest/models/"); // HACK! does not auto-download nor explain what is meant to be there

    // iterate over each
    let mut block_texture_vec: Vec<(PathBuf, String)> = Vec::new();
    let mut particle_texture_vec: Vec<(PathBuf, String)> = Vec::new();
    let mut entity_texture_vec: Vec<(PathBuf, String)> = Vec::new();
    let mut item_texture_vec: Vec<(PathBuf, String)> = Vec::new();
    let mut misc_texture_vec: Vec<(PathBuf, String)> = Vec::new();
    texture_vec_iterator(&mut block_texture_vec, textures_folder.join("block/"), "block");
    texture_vec_iterator(&mut particle_texture_vec, textures_folder.join("particle/"), "particle");
    texture_vec_iterator(&mut entity_texture_vec, textures_folder.join("entity/"), "entity");
    texture_vec_iterator(&mut item_texture_vec, textures_folder.join("item/"), "item");
    
    texture_vec_iterator(&mut misc_texture_vec, textures_folder.join("environment/"), "misc");
    texture_vec_iterator(&mut misc_texture_vec, textures_folder.join("gui/sprites/hud/"), "hud");
    texture_vec_iterator(&mut misc_texture_vec, textures_folder.join("gui/sprites/hud/heart/"), "heart");
    texture_vec_iterator(&mut misc_texture_vec, models_folder, "entitymodel");
    // texture_vec = [("/path/to/allay.png", "entity-allay"), ("/path/to/cactus_bottom.png", "block-cactus_bottom"), ...]
    // call get_mediafilevecs on each entry tuple in *_texture_vec
    let mut announcement_vec: Vec<MediaAnnouncement> = Vec::new();
    let mut block_file_vec: Vec<MediaFileData> = Vec::new();
    let mut particle_file_vec: Vec<MediaFileData> = Vec::new();
    let mut entity_file_vec: Vec<MediaFileData> = Vec::new();
    let mut item_file_vec: Vec<MediaFileData> = Vec::new();
    let mut misc_file_vec: Vec<MediaFileData> = Vec::new();
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
    for path_name_tuple in misc_texture_vec {
        mediafilevecs = get_mediafilevecs(path_name_tuple.0, &path_name_tuple.1);
        announcement_vec.push(mediafilevecs.1);
        misc_file_vec.push(mediafilevecs.0);
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
            num_bunches: 5,
            bunch_index: 1,
            files: block_file_vec
        })
    );
    let particle_media_packet = ToClientCommand::Media(
        Box::new(MediaSpec {
            num_bunches: 5,
            bunch_index: 2,
            files: particle_file_vec
        })
    );
    let entity_media_packet = ToClientCommand::Media(
        Box::new(MediaSpec {
            num_bunches: 5,
            bunch_index: 3,
            files: entity_file_vec
        })
    );
    let item_media_packet = ToClientCommand::Media(
        Box::new(MediaSpec {
            num_bunches: 5,
            bunch_index: 4,
            files: item_file_vec
        })
    );
    let misc_media_packet = ToClientCommand::Media(
        Box::new(MediaSpec {
            num_bunches: 5,
            bunch_index: 5,
            files: misc_file_vec
        })
    );
    return (announcemedia, block_media_packet, particle_media_packet, entity_media_packet, item_media_packet, misc_media_packet);
}
