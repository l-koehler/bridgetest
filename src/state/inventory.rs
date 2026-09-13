use azalea::container::ContainerHandle;
use azalea::inventory::ItemStack;
use azalea::protocol::packets::game::c_merchant_offers::MerchantOffer;
use azalea::protocol::packets::game::c_update_recipes::SingleInputEntry;
use azalea::registry::builtin::{ItemKind, MenuKind};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// inventory state that applies regardless of containers
#[derive(Clone, Default)]
pub struct InventoryState {
    // used to determine need for resyncing (on tick)
    pub clientside_fields: Vec<(String, Vec<ItemStack>)>,
    // never read, only used to not drop the handle.
    // cursed. sorry. just leave it be, it won't break i think
    pub inventory_handle: Option<Arc<Mutex<ContainerHandle>>>,
    // list of known stonecutter recipes, sent once after joining (for some reason).
    pub stonecutter_recipes: Vec<SingleInputEntry>,
}

// the one open container, if any
#[derive(Clone)]
pub struct ContainerState {
    // we could use the ECS, but this is needed for edge detection
    pub id: i32,
    // a later ContainerSetData packet needs this to interpret property IDs
    pub kind: MenuKind,
    // a later MerchantOffers/ContainerSetData packet needs to rebuild the full formspec with title
    pub title: String,
    // trade offers of an open merchant screen, empty until MerchantOffers arrives (or if not a merchant)
    pub trade_offers: Vec<MerchantOffer>,
    // id -> value for ClientboundContainerSetData properties (fuel/cook time etc)
    pub properties: HashMap<u16, u16>,
    // progress bar sprite lengths last sent to the client, length varies
    // used only to compare and prevent useless resends
    pub last_shown_progress: Vec<u16>,
    // enchantment option affordability last sent to the client.
    // depends on player lapis and levels, so we need a tick check for this
    pub last_shown_affordable: [bool; 3],
    // item in an open stonecutters input slot
    pub stonecutter_input: Option<ItemKind>,
    // patterns selectable on an open loom for its current input(s)
    pub loom_patterns: Vec<String>,
}

impl ContainerState {
    pub fn new(id: i32, kind: MenuKind, title: String) -> Self {
        ContainerState {
            id,
            kind,
            title,
            trade_offers: Vec::new(),
            properties: HashMap::new(),
            last_shown_progress: Vec::new(),
            last_shown_affordable: [false; 3],
            stonecutter_input: None,
            loom_patterns: Vec::new(),
        }
    }
}
