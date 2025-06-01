use std::fs::File;
use crate::controller::Controller;
use eel_file::{AppState, FileInfo};
use eframe::egui;
use eframe::egui::{ScrollArea, TextEdit, Ui, ViewportCommand};
use rfd::FileDialog;
use std::net::{IpAddr, Ipv4Addr, SocketAddrV4};
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use chrono::Local;

pub struct UiApp {
    controller: Controller,
    app_state: Arc<Mutex<AppState>>,
    selected_file_str: String,
    selected_file_path: Option<PathBuf>,
    receive_ip: Option<IpAddr>,
    send_ip_str: String,
    receive_ip_str: String,
    password: String,
    port_send: String,
    port_recv: String,
    progress: f32,
    status_message: String,
    shutting_down: bool,
    allowed_to_close: bool,
    file_valid_flag: bool,
    file_info: Option<FileInfo>,
}

impl eframe::App for UiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // handle user clicking X
            if ctx.input(|i| i.viewport().close_requested()) {
                if !self.allowed_to_close {
                    ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                    self.shutting_down = true;
                }
            }

            if self.shutting_down {
                egui::Window::new("Do you want to quit?")
                    .collapsible(false)
                    .resizable(false)
                    .fixed_size([3000.0, 1000.0])
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("No").clicked() {
                                self.shutting_down = false;
                            }

                            if ui.button("Yes").clicked() {
                                self.shutting_down = false;
                                self.allowed_to_close = true;
                                ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                            }
                        });
                    });
            }

            self.draw_sender_ui(ui);
            ui.separator();
            self.draw_receiver_ui(ui);
            ui.separator();
            self.draw_status_ui(ui);
            ui.allocate_space(ui.available_size());
            ctx.request_repaint_after(Duration::from_secs(1));
        });
    }
}

impl UiApp {
    pub fn new(controller: Controller, app_state: Arc<Mutex<AppState>>) -> Self {
        Self {
            controller,
            app_state,
            selected_file_path: None,
            selected_file_str: String::new(),
            receive_ip: None,
            send_ip_str: String::new(),
            receive_ip_str: String::new(),
            password: String::new(),
            port_send: String::new(),
            port_recv: String::new(),
            progress: 0.0,
            status_message: "Selected file: N\\A, size (in bytes lol): N\\A".to_string(),
            shutting_down: false,
            allowed_to_close: false,
            file_valid_flag: false,
            file_info: None,
        }
    }

    fn draw_sender_ui(&mut self, ui: &mut Ui) {
        ui.heading("Send a file");
        ui.label("\nFile to send:");
        ui.horizontal(|ui| {
            let mut resp = ui.text_edit_singleline(&mut self.selected_file_str);

            if ui.button("Select file").clicked() {
                self.selected_file_path = FileDialog::new()
                    //.add_filter("text", &["txt", "rs"])
                    //.add_filter("rust", &["rs", "toml"])
                    .set_directory("/")
                    .pick_file();
                
                if let Some(path) = &self.selected_file_path {
                    let text_path = path.to_str().unwrap();
                    self.selected_file_str = text_path.to_owned();
                    resp.mark_changed();
                }
            }

            if resp.changed() {
                self.selected_file_path = Some(PathBuf::from(&self.selected_file_str.clone()));

                if let Some(path) = &self.selected_file_path {
                    let file = File::open(path);

                    match file {
                        Ok(file) => {
                            self.file_valid_flag = true;
                            let metadata = UiApp::generate_metadata(&file, self.selected_file_path.clone().unwrap());
                            self.status_message = format!("Selected file: {}, size (in bytes lol): {}", metadata.name, metadata.size).as_str().parse().unwrap();
                            self.file_info = Some(metadata);
                        }
                        Err(_) => {
                            let fmt_path = "The current file selection is not valid.".to_string();
                            ui.label(egui::RichText::new(fmt_path).color(egui::Color32::from_rgb(200, 10, 20)));
                            self.file_valid_flag = false;
                            self.status_message = "Selected file: N\\A, size (in bytes lol): N\\A".to_string();
                            self.file_info = None;
                        }
                    }
                }
            }
        });

        // this needs to update the path buffer from the typed line if there's been any manual changes
        let fmt_path = format!(
            "DEBUG: Current app state: {}",
            self.app_state.lock().unwrap()
        );
        ui.label(egui::RichText::new(fmt_path).color(egui::Color32::from_rgb(200, 10, 20)));
        
        let enabled = {
            self.idle_check() && self.file_valid_flag
        };

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("Target IP:");
                
                ui.add_enabled(enabled, egui::TextEdit::singleline(&mut self.send_ip_str));
            });

            ui.add_space(0.5);

            ui.vertical(|ui| {
                ui.label("Port");
                ui.add_enabled(enabled,
                    egui::TextEdit::singleline(&mut self.port_send).desired_width(50.0), // Make it narrower
                );
            });

            ui.add_space(0.5);

            ui.vertical(|ui| {
                ui.label("Password:");
                ui.add_enabled(enabled, egui::TextEdit::singleline(&mut self.password));
            });
        });
        
        ui.add_space(0.5);
        
        if ui.add_enabled(enabled, egui::Button::new("SEND")).clicked() {
            // todo: add checks to see that we have all this info before we show the button
            let addr = Ipv4Addr::new(127, 0, 0, 1);
            let socket = SocketAddrV4::new(addr, 7878);
            self.controller.send(socket, self.file_info.clone().unwrap());
        }
    }

    fn draw_receiver_ui(&mut self, ui: &mut Ui) {
        let enabled = self.idle_check();
        
        ui.heading("Receive a file");

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                
                ui.label("Port");
                ui.add_enabled(
                    enabled,
                    egui::TextEdit::singleline(&mut self.port_recv).desired_width(50.0),
                );
                
            });

            ui.add_space(0.5);

            ui.vertical(|ui| {
                ui.label("Password:");

                ui.add_enabled(
                    enabled,
                    egui::TextEdit::singleline(&mut self.password),
                );
            });
        });

        ui.add_space(0.5);
        
        if ui.add_enabled(enabled, egui::Button::new("LISTEN")).clicked() {
            self.controller.listen(PathBuf::from("C:\\eelfile"), 7878);
        }

    }

    fn draw_status_ui(&mut self, ui: &mut Ui) {
        ui.heading("Status");

        ui.label(format!("{}", self.status_message));

        ui.add(egui::ProgressBar::new(self.progress));

        ui.horizontal(|ui| {
            ui.label(format!("Progress: {}%", (self.progress * 100.0).round()));
            if ui.button("SOTP").clicked() {
                self.controller.abort();
            }
        });

        let mut now = Local::now().to_string();

        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.add(
                    TextEdit::multiline(&mut now)
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(10)
                        .lock_focus(true)
                        .desired_width(f32::INFINITY)
                        .interactive(false),
                );
            });
    }
    
    fn idle_check(&self) -> bool {
        if let AppState::Idle = self.app_state.lock().unwrap().deref() {
            true
        } else {
            false
        }
    }

    fn generate_metadata(file: &File, path: PathBuf) -> FileInfo {
        // name, extension, size, hash
        let metadata = file.metadata().unwrap();

        let size = metadata.len();
        // I'll leave the hashing to the worker thread, there's no point doing this work here
        let name = path.file_name().unwrap().to_str().unwrap().to_owned();

        FileInfo {
            path: Some(path.clone()),
            size,
            hash: None,
            name,
            sender_addr: None,
        }
    }
}
