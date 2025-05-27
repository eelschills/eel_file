#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod controller;
mod eel_error;
mod file_manager;
mod net_controller;
mod ui_app;

use crate::controller::Controller;
use eel_file::AppState;
use eframe::egui;
use std::sync::{Arc, Mutex};

fn main() -> eframe::Result {
    let options = get_options();

    eframe::run_native(
        "EELFILE™ v0.1.5",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            let app_state = Arc::new(Mutex::new(AppState::Idle));
            let controller = Controller::new(cc.egui_ctx.clone(), app_state.clone());
            let ui_frame = ui_app::UiApp::new(controller, app_state.clone());

            Ok(Box::new(ui_frame))
        }),
    )
}

fn get_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 370.0])
            .with_icon(Arc::new(egui::IconData {
                rgba: image::load_from_memory(include_bytes!("../assets/snek.png"))
                    .unwrap()
                    .to_rgba8()
                    .to_vec(),
                width: 512,
                height: 512,
            }))
            .with_resizable(false),
        ..Default::default()
    }
}
