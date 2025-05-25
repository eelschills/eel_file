use crate::controller::Controller;
use eel_file::AppState;
use eframe::egui;
use eframe::egui::{Ui, ViewportCommand};
use rfd::FileDialog;
use std::net::IpAddr;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

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
    message: String,
    shutting_down: bool,
    allowed_to_close: bool,
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
            message: String::new(),
            shutting_down: false,
            allowed_to_close: false,
        }
    }

    fn draw_sender_ui(&mut self, ui: &mut Ui) {
        ui.heading("Send a file");

        ui.label("\nFile to send:");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.selected_file_str);

            if ui.button("Select file").clicked() {
                self.selected_file_path = FileDialog::new()
                    //.add_filter("text", &["txt", "rs"])
                    //.add_filter("rust", &["rs", "toml"])
                    .set_directory("/")
                    .pick_file();

                if let Some(path) = &self.selected_file_path {
                    let text_path = path.to_str().unwrap();
                    self.selected_file_str = text_path.to_owned();
                }
            }
        });

        // needs to update the path buffer from the typed line if there's been any manual changes
        self.selected_file_path = Some(PathBuf::from(&self.selected_file_str.clone()));

        if let Some(path) = &self.selected_file_path {
            let fmt_path = format!("DEBUG: The current actual path buffer: {}", path.display());
            ui.label(egui::RichText::new(fmt_path).color(egui::Color32::from_rgb(200, 10, 20)));
        }

        let fmt_path = format!(
            "DEBUG: Current app state: {}",
            self.app_state.lock().unwrap()
        );
        ui.label(egui::RichText::new(fmt_path).color(egui::Color32::from_rgb(200, 10, 20)));

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("Target IP:");
                ui.text_edit_singleline(&mut self.send_ip_str);
            });

            ui.add_space(0.5);

            ui.vertical(|ui| {
                ui.label("Port");
                ui.add(
                    egui::TextEdit::singleline(&mut self.port_send).desired_width(50.0), // Make it narrower
                );
            });

            ui.add_space(0.5);

            ui.vertical(|ui| {
                ui.label("Password:");
                ui.text_edit_singleline(&mut self.password);
            });
        });
        ui.add_space(0.5);
        if ui.button("SNEED").clicked() {
            // testing file info
            self.controller.send();
        }
    }

    fn draw_receiver_ui(&mut self, ui: &mut Ui) {
        ui.heading("Receive a file");

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("Port");

                match self.app_state.lock().unwrap().deref() {
                    AppState::Idle | AppState::Listening => {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.port_recv).desired_width(50.0), // Make it narrower
                        );
                    }
                    _ => {
                        ui.add_enabled(
                            false,
                            egui::TextEdit::singleline(&mut self.port_recv).desired_width(50.0),
                        );
                    }
                }
            });

            ui.add_space(0.5);

            ui.vertical(|ui| {
                ui.label("Password:");
                ui.text_edit_singleline(&mut self.password);
            });
        });

        ui.add_space(0.5);
        if ui.button("LISTEN").clicked() {
            // todo: ask for result before swapping state
            self.controller.listen();
        }
    }

    fn draw_status_ui(&mut self, ui: &mut Ui) {
        ui.heading("Status");

        ui.label("File metadata: N/A");

        /* if let AppState::Listening = self.ui_state {
            match self.controller.poll() {
                Ok(appstate) => self.message = msg,
                Err(EelError::Poll(_)) => {
                    panic!("Trying to poll a non-existing worker!")
                }
                // this can only be empty stream, do nothing
                Err(msg) => {}
            }

            ui.label(format!("Received message: {}", self.message));
        } */

        ui.add(egui::ProgressBar::new(self.progress));

        self.progress += 0.001;
        self.progress = self.progress % 1f32;

        ui.horizontal(|ui| {
            ui.label(format!("Progress: {}%", (self.progress * 100.0).round()));
            if ui.button("SOTP").clicked() {
                println!("SOTP");
            }
        });
    }
}
