// src/app.rs

use eframe::egui;
use eframe::egui::CollapsingHeader;
use eframe::egui::ScrollArea;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::time::Duration;
use tokio::runtime::Handle;
use tracing::info;
use tracing::warn;

use crate::charts::ChartState;
use crate::charts::ChartType;
use crate::t3_json::T3Json; // Import ChartState and ChartType

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
        chart_state: ChartState, // Add ChartState here
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
                // Initialize ChartState when T3Json is successfully parsed
                let chart_state = ChartState::new();
                return MyDroppedFile::T3Json {
                    file,
                    t3_json,
                    chart_state,
                };
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
            if self.dropped_files.is_empty() {
                ui.vertical(|ui| {
                    ui.heading("Drag a .json export from t3.chat here to get started");
                    ui.separator();
                    ui.label("Find the export option in t3.chat's settings.");
                    ui.label("Alternatively, in Chrome/Edge devtools (F12), find the 'Application' tab -> 'Storage' -> 'IndexedDB' -> 't3.chat' -> 'chat_db'. You can inspect the 'messages' and 'threads' object stores here. You can also use the side note in the README.md to copy the full export JSON to your clipboard.");
                });
            }
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

            // Fix: Use as_deref().unwrap_or().to_string() pattern
            let window_title = format!("Dropped File: {}", file.dropped_file().name);

            egui::Window::new(window_title)
                .id(Id::new(
                    file.dropped_file().path.clone().unwrap_or_default(),
                )) // Use path or default for a stable ID
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

fn draw_dropped_file(file: &mut MyDroppedFile, ui: &mut egui::Ui) {
    // file needs to be mutable
    match file {
        MyDroppedFile::T3Json {
            file,
            t3_json,
            chart_state,
        } => {
            ScrollArea::both().show(ui, |ui| {
                // Get mutable access to chart_state
                // Fix: Use as_deref() pattern for file_info
                let file_info = if let Some(path) = &file.path {
                    path.display().to_string()
                } else {
                    file.name.to_string() // Convert &str to String
                };

                ui.label(format!("File: {}", file_info));
                if let Some(bytes) = &file.bytes {
                    ui.label(format!("Size: {} bytes", bytes.len()));
                }
                ui.separator();
                ui.label("Parsed T3Json Content:");

                // Display Thread count
                ui.horizontal(|ui| {
                    ui.label("Threads:");
                    ui.monospace(format!("{}", t3_json.threads.len()));
                });
                // Display Message count
                ui.horizontal(|ui| {
                    ui.label("Messages:");
                    ui.monospace(format!("{}", t3_json.messages.len()));
                });

                ui.separator();
                CollapsingHeader::new("Charts")
                    .default_open(false)
                    .show(ui, |ui| {
                        // Chart Type Selection
                        ui.horizontal(|ui| {
                            ui.label("View:"); // Label next to the combo box
                            egui::ComboBox::new(
                                format!("chart_type_combo_{}", file_info), // Unique ID
                                chart_state.selected_chart.name(), // Text displayed in the combo box
                            )
                            .show_ui(ui, |ui| {
                                // Fix: Use show_ui
                                for chart_type in ChartType::all() {
                                    if ui
                                        .selectable_value(
                                            &mut chart_state.selected_chart,
                                            *chart_type,
                                            chart_type.name(),
                                        )
                                        .clicked()
                                    {
                                        // The draw function will pick up the new selected_chart on the next frame
                                        info!(
                                            "Selected chart type: {:?}",
                                            chart_state.selected_chart
                                        );
                                    }
                                }
                            });
                        });

                        // Draw the selected chart
                        ui.with_layout(
                            egui::Layout::top_down_justified(egui::Align::Center),
                            |ui| {
                                // Reserve space for the plot
                                let plot_height = 300.0; // Adjust height as needed
                                ui.set_min_height(plot_height);
                                chart_state.draw(ui, &t3_json.messages);
                            },
                        );
                    });

                ui.separator();
                ui.heading("Threads");
                // ui.monospace(format!("{:?}", t3_json)); // Display the parsed T3Json content - potentially large
                draw_t3_json_threads(t3_json, ui); // Use a dedicated function for threads
            });
        }
        MyDroppedFile::Unknown { file } => {
            // Fix: Use as_deref() pattern for file_info
            let file_info = if let Some(path) = &file.path {
                path.display().to_string()
            } else {
                file.name.to_string() // Convert &str to String
            };

            ui.label(format!("File: {}", file_info));
            ui.label(format!(
                "Type: {}",
                // Fix: Use as_deref() pattern
                file.mime
            ));
            if let Some(bytes) = &file.bytes {
                ui.label(format!("Size: {} bytes", bytes.len()));
            }
            ui.separator();
            ui.label("This file could not be parsed as T3Json.");
        }
    }
}

// Function to draw the threads part of the T3Json view
fn draw_t3_json_threads(t3_json: &T3Json, ui: &mut egui::Ui) {
    // Draw each thread as an expando
    // Use ScrollArea here to contain the threads list specifically
    ScrollArea::vertical().show(ui, |ui| {
        // vertical scroll for the threads list
        for thread in &t3_json.threads {
            // Use a simpler title if the actual title is too long for the header
            let display_title = if thread.title.len() > 80 {
                // Arbitrary length limit
                format!("{}...", &thread.title[..77])
            } else {
                thread.title.clone()
            };

            egui::CollapsingHeader::new(format!("Thread: {}", display_title))
                .default_open(false)
                .id_salt(thread.id.clone()) // Use thread ID for stable header state
                .show(ui, |ui| {
                    // Use an inner ScrollArea for messages within a thread if they are numerous
                    ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        // Example max height
                        ui.horizontal(|ui| {
                            // Add a "Copy" button
                            if ui.button("Copy Thread JSON").clicked() {
                                // Collect the thread and its associated messages
                                let thread_data = serde_json::json!({
                                    "thread": thread,
                                    "messages": t3_json.messages.iter()
                                        .filter(|m| m.thread_id == thread.id)
                                        .collect::<Vec<_>>(),
                                });

                                // Copy the JSON to the clipboard
                                if let Ok(json_string) = serde_json::to_string_pretty(&thread_data)
                                {
                                    ui.output_mut(|o| {
                                        o.commands.push(egui::OutputCommand::CopyText(json_string))
                                    });
                                } else {
                                    ui.label("Failed to serialize thread data.");
                                }
                            }

                            // Display the full thread title if truncated in the header
                            if thread.title.len() > 80 {
                                ui.label(format!("Full Title: {}", thread.title));
                            }
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
                        for message in t3_json.messages.iter().filter(|m| m.thread_id == thread.id)
                        {
                            ui.horizontal(|ui| {
                                ui.label(format!("Role: {:?}", message.role));
                                ui.label(format!(
                                    "Message: {}",
                                    // Truncate long messages for display
                                    if message.content.len() > 500 {
                                        // Increase truncation length slightly
                                        format!("{}...", &message.content[..500])
                                    } else {
                                        message.content.clone()
                                    }
                                ));
                            });
                            // Add a button to copy the individual message content
                            ui.horizontal(|ui| {
                                if ui.button("Copy Message Content").clicked() {
                                    ui.output_mut(|o| {
                                        o.commands.push(egui::OutputCommand::CopyText(
                                            message.content.clone(),
                                        ))
                                    });
                                }
                            });
                            ui.separator(); // Separator between messages
                        }
                    }); // End of inner ScrollArea for messages
                });
        }
    }); // End of outer ScrollArea for threads list
}
