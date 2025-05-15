// ItemDefinitions and BlockDefinitions to be sent to the minetest client
// the functions are actually more like consts but
// the "String" type cant be a constant so :shrug:

use luanti_protocol::commands::{server_to_client, server_to_client::ToClientCommand};
use luanti_protocol::types::{ AlignStyle, NodeBox, ContentFeatures, DrawType, Inventory, SimpleSoundSpec, TileAnimationParams, TileDef, InventoryEntry, InventoryList, ItemStackUpdate, SColor};
use luanti_protocol::commands::server_to_client::{ItemDef, ItemAlias, ItemdefList, ItemType};
use luanti_protocol::types;

use minecraft_data_rs::Api;
use minecraft_data_rs::models;
use config::Config;

use std::collections::HashMap;

// same fucking name as in azalea :sob:
// i am lazy, so this gets renamed to the old minetest-protocol types
// except for "v2s32", inconsistent with rust u/i for unsigned/signed. renamed to "v2i32"
use glam::Vec2 as v2f;
use glam::Vec3 as v3f;
use glam::IVec2 as v2i32;

use crate::{ utils, MTServerState };
use crate::settings;

use azalea::registry::{Block, EntityKind, MenuKind};
use azalea::Vec3;
use std::path::PathBuf;

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

// resolves ambiguity in mapping minecraft:thing to textures
// important! only stores paths relative to the texture pack root.
// Stores models using the fake ./model/ path
#[derive(Clone,Eq,PartialEq,Hash,Debug)]
pub enum TextureBlob {
    Block(LuantiTexture),
    Item(LuantiTexture),
    BlockItem(LuantiTexture, LuantiTexture)
}

impl TextureBlob {
    // prefers returning a item texture
    pub fn get_texture(&self) -> &LuantiTexture {
        match self {
            TextureBlob::Block(a)|TextureBlob::Item(a)|TextureBlob::BlockItem(_, a) => return a
        }
    }
}

#[derive(Clone,Eq,PartialEq,Hash,Debug)]
pub struct LuantiTexture {
    rel_path: String
}

impl LuantiTexture {
    pub fn get_relative(&self) -> &str {
        return &self.rel_path;
    }
    pub fn get_absolute(&self) -> PathBuf {
        let textures_folder: PathBuf = dirs::data_local_dir().unwrap().join("bridgetest/textures/");
        return textures_folder.join(PathBuf::from(&self.rel_path))
    }
    // ./block/thing.png -> block-thing.png
    // we need to keep the extension, luanti relies on that for file type
    pub fn to_luanti_safe(&self) -> String {
        return self.get_relative().replace("./", "").replace("/", "-")
    }
    pub fn from_luanti_safe(safe_texture: &str) -> LuantiTexture {
        let rel_path = format!("./{}", safe_texture.replace("-", "/"));
        return LuantiTexture { rel_path }
    }
    pub fn with_postfix(&self, prefix: &str) -> LuantiTexture {
        let new_rpath = format!("{}{}.png", self.rel_path.replace(".png", ""), prefix);
        return LuantiTexture {
            rel_path: new_rpath
        }
    }
    pub fn is_valid(&self) -> bool {
        self.get_absolute().exists()
    }
    pub fn from_string(rpath: &str) -> LuantiTexture {
        LuantiTexture { rel_path: String::from(rpath) }
    }
    pub fn from_absolute(apath: PathBuf) -> LuantiTexture {
        let textures_folder: PathBuf = dirs::data_local_dir().unwrap().join("bridgetest/textures/");
        let rel_path = apath.strip_prefix(textures_folder).unwrap().to_str().unwrap().to_owned();
        LuantiTexture { rel_path }
    }
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
        Box::new(server_to_client::HudSetParamSpec {
            value: types::HudSetParam::SetHotBarItemCount(settings::HOTBAR_SIZE)
        })
    )
}

pub fn set_hotbar_texture() -> ToClientCommand {
    ToClientCommand::HudSetParam(
        Box::new(server_to_client::HudSetParamSpec {
            value: types::HudSetParam::SetHotBarImage(String::from("gui-sprites-hud-hotbar.png"))
        })
    )
}

pub fn set_hotbar_selected() -> ToClientCommand {
    ToClientCommand::HudSetParam(
        Box::new(server_to_client::HudSetParamSpec {
            value: types::HudSetParam::SetHotBarSelectedImage(String::from("gui-sprites-hud-hotbar_selection.png"))
        })
    )
}

pub fn get_sky_stuff() -> [ToClientCommand; 5] {
    [
        ToClientCommand::SetSky(
            Box::new(server_to_client::SetSkyCommand {
                params: server_to_client::SkyboxParams {
                    bgcolor: SColor::new(255, 255, 255, 255),
                    clouds: true,
                    fog_sun_tint: SColor::new(255, 255, 95, 51),
                    fog_moon_tint: SColor::new(255, 255, 255, 255),
                    fog_tint_type: String::from("custom"),
                    data: server_to_client::SkyboxData::Color (
                        types::SkyColor {
                            day_sky: SColor::new(255, 124, 163, 255),
                            day_horizon: SColor::new(255, 192, 216, 255),
                            dawn_sky: SColor::new(255, 124, 163, 255),
                            dawn_horizon: SColor::new(255, 192, 216, 255),
                            night_sky: SColor::new(255, 0, 0, 0),
                            night_horizon: SColor::new(255, 74, 103, 144),
                            indoors: SColor::new(255, 192, 216, 255)
                        },
                    ),
                    r#type: String::from(""), // TODO
                    body_orbit_tilt: 0.0,
                    fog_distance: i16::MAX,
                    fog_start: f32::MAX,
                    fog_color: SColor::new(0, 0, 0, 255)
                }
            })
        ),
        ToClientCommand::SetSun(
            Box::new(server_to_client::SetSunSpec {
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
            Box::new(server_to_client::SetMoonSpec {
                moon: types::MoonParams {
                    visible: true,
                    texture: String::from("environment-moon_phases.png"),
                    tonemap: String::from(""),
                    scale: 3.75
                }
            })
        ),
        ToClientCommand::SetStars(
            Box::new(server_to_client::SetStarsSpec {
                stars: types::StarParams {
                    visible: true,
                    count: 1000,
                    starcolor: SColor::new(105, 235, 235, 255),
                    scale: 1.0,
                    day_opacity: Some(0.0)
                }
            })
        ),
        ToClientCommand::OverrideDayNightRatio(
            Box::new(server_to_client::OverrideDayNightRatioSpec {
                do_override: false,
                day_night_ratio: 0
            })
        )
    ]
}

pub fn empty_inventory() -> ToClientCommand {
    ToClientCommand::Inventory(
        Box::new(server_to_client::InventorySpec {
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
        Box::new(server_to_client::HudaddSpec {
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
                v2i32 {
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
        Box::new(server_to_client::HudaddSpec {
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
                v2i32 {
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
        Box::new(server_to_client::HudaddSpec {
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
                v2i32 {
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
        Box::new(server_to_client::HudaddSpec {
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
                v2i32 {
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
        Box::new(server_to_client::PrivilegesSpec {
            privileges: vec![
                String::from("interact"),
                String::from("shout"),
            ]
        })
    )
}

// 4.317 or 5.612
pub fn get_movementspec(speed: f32) -> ToClientCommand {
    ToClientCommand::Movement(
        Box::new(server_to_client::MovementSpec {
            acceleration_default: 2.9,
            acceleration_air: 1.2,
            acceleration_fast: 10.0,
            speed_walk: speed, //4.317,
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
        Box::new(server_to_client::InventoryFormspecSpec{
            formspec: String::from(formspec),
        })
    )
}

pub fn get_csmrestrictions() -> ToClientCommand {
    ToClientCommand::CsmRestrictionFlags(
        Box::new(server_to_client::CsmRestrictionFlagsSpec {
            csm_restriction_flags: 0,
            csm_restriction_noderange: 0
        })
    )
}

// item def stuff
pub async fn get_item_def_command(path_name_map: &HashMap<String, TextureBlob>) -> ToClientCommand {
    let mc_data_api: Api = utils::compatible_data_api();
    
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
    let mut texture_blob: TextureBlob;
    let mut item_definitions: Vec<ItemDef> = Vec::new();
    for item in mc_data_api.items.items_array().unwrap() {
        mc_name = format!("minecraft:{}", item.name.clone());
        if mc_name.ends_with("_spawn_egg") {
            texture_blob = path_name_map.get("minecraft:template_spawn_egg").unwrap().clone();
        } else {
            texture_blob = path_name_map.get(&mc_name).unwrap().clone();
        }
        // set texture name (as relative path)
        texture_name = texture_blob.get_texture().to_luanti_safe();
        
        utils::logger(&format!("[Itemdefs] Mapped {} to the texture at {}", mc_name, texture_name), 0);
        item_definitions.push(generate_itemdef(&mc_name, item, &texture_name, food_ids.clone(), placeable_ids.clone()));
    }
    
    let alias_definitions: Vec<ItemAlias> = vec![ItemAlias {name: String::from(""), convert_to: String::from("")}];

    ToClientCommand::Itemdef (
        Box::new(server_to_client::ItemdefCommand {
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
        tool_capabilities: types::Option16::None,
        groups,
        node_placement_prediction: String::from(""),
        sound_place: simplesound_placeholder.clone(),
        sound_place_failed: simplesound_placeholder,
        range: 5.0,
        palette_image: String::from(""),
        color: SColor::new(100, 70, 85, 20),
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
    let mut content_features: Vec<(u16, ContentFeatures)> = Vec::new();
    let mut content_feature: ContentFeatures;
    let texture_pack_res: u16 = settings.get_int("texture_pack_res").expect("Failed to read config!") as u16;

    // Azalea provides no nicer way to iterate over blocks, as far as I know.
    for mc_id in 0..std::mem::variant_count::<azalea::registry::Block>() {
        if !azalea::registry::Block::is_valid_id(mc_id as u32) {
            unreachable!();
        }
        // SAFETY: We checked that with is_valid_id above
        // As we are essentially indexing the enum here, `variant_count::<Block>()-1` should be valid. 
        let block = unsafe {
            azalea::registry::Block::from_u32_unchecked(mc_id as u32)
        };
        let mt_id = mc_id as u16 + 128;
        content_feature = generate_contentfeature(block, texture_pack_res, mt_server_state);
        content_features.push((mt_id, content_feature));
    }
    
    // add a special block without MC equivalent: bridgetest:glowing_air. this block will replace cave_air in the nether.
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
        name: String::from("bridgetest:glowing_air"),
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
        post_effect_color: SColor::new(100, 70, 85, 20),
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
        node_box: types::NodeBox::Regular,
        selection_box: types::NodeBox::Regular,
        collision_box: types::NodeBox::Regular,
        sound_footstep: simplesound_placeholder.clone(),
        sound_dig: simplesound_placeholder.clone(),
        sound_dug: simplesound_placeholder.clone(),
        legacy_facedir_simple: false,
        legacy_wallmounted: false,
        node_dig_prediction: String::new(),
        leveled_max: 0,
        alpha: types::AlphaMode::Opaque,
        move_resistance: 0,
        liquid_move_physics: false
    }));
    
    ToClientCommand::Nodedef(
        Box::new(server_to_client::NodedefSpec {
            node_def: types::NodeDefManager {
                content_features,
            }
        })
    )
}

pub fn generate_contentfeature(block: azalea::registry::Block, texture_pack_res: u16, mt_server_state: &mut MTServerState) -> ContentFeatures {
    // If *every* possible state is solid, then walkable=true
    // for light stuff, use the "brightest" state
    // for everything else, do other stuff idk look at the code
    let mc_name = block.to_string();
    let texture_blob: &TextureBlob = mt_server_state.path_name_map.get(&mc_name).unwrap();
    // without extension, to allow messing around with _top etc
    // can't use get_texture, it prefers item textures in ambiguous cases
    let texture = match texture_blob {
        TextureBlob::Block(a)|TextureBlob::Item(a)|TextureBlob::BlockItem(a,_) => a.clone()
    };
    let mut liquid_range = 0;
    let mut liquid_viscosity = 0;
    let mut liquid_renewable = true;
    let mut animation = TileAnimationParams::None;
    
    // liquid stuff
    if block == Block::Water {
        liquid_renewable = true;
        liquid_viscosity = 0; // determines how much the liquid slows the player down
        liquid_range = 7;
    } else if block == Block::Lava {
        liquid_renewable = false;
        liquid_viscosity = 1;
        liquid_range = 4;
    }
    // animated textures
    if [Block::Water, Block::Lava, Block::Seagrass, Block::TallSeagrass, Block::NetherPortal, Block::EndPortal, Block::MagmaBlock].contains(&block) {
        animation = TileAnimationParams::VerticalFrames { aspect_w: texture_pack_res, aspect_h: texture_pack_res, length: 2.0 }
    }

    /*
     * PlantLike: Texture rendered along both diagonal horizontal lines.
     * Normal: Texture rendered on each of the 6 faces.
     * Liquid: Like Normal, but transparency added + shader stuff
     * other stuff - idk too lazy to type, use common sense
     */
    // azalea_registry::blockkind would be ideal, but is unused and thus unusable in azalea.
    // so i need to make this ugly thing matching each block with a non-normal drawtype
    let drawtype = match block {
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
        Block::Fern        => DrawType::PlantLike,
        Block::HangingRoots => DrawType::PlantLike,
        Block::SweetBerryBush => DrawType::PlantLike,
        Block::Seagrass    => DrawType::PlantLike,
        Block::PointedDripstone => DrawType::PlantLike, // totally a plant, whatever

        // works surprisingly fine for tall flowers
        Block::SugarCane   => DrawType::FireLike,
        Block::TallGrass   => DrawType::FireLike,
        Block::LargeFern   => DrawType::FireLike,
        Block::TallSeagrass => DrawType::FireLike,
        Block::RoseBush    => DrawType::FireLike,
        Block::Peony       => DrawType::FireLike,

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
    let walkable = matches!(drawtype, DrawType::AirLike | DrawType::GlassLike | DrawType::Mesh | DrawType::Normal);
    
    let rightclickable = match block {
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
    
    let light_source = match block {
        Block::Beacon | Block::Conduit | Block::EndGateway | Block::EndPortal | Block::Fire | Block::SeaPickle |
            Block::OchreFroglight | Block::VerdantFroglight | Block::PearlescentFroglight | Block::Glowstone | Block::JackOLantern |
            Block::Lantern | Block::Lava | Block::LavaCauldron | Block::Campfire | Block::RedstoneLamp | Block::RespawnAnchor |
            Block::SeaLantern | Block::Shroomlight => 15,
        Block::EndRod | Block::Torch => 14,
        Block::BlastFurnace | Block::Furnace | Block::Smoker => 13,
        Block::Candle => 12,
        Block::NetherPortal => 11,
        Block::CryingObsidian | Block::SoulCampfire | Block::SoulFire | Block::SoulLantern | Block::SoulTorch => 10,
        Block::EnchantingTable | Block::EnderChest | Block::GlowLichen => 7,
        Block::SculkCatalyst => 6,
        Block::AmethystCluster => 5,
        Block::LargeAmethystBud => 4,
        Block::MagmaBlock => 3,
        Block::MediumAmethystBud => 2,
        // TODO level 1 skipped, boring :(
        _ => 0,
    };
    let sunlight_propagates = match drawtype.clone() {
        DrawType::AirLike => 15,
        DrawType::GlassLike => 15,
        DrawType::Liquid => 10,
        _ => 0
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
    fn get_tiledef(texture: &LuantiTexture, animation: &TileAnimationParams) -> TileDef {
        TileDef {
            name: texture.to_luanti_safe(),
            animation: animation.clone(),
            backface_culling: true,
            tileable_horizontal: false,
            tileable_vertical: false,
            color_rgb: utils::get_colormap(texture),
            scale: 0,
            align_style: AlignStyle::Node
        }
    }
    let mut tiledef_sides: [TileDef; 6] = [get_tiledef(&texture, &animation), get_tiledef(&texture, &animation), get_tiledef(&texture, &animation), get_tiledef(&texture, &animation), get_tiledef(&texture, &animation), get_tiledef(&texture, &animation)];
    // if _side/_top/_bottom exists, use that for the respective side(s)
    // use _top for _bottom and _bottom for _top if respectively missing
    match (texture.with_postfix("_top").is_valid(), texture.with_postfix("_bottom").is_valid()) {
        (true, true) => {
            tiledef_sides[0] = get_tiledef(&texture.with_postfix("_top"), &animation);
            tiledef_sides[1] = get_tiledef(&texture.with_postfix("_bottom"), &animation);
        },
        (true, false) => {
            tiledef_sides[0] = get_tiledef(&texture.with_postfix("_top"), &animation);
            tiledef_sides[1] = get_tiledef(&texture.with_postfix("_top"), &animation);
        },
        (false, true) => {
            tiledef_sides[0] = get_tiledef(&texture.with_postfix("_bottom"), &animation);
            tiledef_sides[1] = get_tiledef(&texture.with_postfix("_bottom"), &animation);
        },
        _=>(),
    }
    
    if texture.with_postfix("_side").is_valid() {
        let side_td = get_tiledef(&texture.with_postfix("_side"), &animation);
        tiledef_sides[2] = side_td.clone();
        tiledef_sides[3] = side_td.clone();
        tiledef_sides[4] = side_td.clone();
        tiledef_sides[5] = side_td;
    }
    ContentFeatures {
        version: 13, // https://github.com/minetest/minetest/blob/master/src/nodedef.h#L313
        name: block.to_string(),
        groups: vec![
            (String::from("handy_dig"), 1),
        ],
        param_type: 0,
        param_type_2: 0,
        drawtype: drawtype.clone(),
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
        palette_name: String::new(),
        waving: 0,
        connect_sides: 0,
        connects_to_ids: Vec::new(),
        post_effect_color: SColor::new(100, 70, 85, 20),
        leveled: 0,
        light_propagates: sunlight_propagates,
        sunlight_propagates,
        light_source, // TODO test the effect of this
        is_ground_content: false,
        walkable,
        pointable: drawtype != DrawType::AirLike,
        diggable: block != Block::Bedrock && drawtype != DrawType::Liquid && drawtype != DrawType::AirLike,
        climbable: false,
        buildable_to: !rightclickable, // TODO this is a oversimplification and likely needs its own match abomination
        rightclickable,
        damage_per_second: 0,
        liquid_type_bc: 0,
        liquid_alternative_flowing: String::new(),
        liquid_alternative_source: String::new(),
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
        node_dig_prediction: String::new(),
        leveled_max: 0,
        alpha: types::AlphaMode::Opaque,
        move_resistance: 0,
        liquid_move_physics: false
    }
}
