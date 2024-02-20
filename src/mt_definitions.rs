// ItemDefinitions and BlockDefinitions to be sent to the minetest client

use minetest_protocol::wire::command::{ItemdefSpec, NodedefSpec, ToClientCommand};
use minetest_protocol::wire::types::{s16, Option16, v3f, SColor, SimpleSoundSpec, // generic types
    ItemdefList, ItemDef, ToolCapabilities, ToolGroupCap, ItemAlias, ItemType, // item specific
    NodeDefManager, ContentFeatures, TileDef, AlignStyle, TileAnimationParams, NodeBox, AlphaMode, DrawType // node specific (even more complicated than items qwq)
    };

use alloc::boxed::Box;

pub fn get_item_def_command() -> ToClientCommand{
    pub struct Defaults {
        simplesound: SimpleSoundSpec,
        itemdef: ItemDef,
        itemalias: ItemAlias,
    }

    let placeholder: Defaults = Defaults {
        simplesound: SimpleSoundSpec {
            name: "[[ERROR]]".to_string(),
            gain: 1.0,
            pitch: 1.0,
            fade: 1.0,
        },
        itemdef: ItemDef {
            version: 6, // https://github.com/minetest/minetest/blob/master/src/itemdef.cpp#L192
            item_type: ItemType::None,
            name: "[[ERROR]]".to_string(),
            description: "A unexpected (actually expected) error occured. The proxy was unable to map a MC item to MT".to_string(),
            inventory_image: "".to_string(), // TODO: That is not an image.
            wield_image: "".to_string(),
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
            node_placement_prediction: "".to_string(),
            sound_place: SimpleSoundSpec {
                name: "[[ERROR]]".to_string(),
                gain: 1.0,
                pitch: 1.0,
                fade: 1.0,
            },
            sound_place_failed: SimpleSoundSpec {
                name: "[[ERROR]]".to_string(),
                gain: 1.0,
                pitch: 1.0,
                fade: 1.0,
            },
            range: 5.0,
            palette_image: "".to_string(),
            color: SColor {
                r: 100,
                g: 70,
                b: 85,
                a: 20,
            },
            inventory_overlay: "".to_string(),
            wield_overlay: "".to_string(),
            short_description: Some("Proxy fucked up, sorry!".to_string()),
            place_param2: None,
            sound_use: None,
            sound_use_air: None
        },
        itemalias: ItemAlias {
            name: "".to_string(),
            convert_to: "".to_string()

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
        name: "[[ERROR]]".to_string(),
        gain: 1.0,
        pitch: 1.0,
        fade: 1.0,
    };

    let tiledef_placeholder: TileDef = TileDef {
        name: "[[ERROR]]".to_string(),
        animation: TileAnimationParams::None,
        backface_culling: false,
        tileable_horizontal: false,
        tileable_vertical: false,
        color_rgb: None,
        scale: 1,
        align_style: AlignStyle::Node
    };
    let contentfeatures_placeholder: ContentFeatures = ContentFeatures {
        version: 13, // https://github.com/minetest/minetest/blob/master/src/nodedef.h#L313
        name: "[[ERROR]]".to_string(),
        groups: vec![("".to_string(), 1)], // [(String, i16), (String, i16)]
        param_type: 0,
        param_type_2: 0,
        drawtype: DrawType::Normal,
        mesh: "".to_string(),
        visual_scale: 1.0,
        unused_six: 6, // unused? idk what does this even do
        // TODO!!!!! What the fuck have i done (it works)
        tiledef: [tiledef_placeholder.clone(), tiledef_placeholder.clone(), tiledef_placeholder.clone(), tiledef_placeholder.clone(), tiledef_placeholder.clone(), tiledef_placeholder.clone()],
        tiledef_overlay: [tiledef_placeholder.clone(), tiledef_placeholder.clone(), tiledef_placeholder.clone(), tiledef_placeholder.clone(), tiledef_placeholder.clone(), tiledef_placeholder.clone()],
        // WHAT THE FUCK THIS ONE ISNT EVEN SPECIFIED IN THE minetest-protocol DOCS [[I HAD TO GUESS TO REPEAT THE ACCURSED THING FROM ABOVE]]
        tiledef_special: [tiledef_placeholder.clone(), tiledef_placeholder.clone(), tiledef_placeholder.clone(), tiledef_placeholder.clone(), tiledef_placeholder.clone(), tiledef_placeholder.clone()].to_vec(),
        // this code haunts me
        // WHY THE FUCK IS  THIS NEEDED WHAT AM I DOING WRONG WHYYYYY ARE YOU DOING THIS TO MEEE
        alpha_for_legacy: 20,
        red: 100,
        green: 70,
        blue: 85,
        palette_name: "".to_string(),
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
        liquid_alternative_flowing: "".to_string(),
        liquid_alternative_source: "".to_string(),
        liquid_viscosity: 0,
        liquid_renewable: false,
        liquid_range: 0,
        drowning: 0,
        floodable: false,
        node_box: NodeBox::Regular,
        selection_box: NodeBox::Regular,
        collision_box: NodeBox::Regular,
        sound_footstep: SimpleSoundSpec {
            name: "[[ERROR]]".to_string(),
            gain: 1.0,
            pitch: 1.0,
            fade: 1.0,
        },
        sound_dig: SimpleSoundSpec {
            name: "[[ERROR]]".to_string(),
            gain: 1.0,
            pitch: 1.0,
            fade: 1.0,
        },
        sound_dug: SimpleSoundSpec {
            name: "[[ERROR]]".to_string(),
            gain: 1.0,
            pitch: 1.0,
            fade: 1.0,
        },
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

pub fn get_empty_media_command() -> ToClientCommand {
    todo!()
}
