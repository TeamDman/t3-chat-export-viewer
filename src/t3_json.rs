use eframe::egui::DroppedFile;
use serde::Deserialize;
use serde::Serialize;
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct T3Json {}

impl T3Json {
    pub async fn try_from_async(dropped_file: DroppedFile) -> eyre::Result<Self> {
        info!("Attempting to parse T3Json from dropped file: {:?}", dropped_file.name);
        Ok(T3Json {})
    }
}
