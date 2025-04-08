use chrono::DateTime;
use chrono::Utc;
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
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub last_message_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum T3ThreadStatus {
    Done,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct T3Message {
    pub id: String,
    #[serde(rename = "threadId")]
    pub thread_id: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
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
