use std::net::SocketAddrV4;
use std::path::PathBuf;
use crate::net_controller::NetController;
use eel_file::{AppState, FileInfo};
use eframe::egui;
use std::sync::{Arc, Mutex};
use chrono::Local;
use tokio::sync::mpsc::UnboundedReceiver;

pub struct Controller {
    net_controller: NetController,
    app_state: Arc<Mutex<AppState>>,
    ui_context: egui::Context,
    logger: Arc<Mutex<String>>,
}

impl Controller {
    pub fn new(ui_context: egui::Context, app_state: Arc<Mutex<AppState>>, logger: Arc<Mutex<String>>) -> Controller {
        // todo: lel fix this trash
        let mut c = Controller {
            net_controller: NetController::new(),
            ui_context,
            app_state,
            logger,
        };
        
        Self::temp_log(&mut c);
        
        c
    }
    
    fn temp_log(&mut self) {
        let now = Local::now();
        let mut formatted = now.format("%Y-%m-%d %H:%M:%S%.3f: ").to_string();
        formatted.push_str("Runtime initialized. Ready!\n");

        self.logger.lock().unwrap().push_str(&formatted);
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
        let app_state = self.app_state.lock().unwrap();
        
        match *app_state {
            AppState::Listening => { self.net_controller.abort_server() }
            AppState::Accepting | AppState::Sending => { self.net_controller.abort_task() }
            _ => {}
        }
    }
    
    
    fn listen_to_state(&mut self, mut rx: UnboundedReceiver<AppState>) {
        println!("Listener ENGAGED");
        let app_state = self.app_state.clone();
        let ui_context = self.ui_context.clone(); // Clone the context for thread

        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Could not build controller listen runtime");
            runtime.block_on(async move {
                while let Some(state) = rx.recv().await {
                    let mut app_state = app_state.lock().unwrap();

                    *app_state = state;
                    ui_context.request_repaint();
                }
            })
        });
    }
}

// todo: tests
