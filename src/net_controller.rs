use eel_file::AppState::*;
use eel_file::{AppEvent, EelError, FileInfo};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::net::{SocketAddr, SocketAddrV4};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use sysinfo::Disks;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::{Builder, Runtime};
use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

type CancelToken = Arc<Mutex<Option<CancellationToken>>>;

macro_rules! log {
    ($tx:expr, $($arg:tt)*) => {
        $tx.send(AppEvent::StatusMessage(format!($($arg)*))).unwrap();
    };
}

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

    pub fn start(&mut self, cmd: NetCommand) -> UnboundedReceiver<AppEvent> {
        let (tx, rx) = mpsc::unbounded_channel();

        let server_token = CancellationToken::new();

        match cmd {
            NetCommand::Send(addr, file_info) => {
                let task_token = CancellationToken::new();

                self.task_token
                    .lock()
                    .unwrap()
                    .replace(server_token.clone());

                let futures_rewritten = self.runtime.as_ref().unwrap().spawn(Self::send(
                    tx,
                    task_token.clone(),
                    addr,
                    file_info,
                ));
                self.worker = Some(futures_rewritten);
                rx
            }

            NetCommand::Receive(path, port) => {
                self.server_token = Some(server_token.clone());
                self.task_token
                    .lock()
                    .unwrap()
                    .replace(server_token.clone());

                let futures_rewritten = self.runtime.as_ref().unwrap().spawn(Self::listen(
                    tx,
                    server_token.clone(),
                    self.task_token.clone(),
                    path,
                    port,
                ));

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

    async fn listen(
        tx: UnboundedSender<AppEvent>,
        server_token: CancellationToken,
        task_token_ref: CancelToken,
        path: PathBuf,
        port: u16,
    ) {
        let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
        let listener = TcpListener::bind(addr).await.unwrap();
        tx.send(AppEvent::AppState(Listening)).unwrap();
        // todo: get a macro for logging
        
        log!(tx, "Listening on port {}...", addr.port());

        let task_token = CancellationToken::new();

        task_token_ref.lock().unwrap().replace(task_token.clone());
        // todo: actually put it inside of

        loop {
            tokio::select! {
                _ = server_token.cancelled() => {
                    let _ = tx.send(AppEvent::AppState(Idle));
                    log!(tx, "Listener shut down.");
                    break;
                },

                Ok((stream, addr)) = listener.accept() => {
                    let _ = tx.send(AppEvent::AppState(Accepting));
                    log!(tx, "Accepted connection from {}", addr);
                    // if in the future I want to listen to new connections and tell them to fuck off, this is where I'd do it
                    Self::handle_rx_stream(stream, addr, task_token.clone(), path.clone(), tx.clone()).await;
                    log!(tx, "Communication ended with {}, returning to listening.", addr);
                    let _ = tx.send(AppEvent::AppState(Listening));
                }
            }
        }
    }

    async fn handle_rx_stream(
        mut stream: TcpStream,
        addr: SocketAddr,
        shutdown_token: CancellationToken,
        destination_path_buf: PathBuf,
        tx: UnboundedSender<AppEvent>,
    ) {
        log!(tx, "Attempting to retrieve metadata...");
        let buffer = BufReader::new(&mut stream);
        let mut metadata_lines = buffer.lines();
        let mut metadata = String::new();

        while let Some(line) = metadata_lines.next_line().await.unwrap() {
            metadata += &line;
        }

        let file_info: FileInfo = serde_json::from_str(&metadata).unwrap();
        println!("Received file info: {:?}", file_info);
        if !NetController::is_enough_space(&destination_path_buf, file_info.size) {
            let result = stream
                .write(b"NO, SIRE.")
                .await
                .expect("Couldn't write to stream");
            
            log!(tx, "You don't have enough space for the incoming file.");
            return;
        }

        stream.write(b"HAND IT OVER\n").await.unwrap();
        // todo: start accepting
    }

    async fn send(
        tx: UnboundedSender<AppEvent>,
        task_token: CancellationToken,
        addr: SocketAddrV4,
        file_info: FileInfo,
    ) {
        let _ = tx.send(AppEvent::AppState(Connecting));
        let mut file_info = file_info.clone();
        let mut file_handle = File::open(file_info.path.as_ref().unwrap()).unwrap();
        file_info.hash = Some(Self::hash_file(&mut file_handle));
        
        // todo: maybe make it possible to abort waiting on a connection (later) 
        // I hate writing selects :(
        
        let fuck_you = Builder::new_multi_thread().enable_all().build().unwrap();
        
        loop {
            select! {
                _ = task_token.cancelled() => {
                    let _ = tx.send(AppEvent::AppState(Idle));
                    log!(tx, "Connection closed manually by user.");
                    break;
                }
                
                _ = timeout(Duration::from_secs(2), Self::handle_send_request(tx.clone(),
                task_token.clone(), addr, file_info)) => {
                    let _ = tx.send(AppEvent::AppState(Idle));
                    log!(tx, "Connection handled owo?");
                    break;
                }
            }
        }
    }
    
    async fn handle_send_request(tx: UnboundedSender<AppEvent>,
                                 cancel_token: CancellationToken,
                                 addr: SocketAddrV4, 
                                 file_info: FileInfo) {
        // todo: sending future
        // I took it out so I can cancel the task at any time with a select

        let stream = TcpStream::connect(addr).await;

        if let Err(e) = stream {
            tx.send(AppEvent::Error(EelError::ConnectionError(e.to_string()))).unwrap();
            tx.send(AppEvent::AppState(Idle)).unwrap();
            return;
        }

        tx.send(AppEvent::AppState(Handshake)).unwrap();

        let file_info_serialized = serde_json::to_string(&file_info).unwrap();
        let mut stream = stream.unwrap();

        stream.write(file_info_serialized.as_bytes()).await.unwrap();

        // todo: actual file sending after getting a response
    }

    fn hash_file(file: &mut File) -> Vec<u8> {
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 4096];

        loop {
            let n = file.read(&mut buffer).unwrap();
            if n == 0 {
                break;
            }
            Digest::update(&mut hasher, &buffer[..n]);
        }

        hasher.finalize().to_vec()
    }

    // super hacky solution that assumes the user won't type in a relative path
    // I wish I could test it on some other device but oh well
    // also it will just give me a false even if there's an error or there are no matching drives
    // this is also not portable to other OSs
    // todo: re-evaluate this
    fn is_enough_space(path: &PathBuf, filesize: u64) -> bool {
        let mut volume = path.to_str().unwrap()[0..2].to_string();
        volume.push_str("\\");

        let disks = Disks::new_with_refreshed_list();

        disks.list().iter().any(|disk| {
            disk.mount_point().to_str().unwrap() == volume && disk.available_space() > filesize
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;
    use std::fs::File;

    #[test]
    fn hash_test() {
        let mut voidcat = File::open("assets/voidcat.png").expect("you fail");
        let result = NetController::hash_file(&mut voidcat);
        assert_eq!(
            result,
            hex!("F3E79833B5D642E4C84DAA8E274B5389D759BE391B90F6B62A6C785EF2FA1BCF")
        );
    }

    #[test]
    fn test_free_space_check() {
        let file_size: u64 = 1024 * 1024 * 1024 * 150;
        let path_buf = PathBuf::from("C:/Users/user/RustroverProjects/eel_file/assets");

        let result = NetController::is_enough_space(&path_buf, file_size);

        assert_eq!(result, false);

        let file_size: u64 = 1024 * 1024 * 1024 * 20;

        let result = NetController::is_enough_space(&path_buf, file_size);

        assert_eq!(result, true);
    }
}
