use azalea::Client;
use log::*;

use crate::s2c;
use crate::state::PlayerState;
use crate::utils;
use luanti_protocol::types;

use glam::I16Vec3;

pub async fn interact_node(
    action: types::InteractAction,
    under_surface: I16Vec3,
    above_surface: I16Vec3,
    mc_client: &mut Client,
    player_state: &mut PlayerState,
) {
    // under_surface/above_surface are node coordinates in Luanti's frame;
    // mirror_block_pos converts the X axis into Minecraft's frame (see
    // utils::mirror_pos for why this mirroring is needed at all).
    let under_blockpos = azalea::BlockPos {
        x: utils::mirror_block_pos(under_surface.x.into()),
        y: under_surface.y.into(),
        z: under_surface.z.into(),
    };
    let above_blockpos = azalea::BlockPos {
        x: utils::mirror_block_pos(above_surface.x.into()),
        y: above_surface.y.into(),
        z: above_surface.z.into(),
    };
    match action {
        types::InteractAction::StartDigging => {
            // declare that this button press wasn't for attacking, rather for mining
            // whenever that is set to false, "dig_pressed" switching to true will trigger an attack
            player_state.next_click_no_attack = true;
            mc_client.start_mining(under_blockpos);
        }
        types::InteractAction::StopDigging => stop_digging(mc_client),
        // using a node needs the position of the node that was clicked
        types::InteractAction::Place => {
            node_rightclick(mc_client, under_blockpos, above_blockpos).await
        }
        _ => warn!("Client sent unsupported node interaction: {:?}", action),
    }
}

pub fn stop_digging(mc_client: &mut Client) {
    // HACK: azalea does not seem to have a proper way to do this.
    // mining a block that is out-of-range should cancel any current mining
    // (and trigger anticheats)
    mc_client.start_mining(azalea::BlockPos {
        x: 0,
        y: 1000,
        z: 0,
    })
}

pub async fn node_rightclick(
    mc_client: &mut Client,
    under: azalea::BlockPos,
    above: azalea::BlockPos,
) {
    let block_type = utils::get_block_at(mc_client, &under).unwrap();
    if s2c::defs::INTERACTIVE_BLOCKS.contains(&block_type) {
        mc_client.block_interact(under)
    } else {
        mc_client.block_interact(above)
    }
}
