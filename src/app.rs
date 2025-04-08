use eframe::egui;
use eframe::egui::ScrollArea;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::time::Duration;
use tokio::runtime::Handle;
use tracing::info;
use tracing::warn;

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
            file.path
        );
        match T3Json::try_from_async(file.clone()).await {
            Ok(t3_json) => {
                info!("Parsed T3Json successfully");
                return MyDroppedFile::T3Json { file, t3_json };
            }
            Err(e) => {
                warn!("Failed to parse T3Json {:?}: {:#?}", file.path, e);
            }
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

        // Show each dropped file in its own window:
        let mut indices_to_remove = vec![];
        for (index, file) in self.dropped_files.iter_mut().enumerate() {
            let mut open = true;

            egui::Window::new(format!("Dropped File: {}", file.dropped_file().name))
                .id(Id::new(file.dropped_file().path.clone()))
                .open(&mut open)
                .show(ctx, |ui| {
                    draw_dropped_file(file, ui);
                });

            // If the window is closed, mark the file for removal
            if !open {
                indices_to_remove.push(index);
            }
        }

        // Remove closed files
        for index in indices_to_remove.into_iter().rev() {
            self.dropped_files.remove(index);
        }
    }
}

fn draw_dropped_file(file: &MyDroppedFile, ui: &mut egui::Ui) {
    match file {
        MyDroppedFile::T3Json { file, t3_json } => {
            let file_info = if let Some(path) = &file.path {
                path.display().to_string()
            } else if !file.name.is_empty() {
                file.name.clone()
            } else {
                "???".to_owned()
            };

            ui.label(format!("File: {}", file_info));
            ui.label(format!("Type: {}", file.mime));
            if let Some(bytes) = &file.bytes {
                ui.label(format!("Size: {} bytes", bytes.len()));
            }
            ui.separator();
            ui.label("Parsed T3Json Content:");
            // ui.monospace(format!("{:?}", t3_json)); // Display the parsed T3Json content
            draw_t3_json(t3_json, ui);
        }
        MyDroppedFile::Unknown { file } => {
            let file_info = if let Some(path) = &file.path {
                path.display().to_string()
            } else if !file.name.is_empty() {
                file.name.clone()
            } else {
                "???".to_owned()
            };

            ui.label(format!("File: {}", file_info));
            ui.label(format!("Type: {}", file.mime));
            if let Some(bytes) = &file.bytes {
                ui.label(format!("Size: {} bytes", bytes.len()));
            }
            ui.separator();
            ui.label("This file could not be parsed as T3Json.");
        }
    }
}

fn draw_t3_json(t3_json: &T3Json, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label("Threads:");
        ui.monospace(format!("{:?}", t3_json.threads.len()));
    });

    // Draw each thread as an expando
    ScrollArea::both().show(ui, |ui| {
        for thread in &t3_json.threads {
            egui::CollapsingHeader::new(format!(
                "Thread: {}",
                thread.title.lines().next().unwrap_or(&thread.id)
            ))
            .default_open(false)
            .id_salt(thread.id.clone())
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Add a "Copy" button
                    if ui.button("Copy").clicked() {
                        // Collect the thread and its associated messages
                        let thread_data = serde_json::json!({
                            "thread": thread,
                            "messages": t3_json.messages.iter()
                                .filter(|m| m.thread_id == thread.id)
                                .collect::<Vec<_>>(),
                        });

                        // Copy the JSON to the clipboard
                        if let Ok(json_string) = serde_json::to_string_pretty(&thread_data) {
                            ui.output_mut(|o| {
                                o.commands.push(egui::OutputCommand::CopyText(json_string))
                            });
                        }
                    }

                    // Display the thread title
                    ui.label(format!("Thread: {}", thread.title));
                });

                ui.label(format!("Thread ID: {}", thread.id));
                ui.label(format!("Created At: {}", thread.created_at));
                if let Some(updated_at) = thread.updated_at {
                    ui.label(format!("Updated At: {}", updated_at));
                }
                ui.label(format!("Last Message At: {}", thread.last_message_at));
                ui.label(format!("Status: {:?}", thread.status));
                ui.separator();

                // Show messages within the thread
                for message in t3_json.messages.iter().filter(|m| m.thread_id == thread.id) {
                    ui.horizontal(|ui| {
                        ui.label(format!("Role: {:?}", message.role));
                        ui.label(format!(
                            "Message: {}",
                            if message.content.len() > 256 {
                                format!("{}...", &message.content[..256])
                            } else {
                                message.content.clone()
                            }
                        ));
                    });
                    ui.separator();
                }
            });
        }
    });
}
