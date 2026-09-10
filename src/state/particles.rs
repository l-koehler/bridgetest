use azalea::BlockPos;
use std::collections::HashMap;

// tracks which particle spawners are currently registered client-side
#[derive(Clone, Default)]
pub struct ParticleSpawnerState {
    /// pos -> (index into s2c::particles::SPAWNER_DEFS, server_ids of emissions)
    active: HashMap<BlockPos, (usize, Vec<u32>)>,
    next_id: u32,
}

impl ParticleSpawnerState {
    pub fn get(&self, pos: &BlockPos) -> Option<(usize, Vec<u32>)> {
        self.active.get(pos).cloned()
    }

    pub fn remove(&mut self, pos: &BlockPos) -> Option<(usize, Vec<u32>)> {
        self.active.remove(pos)
    }

    // allocates fresh server_ids and tracks them for pos
    pub fn insert(&mut self, pos: BlockPos, def_index: usize, count: usize) -> Vec<u32> {
        let ids: Vec<u32> = (0..count)
            .map(|_| {
                let id = self.next_id;
                self.next_id = self.next_id.wrapping_add(1);
                id
            })
            .collect();
        self.active.insert(pos, (def_index, ids.clone()));
        ids
    }
}
