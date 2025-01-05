// ItemDefinitions and BlockDefinitions to be sent to the minetest client
// the functions are actually more like consts but
// the "String" type cant be a constant so :shrug:

use minecraft_data_rs::models::block::BoundingBox;
use minetest_protocol::wire::command::ToClientCommand;
use minetest_protocol::wire::command;
use minetest_protocol::wire::types::{ v2f, v3f, v2s32, AlignStyle, BlockPos, ContentFeatures, DrawType, Inventory, ItemAlias, ItemDef, ItemType, ItemdefList, NodeBox, NodeDefManager, NodeMetadata, Option16, SColor, SimpleSoundSpec, TileAnimationParams, TileDef, InventoryEntry, InventoryList, ItemStackUpdate}; // AAAAAA
use minetest_protocol::wire::types;

use minecraft_data_rs::Api;
use minecraft_data_rs::models;

use alloc::boxed::Box;
use config::Config;
use bimap::BiHashMap;

use std::path::PathBuf;

use crate::{ utils, MTServerState };
use crate::settings;

use azalea::registry::{Block, EntityKind, MenuKind};
use azalea::Vec3;

pub const V3F_ZERO: v3f = v3f {
    x: 0.0,
    y: 0.0,
    z: 0.0
};

#[derive(Clone)]
pub struct EntityMetadata {
    pub position: Vec3,
    pub velocity: Vec3,
    pub rotation: (i8, i8),
    pub entity_kind: EntityKind
}

impl Default for EntityMetadata {
    fn default() -> Self {
        EntityMetadata {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            rotation: (0, 0),
            entity_kind: EntityKind::Pig
        }
    }
}

#[derive(Clone)]
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

#[derive(Clone)]
pub enum FoodDisplay {
    Normal,
    Hunger,
    
    NoChange
}

#[derive(Clone, PartialEq, Copy)]
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

pub fn get_container_formspec(container: &MenuKind, title: &str) -> String {
    // TODO: Sanitize the title, currently someone could name a chest "hi]list[...]" to break a lot of stuff.
    match container {
        MenuKind::Generic9x3 => format!(
"formspec_version[7]\
size[11.5,11]\
background[0,0;17.5,17.5;gui-container-shulker_box.png]\
style_type[list;spacing=0.135,0.135;size=1.09,1.09;border=false]\
listcolors[#0000;#0002]\
list[current_player;container;0.55,1.3;9,3]\
list[current_player;main;0.55,9.7;9,1]\
list[current_player;main;0.55,5.75;9,3;9]\
label[0.55,0.5;{}]\
",
            title),
        MenuKind::Generic9x6 => format!(
"size[9,6]\
label[0,0;{}]\
list[current_player;main;0,0;9,6;]",
            title),
        MenuKind::Generic3x3 => format!(
"size[3,3]\
label[0,0;{}]\
list[current_player;main;0,0;3,3;]",
            title),
        MenuKind::Crafter3x3 => format!(
"size[4.5,3]\
label[0,0;{}]\
list[current_player;main;0,0;3,3;]\
list[current_player;main;3.5,1;1,1;]",
            title),
        MenuKind::BlastFurnace => format!("size[3,2]label[0,0;{}]list[current_player;main;0,0;1,2;]list[current_player;main;2,0.5;1,1;]", title),
        MenuKind::Furnace => format!("size[3,2]label[0,0;{}]list[current_player;main;0,0;1,2;]list[current_player;main;2,0.5;1,1;]", title),
        MenuKind::Smoker => format!("size[3,2]label[0,0;{}]list[current_player;main;0,0;1,2;]list[current_player;main;2,0.5;1,1;]", title),
        _ => format!("size[5,1]label[0,0;Error!\nAs-of-now unsupported MenuKind,\nUI cannot be shown!\nMenu Title: {}]", title),
    }
}

pub fn set_hotbar_size() -> ToClientCommand {
    ToClientCommand::HudSetParam(
        Box::new(command::HudSetParamSpec {
            value: types::HudSetParam::SetHotBarItemCount(settings::HOTBAR_SIZE)
        })
    )
}

pub fn set_hotbar_texture() -> ToClientCommand {
    ToClientCommand::HudSetParam(
        Box::new(command::HudSetParamSpec {
            value: types::HudSetParam::SetHotBarImage(String::from("gui-sprites-hud-hotbar.png"))
        })
    )
}

pub fn set_hotbar_selected() -> ToClientCommand {
    ToClientCommand::HudSetParam(
        Box::new(command::HudSetParamSpec {
            value: types::HudSetParam::SetHotBarSelectedImage(String::from("gui-sprites-hud-hotbar_selection.png"))
        })
    )
}

pub fn get_sky_stuff() -> [ToClientCommand; 5] {
    [
        ToClientCommand::SetSky(
            Box::new(command::SetSkySpec {
                params: types::SkyboxParams {
                    bgcolor: SColor {
                        r: 255,
                        g: 255,
                        b: 255,
                        a: 255,
                    },
                    clouds: true,
                    fog_sun_tint: SColor {
                        r: 255,
                        g: 255,
                        b: 95,
                        a: 51,
                    },
                    fog_moon_tint: SColor {
                        r: 255,
                        g: 255,
                        b: 255,
                        a: 255,
                    },
                    fog_tint_type: String::from("custom"),
                    data: types::SkyboxData::Color (
                        types::SkyColor {
                            day_sky: SColor {
                                r: 255,
                                g: 124,
                                b: 163,
                                a: 255,
                            },
                            day_horizon: SColor {
                                r: 255,
                                g: 192,
                                b: 216,
                                a: 255,
                            },
                            dawn_sky: SColor {
                                r: 255,
                                g: 124,
                                b: 163,
                                a: 255,
                            },
                            dawn_horizon: SColor {
                                r: 255,
                                g: 192,
                                b: 216,
                                a: 255,
                            },
                            night_sky: SColor {
                                r: 255,
                                g: 0,
                                b: 0,
                                a: 0,
                            },
                            night_horizon: SColor {
                                r: 255,
                                g: 74,
                                b: 103,
                                a: 144,
                            },
                            indoors: SColor {
                                r: 255,
                                g: 192,
                                b: 216,
                                a: 255,
                            },
                        },
                    ),
                    body_orbit_tilt: Some(-1024.0),
                }
            })
        ),
        ToClientCommand::SetSun(
            Box::new(command::SetSunSpec {
                sun: types::SunParams {
                    visible: true,
                    texture: String::from("environment-sun.png"),
                    tonemap: String::from(""),
                    sunrise: String::from("air.png"),
                    sunrise_visible: true,
                    scale: 1.0
                }
            })
        ),
        ToClientCommand::SetMoon(
            Box::new(command::SetMoonSpec {
                moon: types::MoonParams {
                    visible: true,
                    texture: String::from("environment-moon_phases.png"),
                    tonemap: String::from(""),
                    scale: 3.75
                }
            })
        ),
        ToClientCommand::SetStars(
            Box::new(command::SetStarsSpec {
                stars: types::StarParams {
                    visible: true,
                    count: 1000,
                    starcolor: SColor {
                        r: 105,
                        g: 235,
                        b: 235,
                        a: 255,
                    },
                    scale: 1.0,
                    day_opacity: Some(0.0)
                }
            })
        ),
        ToClientCommand::OverrideDayNightRatio(
            Box::new(command::OverrideDayNightRatioSpec {
                do_override: false,
                day_night_ratio: 0
            })
        )
    ]
}

pub fn empty_inventory() -> ToClientCommand {
    ToClientCommand::Inventory(
        Box::new(command::InventorySpec {
            inventory: Inventory {
                entries: vec![
                    InventoryEntry::Update {
                        0: InventoryList {
                            name: String::from("main"),
                            width: 0,
                            items: vec![ItemStackUpdate::Empty; 36]
                        }
                    },
                    InventoryEntry::Update {
                        0: InventoryList {
                            name: String::from("armor"),
                            width: 0,
                            items: vec![ItemStackUpdate::Empty; 4]
                        }
                    },
                    InventoryEntry::Update {
                        0: InventoryList {
                            name: String::from("offhand"),
                            width: 0,
                            items: vec![ItemStackUpdate::Empty]
                        }
                    },
                    InventoryEntry::Update {
                        0: InventoryList {
                            name: String::from("craft"),
                            width: 3,
                            items: vec![ItemStackUpdate::Empty; 4]
                        }
                    },
                    InventoryEntry::Update {
                        0: InventoryList {
                            name: String::from("craftpreview"),
                            width: 0,
                            items: vec![ItemStackUpdate::Empty]
                        }
                    }
                ]
            }
        })
    )
}

pub fn add_healthbar() -> ToClientCommand {
    ToClientCommand::Hudadd(
        Box::new(command::HudaddSpec {
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
            text: String::from("gui-sprites-hud-heart-full.png"),
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
                String::from("gui-sprites-hud-heart-container.png"),
            ),
            style: Some(0)
        })
    )
}

pub fn add_foodbar() -> ToClientCommand {
    ToClientCommand::Hudadd(
        Box::new(command::HudaddSpec {
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
            text: String::from("gui-sprites-hud-food_full.png"),
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
                String::from("gui-sprites-hud-food_empty.png"),
            ),
            style: Some(0)
        })
    )
}

pub fn add_airbar() -> ToClientCommand {
    ToClientCommand::Hudadd(
        Box::new(command::HudaddSpec {
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
            text: String::from("gui-sprites-hud-air.png"),
            number: 0, // default to not show this element
            item: 0,   // item count also gets changed when needed
            dir: 0,
            align: v2f {
                x: 0.0,
                y: 0.0
            },
            offset: v2f {
                x: 45.0,
                y: -113.0
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
                    y: 24
                }
            ),
            z_index: Some(0),
            text2: Some(
                String::from("gui-sprites-hud-air_bursting.png"),
            ),
            style: Some(0)
        })
    )
}

pub fn add_subtitlebox() -> ToClientCommand { 
    ToClientCommand::Hudadd(
        Box::new(command::HudaddSpec {
            server_id: settings::SUBTITLE_ID,
            typ: 1,
            pos: v2f {
                x: 0.5,
                y: 1.0
            },
            name: String::from(""),
            scale: v2f {
                x: 0.0,
                y: 0.0
            },
            text: String::from("-\n-"),
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
            world_pos: None,
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
    )
}

pub fn get_defaultpriv() -> ToClientCommand {
    ToClientCommand::Privileges(
        Box::new(command::PrivilegesSpec {
            privileges: vec![
                String::from("interact"),
                String::from("shout"),
            ]
        })
    )
}

pub fn get_movementspec() -> ToClientCommand {
    ToClientCommand::Movement(
        Box::new(command::MovementSpec {
            acceleration_default: 2.9,
            acceleration_air: 1.2,
            acceleration_fast: 10.0,
            speed_walk: 4.317,
            speed_crouch: 1.295,
            speed_fast: 5.612,
            speed_climb: 2.35,
            speed_jump: 6.6,
            liquid_fluidity: 1.13,
            liquid_fluidity_smooth: 0.5,
            liquid_sink: 23.0,
            gravity: 10.4,
        })
    )
}

pub fn get_inventory_formspec(formspec: &str) -> ToClientCommand {
    ToClientCommand::InventoryFormspec(
        Box::new(command::InventoryFormspecSpec{
            formspec: String::from(formspec),
        })
    )
}

pub fn get_csmrestrictions() -> ToClientCommand {
    ToClientCommand::CsmRestrictionFlags(
        Box::new(command::CsmRestrictionFlagsSpec {
            csm_restriction_flags: 0,
            csm_restriction_noderange: 0
        })
    )
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

pub async fn get_item_def_command(path_name_map: &BiHashMap<(PathBuf, String), String>, settings: &Config) -> ToClientCommand {
    let mc_data_api = Api::latest().expect("Failed to retrieve minecraft data! (check network?)");
    
    // we need food- and placeable IDs to predict right-click behavior of every item
    let food_ids: Vec<u32> = mc_data_api.foods.foods().unwrap().into_keys().collect();
    // assume placeable when a object with the same name exists as a block
    let block_names: Vec<String> = mc_data_api.blocks.blocks_array().unwrap().iter()
        .map(|item| item.name.clone())
        .collect();
    let placeable_ids: Vec<u32> = mc_data_api.items.items_array().unwrap().iter()
        .filter(|item| block_names.contains(&item.name))
        .map(|item|item.id)
        .collect();
    
    let mut mc_name: String;
    let mut texture_name: String;
    let mut item_definitions: Vec<ItemDef> = Vec::new();
    for item in mc_data_api.items.items_array().unwrap() {
        mc_name = item.name.clone();
        if path_name_map.contains_right(&format!("item-{}.png", mc_name.replace("minecraft:", ""))) {
            texture_name = format!("item-{}.png", mc_name.replace("minecraft:", ""));
        } else {
            texture_name = format!("block-{}.png", mc_name.replace("minecraft:", ""));
        };
        utils::logger(&format!("[Itemdefs] Mapped {} to the texture {}", mc_name, texture_name), 0);
        item_definitions.push(generate_itemdef(&mc_name, item, &texture_name, food_ids.clone(), placeable_ids.clone()));
    }
    
    let alias_definitions: Vec<ItemAlias> = vec![ItemAlias {name: String::from(""), convert_to: String::from("")}];

    ToClientCommand::Itemdef(
        Box::new(command::ItemdefSpec {
            item_def: ItemdefList {
                itemdef_manager_version: 0, // https://github.com/minetest/minetest/blob/master/src/itemdef.cpp#L616
                 defs: item_definitions,
                 aliases: alias_definitions
            }
        })
    )
}

pub fn generate_itemdef(name: &str, item: models::item::Item, inventory_image: &str, food_ids: Vec<u32>, placeable_ids: Vec<u32>) -> ItemDef {
    let stack_max: i16 = item.stack_size as i16;
    let max_durability = item.max_durability;
    let is_edible: bool = food_ids.contains(&item.id);
    let mut groups: Vec<(String, i16)> = Vec::new();

    let mut item_type: ItemType = ItemType::Craft;
    if max_durability.is_some() {
        item_type = ItemType::Tool;
    } else if placeable_ids.contains(&item.id) {
        item_type = ItemType::Node;
    }
    
    if item_type == ItemType::Node {
        groups.push(
               (String::from("building_block"), 1)
        )
    }
    
    let simplesound_placeholder: SimpleSoundSpec = SimpleSoundSpec {
        name: String::from(""),
        gain: 1.0,
        pitch: 1.0,
        fade: 1.0,
    };
    ItemDef {
        version: 6, // https://github.com/minetest/minetest/blob/master/src/itemdef.cpp#L192
        item_type: item_type.clone(),
        name: String::from(name),
        description: String::from(""),
        // legible formatted string (curly braces are escaped by duplication, so the output is "{a{b{c")
        inventory_image: match item_type {
            ItemType::Node => format!("[inventorycube{{{}{{{}{{{}", inventory_image, inventory_image, inventory_image),
            _ => String::from(inventory_image)
        },
        wield_image: match item_type {
            ItemType::Node => format!("[inventorycube{{{}{{{}{{{}", inventory_image, inventory_image, inventory_image),
            _ => String::from(inventory_image)
        },
        wield_scale: v3f {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
        stack_max,
        usable: (item_type == ItemType::Node || item_type == ItemType::Tool || is_edible),
        liquids_pointable: false,
        tool_capabilities: Option16::None,
        groups,
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

// node def stuff

pub async fn get_node_def_command(settings: &Config, mt_server_state: &mut MTServerState) -> ToClientCommand {
    let mc_data_api = Api::latest().expect("Failed to retrieve minecraft data! (check network?)");
    
    let mut id: u16;
    let mut content_features: Vec<(u16, ContentFeatures)> = Vec::new();
    let mut content_feature: ContentFeatures;
    for block in mc_data_api.blocks.blocks_array().unwrap() {
        id = block.id as u16;
        // +128 because the MT engine has some builtin nodes below that.
        // generate_contentfeature ignores that and recieves the regular id,
        // everything else must adjust for this offset.
        let texture_pack_res: u16 = settings.get_int("texture_pack_res").expect("Failed to read config!")
        .try_into().expect("Texture pack resolutions above 2^16 are not supported. What are you even doing?");
        content_feature = generate_contentfeature(block, texture_pack_res, mt_server_state);
        content_features.push((id+128, content_feature));
    }
    
    // add a special block without MC equivalent: glowing_air. this block will replace cave_air in the nether.
    // because the minetest engine has no concept of dimensions, it is impossible to tell it to make air glow in the nether.
    let tiledef = TileDef {
        name: String::from("air.png"),
        animation: TileAnimationParams::None,
        backface_culling: true,
        tileable_horizontal: false,
        tileable_vertical: false,
        color_rgb: None,
        scale: 0,
        align_style: AlignStyle::Node
    };
    let simplesound_placeholder: SimpleSoundSpec = SimpleSoundSpec {
        name: String::from(""),
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
    
    ToClientCommand::Nodedef(
        Box::new(command::NodedefSpec {
            node_def: NodeDefManager {
                content_features,
            }
        })
    )
}

pub fn generate_contentfeature(block: models::block::Block, texture_pack_res: u16, mt_server_state: &mut MTServerState) -> ContentFeatures {
    // If *every* possible state is solid, then walkable=true
    // for light stuff, use the "brightest" state
    // for everything else, do other stuff idk look at the code
    let mut texture_base_name: String = block.name.clone();
    let this_block: azalea::registry::Block = block.id.try_into().expect("Got invalid ID!");
    // BoundingBox does not implement partialEq for some reason
    let mut walkable = match block.bounding_box {
        BoundingBox::Block => true,
        BoundingBox::Empty => false
    };
    let mut light_source = block.emit_light;
    let mut sunlight_propagates = block.filter_light;
    let mut liquid_range = 0;
    let mut liquid_viscosity = 0;
    let mut liquid_renewable = true;
    let mut animation = TileAnimationParams::None;
    
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
        texture_base_name = texture_base_name.replace("_gate", "_planks");
    }
    if !texture_base_name.contains("stone") {
        texture_base_name = texture_base_name.replace("_slab", "_planks");
        texture_base_name = texture_base_name.replace("_stairs", "_planks");
    }
    texture_base_name = texture_base_name.replace("_carpet", "_block");
    texture_base_name = texture_base_name.replace("_wall_banner", "_concrete");
    texture_base_name = texture_base_name.replace("_bulb", "_block");
    if texture_base_name.contains("brick") {
        texture_base_name = texture_base_name.replace("_wall", "s");
    }
    if [Block::StoneButton, Block::StonePressurePlate].contains(&this_block) {
        texture_base_name = texture_base_name.replace("_button", "");
        texture_base_name = texture_base_name.replace("_pressure_plate", "");
    } else {
        texture_base_name = texture_base_name.replace("_button", "_block");
        texture_base_name = texture_base_name.replace("_pressure_plate", "_block");
    }
    
    // dripstone cant be implemented properly with the current metadata system
    if this_block == Block::PointedDripstone {
        texture_base_name = String::from("pointed_dripstone_down_middle")
    }
    // crops neither
    texture_base_name += match this_block {
        Block::Bamboo => "_stalk",
        Block::Cocoa | Block::NetherWart => "_stage2",
        Block::Carrots | Block::Beetroots | Block::SweetBerryBush | Block::Potatoes => "_stage3",
        Block::Wheat => "_stage7",
        _ => "",
    };

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
        
        // leaves, vines etc
        Block::MangroveRoots => DrawType::GlassLike,
        Block::Vine        => DrawType::GlassLike,
        Block::GlowLichen  => DrawType::GlassLike,
        Block::OakLeaves   => DrawType::GlassLike,
        Block::SpruceLeaves => DrawType::GlassLike,
        Block::BirchLeaves => DrawType::GlassLike,
        Block::JungleLeaves => DrawType::GlassLike,
        Block::AcaciaLeaves => DrawType::GlassLike,
        Block::CherryPlanks => DrawType::GlassLike,
        Block::DarkOakLeaves => DrawType::GlassLike,
        Block::MangroveLeaves => DrawType::GlassLike,
        Block::AzaleaLeaves => DrawType::GlassLike,
        Block::FloweringAzaleaLeaves => DrawType::GlassLike,
        
        _ => DrawType::Normal,
    };
    
    let rightclickable = match this_block {
        // opens inventory
        Block::Chest => true,
        Block::EnderChest => true,
        Block::EnchantingTable => true,
        Block::Anvil => true,
        Block::Grindstone => true,
        
        // changes own state
        Block::Lever => true,
        Block::Comparator => true,
        Block::Repeater => true,
        Block::RedstoneOre => true,
        Block::RedstoneWire => true,
        
        Block::OakButton => true,
        Block::SpruceButton => true,
        Block::BirchButton => true,
        Block::JungleButton => true,
        Block::AcaciaButton => true,
        Block::DarkOakButton => true,
        Block::MangroveButton => true,
        Block::CherryButton => true,
        Block::BambooButton => true,
        Block::CrimsonButton => true,
        Block::WarpedButton => true,

        // other stuff
        Block::WhiteBed => true,
        Block::LightGrayBed => true,
        Block::GrayBed  => true,
        Block::BlackBed => true,
        Block::BrownBed => true,
        Block::RedBed   => true,
        Block::OrangeBed => true,
        Block::YellowBed => true,
        Block::LimeBed  => true,
        Block::CyanBed  => true,
        Block::LightBlueBed => true,
        Block::BlueBed  => true,
        Block::PurpleBed => true,
        Block::MagentaBed => true,
        Block::PinkBed  => true,
        
        _ => false
    };
    
    let simplesound_placeholder: SimpleSoundSpec = SimpleSoundSpec {
        name: String::from(""),
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
    let texture_fallback_name: String; // what we initialize everything to
    if utils::basename_to_prefixed(&mt_server_state, &format!("{}.png", texture_base_name)).is_some() {
        texture_fallback_name = utils::basename_to_prefixed(&mt_server_state, &format!("{}.png", texture_base_name)).unwrap();
    } else if utils::basename_to_prefixed(&mt_server_state, &format!("{}_side.png", texture_base_name)).is_some() {
        texture_fallback_name = utils::basename_to_prefixed(&mt_server_state, &format!("{}_side.png", texture_base_name)).unwrap();
    } else if utils::basename_to_prefixed(&mt_server_state, &format!("{}_top.png", texture_base_name)).is_some() {
        texture_fallback_name = utils::basename_to_prefixed(&mt_server_state, &format!("{}_top.png", texture_base_name)).unwrap();
    } else if utils::basename_to_prefixed(&mt_server_state, &format!("{}_bottom.png", texture_base_name)).is_some() {
        texture_fallback_name = utils::basename_to_prefixed(&mt_server_state, &format!("{}_bottom.png", texture_base_name)).unwrap();
    } else {
        utils::logger(&format!("[Minetest] Unable to set sane default texture for block {}, using air.png", texture_base_name), 2);
        texture_fallback_name = String::from("air.png");
    }

    let mut tiledef_sides: [TileDef; 6] = [get_tiledef(&texture_fallback_name, &animation), get_tiledef(&texture_fallback_name, &animation), get_tiledef(&texture_fallback_name, &animation), get_tiledef(&texture_fallback_name, &animation), get_tiledef(&texture_fallback_name, &animation), get_tiledef(&texture_fallback_name, &animation)];
    // if _side/_top/_bottom exists, use that for the respective side(s)
    if utils::basename_to_prefixed(&mt_server_state, &format!("{}_top.png", texture_base_name)).is_some() {
        tiledef_sides[0] = get_tiledef(
            &utils::basename_to_prefixed(&mt_server_state, &format!("{}_top.png", texture_base_name)).unwrap(),
            &animation);
    }
    if utils::basename_to_prefixed(&mt_server_state, &format!("{}_bottom.png", texture_base_name)).is_some() {
        tiledef_sides[1] = get_tiledef(&utils::basename_to_prefixed(&mt_server_state, &format!("{}_bottom.png", texture_base_name)).unwrap(), &animation);
    }
    if utils::basename_to_prefixed(&mt_server_state, &format!("{}_side.png", texture_base_name)).is_some() {
        tiledef_sides[2] = get_tiledef(
            &utils::basename_to_prefixed(&mt_server_state, &format!("{}_side.png", texture_base_name)).unwrap(),
            &animation);
        tiledef_sides[3] = get_tiledef(
            &utils::basename_to_prefixed(&mt_server_state, &format!("{}_side.png", texture_base_name)).unwrap(),
            &animation);
        tiledef_sides[4] = get_tiledef(
            &utils::basename_to_prefixed(&mt_server_state, &format!("{}_side.png", texture_base_name)).unwrap(),
            &animation);
        tiledef_sides[5] = get_tiledef(
            &utils::basename_to_prefixed(&mt_server_state, &format!("{}_side.png", texture_base_name)).unwrap(),
            &animation);
    }
    ContentFeatures {
        version: 13, // https://github.com/minetest/minetest/blob/master/src/nodedef.h#L313
        name: block.name,
        groups: vec![
            (String::from("handy_dig"), 1),
        ],
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
        diggable: !matches!(this_block, Block::Bedrock),
        climbable: false,
        buildable_to: (rightclickable == false), // TODO this is a oversimplification and likely needs its own match abomination
        rightclickable,
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
    }
}
