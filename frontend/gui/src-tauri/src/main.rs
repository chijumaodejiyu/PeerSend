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
    time::Duration,
};
use tokio::time::sleep;

const DEFAULT_RPC_PORTAL: &str = "127.0.0.1:15888";
const PID_FILE: &str = "/tmp/peersend-easytier.pid";

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

    /// 检查进程是否运行
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

    /// 读取 PID 文件
    fn read_pid(&self) -> Option<u32> {
        std::fs::read_to_string(&self.pid_file)
            .ok()?
            .trim()
            .parse()
            .ok()
    }

    /// 查找 easytier-core 二进制路径
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

    /// 启动守护进程
    pub async fn start(&self, config: &NetworkConfig) -> Result<()> {
        if self.is_running() {
            self.stop().await?;
            sleep(Duration::from_secs(1)).await;
        }

        // 尝试通过 peersend CLI 启动
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

        // 回退：直接启动 easytier-core
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

    /// 停止守护进程
    pub async fn stop(&self) -> Result<()> {
        // 尝试通过 peersend CLI 停止
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

        // 回退：直接通过 PID 停止
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

    /// 清理 PID 文件
    fn cleanup_pid(&self) {
        let _ = std::fs::remove_file(&self.pid_file);
    }

    /// 获取守护进程状态
    pub async fn status(&self) -> DaemonStatus {
        if !self.is_running() {
            return DaemonStatus {
                running: false,
                pid: None,
                peer_count: 0,
                network_name: "".to_string(),
            };
        }

        DaemonStatus {
            running: true,
            pid: self.read_pid(),
            peer_count: 0,
            network_name: "".to_string(),
        }
    }

    /// 等待 RPC 端口就绪
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
async fn discover_peers() -> Vec<serde_json::Value> {
    vec![]
}

#[tauri::command]
async fn send_files(_window: tauri::Window, _paths: Vec<String>, _peer_id: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
async fn get_transfers() -> Vec<serde_json::Value> {
    vec![]
}

#[tauri::command]
async fn cancel_transfer(_id: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
async fn accept_transfer(_id: String, _path: String) -> Result<(), String> {
    Ok(())
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
            accept_transfer
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
