use crate::s2c;
use crate::state;

use log::*;
use luanti_protocol::LuantiConnection;
use luanti_protocol::commands::server_to_client;
use luanti_protocol::commands::server_to_client::ToClientCommand;
use luanti_protocol::types::ItemStackMetadata;
use luanti_protocol::types::{InventoryEntry, InventoryList, ItemStack, ItemStackUpdate};

use azalea::registry::MenuKind;
use azalea_client::{Client, inventory};

use azalea::protocol::packets::game::c_open_screen::ClientboundOpenScreen;

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
            title
        ),
        MenuKind::Generic9x6 => format!(
            "size[9,6]\
label[0,0;{}]\
list[current_player;main;0,0;9,6;]",
            title
        ),
        MenuKind::Generic3x3 => format!(
            "size[3,3]\
label[0,0;{}]\
list[current_player;main;0,0;3,3;]",
            title
        ),
        MenuKind::Crafter3x3 => format!(
            "size[4.5,3]\
label[0,0;{}]\
list[current_player;main;0,0;3,3;]\
list[current_player;main;3.5,1;1,1;]",
            title
        ),
        MenuKind::BlastFurnace => format!(
            "size[3,2]label[0,0;{}]list[current_player;main;0,0;1,2;]list[current_player;main;2,0.5;1,1;]",
            title
        ),
        MenuKind::Furnace => format!(
            "size[3,2]label[0,0;{}]list[current_player;main;0,0;1,2;]list[current_player;main;2,0.5;1,1;]",
            title
        ),
        MenuKind::Smoker => format!(
            "size[3,2]label[0,0;{}]list[current_player;main;0,0;1,2;]list[current_player;main;2,0.5;1,1;]",
            title
        ),
        MenuKind::Crafting => format!(
            "formspec_version[7]\
size[11.5,11]\
background[0,0;17.5,17.5;gui-container-crafting_table.png]\
style_type[list;spacing=0.135,0.135;size=1.09,1.09;border=false]\
listcolors[#0000;#0002]\
list[current_player;container;2.05,1.17;3,3]\
list[current_player;container;8.45,2.4;1,1;9]\
list[current_player;main;0.55,9.7;9,1]\
list[current_player;main;0.55,5.75;9,3;9]\
label[0.55,0.5;{}]\
",
            title
        ),
        _ => format!(
            "size[5,1]label[0,0;Error!\nAs-of-now unsupported MenuKind,\nUI cannot be shown!\nMenu Title: {}]",
            title
        ),
    }
}

pub async fn update_inventory(
    conn: &mut LuantiConnection,
    to_change: Vec<(&str, Vec<inventory::ItemStack>)>,
) {
    let mut entries: Vec<InventoryEntry> = vec![];
    let mut changed_fields: Vec<&str> = vec![];
    for field in to_change {
        changed_fields.push(field.0);
        let mut field_items: Vec<ItemStackUpdate> = vec![];
        for item in field.1 {
            match item {
                inventory::ItemStack::Present(ref slot_data) => {
                    field_items.push(ItemStackUpdate::Item(ItemStack {
                        name: slot_data.kind.to_string(),
                        count: slot_data.count as u16,
                        wear: 0,
                        metadata: ItemStackMetadata {
                            string_vars: vec![],
                        },
                    }));
                }
                inventory::ItemStack::Empty => field_items.push(ItemStackUpdate::Empty),
            }
        }
        entries.push(InventoryEntry::Update {
            0: InventoryList {
                name: String::from(field.0),
                width: 0, // idk what this does
                items: field_items,
            },
        });
    }
    // send keep to unchanged fields (not doing that deletes the associated UI element)
    let unchanged_fields: Vec<&str> = s2c::defs::ALL_INV_FIELDS
        .into_iter()
        .filter(|item| !changed_fields.contains(item))
        .collect();
    for field in unchanged_fields {
        entries.push(InventoryEntry::KeepList(String::from(field)))
    }
    let update_inventory_packet =
        ToClientCommand::Inventory(Box::new(server_to_client::InventorySpec {
            inventory: luanti_protocol::types::Inventory { entries },
        }));
    conn.send(update_inventory_packet).unwrap();
}

pub async fn open_screen(
    packet_data: &ClientboundOpenScreen,
    conn: &mut LuantiConnection,
    inventory_state: &mut state::InventoryState,
) {
    let ClientboundOpenScreen {
        container_id,
        menu_type,
        title,
    } = packet_data;
    inventory_state.container_id = Some(*container_id);
    let form_spec = get_container_formspec(menu_type, &title.to_string());
    debug!("Sending S2C ShowFormspec for opened container");
    let formspec_command =
        ToClientCommand::ShowFormspec(Box::new(server_to_client::ShowFormspecSpec {
            form_spec,
            form_name: String::from("current-container-form"),
        }));
    conn.send(formspec_command).unwrap();
}

pub async fn refresh_inv(
    mc_client: &Client,
    luanti_conn: &mut LuantiConnection,
    inventory_state: &mut state::InventoryState,
) {
    let mut to_update: Vec<(&str, Vec<inventory::ItemStack>)> = vec![];
    match mc_client.menu() {
        inventory::Menu::Player(serverside_inventory) => {
            // fields of the inventory needing a update
            if serverside_inventory.craft_result
                != inventory_state.mt_clientside_player_inv.craft_result
            {
                to_update.push((
                    "craftpreview",
                    vec![serverside_inventory.craft_result.clone()],
                ));
            }
            if serverside_inventory.craft.as_slice()
                != inventory_state.mt_clientside_player_inv.craft.as_slice()
            {
                to_update.push(("craft", serverside_inventory.craft.to_vec()))
            }
            if serverside_inventory.armor.as_slice()
                != inventory_state.mt_clientside_player_inv.armor.as_slice()
            {
                to_update.push(("armor", serverside_inventory.armor.to_vec()))
            }
            if serverside_inventory.inventory.as_slice()
                != inventory_state
                    .mt_clientside_player_inv
                    .inventory
                    .as_slice()
            {
                to_update.push(("main", serverside_inventory.inventory.to_vec()));
            }
            if serverside_inventory.offhand != inventory_state.mt_clientside_player_inv.offhand {
                to_update.push(("offhand", vec![serverside_inventory.offhand.clone()]))
            }
            inventory_state.mt_clientside_player_inv = serverside_inventory;
        }
        // contents: SlotList<n>
        // different n per menu type, so incompatible types
        // my apologies to anyone having to read this
        inventory::Menu::Generic9x1 { contents, player } => {
            to_update.push(("container", contents.to_vec()));
            to_update.push(("main", player.to_vec()))
        }
        inventory::Menu::Generic9x2 { contents, player } => {
            to_update.push(("container", contents.to_vec()));
            to_update.push(("main", player.to_vec()))
        }
        inventory::Menu::Generic9x3 { contents, player } => {
            to_update.push(("container", contents.to_vec()));
            to_update.push(("main", player.to_vec()))
        }
        inventory::Menu::Generic9x4 { contents, player } => {
            to_update.push(("container", contents.to_vec()));
            to_update.push(("main", player.to_vec()))
        }
        inventory::Menu::Generic9x5 { contents, player } => {
            to_update.push(("container", contents.to_vec()));
            to_update.push(("main", player.to_vec()))
        }
        inventory::Menu::Generic9x6 { contents, player } => {
            to_update.push(("container", contents.to_vec()));
            to_update.push(("main", player.to_vec()))
        }
        inventory::Menu::Generic3x3 { contents, player } => {
            to_update.push(("container", contents.to_vec()));
            to_update.push(("main", player.to_vec()))
        }
        inventory::Menu::Crafter3x3 { contents, player } => {
            to_update.push(("container", contents.to_vec()));
            to_update.push(("main", player.to_vec()))
        }
        inventory::Menu::Anvil {
            first,
            second,
            result,
            player,
        } => {
            to_update.push(("container", vec![first, second, result]));
            to_update.push(("main", player.to_vec()))
        }
        inventory::Menu::Beacon { payment, player } => {
            to_update.push(("container", vec![payment]));
            to_update.push(("main", player.to_vec()))
        }
        inventory::Menu::BlastFurnace {
            ingredient,
            fuel,
            result,
            player,
        } => {
            to_update.push(("container", vec![ingredient, fuel, result]));
            to_update.push(("main", player.to_vec()))
        }
        inventory::Menu::BrewingStand {
            bottles,
            ingredient,
            fuel,
            player,
        } => {
            let item_vec = [bottles.to_vec(), vec![ingredient, fuel]].concat();
            to_update.push(("container", item_vec));
            to_update.push(("main", player.to_vec()))
        }
        inventory::Menu::Crafting {
            result,
            grid,
            player,
        } => {
            let item_vec = [grid.to_vec(), vec![result]].concat();
            to_update.push(("container", item_vec));
            to_update.push(("main", player.to_vec()))
        }
        inventory::Menu::Enchantment {
            item,
            lapis,
            player,
        } => {
            to_update.push(("container", vec![item, lapis]));
            to_update.push(("main", player.to_vec()))
        }
        inventory::Menu::Grindstone {
            input,
            additional,
            result,
            player,
        } => {
            to_update.push(("container", vec![input, additional, result]));
            to_update.push(("main", player.to_vec()))
        }
        inventory::Menu::Hopper { contents, player } => {
            to_update.push(("container", contents.to_vec()));
            to_update.push(("main", player.to_vec()))
        }
        inventory::Menu::Lectern { book, player } => {
            to_update.push(("container", vec![book]));
            to_update.push(("main", player.to_vec()))
        }
        inventory::Menu::Loom {
            banner,
            dye,
            pattern,
            result,
            player,
        } => {
            to_update.push(("container", vec![banner, dye, pattern, result]));
            to_update.push(("main", player.to_vec()))
        }
        inventory::Menu::Merchant {
            payments,
            result,
            player,
        } => {
            let item_vec = [payments.to_vec(), vec![result]].concat();
            to_update.push(("container", item_vec));
            to_update.push(("main", player.to_vec()))
        }
        inventory::Menu::ShulkerBox { contents, player } => {
            to_update.push(("container", contents.to_vec()));
            to_update.push(("main", player.to_vec()))
        }
        inventory::Menu::Smithing {
            template,
            base,
            additional,
            result,
            player,
        } => {
            to_update.push(("container", vec![template, base, additional, result]));
            to_update.push(("main", player.to_vec()))
        }
        inventory::Menu::Smoker {
            ingredient,
            fuel,
            result,
            player,
        } => {
            to_update.push(("container", vec![ingredient, fuel, result]));
            to_update.push(("main", player.to_vec()))
        }
        inventory::Menu::CartographyTable {
            map,
            additional,
            result,
            player,
        } => {
            to_update.push(("container", vec![map, additional, result]));
            to_update.push(("main", player.to_vec()))
        }
        inventory::Menu::Stonecutter {
            input,
            result,
            player,
        } => {
            to_update.push(("container", vec![input, result]));
            to_update.push(("main", player.to_vec()))
        }
        inventory::Menu::Furnace {
            ingredient,
            fuel,
            result,
            player,
        } => {
            to_update.push(("container", vec![ingredient, fuel, result]));
            to_update.push(("main", player.to_vec()))
        }
    }
    if !to_update.is_empty() {
        // we need to shift the inventory that is sent to the client
        // because the hotbar for some reason isnt the first (or even last!) row in the sent data
        // if we ever use indexes on "main" that were sent by the minetest client,
        // we first need to fix these: serverside = (clientside - 9) % 36
        for list in to_update.iter_mut() {
            if list.0 == "main" {
                list.1.rotate_right(9);
            }
        }
        update_inventory(luanti_conn, to_update).await;
    }
}
