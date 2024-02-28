// this contains functions that TAKE data from the client
// and send it to the MC server.
use crate::utils;

use azalea_client::Client;
use alloc::boxed::Box;
use minetest_protocol::wire::command::{TSChatMessageSpec, PlayerposSpec};

pub fn send_message(mc_client: &Client, specbox: Box<TSChatMessageSpec>) {
    utils::logger("[Minetest] C->S Forwarding Message sent by client", 1);
    let TSChatMessageSpec { message } = *specbox;
    mc_client.chat(&message);
}

pub fn playerpos(mc_client: &Client, specbox: Box<PlayerposSpec>) {
    
}
