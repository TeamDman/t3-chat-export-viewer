use eframe::egui;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::time::Duration;
use tokio::runtime::Handle;
use tracing::info;

use crate::t3_json::T3Json;

pub enum UiBoundMessage {
    ContentLoaded(MyDroppedFile),
}

pub struct MyApp {
    // For incremental indexing:
    tx: Sender<UiBoundMessage>,
    rx: Receiver<UiBoundMessage>,

    // We'll hold a handle to the runtime so we can spawn tasks.
    rt_handle: Handle,

    dropped_files: Vec<MyDroppedFile>,
}

pub enum MyDroppedFile {
    T3Json {
        file: egui::DroppedFile,
        t3_json: T3Json,
    },
    Unknown {
        file: egui::DroppedFile,
    },
}
impl MyDroppedFile {
    pub async fn from_async(file: egui::DroppedFile) -> Self {
        info!(
            "Attempting to parse T3Json from dropped file: {:?}",
            file.name
        );
        if let Ok(t3_json) = T3Json::try_from_async(file.clone()).await {
            return MyDroppedFile::T3Json { file, t3_json };
        }
        MyDroppedFile::Unknown { file }
    }
    pub fn dropped_file(&self) -> &egui::DroppedFile {
        match self {
            MyDroppedFile::T3Json { file, .. } => file,
            MyDroppedFile::Unknown { file } => file,
        }
    }
}

impl MyApp {
    pub fn new(rt_handle: Handle) -> Self {
        let (tx, rx) = mpsc::channel();

        Self {
            tx,
            rx,
            rt_handle,
            dropped_files: vec![],
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(100));
        // 1) Read from channel
        let mut new_messages = vec![];
        while let Ok(msg) = self.rx.try_recv() {
            new_messages.push(msg);
        }

        // 2) Apply them
        for msg in new_messages {
            match msg {
                UiBoundMessage::ContentLoaded(file) => self.dropped_files.push(file),
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.heading("T3 Chat Export Viewer");
                ui.separator();
            });
        });

        self.ui_file_drag_and_drop(ctx);
    }
}

impl MyApp {
    fn ui_file_drag_and_drop(&mut self, ctx: &egui::Context) {
        use egui::Align2;
        use egui::Color32;
        use egui::Id;
        use egui::LayerId;
        use egui::Order;
        use egui::TextStyle;
        use std::fmt::Write as _;

        // Preview hovering files:
        if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
            let text = ctx.input(|i| {
                let mut text = "Dropping files:\n".to_owned();
                for file in &i.raw.hovered_files {
                    if let Some(path) = &file.path {
                        write!(text, "\n{}", path.display()).ok();
                    } else if !file.mime.is_empty() {
                        write!(text, "\n{}", file.mime).ok();
                    } else {
                        text += "\n???";
                    }
                }
                text
            });

            let painter =
                ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

            let screen_rect = ctx.screen_rect();
            painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
            painter.text(
                screen_rect.center(),
                Align2::CENTER_CENTER,
                text,
                TextStyle::Heading.resolve(&ctx.style()),
                Color32::WHITE,
            );
        }

        // Collect dropped files:
        let tx = self.tx.clone();
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        if !dropped_files.is_empty() {
            info!("Dropped files: {:?}", dropped_files);
            self.rt_handle.spawn(async move {
                for file in dropped_files {
                    info!("File dropped: {:#?}", file.name);
                    tx.send(UiBoundMessage::ContentLoaded(
                        MyDroppedFile::from_async(file).await,
                    ))
                    .ok();
                }
            });
        }

        // Show dropped files (if any):
        if !self.dropped_files.is_empty() {
            let mut open = true;
            egui::Window::new("Dropped files")
                .open(&mut open)
                .show(ctx, |ui| {
                    for file in &self.dropped_files {
                        let file = file.dropped_file();
                        let mut info = if let Some(path) = &file.path {
                            path.display().to_string()
                        } else if !file.name.is_empty() {
                            file.name.clone()
                        } else {
                            "???".to_owned()
                        };

                        let mut additional_info = vec![];
                        if !file.mime.is_empty() {
                            additional_info.push(format!("type: {}", file.mime));
                        }
                        if let Some(bytes) = &file.bytes {
                            additional_info.push(format!("{} bytes", bytes.len()));
                        }
                        if !additional_info.is_empty() {
                            info += &format!(" ({})", additional_info.join(", "));
                        }

                        ui.label(info);
                    }
                });
            if !open {
                self.dropped_files.clear();
            }
        }
    }
}
