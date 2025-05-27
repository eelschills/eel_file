use eel_file::{AppState, FileInfo};
use std::fs::{remove_file, File};
use std::io::Write;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
type CancelToken = Arc<Mutex<Option<CancellationToken>>>;

pub struct NetController {
    runtime: Option<Runtime>,
    worker: Option<JoinHandle<()>>,
    server_token: Option<CancellationToken>,
    task_token: CancelToken,
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
            server_token: None,
            task_token: Arc::new(Mutex::new(None)),
        }
    }

    pub fn start(&mut self, _port: usize, cmd: NetCommand) -> UnboundedReceiver<AppState> {
        
        let (tx, rx) = mpsc::unbounded_channel();
        
        let server_token = CancellationToken::new();

        match cmd {
            NetCommand::Send => {
                let task_token = CancellationToken::new();
                println!("Attempting to send a message");
                self.task_token.lock().unwrap().replace(server_token.clone());
                let futures_rewritten = self
                    .runtime
                    .as_ref()
                    .unwrap()
                    .spawn(Self::send(tx, task_token.clone()));
                self.worker = Some(futures_rewritten);
                rx
            }
            NetCommand::Receive => {
                self.server_token = Some(server_token.clone());
                self.task_token.lock().unwrap().replace(server_token.clone());
                let futures_rewritten = self
                    .runtime
                    .as_ref()
                    .unwrap()
                    .spawn(Self::listen(tx, server_token.clone(), self.task_token.clone()));
                self.worker = Some(futures_rewritten);
                rx
            }
        }
    }
    
    pub fn abort_task(&mut self) {
        self.task_token.lock().unwrap().take().unwrap().cancel();
    }
    
    pub fn abort_server(&mut self) {
        self.server_token.take().unwrap().cancel();
        self.task_token.lock().unwrap().take();
    }

    async fn listen(tx: UnboundedSender<AppState>, server_token: CancellationToken, task_token_ref: CancelToken) {
        // todo: fix unwrapping
        let addr: SocketAddr = format!("127.0.0.1:{}", 7878).parse().unwrap();
        let listener = TcpListener::bind(addr).await.unwrap();
        let _ = tx.send(AppState::Listening);
        
        let task_token = CancellationToken::new();
        
        task_token_ref.lock().unwrap().replace(task_token.clone());
        // todo: actually put it inside of

        loop {
            tokio::select! {
                _ = server_token.cancelled() => {
                    println!("Supposed to be shutting down from listener");
                    let _ = tx.send(AppState::Idle);
                    break;
                },

                Ok((stream, addr)) = listener.accept() => {
                    let _ = tx.send(AppState::Accepting(0.0));
                    // if in the future I want to listen to new connections and tell them to fuck off, this is where I'd do it
                    Self::handle_rx_stream(stream, addr, task_token.clone()).await;
                    let _ = tx.send(AppState::Listening);
                }
            }
        }
    }

    async fn handle_rx_stream(
        mut stream: TcpStream,
        addr: SocketAddr,
        shutdown_token: CancellationToken,
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

        let file = File::create("testfile.txt");

        match file {
            Ok(mut file) => {
                for i in 1..10000 {
                    if shutdown_token.is_cancelled() {
                        println!("Shutting down from file loop?");
                        remove_file("testfile.txt").expect("Could not delete file!");
                        break;
                    }
                    file.write_all(i.to_string().as_bytes()).expect("Write failed!");
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                }
            }
            Err(e) => { println!("File creation failed! Reason {}", e) }
        }
    }

    async fn send(tx: UnboundedSender<AppState>, task_token: CancellationToken) {
        tokio::select! {
        _ = task_token.cancelled() => {
            let tx = tx.clone();
            let _ = tx.send(AppState::Idle);
            println!("shutdown received");
        }

        _ = async {
            println!("sending?");
            let _ = tx.send(AppState::Sending(FileInfo::default()));
            sleep(Duration::from_secs(5)).await;
            println!("done!");
            } => {
                let _ = tx.send(AppState::Idle).unwrap();
            }
        }
    }
}
