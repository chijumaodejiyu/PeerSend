# PeerSend

基于 EasyTier P2P VPN 的文件传输应用，支持跨网络的文件共享。
本项目使用大量AI Code

## 功能特性

- P2P 文件传输，无需中央服务器
- 基于 EasyTier VPN 核心，支持复杂网络环境
- 支持多平台（CLI + GUI）

## 快速开始

### 前置条件

- Rust 1.89+
- Cargo

### 构建

```bash
# 构建所有组件
cargo build --workspace

# 构建 CLI
cargo build -p peersend-cli

# 构建 GUI
cargo build -p peersend-gui
```

### 使用 CLI

```bash
# 启动网络连接
./target/debug/peersend start --network-name mynetwork --peers peer1.example.com:11011

# 查看对等点
./target/debug/peersend peer list

# 查看状态
./target/debug/peersend status
```

## 项目结构

```
PeerSend/
├── easytier-core/           # EasyTier P2P VPN 核心库
├── easytier-rpc-build/      # Protobuf RPC 代码生成器
├── protocol/                # 文件传输协议定义
└── frontend/
    ├── cli/                 # 命令行界面
    └── gui/                 # 图形界面 (Tauri)
```

## 许可证

[DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE](LICENSE)
