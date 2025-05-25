use crate::eel_error::EelError;
use crate::net_controller::NetController;
use eel_file::{AppState, FileInfo};
use eframe::egui;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::watch::Sender;

pub struct Controller {
    status_message: String,
    net_controller: NetController,
    task_receiver: Option<UnboundedReceiver<AppState>>,
    shutdown_tx: Option<Sender<bool>>,
    app_state: Arc<Mutex<AppState>>,
    ui_context: egui::Context,
}

impl Controller {
    pub fn new(ui_context: egui::Context, app_state: Arc<Mutex<AppState>>) -> Controller {
        Controller {
            status_message: String::new(),
            net_controller: NetController::new(),
            task_receiver: None,
            shutdown_tx: None,
            ui_context,
            app_state,
        }
    }

    pub fn poll(&mut self) -> Result<AppState, EelError> {
        match &mut self.task_receiver {
            None => Err(EelError::Poll(String::from(
                "Trying to poll a non-existent future",
            ))),
            Some(rx) => match rx.try_recv() {
                Err(_) => Err(EelError::EmptyMessage(String::from(
                    "Receiving on an empty stream",
                ))),
                Ok(appstate) => Ok(appstate),
            },
        }
    }

    pub fn listen(&mut self) {
        // todo: add None check
        let (task_receiver, shutdown_tx) = self
            .net_controller
            .start(7878, super::net_controller::NetCommand::Receive);
        
        self.task_receiver = Some(task_receiver);
        self.shutdown_tx = Some(shutdown_tx);
        
        let mut app_state = self.app_state.lock().unwrap();

        *app_state = AppState::Listening;
    }

    pub fn send(&mut self) {
        let (task_receiver, shutdown_tx) = self
            .net_controller
            .start(7878, super::net_controller::NetCommand::Send);

        self.task_receiver = Some(task_receiver);
        self.shutdown_tx = Some(shutdown_tx);

        let mut app_state = self.app_state.lock().unwrap();
        *app_state = AppState::Sending(FileInfo::default());
    }

    pub fn abort(&mut self) {
        // todo: implement actually aborting a task through a shutdown signal
        self.shutdown_tx.as_mut().unwrap().send(true).expect("Shutdown failed");
        
        let mut app_state = self.app_state.lock().unwrap();
        *app_state = AppState::Idle;
    }

    // why do I need this again?
    pub fn run_updates(&'static mut self) {
        tokio::spawn(async move {
            while let Some(msg) = self.task_receiver.as_mut().unwrap().recv().await {
                self.ui_context.request_repaint(); // wake the UI
            }
        });
    }
}

// todo: tests