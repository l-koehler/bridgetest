use azalea::Client;
use azalea::protocol::packets::game::s_container_button_click::ServerboundContainerButtonClick;
use azalea::protocol::packets::game::s_select_trade::ServerboundSelectTrade;
use azalea::protocol::packets::game::s_set_beacon::ServerboundSetBeacon;
use log::*;

use luanti_protocol::commands::client_to_server::InventoryFieldsSpec;

use crate::s2c;
use crate::state;

// luanti sends this whenever any formspec is submitted or closed
pub fn handle_form_fields(
    mc_client: &mut Client,
    specbox: Box<InventoryFieldsSpec>,
    inventory_state: &mut state::InventoryState,
    container: &mut Option<state::ContainerState>,
) {
    let InventoryFieldsSpec {
        client_formspec_name,
        fields,
    } = *specbox;
    // inventory handled differently
    // so no form that isnt our own container form has any reason to be handled
    if client_formspec_name != s2c::containers::CONTAINER_FORM_NAME {
        return;
    }
    if fields.iter().any(|(name, _)| name == "quit") {
        close_open_container(mc_client, inventory_state, container);
        return;
    }
    for (name, _) in &fields {
        if let Some(index) = name.strip_prefix("trade_")
            && let Ok(index) = index.parse::<u32>()
        {
            debug!("Client selected trade offer {}", index);
            let _ = mc_client.write_packet(ServerboundSelectTrade { item: index });
        } else if let Some(index) = name
            .strip_prefix("enchant_")
            .or_else(|| name.strip_prefix("stonecut_"))
            .or_else(|| name.strip_prefix("loom_"))
            && let Ok(button_id) = index.parse::<u32>()
            && let Some(open) = container.as_ref()
        {
            debug!("Client clicked container button {}", button_id);
            let _ = mc_client.write_packet(ServerboundContainerButtonClick {
                container_id: open.id,
                button_id,
            });
        } else if let Some(index) = name.strip_prefix("beacon_")
            && let Ok(index) = index.parse::<usize>()
            && let Some((_, effect)) = s2c::containers::BEACON_EFFECTS.get(index)
        {
            debug!("Client selected beacon effect {:?}", effect);
            let _ = mc_client.write_packet(ServerboundSetBeacon {
                primary: Some(*effect as u32),
                secondary: None,
            });
        }
    }
}

pub fn close_open_container(
    mc_client: &mut Client,
    inventory_state: &mut state::InventoryState,
    container: &mut Option<state::ContainerState>,
) {
    if container.is_some()
        && let Ok(handle) = mc_client.get_inventory()
    {
        handle.close();
    }
    *container = None;
    // drop this to let azalea close our inventory/2x2-grid
    inventory_state.inventory_handle = None;
}
