use azalea::registry::Holder;
use log::*;
use luanti_protocol::LuantiConnection;
use luanti_protocol::commands::server_to_client;
use luanti_protocol::commands::server_to_client::ToClientCommand;

use azalea_client::chat::ChatPacket;
use azalea_language;

use azalea::protocol::packets::game::c_sound::ClientboundSound;

use azalea::protocol::packets::game::c_system_chat::ClientboundSystemChat;

use crate::state;
use std::time::Instant;

pub async fn send_message(conn: &mut LuantiConnection, message: ChatPacket) {
    let chat_packet =
        ToClientCommand::TCChatMessage(Box::new(server_to_client::TCChatMessageSpec {
            version: 1,               // idk what this or message_type do
            message_type: 1,          // but it works, dont touch it
            sender: String::from(""), // already in message
            message: message.message().to_string(),
            timestamp: chrono::Utc::now().timestamp().try_into().unwrap_or(0),
        }));
    conn.send(chat_packet).unwrap();
}

pub async fn send_sys_message(conn: &mut LuantiConnection, message: &ClientboundSystemChat) {
    if let azalea::FormattedText::Text(component) = &message.content {
        let chat_packet =
            ToClientCommand::TCChatMessage(Box::new(server_to_client::TCChatMessageSpec {
                version: 1,      // idk what this or message_type do
                message_type: 1, // but it works, dont touch it
                sender: String::from("System"),
                message: component.text.to_string(),
                timestamp: chrono::Utc::now().timestamp().try_into().unwrap_or(0),
            }));
        conn.send(chat_packet).unwrap();
    }
}

// can't figure out how to get "actual" subtitles, so these are just the audio keys mapped to subtitle keys
pub fn show_sound(
    packet_data: &ClientboundSound,
    chat_state: &mut state::ChatState,
) {
    let ClientboundSound {
        sound,
        source: _,
        x: _,
        y: _,
        z: _,
        volume: _,
        pitch: _,
        seed: _,
    } = packet_data;
    trace!("[Minetest] New Subtitle: {:?}", sound);
    let key = match sound {
        Holder::Reference(sound_ref) => sound_ref.to_string().replace("minecraft:", "subtitles."),
        Holder::Direct(_) => {
            // shouldn't happen on vanilla server i think
            String::from("custom sound (unsupported)")
        }
    };

    let Some(subtitle_str) = azalea_language::get(&key) else {
        info!("Did not find subtitle in azalea_language, using key as value!");
        chat_state.subtitles.push((key, Instant::now()));
        return;
    };
    chat_state
        .subtitles
        .push((String::from(subtitle_str), Instant::now()));
}
