use axum::extract::ws::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::RwLock;

use crate::migration::{migrate_default_notepad, sanitize_filename, Notepad};
use crate::search::IndexedItem;

#[derive(Debug, Clone)]
pub struct LoginAttempts {
    pub count: usize,
    pub last_attempt: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotepadsJson {
    pub notepads: Vec<Notepad>,
}

pub use crate::config::AppConfig;

pub struct AppStateInner {
    // Config
    pub config: AppConfig,
    pub data_dir: PathBuf,
    pub notepads_file: PathBuf,

    // Real-time clients map: userId -> UnboundedSender<Message>
    pub clients: RwLock<HashMap<String, UnboundedSender<Message>>>,

    // Operational Transformation (OT) history map: notepadId -> Operations
    pub operations_history: RwLock<HashMap<String, Vec<serde_json::Value>>>,

    // Login attempts brute-force prevention
    pub login_attempts: RwLock<HashMap<IpAddr, LoginAttempts>>,

    // Notepad metadata and index cache
    pub notepads: RwLock<Vec<Notepad>>,
    pub index_items: RwLock<Vec<IndexedItem>>,
}

pub type AppState = Arc<AppStateInner>;

impl AppStateInner {
    pub async fn ensure_data_dir(&self) -> Result<(), std::io::Error> {
        fs::create_dir_all(&self.data_dir).await?;

        if fs::metadata(&self.notepads_file).await.is_err() {
            println!("Creating new notepads.json");
            let default_data = NotepadsJson {
                notepads: vec![Notepad {
                    id: "default".to_string(),
                    name: "default".to_string(),
                }],
            };
            let content = serde_json::to_string_pretty(&default_data)?;
            fs::write(&self.notepads_file, content).await?;
        } else {
            // Validate structure
            let content = fs::read_to_string(&self.notepads_file).await?;
            if let Err(e) = serde_json::from_str::<NotepadsJson>(&content) {
                eprintln!("Invalid notepads.json, recreating: {}", e);
                let default_data = NotepadsJson {
                    notepads: vec![Notepad {
                        id: "default".to_string(),
                        name: "default".to_string(),
                    }],
                };
                let content = serde_json::to_string_pretty(&default_data)?;
                fs::write(&self.notepads_file, content).await?;
            }
        }

        migrate_default_notepad(&self.data_dir).await?;
        Ok(())
    }

    pub async fn load_notepads_list(&self) -> Vec<Notepad> {
        self.get_notepads_from_dir().await.unwrap_or_default()
    }

    pub async fn get_notepads_from_dir(&self) -> Result<Vec<Notepad>, std::io::Error> {
        self.ensure_data_dir().await?;

        let file_content = fs::read_to_string(&self.notepads_file).await?;
        let mut data: NotepadsJson =
            serde_json::from_str(&file_content).unwrap_or(NotepadsJson { notepads: vec![] });

        let mut read_dir = fs::read_dir(&self.data_dir).await?;
        let mut txt_files = Vec::new();

        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "txt") {
                if let Some(name_str) = path.file_stem().and_then(|s| s.to_str()) {
                    txt_files.push(name_str.to_string());
                }
            }
        }

        // Find new files that don't match existing notepad IDs or sanitized names
        let mut new_notepads = Vec::new();
        for txt_file in txt_files {
            let matches_id = data.notepads.iter().any(|n| n.id == txt_file);
            let matches_sanitized_name = data
                .notepads
                .iter()
                .any(|n| sanitize_filename(&n.name) == txt_file);

            if !matches_id && !matches_sanitized_name {
                let unique_name = self.generate_unique_name(&txt_file, &data.notepads);
                new_notepads.push(Notepad {
                    id: txt_file,
                    name: unique_name,
                });
            }
        }

        if !new_notepads.is_empty() {
            data.notepads.extend(new_notepads.clone());
            let content = serde_json::to_string_pretty(&data)?;
            fs::write(&self.notepads_file, content).await?;
            println!(
                "Added new notepads: {}",
                data.notepads
                    .iter()
                    .map(|n| n.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        Ok(data.notepads)
    }

    pub fn generate_unique_name(&self, desired_name: &str, existing: &[Notepad]) -> String {
        let mut unique_name = desired_name.to_string();
        let mut counter = 1;

        while existing.iter().any(|n| n.name == unique_name)
            || sanitize_filename(&unique_name).to_lowercase() == "default"
        {
            unique_name = format!("{}-{}", desired_name, counter);
            counter += 1;
        }

        unique_name
    }

    // Rate Limiting helper: check lockout
    pub async fn is_locked_out(&self, ip: IpAddr) -> bool {
        let map = self.login_attempts.read().await;
        if let Some(attempts) = map.get(&ip) {
            if attempts.count >= self.config.max_attempts {
                let elapsed = attempts.last_attempt.elapsed();
                if elapsed < Duration::from_secs(self.config.lockout_time_minutes * 60) {
                    return true;
                }
            }
        }
        false
    }

    pub async fn record_login_attempt(&self, ip: IpAddr) {
        let mut map = self.login_attempts.write().await;
        let attempts = map.entry(ip).or_insert(LoginAttempts {
            count: 0,
            last_attempt: Instant::now(),
        });
        attempts.count += 1;
        attempts.last_attempt = Instant::now();
    }

    pub async fn reset_login_attempts(&self, ip: IpAddr) {
        let mut map = self.login_attempts.write().await;
        map.remove(&ip);
    }

    pub async fn clean_old_lockouts(&self) {
        let mut map = self.login_attempts.write().await;
        let lockout_dur = Duration::from_secs(self.config.lockout_time_minutes * 60);
        map.retain(|_, attempts| attempts.last_attempt.elapsed() < lockout_dur);
    }
}
