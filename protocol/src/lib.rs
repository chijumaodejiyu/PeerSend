//! LocalSend 协议实现
//!
//! 此模块实现了与 LocalSend 客户端互通的文件传输协议
//! 包括：设备发现、文件传输、会话管理等功能

pub mod dto;
pub mod crypto;
pub mod session;
pub mod discovery;
pub mod server;

pub use dto::AnnouncementMessage;

use std::sync::Arc;
use tokio::sync::Mutex;

/// 块大小 (1MB)
pub const BLOCK_SIZE: usize = 1024 * 1024;

/// LocalSend 协议常量
pub const PROTOCOL_VERSION: &str = "2.0";
pub const DEFAULT_PORT: u16 = 53317;
pub const ANNOUNCEMENT_INTERVAL_MS: u64 = 5000;
pub const SESSION_TIMEOUT_SECS: u64 = 300;

/// LocalSend 客户端配置
#[derive(Clone, Debug)]
pub struct LocalSendConfig {
    pub device_id: String,
    pub device_name: String,
    pub device_type: String,
    pub api_key: String,
    pub port: u16,
    pub use_tls: bool,
    pub download_dir: String,
}

impl Default for LocalSendConfig {
    fn default() -> Self {
        Self {
            device_id: uuid::Uuid::new_v4().to_string(),
            device_name: hostname::get().unwrap_or_default().to_string_lossy().into_owned(),
            device_type: "desktop".to_string(),
            api_key: uuid::Uuid::new_v4().to_string(),
            port: DEFAULT_PORT,
            use_tls: false,
            download_dir: std::env::temp_dir().to_string_lossy().into_owned(),
        }
    }
}

/// 会话状态
#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    Waiting,
    Transferring,
    Finished,
    Cancelled,
    Error(String),
}

/// 文件传输会话
#[derive(Debug, Clone)]
pub struct FileSession {
    pub id: String,
    pub sender_id: String,
    pub receiver_id: String,
    pub files: Vec<FileInfo>,
    pub state: Arc<Mutex<SessionState>>,
    pub progress: Arc<Mutex<TransferProgress>>,
}

impl FileSession {
    pub fn new(id: String, sender_id: String, receiver_id: String, files: Vec<FileInfo>) -> Self {
        Self {
            id,
            sender_id,
            receiver_id,
            files,
            state: Arc::new(Mutex::new(SessionState::Waiting)),
            progress: Arc::new(Mutex::new(TransferProgress::default())),
        }
    }
}

/// 文件信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileInfo {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub file_type: String,
    pub metadata: Option<serde_json::Value>,
}

/// 传输进度
#[derive(Debug, Default, Clone)]
pub struct TransferProgress {
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub speed_bytes_per_sec: f64,
}

impl TransferProgress {
    pub fn progress(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            self.bytes_transferred as f64 / self.total_bytes as f64
        }
    }
}

/// 发现到的设备信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub device_type: String,
    pub ip: String,
    pub port: u16,
    pub version: String,
    #[serde(default)]
    pub protocol_version: String,
    #[serde(default)]
    pub announcement_id: String,
    #[serde(default)]
    pub uses_password: bool,
}

/// 会话管理器
#[derive(Debug, Clone)]
pub struct SessionManager {
    sessions: Arc<Mutex<Vec<FileSession>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn create_session(
        &self,
        sender_id: String,
        receiver_id: String,
        files: Vec<FileInfo>,
    ) -> FileSession {
        let session_id = uuid::Uuid::new_v4().to_string();
        let session = FileSession::new(
            session_id.clone(),
            sender_id,
            receiver_id,
            files,
        );

        let mut sessions = self.sessions.lock().await;
        sessions.push(session.clone());

        session
    }

    pub async fn get_session(&self, session_id: &str) -> Option<FileSession> {
        let sessions = self.sessions.lock().await;
        sessions.iter().find(|s| s.id == session_id).cloned()
    }

    pub async fn remove_session(&self, session_id: &str) {
        let mut sessions = self.sessions.lock().await;
        sessions.retain(|s| s.id != session_id);
    }

    pub async fn get_all_sessions(&self) -> Vec<FileSession> {
        self.sessions.lock().await.clone()
    }
}

/// 设备发现管理器
#[derive(Debug, Clone)]
pub struct DiscoveryManager {
    discovered_devices: Arc<Mutex<Vec<DeviceInfo>>>,
}

impl DiscoveryManager {
    pub fn new() -> Self {
        Self {
            discovered_devices: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn add_device(&self, device: DeviceInfo) {
        let mut devices = self.discovered_devices.lock().await;
        if !devices.iter().any(|d| d.id == device.id) {
            devices.push(device);
        }
    }

    pub async fn remove_device(&self, id: &str) {
        let mut devices = self.discovered_devices.lock().await;
        devices.retain(|d| d.id != id);
    }

    pub async fn get_devices(&self) -> Vec<DeviceInfo> {
        self.discovered_devices.lock().await.clone()
    }

    pub async fn clear(&self) {
        self.discovered_devices.lock().await.clear();
    }
}
