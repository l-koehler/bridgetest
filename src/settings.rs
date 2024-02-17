/*
 * This file contains various constants.
 * Until I implement some argv parsing or config file stuff,
 * just change the stuff here before compiling.
 */

use std::net::Ipv4Addr;
use azalea_protocol::ServerAddress;

pub const MT_SERVER_PORT: u16 = 30000;
pub const MT_SERVER_ADDR: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);

pub const MC_SERVER_ADDR: &str = "127.0.0.1:25565";
