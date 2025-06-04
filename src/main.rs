#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod controller;
mod net_controller;
mod ui_app;

use crate::controller::Controller;
use eframe::egui;
use std::sync::{Arc, Mutex};
use eel_file::eel_log::EelWatcher;

fn main() -> eframe::Result {
    let options = get_options();

    eframe::run_native(
        "EELFILE™ v0.1.6",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);
            
            let watcher = Arc::new(Mutex::new(EelWatcher::new()));
            
            watcher.lock().unwrap().log("Welcome to EELFILE™ 🐍");
            
            let controller = Controller::new(cc.egui_ctx.clone(), watcher.clone());
            let ui_frame = ui_app::UiApp::new(controller, watcher.clone());

            Ok(Box::new(ui_frame))
        }),
    )
}

fn get_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 520.0])
            .with_icon(Arc::new(egui::IconData {
                rgba: image::load_from_memory(include_bytes!("../assets/snek.png"))
                    .unwrap()
                    .to_rgba8()
                    .to_vec(),
                width: 512,
                height: 512,
            }))
            .with_resizable(false).with_maximize_button(false),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    }
}
