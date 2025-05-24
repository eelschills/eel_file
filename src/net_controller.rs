use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::{mpsc, watch};
use tokio::sync::watch::{Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use eel_file::AppState;

pub struct NetController {
    runtime: Option<Runtime>,
    worker: Option<JoinHandle<()>>,
    shutdown_tx: Option<Sender<bool>>,
}

pub enum NetCommand {
    Send,
    Receive
}

impl NetController {
    pub fn new() -> NetController {
        NetController {
            runtime: None,
            worker: None,
            shutdown_tx: None,
        }
    }

    pub fn start(&mut self, _port: usize, cmd: NetCommand) -> Option<UnboundedReceiver<AppState>> {
        let (tx, rx) = mpsc::unbounded_channel();

        let rt = Builder::new_multi_thread().enable_all().build().unwrap();
        self.runtime = Some(rt);

        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        self.shutdown_tx = Some(shutdown_tx);

        match cmd {
            NetCommand::Send => {
                println!("Attempting to send a message");
                let futures_rewritten = self.runtime.as_ref().unwrap().spawn(Self::send(tx, shutdown_rx));
                self.worker = Some(futures_rewritten);
                Some(rx)
            }
            NetCommand::Receive => {
                let futures_rewritten = self.runtime.as_ref().unwrap().spawn(Self::listen(tx, shutdown_rx));
                self.worker = Some(futures_rewritten);
                Some(rx)
            }
        }

    }

    async fn listen(tx: UnboundedSender<AppState>, mut shutdown: Receiver<bool>) {
        // todo: fix unwrapping
        let addr: SocketAddr = format!("127.0.0.1:{}", 7878).parse().unwrap();
        let listener = TcpListener::bind(addr).await.unwrap();
        let _ = tx.send(AppState::Listening);

        loop {
            tokio::select! {

            _ = shutdown.changed() => {
                    let tx = tx.clone();
                    let _ = tx.send(AppState::Idle);
                println!("shutdown received");
                break;
            }

            Ok((mut stream, addr)) = listener.accept() => {
               let tx = tx.clone();
               tokio::spawn(async move {
                   let mut reader = BufReader::new(&mut stream);
                   let mut lines = reader.lines();
                   let mut payload = Vec::new();

                   while let Ok(Some(line)) = lines.next_line().await {
                       if line.is_empty() {
                           break;
                       }
                       payload.push(line);
                   }

                   let _ = tx.send(AppState::Accepting(0.0));

                   let body = r#"{"message":"Hello, Eels!"}"#;
                   let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                   body.len(), body);
                   stream.write_all(response.as_bytes()).await.expect("Fug");
                   stream.flush().await.expect("Fug");
                   }
            );
                }
            }
        }
    }

    async fn send(tx: UnboundedSender<AppState>, shutdown_r: Receiver<bool>) {
        println!("sneeding");
        sleep(Duration::from_secs(5)).await;
        println!("done!");
    }
}
