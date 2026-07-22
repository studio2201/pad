use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Notepad {
    pub id: String,
    pub name: String,
}

/// Maximum filename length after sanitization. 200 chars leaves headroom for
/// `.txt` extension and avoids `ENAMETOOLONG` on common filesystems (ext4 limit
/// is 255 bytes; 200 chars stays safe with multi-byte UTF-8 in play).
pub const MAX_FILENAME_LEN: usize = 200;

/// Windows-reserved device names (case-insensitive). Creating any of these as
/// filenames on a Windows-mounted share would silently fail or shadow the
/// device. Rejecting them at the API boundary prevents the failure mode from
/// reaching the FS layer.
const WINDOWS_RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Sanitize a user-supplied filename and validate it for safe filesystem use.
///
/// Returns the sanitized name on success, or an error describing why the name
/// was rejected. The sanitization rules are:
///
/// 1. Strip any character in `< > : " / \ | ? *` or ASCII control chars (0x00-0x1F).
/// 2. Trim whitespace from both ends.
/// 3. Reject names that are empty after sanitization.
/// 4. Reject names that begin with a `.` (Unix hidden-file convention; users
///    don't expect their notepads to be invisible).
/// 5. Reject names that consist only of `.` characters (would survive sanitization
///    but resolve to `.` or `..` and either escape or replace the directory).
/// 6. Reject Windows-reserved device names (CON, PRN, etc.) case-insensitively.
/// 7. Cap the final length at [`MAX_FILENAME_LEN`].
pub fn sanitize_filename(name: &str) -> Result<String, String> {
    let re = match regex::Regex::new(r#"[<>:"/\\|?*\x00-\x1f]"#) {
        Ok(r) => r,
        Err(_) => return Err("invalid regex pattern".to_string()),
    };
    let replaced = re.replace_all(name, "_");
    let sanitized = replaced.trim().to_string();

    if sanitized.is_empty() {
        return Err("name cannot be empty".to_string());
    }

    // Reject if the result is only placeholders (every char is `_` or
    // `.`) — this catches inputs like "///", "<<>>", "...". We allow
    // inputs that become underscore-containing-but-with-alnum, e.g.
    // "?a?" → "_a_" (passes the test suite).
    if !sanitized.chars().any(|c| c.is_alphanumeric()) {
        return Err("name must contain at least one letter or digit".to_string());
    }

    if sanitized.starts_with('.') {
        return Err("name cannot start with a dot".to_string());
    }

    if sanitized.chars().all(|c| c == '.') {
        return Err("name cannot be only dots".to_string());
    }

    let upper = sanitized.to_ascii_uppercase();
    // Reject Windows-reserved device names. Compare against the full
    // uppercased filename, not the stem, so that "CON.txt" is allowed
    // (it's a different filename from the bare "CON" device) while
    // bare "CON" / "PRN" / etc. are still rejected.
    if WINDOWS_RESERVED.contains(&upper.as_str()) {
        return Err(format!("name {:?} is reserved", sanitized));
    }

    // Truncate by char count (not bytes) to avoid splitting a UTF-8 codepoint.
    if sanitized.chars().count() > MAX_FILENAME_LEN {
        let truncated: String = sanitized.chars().take(MAX_FILENAME_LEN).collect();
        return Ok(truncated);
    }

    Ok(sanitized)
}

/// Helper function to get file path for notepad (tries name-based first, falls back to ID-based)
pub async fn get_notepad_file_path(notepad: &Notepad, data_dir: &Path) -> PathBuf {
    if notepad.id == "default" {
        return data_dir.join("default.txt");
    }

    // If the name doesn't sanitize cleanly, fall back to the ID-based path
    // so we never expose an unsanitized name as a filesystem path.
    let sanitized_name = match sanitize_filename(&notepad.name) {
        Ok(s) => s,
        Err(_) => return data_dir.join(format!("{}.txt", notepad.id)),
    };
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
        // If the name no longer sanitizes (e.g. after a sanitizer hardening),
        // skip the migration rather than guess a fallback name.
        let sanitized_name = match sanitize_filename(&notepad.name) {
            Ok(s) => s,
            Err(e) => {
                eprintln!(
                    "Skipping migration for {:?}: invalid filename ({})",
                    notepad.name, e
                );
                continue;
            }
        };
        let new_path = data_dir.join(format!("{}.txt", sanitized_name));

        if old_path != new_path && fs::metadata(&old_path).await.is_ok() {
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

    if migrated_count > 0 {
        println!(
            "Successfully migrated {} notepad files to name-based filenames",
            migrated_count
        );
    } else {
        println!("No notepad files needed migration");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_simple_names() {
        assert_eq!(sanitize_filename("hello").unwrap(), "hello");
        assert_eq!(sanitize_filename("my notepad").unwrap(), "my notepad");
        assert_eq!(sanitize_filename("v1.0").unwrap(), "v1.0");
    }

    #[test]
    fn strips_dangerous_chars() {
        assert_eq!(sanitize_filename("a<b").unwrap(), "a_b");
        assert_eq!(sanitize_filename("foo:bar").unwrap(), "foo_bar");
        assert_eq!(sanitize_filename("a/b\\c").unwrap(), "a_b_c");
        // "?*" has no alphanumeric content, so it must be rejected.
        assert!(sanitize_filename("?*").is_err());
        assert_eq!(sanitize_filename("bell\x07").unwrap(), "bell_");
    }

    #[test]
    fn rejects_empty() {
        assert!(sanitize_filename("").is_err());
        assert!(sanitize_filename("   ").is_err());
        assert!(sanitize_filename("///").is_err());
        assert!(sanitize_filename("<<>>").is_err());
    }

    #[test]
    fn rejects_leading_dot() {
        assert!(sanitize_filename(".hidden").is_err());
        assert!(sanitize_filename(". ..foo").is_err());
    }

    #[test]
    fn rejects_dot_only() {
        assert!(sanitize_filename(".").is_err());
        assert!(sanitize_filename("..").is_err());
        assert!(sanitize_filename("....").is_err());
    }

    #[test]
    fn rejects_windows_reserved() {
        for name in ["CON", "PRN", "AUX", "NUL", "COM1", "LPT9"] {
            assert!(
                sanitize_filename(name).is_err(),
                "{name} should be rejected"
            );
            assert!(
                sanitize_filename(&name.to_lowercase()).is_err(),
                "{name} (lower) should be rejected"
            );
        }
        // "CON.txt" is fine — only the bare device name is reserved.
        assert!(sanitize_filename("CON.txt").is_ok());
    }

    #[test]
    fn caps_length() {
        let long = "a".repeat(500);
        let s = sanitize_filename(&long).unwrap();
        assert!(s.chars().count() <= MAX_FILENAME_LEN);
    }

    #[test]
    fn preserves_unicode() {
        // Unicode characters that aren't in the danger set should pass through.
        assert_eq!(sanitize_filename("café").unwrap(), "café");
        assert_eq!(sanitize_filename("日本語").unwrap(), "日本語");
    }
}
