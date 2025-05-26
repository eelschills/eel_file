use crate::eel_error::EelError;
use crate::net_controller::NetController;
use eel_file::{AppState, FileInfo};
use eframe::egui;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::watch::Sender;

pub struct Controller {
    net_controller: NetController,
    shutdown_tx: Option<Sender<bool>>,
    app_state: Arc<Mutex<AppState>>,
    ui_context: egui::Context,
}

impl Controller {
    pub fn new(ui_context: egui::Context, app_state: Arc<Mutex<AppState>>) -> Controller {
        Controller {
            net_controller: NetController::new(),
            shutdown_tx: None,
            ui_context,
            app_state,
        }
    }

    pub fn listen(&mut self) {
        // todo: add None check
        let (task_receiver, shutdown_tx) = self
            .net_controller
            .start(7878, super::net_controller::NetCommand::Receive);
        
        self.shutdown_tx = Some(shutdown_tx);
        
        self.listen_to_state(task_receiver);
    }

    pub fn send(&mut self) {
        let (task_receiver, shutdown_tx) = self
            .net_controller
            .start(7878, super::net_controller::NetCommand::Send);
        
        self.shutdown_tx = Some(shutdown_tx);
        self.listen_to_state(task_receiver);
    }
    
    pub fn abort(&mut self) {
        // todo: current bug: if a task successfully resolves, there is still a Some() listener inside the struct.
        // that means the program just crashes if I press abort again. So it needs to be removed from the watcher, but I gotta think as to how
        // I'll have to do it tomorrow
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            shutdown_tx.send(true).expect("Shutdown failed");
        }
        
        let mut app_state = self.app_state.lock().unwrap();
        *app_state = AppState::Idle;
    }

    fn listen_to_state(&mut self, mut rx: UnboundedReceiver<AppState>) {
        println!("Listener ENGAGED");
        let app_state = self.app_state.clone();
        let ui_context = self.ui_context.clone(); // Clone the context for thread

        std::thread::spawn(move || {
           let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("Could not build controller listen runtime");
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