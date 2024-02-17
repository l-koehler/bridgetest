/*
 * This file contains functions which get called from the main loop
 * as a reaction to certain packets arriving.
 * For example, the handshake() function here will just relay to
 * commands.rs, where a handshake with both server and client will be performed.
 */

use crate::utils;

use minetest_protocol::wire::command::ToServerCommand;
use minetest_protocol::MinetestConnection;

pub async fn auto(_command: ToServerCommand, _conn: &mut MinetestConnection) {
    // do whatever TODO
}

pub async fn handshake(_command: ToServerCommand, _conn: &mut MinetestConnection) {
    // TODO
}
