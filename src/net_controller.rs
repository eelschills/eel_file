use eel_file::AppState::*;
use eel_file::{AppEvent, FileInfo, Util};
use std::io::{Error, ErrorKind};
use std::net::{SocketAddr, SocketAddrV4};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use sysinfo::Disks;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::{Builder, Runtime};
use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
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

        match cmd {
            NetCommand::Send(addr, file_info) => {
                let task_token = CancellationToken::new();

                self.task_token.lock().unwrap().replace(task_token.clone());

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
                let server_token = CancellationToken::new();
                let task_token = CancellationToken::new();
                self.server_token = Some(server_token.clone());
                self.task_token.lock().unwrap().replace(task_token.clone());

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
        self.task_token.lock().unwrap().take().unwrap().cancel();
    }

    async fn listen(
        tx: UnboundedSender<AppEvent>,
        server_token: CancellationToken,
        task_token_ref: CancelToken,
        path: PathBuf,
        port: u16,
    ) {
        let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
        let listener = TcpListener::bind(addr).await;

        if let Err(_) = listener {
            let _ = tx.send(AppEvent::AppState(Idle));
            log!(tx, "Could not listen: port most likely already in use.");
            return;
        }

        let listener = listener.unwrap();

        tx.send(AppEvent::AppState(Listening)).unwrap();
        // todo: get a macro for logging

        log!(tx, "Listening on port {}...", addr.port());

        let task_token = CancellationToken::new();

        task_token_ref.lock().unwrap().replace(task_token.clone());

        loop {
            select! {
                _ = server_token.cancelled() => {
                    let _ = tx.send(AppEvent::AppState(Idle));
                    log!(tx, "Listener shut down.");
                    break;
                },

                Ok((stream, addr)) = listener.accept() => {
                    let _ = tx.send(AppEvent::AppState(Handshake));
                    log!(tx, "Accepted connection from {}", addr);
                    // if in the future I want to listen to new connections and tell them to fuck off, this is where I'd do it
                    Self::handle_rx_stream(stream, task_token.clone(), path.clone(), tx.clone()).await;
                    log!(tx, "Communication ended with {}, returning to listening.", addr);
                    let task_token = CancellationToken::new();
                    task_token_ref.lock().unwrap().replace(task_token.clone());
                    let _ = tx.send(AppEvent::AppState(Listening));
                }
            }
        }
    }

    async fn handle_rx_stream(
        mut stream: TcpStream,
        shutdown_token: CancellationToken,
        destination_path_buf: PathBuf,
        tx: UnboundedSender<AppEvent>,
    ) {
        log!(tx, "Attempting to retrieve metadata...");
        let mut buffer = BufReader::new(&mut stream);
        let mut metadata_lines = (&mut buffer).lines();
        let mut metadata = String::new();

        while let Some(line) = metadata_lines.next_line().await.unwrap() {
            if line == "ITS OVER".to_string() {
                break;
            }
            metadata += &line;
        }

        // ugh jank ugh
        drop(metadata_lines);
        drop(buffer);

        let mut file_info: FileInfo = serde_json::from_str(&metadata).unwrap();
        log!(tx, "Received file info. Name: {}, size: {}", file_info.name, Util::display_size(file_info.size));
        
        tx.send(AppEvent::FileInfo(file_info.clone())).unwrap();

        let mut file_to_create = destination_path_buf.clone();
        file_to_create.push(&file_info.name);

        file_info.path = Some(file_to_create);

        if !NetController::is_enough_space(&destination_path_buf, file_info.size) {
            let res = stream
                .write(b"NO, SIRE.\r\n")
                .await;
            
            if let Err(_) = res {
                log!(tx, "Could not write to stream. This is HIGHLY unlikely at this point. :)");
                tx.send(AppEvent::AppState(Idle)).unwrap();
                return;
            }

            log!(tx, "You don't have enough space for the incoming file. Connection aborted.");
            return;
        }

        let file_result = NetController::create_file(file_info.clone()).await;

        if let Err(e) = file_result {
            log!(tx, "Failed to create file for the following reason: {}", e);
            return;
        }

        let file_result = file_result.unwrap();

        stream.write(b"HAND IT OVER\r\n").await.unwrap();

        Self::accept_file(
            tx.clone(),
            &mut stream,
            file_result,
            file_info,
            shutdown_token.clone(),
        )
        .await;
    }

    async fn accept_file(
        tx: UnboundedSender<AppEvent>,
        stream: &mut TcpStream,
        mut file_handle: File,
        file_info: FileInfo,
        shutdown_token: CancellationToken,
    ) {
        let size = file_info.size;
        let mut remaining_size = size;
        let file_path = file_info.path.clone().unwrap();
        let mut buffer = vec![0u8; 64 * 1024];

        tx.send(AppEvent::AppState(Accepting)).unwrap();
        log!(tx, "File transfer starting...");

        loop {
            select! {
                _ = shutdown_token.cancelled() => {
                    tx.send(AppEvent::AppState(Idle)).unwrap();
                    log!(tx, "File download cancelled.");
                    // cleanup (I should be making invisible temp files but whatever)
                    drop(file_handle);
                    let _ = std::fs::remove_file(file_path).expect("Couldn't remove file!!!");
                    break;
                }

                _ = stream.readable() => {
                    match stream.try_read(&mut buffer) {
                        Ok(0) => {
                            log!(tx, "Unexpected end of file: connection was unexpectedly terminated!");
                            drop(file_handle);
                            let _ = std::fs::remove_file(file_path).expect("Couldn't remove file!!!");
                            break;
                        }

                        Ok(bytes) => {
                            if bytes == 0 {
                            log!(tx, "Download interrupted unexpectedly. It's over.");
                            break;
                            }

                            // todo: better error handling? this is gonna crash but it shouldn't really happen since I'm checking for permissions
                            file_handle.write_all(&buffer[..bytes]).await.expect("Couldn't write to file");
                            remaining_size -= bytes as u64;
                            tx.send(AppEvent::Progress(1.0 - (remaining_size as f32 / file_info.size as f32))).unwrap();

                            if remaining_size == 0 {
                                log!(tx, "File transfer supposedly complete.");
                            break;
                            }
                        }

                        Err(e) if e.kind() == ErrorKind::WouldBlock => {
                            // println!("No data");
                            continue;
                        }

                        Err(_) => {
                            log!(tx, "Connection was unexpectedly terminated!");
                            break;
                        }
                    }
                }
            }
        }
    }

    async fn send(
        tx: UnboundedSender<AppEvent>,
        task_token: CancellationToken,
        addr: SocketAddrV4,
        file_info: FileInfo,
    ) {
        let _ = tx.send(AppEvent::AppState(Connecting));
        log!(tx, "Attempting to establish TCP connection to {}...", addr);

        loop {
            select! {
                _ = task_token.cancelled() => {
                    let _ = tx.send(AppEvent::AppState(Idle));
                    log!(tx, "Connection aborted manually by user.");
                    break;
                }

                conn = TcpStream::connect(addr) => {
                    match conn {
                        Ok(stream) => {
                            Self::handle_send_request(stream, tx.clone(), task_token.clone(), file_info).await;

                            let _ = tx.send(AppEvent::AppState(Idle));
                            break;
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::AppState(Idle));
                            log!(tx, "Connection closed with error: {}", e);
                            break;
                        }
                    }
                }
            }
        }
    }

    async fn handle_send_request(
        mut stream: TcpStream,
        tx: UnboundedSender<AppEvent>,
        cancel_token: CancellationToken,
        file_info: FileInfo,
    ) {
        tx.send(AppEvent::AppState(Handshake)).unwrap();
        let file_info_serialized = serde_json::to_string(&file_info).unwrap();
        stream.write(file_info_serialized.as_bytes()).await.unwrap();
        stream.write(b"\r\nITS OVER\r\n").await.unwrap();

        let mut reader = BufReader::new(&mut stream);
        let mut response = String::new();

        let response_result =
            tokio::time::timeout(Duration::from_secs(10), reader.read_line(&mut response)).await;

        match response_result {
            Ok(_) => {
                log!(tx, "Received response: {}", response);
            }
            Err(_) => {
                log!(tx, "Connection timeout elapsed! Aborting.");
                tx.send(AppEvent::AppState(Idle)).unwrap();
                return;
            }
        }
        
        match response.as_str() {
            "NO, SIRE." => {
                log!(tx, "Remote EELFILE rejected the file for, as of now, vague reasons. Probably not enough space?");
                tx.send(AppEvent::AppState(Idle)).unwrap();
                return;
            } 
            _ => {
                log!(tx, "Affirmative remote response received! Attempting to start transfer.");
                tx.send(AppEvent::AppState(Sending)).unwrap();
            }
        }

        let mut remaining_size = file_info.size;
        let mut buffer = vec![0u8; 64 * 1024];
        let mut file = File::open(file_info.path.as_ref().unwrap()).await.unwrap();

        while remaining_size > 0 {
            select! {
                _ = cancel_token.cancelled() => {
                    log!(tx, "Upload cancelled!");
                    break;
                }

                read = file.read(&mut buffer) => {
                    if let Err(e) = read {
                        log!(tx, "Error reading file. Aborting connection. Error: {}", e);
                        tx.send(AppEvent::AppState(Idle)).unwrap();
                        break;
                    }

                    let bytes = read.unwrap();
                    if bytes == 0 {
                        tx.send(AppEvent::AppState(Idle)).unwrap();
                        log!(tx, "Funny EOF error. This should never happen (it always does when I write this.");
                    }

                    let to_write = std::cmp::min(bytes as u64, remaining_size) as usize;
                    
                    // todo: if the other end drops connection this freezes as it waits
                    let write_result = stream.write_all(&buffer[..to_write]).await;

                    if let Err(e) = write_result {
                        tx.send(AppEvent::AppState(Idle)).unwrap();
                        log!(tx, "Connection to remote host closed unexpectedly. Aborting. Error: {}", e);
                        break;
                    }

                    remaining_size -= to_write as u64;
                    tx.send(AppEvent::Progress(1.0 - (remaining_size as f32 / file_info.size as f32))).unwrap();
            }
        }
    }
    }

    async fn create_file(file_info: FileInfo) -> Result<File, Error> {
        if let None = file_info.path {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "The save directory is missing from the passed data!",
            ));
        };

        File::options()
            .read(true)
            .append(true)
            .create_new(true)
            .open(file_info.path.as_ref().unwrap())
            .await
    }

    // super hacky solution that just assumes the user won't type in a relative path
    // I wish I could test it on some other device but oh well
    // also it will just give me a false even if there's an error or there are no matching drives
    // this is also not portable to other OSs, not that I care
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