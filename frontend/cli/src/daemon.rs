//! EasyTier Daemon 管理模块
//!
//! 管理 easytier-core 的启动、停止、状态查询

use anyhow::{Context, Result};
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
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub network_name: String,
    pub network_secret: Option<String>,
    pub peers: Vec<String>,
    pub dhcp: bool,
    pub ipv4: Option<String>,
    pub enable_wg: bool,
    pub rpc_portal: SocketAddr,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            network_name: "PeerSend".to_string(),
            network_secret: None,
            peers: Vec::new(),
            dhcp: true,
            ipv4: None,
            enable_wg: false,
            rpc_portal: DEFAULT_RPC_PORTAL.parse().unwrap(),
        }
    }
}

/// 守护进程状态
#[derive(Debug, Clone)]
pub struct DaemonStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub peer_count: usize,
    pub network_name: String,
}

impl DaemonStatus {
    pub fn stopped() -> Self {
        Self {
            running: false,
            pid: None,
            peer_count: 0,
            network_name: "".to_string(),
        }
    }
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
            // 检查进程是否存在
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

    /// 通过 systemd 启动
    async fn start_via_systemd(&self, _config: &NetworkConfig) -> Result<bool> {
        // 检查 systemd 是否可用
        let check = Command::new("systemctl")
            .arg("is-active")
            .arg("easytier")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        if check.map(|s| s.success()).unwrap_or(false) {
            println!("通过 systemd 启动 easytier...");
            let output = Command::new("systemctl")
                .arg("start")
                .arg("easytier")
                .output()
                .context("执行 systemctl start 失败")?;

            if output.status.success() {
                self.wait_for_rpc().await?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// 直接启动进程
    async fn start_via_process(&self, config: &NetworkConfig) -> Result<()> {
        println!("启动 easytier-core 进程...");

        // 构建 easytier-core 命令行参数
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

        // 启动进程
        let bin_path = self.find_easytier_binary()?;
        println!("执行: {} {}", bin_path.display(), args.join(" "));

        let child = Command::new(&bin_path)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .context("启动 easytier-core 失败")?;

        // 写入 PID
        let pid = child.id();
        std::fs::write(&self.pid_file, pid.to_string())
            .context("写入 PID 文件失败")?;
        println!("easytier-core 已启动 (PID: {})", pid);

        // 等待 RPC 端口就绪
        self.wait_for_rpc().await?;

        Ok(())
    }

    /// 启动守护进程
    pub async fn start(&self, config: &NetworkConfig) -> Result<()> {
        // 1. 如果已运行，先停止
        if self.is_running() {
            println!("easytier 已在运行，先停止...");
            self.stop().await?;
            sleep(Duration::from_secs(1)).await;
        }

        // 2. 优先尝试 systemd
        if let Ok(true) = self.start_via_systemd(config).await {
            println!("easytier 已通过 systemd 启动");
            return Ok(());
        }

        // 3. 回退到直接进程管理
        self.start_via_process(config).await?;
        println!("easytier 已启动");

        Ok(())
    }

    /// 停止守护进程
    pub async fn stop(&self) -> Result<()> {
        // 1. 尝试 systemd
        let systemd_stop = Command::new("systemctl")
            .arg("stop")
            .arg("easytier")
            .output();

        if systemd_stop.map(|s| s.status.success()).unwrap_or(false) {
            println!("easytier 已通过 systemd 停止");
            self.cleanup_pid();
            return Ok(());
        }

        // 2. 通过 PID 停止
        if let Some(pid) = self.read_pid() {
            println!("发送 SIGTERM 到 PID {}...", pid);
            let _ = Command::new("kill")
                .arg(pid.to_string())
                .output();

            // 等待进程退出
            for i in 0..10 {
                sleep(Duration::from_secs(1)).await;
                if !self.is_running() {
                    break;
                }
                if i >= 9 {
                    println!("发送 SIGKILL...");
                    let _ = Command::new("kill")
                        .arg("-9")
                        .arg(pid.to_string())
                        .output();
                }
            }
            self.cleanup_pid();
            println!("easytier 已停止");
        } else {
            println!("未找到 PID 文件，无法停止");
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
            return DaemonStatus::stopped();
        }

        // 尝试连接 RPC 获取信息
        let pid = self.read_pid();
        let peer_count = self.get_peer_count().await.unwrap_or(0);

        DaemonStatus {
            running: true,
            pid,
            peer_count,
            network_name: self.get_network_name().await.unwrap_or_default(),
        }
    }

    /// 等待 RPC 端口就绪
    async fn wait_for_rpc(&self) -> Result<()> {
        println!("等待 RPC 端口 {} ...", self.rpc_portal);

        for i in 0..30 {
            if self.check_rpc_connection().await {
                println!("RPC 端口已就绪");
                return Ok(());
            }
            sleep(Duration::from_secs(1)).await;
            if i % 5 == 0 {
                println!("等待中... ({})", i + 1);
            }
        }

        anyhow::bail!("RPC 端口连接超时")
    }

    /// 检查 RPC 连接
    async fn check_rpc_connection(&self) -> bool {
        tokio::net::TcpStream::connect(self.rpc_portal).await.is_ok()
    }

    /// 获取对等点数量
    async fn get_peer_count(&self) -> Option<usize> {
        let addr = format!("tcp://{}", self.rpc_portal);
        // 简化实现：尝试连接 RPC
        self.check_rpc_connection().await.then_some(0)
    }

    /// 获取网络名称
    async fn get_network_name(&self) -> Option<String> {
        // 从配置读取
        None
    }

    /// 查找 easytier-core 二进制路径
    fn find_easytier_binary(&self) -> Result<PathBuf> {
        // 优先查找当前项目的 binary
        let project_bin = PathBuf::from("/home/ryanz/Documents/PeerSend/PeerSend/target/debug/easytier-core");
        if project_bin.exists() {
            return Ok(project_bin);
        }

        // 查找系统安装的 binary
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

        // 尝试通过 which 查找
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
}
