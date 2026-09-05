use super::super::state::MediaState;
use crate::s2c;
use crate::s2c::media::lookup_block_mapping;
use crate::utils;
use azalea::block::BlockState;
use azalea::registry::builtin::BlockKind;
use config::Config;
use log::*;
use luanti_protocol::LuantiConnection;
use luanti_protocol::commands::server_to_client;
use luanti_protocol::commands::server_to_client::ItemAlias;
use luanti_protocol::commands::server_to_client::ItemDef;
use luanti_protocol::commands::server_to_client::ItemImageDef;
use luanti_protocol::commands::server_to_client::ItemType;
use luanti_protocol::commands::server_to_client::ItemdefList;
use luanti_protocol::commands::server_to_client::ToClientCommand;
use luanti_protocol::commands::server_to_client::TouchInteraction;
use luanti_protocol::types::{
    self, AlignStyle, ContentFeatures, DrawType, Inventory, InventoryEntry, InventoryList,
    ItemStackUpdate, LiquidType, ParamType, ParamType2, PointabilityType, SColor, SoundSpec,
    TileAnimationParams, TileDef,
};
use minecraft_data_rs::{Api, models};
use std::collections::HashMap;

use glam::Vec2 as v2f;
use glam::Vec3 as v3f;

// Luanti IDs below this are reserved (Unknown/Air/Ignored) or used by hacks (glowing air).
pub const CONTENT_ID_OFFSET: u16 = 128;

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
    NoChange, // special value: do not change the heart texture
}

#[derive(Clone)]
pub enum FoodDisplay {
    Normal,
    Hunger,

    NoChange,
}

// Send media announcements (models and textures)
pub fn register_media(luanti_conn: &mut LuantiConnection) {
    debug!("Sending S2C Media Announcement");
    luanti_conn.send(s2c::media::get_announcement()).unwrap();
}

// Send nodedefs
pub async fn register_nodes(
    luanti_conn: &mut LuantiConnection,
    media_state: &mut MediaState,
    settings: &Config,
) {
    luanti_conn
        .send(get_node_def_command(settings, media_state).await)
        .unwrap();
}

// Send itemdefs
pub async fn register_items(luanti_conn: &mut LuantiConnection, media_state: &MediaState) {
    debug!("Sending S2C Itemdef");
    luanti_conn
        .send(get_item_def_command(media_state).await)
        .unwrap();
}

// to send after ClientReady
pub fn register_misc_late(luanti_conn: &mut LuantiConnection) {
    debug!("Sending S2C Hotbar Definition");
    luanti_conn.send(set_hotbar_size()).unwrap();
    luanti_conn.send(set_hotbar_texture()).unwrap();
    luanti_conn.send(set_hotbar_selected()).unwrap();

    debug!("Sending S2C HUD Flags");
    luanti_conn.send(set_hud_flags()).unwrap();

    debug!("Sending S2C Inventory Data");
    luanti_conn.send(empty_inventory()).unwrap();

    debug!("Sending S2C various sky/lighting-related commands");
    for rule_def in get_sky_stuff() {
        luanti_conn.send(rule_def).unwrap();
    }
}

// Send HUD definitions and various stuff (MovementSpec, day/night definition etc)
pub fn register_rules(luanti_conn: &mut LuantiConnection) {
    debug!("Sending S2C MovementSpec");
    luanti_conn.send(get_movementspec(4.317)).unwrap();

    debug!("Sending S2C SetPriv");
    luanti_conn.send(get_defaultpriv()).unwrap();

    debug!("Sending S2C AddHUD (HealthBar)");
    luanti_conn.send(add_healthbar()).unwrap();
    debug!("Sending S2C AddHUD (FoodBar)");
    luanti_conn.send(add_foodbar()).unwrap();
    debug!("Sending S2C AddHUD (AirBar)");
    luanti_conn.send(add_airbar()).unwrap();
    debug!("Sending S2C AddHUD (Subtitles)");
    luanti_conn.send(add_subtitlebox()).unwrap();
    debug!("Sending S2C AddHUD (Effects)");
    luanti_conn.send(add_effect_img()).unwrap();
    debug!("Sending S2C AddHUD (Hotbar)");
    luanti_conn.send(add_hudbar()).unwrap();

    debug!("Sending S2C Formspec (Inventory)");
    luanti_conn
        .send(get_inventory_formspec(PLAYER_INV_FORMSPEC))
        .unwrap();

    debug!("Sending S2C CsmRestrictions");
    luanti_conn.send(get_csmrestrictions()).unwrap();
}

// misc GUI/formspec stuff
pub const HEALTHBAR_ID: u32 = 0;
pub const FOODBAR_ID: u32 = 1;
pub const AIRBAR_ID: u32 = 2;
pub const SUBTITLE_ID: u32 = 3;
pub const EFFECTS_ID: u32 = 4;
pub const HOTBAR_ID: u32 = 5;

pub const PLAYER_INV_FORMSPEC: &str = "\
formspec_version[7]
size[12,11.3]
background[0,0;17.45,17.45;gui-container-inventory.png]
style_type[list;spacing=0.135,0.135;size=1.09,1.09;border=false]
listcolors[#0000;#0002]
list[current_player;armor;0.55,0.575;1,4]
list[current_player;craft;6.7,1.26;2,2]
list[current_player;craftpreview;10.5,1.9;1,1]
list[current_player;offhand;5.29,4.25;1,1]
list[current_player;main;0.55,9.7;9,1]
list[current_player;main;0.55,5.75;9,3;9]
list[current_player;container;0,0;0,0]
";

// list[current_player; _NAME_ ; x,y ; size_x,size_y;]
pub const ALL_INV_FIELDS: [&str; 6] = [
    "main",
    "armor",
    "offhand",
    "craft",
    "craftpreview",
    "container",
]; // container is dynamic in size

pub const HOTBAR_SIZE: i32 = 9;

pub fn set_hotbar_size() -> ToClientCommand {
    ToClientCommand::HudSetParam(Box::new(server_to_client::HudSetParamSpec {
        value: types::HudSetParam::SetHotBarItemCount(HOTBAR_SIZE),
    }))
}

pub fn set_hotbar_texture() -> ToClientCommand {
    ToClientCommand::HudSetParam(Box::new(server_to_client::HudSetParamSpec {
        value: types::HudSetParam::SetHotBarImage(String::from("gui-sprites-hud-hotbar.png")),
    }))
}

pub fn set_hotbar_selected() -> ToClientCommand {
    ToClientCommand::HudSetParam(Box::new(server_to_client::HudSetParamSpec {
        value: types::HudSetParam::SetHotBarSelectedImage(String::from(
            "gui-sprites-hud-hotbar_selection.png",
        )),
    }))
}

pub fn set_hud_flags() -> ToClientCommand {
    ToClientCommand::HudSetFlags(Box::new(server_to_client::HudSetFlagsSpec {
        flags: types::HudFlags {
            hotbar_visible: true,
            healthbar_visible: true,
            crosshair_visible: true,
            wielditem_visible: true,
            breathbar_visible: false, // tick.rs updates that
            minimap_visible: false,
            minimap_radar_visible: false,
            basic_debug: true,
            chat_visible: true,
        },
        // which of the above should be applied (all)
        mask: types::HudFlags {
            hotbar_visible: true,
            healthbar_visible: true,
            crosshair_visible: true,
            wielditem_visible: true,
            breathbar_visible: true,
            minimap_visible: true,
            minimap_radar_visible: true,
            basic_debug: true,
            chat_visible: true,
        },
    }))
}

// Values here mostly from copying Mineclonia, should not need adjustments
pub fn get_sky_stuff() -> [ToClientCommand; 7] {
    [
        ToClientCommand::SetLighting(Box::new(server_to_client::SetLightingSpec {
            lighting: types::Lighting {
                shadow_intensity: 0.33,
                saturation: 1.1,
                exposure: types::AutoExposure {
                    luminance_min: -3.5,
                    luminance_max: -2.5,
                    exposure_correction: 0.33,
                    speed_dark_bright: 1500.0,
                    speed_bright_dark: 700.0,
                    center_weight_power: 1.0,
                },
                volumetric_light_strength: 0.3,
                shadow_tint: SColor::new(255, 0, 0, 0),
                shadow_direction: v3f::ZERO,
                bloom_intensity: 0.05,
                bloom_strength_factor: 1.0,
                bloom_radius: 1.0,
            },
        })),
        ToClientCommand::SetSky(Box::new(server_to_client::SetSkyCommand {
            params: server_to_client::SkyboxParams {
                bgcolor: SColor::new(255, 255, 255, 255),
                clouds: true,
                fog_sun_tint: SColor::new(255, 255, 95, 51),
                fog_moon_tint: SColor::new(255, 255, 255, 255),
                fog_tint_type: String::from("custom"),
                data: server_to_client::SkyboxData::Color(types::SkyColor {
                    day_sky: SColor::new(255, 124, 163, 255),
                    day_horizon: SColor::new(255, 192, 216, 255),
                    dawn_sky: SColor::new(255, 124, 163, 255),
                    dawn_horizon: SColor::new(255, 192, 216, 255),
                    night_sky: SColor::new(255, 0, 0, 0),
                    night_horizon: SColor::new(255, 74, 103, 144),
                    indoors: SColor::new(255, 192, 216, 255),
                }),
                r#type: String::from("regular"),
                body_orbit_tilt: 0.0,
                fog_distance: -1,
                fog_start: -1.0,
                fog_color: SColor::new(0, 0, 0, 0),
                auto_dim_skybox: None,
            },
        })),
        ToClientCommand::SetSun(Box::new(server_to_client::SetSunSpec {
            sun: types::SunParams {
                visible: true,
                texture: String::from("environment-celestial-sun.png"),
                tonemap: String::from(""),
                sunrise: String::from(""),
                sunrise_visible: true,
                scale: 1.0,
            },
        })),
        ToClientCommand::SetMoon(Box::new(server_to_client::SetMoonSpec {
            moon: types::MoonParams {
                visible: true,
                texture: String::from("environment-celestial-moon-full_moon.png"),
                tonemap: String::from(""),
                scale: 3.75,
            },
        })),
        ToClientCommand::SetStars(Box::new(server_to_client::SetStarsSpec {
            stars: types::StarParams {
                visible: true,
                count: 1000,
                starcolor: SColor::new(105, 235, 235, 255),
                scale: 1.0,
                day_opacity: Some(0.0),
                star_seed: None,
            },
        })),
        ToClientCommand::CloudParams(Box::new(server_to_client::CloudParamsSpec {
            density: 0.4,
            color_bright: SColor::new(229, 240, 240, 255),
            color_ambient: SColor::new(0, 0, 0, 255),
            height: 196.0, // :3
            thickness: 4.0,
            speed: v2f::new(-2.0, 0.0),
            color_shadow: SColor::new(255, 204, 204, 204),
        })),
        ToClientCommand::OverrideDayNightRatio(Box::new(
            server_to_client::OverrideDayNightRatioSpec {
                do_override: false,
                day_night_ratio: 0,
            },
        )),
    ]
}

pub fn empty_inventory() -> ToClientCommand {
    ToClientCommand::Inventory(Box::new(server_to_client::InventorySpec {
        inventory: Inventory {
            entries: vec![
                InventoryEntry::Update(InventoryList {
                    name: String::from("main"),
                    width: 0,
                    items: vec![ItemStackUpdate::Empty; 36],
                }),
                InventoryEntry::Update(InventoryList {
                    name: String::from("armor"),
                    width: 0,
                    items: vec![ItemStackUpdate::Empty; 4],
                }),
                InventoryEntry::Update(InventoryList {
                    name: String::from("offhand"),
                    width: 0,
                    items: vec![ItemStackUpdate::Empty],
                }),
                InventoryEntry::Update(InventoryList {
                    name: String::from("craft"),
                    width: 3,
                    items: vec![ItemStackUpdate::Empty; 4],
                }),
                InventoryEntry::Update(InventoryList {
                    name: String::from("craftpreview"),
                    width: 0,
                    items: vec![ItemStackUpdate::Empty],
                }),
            ],
        },
        skip_wield_anim: false,
    }))
}

pub fn add_healthbar() -> ToClientCommand {
    ToClientCommand::Hudadd(Box::new(server_to_client::HudaddSpec {
        server_id: HEALTHBAR_ID,
        typ: 2,
        pos: v2f { x: 0.5, y: 1.0 },
        name: String::from(""),
        scale: v2f { x: 0.0, y: 0.0 },
        text: String::from("gui-sprites-hud-heart-full.png"),
        number: 20,
        item: 20,
        dir: 0,
        align: v2f { x: 0.0, y: 0.0 },
        offset: v2f {
            x: -265.0,
            y: -88.0,
        },
        world_pos: v3f::ZERO,
        size: v2f { x: 24.0, y: 24.0 },
        z_index: Some(0),
        text2: Some(String::from("gui-sprites-hud-heart-container.png")),
        style: Some(0),
        flags: None,
    }))
}

pub fn add_foodbar() -> ToClientCommand {
    ToClientCommand::Hudadd(Box::new(server_to_client::HudaddSpec {
        server_id: FOODBAR_ID,
        typ: 2,
        pos: v2f { x: 0.5, y: 1.0 },
        name: String::from(""),
        scale: v2f { x: 0.0, y: 0.0 },
        text: String::from("gui-sprites-hud-food_full.png"),
        number: 20,
        item: 20,
        dir: 0,
        align: v2f { x: 0.0, y: 0.0 },
        offset: v2f { x: 45.0, y: -88.0 },
        world_pos: v3f::ZERO,
        size: v2f { x: 24.0, y: 24.0 },
        z_index: Some(0),
        text2: Some(String::from("gui-sprites-hud-food_empty.png")),
        style: Some(0),
        flags: None,
    }))
}

pub fn add_airbar() -> ToClientCommand {
    ToClientCommand::Hudadd(Box::new(server_to_client::HudaddSpec {
        server_id: AIRBAR_ID,
        typ: 2,
        pos: v2f { x: 0.5, y: 1.0 },
        name: String::from(""),
        scale: v2f { x: 0.0, y: 0.0 },
        text: String::from("gui-sprites-hud-air.png"),
        number: 0, // default to not show this element
        item: 0,   // item count also gets changed when needed
        dir: 0,
        align: v2f { x: 0.0, y: 0.0 },
        offset: v2f { x: 45.0, y: -113.0 },
        world_pos: v3f::ZERO,
        size: v2f { x: 24.0, y: 24.0 },
        z_index: Some(0),
        text2: Some(String::from("gui-sprites-hud-air_bursting.png")),
        style: Some(0),
        flags: None,
    }))
}

pub fn add_subtitlebox() -> ToClientCommand {
    ToClientCommand::Hudadd(Box::new(server_to_client::HudaddSpec {
        server_id: SUBTITLE_ID,
        typ: 1,
        pos: v2f { x: 0.5, y: 1.0 },
        name: String::from(""),
        scale: v2f { x: 0.0, y: 0.0 },
        text: String::from("-\n-"),
        number: 0, // default to not show this element
        item: 20,
        dir: 0,
        align: v2f { x: 0.0, y: 0.0 },
        offset: v2f {
            x: -265.0,
            y: -116.0,
        },
        world_pos: v3f::ZERO,
        size: v2f { x: 1.0, y: 1.0 },
        z_index: Some(0),
        text2: Some(String::new()),
        style: Some(0),
        flags: None,
    }))
}

pub fn add_effect_img() -> ToClientCommand {
    ToClientCommand::Hudadd(Box::new(server_to_client::HudaddSpec {
        server_id: EFFECTS_ID,
        typ: 0,
        pos: v2f { x: 1.0, y: 0.0 },
        name: String::new(),
        scale: v2f { x: 2.0, y: 2.0 },
        text: String::from(""),
        number: 0,
        item: 20,
        dir: 0,
        // offset is top-right corner
        align: v2f { x: 1.0, y: 1.0 },
        offset: v2f { x: -54.0, y: 6.0 },
        world_pos: v3f::ZERO,
        size: v2f { x: 24.0, y: 24.0 },
        z_index: Some(0),
        text2: None,
        style: None,
        flags: None,
    }))
}

pub fn add_hudbar() -> ToClientCommand {
    ToClientCommand::Hudadd(Box::new(server_to_client::HudaddSpec {
        server_id: HOTBAR_ID,
        typ: 8, // HUD_ELEM_HOTBAR
        pos: v2f { x: 0.5, y: 1.0 },
        name: String::new(),
        scale: v2f::ZERO,
        text: String::new(),
        number: 0,
        item: 0,
        dir: 0,
        align: v2f { x: 0.0, y: -1.0 },
        // extra padding below the hotbar
        offset: v2f { x: 0.0, y: -4.0 },
        world_pos: v3f::ZERO,
        size: v2f::ZERO,
        z_index: Some(0),
        text2: None,
        style: None,
        flags: None,
    }))
}

pub fn get_defaultpriv() -> ToClientCommand {
    ToClientCommand::Privileges(Box::new(server_to_client::PrivilegesSpec {
        privileges: vec![String::from("interact"), String::from("shout")],
    }))
}

// 4.317 or 5.612
pub fn get_movementspec(speed: f32) -> ToClientCommand {
    ToClientCommand::Movement(Box::new(server_to_client::MovementSpec {
        acceleration_default: 2.9,
        acceleration_air: 1.2,
        acceleration_fast: 10.0,
        speed_walk: speed, //4.317, variable for sprinting (speed_fast only works if i could get the client to use it :3)
        speed_crouch: 1.295,
        speed_fast: 5.612,
        speed_climb: 2.35,
        speed_jump: 7.494, // 1.249 (height gain) * 0.6 (jump duration), *10 because it works that way idk
        liquid_fluidity: 1.13,
        liquid_fluidity_smooth: 0.5,
        liquid_sink: 23.0,
        gravity: 10.4,
    }))
}

pub fn get_inventory_formspec(formspec: &str) -> ToClientCommand {
    ToClientCommand::InventoryFormspec(Box::new(server_to_client::InventoryFormspecSpec {
        formspec: String::from(formspec),
    }))
}

pub fn get_csmrestrictions() -> ToClientCommand {
    ToClientCommand::CsmRestrictionFlags(Box::new(server_to_client::CsmRestrictionFlagsSpec {
        csm_restriction_flags: 0,
        csm_restriction_noderange: 0,
    }))
}

// constants
pub const INTERACTIVE_BLOCKS: [BlockKind; 50] = [
    // opens inventory
    BlockKind::Chest,
    BlockKind::EnderChest,
    BlockKind::EnchantingTable,
    BlockKind::Anvil,
    BlockKind::Grindstone,
    BlockKind::CraftingTable,
    // changes own state
    BlockKind::Lever,
    BlockKind::Comparator,
    BlockKind::Repeater,
    BlockKind::RedstoneOre,
    BlockKind::RedstoneWire,
    BlockKind::OakButton,
    BlockKind::SpruceButton,
    BlockKind::BirchButton,
    BlockKind::JungleButton,
    BlockKind::AcaciaButton,
    BlockKind::DarkOakButton,
    BlockKind::MangroveButton,
    BlockKind::CherryButton,
    BlockKind::BambooButton,
    BlockKind::CrimsonButton,
    BlockKind::WarpedButton,
    // other stuff
    BlockKind::WhiteBed,
    BlockKind::LightGrayBed,
    BlockKind::GrayBed,
    BlockKind::BlackBed,
    BlockKind::BrownBed,
    BlockKind::RedBed,
    BlockKind::OrangeBed,
    BlockKind::YellowBed,
    BlockKind::LimeBed,
    BlockKind::CyanBed,
    BlockKind::LightBlueBed,
    BlockKind::BlueBed,
    BlockKind::PurpleBed,
    BlockKind::MagentaBed,
    BlockKind::PinkBed,
    BlockKind::Campfire,
    BlockKind::SoulCampfire,
    BlockKind::Cauldron,
    BlockKind::Cake,
    BlockKind::CandleCake,
    BlockKind::RedCandleCake,
    BlockKind::BlueCandleCake,
    BlockKind::CyanCandleCake,
    BlockKind::GrayCandleCake,
    BlockKind::LimeCandleCake,
    BlockKind::PinkCandleCake,
    BlockKind::BlackCandleCake,
    BlockKind::BrownCandleCake,
];

// item def stuff
pub async fn get_item_def_command(media_state: &MediaState) -> ToClientCommand {
    let mc_data_api: Api = utils::compatible_data_api();

    // we need food- and placeable IDs to predict right-click behavior of every item
    let food_ids: Vec<u32> = mc_data_api.foods.foods().unwrap().into_keys().collect();
    // assume placeable when a object with the same name exists as a block
    let block_names: Vec<String> = mc_data_api
        .blocks
        .blocks_array()
        .unwrap()
        .iter()
        .map(|item| item.name.clone())
        .collect();
    let placeable_ids: Vec<u32> = mc_data_api
        .items
        .items_array()
        .unwrap()
        .iter()
        .filter(|item| block_names.contains(&item.name))
        .map(|item| item.id)
        .collect();

    let mut mc_name: String;
    let mut inventory_image: String;
    let mut item_definitions: Vec<ItemDef> = Vec::new();
    for item in mc_data_api.items.items_array().unwrap() {
        mc_name = format!("minecraft:{}", item.name.clone());
        // generate inventory image
        // if only present as block mapping, use inventory cube
        // this logic is duplicated in utils::texture_from_itemstack
        if media_state.item_texture_map.contains_key(&mc_name) {
            inventory_image = media_state
                .item_texture_map
                .get(&mc_name)
                .unwrap()
                .clone()
                .to_luanti_safe();
        } else {
            inventory_image = lookup_block_mapping(&media_state.block_texture_map, &mc_name, "")
                .expect("block_texture_map invalid, mapping messed up!")
                .clone()
                .to_safe_cube();
        }

        item_definitions.push(generate_itemdef(
            &mc_name,
            item,
            inventory_image,
            food_ids.clone(),
            placeable_ids.clone(),
        ));
    }

    let alias_definitions: Vec<ItemAlias> = vec![ItemAlias {
        name: String::from(""),
        convert_to: String::from(""),
    }];

    ToClientCommand::Itemdef(Box::new(server_to_client::ItemdefCommand {
        item_def: ItemdefList {
            itemdef_manager_version: 0, // https://github.com/minetest/minetest/blob/master/src/itemdef.cpp#L616
            defs: item_definitions,
            aliases: alias_definitions,
        },
    }))
}

pub fn generate_itemdef(
    name: &str,
    item: models::item::Item,
    inventory_image: String,
    food_ids: Vec<u32>,
    placeable_ids: Vec<u32>,
) -> ItemDef {
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
        groups.push((String::from("building_block"), 1))
    }

    let sound_placeholder: SoundSpec = SoundSpec {
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
        inventory_image: ItemImageDef::plain(inventory_image.clone()),
        wield_image: ItemImageDef::plain(inventory_image),
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
        sound_place: sound_placeholder.clone(),
        sound_place_failed: sound_placeholder.clone(),
        range: 5.0,
        palette_image: String::from(""),
        color: SColor::new(255, 255, 255, 255),
        inventory_overlay: ItemImageDef::plain(String::from("")),
        wield_overlay: ItemImageDef::plain(String::from("")),
        short_description: String::from("Proxy fucked up, sorry!"),
        sound_use: sound_placeholder.clone(),
        sound_use_air: sound_placeholder,
        place_param2: None,
        wallmounted_rotate_vertical: false,
        touch_interaction: TouchInteraction::all_user(),
        pointabilities: types::Option16::None,
        wear_bar_params: None,
    }
}

// node def stuff
//
// Luanti node definitions are per content-id, not block-kind, so every BlockState
// gets resolved to its own shape in scripts/texture_maps.py, then deduplicated here.
// the resulting table is stored on MediaState for utils::state_to_node to use
pub async fn get_node_def_command(
    settings: &Config,
    media_state: &mut MediaState,
) -> ToClientCommand {
    let texture_pack_res: u16 = settings.get_int("media.texture_pack_res").unwrap() as u16;

    let mut content_features: Vec<(u16, ContentFeatures)> = Vec::new();
    // dedup key -> content id
    let mut signature_to_id: HashMap<String, u16> = HashMap::new();
    // non default variants get "name_1" etc to have unique names
    let mut name_variant_counts: HashMap<String, u32> = HashMap::new();
    let mut state_content_ids: Vec<u16> = vec![0u16; BlockState::MAX_STATE as usize + 1];
    let mut next_id: u32 = CONTENT_ID_OFFSET as u32;

    for raw_id in 0..=BlockState::MAX_STATE {
        let state = BlockState::try_from(raw_id).expect("id in MAX_STATE range is always valid");
        if state.is_air() {
            continue;
        }
        let kind = BlockKind::try_from(state).unwrap();

        let (announce_name, mut feature) = if kind == BlockKind::Water || kind == BlockKind::Lava {
            generate_liquid_feature(kind, state, texture_pack_res)
        } else {
            let block_name = kind.to_str();
            let variant_key = utils::variant_key_from_state(state);
            let feature = generate_contentfeature(
                state,
                kind,
                block_name,
                &variant_key,
                texture_pack_res,
                media_state,
            );
            (block_name.to_string(), feature)
        };

        let mut signature_source = feature.clone();
        signature_source.name = String::new();
        let signature = format!("{signature_source:?}");

        let content_id = if let Some(&id) = signature_to_id.get(&signature) {
            id
        } else {
            let id = next_id as u16;
            next_id += 1;
            let count = name_variant_counts
                .entry(announce_name.clone())
                .or_insert(0);
            feature.name = if *count == 0 {
                announce_name
            } else {
                format!("{announce_name}_{count}")
            };
            *count += 1;
            signature_to_id.insert(signature, id);
            content_features.push((id, feature));
            id
        };
        state_content_ids[raw_id as usize] = content_id;
    }
    assert!(
        next_id <= u16::MAX as u32,
        "ran out of Luanti content ids ({next_id} needed, {} available)",
        u16::MAX as u32 - CONTENT_ID_OFFSET as u32
    );
    debug!(
        "Resolved {} block states into {} distinct Luanti node definitions",
        BlockState::MAX_STATE as u32 + 1,
        content_features.len()
    );
    media_state.state_content_ids = state_content_ids;

    // add a special block without MC equivalent: bridgetest:glowing_air. this block will replace cave_air in the nether.
    // because the minetest engine has no concept of dimensions, it is impossible to tell it to make air glow in the nether.
    let tiledef = TileDef {
        name: String::from("blank.png"),
        animation: TileAnimationParams::None,
        backface_culling: true,
        tileable_horizontal: false,
        tileable_vertical: false,
        color_rgb: None,
        scale: 0,
        align_style: AlignStyle::Node,
    };
    let sound_placeholder: SoundSpec = SoundSpec {
        name: String::from(""),
        gain: 1.0,
        pitch: 1.0,
        fade: 1.0,
    };
    let tiledef_sides = [
        tiledef.clone(),
        tiledef.clone(),
        tiledef.clone(),
        tiledef.clone(),
        tiledef.clone(),
        tiledef.clone(),
    ];
    content_features.push((
        120,
        ContentFeatures {
            version: 13,
            name: String::from("bridgetest:glowing_air"),
            groups: vec![(String::from(""), 1)],
            param_type: ParamType::Light,
            param_type_2: ParamType2::None,
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
            light_propagates: true,
            sunlight_propagates: true,
            light_source: 15,
            is_ground_content: false,
            walkable: false,
            pointable: PointabilityType::PointableNot,
            diggable: false,
            climbable: false,
            buildable_to: false,
            rightclickable: false,
            damage_per_second: 0,
            liquid_type: LiquidType::None,
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
            sound_footstep: sound_placeholder.clone(),
            sound_dig: sound_placeholder.clone(),
            sound_dug: sound_placeholder.clone(),
            legacy_facedir_simple: false,
            legacy_wallmounted: false,
            node_dig_prediction: String::new(),
            leveled_max: 0,
            alpha: types::AlphaMode::Opaque,
            move_resistance: 0,
            liquid_move_physics: false,
            post_effect_color_shaded: false,
        },
    ));

    ToClientCommand::Nodedef(Box::new(server_to_client::NodedefSpec {
        node_def: types::NodeDefManager { content_features },
    }))
}

pub fn generate_contentfeature(
    state: BlockState,
    block: BlockKind,
    mc_name: &str,
    variant_key: &str,
    texture_pack_res: u16,
    media_state: &MediaState,
) -> ContentFeatures {
    let mut animation = TileAnimationParams::None;
    // animated textures
    if [
        BlockKind::Seagrass,
        BlockKind::TallSeagrass,
        BlockKind::NetherPortal,
        BlockKind::EndPortal,
        BlockKind::MagmaBlock,
    ]
    .contains(&block)
    {
        animation = TileAnimationParams::VerticalFrames {
            aspect_w: texture_pack_res,
            aspect_h: texture_pack_res,
            length: 2.0,
        }
    }

    let rightclickable = INTERACTIVE_BLOCKS.contains(&block);

    // extra_data/block_info.json
    let info = media_state.block_info.get(mc_name);
    let mut light_source = info.map(|i| i.light_source).unwrap_or(0);
    if let Some(lit_value) = info.and_then(|i| i.lit_light_source) {
        if state.to_trait().get_property("lit") == Some("true") {
            light_source = lit_value;
        }
    }
    let climbable = info.is_some_and(|i| i.climbable);
    let glasslike = info.is_some_and(|i| i.glasslike);

    let Some(texture) = lookup_block_mapping(&media_state.block_texture_map, mc_name, variant_key)
    else {
        error!("Block texture not mapped to path: {mc_name} [{variant_key}]");
        std::process::exit(1)
    };

    let cutout = texture.cutout || info.is_some_and(|i| i.cutout);

    let drawtype = if glasslike {
        DrawType::GlassLike
    } else if cutout && texture.drawtype == DrawType::Normal {
        // fix leaves culling
        DrawType::AllFacesOptional
    } else {
        texture.drawtype.clone()
    };
    // scripts/texture_maps.py only puts DrawType::Normal for full cubes
    // anything else already has some gap for light to pass through
    // cutout is an exception here.
    let light_propagates = drawtype != DrawType::Normal || cutout;

    let waving: u8 =
        (info.is_some_and(|i| i.waving) || texture.drawtype == DrawType::PlantLike) as u8 * 100;

    let sound_placeholder: SoundSpec = SoundSpec {
        name: String::from(""),
        gain: 1.0,
        pitch: 1.0,
        fade: 1.0,
    };

    let mut tiledef_sides: [TileDef; 6] = texture.get_tiledefs(&animation);
    if block == BlockKind::RedstoneWire {
        // texture map file is huge as-is, don't add sixteen copies of every redstone orientation
        // colorize depending on signal level:
        let power: u32 = state
            .to_trait()
            .get_property("power")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let red = 75 + 12 * power; // 75 (unpowered) ..= 255 (power 15)
        let colorize = format!("^[colorize:#{red:02x}0000:200");
        tiledef_sides[0].name.push_str(&colorize);
        tiledef_sides[1].name.push_str(&colorize);
    }
    let walkable = matches!(
        drawtype,
        DrawType::GlassLike | DrawType::NodeBox | DrawType::Normal | DrawType::AllFacesOptional
    );
    let bool_pointable = drawtype != DrawType::AirLike && drawtype != DrawType::Liquid;
    let mut pointable = PointabilityType::PointableNot;
    if (bool_pointable) {
        pointable = PointabilityType::Pointable;
    }
    ContentFeatures {
        version: 13, // https://github.com/minetest/minetest/blob/master/src/nodedef.h#L313
        name: mc_name.to_string(),
        groups: vec![(String::from("handy_dig"), 1)],
        // CPT_LIGHT: tells the client that param1 carries light data (low 4 bit = day/sky,
        // high 4 bit = block). Without this, has_light=false and getLightRaw() returns 0,
        // so blocks ignore our light values entirely and never dim at night.
        param_type: ParamType::Light,
        param_type_2: ParamType2::None,
        drawtype: drawtype.clone(),
        mesh: String::new(),
        visual_scale: match texture.drawtype {
            DrawType::NodeBox => s2c::media::NB_SCALE_FACTOR,
            _ => 1.0,
        },
        unused_six: 6, // unused? idk what does this even do
        tiledef: tiledef_sides.clone(),
        tiledef_overlay: s2c::media::get_empty_tiledefs(),
        // unexplained in the minetest-protocol crate
        tiledef_special: s2c::media::get_empty_tiledefs().to_vec(),
        alpha_for_legacy: 160, // only used for liquids
        red: 100,
        green: 70,
        blue: 85,
        palette_name: String::new(),
        waving,
        connect_sides: 0,
        connects_to_ids: Vec::new(),
        post_effect_color: SColor::new(100, 70, 85, 20),
        leveled: 0,
        light_propagates,
        sunlight_propagates: light_propagates,
        light_source,
        is_ground_content: false,
        walkable,
        pointable,
        diggable: block != BlockKind::Bedrock
            && drawtype != DrawType::Liquid
            && drawtype != DrawType::AirLike,
        climbable,
        buildable_to: bool_pointable && !rightclickable,
        rightclickable,
        damage_per_second: 0, // the 100 DPS dirt block
        liquid_type: LiquidType::None,
        liquid_alternative_flowing: String::new(),
        liquid_alternative_source: String::new(),
        liquid_viscosity: 0,
        liquid_renewable: false,
        liquid_range: 0,
        drowning: 0,
        floodable: false,
        node_box: texture.nodebox.clone(),
        selection_box: texture.nodebox.clone(),
        collision_box: texture.nodebox.clone(),
        sound_footstep: sound_placeholder.clone(),
        sound_dig: sound_placeholder.clone(),
        sound_dug: sound_placeholder.clone(),
        legacy_facedir_simple: true,
        legacy_wallmounted: false,
        node_dig_prediction: String::new(),
        leveled_max: 0,
        alpha: if glasslike {
            types::AlphaMode::Blend
        } else if cutout {
            types::AlphaMode::Clip
        } else {
            match texture.drawtype {
                DrawType::PlantLike | DrawType::PlantLikeRooted => types::AlphaMode::Blend,
                _ => types::AlphaMode::Opaque,
            }
        },
        move_resistance: 0,
        liquid_move_physics: false,
        post_effect_color_shaded: false,
    }
}

// Water and lava are the only fluids MC has, neither has a blockstates.json model (the client does special cases yay)
// this means we also get to do special cases.
// the actual per-node height comes from param2, set in utils::state_to_node
fn generate_liquid_feature(
    kind: BlockKind,
    state: BlockState,
    texture_pack_res: u16,
) -> (String, ContentFeatures) {
    let is_water = kind == BlockKind::Water;
    let base = if is_water { "water" } else { "lava" };
    let raw_level: u8 = state
        .to_trait()
        .get_property("level")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let is_source = raw_level == 0;

    let source_name = format!("minecraft:{base}");
    let flowing_name = format!("minecraft:{base}_flowing");
    let announce_name = if is_source {
        source_name.clone()
    } else {
        flowing_name.clone()
    };

    let animation = TileAnimationParams::VerticalFrames {
        aspect_w: texture_pack_res,
        aspect_h: texture_pack_res,
        length: 2.0,
    };
    let tile = |name: &str| TileDef {
        name: name.to_string(),
        animation: animation.clone(),
        backface_culling: true,
        tileable_horizontal: true,
        tileable_vertical: true,
        color_rgb: None,
        scale: 0,
        align_style: AlignStyle::Node,
    };
    let still = format!("block-{base}_still.png");
    let flow = format!("block-{base}_flow.png");
    let tiledef: [TileDef; 6] = if is_source {
        std::array::from_fn(|_| tile(&still))
    } else {
        [
            tile(&still),
            tile(&still),
            tile(&flow),
            tile(&flow),
            tile(&flow),
            tile(&flow),
        ]
    };

    let sound_placeholder: SoundSpec = SoundSpec {
        name: String::from(""),
        gain: 1.0,
        pitch: 1.0,
        fade: 1.0,
    };

    // evil
    let (light_source, viscosity, renewable, range, post_effect, drowning) = if is_water {
        (0u8, 0u8, true, 7u8, SColor::new(38, 68, 168, 120), 2u8)
    } else {
        (15u8, 7u8, false, 4u8, SColor::new(208, 88, 0, 190), 0u8)
    };

    let feature: ContentFeatures = ContentFeatures {
        version: 13,
        name: announce_name.clone(), // overwritten by the caller naming pass
        groups: vec![(String::new(), 1)],
        param_type: ParamType::Light,
        param_type_2: if is_source {
            ParamType2::None
        } else {
            ParamType2::FlowingLiquid
        },
        drawtype: if is_source {
            DrawType::Liquid
        } else {
            DrawType::FlowingLiquid
        },
        mesh: String::new(),
        visual_scale: 1.0,
        unused_six: 6,
        tiledef: tiledef.clone(),
        tiledef_overlay: s2c::media::get_empty_tiledefs(),
        tiledef_special: tiledef.to_vec(),
        alpha_for_legacy: 160,
        red: 100,
        green: 70,
        blue: 85,
        palette_name: String::new(),
        waving: 0,
        connect_sides: 0,
        connects_to_ids: Vec::new(),
        post_effect_color: post_effect,
        leveled: 0,
        light_propagates: false,
        sunlight_propagates: false,
        light_source,
        is_ground_content: false,
        walkable: false,
        pointable: PointabilityType::PointableNot,
        diggable: false,
        climbable: false,
        buildable_to: true,
        rightclickable: false,
        damage_per_second: 0,
        liquid_type: if is_source {
            LiquidType::Source
        } else {
            LiquidType::Flowing
        },
        liquid_alternative_flowing: flowing_name,
        liquid_alternative_source: source_name,
        liquid_viscosity: viscosity,
        liquid_renewable: renewable,
        liquid_range: range,
        drowning,
        floodable: false,
        node_box: types::NodeBox::Regular,
        selection_box: types::NodeBox::Regular,
        collision_box: types::NodeBox::Regular,
        sound_footstep: sound_placeholder.clone(),
        sound_dig: sound_placeholder.clone(),
        sound_dug: sound_placeholder.clone(),
        legacy_facedir_simple: false,
        legacy_wallmounted: false,
        node_dig_prediction: String::new(),
        leveled_max: 0,
        alpha: types::AlphaMode::Blend,
        move_resistance: 0,
        liquid_move_physics: true,
        post_effect_color_shaded: false,
    };
    (announce_name, feature)
}
