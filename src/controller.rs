use crate::net_controller::NetController;
use eel_file::AppState;
use eframe::egui;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_util::sync::CancellationToken;

pub struct Controller {
    net_controller: NetController,
    app_state: Arc<Mutex<AppState>>,
    ui_context: egui::Context,
}

impl Controller {
    pub fn new(ui_context: egui::Context, app_state: Arc<Mutex<AppState>>) -> Controller {
        Controller {
            net_controller: NetController::new(),
            ui_context,
            app_state,
        }
    }

    pub fn listen(&mut self) {
        // todo: add None check
        let task_receiver = self
            .net_controller
            .start(7878, super::net_controller::NetCommand::Receive);

        self.listen_to_state(task_receiver);
    }

    pub fn send(&mut self) {
        let task_receiver = self
            .net_controller
            .start(7878, super::net_controller::NetCommand::Send);

        self.listen_to_state(task_receiver);
    }

    pub fn abort(&mut self) {
        let app_state = self.app_state.lock().unwrap();
        
        match *app_state {
            AppState::Listening => { self.net_controller.abort_server() }
            AppState::Accepting(_) | AppState::Sending(_) => { self.net_controller.abort_task() }
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
