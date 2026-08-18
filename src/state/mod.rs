pub mod chat;
pub mod entities;
pub mod inventory;
pub mod media;
pub mod player;
pub mod world;

pub use chat::*;
pub use entities::*;
pub use inventory::*;
pub use media::*;
pub use player::*;
// world state is partially used (TimeState, Dimensions)
pub use world::*;

#[derive(Clone, Default)]
pub struct ProxyState {
    pub chat: ChatState,
    pub entities: EntityState,
    pub inventory: InventoryState,
    pub media: MediaState,
    pub player: PlayerState,
    pub time: TimeState,
}
