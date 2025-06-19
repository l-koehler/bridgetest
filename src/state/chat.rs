use std::time::Instant;

#[derive(Clone)]
pub struct ChatState {
    // not the sendable subtitle, just the localization key!
    // the instant is when it expires
    pub subtitles: Vec<(String, Instant)>,
    // the last-sent complete subtitle text.
    // used for edge detection, but TODO remove it and just send updates when subtitles expire
    pub prev_subtitle_string: String,
}

impl Default for ChatState {
    fn default() -> Self {
        ChatState {
            subtitles: Vec::new(),
            prev_subtitle_string: String::new(),
        }
    }
}
