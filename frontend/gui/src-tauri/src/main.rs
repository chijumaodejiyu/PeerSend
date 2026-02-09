#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use anyhow::{Context, Result};
use serde::Serialize;
use std::{
    net::SocketAddr,
    path::PathBuf,
    process::{Command, Stdio},
    sync::Arc,
    time::Duration,
};
use tokio::sync::{Mutex, broadcast};
use tokio::time::sleep;

use easytier::{
    proto::{
        api::instance::{InstanceIdentifier, ListPeerRequest, PeerManageRpc, PeerManageRpcClientFactory, ShowNodeInfoRequest},
        rpc_impl::standalone::StandAloneClient,
        rpc_types::controller::BaseController,
    },
    tunnel::tcp::TcpTunnelConnector,
};

const DEFAULT_RPC_PORTAL: &str = "127.0.0.1:15888";
const PID_FILE: &str = "/tmp/peersend-easytier.pid";
const LOCALSEND_PORT: u16 = 53317;

/// 网络配置
#[derive(Debug, Clone, Serialize)]
pub struct NetworkConfig {
    pub network_name: String,
    pub network_secret: Option<String>,
    pub peers: Vec<String>,
    pub dhcp: bool,
    pub ipv4: Option<String>,
    pub enable_wg: bool,
    pub rpc_portal: SocketAddr,
}

/// 守护进程状态
#[derive(Debug, Clone, Serialize)]
pub struct DaemonStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub peer_count: usize,
    pub network_name: String,
}

/// 文件传输状态
#[derive(Debug, Clone, Serialize)]
pub struct TransferStatus {
    pub id: String,
    pub r#type: String,
    pub state: String,
    pub progress: f64,
    pub speed: u64,
    pub file_name: String,
    pub sender: String,
    pub receiver: String,
}

/// 设备信息
#[derive(Debug, Clone, Serialize)]
pub struct DeviceStatus {
    pub id: String,
    pub name: String,
    pub device_type: String,
    pub ip: String,
    pub port: u16,
    pub version: String,
    pub online: bool,
}

/// 收到的文件请求
#[derive(Debug, Clone, Serialize)]
pub struct FileRequest {
    pub session_id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub files: Vec<IncomingFile>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IncomingFile {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub file_type: String,
}

/// EasyTier 守护进程管理器
#[derive(Debug)]
pub struct EasyTierDaemon {
    rpc_portal: SocketAddr,
    pid_file: PathBuf,
}

impl EasyTierDaemon {
    pub fn new(rpc_portal: Option<SocketAddr>) -> Self {
        Self {
            rpc_portal: rpc_portal.unwrap_or_else(|| DEFAULT_RPC_PORTAL.parse().unwrap()),
            pid_file: PathBuf::from(PID_FILE),
        }
    }

    pub fn is_running(&self) -> bool {
        if let Some(pid) = self.read_pid() {
            std::process::Command::new("kill")
                .arg("-0")
                .arg(pid.to_string())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        } else {
            false
        }
    }

    fn read_pid(&self) -> Option<u32> {
        std::fs::read_to_string(&self.pid_file)
            .ok()?
            .trim()
            .parse()
            .ok()
    }

    fn find_easytier_binary(&self) -> Result<PathBuf> {
        let project_bin = PathBuf::from("/home/ryanz/Documents/PeerSend/PeerSend/target/debug/easytier-core");
        if project_bin.exists() {
            return Ok(project_bin);
        }

        let paths = [
            PathBuf::from("/usr/bin/easytier-core"),
            PathBuf::from("/usr/local/bin/easytier-core"),
            PathBuf::from("/home/ryanz/easytier/target/debug/easytier-core"),
        ];

        for p in &paths {
            if p.exists() {
                return Ok(p.clone());
            }
        }

        let output = Command::new("which")
            .arg("easytier-core")
            .output()
            .context("执行 which 命令失败")?;

        if output.status.success() {
            let path = String::from_utf8(output.stdout)?;
            let path = PathBuf::from(path.trim());
            if path.exists() {
                return Ok(path);
            }
        }

        anyhow::bail!("找不到 easytier-core 二进制文件")
    }

    pub async fn start(&self, config: &NetworkConfig) -> Result<()> {
        if self.is_running() {
            self.stop().await?;
            sleep(Duration::from_secs(1)).await;
        }

        let peersend_cli = PathBuf::from("/home/ryanz/Documents/PeerSend/PeerSend/target/debug/peersend");
        if peersend_cli.exists() {
            let output = Command::new(&peersend_cli)
                .arg("start")
                .arg("--network-name")
                .arg(&config.network_name)
                .args(config.peers.iter().flat_map(|p| vec!["--peers", p]))
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .output()
                .context("执行 peersend start 失败")?;

            if output.status.success() {
                return Ok(());
            }
        }

        let rpc_portal_str = self.rpc_portal.to_string();
        let mut args = vec!["--rpc-portal", &rpc_portal_str];

        if config.dhcp {
            args.push("--dhcp");
        } else if let Some(ip) = &config.ipv4 {
            args.push("--ipv4");
            args.push(ip);
        }

        args.push("--network-name");
        args.push(&config.network_name);

        if let Some(secret) = &config.network_secret {
            args.push("--network-secret");
            args.push(secret);
        }

        for peer in &config.peers {
            args.push("--peers");
            args.push(peer);
        }

        if config.enable_wg {
            args.push("--enable-wireguard");
        }

        let bin_path = self.find_easytier_binary()?;
        let child = Command::new(&bin_path)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .context("启动 easytier-core 失败")?;

        let pid = child.id();
        std::fs::write(&self.pid_file, pid.to_string())
            .context("写入 PID 文件失败")?;

        self.wait_for_rpc().await?;
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        let peersend_cli = PathBuf::from("/home/ryanz/Documents/PeerSend/PeerSend/target/debug/peersend");
        if peersend_cli.exists() {
            let output = Command::new(&peersend_cli)
                .arg("stop")
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .output()
                .context("执行 peersend stop 失败")?;

            if output.status.success() {
                self.cleanup_pid();
                return Ok(());
            }
        }

        if let Some(pid) = self.read_pid() {
            let _ = Command::new("kill")
                .arg(pid.to_string())
                .output();

            for i in 0..10 {
                sleep(Duration::from_secs(1)).await;
                if !self.is_running() {
                    break;
                }
                if i >= 9 {
                    let _ = Command::new("kill")
                        .arg("-9")
                        .arg(pid.to_string())
                        .output();
                }
            }
            self.cleanup_pid();
        }

        Ok(())
    }

    fn cleanup_pid(&self) {
        let _ = std::fs::remove_file(&self.pid_file);
    }

    pub async fn status(&self) -> DaemonStatus {
        let rpc_portal: SocketAddr = DEFAULT_RPC_PORTAL.parse().unwrap();

        // 首先检查进程是否在运行
        let process_running = self.is_running();

        if !process_running {
            return DaemonStatus {
                running: false,
                pid: None,
                peer_count: 0,
                network_name: "".to_string(),
            };
        }

        // 尝试通过 RPC 获取网络状态
        let rpc_url = format!("tcp://{}", rpc_portal);
        let tcp_connector = TcpTunnelConnector::new(rpc_url.parse().unwrap());

        let mut client = StandAloneClient::new(tcp_connector);

        // 获取对等点数量和节点信息
        let request = ListPeerRequest {
            instance: Some(InstanceIdentifier {
                selector: Some(
                    easytier::proto::api::instance::instance_identifier::Selector::Id(
                        easytier::proto::common::Uuid {
                            part1: 0,
                            part2: 0,
                            part3: 0,
                            part4: 0,
                        },
                    ),
                ),
            }),
        };

        let peer_count = match client
            .scoped_client::<PeerManageRpcClientFactory<BaseController>>("".to_string())
            .await
        {
            Ok(mut peer_client) => match peer_client
                .list_peer(BaseController::default(), request)
                .await
            {
                Ok(response) => response.peer_infos.len(),
                Err(_) => 0,
            },
            Err(_) => 0,
        };

        // 尝试获取节点信息以获取网络名称
        let mut network_name = String::new();
        if let Ok(mut peer_client) = client
            .scoped_client::<PeerManageRpcClientFactory<BaseController>>("".to_string())
            .await
        {
            let node_request = easytier::proto::api::instance::ShowNodeInfoRequest {
                instance: Some(InstanceIdentifier {
                    selector: Some(
                        easytier::proto::api::instance::instance_identifier::Selector::Id(
                            easytier::proto::common::Uuid {
                                part1: 0,
                                part2: 0,
                                part3: 0,
                                part4: 0,
                            },
                        ),
                    ),
                }),
            };
            if let Ok(response) = peer_client
                .show_node_info(BaseController::default(), node_request)
                .await
            {
                if let Some(n) = response.node_info {
                    if !n.hostname.is_empty() {
                        network_name = n.hostname;
                    } else if !n.ipv4_addr.is_empty() {
                        network_name = n.ipv4_addr;
                    }
                }
            }
        }

        DaemonStatus {
            running: true,
            pid: self.read_pid(),
            peer_count,
            network_name,
        }
    }

    async fn wait_for_rpc(&self) -> Result<()> {
        for _ in 0..30 {
            if tokio::net::TcpStream::connect(self.rpc_portal).await.is_ok() {
                return Ok(());
            }
            sleep(Duration::from_secs(1)).await;
        }
        anyhow::bail!("RPC 端口连接超时")
    }
}

/// 全局状态管理
struct AppState {
    transfers: Arc<Mutex<Vec<TransferStatus>>>,
    devices: Arc<Mutex<Vec<DeviceStatus>>>,
    incoming_requests: Arc<Mutex<Vec<FileRequest>>>,
    /// 用于通知前端有新请求的通道
    request_sender: broadcast::Sender<FileRequest>,
}

impl AppState {
    fn new() -> Self {
        let (tx, _rx) = broadcast::channel(100);
        Self {
            transfers: Arc::new(Mutex::new(Vec::new())),
            devices: Arc::new(Mutex::new(Vec::new())),
            incoming_requests: Arc::new(Mutex::new(Vec::new())),
            request_sender: tx,
        }
    }

    fn subscribe(&self) -> broadcast::Receiver<FileRequest> {
        self.request_sender.subscribe()
    }
}

static APP_STATE: once_cell::sync::Lazy<Arc<AppState>> =
    once_cell::sync::Lazy::new(|| Arc::new(AppState::new()));

#[tauri::command]
async fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
async fn get_status() -> DaemonStatus {
    let daemon = EasyTierDaemon::new(None);
    daemon.status().await
}

#[tauri::command]
async fn start_daemon(
    _window: tauri::Window,
    network_name: String,
    network_secret: Option<String>,
    peers: Vec<String>,
) -> Result<(), String> {
    let daemon = EasyTierDaemon::new(None);

    let config = NetworkConfig {
        network_name,
        network_secret,
        peers,
        dhcp: true,
        ipv4: None,
        enable_wg: false,
        rpc_portal: DEFAULT_RPC_PORTAL.parse().unwrap(),
    };

    daemon.start(&config)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_daemon() -> Result<(), String> {
    let daemon = EasyTierDaemon::new(None);
    daemon.stop().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn discover_peers() -> Result<Vec<serde_json::Value>, String> {
    let rpc_portal: SocketAddr = DEFAULT_RPC_PORTAL.parse().unwrap();

    // 检查 RPC 服务是否可用
    if tokio::net::TcpStream::connect(rpc_portal).await.is_err() {
        return Ok(vec![]);
    }

    // 创建 RPC 客户端 - 使用 URL 格式
    let rpc_url = format!("tcp://{}", rpc_portal);
    let tcp_connector = TcpTunnelConnector::new(rpc_url.parse().unwrap());
    let mut client = StandAloneClient::new(tcp_connector);

    // 获取对等点列表 - 使用空字符串作为默认实例 ID
    let request = ListPeerRequest {
        instance: Some(InstanceIdentifier {
            selector: Some(
                easytier::proto::api::instance::instance_identifier::Selector::Id(
                    easytier::proto::common::Uuid {
                        part1: 0,
                        part2: 0,
                        part3: 0,
                        part4: 0,
                    },
                ),
            ),
        }),
    };

    match client
        .scoped_client::<PeerManageRpcClientFactory<BaseController>>("".to_string())
        .await
    {
        Ok(mut peer_client) => {
            match peer_client
                .list_peer(BaseController::default(), request)
                .await
            {
                Ok(response) => {
                    let result: Vec<serde_json::Value> = response
                        .peer_infos
                        .iter()
                        .map(|p| {
                            let peer_id = p.peer_id.to_string();

                            // 获取第一个连接的信息
                            let first_conn = p.conns.first();

                            // 远程地址 URL
                            let remote_url = first_conn
                                .and_then(|c| c.tunnel.as_ref())
                                .and_then(|t| t.remote_addr.as_ref())
                                .map(|u| u.url.clone())
                                .unwrap_or_default();

                            // 连接类型
                            let conn_type = first_conn
                                .and_then(|c| c.tunnel.as_ref())
                                .map(|t| t.tunnel_type.clone())
                                .unwrap_or_default();

                            // 延迟 (微秒转毫秒)
                            let latency_ms = first_conn
                                .and_then(|c| c.stats.as_ref())
                                .map(|s| s.latency_us as f64 / 1000.0)
                                .unwrap_or(0.0);

                            // 接收字节
                            let rx_bytes = first_conn
                                .and_then(|c| c.stats.as_ref())
                                .map(|s| s.rx_bytes)
                                .unwrap_or(0);

                            // 发送字节
                            let tx_bytes = first_conn
                                .and_then(|c| c.stats.as_ref())
                                .map(|s| s.tx_bytes)
                                .unwrap_or(0);

                            // 连接特性
                            let features = first_conn
                                .map(|c| c.features.join(", "))
                                .unwrap_or_default();

                            // 生成设备名称
                            let name = if !features.is_empty() {
                                // 从特性中提取有用的信息
                                if features.contains("wg") {
                                    format!("Peer {} (WireGuard)", peer_id)
                                } else if features.contains("tcp") {
                                    format!("Peer {} (TCP)", peer_id)
                                } else if features.contains("quic") {
                                    format!("Peer {} (QUIC)", peer_id)
                                } else {
                                    format!("Peer {} ({})", peer_id, conn_type)
                                }
                            } else {
                                format!("Peer {} ({})", peer_id, conn_type)
                            };

                            serde_json::json!({
                                "id": peer_id,
                                "name": name,
                                "type": "peer",
                                "ip": remote_url,
                                "port": 0u16,
                                "version": "",
                                "status": "online",
                                "latency_ms": latency_ms,
                                "rx_bytes": rx_bytes,
                                "tx_bytes": tx_bytes,
                                "connection_type": conn_type,
                                "features": features
                            })
                        })
                        .collect();
                    Ok(result)
                }
                Err(e) => {
                    println!("获取对等点列表失败: {}", e);
                    Ok(vec![])
                }
            }
        }
        Err(e) => {
            println!("创建 RPC 客户端失败: {}", e);
            Ok(vec![])
        }
    }
}

#[tauri::command]
async fn send_files(
    _window: tauri::Window,
    paths: Vec<String>,
    peer_id: String,
) -> Result<(), String> {
    let state = APP_STATE.clone();
    let display_peer_id = peer_id.clone();

    let transfer = TransferStatus {
        id: uuid::Uuid::new_v4().to_string(),
        r#type: "send".to_string(),
        state: "pending".to_string(),
        progress: 0.0,
        speed: 0,
        file_name: paths.first().unwrap_or(&"".to_string()).clone(),
        sender: "self".to_string(),
        receiver: peer_id,
    };

    let mut transfers = state.transfers.lock().await;
    transfers.push(transfer);

    println!("发送文件: {:?} 到 {}", paths, display_peer_id);

    Ok(())
}

#[tauri::command]
async fn get_transfers() -> Result<Vec<serde_json::Value>, String> {
    let state = APP_STATE.clone();
    let transfers = state.transfers.lock().await;

    let result: Vec<serde_json::Value> = transfers
        .iter()
        .map(|t| serde_json::json!({
            "id": t.id,
            "type": t.r#type,
            "state": t.state,
            "progress": t.progress,
            "speed": t.speed,
            "fileName": t.file_name,
            "sender": t.sender,
            "receiver": t.receiver
        }))
        .collect();

    Ok(result)
}

#[tauri::command]
async fn cancel_transfer(id: String) -> Result<(), String> {
    let state = APP_STATE.clone();
    let mut transfers = state.transfers.lock().await;

    if let Some(transfer) = transfers.iter_mut().find(|t| t.id == id) {
        transfer.state = "cancelled".to_string();
    }

    Ok(())
}

#[tauri::command]
async fn accept_transfer(id: String, path: String) -> Result<(), String> {
    let state = APP_STATE.clone();
    let mut transfers = state.transfers.lock().await;

    if let Some(transfer) = transfers.iter_mut().find(|t| t.id == id) {
        transfer.state = "transferring".to_string();
    }

    println!("接受传输 {} 到 {}", id, path);
    Ok(())
}

#[tauri::command]
async fn get_devices() -> Result<Vec<serde_json::Value>, String> {
    let state = APP_STATE.clone();
    let devices = state.devices.lock().await;

    let result: Vec<serde_json::Value> = devices
        .iter()
        .map(|d| serde_json::json!({
            "id": d.id,
            "name": d.name,
            "type": d.device_type,
            "ip": d.ip,
            "port": d.port,
            "version": d.version,
            "status": if d.online { "online" } else { "offline" }
        }))
        .collect();

    Ok(result)
}

/// 接收文件请求（从 LocalSend 客户端）
#[tauri::command]
async fn receive_file_request(
    session_id: String,
    sender_id: String,
    sender_name: String,
    files: Vec<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let state = APP_STATE.clone();

    let incoming_files: Vec<IncomingFile> = files
        .iter()
        .map(|f| IncomingFile {
            id: f.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            name: f.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            size: f.get("size").and_then(|v| v.as_u64()).unwrap_or(0),
            file_type: f.get("fileType").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        })
        .collect();

    let request = FileRequest {
        session_id: session_id.clone(),
        sender_id: sender_id.clone(),
        sender_name,
        files: incoming_files,
    };

    // 保存请求
    let mut requests = state.incoming_requests.lock().await;
    requests.push(request.clone());

    // 通知前端
    let _ = state.request_sender.send(request.clone());

    // 创建传输记录
    let transfer = TransferStatus {
        id: session_id.clone(),
        r#type: "receive".to_string(),
        state: "waiting".to_string(),
        progress: 0.0,
        speed: 0,
        file_name: files.first().and_then(|f| f.get("name").map(|n| n.to_string())).unwrap_or_default(),
        sender: sender_id,
        receiver: "self".to_string(),
    };

    let mut transfers = state.transfers.lock().await;
    transfers.push(transfer);

    Ok(serde_json::json!({
        "sessionId": session_id,
        "accepted": true,
        "token": uuid::Uuid::new_v4().to_string()
    }))
}

/// 获取收到的文件请求
#[tauri::command]
async fn get_file_requests() -> Result<Vec<serde_json::Value>, String> {
    let state = APP_STATE.clone();
    let requests = state.incoming_requests.lock().await;

    let result: Vec<serde_json::Value> = requests
        .iter()
        .map(|r| serde_json::json!({
            "sessionId": r.session_id,
            "senderId": r.sender_id,
            "senderName": r.sender_name,
            "files": r.files.iter().map(|f| serde_json::json!({
                "id": f.id,
                "name": f.name,
                "size": f.size,
                "fileType": f.file_type
            })).collect::<Vec<_>>()
        }))
        .collect();

    Ok(result)
}

/// 拒绝文件请求
#[tauri::command]
async fn reject_file_request(session_id: String) -> Result<(), String> {
    let state = APP_STATE.clone();

    let mut requests = state.incoming_requests.lock().await;
    requests.retain(|r| r.session_id != session_id);

    let mut transfers = state.transfers.lock().await;
    if let Some(t) = transfers.iter_mut().find(|t| t.id == session_id) {
        t.state = "rejected".to_string();
    }

    Ok(())
}

/// 获取监听器端口
#[tauri::command]
async fn get_listener_port() -> Result<u16, String> {
    Ok(LOCALSEND_PORT)
}

/// 设置保存目录
#[tauri::command]
async fn set_download_dir(_path: String) -> Result<(), String> {
    // 保存到配置文件
    Ok(())
}

/// 获取保存目录
#[tauri::command]
async fn get_download_dir() -> Result<String, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    Ok(format!("{}/Downloads/PeerSend", home))
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_version,
            get_status,
            start_daemon,
            stop_daemon,
            discover_peers,
            send_files,
            get_transfers,
            cancel_transfer,
            accept_transfer,
            get_devices,
            receive_file_request,
            get_file_requests,
            reject_file_request,
            get_listener_port,
            set_download_dir,
            get_download_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
