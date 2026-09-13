pub mod chat;
pub mod entities;
pub mod inventory;
pub mod media;
pub mod particles;
pub mod player;
pub mod world;

pub use chat::*;
pub use entities::*;
pub use inventory::*;
pub use media::*;
pub use particles::*;
pub use player::*;
pub use world::*;

#[derive(Clone, Default)]
pub struct ProxyState {
    pub chat: ChatState,
    pub entities: EntityState,
    pub inventory: InventoryState,
    pub container: Option<ContainerState>, // None if no container open
    pub media: MediaState,
    pub particles: ParticleSpawnerState,
    pub player: PlayerState,
    pub time: TimeState,
    pub light: LightCache,
    pub chunk_batch: ChunkBatchState,
}
