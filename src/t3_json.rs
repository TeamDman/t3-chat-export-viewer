use crate::t3_timestamp::T3Timestamp;
use eframe::egui::DroppedFile;
use eyre::bail;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct T3Json {
    pub threads: Vec<T3Thread>,
    pub messages: Vec<T3Message>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct T3Thread {
    pub title: String,
    pub user_edited_title: bool,
    pub status: T3ThreadStatus,
    pub model: String,
    pub id: String,
    pub created_at: T3Timestamp,
    pub updated_at: Option<T3Timestamp>,
    pub last_message_at: T3Timestamp,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum T3ThreadStatus {
    Done,
    Completed,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct T3Message {
    pub id: String,
    #[serde(rename = "threadId")]
    pub thread_id: String,
    pub content: String,
    pub created_at: T3Timestamp,
    pub role: T3MessageRole,
    pub status: T3MessageStatus,
    pub model: String,
    #[serde(rename = "modelParams")]
    pub model_params: Option<Value>,
    pub attachments: Option<Vec<Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum T3MessageRole {
    User,
    Assistant,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum T3MessageStatus {
    Done,
    Deleted,
    Error,
    Cancelled,
    Waiting,
    Streaming,
    Thinking,
}

impl T3Json {
    pub async fn try_from_async(dropped_file: DroppedFile) -> eyre::Result<Self> {
        info!(
            "Attempting to parse T3Json from dropped file: {:?}",
            dropped_file.path
        );
        let bytes;
        let bytes = match dropped_file.bytes {
            Some(ref bytes) => bytes.as_ref(),
            None => {
                let Some(path) = dropped_file.path else {
                    bail!("Dropped file has no bytes or path");
                };
                bytes = tokio::fs::read(path).await?;
                info!("Read bytes from file: {:?}", bytes.len());
                &bytes
            }
        };

        let t3_json: T3Json = serde_json::from_slice(bytes)
            .map_err(|e| eyre::eyre!("Failed to parse JSON string to T3Json: {:#?}", e))?;
        info!("Parsed T3Json: {:?}", t3_json.threads.len());
        Ok(t3_json)
    }
}

#[cfg(test)]
mod test {
    use std::path::Path;

    #[test]
    fn newest_file_works() -> eyre::Result<()> {
        let t3_backup_path = Path::new(r#"C:\Users\TeamD\OneDrive\Documents\Backups\t3chat"#);
        let newest_file_in_backup_path = std::fs::read_dir(t3_backup_path)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .map_or(false, |ext| ext == "json")
            })
            .max_by_key(|entry| entry.metadata().and_then(|m| m.modified()).ok())
            .map(|entry| entry.path())
            .ok_or_else(|| eyre::eyre!("No JSON files found in backup path"))?;
        let file_bytes = std::fs::read(&newest_file_in_backup_path)?;
        let t3_json: super::T3Json = serde_json::from_slice(&file_bytes)
            .map_err(|e| eyre::eyre!("Failed to parse JSON string to T3Json: {:#?}", e))?;
        println!("Parsed T3Json: {:?}", t3_json.threads.len());
        Ok(())
    }

    #[test]
    fn all_files_work() -> eyre::Result<()> {
        // for each json in the t3 backup path, ensure it can be parsed
        let t3_backup_path = Path::new(r#"C:\Users\TeamD\OneDrive\Documents\Backups\t3chat"#);
        let mut failures = Vec::new();
        for entry in std::fs::read_dir(t3_backup_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                let file_bytes = std::fs::read(&path)?;
                match serde_json::from_slice::<super::T3Json>(&file_bytes) {
                    Ok(t3_json) => {
                        println!(
                            "Parsed T3Json from file {:?}: {:?} threads",
                            path,
                            t3_json.threads.len()
                        );
                    }
                    Err(e) => {
                        failures.push(format!("Failed to parse {:?}: {:#?}", path, e));
                    }
                }
            }
        }
        if !failures.is_empty() {
            for failure in &failures {
                println!("{}", failure);
            }
            eyre::bail!("Failed to parse {} files", failures.len());
        }
        Ok(())
    }
}