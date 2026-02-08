# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

PeerSend 是一个基于 EasyTier P2P VPN 的文件传输应用，支持跨网络的文件共享。

## 常用命令

```bash
# 构建整个工作空间
cargo build

# 构建发布版本
cargo build --release

# 构建特定包
cargo build -p peersend-cli      # CLI
cargo build -p peersend-gui      # GUI (Tauri)

# 运行 CLI
./target/debug/peersend start --network-name <name> --peers <peer-addrs>
./target/debug/peersend peer list
./target/debug/peersend status

# 运行测试
cargo test --workspace
cargo test -p easytier-core      # 仅核心模块测试
```

## 项目架构

```
PeerSend/
├── easytier-core/           # EasyTier P2P VPN 核心库
│   └── src/
│       ├── common/          # 公共工具 (配置、DNS、网络接口、路由)
│       ├── connector/       # 连接器 (UDP 打孔、WebSocket、TCP)
│       ├── core/            # 核心网络逻辑
│       ├── instance/        # 实例管理
│       ├── peer_center/     # Peer Center 全局发现
│       ├── peers/           # 对等点管理
│       ├── proto/           # Protobuf RPC 定义
│       ├── tunnel/          # 隧道实现 (TCP/KCP/QUIC/WireGuard)
│       ├── rpc_service/     # RPC 服务层
│       └── vpn_portal/      # VPN 门户 (WireGuard 接口)
├── easytier-rpc-build/      # Protobuf RPC 代码生成器
├── protocol/                # PeerSend 文件传输协议
└── frontend/
    ├── cli/                 # CLI 入口 (src/main.rs)
    └── gui/                 # Tauri GUI 应用
```

## 关键技术点

- **RPC 通信**: 默认端口 `127.0.0.1:15888`，使用 tonic/prost
- **异步运行时**: tokio
- **网络隧道**: 支持 TCP、KCP、QUIC、WebSocket、WireGuard
- **NAT 穿透**: UDP 打孔 + STUN
- **路由协议**: OSPF 变体用于 P2P 网络路由
- **GUI 框架**: Tauri 2 + Rust 后端

## 代码布局

- `easytier-core/src/lib.rs`: 库入口和公共导出
- `frontend/cli/src/main.rs`: CLI 入口点，包含 daemon 管理
- `frontend/gui/src-tauri/src/main.rs`: Tauri GUI 入口
- `easytier-core/src/proto/*.proto`: RPC 接口定义

## RPC 服务结构

通过 `easytier::proto::rpc_impl::standalone::StandAloneClient<TcpTunnelConnector>` 连接，提供的 RPC 服务:
- `PeerManageRpc`: 对等点管理
- `ConnectorManageRpc`: 连接器管理
- `VpnPortalRpc`: VPN 门户信息
- `TcpProxyRpc`: TCP/KCP 代理状态
- `StatsRpc`: 统计信息
- `ConfigRpc`: 配置管理
