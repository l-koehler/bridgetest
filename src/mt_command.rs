/*
 * This file contains functions that perform specific actions with the MT client
 */

use minetest_protocol::wire::command::ToClientCommand;
use minetest_protocol::wire::command::ToServerCommand;
use minetest_protocol::CommandDirection;
use minetest_protocol::CommandRef;
use minetest_protocol::MinetestClient;
use minetest_protocol::MinetestConnection;
use minetest_protocol::MinetestServer;


pub fn handshake(command: ToServerCommand, mut conn: MinetestConnection) {
    println!("NOT IMPLEMENTED");
}
