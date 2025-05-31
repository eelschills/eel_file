use eel_file::{AppState, FileInfo};
use std::net::{SocketAddr, SocketAddrV4};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use eel_file::TransferState::Transferring;

type CancelToken = Arc<Mutex<Option<CancellationToken>>>;

pub struct NetController {
    runtime: Option<Runtime>,
    worker: Option<JoinHandle<()>>,
    server_token: Option<CancellationToken>,
    task_token: CancelToken,
}

pub enum NetCommand {
    Send(SocketAddrV4, FileInfo),
    Receive(PathBuf, u16),
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

    pub fn start(&mut self, cmd: NetCommand) -> UnboundedReceiver<AppState> {
        let (tx, rx) = mpsc::unbounded_channel();
        
        let server_token = CancellationToken::new();

        match cmd {
            NetCommand::Send(addr, file_info) => {
                let task_token = CancellationToken::new();
                println!("Attempting to send a message");
                self.task_token.lock().unwrap().replace(server_token.clone());
                let futures_rewritten = self
                    .runtime
                    .as_ref()
                    .unwrap()
                    .spawn(Self::send(tx, task_token.clone(), addr, file_info));
                self.worker = Some(futures_rewritten);
                rx
            }
            NetCommand::Receive(path, port) => {
                self.server_token = Some(server_token.clone());
                self.task_token.lock().unwrap().replace(server_token.clone());
                let futures_rewritten = self
                    .runtime
                    .as_ref()
                    .unwrap()
                    .spawn(Self::listen(tx, server_token.clone(), self.task_token.clone(), path, port));
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

    async fn listen(tx: UnboundedSender<AppState>, server_token: CancellationToken, task_token_ref: CancelToken, path: PathBuf, port: u16) {
        // todo: fix unwrapping
        let addr: SocketAddr = format!("0.0.0.0:{}", 7878).parse().unwrap();
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
                    let _ = tx.send(AppState::Accepting(Transferring(0.0)));
                    // if in the future I want to listen to new connections and tell them to fuck off, this is where I'd do it
                    Self::handle_rx_stream(stream, addr, task_token.clone(), path.clone(), tx.clone()).await;
                    let _ = tx.send(AppState::Listening);
                }
            }
        }
    }

    async fn handle_rx_stream(
        mut stream: TcpStream,
        addr: SocketAddr,
        shutdown_token: CancellationToken,
        path_buf: PathBuf,
        tx: UnboundedSender<AppState>,
    ) {
        let mut metadata = String::new();
        let mut buffer = BufReader::new(&mut stream);
        let mut content_length: usize = 0;

        let mut lines = (&mut buffer).lines();
        
        while let Some(line) = lines.next_line().await.unwrap() {
            if line.starts_with("Content-Length") {
                // wow if only I could chain these
                let mut length = line.clone();
                length.retain(|c| c.is_digit(10));
                content_length = length.parse().unwrap();
                break;
            }
        }
        
        // skipping \r\n
        let mut skip_buf = [0u8; 2];
        buffer.read_exact(&mut skip_buf).await.unwrap();
        
        let mut content = vec![0u8; content_length];
        buffer.read_exact(&mut content).await.unwrap();
        
        for byte in &content {
            metadata.push(*byte as char);
        }
        
        println!("Metadata: {}", metadata);
        
        let file_info: FileInfo = serde_json::from_str(&metadata).expect("What the FUG");
        
        println!("{:?},", file_info);
    }

    async fn send(tx: UnboundedSender<AppState>, task_token: CancellationToken, addr: SocketAddrV4, file_info: FileInfo) {
        tokio::select! {
        _ = task_token.cancelled() => {
            let tx = tx.clone();
            let _ = tx.send(AppState::Idle);
            println!("shutdown received");
        }

        _ = async {
            println!("sending?");
            let _ = tx.send(AppState::Sending(Transferring(0.0)));
            sleep(Duration::from_secs(5)).await;
            println!("done!");
            } => {
                let _ = tx.send(AppState::Idle).unwrap();
            }
        }
    }
}
