use crate::s2c;
use crate::state;

use log::*;
use luanti_protocol::LuantiConnection;
use luanti_protocol::commands::server_to_client;
use luanti_protocol::commands::server_to_client::ToClientCommand;
use luanti_protocol::types::ItemStackMetadata;
use luanti_protocol::types::{InventoryEntry, InventoryList, ItemStack, ItemStackUpdate};

use azalea::Client;
use azalea::entity::inventory::Inventory as InventoryComponent;
use azalea::inventory;
use azalea::registry::builtin::MenuKind;

use azalea::protocol::packets::game::c_container_set_content::ClientboundContainerSetContent;
use azalea::protocol::packets::game::c_set_cursor_item::ClientboundSetCursorItem;

pub async fn update_inventory(
    conn: &mut LuantiConnection,
    to_change: Vec<(String, Vec<inventory::ItemStack>)>,
) {
    let mut entries: Vec<InventoryEntry> = vec![];
    let mut changed_fields: Vec<String> = vec![];
    for field in to_change {
        changed_fields.push(field.0.clone());
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
        .filter(|item| !changed_fields.contains(&item.to_string()))
        .collect();
    for field in unchanged_fields {
        entries.push(InventoryEntry::KeepList(String::from(field)))
    }
    let update_inventory_packet =
        ToClientCommand::Inventory(Box::new(server_to_client::InventorySpec {
            inventory: luanti_protocol::types::Inventory { entries },
            skip_wield_anim: false,
        }));
    conn.send(update_inventory_packet).unwrap();
}

fn with_inventory(mc_client: &Client, f: impl FnOnce(&mut InventoryComponent)) {
    let mut ecs = (*mc_client.ecs).write();
    let mut query = ecs.query::<&mut InventoryComponent>();
    let Ok(mut inventory) = query.get_mut(&mut ecs, mc_client.entity) else {
        return;
    };
    f(&mut inventory);
}

// azalea applies ContainerSetContent slots but drops the state_id
// once the server does a resync all further clicks would look stale to it
pub fn sync_container_state(mc_client: &Client, packet: &ClientboundContainerSetContent) {
    with_inventory(mc_client, |inventory| {
        if packet.container_id == inventory.id {
            inventory.state_id = packet.state_id;
            inventory.carried = packet.carried_item.clone();
        }
    });
}

// azalea drops SetCursorItem, leaving stack stale. similar to above
pub fn sync_cursor_item(mc_client: &Client, packet: &ClientboundSetCursorItem) {
    with_inventory(mc_client, |inventory| {
        inventory.carried = packet.contents.clone()
    });
}

// see https://minecraft.wiki/w/Java_Edition_protocol/Inventory#Crafting
pub async fn refresh_inv(
    mc_client: &Client,
    luanti_conn: &mut LuantiConnection,
    inventory_state: &mut state::InventoryState,
    container: &mut Option<state::ContainerState>,
    force_full: bool,
) {
    let menu = mc_client.menu().unwrap();
    // option lists derive from stonecutter_input/loom_patterns, reshow the formspec when relevant slots change
    let mut slots = s2c::containers::container_slot_lists(&menu).unwrap_or_else(|| {
        // fields of the player's own inventory needing a update
        let serverside_inventory = menu.as_player();
        s2c::containers::ContainerSlots {
            lists: vec![
                (
                    "craftpreview".to_string(),
                    vec![serverside_inventory.craft_result.clone()],
                ),
                ("craft".to_string(), serverside_inventory.craft.to_vec()),
                ("armor".to_string(), serverside_inventory.armor.to_vec()),
                ("main".to_string(), serverside_inventory.inventory.to_vec()),
                (
                    "offhand".to_string(),
                    vec![serverside_inventory.offhand.clone()],
                ),
            ],
            ..Default::default()
        }
    });
    // we need to shift the inventory that is sent to the client
    // because the hotbar for some reason isnt the first (or even last!) row in the sent data
    // if we ever use indexes on "main" that were sent by the minetest client,
    // we first need to fix these: serverside = (clientside - 9) % 36
    for list in slots.lists.iter_mut() {
        if list.0 == "main" {
            list.1.rotate_right(9);
        }
    }
    if force_full || inventory_state.clientside_fields != slots.lists {
        update_inventory(luanti_conn, slots.lists.clone()).await;
        inventory_state.clientside_fields = slots.lists;
    }
    // the rest only describes an open container
    let Some(container) = container.as_mut() else {
        return;
    };
    let recipes = &inventory_state.stonecutter_recipes;
    if slots.stonecutter_input != container.stonecutter_input
        || slots.loom_patterns != container.loom_patterns
    {
        container.stonecutter_input = slots.stonecutter_input;
        container.loom_patterns = slots.loom_patterns;
        if matches!(container.kind, MenuKind::Stonecutter | MenuKind::Loom) {
            debug!(
                "Sending S2C ShowFormspec for changed options (stonecutter_input={:?}, {} loom patterns)",
                container.stonecutter_input,
                container.loom_patterns.len()
            );
            s2c::containers::reshow_formspec(mc_client, luanti_conn, container, recipes);
        }
    }
    // enchantment options depend on lapis in input slot and player level
    // neither of those is a container property
    if container.kind == MenuKind::Enchantment {
        let affordable = s2c::containers::affordable_enchantments(mc_client, &container.properties);
        if affordable != container.last_shown_affordable {
            container.last_shown_affordable = affordable;
            debug!("Sending S2C ShowFormspec for enchantment affordability {affordable:?}");
            s2c::containers::reshow_formspec(mc_client, luanti_conn, container, recipes);
        }
    }
}
