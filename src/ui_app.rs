use std::fs::File;
use crate::controller::Controller;
use eel_file::{AppState, EelFlags, FileInfo};
use eframe::egui;
use eframe::egui::{RichText, ScrollArea, TextEdit, Ui, ViewportCommand};
use rfd::FileDialog;
use std::net::{AddrParseError, Ipv4Addr, SocketAddrV4};
use std::num::ParseIntError;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration};
use eel_file::eel_log::EelWatcher;

pub struct UiApp {
    controller: Controller,
    file_info: Option<FileInfo>,
    selected_file_str: String,
    selected_file_path: Option<PathBuf>,
    receive_dir_str: String,
    receive_dir_path: Option<PathBuf>,
    receive_ip: Option<Ipv4Addr>,
    receive_ip_str: String,
    send_ip: Option<Ipv4Addr>,
    send_ip_str: String,
    password: String,
    port_send_str: String,
    port_recv_str: String,
    port_send: Option<u16>,
    port_recv: Option<u16>,
    progress: f32,
    status_message: String,
    logger: Arc<Mutex<EelWatcher>>,
    flags: EelFlags
}

impl eframe::App for UiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // handle user clicking X
            if ctx.input(|i| i.viewport().close_requested()) {
                if !self.flags.contains(EelFlags::allowed_to_close) {
                    ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                    self.flags.insert(EelFlags::shutting_down);
                }
            }

            if self.flags.contains(EelFlags::shutting_down) {
                egui::Window::new("Do you want to quit?")
                    .collapsible(false)
                    .fixed_pos(egui::Pos2::new(200.0, 200.0))
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("No").clicked() {
                                self.flags.remove(EelFlags::shutting_down);
                            }

                            if ui.button("Yes").clicked() {
                                self.flags.remove(EelFlags::shutting_down);
                                self.flags.insert(EelFlags::allowed_to_close);
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
    pub fn new(controller: Controller, logger: Arc<Mutex<EelWatcher>>) -> Self {
        Self {
            controller,
            selected_file_path: None,
            selected_file_str: String::new(),
            receive_dir_path: None,
            receive_dir_str: String::new(),
            receive_ip: None,
            send_ip_str: String::new(),
            receive_ip_str: String::new(),
            send_ip: None,
            password: String::new(),
            port_send_str: String::new(),
            port_recv_str: String::new(),
            port_send: None,
            port_recv: None,
            progress: 0.0,
            logger,
            status_message: "Selected file: N\\A, size (in bytes lol): N\\A".to_string(),
            file_info: None,
            flags: EelFlags::empty(),
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
                            self.flags.insert(EelFlags::file_valid);
                            let metadata = UiApp::generate_metadata(&file, self.selected_file_path.clone().unwrap());
                            self.status_message = format!("Selected file: {}, size (in bytes lol): {}", metadata.name, metadata.size).as_str().parse().unwrap();
                            self.file_info = Some(metadata);
                        }
                        Err(_) => {
                            let fmt_path = "The current file selection is not valid.".to_string();
                            ui.label(egui::RichText::new(fmt_path).color(egui::Color32::from_rgb(200, 10, 20)));
                            self.flags.remove(EelFlags::file_valid);
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
            self.logger.lock().unwrap().app_state
        );
        ui.label(egui::RichText::new(fmt_path).color(egui::Color32::from_rgb(200, 10, 20)));
        
        let send_button_enabled = {
            // hmmmmmm
            let valid_send_settings: EelFlags = EelFlags::file_valid | EelFlags::send_ip_valid | EelFlags::send_port_valid;
            self.idle_check() && self.flags.contains(valid_send_settings)
        };

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("Target IP:");
                let ip_textbox = ui.add_enabled(self.idle_check(), TextEdit::singleline(&mut self.send_ip_str));

                if ip_textbox.changed() {
                    // reparse the IP
                    match Self::check_ip(&self.send_ip_str) {
                        Ok(ip) => {
                            self.flags.insert(EelFlags::send_ip_valid);
                            self.send_ip = Some(ip);
                            println!("IP valid!");
                        }
                        Err(e) => {
                            self.flags.remove(EelFlags::send_ip_valid);
                            self.send_ip = None;
                        }
                    }
                }
            });

            ui.add_space(0.5);

            ui.vertical(|ui| {
                ui.label("Port");
                let send_port_field = ui.add_enabled(self.idle_check(),
                                                     TextEdit::singleline(&mut self.port_send_str).desired_width(50.0), // Make it narrower
                );

                self.port_send_str.retain(|c| c.is_digit(10));

                if send_port_field.changed() {
                    // reparse port
                    match Self::validate_port(self.port_send_str.as_str()) {
                        Ok(port) => {
                            self.flags.insert(EelFlags::send_port_valid);
                            self.port_send = Some(port);
                        }
                        Err(_) => {
                            self.flags.remove(EelFlags::send_port_valid);
                            self.port_send = None;
                        }
                    }
                }

            });

            ui.add_space(0.5);

            ui.vertical(|ui| {
                ui.label("Password:");
                ui.add_enabled(self.idle_check(), TextEdit::singleline(&mut self.password));
            });
        });
        
        ui.add_space(0.5);
        
        if ui.add_enabled(send_button_enabled, egui::Button::new("SEND")).clicked() {
            let socket = SocketAddrV4::new(self.send_ip.unwrap(), self.port_send.unwrap());
            self.controller.send(socket, self.file_info.clone().unwrap());
        }
    }

    fn draw_receiver_ui(&mut self, ui: &mut Ui) {
        
        ui.heading("Receive a file");
        
        ui.horizontal(|ui| {
            let mut resp = ui.text_edit_singleline(&mut self.receive_dir_str);

            if ui.button("Select folder").clicked() {
                self.receive_dir_path = FileDialog::new()
                    //.add_filter("text", &["txt", "rs"])
                    //.add_filter("rust", &["rs", "toml"])
                    .set_directory("/")
                    .pick_folder();

                if let Some(path) = &self.receive_dir_path {
                    let text_path = path.to_str().unwrap();
                    self.receive_dir_str = text_path.to_owned();
                    resp.mark_changed();
                }
            }
        });
        
        // todo: add reparsing on changed()!!!!!!!!

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                
                ui.label("Port");
                let listening_port_box = ui.add_enabled(
                    self.idle_check(),
                    TextEdit::singleline(&mut self.port_recv_str).desired_width(50.0),
                );

                if listening_port_box.changed() {
                    match Self::validate_port(self.port_recv_str.as_str()) {
                        Ok(port) => {
                            self.port_recv = Some(port);
                            self.flags.insert(EelFlags::receive_port_valid);
                        }
                        Err(_) => {
                            self.port_recv = None;
                            self.flags.remove(EelFlags::receive_port_valid);
                        }
                    }
                }
            });

            ui.add_space(0.5);

            ui.vertical(|ui| {
                ui.label("Password:");

                ui.add_enabled(
                    self.idle_check(),
                    // this is shit and awfully specific, if I weren't lazy, I'd do it with the layout
                    TextEdit::singleline(&mut self.password).desired_width(213.0),
                );
            });
        });

        ui.add_space(0.5);

        if ui.add_enabled(self.flags.contains(EelFlags::receive_port_valid), egui::Button::new("LISTEN")).clicked() {
            self.controller.listen(PathBuf::from("C:\\eelfile"), self.port_recv.unwrap());
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
        
        let mut log_text = self.logger.lock().unwrap().messages.clone();
        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.add(
                    TextEdit::multiline(&mut log_text)
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(10)
                        .desired_width(f32::INFINITY)
                        .interactive(false)
                );
            });
    }
    
    fn idle_check(&self) -> bool {
        if let AppState::Idle = self.logger.lock().unwrap().app_state {
            true
        } else {
            false
        }
    }

    fn validate_port(port: &str) -> Result<u16, ParseIntError> {
        match port.parse::<u16>() {
            Ok(port) => { Ok(port) },
            Err(e) => {Err(e)}
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

    fn check_ip(ip: &String) -> Result<Ipv4Addr, AddrParseError> {
        Ok(ip.parse::<Ipv4Addr>()?)
    }
}