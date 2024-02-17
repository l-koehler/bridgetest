mod translator;
mod utils;
mod mt_command;

use minetest_protocol::MinetestServer;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() {
    println!("[Debug] main::main()");
    start_client_handler().await;
}

async fn start_client_handler() {
    println!("[Debug] async main::start_client_handler()");
    // TODO: read the port from a config file or something to that effect
    let mt_server_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 30000);
    println!("[Minecraft] Connecting to 127.0.0.1:25565...");
    let mc_conn = TcpStream::connect("127.0.0.1:25565").await;
    
    match mc_conn {
        Ok(mc_stream) => {
            println!("[Minecraft] Connected!");
            let mut mt_server = MinetestServer::new(mt_server_addr);
            println!("[Minetest] Waiting for client to connect...");
            tokio::select! {
                conn = mt_server.accept() => {
                    println!("[Minetest] Client connected from {:?}", conn.remote_addr());
                    translator::client_handler(mt_server, conn, mc_stream).await;
                }
            }
        }
        Err(err) => {
            println!("[Minecraft] Failed to connect! Is the server running? ({})", err);
        }
    }
}
