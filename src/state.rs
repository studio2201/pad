use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::fs;
use axum::extract::ws::Message;
use tokio::sync::mpsc::UnboundedSender;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::{Serialize, Deserialize};

use crate::migration::{Notepad, sanitize_filename, get_notepad_file_path, migrate_default_notepad};

#[derive(Debug, Clone)]
pub struct IndexedItem {
    pub id: String,
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub name: String,
    pub r#match: String,
}

#[derive(Debug, Clone)]
pub struct LoginAttempts {
    pub count: usize,
    pub last_attempt: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotepadsJson {
    pub notepads: Vec<Notepad>,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub port: u16,
    pub site_title: String,
    pub pin: Option<String>,
    pub cookie_max_age_hours: i64,
    pub page_history_cookie_age_days: i64,
    pub max_attempts: usize,
    pub lockout_time_minutes: u64,
    pub trust_proxy: bool,
    pub trusted_proxies: Vec<ipnet::IpNet>,
    pub highlight_languages: Vec<String>,
    pub base_url: String,
    pub node_env: String,
    pub version: String,
}

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
                    name: "Default Notepad".to_string(),
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
                        name: "Default Notepad".to_string(),
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
        let mut data: NotepadsJson = serde_json::from_str(&file_content).unwrap_or(NotepadsJson { notepads: vec![] });

        let mut read_dir = fs::read_dir(&self.data_dir).await?;
        let mut txt_files = Vec::new();

        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "txt") {
                if let Some(name_str) = path.file_stem().and_then(|s| s.to_str()) {
                    txt_files.push(name_str.to_string());
                }
            }
        }

        // Find new files that don't match existing notepad IDs or sanitized names
        let mut new_notepads = Vec::new();
        for txt_file in txt_files {
            let matches_id = data.notepads.iter().any(|n| n.id == txt_file);
            let matches_sanitized_name = data.notepads.iter().any(|n| {
                sanitize_filename(&n.name) == txt_file
            });

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
            println!("Added new notepads: {}", data.notepads.iter().map(|n| n.id.as_str()).collect::<Vec<_>>().join(", "));
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

    pub async fn index_notepads(&self) {
        println!("Indexing notepads...");
        let list = self.load_notepads_list().await;
        
        let mut items = Vec::new();
        for notepad in &list {
            let file_path = get_notepad_file_path(notepad, &self.data_dir).await;
            let content = fs::read_to_string(&file_path).await.unwrap_or_default();
            items.push(IndexedItem {
                id: notepad.id.clone(),
                name: notepad.name.clone(),
                content,
            });
        }

        *self.notepads.write().await = list;
        *self.index_items.write().await = items;
        println!("Indexing complete. Notepads indexed: {}", self.notepads.read().await.len());
    }

    pub async fn search_notepads(&self, query: &str) -> Vec<SearchResult> {
        let items = self.index_items.read().await;
        let query_lower = query.to_lowercase();
        let matcher = SkimMatcherV2::default();

        let mut scored_results = Vec::new();

        for item in items.iter() {
            let name_lower = item.name.to_lowercase();
            let content_lower = item.content.to_lowercase();

            let name_score = matcher.fuzzy_match(&name_lower, &query_lower);
            let content_score = matcher.fuzzy_match(&content_lower, &query_lower);

            if name_score.is_some() || content_score.is_some() {
                let score = std::cmp::max(name_score.unwrap_or(0), content_score.unwrap_or(0));
                scored_results.push((item, score));
            }
        }

        // Sort by search score descending
        scored_results.sort_by(|a, b| b.1.cmp(&a.1));

        scored_results
            .into_iter()
            .map(|(item, _)| {
                let is_filename_match = item.name.to_lowercase().contains(&query_lower);
                let content_lower = item.content.to_lowercase();
                let mut truncated_content = item.content.clone();

                let mut r#match = "notepad".to_string();
                let name_truncated = if item.name.len() >= 20 {
                    format!("{}...", &item.name[..20].trim())
                } else {
                    item.name.clone()
                };

                if !is_filename_match {
                    r#match = format!("content in {}", name_truncated);
                    
                    if let Some(match_index) = content_lower.find(&query_lower) {
                        let mut start = match_index;
                        let mut space_count = 0;
                        while start > 0 && space_count < 3 {
                            start -= 1;
                            if item.content.chars().nth(start) == Some(' ') {
                                space_count += 1;
                            }
                        }

                        let mut end = match_index + query.len();
                        while end < item.content.len() && (end - start) < 25 {
                            end += 1;
                        }

                        let snippet = item.content[start..end].trim().to_string();
                        truncated_content = if start > 0 {
                            format!("...{}", snippet)
                        } else {
                            snippet
                        };
                        if end < item.content.len() {
                            truncated_content = format!("{}...", truncated_content);
                        }
                    } else if item.content.len() > 20 {
                        truncated_content = format!("{}...", &item.content[..20].trim());
                    }
                }

                SearchResult {
                    id: item.id.clone(),
                    name: if is_filename_match { name_truncated } else { truncated_content },
                    r#match,
                }
            })
            .collect()
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
