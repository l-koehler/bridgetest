mod translator;
mod utils;
mod commands;
mod packet_handler;
mod settings;

use minetest_protocol::MinetestServer;

#[tokio::main]
async fn main() {
    println!("[Debug] main::main()");
    start_client_handler().await;
}

async fn start_client_handler() {
    // Create/Host a Minetest Server
    println!("[Minetest] Creating Server ({})...", settings::MT_SERVER_ADDR);
    let mut mt_server = MinetestServer::new(settings::MT_SERVER_ADDR.parse().unwrap());
    // Wait for a client to join
    tokio::select! {
        conn = mt_server.accept() => {
            println!("[Minetest] Client connected from {:?}", conn.remote_addr());
            translator::client_handler(mt_server, conn).await;
        }
    }
}
