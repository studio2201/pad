use gloo_net::http::Request;
use crate::types::{Notepad, SearchItem, Settings};
use serde::{Deserialize, Serialize};

pub struct StorageService;

impl StorageService {
    fn local_storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok().flatten()
    }

    pub fn get_theme() -> String {
        Self::local_storage()
            .and_then(|s| s.get_item("rustpad_theme").ok().flatten()
                .or_else(|| s.get_item("dumbpad_theme").ok().flatten()))
            .unwrap_or_else(|| "light".to_string())
    }

    pub fn set_theme(theme: &str) {
        if let Some(s) = Self::local_storage() {
            let _ = s.set_item("rustpad_theme", theme);
        }
    }

    pub fn get_settings() -> Settings {
        Self::local_storage()
            .and_then(|s| s.get_item("rustpad_settings").ok().flatten()
                .or_else(|| s.get_item("dumbpad_settings").ok().flatten()))
            .and_then(|val| serde_json::from_str(&val).ok())
            .unwrap_or_default()
    }

    pub fn set_settings(settings: &Settings) {
        if let Some(s) = Self::local_storage() {
            if let Ok(serialized) = serde_json::to_string(settings) {
                let _ = s.set_item("rustpad_settings", &serialized);
            }
        }
    }
}

pub struct ApiService;

#[derive(Deserialize)]
pub struct NotepadsResponse {
    pub notepads_list: Vec<Notepad>,
    pub note_history: String,
}

#[derive(Deserialize)]
pub struct NotesResponse {
    pub content: String,
}

#[derive(Serialize)]
pub struct SaveNotesPayload {
    pub content: String,
}

#[derive(Serialize)]
pub struct RenameNotepadPayload {
    pub name: String,
}



#[derive(Deserialize)]
pub struct PinRequiredResponse {
    pub required: bool,
    pub length: usize,
    pub locked: bool,
}

#[derive(Serialize)]
pub struct VerifyPinPayload {
    pub pin: String,
}

#[derive(Deserialize)]
pub struct VerifyPinResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigResponse {
    pub version: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub results: Vec<SearchItem>,
}

impl ApiService {
    pub async fn check_pin_required() -> Result<PinRequiredResponse, gloo_net::Error> {
        Request::get("/api/pin-required")
            .send()
            .await?
            .json::<PinRequiredResponse>()
            .await
    }

    pub async fn verify_pin(pin: &str) -> Result<VerifyPinResponse, gloo_net::Error> {
        let payload = VerifyPinPayload { pin: pin.to_string() };
        let response = Request::post("/api/verify-pin")
            .json(&payload)?
            .send()
            .await?;
        if response.status() == 401 || response.status() == 429 || response.status() == 400 {
            if let Ok(err_res) = response.json::<serde_json::Value>().await {
                if let Some(err_str) = err_res.get("error").and_then(|v| v.as_str()) {
                    return Ok(VerifyPinResponse { success: false, error: Some(err_str.to_string()) });
                }
            }
        }
        response.json::<VerifyPinResponse>().await
    }

    pub async fn logout() -> Result<(), gloo_net::Error> {
        Request::post("/api/logout")
            .send()
            .await?;
        Ok(())
    }

    pub async fn get_config() -> Result<ConfigResponse, gloo_net::Error> {
        Request::get("/api/config")
            .send()
            .await?
            .json::<ConfigResponse>()
            .await
    }

    pub async fn get_notepads() -> Result<NotepadsResponse, gloo_net::Error> {
        Request::get("/api/notepads")
            .send()
            .await?
            .json::<NotepadsResponse>()
            .await
    }

    pub async fn get_notes(id: &str) -> Result<NotesResponse, gloo_net::Error> {
        Request::get(&format!("/api/notes/{}", id))
            .send()
            .await?
            .json::<NotesResponse>()
            .await
    }

    pub async fn save_notes(id: &str, content: &str) -> Result<(), gloo_net::Error> {
        let payload = SaveNotesPayload { content: content.to_string() };
        Request::post(&format!("/api/notes/{}", id))
            .json(&payload)?
            .send()
            .await?;
        Ok(())
    }

    pub async fn create_notepad() -> Result<Notepad, gloo_net::Error> {
        Request::post("/api/notepads")
            .send()
            .await?
            .json::<Notepad>()
            .await
    }

    pub async fn rename_notepad(id: &str, name: &str) -> Result<(), gloo_net::Error> {
        let payload = RenameNotepadPayload { name: name.to_string() };
        Request::put(&format!("/api/notepads/{}", id))
            .json(&payload)?
            .send()
            .await?;
        Ok(())
    }

    pub async fn delete_notepad(id: &str) -> Result<(), gloo_net::Error> {
        Request::delete(&format!("/api/notepads/{}", id))
            .send()
            .await?;
        Ok(())
    }

    pub async fn search(query: &str) -> Result<SearchResponse, gloo_net::Error> {
        let encoded = percent_encoding::utf8_percent_encode(query, percent_encoding::NON_ALPHANUMERIC).to_string();
        Request::get(&format!("/api/search?query={}", encoded))
            .send()
            .await?
            .json::<SearchResponse>()
            .await
    }
}
