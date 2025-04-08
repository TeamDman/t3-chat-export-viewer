// src/main.rs

mod app;
mod init;
pub mod t3_json;

use app::MyApp;
use eframe::egui;
use eyre::Result;
use tracing::{error, info};
use std::time::Duration;
use tokio::runtime::Runtime;

fn main() -> Result<()> {
    init::init()?;
    info!("Ahoy!");

    // 1) Create a Tokio runtime
    let rt = Runtime::new()?;

    // 2) Keep the runtime alive in a separate thread:
    std::thread::spawn({
        let rt_handle = rt.handle().clone();
        // install panic hook to ensure the runtime is dropped on panic
        let panic_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            error!("Panic in the runtime thread: {:#?}", info);
            panic_hook(info);
        }));
        info!("Starting a thread to keep the runtime alive.");
        move || {
            // block_on a never-ending future
            rt_handle.block_on(async {
                loop {
                    tokio::time::sleep(Duration::from_secs(3600)).await;
                }
            });
        }
    });

    // 3) Pass the runtime HANDLE (not the entire runtime) into our MyApp.
    let app = MyApp::new(rt.handle().clone());

    // 4) Launch eframe:
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 550.0]),
        ..Default::default()
    };

    eframe::run_native(
        "T3 Chat Export Viewer",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
    .map_err(|err| eyre::eyre!("Failed to run eframe: {}", err))?;

    Ok(())
}
