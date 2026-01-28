use azalea::Client;
use log::*;
use luanti_protocol::commands::client_to_server::TSChatMessageSpec;

pub fn send_message(mc_client: &Client, specbox: Box<TSChatMessageSpec>) {
    debug!("Forwarding chat message sent by client");
    let TSChatMessageSpec { message } = *specbox;
    mc_client.chat(&message);
}
