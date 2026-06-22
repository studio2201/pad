use std::path::{Path, PathBuf};
use tokio::fs;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Notepad {
    pub id: String,
    pub name: String,
}

/// Helper function to sanitize filename for file system
pub fn sanitize_filename(name: &str) -> String {
    let re = regex::Regex::new(r#"[<>:"/\\|?*\x00-\x1f]"#).unwrap();
    let sanitized = re.replace_all(name, "_");
    sanitized.trim().to_string()
}

/// Helper function to get file path for notepad (tries name-based first, falls back to ID-based)
pub async fn get_notepad_file_path(notepad: &Notepad, data_dir: &Path) -> PathBuf {
    if notepad.id == "default" {
        return data_dir.join("default.txt");
    }

    let sanitized_name = sanitize_filename(&notepad.name);
    let name_based_path = data_dir.join(format!("{}.txt", sanitized_name));
    let id_based_path = data_dir.join(format!("{}.txt", notepad.id));

    if fs::metadata(&name_based_path).await.is_ok() {
        name_based_path
    } else if fs::metadata(&id_based_path).await.is_ok() {
        id_based_path
    } else {
        name_based_path
    }
}

/// Ensure default notepad file exists
pub async fn migrate_default_notepad(data_dir: &Path) -> Result<(), std::io::Error> {
    let default_note_path = data_dir.join("default.txt");
    if fs::metadata(&default_note_path).await.is_err() {
        fs::write(&default_note_path, "").await?;
        println!("Created default notepad file: {:?}", default_note_path);
    }
    Ok(())
}

/// Function to migrate all existing notepads to name-based filenames
pub async fn migrate_all_notepads_to_name_based_files(notepads: &[Notepad], data_dir: &Path) {
    println!("Checking for notepad files to migrate...");
    let mut migrated_count = 0;

    for notepad in notepads {
        if notepad.id == "default" {
            continue;
        }

        let old_path = data_dir.join(format!("{}.txt", notepad.id));
        let sanitized_name = sanitize_filename(&notepad.name);
        let new_path = data_dir.join(format!("{}.txt", sanitized_name));

        if old_path != new_path {
            if fs::metadata(&old_path).await.is_ok() {
                if fs::metadata(&new_path).await.is_ok() {
                    println!(
                        "Skipping migration for {}: both {:?} and {:?} exist",
                        notepad.name, old_path, new_path
                    );
                } else {
                    if let Err(e) = fs::rename(&old_path, &new_path).await {
                        eprintln!("Failed to migrate {:?} to {:?}: {}", old_path, new_path, e);
                    } else {
                        println!("Migrated: {:?} -> {:?}", old_path, new_path);
                        migrated_count += 1;
                    }
                }
            }
        }
    }

    if migrated_count > 0 {
        println!("Successfully migrated {} notepad files to name-based filenames", migrated_count);
    } else {
        println!("No notepad files needed migration");
    }
}
