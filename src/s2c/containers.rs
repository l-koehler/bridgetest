use crate::state;
use crate::utils;

use log::*;
use luanti_protocol::LuantiConnection;
use luanti_protocol::commands::server_to_client;
use luanti_protocol::commands::server_to_client::ToClientCommand;

use std::collections::HashMap;
use std::sync::LazyLock;

use azalea::Client;
use azalea::entity::PlayerAbilities;
use azalea::inventory;
use azalea::local_player::Experience;
use azalea::registry::builtin::{ItemKind, MenuKind, MobEffect};
use azalea::registry::data::Enchantment;
use azalea::registry::{DataRegistry, HolderSet};

use azalea::protocol::common::recipe::{ItemSlotDisplay, ItemStackSlotDisplay, SlotDisplayData};
use azalea::protocol::packets::game::c_container_close::ClientboundContainerClose;
use azalea::protocol::packets::game::c_container_set_data::ClientboundContainerSetData;
use azalea::protocol::packets::game::c_merchant_offers::{
    ClientboundMerchantOffers, MerchantOffer,
};
use azalea::protocol::packets::game::c_open_screen::ClientboundOpenScreen;
use azalea::protocol::packets::game::c_update_recipes::{
    ClientboundUpdateRecipes, SingleInputEntry,
};

pub const CONTAINER_FORM_NAME: &str = "current-container-form";

// sending an empty form_spec closes whatever is currently shown under that name
fn send_formspec(conn: &mut LuantiConnection, form_spec: String) {
    let formspec_command =
        ToClientCommand::ShowFormspec(Box::new(server_to_client::ShowFormspecSpec {
            form_spec,
            form_name: String::from(CONTAINER_FORM_NAME),
        }));
    conn.send(formspec_command).unwrap();
}

// guards against racing container packets
fn open_container(
    container: &mut Option<state::ContainerState>,
    id: i32,
) -> Option<&mut state::ContainerState> {
    container.as_mut().filter(|open| open.id == id)
}

// rebuild the open formspec from the current state and resend it,
// needed as formspecs can need to change without slot changes.
// stonecutter recipes are not container-scoped, so they come in separately
pub fn reshow_formspec(
    mc_client: &Client,
    conn: &mut LuantiConnection,
    container: &state::ContainerState,
    recipes: &[SingleInputEntry],
) {
    send_formspec(conn, get_container_formspec(mc_client, container, recipes));
}

// formspec units per texture pixel, gui textures are stretched so
// minecrafts 18px slot pitch lines up with the 1.09+0.135 list styling below
const PX: f32 = 17.5 / 256.0;

fn px(v: f32) -> f32 {
    v * PX
}

// frame for container GUIs
#[derive(Clone, Copy)]
struct Gui {
    texture: &'static str, // without ".png"
    tex_w: f32,
    gui_w: f32,
    gui_h: f32,
    inv_x: f32,
}

// common case: background texture is 256px wide, GUI region is 176x166 px,
// player inventory at x=8. others override
fn gui(texture: &'static str) -> Gui {
    Gui {
        texture,
        tex_w: 256.0,
        gui_w: 176.0,
        gui_h: 166.0,
        inv_x: 8.0,
    }
}

impl Gui {
    // form sized to the gui region, texture stretched behind it, chest-like slot styles, title label
    fn prelude(&self, title: &str) -> String {
        format!(
            "formspec_version[11]\
size[{w:.2},{h:.2}]\
background[0,0;{bg_w:.2},{bg_h:.2};gui-container-{texture}.png]\
style_type[list;spacing=0.135,0.135;size=1.09,1.09;border=false]\
listcolors[#0000;#0002]\
label[0.55,0.5;{title}]\
",
            texture = self.texture,
            w = px(self.gui_w),
            h = px(self.gui_h),
            bg_w = px(self.tex_w),
            bg_h = px(256.0),
        )
    }

    // vanilla puts the player inventory rows at gui_h-82px and the hotbar at gui_h-24px
    fn player_inventory(&self) -> String {
        format!(
            "list[current_player;main;{x:.2},{main_y:.2};9,3;9]\
list[current_player;main;{x:.2},{hotbar_y:.2};9,1]\
",
            x = px(self.inv_x),
            main_y = px(self.gui_h - 82.0),
            hotbar_y = px(self.gui_h - 24.0),
        )
    }
}

// a single container slot, with given index for the "container" list built by refresh_inv
fn slot(x: f32, y: f32, index: usize) -> String {
    // index 0 is the default and stays implicit
    let index = if index == 0 {
        String::new()
    } else {
        format!(";{index}")
    };
    format!(
        "list[current_player;container;{x:.2},{y:.2};1,1{index}]",
        x = px(x),
        y = px(y),
    )
}

// frame pos/size are in texture pixels (like above), content_h in formspec units to match rows
fn scroll_area(x: f32, y: f32, w: f32, h: f32, content_h: f32, name: &str) -> String {
    // scrollbar steps are 0.1 formspec units each
    let max = ((content_h - px(h)).max(0.0) * 10.0).ceil();
    format!(
        "scrollbaroptions[min=0;max={max}]\
scrollbar[{bar_x:.2},{y:.2};0.3,{h:.2};vertical;{name};0]\
scroll_container[{x:.2},{y:.2};{w:.2},{h:.2};{name};vertical]\
",
        bar_x = px(x) + px(w) + 0.05,
        x = px(x),
        y = px(y),
        w = px(w),
        h = px(h),
    )
}

// helper for buttons and styles, rect is in formspec units
// an item makes this return a image_item_button[]
fn button(
    name: &str,
    rect: (f32, f32, f32, f32),
    sprite: &str,
    hovered: Option<&str>,
    nine_slice: Option<u8>,
    item: Option<&str>,
) -> String {
    let (x, y, w, h) = rect;
    let mut out = format!("style[{name};border=false;bgimg=gui-sprites-{sprite}.png");
    if let Some(border) = nine_slice {
        out.push_str(&format!(";bgimg_middle={border}"));
    }
    out.push(']');
    if let Some(hovered) = hovered {
        out.push_str(&format!(
            "style[{name}:hovered;bgimg=gui-sprites-{hovered}.png]"
        ));
    }
    out.push_str(&match item {
        Some(item) => {
            format!("item_image_button[{x:.2},{y:.2};{w:.2},{h:.2};{item};{name};]")
        }
        None => format!("button[{x:.2},{y:.2};{w:.2},{h:.2};{name};]"),
    });
    out
}

// trade rows are ordinary buttons, as opposed to eg. enchanting table
const TRADE_ROW: (f32, f32) = (89.0, 20.0);
const TRADE_ROWS_SHOWN: f32 = 7.0;

// itemstring for item_image[]/item_image_button[], like "minecraft:emerald 5"
fn item_string(kind: ItemKind, count: i32) -> String {
    format!("{kind} {count}")
}

fn item_image_string(item: &inventory::ItemStack) -> String {
    match item {
        inventory::ItemStack::Present(data) => item_string(data.kind, data.count),
        inventory::ItemStack::Empty => String::new(),
    }
}

// scrollable list of trade offers on the left, payment/result slots on the right
fn get_merchant_formspec(title: &str, offers: &[MerchantOffer]) -> String {
    let (row_w, row_h) = TRADE_ROW;
    let list_h = row_h * TRADE_ROWS_SHOWN;
    let content_h = (offers.len() as f32 * px(row_h)).max(px(list_h));

    let gui = Gui {
        tex_w: 512.0,
        gui_w: 276.0,
        inv_x: 108.0,
        ..gui("villager")
    };
    let mut form_spec = gui.prelude(title);
    form_spec.push_str(&scroll_area(5.0, 18.0, row_w, list_h, content_h, "trades"));

    for (i, offer) in offers.iter().enumerate() {
        let row_y = i as f32 * px(row_h);
        // the row background is the button, so items and arrow need to draw over it
        form_spec.push_str(&button(
            &format!("trade_{i}"),
            (0.0, row_y, px(row_w), px(row_h)),
            "widget-button",
            Some("widget-button_highlighted"),
            Some(3),
            None,
        ));
        let y = row_y + (px(row_h) - 0.8) / 2.0;
        // first input item
        let cost_a = item_string(offer.base_cost_a.item, offer.base_cost_a.count);
        form_spec.push_str(&format!("item_image[0,{y:.2};0.8,0.8;{cost_a}]"));

        // first input cost adjustment
        let count = offer.base_cost_a.count;
        let demand_bonus = (offer.price_multiplier * count as f32 * offer.demand as f32)
            .floor()
            .max(0.0) as i32;
        let cost_a_final = (count + demand_bonus + offer.special_price_diff).clamp(1, 64);
        if (cost_a_final != count) {
            // strikethrough and new price
            form_spec.push_str(&format!("image[0.5,{:.2};0.35,0.06;gui-sprites-container-villager-discount_strikethrough.png]", y+0.6));
            form_spec.push_str(&format!("label[1.05,{:.2};{}]", y + 0.63, cost_a_final));
        }
        // second input
        if let Some(cost_b) = &offer.cost_b {
            let cost_b = item_string(cost_b.item, cost_b.count);
            form_spec.push_str(&format!("item_image[2,{y:.2};0.8,0.8;{cost_b}]"));
        }
        let result = item_image_string(&offer.result);
        form_spec.push_str(&format!("item_image[4.2,{y:.2};0.8,0.8;{result}]"));

        let texturename = if offer.out_of_stock {
            "trade_arrow_out_of_stock"
        } else {
            "trade_arrow"
        };
        form_spec.push_str(&format!(
            "image[3.2,{y:.2};0.75,0.75;gui-sprites-container-villager-{texturename}.png]"
        ));
    }
    form_spec.push_str("scroll_container_end[]");

    // the two payment slots and the result
    form_spec.push_str(&slot(136.0, 37.0, 0));
    form_spec.push_str(&slot(162.0, 37.0, 1));
    form_spec.push_str(&slot(220.0, 37.0, 2));
    form_spec.push_str(&gui.player_inventory());

    form_spec
}

// chest-alikes all share one gui texture and differ only in row count
fn get_chest_formspec(title: &str, rows: u32) -> String {
    let gui = Gui {
        gui_h: 222.0,
        ..gui("generic_54")
    };
    get_grid_formspec(title, gui, (8.0, 18.0), 9, rows)
}

// formspec for grid of container slots + player inventory.
fn get_grid_formspec(
    title: &str,
    gui: Gui,
    grid_px: (f32, f32),
    width: u32,
    height: u32,
) -> String {
    format!(
        "{prelude}\
list[current_player;container;{x:.2},{y:.2};{width},{height}]\
{inv}",
        prelude = gui.prelude(title),
        x = px(grid_px.0),
        y = px(grid_px.1),
        inv = gui.player_inventory(),
    )
}

// helper for non-grid container slots
// slots[i] = index i of the "container" list from refresh_inv
fn get_slots_formspec(title: &str, gui: Gui, slots: &[(f32, f32)]) -> String {
    let mut form_spec = gui.prelude(title);
    for (i, (x, y)) in slots.iter().enumerate() {
        form_spec.push_str(&slot(*x, *y, i));
    }
    form_spec.push_str(&gui.player_inventory());
    form_spec
}

// property ids of the progress values the server sends via ContainerSetData, see
// https://minecraft.wiki/w/Java_Edition_protocol/Packets#Set_Container_Property
const FURNACE_LIT_TIME: u16 = 0;
const FURNACE_LIT_DURATION: u16 = 1;
const FURNACE_COOK_TIME: u16 = 2;
const FURNACE_COOK_TOTAL: u16 = 3;
// brewTime counts down from 400, fuelTime down from 20
const BREW_TIME: u16 = 0;
const BREW_TIME_TOTAL: f32 = 400.0;
const BREW_FUEL_TIME: u16 = 1;
const BREW_FUEL_TOTAL: f32 = 20.0;
// one per enchantment table row: required level and enchantment hint (ID, level)
const ENCHANT_LEVEL_COST: u16 = 0;
const ENCHANT_HINT_ID: u16 = 4;
const ENCHANT_HINT_LEVEL: u16 = 7;

enum FillDirection {
    Right,
    Down,
    Up,
}

struct ProgressSprite {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    fill: FillDirection,
}

// blitted over the empty outlines in the gui textures
const FURNACE_FLAME: ProgressSprite = ProgressSprite {
    x: 57.0,
    y: 37.5,
    w: 14.0,
    h: 14.0,
    fill: FillDirection::Up,
};
const FURNACE_ARROW: ProgressSprite = ProgressSprite {
    x: 79.0,
    y: 35.0,
    w: 24.0,
    h: 16.0,
    fill: FillDirection::Right,
};
const BREW_ARROW: ProgressSprite = ProgressSprite {
    x: 97.0,
    y: 16.0,
    w: 9.0,
    h: 28.0,
    fill: FillDirection::Down,
};
const BREW_FUEL_BAR: ProgressSprite = ProgressSprite {
    x: 60.0,
    y: 44.0,
    w: 18.0,
    h: 4.0,
    fill: FillDirection::Right,
};

impl ProgressSprite {
    // length of drawn bar part along draw axis, in px
    fn filled_pixels(&self, pct: f32) -> u16 {
        let axis = match self.fill {
            FillDirection::Right => self.w,
            FillDirection::Down | FillDirection::Up => self.h,
        };
        (axis * pct).ceil() as u16
    }

    // crop sprite to filled part
    // luanti has no crop modifier, [combine clips to canvas
    fn draw(&self, sprite: &str, pct: f32) -> String {
        let filled = self.filled_pixels(pct) as f32;
        if filled < 1.0 {
            return String::new();
        }
        let (w, h) = match self.fill {
            FillDirection::Right => (filled, self.h),
            FillDirection::Down | FillDirection::Up => (self.w, filled),
        };
        let (y, offset) = match self.fill {
            FillDirection::Up => (self.y + self.h - h, h - self.h),
            _ => (self.y, 0.0),
        };
        format!(
            "image[{x:.2},{y:.2};{img_w:.2},{img_h:.2};[combine:{w}x{h}:0,{offset}={sprite}]",
            x = px(self.x),
            y = px(y),
            img_w = px(w),
            img_h = px(h),
            w = w as i32,
            h = h as i32,
            offset = offset as i32,
        )
    }
}

fn property(properties: &HashMap<u16, u16>, id: u16) -> u16 {
    *properties.get(&id).unwrap_or(&0)
}

// (fuel, cook) of furnace bars, 0..=1
fn furnace_fraction(properties: &HashMap<u16, u16>) -> (f32, f32) {
    let fraction = |elapsed: u16, total: u16| match property(properties, total) {
        0 => 0.0,
        total => (property(properties, elapsed) as f32 / total as f32).clamp(0.0, 1.0),
    };
    (
        fraction(FURNACE_LIT_TIME, FURNACE_LIT_DURATION),
        fraction(FURNACE_COOK_TIME, FURNACE_COOK_TOTAL),
    )
}

// (brew, fuel) of brewing stand bars, 0..=1
fn brewing_fraction(properties: &HashMap<u16, u16>) -> (f32, f32) {
    let brew_time = property(properties, BREW_TIME);
    let fuel_time = property(properties, BREW_FUEL_TIME) as f32;
    // 0 = nothing brewing
    let brew_pct = if brew_time == 0 {
        0.0
    } else {
        (1.0 - (brew_time as f32 / BREW_TIME_TOTAL)).clamp(0.0, 1.0)
    };
    (brew_pct, (fuel_time / BREW_FUEL_TOTAL).clamp(0.0, 1.0))
}

// ingredient/fuel/result slots, cook+fuel progress sprites
fn get_furnace_formspec(title: &str, kind: MenuKind, properties: &HashMap<u16, u16>) -> String {
    let name = match kind {
        MenuKind::Smoker => "smoker",
        MenuKind::BlastFurnace => "blast_furnace",
        _ => "furnace",
    };
    let (fuel_pct, cook_pct) = furnace_fraction(properties);

    let gui = gui(name);
    let mut form_spec = gui.prelude(title);
    form_spec.push_str(&slot(56.0, 17.0, 0)); // ingredient
    form_spec.push_str(&slot(56.0, 53.0, 1)); // fuel
    form_spec.push_str(&slot(116.0, 35.0, 2)); // result
    form_spec.push_str(&FURNACE_FLAME.draw(
        &format!("gui-sprites-container-{name}-lit_progress.png"),
        fuel_pct,
    ));
    form_spec.push_str(&FURNACE_ARROW.draw(
        &format!("gui-sprites-container-{name}-burn_progress.png"),
        cook_pct,
    ));
    form_spec.push_str(&gui.player_inventory());
    form_spec
}

// 3 bottle slots + ingredient/fuel slots, brew+fuel progress sprites
fn get_brewing_stand_formspec(title: &str, properties: &HashMap<u16, u16>) -> String {
    let (brew_pct, fuel_pct) = brewing_fraction(properties);

    let gui = gui("brewing_stand");
    let mut form_spec = gui.prelude(title);
    form_spec.push_str(&slot(56.0, 51.0, 0)); // bottles
    form_spec.push_str(&slot(79.0, 58.0, 1));
    form_spec.push_str(&slot(102.0, 51.0, 2));
    form_spec.push_str(&slot(79.0, 17.0, 3)); // ingredient
    form_spec.push_str(&slot(17.0, 17.0, 4)); // fuel
    form_spec.push_str(&BREW_ARROW.draw(
        "gui-sprites-container-brewing_stand-brew_progress.png",
        brew_pct,
    ));
    form_spec.push_str(&BREW_FUEL_BAR.draw(
        "gui-sprites-container-brewing_stand-fuel_length.png",
        fuel_pct,
    ));
    form_spec.push_str(&gui.player_inventory());
    form_spec
}

// enchantment hints shown on each of the three option rows, if known
fn enchantment_hints(mc_client: &Client, properties: &HashMap<u16, u16>) -> [Option<String>; 3] {
    std::array::from_fn(|i| {
        let id = *properties.get(&(ENCHANT_HINT_ID + i as u16))?;
        if id == u16::MAX {
            return None;
        }
        let level = property(properties, ENCHANT_HINT_LEVEL + i as u16);
        let name = mc_client
            .resolve_registry_key(&Enchantment::new_raw(id as u32))
            .ok()
            .flatten()
            .map(|key| format!("{key:?}"))
            .unwrap_or_else(|| format!("Enchantment #{id}"));
        // we only get one hint, more enchantments may exist
        Some(format!("{name} {} ...?", utils::roman_numeral(level)))
    })
}

const ENCHANT_ROW_ORIGIN: (f32, f32) = (60.0, 14.0);
const ENCHANT_ROW_SIZE: (f32, f32) = (108.0, 19.0);
const ENCHANT_TEXT_X: f32 = 80.0;
const ENCHANT_TEXT_W: f32 = 86.0;

// grey out optionjs the player cant afford
pub fn affordable_enchantments(mc_client: &Client, properties: &HashMap<u16, u16>) -> [bool; 3] {
    let creative = mc_client
        .component::<PlayerAbilities>()
        .map(|abilities| abilities.instant_break)
        .unwrap_or(false);
    if creative {
        return [true; 3];
    }
    let level = mc_client
        .component::<Experience>()
        .map(|experience| experience.level)
        .unwrap_or(0);
    let lapis = match mc_client.menu() {
        Ok(inventory::Menu::Enchantment { lapis, .. }) => {
            lapis.as_present().map(|data| data.count).unwrap_or(0)
        }
        _ => 0,
    };
    trace!("Enchantment affordability with {level} levels and {lapis} lapis");
    std::array::from_fn(|i| {
        let required = property(properties, ENCHANT_LEVEL_COST + i as u16);
        lapis > i as i32 && level >= required as u32
    })
}

// item/lapis slots, 3 enchant options
// https://minecraft.wiki/w/Java_Edition_protocol/Packets#Set_Container_Property
fn get_enchantment_formspec(
    mc_client: &Client,
    title: &str,
    properties: &HashMap<u16, u16>,
) -> String {
    let hints = enchantment_hints(mc_client, properties);
    let affordable = affordable_enchantments(mc_client, properties);
    let (row_x, row_y) = ENCHANT_ROW_ORIGIN;
    let (row_w, row_h) = ENCHANT_ROW_SIZE;

    let gui = gui("enchanting_table");
    let mut form_spec = gui.prelude(title);
    form_spec.push_str(&slot(15.0, 47.0, 0)); // item
    form_spec.push_str(&slot(35.0, 47.0, 1)); // lapis
    form_spec.push_str("style_type[label;valign=center]");
    for i in 0..3usize {
        let required = property(properties, ENCHANT_LEVEL_COST + i as u16);
        let y = row_y + row_h * i as f32;
        // level cost of zero -> the option is not offered
        let offered = required > 0;
        let enabled = offered && affordable[i];
        let plate = if enabled {
            "enchantment_slot"
        } else {
            "enchantment_slot_disabled"
        };
        // button in background/at start of spec, as its bg-image is the plate
        form_spec.push_str(&button(
            &format!("enchant_{i}"),
            (px(row_x), px(y), px(row_w), px(row_h)),
            &format!("container-enchanting_table-{plate}"),
            enabled.then_some("container-enchanting_table-enchantment_slot_highlighted"),
            None,
            None,
        ));
        if offered {
            let dim = if enabled { "" } else { "_disabled" };
            form_spec.push_str(&format!(
                "image[{x:.2},{y:.2};{s:.2},{s:.2};gui-sprites-container-enchanting_table-level_{n}{dim}.png]",
                x = px(row_x + 1.0),
                y = px(y + 1.0),
                s = px(16.0),
                n = i + 1,
            ));
            let label = match &hints[i] {
                Some(hint) => format!("{hint} ({required} lvls)"),
                None => format!("Level {required}"),
            };
            // area label, so long hints wrap inside the plate instead of past it
            form_spec.push_str(&format!(
                "label[{x:.2},{y:.2};{w:.2},{h:.2};{label}]",
                x = px(ENCHANT_TEXT_X),
                y = px(y),
                w = px(ENCHANT_TEXT_W),
                h = px(row_h),
            ));
        }
    }
    form_spec.push_str(&gui.player_inventory());
    form_spec
}

// SlotDisplayData -> item_image[]-compatible string "item count" or blank
fn slot_display_to_itemstring(display: &SlotDisplayData) -> String {
    match display {
        SlotDisplayData::Item(ItemSlotDisplay { item }) => item_string(*item, 1),
        SlotDisplayData::ItemStack(ItemStackSlotDisplay { stack }) => {
            item_string(stack.kind, stack.count)
        }
        _ => String::new(),
    }
}

// order relevant, server receives indices into this list!
fn matching_stonecutter_recipes(
    recipes: &[SingleInputEntry],
    input: Option<ItemKind>,
) -> Vec<&SingleInputEntry> {
    let Some(input) = input else {
        trace!("Stonecutter has no input item, showing no recipes");
        return Vec::new();
    };
    let matching: Vec<&SingleInputEntry> = recipes
        .iter()
        .filter(|entry| match &entry.input.allowed {
            HolderSet::Direct { contents } => contents.contains(&input),
            HolderSet::Named { key, .. } => {
                debug!("Can't match stonecutter ingredient tag {key:?}, hiding its recipe");
                false
            }
        })
        .collect();
    debug!(
        "Stonecutter input {input:?}: {} of {} known recipes match",
        matching.len(),
        recipes.len()
    );
    matching
}

// vanilla lays the recipes out in 16x18 cells, 4 to a row
const STONECUTTER_COLS: usize = 4;
const STONECUTTER_CELL: (f32, f32) = (16.0, 18.0);

// input/result slots + a scrollable grid of selectable recipes
fn get_stonecutter_formspec(title: &str, recipes: &[&SingleInputEntry]) -> String {
    let (cell_w, cell_h) = STONECUTTER_CELL;
    let row_count = recipes.len().div_ceil(STONECUTTER_COLS);
    let list_h = 54.0;
    let content_h = (row_count as f32 * px(cell_h)).max(px(list_h));

    let gui = gui("stonecutter");
    let mut form_spec = gui.prelude(title);
    form_spec.push_str(&slot(20.0, 33.0, 0)); // input
    form_spec.push_str(&slot(143.0, 33.0, 1)); // result

    if !recipes.is_empty() {
        form_spec.push_str(&scroll_area(52.0, 15.0, 64.0, list_h, content_h, "recipes"));
        for (i, entry) in recipes.iter().enumerate() {
            let icon = slot_display_to_itemstring(&entry.recipe.option_display);
            //TODO also use recipe_selected, but needs proxy to track
            form_spec.push_str(&button(
                &format!("stonecut_{i}"),
                (
                    (i % STONECUTTER_COLS) as f32 * px(cell_w),
                    (i / STONECUTTER_COLS) as f32 * px(cell_h),
                    px(cell_w),
                    px(cell_h),
                ),
                "container-stonecutter-recipe",
                Some("container-stonecutter-recipe_highlighted"),
                None,
                Some(&icon),
            ));
        }
        form_spec.push_str("scroll_container_end[]");
    }

    form_spec.push_str(&gui.player_inventory());
    form_spec
}

// the banner pattern list is defined by the client (then indices into that list get sent to the server :)
// compare to stonecutter, which gets a packet
#[derive(serde::Deserialize)]
struct BannerPatternData {
    no_item_required: Vec<String>,
    pattern_items: HashMap<String, Vec<String>>,
}

static BANNER_PATTERNS: LazyLock<BannerPatternData> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../extra_data/banner_patterns.json"))
        .expect("extra_data/banner_patterns.json is invalid")
});

fn selectable_loom_patterns(
    banner: &inventory::ItemStack,
    dye: &inventory::ItemStack,
    pattern: &inventory::ItemStack,
) -> Vec<String> {
    if banner.is_empty() || dye.is_empty() {
        return Vec::new();
    }
    match pattern {
        inventory::ItemStack::Empty => BANNER_PATTERNS.no_item_required.clone(),
        // an unknown pattern item provides nothing rather than everything
        inventory::ItemStack::Present(data) => BANNER_PATTERNS
            .pattern_items
            .get(&data.kind.to_string())
            .cloned()
            .unwrap_or_default(),
    }
}

//FIXME the vanilla UI assumes a rendered banner preview in each slot
// we can't do that yet, held-item metadata isn't supported
// use buttons with text
const LOOM_COLS: usize = 2;
const LOOM_ROW: (f32, f32) = (2.2, 0.9);
const LOOM_BUTTON: (f32, f32) = (2.1, 0.8);

// banner/dye/pattern/result slots + list of selectable patterns
fn get_loom_formspec(title: &str, patterns: &[String]) -> String {
    let (step_x, step_y) = LOOM_ROW;
    let (button_w, button_h) = LOOM_BUTTON;
    let row_count = patterns.len().div_ceil(LOOM_COLS);
    let list_h = 56.0;
    let content_h = (row_count as f32 * step_y).max(px(list_h));

    let gui = gui("loom");
    let mut form_spec = gui.prelude(title);
    form_spec.push_str(&slot(13.0, 26.0, 0)); // banner
    form_spec.push_str(&slot(33.0, 26.0, 1)); // dye
    form_spec.push_str(&slot(23.0, 45.0, 2)); // pattern
    form_spec.push_str(&slot(143.0, 58.0, 3)); // result

    if !patterns.is_empty() {
        form_spec.push_str("style_type[label;valign=center]");
        form_spec.push_str(&scroll_area(
            60.0, 13.0, 64.0, list_h, content_h, "patterns",
        ));
        for (i, pattern) in patterns.iter().enumerate() {
            let x = (i % LOOM_COLS) as f32 * step_x;
            let y = (i / LOOM_COLS) as f32 * step_y;
            form_spec.push_str(&button(
                &format!("loom_{i}"),
                (x, y, button_w, button_h),
                "container-loom-pattern_selected",
                None,
                Some(1),
                None,
            ));
            form_spec.push_str(&format!(
                "image[{icon_x:.2},{icon_y:.2};0.7,0.7;gui-sprites-container-loom-pattern.png]",
                icon_x = x + 0.05,
                icon_y = y + 0.05,
            ));
            //TODO id path as label ("square_bottom_left")
            let name = pattern.rsplit(':').next().unwrap_or(pattern);
            form_spec.push_str(&format!(
                "label[{label_x:.2},{y:.2};{label_w:.2},{button_h:.2};{name}]",
                label_x = x + 0.8,
                label_w = button_w - 0.8,
            ));
        }
        form_spec.push_str("scroll_container_end[]");
    }

    form_spec.push_str(&gui.player_inventory());
    form_spec
}

// (button label, effect)
//TODO not gated by beacon power level
// would need to actually read map data
pub const BEACON_EFFECTS: &[(&str, MobEffect)] = &[
    ("Speed", MobEffect::Speed),
    ("Haste", MobEffect::Haste),
    ("Resistance", MobEffect::Resistance),
    ("Jump Boost", MobEffect::JumpBoost),
    ("Strength", MobEffect::Strength),
    ("Regeneration", MobEffect::Regeneration),
];

// vanilla puts two 22x22 effect buttons per row, centred on x=76, from y=22
const BEACON_BUTTON: f32 = 22.0;
const BEACON_ICON: f32 = 18.0;

// payment slot + button per available beacon effect
//TODO doesn't do secondary effects
fn get_beacon_formspec(title: &str) -> String {
    let gui = Gui {
        gui_w: 230.0,
        gui_h: 219.0,
        inv_x: 36.0,
        ..gui("beacon")
    };
    let mut form_spec = gui.prelude(title);

    for (i, (name, effect)) in BEACON_EFFECTS.iter().enumerate() {
        let x = 53.0 + (i % 2) as f32 * 24.0;
        let y = 22.0 + (i / 2) as f32 * 25.0;
        //TODO no button_selected.png/button_disabled.png
        // would need to track beacon power level
        form_spec.push_str(&button(
            &format!("beacon_{i}"),
            (px(x), px(y), px(BEACON_BUTTON), px(BEACON_BUTTON)),
            "container-beacon-button",
            Some("container-beacon-button_highlighted"),
            None,
            None,
        ));
        // "minecraft:jump_boost" -> "mob_effect-jump_boost.png"
        let icon = effect.to_str().rsplit(':').next().unwrap_or_default();
        form_spec.push_str(&format!(
            "image[{icon_x:.2},{icon_y:.2};{icon_s:.2},{icon_s:.2};mob_effect-{icon}.png]\
tooltip[beacon_{i};{name}]",
            icon_x = px(x + 2.0),
            icon_y = px(y + 2.0),
            icon_s = px(BEACON_ICON),
        ));
    }

    form_spec.push_str(&slot(136.0, 110.0, 0)); // payment
    form_spec.push_str(&gui.player_inventory());
    form_spec
}

// 3x3 workbench
fn get_crafting_formspec(title: &str) -> String {
    let gui = gui("crafting_table");
    format!(
        "{prelude}{result}\
list[current_player;container;{grid_x:.2},{grid_y:.2};3,3;1]\
{inv}",
        prelude = gui.prelude(title),
        result = slot(124.0, 35.0, 0),
        grid_x = px(30.0),
        grid_y = px(17.0),
        inv = gui.player_inventory(),
    )
}

// turn an open container into its formspec
// can be used for reshow, slot contents are read from the state
fn get_container_formspec(
    mc_client: &Client,
    container: &state::ContainerState,
    recipes: &[SingleInputEntry],
) -> String {
    let title = container.title.as_str();
    match container.kind {
        MenuKind::Crafting => get_crafting_formspec(title),
        MenuKind::Generic9x3 | MenuKind::ShulkerBox => {
            get_grid_formspec(title, gui("shulker_box"), (8.0, 18.0), 9, 3)
        }
        MenuKind::Generic9x1 => get_chest_formspec(title, 1),
        MenuKind::Generic9x2 => get_chest_formspec(title, 2),
        MenuKind::Generic9x4 => get_chest_formspec(title, 4),
        MenuKind::Generic9x5 => get_chest_formspec(title, 5),
        MenuKind::Generic9x6 => get_chest_formspec(title, 6),
        MenuKind::Generic3x3 => get_grid_formspec(title, gui("dispenser"), (62.0, 17.0), 3, 3),
        MenuKind::Crafter3x3 => get_grid_formspec(title, gui("crafter"), (26.0, 17.0), 3, 3),
        MenuKind::Hopper => get_grid_formspec(
            title,
            Gui {
                gui_h: 133.0,
                ..gui("hopper")
            },
            (44.0, 20.0),
            5,
            1,
        ),
        // slot rows that are not on a grid get placed slot-by-slot
        MenuKind::Anvil => get_slots_formspec(
            title,
            gui("anvil"),
            &[(27.0, 47.0), (76.0, 47.0), (134.0, 47.0)],
        ),
        MenuKind::Grindstone => get_slots_formspec(
            title,
            gui("grindstone"),
            &[(49.0, 19.0), (49.0, 40.0), (129.0, 34.0)],
        ),
        MenuKind::CartographyTable => get_slots_formspec(
            title,
            gui("cartography_table"),
            &[(15.0, 15.0), (15.0, 52.0), (145.0, 39.0)],
        ),
        MenuKind::Smithing => get_slots_formspec(
            title,
            gui("smithing"),
            &[(8.0, 48.0), (26.0, 48.0), (44.0, 48.0), (98.0, 48.0)],
        ),
        // trade rows empty until MerchantOffers arrives
        MenuKind::Merchant => get_merchant_formspec(title, &container.trade_offers),
        // progress bars and enchant levels empty until ContainerSetData arrives
        MenuKind::Furnace | MenuKind::Smoker | MenuKind::BlastFurnace => {
            get_furnace_formspec(title, container.kind, &container.properties)
        }
        MenuKind::BrewingStand => get_brewing_stand_formspec(title, &container.properties),
        MenuKind::Enchantment => get_enchantment_formspec(mc_client, title, &container.properties),
        // stonecutter/loom show only the options valid for the current state
        // the server assumes we use that list for the index, so we have to do that
        MenuKind::Stonecutter => get_stonecutter_formspec(
            title,
            &matching_stonecutter_recipes(recipes, container.stonecutter_input),
        ),
        MenuKind::Loom => get_loom_formspec(title, &container.loom_patterns),
        MenuKind::Beacon => get_beacon_formspec(title),
        _ => format!(
            "size[5,1]label[0,0;Error!\nAs-of-now unsupported MenuKind,\nUI cannot be shown!\nMenu Title: {title}]"
        ),
    }
}

// needs the inventory state, containers displace the 2x2 grid
pub fn open_screen(
    packet_data: &ClientboundOpenScreen,
    mc_client: &Client,
    conn: &mut LuantiConnection,
    inventory_state: &mut state::InventoryState,
    container: &mut Option<state::ContainerState>,
) {
    let ClientboundOpenScreen {
        container_id,
        menu_type,
        title,
    } = packet_data;
    //drop 2x2
    inventory_state.inventory_handle = None;
    // replaces whatever was open before, dropping its state with it
    let container = container.insert(state::ContainerState::new(
        *container_id,
        *menu_type,
        title.to_string(),
    ));
    debug!("Sending S2C ShowFormspec for opened container");
    reshow_formspec(
        mc_client,
        conn,
        container,
        &inventory_state.stonecutter_recipes,
    );
}

// minecraft server can close containers on its own
pub fn server_closed_container(
    packet: &ClientboundContainerClose,
    conn: &mut LuantiConnection,
    container: &mut Option<state::ContainerState>,
) {
    if open_container(container, packet.container_id).is_none() {
        return;
    }
    *container = None;
    debug!("MC server closed our container, dismissing luanti formspec for it");
    send_formspec(conn, String::new());
}

// server (re)sends this whenever offers/stock/prices change
pub fn merchant_offers(
    packet: &ClientboundMerchantOffers,
    mc_client: &Client,
    conn: &mut LuantiConnection,
    container: &mut Option<state::ContainerState>,
    recipes: &[SingleInputEntry],
) {
    let Some(container) = open_container(container, packet.container_id) else {
        return;
    };
    if container.trade_offers == packet.offers {
        return;
    }
    container.trade_offers = packet.offers.clone();
    debug!("Sending S2C ShowFormspec for merchant trade offers");
    reshow_formspec(mc_client, conn, container, recipes);
}

// sent once with full stonecutter list
// yay for special-cases, why do that the same way as looms when you can add A WHOLE PACKET FOR ONE BLOCK
pub fn update_recipes(
    packet: &ClientboundUpdateRecipes,
    inventory_state: &mut state::InventoryState,
) {
    debug!(
        "Got {} stonecutter recipes from ClientboundUpdateRecipes",
        packet.stonecutter_recipes.len()
    );
    inventory_state.stonecutter_recipes = packet.stonecutter_recipes.clone();
}

// dirty check, returns the progress values as the sprite lengths we would draw
// prevents subpixel/invisible changes from redrawing (which can flicker the formspec)
fn progress_bar_pixels(kind: MenuKind, properties: &HashMap<u16, u16>) -> Option<Vec<u16>> {
    match kind {
        MenuKind::Furnace | MenuKind::Smoker | MenuKind::BlastFurnace => {
            let (fuel_pct, cook_pct) = furnace_fraction(properties);
            Some(vec![
                FURNACE_FLAME.filled_pixels(fuel_pct),
                FURNACE_ARROW.filled_pixels(cook_pct),
            ])
        }
        MenuKind::BrewingStand => {
            let (brew_pct, fuel_pct) = brewing_fraction(properties);
            Some(vec![
                BREW_ARROW.filled_pixels(brew_pct),
                BREW_FUEL_BAR.filled_pixels(fuel_pct),
            ])
        }
        // not a percentage bar, but same dirty check applies for enchantment hints
        MenuKind::Enchantment => Some(
            (ENCHANT_LEVEL_COST..=ENCHANT_LEVEL_COST + 2)
                .chain(ENCHANT_HINT_ID..=ENCHANT_HINT_LEVEL + 2)
                .map(|id| property(properties, id))
                .collect(),
        ),
        _ => None,
    }
}

// on container property change (furnace burn/cook time, brewing stand progress etc)
pub fn update_progress_bar(
    packet: &ClientboundContainerSetData,
    mc_client: &Client,
    conn: &mut LuantiConnection,
    container: &mut Option<state::ContainerState>,
    recipes: &[SingleInputEntry],
) {
    // race-able, ensure that we still have an applicable formspec open
    let Some(container) = open_container(container, packet.container_id) else {
        return;
    };
    container.properties.insert(packet.id, packet.value);

    let Some(progress) = progress_bar_pixels(container.kind, &container.properties) else {
        return; // kind has no progress bar
    };
    if progress == container.last_shown_progress {
        return; // no visible change, prevent flicker
    }
    container.last_shown_progress = progress;
    // the level costs we just stored feed into this, keep the tick check in sync
    // so it doesn't send the same formspec a second time
    if container.kind == MenuKind::Enchantment {
        container.last_shown_affordable = affordable_enchantments(mc_client, &container.properties);
    }
    debug!("Sending S2C ShowFormspec for updated container property display");
    reshow_formspec(mc_client, conn, container, recipes);
}

#[derive(Default)]
pub struct ContainerSlots {
    // slot lists (container, player rows) for a non-player menu
    pub lists: Vec<(String, Vec<inventory::ItemStack>)>,
    pub stonecutter_input: Option<ItemKind>,
    pub loom_patterns: Vec<String>,
}

pub fn container_slot_lists(menu: &inventory::Menu) -> Option<ContainerSlots> {
    if matches!(menu, inventory::Menu::Player(_)) {
        return None;
    }
    let player = menu.slots()[menu.player_slots_range()].to_vec();
    let mut slots = ContainerSlots {
        lists: vec![
            ("container".to_string(), menu.contents()),
            ("main".to_string(), player),
        ],
        ..Default::default()
    };
    // menus where formspec content depends on slots
    match menu {
        inventory::Menu::Loom {
            banner,
            dye,
            pattern,
            ..
        } => slots.loom_patterns = selectable_loom_patterns(banner, dye, pattern),
        inventory::Menu::Stonecutter { input, .. } => {
            slots.stonecutter_input = input.as_present().map(|data| data.kind)
        }
        _ => {}
    }
    Some(slots)
}
