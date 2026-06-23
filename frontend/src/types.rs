#![allow(dead_code)]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notepad {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub save_status_message_interval: u64,
    pub enable_remote_connection_messages: bool,
    pub disable_print_expand: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            save_status_message_interval: 500,
            enable_remote_connection_messages: false,
            disable_print_expand: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchItem {
    pub id: String,
    pub name: String,
    #[serde(rename = "match")]
    pub r#match: String,
}
