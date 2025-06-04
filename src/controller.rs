use crate::net_controller::NetController;
use eel_file::eel_log::EelWatcher;
use eel_file::{AppEvent, AppState, FileInfo};
use eframe::egui;
use std::net::SocketAddrV4;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedReceiver;

pub struct Controller {
    net_controller: NetController,
    ui_context: egui::Context,
    watcher: Arc<Mutex<EelWatcher>>,
}

impl Controller {
    pub fn new(ui_context: egui::Context, logger: Arc<Mutex<EelWatcher>>) -> Controller {
        // todo: lel fix this trash
        Controller {
            net_controller: NetController::new(),
            ui_context,
            watcher: logger,
        }
    }

    pub fn listen(&mut self, path: PathBuf, port: u16) {
        // todo: add None check
        let task_receiver = self
            .net_controller
            .start(super::net_controller::NetCommand::Receive(path, port));

        self.listen_to_state(task_receiver);
    }

    pub fn send(&mut self, addr: SocketAddrV4, file_info: FileInfo) {
        let task_receiver = self
            .net_controller
            .start(super::net_controller::NetCommand::Send(addr, file_info));

        self.listen_to_state(task_receiver);
    }

    pub fn abort(&mut self) {
        match self.watcher.lock().unwrap().app_state {
            AppState::Listening => self.net_controller.abort_server(),
            AppState::Accepting | AppState::Sending | AppState::Connecting | AppState::Handshake => self.net_controller.abort_task(),
            _ => {}
        }
    }

    fn listen_to_state(&mut self, mut rx: UnboundedReceiver<AppEvent>) {
        let watcher = self.watcher.clone();
        let ui_context = self.ui_context.clone(); // Clone the context for thread

        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Could not build controller listen runtime");
            runtime.block_on(async move {
                while let Some(event_msg) = rx.recv().await {
                    match event_msg {
                        AppEvent::AppState(state) => {
                            watcher.lock().unwrap().app_state = state;
                        }
                        
                        AppEvent::FileInfo(metadata) => {
                            watcher.lock().unwrap().metadata = Some(metadata);
                        }
                        
                        AppEvent::Progress(progress) => {
                            watcher.lock().unwrap().progress = progress;
                        }
                        AppEvent::StatusMessage(loggie) => {
                            watcher.lock().unwrap().log(&loggie);
                        }
                        
                        AppEvent::Error(err) => {
                            watcher.lock().unwrap().log(format!("ERROR: {}", err).as_str());
                        }
                    }

                    ui_context.request_repaint();
                }
            })
        });
    }
}