pub mod chat;
pub mod entities;
pub mod inventory;
pub mod media;
pub mod player;
pub mod world;

pub use chat::*;
pub use entities::*;
// world state is unused right now
//pub use world::*;
pub use inventory::*;
pub use media::*;
pub use player::*;

#[derive(Clone, Default)]
pub struct ProxyState {
    pub chat: ChatState,
    pub entities: EntityState,
    pub inventory: InventoryState,
    pub media: MediaState,
    pub player: PlayerState,
    // pub world: WorldState // could go here, but i don't need it right now
}
