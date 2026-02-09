//! HTTP 服务器模块
//!
//! LocalSend HTTP API 服务器
//! 将在 Phase 4 中完整实现

use std::sync::Arc;
use tokio::sync::Mutex;
use std::net::SocketAddr;
use crate::{LocalSendConfig, FileSession, FileInfo, DeviceInfo, SessionManager, DiscoveryManager};

/// HTTP 服务器
#[derive(Debug)]
pub struct LocalSendServer {
    addr: SocketAddr,
    config: LocalSendConfig,
    session_manager: Arc<Mutex<SessionManager>>,
    discovery_manager: Arc<Mutex<DiscoveryManager>>,
}

impl LocalSendServer {
    /// 创建新的 HTTP 服务器
    pub fn new(
        addr: SocketAddr,
        config: LocalSendConfig,
        session_manager: Arc<Mutex<SessionManager>>,
        discovery_manager: Arc<Mutex<DiscoveryManager>>,
    ) -> Self {
        Self {
            addr,
            config,
            session_manager,
            discovery_manager,
        }
    }

    /// 启动服务器
    pub async fn start(&self) -> Result<(), std::io::Error> {
        println!("LocalSend HTTP 服务器已启动，监听 {}", self.addr);
        Ok(())
    }

    /// 获取会话管理器
    pub fn get_session_manager(&self) -> Arc<Mutex<SessionManager>> {
        self.session_manager.clone()
    }

    /// 获取发现管理器
    pub fn get_discovery_manager(&self) -> Arc<Mutex<DiscoveryManager>> {
        self.discovery_manager.clone()
    }
}

/// 创建设备发现服务
pub async fn start_discovery(
    _config: LocalSendConfig,
) -> Result<(), std::io::Error> {
    println!("设备发现服务已启动");
    Ok(())
}
