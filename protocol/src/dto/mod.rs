//! LocalSend 协议数据传输对象 (DTO)
//!
//! 定义与 LocalSend 协议通信使用的数据结构

use serde::{Deserialize, Serialize};

/// 设备注册请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub id: String,
    #[serde(rename = "type")]
    pub device_type: String,
    pub name: String,
    pub version: String,
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    #[serde(default)]
    pub download: bool,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub announcement_id: Option<String>,
    #[serde(default)]
    pub uses_password: bool,
}

/// 设备注册响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub device_type: String,
    pub name: String,
    pub version: String,
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    #[serde(default)]
    pub download: bool,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub announcement_id: Option<String>,
    #[serde(default)]
    pub uses_password: bool,
}

/// 文件请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRequest {
    pub id: String,
    pub sender: String,
    #[serde(rename = "senderType")]
    pub sender_type: String,
    pub files: Vec<FileMetadata>,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub message: String,
}

/// 文件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub id: String,
    pub name: String,
    #[serde(rename = "fileType")]
    pub file_type: String,
    pub size: u64,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

/// 文件响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileResponse {
    pub id: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub accepted: bool,
    #[serde(default)]
    pub token: String,
}

/// 准备接收请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareRequest {
    pub id: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub files: Vec<IncomingFileMetadata>,
    #[serde(default)]
    pub token: String,
}

/// 准备接收响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareResponse {
    pub id: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(default)]
    pub files: Vec<IncomingFileMetadata>,
}

/// 传入文件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingFileMetadata {
    pub id: String,
    pub name: String,
    #[serde(rename = "fileType")]
    pub file_type: String,
    pub size: u64,
    #[serde(rename = "saveAs")]
    #[serde(default)]
    pub save_as: Option<String>,
}

/// 传输块请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockRequest {
    pub id: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub file_id: String,
    pub size: u64,
    #[serde(default)]
    pub token: String,
}

/// 取消请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelRequest {
    pub id: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(default)]
    pub reason: String,
}

/// HTTP 请求类型
#[derive(Debug, Clone, PartialEq)]
pub enum RequestType {
    /// 设备注册
    Register,
    /// 请求文件
    Request,
    /// 准备接收
    Prepare,
    /// 传输数据块
    Block,
    /// 取消传输
    Cancel,
    /// 未知
    Unknown,
}

/// 解析 HTTP 请求获取请求类型
pub fn parse_request_type(path: &str) -> RequestType {
    match path {
        "/api/v1/localsend/register" => RequestType::Register,
        "/api/v1/localsend/request" => RequestType::Request,
        "/api/v1/localsend/prepare-upload" => RequestType::Prepare,
        "/api/v1/localsend/upload" => RequestType::Block,
        "/api/v1/localsend/cancel" => RequestType::Cancel,
        _ => RequestType::Unknown,
    }
}

/// API 响应封装
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub data: Option<T>,
    #[serde(default)]
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            data: None,
            error: Some(message),
        }
    }
}

/// 公告消息 (用于 UDP 多播)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnouncementMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub id: String,
    #[serde(rename = "deviceType")]
    pub device_type: String,
    pub name: String,
    pub version: String,
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    #[serde(default)]
    pub download: bool,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub announcement_id: Option<String>,
    #[serde(default)]
    pub uses_password: bool,
}

impl AnnouncementMessage {
    pub fn from_register(req: &RegisterRequest, port: u16) -> Self {
        Self {
            msg_type: "announce".to_string(),
            id: req.id.clone(),
            device_type: req.device_type.clone(),
            name: req.name.clone(),
            version: req.version.clone(),
            protocol_version: req.protocol_version.clone(),
            download: req.download,
            port: req.port.or(Some(port)),
            announcement_id: req.announcement_id.clone(),
            uses_password: req.uses_password,
        }
    }
}

/// 证书信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateInfo {
    pub fingerprint: String,
    #[serde(rename = "startsAt")]
    pub starts_at: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: String,
}

/// 证书请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateRequest {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
}

/// 握手请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeRequest {
    pub id: String,
    #[serde(default)]
    pub public_key: String,
    #[serde(default)]
    pub session_id: String,
}

/// 握手响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeResponse {
    pub id: String,
    #[serde(default)]
    pub public_key: String,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub success: bool,
}
