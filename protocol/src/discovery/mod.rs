//! 设备发现模块
//!
//! 实现 LocalSend 协议的设备发现功能
//! 包括 UDP 多播发现和 HTTP 扫描发现

use std::net::{UdpSocket, SocketAddr, Ipv4Addr};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};
use serde_json;
use crate::{DeviceInfo, LocalSendConfig, DiscoveryManager, AnnouncementMessage, PROTOCOL_VERSION};

/// 发现管理器引用类型
pub type DiscoveryManagerRef = Arc<Mutex<DiscoveryManager>>;

/// UDP 多播地址
const MULTICAST_ADDR: &str = "224.0.0.115";
const MULTICAST_PORT: u16 = 53317;

/// UDP 发现器
#[derive(Debug)]
pub struct UdpDiscoverer {
    config: LocalSendConfig,
    manager: DiscoveryManagerRef,
    socket: Arc<UdpSocket>,
}

impl UdpDiscoverer {
    /// 创建新的 UDP 发现器
    pub fn new(config: LocalSendConfig, manager: DiscoveryManagerRef) -> Self {
        let socket = Arc::new(UdpSocket::bind("0.0.0.0:0").expect("绑定 UDP socket 失败"));
        let _ = socket.set_multicast_loop_v4(true);

        Self {
            config,
            manager,
            socket,
        }
    }

    /// 发送公告
    pub async fn send_announcement(&self) -> Result<(), std::io::Error> {
        let announcement = AnnouncementMessage {
            msg_type: "announce".to_string(),
            id: self.config.device_id.clone(),
            device_type: self.config.device_type.clone(),
            name: self.config.device_name.clone(),
            version: format!("0.1.0-peersend"),
            protocol_version: PROTOCOL_VERSION.to_string(),
            download: true,
            port: Some(self.config.port),
            announcement_id: None,
            uses_password: false,
        };

        let msg = serde_json::to_string(&announcement)?;
        let addr: SocketAddr = format!("{}:{}", MULTICAST_ADDR, MULTICAST_PORT).parse().unwrap();

        let written = self.socket.send_to(msg.as_bytes(), addr)?;
        if written != msg.len() {
            eprintln!("警告: 公告未完全发送");
        }

        Ok(())
    }

    /// 开始发现 (发送和接收)
    pub async fn start_discovery(&self) -> Result<(), std::io::Error> {
        // 创建一个任务来接收公告
        let socket = self.socket.clone();
        let manager = self.manager.clone();
        let config = self.config.clone();

        let _ = tokio::spawn(async move {
            let mut buf = [0u8; 2048];
            loop {
                match socket.recv_from(&mut buf) {
                    Ok((len, addr)) => {
                        if let Ok(data) = std::str::from_utf8(&buf[..len]) {
                            if let Ok(msg) = serde_json::from_str::<AnnouncementMessage>(data) {
                                if msg.id != config.device_id {
                                    let device = DeviceInfo {
                                        id: msg.id,
                                        name: msg.name,
                                        device_type: msg.device_type,
                                        ip: addr.ip().to_string(),
                                        port: msg.port.unwrap_or(config.port),
                                        version: msg.version,
                                        protocol_version: msg.protocol_version,
                                        announcement_id: msg.announcement_id.unwrap_or_default(),
                                        uses_password: msg.uses_password,
                                    };

                                    let m = manager.lock().await;
                                    m.add_device(device).await;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("接收公告失败: {}", e);
                    }
                }
            }
        });

        // 定期发送公告
        let mut interval = interval(Duration::from_millis(5000));
        loop {
            interval.tick().await;
            if let Err(e) = self.send_announcement().await {
                eprintln!("发送公告失败: {}", e);
            }
        }
    }
}

/// HTTP 发现器
#[derive(Debug)]
pub struct HttpDiscoverer {
    config: LocalSendConfig,
    manager: DiscoveryManagerRef,
}

impl HttpDiscoverer {
    /// 创建新的 HTTP 发现器
    pub fn new(config: LocalSendConfig, manager: DiscoveryManagerRef) -> Self {
        Self { config, manager }
    }

    /// 扫描 IP 范围
    pub async fn scan_range(&self, base_ip: &str, range: u8) -> Result<(), std::io::Error> {
        let parts: Vec<u8> = base_ip.split('.').map(|s| s.parse().unwrap_or(0)).collect();
        if parts.len() != 4 {
            return Ok(());
        }

        let mut handles = Vec::new();

        for i in 1..=range {
            let ip = format!("{}.{}.{}.{}", parts[0], parts[1], parts[2], parts[3] + i);
            let port = self.config.port;
            let manager = self.manager.clone();

            let handle = tokio::spawn(async move {
                let addr = format!("http://{}:{}/api/v1/localsend/register", ip, port);

                let client = reqwest::Client::new();
                if let Ok(response) = client.get(&addr).send().await {
                    if let Ok(text) = response.text().await {
                        if let Ok(device) = serde_json::from_str::<crate::dto::RegisterResponse>(&text) {
                            let info = DeviceInfo {
                                id: device.id,
                                name: device.name,
                                device_type: device.device_type,
                                ip,
                                port: device.port.unwrap_or(port),
                                version: device.version,
                                protocol_version: device.protocol_version,
                                announcement_id: device.announcement_id.unwrap_or_default(),
                                uses_password: device.uses_password,
                            };

                            let m = manager.lock().await;
                            m.add_device(info).await;
                        }
                    }
                }
            });

            handles.push(handle);
        }

        // 等待所有任务完成
        for handle in handles {
            let _ = handle.await;
        }

        Ok(())
    }

    /// 检查特定 IP 是否运行 LocalSend
    pub async fn check_device(&self, ip: &str) -> Option<DeviceInfo> {
        let addr = format!("http://{}:{}/api/v1/localsend/register", ip, self.config.port);
        let client = reqwest::Client::new();

        if let Ok(response) = client.get(&addr).send().await {
            if let Ok(text) = response.text().await {
                if let Ok(device) = serde_json::from_str::<crate::dto::RegisterResponse>(&text) {
                    return Some(DeviceInfo {
                        id: device.id,
                        name: device.name,
                        device_type: device.device_type,
                        ip: ip.to_string(),
                        port: device.port.unwrap_or(self.config.port),
                        version: device.version,
                        protocol_version: device.protocol_version,
                        announcement_id: device.announcement_id.unwrap_or_default(),
                        uses_password: device.uses_password,
                    });
                }
            }
        }

        None
    }
}

/// 设备发现服务
#[derive(Debug)]
pub struct DiscoveryService {
    udp_discoverer: Option<UdpDiscoverer>,
    http_discoverer: Option<HttpDiscoverer>,
    manager: DiscoveryManagerRef,
}

impl DiscoveryService {
    /// 创建设备发现服务
    pub fn new(config: LocalSendConfig) -> Self {
        let manager = Arc::new(Mutex::new(DiscoveryManager::new()));

        Self {
            udp_discoverer: Some(UdpDiscoverer::new(config.clone(), manager.clone())),
            http_discoverer: Some(HttpDiscoverer::new(config, manager.clone())),
            manager,
        }
    }

    /// 获取发现管理器
    pub fn get_manager(&self) -> DiscoveryManagerRef {
        self.manager.clone()
    }

    /// 获取发现的设备
    pub async fn get_devices(&self) -> Vec<DeviceInfo> {
        self.manager.lock().await.get_devices().await
    }

    /// 清除发现的设备
    pub async fn clear_devices(&self) {
        self.manager.lock().await.clear().await;
    }

    /// 开始发现
    pub async fn start(&self) {
        if let Some(udp) = &self.udp_discoverer {
            if let Err(e) = udp.start_discovery().await {
                eprintln!("UDP 发现失败: {}", e);
            }
        }
    }
}
