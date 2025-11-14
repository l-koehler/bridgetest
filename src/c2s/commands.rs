use crate::c2s;
use crate::state::ProxyState;
use log::*;
use luanti_protocol::LuantiConnection;
use luanti_protocol::commands::CommandProperties;
use luanti_protocol::commands::client_to_server::ToServerCommand;

pub async fn process(
    command: ToServerCommand,
    luanti_conn: &mut LuantiConnection,
    mc_client: &mut azalea::Client,
    proxy_state: &mut ProxyState,
) {
    match command {
        ToServerCommand::Init(_) => error!("Client sent Init, but handshake already done!"),
        ToServerCommand::Init2(_) => debug!(
            "Client sent Init2 (preferred language), this is not implemented and will be ignored."
        ),
        // Minecraft has no concept of modchannels and does not need these.
        ToServerCommand::ModchannelJoin(_) => {
            trace!("Client sent ModchannelJoin, this is not implemented and will be ignored.")
        }
        ToServerCommand::ModchannelLeave(_) => {
            trace!("Client sent ModchannelLeave, this is not implemented and will be ignored.")
        }
        ToServerCommand::TSModchannelMsg(_) => {
            trace!("Client sent TSModchannelMsg, this is not implemented and will be ignored.")
        }
        ToServerCommand::Playerpos(specbox) => {
            c2s::player::playerpos(
                mc_client,
                specbox,
                &mut proxy_state.player,
                &mut proxy_state.inventory,
            )
            .await
        }
        ToServerCommand::TSChatMessage(specbox) => c2s::chat::send_message(mc_client, specbox),
        ToServerCommand::Interact(specbox) => {
            c2s::player::interact(mc_client, specbox, &mut proxy_state.player).await
        }
        ToServerCommand::PlayerItem(specbox) => c2s::inventory::set_mainhand(mc_client, specbox),
        ToServerCommand::InventoryAction(specbox) => {
            c2s::inventory::inventory_action(
                mc_client,
                luanti_conn,
                specbox,
                &mut proxy_state.inventory,
                &proxy_state.player
            )
            .await
        }
        ToServerCommand::GotBlocks(_) => (), // Gotblocks just confirms to the server that blocks were received
        _ => warn!(
            "Got unimplemented C2S command, dropping {}",
            command.command_name()
        ),
    }
}
