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
