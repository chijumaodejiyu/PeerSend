//! LocalSend 文件传输器
//!
//! 实现完整的文件发送和接收逻辑

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::{FileSession, FileInfo, TransferProgress, SessionState};

/// 块大小 (1MB)
const BLOCK_SIZE: usize = 1024 * 1024;

/// 文件发送器
#[derive(Debug, Clone)]
pub struct FileSender {
    session: FileSession,
    file_index: usize,
    bytes_sent: u64,
    chunk_size: usize,
}

impl FileSender {
    /// 创建新的文件发送器
    pub fn new(session: FileSession) -> Self {
        Self {
            session,
            file_index: 0,
            bytes_sent: 0,
            chunk_size: BLOCK_SIZE,
        }
    }

    /// 获取当前文件信息
    pub fn current_file_info(&self) -> Option<&FileInfo> {
        self.session.files.get(self.file_index)
    }

    /// 设置块大小
    pub fn set_chunk_size(&mut self, size: usize) {
        self.chunk_size = size;
    }

    /// 获取文件总数
    pub fn total_files(&self) -> usize {
        self.session.files.len()
    }

    /// 获取当前文件索引
    pub fn current_index(&self) -> usize {
        self.file_index
    }

    /// 检查是否完成
    pub fn is_complete(&self) -> bool {
        self.file_index >= self.session.files.len()
    }

    /// 跳到下一个文件
    pub fn next_file(&mut self) -> bool {
        self.file_index += 1;
        !self.is_complete()
    }

    /// 读取文件数据块
    pub async fn read_chunk(&mut self) -> Result<Option<Vec<u8>>, std::io::Error> {
        if let Some(file_info) = self.current_file_info() {
            let path = PathBuf::from(&file_info.name);

            match File::open(&path).await {
                Ok(mut file) => {
                    let mut buffer = vec![0u8; self.chunk_size];
                    match file.read(&mut buffer).await {
                        Ok(n) => {
                            buffer.truncate(n);
                            self.bytes_sent += n as u64;
                            Ok(Some(buffer))
                        }
                        Err(e) => Err(e),
                    }
                }
                Err(e) => Err(e),
            }
        } else {
            Ok(None)
        }
    }

    /// 获取当前进度
    pub async fn get_progress(&self) -> TransferProgress {
        let total: u64 = self.session.files.iter().map(|f| f.size).sum();
        TransferProgress {
            bytes_transferred: self.bytes_sent,
            total_bytes: total,
            speed_bytes_per_sec: 0.0,
        }
    }

    /// 获取当前字节偏移
    pub fn get_offset(&self) -> u64 {
        self.bytes_sent
    }
}

/// 文件接收器
#[derive(Debug, Clone)]
pub struct FileReceiver {
    session: FileSession,
    output_dir: PathBuf,
    file_index: usize,
    bytes_received: u64,
    current_file: Option<PathBuf>,
}

impl FileReceiver {
    /// 创建新的文件接收器
    pub fn new(session: FileSession, output_dir: PathBuf) -> Self {
        Self {
            session,
            output_dir,
            file_index: 0,
            bytes_received: 0,
            current_file: None,
        }
    }

    /// 获取当前文件信息
    pub fn current_file_info(&self) -> Option<&FileInfo> {
        self.session.files.get(self.file_index)
    }

    /// 获取保存路径
    pub fn get_save_path(&self, filename: &str) -> PathBuf {
        self.output_dir.join(filename)
    }

    /// 开始接收新文件
    pub async fn start_file(&mut self, filename: &str) -> Result<(), std::io::Error> {
        let save_path = self.get_save_path(filename);

        // 确保目录存在
        if let Some(parent) = save_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let file = File::create(&save_path).await?;
        self.current_file = Some(save_path);
        Ok(())
    }

    /// 写入数据块
    pub async fn write_chunk(&mut self, data: &[u8]) -> Result<(), std::io::Error> {
        if let Some(path) = &self.current_file {
            let mut file = OpenOptions::new()
                .append(true)
                .open(path)
                .await?;
            file.write_all(data).await?;
            self.bytes_received += data.len() as u64;
        }
        Ok(())
    }

    /// 完成当前文件
    pub async fn finish_current_file(&mut self) {
        self.current_file = None;
        self.file_index += 1;
    }

    /// 检查是否完成
    pub fn is_complete(&self) -> bool {
        self.file_index >= self.session.files.len()
    }

    /// 获取当前进度
    pub async fn get_progress(&self) -> TransferProgress {
        let total: u64 = self.session.files.iter().map(|f| f.size).sum();
        TransferProgress {
            bytes_transferred: self.bytes_received,
            total_bytes: total,
            speed_bytes_per_sec: 0.0,
        }
    }

    /// 获取当前文件索引
    pub fn current_index(&self) -> usize {
        self.file_index
    }
}

/// 文件传输管理器
#[derive(Debug)]
pub struct TransferManager {
    sessions: Arc<Mutex<Vec<FileSession>>>,
    receivers: Arc<Mutex<Vec<FileReceiver>>>,
    senders: Arc<Mutex<Vec<FileSender>>>,
}

impl TransferManager {
    /// 创建新的传输管理器
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(Vec::new())),
            receivers: Arc::new(Mutex::new(Vec::new())),
            senders: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 创建接收会话
    pub async fn create_receiver(
        &self,
        session_id: String,
        sender_id: String,
        files: Vec<FileInfo>,
        output_dir: PathBuf,
    ) -> FileReceiver {
        let session = FileSession::new(
            session_id.clone(),
            sender_id,
            "self".to_string(),
            files.clone(),
        );

        let mut sessions = self.sessions.lock().await;
        sessions.push(session.clone());

        let receiver = FileReceiver::new(session, output_dir);

        let mut receivers = self.receivers.lock().await;
        receivers.push(receiver.clone());

        receiver
    }

    /// 创建发送器
    pub async fn create_sender(&self, session: FileSession) -> FileSender {
        let sender = FileSender::new(session.clone());

        let mut senders = self.senders.lock().await;
        senders.push(sender.clone());

        sender
    }

    /// 获取接收器
    pub async fn get_receiver(&self, session_id: &str) -> Option<FileReceiver> {
        let receivers = self.receivers.lock().await;
        receivers.iter().find(|r| r.session.id == session_id).cloned()
    }

    /// 获取发送器
    pub async fn get_sender(&self, session_id: &str) -> Option<FileSender> {
        let senders = self.senders.lock().await;
        senders.iter().find(|s| s.session.id == session_id).cloned()
    }

    /// 获取所有会话
    pub async fn get_sessions(&self) -> Vec<FileSession> {
        self.sessions.lock().await.clone()
    }

    /// 移除接收器
    pub async fn remove_receiver(&self, session_id: &str) {
        let mut receivers = self.receivers.lock().await;
        receivers.retain(|r| r.session.id != session_id);
    }

    /// 移除发送器
    pub async fn remove_sender(&self, session_id: &str) {
        let mut senders = self.senders.lock().await;
        senders.retain(|s| s.session.id != session_id);
    }
}
