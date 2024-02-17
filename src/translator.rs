/*
 * This file contains the loop in which packets from the MT Client are
 * processed (fn client_handler).
 * Also, this file is badly named (as you might have noticed).
 */


use crate::utils;
use crate::mt_command;

use minetest_protocol::peer::peer::PeerError;
use minetest_protocol::wire::command::CommandProperties;
use minetest_protocol::wire::command::ToServerCommand;
use minetest_protocol::MinetestConnection;
use minetest_protocol::MinetestServer;

use tokio::net::TcpStream;

const MAX_MC_PACKET_SIZE: usize = 2097151;

pub async fn client_handler(_mt_server: MinetestServer, mut conn: MinetestConnection, mc_conn: TcpStream) {
    println!("[Debug] async translator::client_handler()");

    loop {
        tokio::select! {
            // recieve data over the MinetestConnection
            t = conn.recv() => {
                // Check if the client disconnected
                match t {
                    Ok(_) => (),
                    Err(err) => {
                        let show_err = if let Some(err) = err.downcast_ref::<PeerError>() {
                            match err {
                                PeerError::PeerSentDisconnect => false,
                                _ => true,
                            }
                        } else {
                            true
                        };
                        if show_err {
                            println!("Minetest Client Disconnected: {:?}", err)
                        } else {
                            println!("Minetest Client Disconnected")
                        }
                        break; // Exit the client handler on client disconnect
                    }
                }
                
                let command = t.unwrap();
                utils::show_mt_command(&command);
                
                // pass the command to somewhere else for handling
                command_handler(command, &mut conn).await;
            },
            // or recieve data over the minecraft/TCP connection (mc_conn)
            _ready = mc_conn.readable() => {
                println!("[Minecraft] TCP Connection became readable!");
                // ready isnt even relevant
                let mut buffer = vec![0; MAX_MC_PACKET_SIZE];
                
                match mc_conn.try_read(&mut buffer) {
                    Ok(n) => {
                        buffer.truncate(n);
                        
                        break;
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // stream can't be read, consume the readiness event
                        continue;
                    }
                    Err(e) => {
                        // some actual error while reading the minecraft tcp
                        println!("[Minecraft] Encontered Error in readable TCP: {}", e);
                    }
                }
            }
        }
    }
}

async fn command_handler(_command: ToServerCommand, _conn: &mut MinetestConnection) {
    println!("[Debug] translator::command_handler()");
    match _command.command_name() {
        "Init" => mt_command::handshake(_command, _conn).await,
        _ => println!("Dropping MT Packet: Unknown Name")
    }
}
