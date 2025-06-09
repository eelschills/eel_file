#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod controller;
mod net_controller;
mod ui_app;
mod normal_facts;
mod sus_facts;
mod insanity_facts;
mod amogus_facts;

use crate::controller::Controller;
use eframe::egui;
use std::sync::{Arc, Mutex};
use eel_file::eel_log::EelWatcher;
use rand::prelude::*;
use rand::rng;
use crate::amogus_facts::AMOGUS_FACTS;
use crate::insanity_facts::INSANITY_FACTS;
use crate::normal_facts::NORMAL_FACTS;
use crate::sus_facts::SUS_FACTS;

fn main() -> eframe::Result {
    let options = get_options();

    eframe::run_native(
        "EELFILE™ v0.9.0",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);
            
            let watcher = Arc::new(Mutex::new(EelWatcher::new()));
            
            watcher.lock().unwrap().log("Welcome to EELFILE™ 🐍");
            watcher.lock().unwrap().log("Here is a random eel fact:");
            watcher.lock().unwrap().log(display_eelfact());
            
            let controller = Controller::new(cc.egui_ctx.clone(), watcher.clone());
            let ui_frame = ui_app::UiApp::new(controller, watcher.clone());

            Ok(Box::new(ui_frame))
        }),
    )
}

fn get_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 530.0])
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

fn display_eelfact() -> &'static str {
    let mut rng = rng();
    let roll: u8 = rng.random_range(0..100);

    match roll {
        0..=49 => NORMAL_FACTS.choose(&mut rng).unwrap(),
        50..=89 => SUS_FACTS.choose(&mut rng).unwrap(),
        90..=98 => AMOGUS_FACTS.choose(&mut rng).unwrap(),
        _ => INSANITY_FACTS.choose(&mut rng).unwrap()
    }
}