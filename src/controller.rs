use eframe::egui;
use crate::eel_error::EelError;
use crate::net_controller::NetController;
use tokio::sync::mpsc::UnboundedReceiver;
use eel_file::AppState;

pub struct Controller {
    status_message: String,
    net_controller: NetController,
    task_receiver: Option<UnboundedReceiver<AppState>>,
    ui_context: egui::Context
}

impl Controller {
    pub fn new(ui_context: egui::Context) -> Controller {
        Controller {
            status_message: String::new(),
            net_controller: NetController::new(),
            task_receiver: None,
            ui_context
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
        self.task_receiver = self.net_controller.start(7878, super::net_controller::NetCommand::Receive);
    }

    pub fn send(&mut self) {
        self.task_receiver = self.net_controller.start(7878, super::net_controller::NetCommand::Send);
    }

    pub fn abort(&mut self) {
        // todo: implement
    }
}

// todo: tests
