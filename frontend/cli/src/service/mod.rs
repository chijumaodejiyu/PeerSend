//! 服务管理模块
//!
//! 提供跨平台服务管理功能

/// 服务安装选项
#[derive(Debug, Default)]
pub struct ServiceInstallOptions {
    pub program: String,
    pub args: Vec<String>,
    pub work_directory: String,
    pub disable_autostart: bool,
    pub description: Option<String>,
}

/// 服务管理器 trait
pub trait ServiceManager {
    fn install(&self, options: &ServiceInstallOptions) -> Result<(), anyhow::Error>;
    fn uninstall(&self, name: &str) -> Result<(), anyhow::Error>;
    fn start(&self, name: &str) -> Result<(), anyhow::Error>;
    fn stop(&self, name: &str) -> Result<(), anyhow::Error>;
    fn status(&self, name: &str) -> Result<ServiceStatus, anyhow::Error>;
}

/// 服务状态
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    Running,
    Stopped,
    NotInstalled,
}

/// 系统服务管理器
pub struct SystemServiceManager;

impl SystemServiceManager {
    /// 创建新的系统服务管理器
    pub fn new() -> Self {
        Self
    }
}

impl ServiceManager for SystemServiceManager {
    fn install(&self, options: &ServiceInstallOptions) -> Result<(), anyhow::Error> {
        #[cfg(target_os = "linux")]
        {
            self.install_systemd(options)
        }
        #[cfg(target_os = "macos")]
        {
            self.install_launchd(options)
        }
        #[cfg(target_os = "windows")]
        {
            self.install_windows(options)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            anyhow::bail!("不支持的操作系统")
        }
    }

    fn uninstall(&self, name: &str) -> Result<(), anyhow::Error> {
        #[cfg(target_os = "linux")]
        {
            self.uninstall_systemd(name)
        }
        #[cfg(target_os = "macos")]
        {
            self.uninstall_launchd(name)
        }
        #[cfg(target_os = "windows")]
        {
            self.uninstall_windows(name)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            anyhow::bail!("不支持的操作系统")
        }
    }

    fn start(&self, name: &str) -> Result<(), anyhow::Error> {
        #[cfg(target_os = "linux")]
        {
            self.start_systemd(name)
        }
        #[cfg(target_os = "macos")]
        {
            self.start_launchd(name)
        }
        #[cfg(target_os = "windows")]
        {
            self.start_windows(name)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            anyhow::bail!("不支持的操作系统")
        }
    }

    fn stop(&self, name: &str) -> Result<(), anyhow::Error> {
        #[cfg(target_os = "linux")]
        {
            self.stop_systemd(name)
        }
        #[cfg(target_os = "macos")]
        {
            self.stop_launchd(name)
        }
        #[cfg(target_os = "windows")]
        {
            self.stop_windows(name)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            anyhow::bail!("不支持的操作系统")
        }
    }

    fn status(&self, name: &str) -> Result<ServiceStatus, anyhow::Error> {
        #[cfg(target_os = "linux")]
        {
            self.status_systemd(name)
        }
        #[cfg(target_os = "macos")]
        {
            self.status_launchd(name)
        }
        #[cfg(target_os = "windows")]
        {
            self.status_windows(name)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            anyhow::bail!("不支持的操作系统")
        }
    }
}

#[cfg(target_os = "linux")]
impl SystemServiceManager {
    fn install_systemd(&self, options: &ServiceInstallOptions) -> Result<(), anyhow::Error> {
        use std::fs;
        use std::path::Path;

        let unit_content = format!(
            r#"[Unit]
Description = {}
After = network.target

[Service]
Type = simple
WorkingDirectory = {}
ExecStart = {} {}
Restart = always
RestartSec = 1

[Install]
WantedBy = multi-user.target
"#,
            options.description.as_deref().unwrap_or("PeerSend Service"),
            options.work_directory,
            options.program,
            options.args.join(" ")
        );

        let unit_path = format!("/etc/systemd/system/{}.service", options.program);
        fs::write(&unit_path, unit_content)
            .with_context(|| format!("Failed to write systemd unit file: {}", unit_path))?;

        std::process::Command::new("systemctl")
            .arg("daemon-reload")
            .output()
            .with_context(|| "Failed to reload systemd daemon")?;

        if !options.disable_autostart {
            std::process::Command::new("systemctl")
                .arg("enable")
                .arg(&format!("{}.service", options.program))
                .output()
                .with_context(|| "Failed to enable service")?;
        }

        Ok(())
    }

    fn uninstall_systemd(&self, name: &str) -> Result<(), anyhow::Error> {
        let _ = std::process::Command::new("systemctl")
            .arg("stop")
            .arg(format!("{}.service", name))
            .output();

        let _ = std::process::Command::new("systemctl")
            .arg("disable")
            .arg(format!("{}.service", name))
            .output();

        let unit_path = format!("/etc/systemd/system/{}.service", name);
        let _ = std::fs::remove_file(&unit_path);

        let _ = std::process::Command::new("systemctl")
            .arg("daemon-reload")
            .output();

        Ok(())
    }

    fn start_systemd(&self, name: &str) -> Result<(), anyhow::Error> {
        std::process::Command::new("systemctl")
            .arg("start")
            .arg(format!("{}.service", name))
            .output()
            .with_context(|| format!("Failed to start service: {}", name))?;
        Ok(())
    }

    fn stop_systemd(&self, name: &str) -> Result<(), anyhow::Error> {
        std::process::Command::new("systemctl")
            .arg("stop")
            .arg(format!("{}.service", name))
            .output()
            .with_context(|| format!("Failed to stop service: {}", name))?;
        Ok(())
    }

    fn status_systemd(&self, name: &str) -> Result<ServiceStatus, anyhow::Error> {
        let output = std::process::Command::new("systemctl")
            .arg("is-active")
            .arg(format!("{}.service", name))
            .output()?;

        if output.status.success() {
            Ok(ServiceStatus::Running)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("inactive") {
                Ok(ServiceStatus::Stopped)
            } else {
                Ok(ServiceStatus::NotInstalled)
            }
        }
    }
}

#[cfg(target_os = "macos")]
impl SystemServiceManager {
    fn install_launchd(&self, options: &ServiceInstallOptions) -> Result<(), anyhow::Error> {
        use std::fs;
        use std::path::Path;

        let plist_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.peersend.{}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        {}
    </array>
    <key>WorkingDirectory</key>
    <string>{}</string>
    <key>RunAtLoad</key>
    <{} />
    <key>KeepAlive</key>
    <true />
</dict>
</plist>
"#,
            options.program,
            options.program,
            options.args.iter().map(|a| format!("<string>{}</string>", a)).collect::<Vec<_>>().join("\n        "),
            options.work_directory,
            if options.disable_autostart { "false" } else { "true" }
        );

        let plist_path = format!("/Library/LaunchDaemons/com.peersend.{}.plist", options.program);
        fs::write(&plist_path, plist_content)
            .with_context(|| format!("Failed to write launchd plist: {}", plist_path))?;

        Ok(())
    }

    fn uninstall_launchd(&self, name: &str) -> Result<(), anyhow::Error> {
        let plist_path = format!("/Library/LaunchDaemons/com.peersend.{}.plist", name);
        let _ = std::fs::remove_file(&plist_path);
        Ok(())
    }

    fn start_launchd(&self, name: &str) -> Result<(), anyhow::Error> {
        let plist_path = format!("/Library/LaunchDaemons/com.peersend.{}.plist", name);
        std::process::Command::new("launchctl")
            .arg("load")
            .arg("-F")
            .arg(&plist_path)
            .output()
            .with_context(|| format!("Failed to start service: {}", name))?;
        Ok(())
    }

    fn stop_launchd(&self, name: &str) -> Result<(), anyhow::Error> {
        let _ = std::process::Command::new("launchctl")
            .arg("unload")
            .arg(format!("/Library/LaunchDaemons/com.peersend.{}.plist", name))
            .output();
        Ok(())
    }

    fn status_launchd(&self, name: &str) -> Result<ServiceStatus, anyhow::Error> {
        let output = std::process::Command::new("launchctl")
            .arg("print")
            .arg(format!("system/com.peersend.{}", name))
            .output()?;

        if output.status.success() {
            Ok(ServiceStatus::Running)
        } else {
            Ok(ServiceStatus::Stopped)
        }
    }
}

#[cfg(target_os = "windows")]
impl SystemServiceManager {
    fn install_windows(&self, _options: &ServiceInstallOptions) -> Result<(), anyhow::Error> {
        // Windows 服务安装需要使用 windows-service crate
        // 这里提供一个简单的实现框架
        anyhow::bail!("Windows 服务安装需要完整的 windows-service 实现")
    }

    fn uninstall_windows(&self, _name: &str) -> Result<(), anyhow::Error> {
        anyhow::bail!("Windows 服务卸载需要完整的 windows-service 实现")
    }

    fn start_windows(&self, _name: &str) -> Result<(), anyhow::Error> {
        let output = std::process::Command::new("sc")
            .arg("start")
            .arg(_name)
            .output()?;
        if output.status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to start service: {}", String::from_utf8_lossy(&output.stderr)))
        }
    }

    fn stop_windows(&self, _name: &str) -> Result<(), anyhow::Error> {
        let output = std::process::Command::new("sc")
            .arg("stop")
            .arg(_name)
            .output()?;
        if output.status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to stop service: {}", String::from_utf8_lossy(&output.stderr)))
        }
    }

    fn status_windows(&self, name: &str) -> Result<ServiceStatus, anyhow::Error> {
        let output = std::process::Command::new("sc")
            .arg("query")
            .arg(name)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("RUNNING") {
            Ok(ServiceStatus::Running)
        } else if stdout.contains("STOPPED") {
            Ok(ServiceStatus::Stopped)
        } else {
            Ok(ServiceStatus::NotInstalled)
        }
    }
}
