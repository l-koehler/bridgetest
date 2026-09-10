/// We anchor on the latest `total_ticks` we saw, then advance it by the change in
/// `game_time` to get a continuously-updating daytime phase.
#[derive(Clone, Copy)]
pub struct TimeState {
    /// the most recent world-clock total_ticks (daylight-cycle tick counter).
    pub clock_total: u64,
    /// the game_time value that was present when `clock_total` was last updated.
    pub anchor_game_time: u64,
}

impl Default for TimeState {
    fn default() -> Self {
        Self {
            clock_total: 0,
            anchor_game_time: 0,
        }
    }
}

#[derive(Clone, PartialEq, Copy)]
pub enum Dimensions {
    Overworld,
    Nether,
    End,
    Custom, // assumes overworld height
}

impl Dimensions {
    pub const fn get_y_bounds(self: Self) -> (i16, i16) {
        match self {
            Dimensions::Nether => (0, 255), // worldgen limit is 128, but players can go above that
            Dimensions::End => (0, 255),
            Dimensions::Overworld => (-64, 320),
            Dimensions::Custom => (-64, 320),
        }
    }
}

// tracks whether we're between a ChunkBatchStart and its ChunkBatchFinished
#[derive(Clone, Copy, Default)]
pub struct ChunkBatchState {
    pub active: bool,
}

// default for sections we never got light data for
const UNKNOWN_LIGHT: u8 = 14;

#[derive(Clone, Default)]
pub struct LightCache {
    sections: std::collections::HashMap<(i16, i16, i16), ([u8; 4096], [u8; 4096])>,
}

impl LightCache {
    pub fn store(
        &mut self,
        x_pos: i16,
        y_pos: i16,
        z_pos: i16,
        sky: [u8; 4096],
        block: [u8; 4096],
    ) {
        self.sections.insert((x_pos, y_pos, z_pos), (sky, block));
    }

    // same indexing as state_array, minecraft X-handedness
    pub fn get_section(&self, x_pos: i16, y_pos: i16, z_pos: i16) -> ([u8; 4096], [u8; 4096]) {
        self.sections
            .get(&(x_pos, y_pos, z_pos))
            .copied()
            .unwrap_or(([UNKNOWN_LIGHT; 4096], [UNKNOWN_LIGHT; 4096]))
    }

    pub fn get_node(&self, x: i32, y: i32, z: i32) -> (u8, u8) {
        let section = (
            x.div_euclid(16) as i16,
            y.div_euclid(16) as i16,
            z.div_euclid(16) as i16,
        );
        let idx = x.rem_euclid(16) as usize
            + (y.rem_euclid(16) as usize) * 16
            + (z.rem_euclid(16) as usize) * 256;
        match self.sections.get(&section) {
            Some((sky, block)) => (sky[idx], block[idx]),
            None => (UNKNOWN_LIGHT, UNKNOWN_LIGHT),
        }
    }

    // empties the cache, returns positions of every section that was stored
    pub fn take_positions(&mut self) -> Vec<(i16, i16, i16)> {
        self.sections.drain().map(|(pos, _)| pos).collect()
    }
}
