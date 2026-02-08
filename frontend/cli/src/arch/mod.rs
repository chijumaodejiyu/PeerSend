//! 跨平台架构适配模块
//!
//! 复制自 easytier-core/src/arch/mod.rs

#[cfg(target_os = "windows")]
pub mod windows;
