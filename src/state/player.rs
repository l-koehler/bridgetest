use super::world::Dimensions;
use azalea::registry::MobEffect;
use std::time::Instant;

#[derive(Clone)]
pub struct PlayerState {
    // used to not attack on every left click, only on ones that aren't breaking blocks
    pub next_click_no_attack: bool,
    // used to only attack on the rising edge, not constantly
    pub previous_dig_held: bool,
    // (potion_effect, ends_at, flags) on the client
    // used to update the formspec each tick
    pub client_effects: Vec<(MobEffect, Instant, u8)>,
    // used to determine if a HP change should trigger a damage effect flash
    pub mt_last_known_health: u16,
    // used to determine if the air supply bar should change
    pub mc_last_air_supply: u32,
    // needed for respawning
    pub respawn_pos: (f32, f32, f32),
    pub current_dimension: Dimensions,
    // stuff for input edges
    pub is_sneaking: bool,
    pub mt_max_speed: f32,
    pub has_moved_since_sync: bool,
    pub keys_pressed: u32,
    // used to tolerate slight position differences, resulting in far smoother movement
    pub mt_clientside_pos: (f32, f32, f32),
    // TODO remove this trash, use ECS instead
    pub players: Vec<String>,        // names of all players
    pub client_rotation: (f32, f32), // yaw/pitch
}

impl Default for PlayerState {
    fn default() -> Self {
        PlayerState {
            next_click_no_attack: false,
            previous_dig_held: false,
            client_effects: Vec::new(),
            mt_last_known_health: 0,
            mc_last_air_supply: 0,
            respawn_pos: (0.0, 0.0, 0.0),
            current_dimension: Dimensions::Overworld,
            is_sneaking: false,
            mt_max_speed: 4.317,
            has_moved_since_sync: true,
            keys_pressed: 0,
            mt_clientside_pos: (0.0, 0.0, 0.0),
            //TODO remove these, use ECS
            players: Vec::new(),
            client_rotation: (0.0, 0.0),
        }
    }
}
