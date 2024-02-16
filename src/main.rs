mod translator;
mod utils;
mod mt_command;
mod mc_command;

use minetest_protocol::MinetestServer;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[tokio::main]
async fn main() {
    println!("[Debug] main::main()");
    mt_server().await;
}

async fn mt_server() {
    println!("[Debug] async main::mt_server()");
    // TODO: read the port from a config file or something to that effect
    let mt_server_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 30000);
    let mut mt_server = MinetestServer::new(mt_server_addr);
    
    tokio::select! {
        conn = mt_server.accept() => {
            println!("Minetest client connected from {:?}", conn.remote_addr());
            translator::client_handler(mt_server, conn).await;
        }
    }
}
