use crate::app_state::AppState;
use eframe::egui;
use rfd::FileDialog;
use std::net::IpAddr;
use std::path::PathBuf;
use eframe::egui::Ui;

pub struct UiApp {
    current_state: AppState,
    current_ui_state: UiState,
    selected_file_str: String,
    selected_file_path: Option<PathBuf>,
    send_ip: Option<IpAddr>,
    receive_ip: Option<IpAddr>,
    send_ip_str: String,
    receive_ip_str: String,
    password: String,
    port_send: String,
    port_recv: String,
    progress: f32
}

impl Default for UiApp {
    fn default() -> Self {
        Self {
            current_state: AppState::new(),
            current_ui_state: UiState::Idle,
            selected_file_path: None,
            selected_file_str: String::new(),
            send_ip: None,
            receive_ip: None,
            send_ip_str: String::new(),
            receive_ip_str: String::new(),
            password: String::new(),
            port_send: String::new(),
            port_recv: String::new(),
            progress: 0.0,
        }
    }
}

enum UiState {
    Idle,
    Listening,
    Accepting,
    Sending,
    LeChatting
}

impl eframe::App for UiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.draw_sender_ui(ui);
            ui.separator();
            self.draw_receiver_ui(ui);
            ui.separator();
            self.draw_status_ui(ui);
            ui.allocate_space(ui.available_size());
        });
    }
}

impl UiApp {
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
            self.current_ui_state = UiState::Idle;
        }
    }

    fn draw_receiver_ui(&mut self, ui: &mut Ui) {
        ui.heading("Receive a file");

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("Port");
                ui.add(
                    egui::TextEdit::singleline(&mut self.port_recv).desired_width(50.0), // Make it narrower
                );
            });

            ui.add_space(0.5);

            ui.vertical(|ui| {
                ui.label("Password:");
                ui.text_edit_singleline(&mut self.password);
            });
        });

        ui.add_space(0.5);
        if ui.button("LISTEN").clicked() {
            // todo
        }
    }

    fn draw_status_ui(&mut self, ui: &mut Ui) {
        ui.heading("Status");

        ui.label("File metadata: N/A");

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
