mod translator;
mod utils;
mod commands;
mod packet_handler;
mod settings;

use minetest_protocol::MinetestServer;
use std::net::{IpAddr, SocketAddr};

#[tokio::main]
async fn main() {
    println!("[Debug] main::main()");
    start_client_handler().await;
}

async fn start_client_handler() {
    // Create/Host a Minetest Server
    println!("[Minetest] Creating Server (Port: {})...", settings::MT_SERVER_PORT);
    let mt_server_addr = SocketAddr::new(IpAddr::V4(settings::MT_SERVER_ADDR),
                                         settings::MT_SERVER_PORT);
    let mut mt_server = MinetestServer::new(mt_server_addr);
    // Wait for a client to join
    tokio::select! {
        conn = mt_server.accept() => {
            println!("[Minetest] Client connected from {:?}", conn.remote_addr());
            translator::client_handler(mt_server, conn).await;
        }
    }
}
