/*
 * This file contains the loop in which packets from the MT Client are
 * processed (fn client_handler)
 */


use crate::utils;
use crate::mt_command;
use minetest_protocol::peer::peer::PeerError;
use minetest_protocol::wire::command::ToClientCommand;
use minetest_protocol::wire::command::ToServerCommand;
use minetest_protocol::wire::command::CommandProperties;
use minetest_protocol::CommandRef;
use minetest_protocol::MinetestClient;
use minetest_protocol::MinetestConnection;
use minetest_protocol::MinetestServer;

pub async fn client_handler(mt_server: MinetestServer, mut conn: MinetestConnection) {
    println!("[Debug] async translator::client_handler()");

    loop {
        tokio::select! {
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
                command_handler(command, &mut conn);
            }
        }
    }
}

fn command_handler(command: ToServerCommand, conn: &mut MinetestConnection) {
    println!("[Debug] translator::command_handler()");
    // match command.command_name() {
    //     "Init" => mt_command::handshake(command, conn),
    //     _ => println!("not implemented")
    // }
}
