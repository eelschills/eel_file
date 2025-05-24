use crate::eel_error::EelError;
use crate::net_controller::NetController;
use eel_file::{AppState, FileInfo};
use eframe::egui;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedReceiver;

pub struct Controller {
    status_message: String,
    net_controller: NetController,
    task_receiver: Option<UnboundedReceiver<AppState>>,
    app_state: Arc<Mutex<AppState>>,
    ui_context: egui::Context,
}

impl Controller {
    pub fn new(ui_context: egui::Context, app_state: Arc<Mutex<AppState>>) -> Controller {
        Controller {
            status_message: String::new(),
            net_controller: NetController::new(),
            task_receiver: None,
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
        self.task_receiver = self
            .net_controller
            .start(7878, super::net_controller::NetCommand::Receive);
        let mut app_state = self.app_state.lock().unwrap();

        *app_state = AppState::Listening;
    }

    pub fn send(&mut self) {
        self.task_receiver = self
            .net_controller
            .start(7878, super::net_controller::NetCommand::Send);

        let mut app_state = self.app_state.lock().unwrap();
        *app_state = AppState::Sending(FileInfo::default());
    }

    pub fn abort(&mut self) {
        // todo: implement actually aborting a task through a shutdown signal
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
