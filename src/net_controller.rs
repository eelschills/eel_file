use std::fs::{remove_file, File};
use std::io::{BufWriter, Seek, Write};
use eel_file::AppState;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::watch::{Receiver, Sender};
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tokio::time::sleep;

pub struct NetController {
    runtime: Option<Runtime>,
    worker: Option<JoinHandle<()>>,
}

pub enum NetCommand {
    Send,
    Receive,
}

impl NetController {
    pub fn new() -> NetController {
        let rt = Builder::new_multi_thread().enable_all().build().unwrap();

        NetController {
            runtime: Some(rt),
            worker: None,
        }
    }

    pub fn start(&mut self, _port: usize, cmd: NetCommand) -> (UnboundedReceiver<AppState>, Sender<bool>) {
        let (tx, rx) = mpsc::unbounded_channel();

        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        match cmd {
            NetCommand::Send => {
                println!("Attempting to send a message");
                let futures_rewritten = self
                    .runtime
                    .as_ref()
                    .unwrap()
                    .spawn(Self::send(tx, shutdown_rx));
                self.worker = Some(futures_rewritten);
                (rx, shutdown_tx)
            }
            NetCommand::Receive => {
                let futures_rewritten = self
                    .runtime
                    .as_ref()
                    .unwrap()
                    .spawn(Self::listen(tx, shutdown_rx));
                self.worker = Some(futures_rewritten);
                (rx, shutdown_tx)
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
                    println!("Supposed to be shutting down");
                    break;
                },

                Ok((stream, addr)) = listener.accept() => {
                    let _ = tx.send(AppState::Accepting(0.0));
                    // if in the future I want to listen to new connections and tell them to fuck off, this is where I'd do it
                    let shutdown_rx = shutdown.clone();
                    Self::handle_rx_stream(stream, addr, shutdown_rx).await;
                }
            }
        }
    }

    async fn handle_rx_stream(
        mut stream: TcpStream,
        addr: SocketAddr,
        mut shutdown_signal: Receiver<bool>,
    ) {
        println!("Handling request...");
        let response = r#"{"message":"Hello, Eels!"}"#;
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
            response.len(),
            response
        );
        stream.write_all(resp.as_bytes()).await.unwrap();
        stream.flush().await.unwrap();

        let file = File::create("lollmao.txt");

        match file {
            Ok(mut file) => {
                for i in 1..10000 {
                    if shutdown_signal.has_changed().unwrap_or(false) {
                        println!("Shutting down?");
                        let _ = shutdown_signal.borrow_and_update();
                        remove_file("lollmao.txt").expect("Could not delete file!");
                    }
                    file.write_all(i.to_string().as_bytes()).expect("Write failed!");
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                }
            }
            Err(e) => { println!("File creation failed! Reason {}", e) }
        }
    }

    async fn send(tx: UnboundedSender<AppState>, mut shutdown_r: Receiver<bool>) {
        tokio::select! {
        _ = shutdown_r.changed() => {
            let tx = tx.clone();
            let _ = tx.send(AppState::Idle);
            println!("shutdown received");
        }

        _ = async {
            println!("sneeding");
            sleep(Duration::from_secs(5)).await;
            println!("done!");
            } => {
                println!("done sneeding");
            }
        }
    }
}
