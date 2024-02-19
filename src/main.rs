// assorted stuff

extern crate alloc;

mod translator;
mod utils;
mod commands;
mod settings;
mod clientbound_translator;

use minetest_protocol::MinetestServer;

use alloc::vec::Vec;

#[tokio::main]
async fn main() {
    // idk why main exists when it only calls start_client_handler
    // save me compiler optimization,,,
    start_client_handler().await;
}

pub struct MTServerState {
    players: Vec<String>,
    // add other stuff i need to keep track of
}

async fn start_client_handler() {
    // Create/Host a Minetest Server
    utils::logger(&format!("[Minetest] Creating Server ({})...", settings::MT_SERVER_ADDR), 1);
    let mut mt_server = MinetestServer::new(settings::MT_SERVER_ADDR.parse().unwrap());
    // Define a server state with stuff to keep track of because MinetestServer
    // is NOT being helpful here :(
    let mut mt_server_state = MTServerState {
        players: Vec::new(),
    };


    // Wait for a client to join
    tokio::select! {
        conn = mt_server.accept() => {
            utils::logger(&format!("[Minetest] Client connected from {:?}", conn.remote_addr()), 1);
            translator::client_handler(mt_server, conn, mt_server_state).await;
        }
    }
}
