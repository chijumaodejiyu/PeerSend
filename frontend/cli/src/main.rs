//! PeerSend CLI - 基于 EasyTier CLI
//!
//! P2P 文件传输命令行工具，参考 EasyTier CLI 实现

mod daemon;

use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

use anyhow::Context;
use cidr::Ipv4Inet;
use clap::{Args, Parser, Subcommand};
use daemon::{EasyTierDaemon, NetworkConfig};
use humansize::format_size;
use tabled::settings::{location::ByColumnName, object::Columns, Disable, Modify, Style, Width};
use terminal_size::{terminal_size, Width as TerminalWidth};
use unicode_width::UnicodeWidthStr;

use easytier::{
    peers,
    proto::{
        api::{
            config::{ConfigRpc, ConfigRpcClientFactory},
            instance::{
                instance_identifier::{InstanceSelector, Selector},
                list_peer_route_pair, ConnectorManageRpc, ConnectorManageRpcClientFactory,
                GetStatsRequest, GetVpnPortalInfoRequest, InstanceIdentifier, ListConnectorRequest,
                ListPeerRequest, ListPeerResponse, ListRouteRequest, ListRouteResponse,
                PeerManageRpc, PeerManageRpcClientFactory, ShowNodeInfoRequest, StatsRpc,
                StatsRpcClientFactory, TcpProxyEntryState, TcpProxyEntryTransportType, TcpProxyRpc,
                TcpProxyRpcClientFactory, VpnPortalRpc, VpnPortalRpcClientFactory,
            },
        },
        rpc_impl::standalone::StandAloneClient,
        rpc_types::controller::BaseController,
    },
    tunnel::tcp::TcpTunnelConnector,
    utils::{cost_to_str, PeerRoutePair},
};

use uuid::Uuid;

type Error = anyhow::Error;
type RpcClient = StandAloneClient<TcpTunnelConnector>;

const PEERSEND_VERSION: &str = "0.1.0";

/// 启动网络连接参数
#[derive(Args, Debug)]
struct StartArgs {
    #[arg(short, long, help = "网络名称")]
    network_name: String,

    #[arg(short, long = "secret", help = "网络密钥（可选）")]
    network_secret: Option<String>,

    #[arg(short = 'e', long, help = "对等点地址列表")]
    peers: Vec<String>,

    #[arg(long, help = "使用 DHCP 获取 IP（默认）")]
    dhcp: bool,

    #[arg(long, help = "静态 IPv4 地址")]
    ipv4: Option<String>,

    #[arg(long, help = "启用 WireGuard")]
    enable_wg: bool,

    #[arg(long, help = "RPC 端口（默认 15888）")]
    rpc_portal: Option<SocketAddr>,
}

#[derive(Parser, Debug)]
#[command(name = "peersend", author, version = PEERSEND_VERSION, about, long_about = None)]
struct Cli {
    #[arg(
        short = 'p',
        long,
        default_value = "127.0.0.1:15888",
        help = "easytier-core rpc portal address"
    )]
    rpc_portal: SocketAddr,

    #[arg(short, long, default_value = "false", help = "verbose output")]
    verbose: bool,

    #[arg(
        short = 'o',
        long = "output",
        value_enum,
        default_value = "table",
        help = "output format"
    )]
    output_format: OutputFormat,

    #[arg(
        long = "no-trunc",
        default_value = "false",
        help = "disable column truncation"
    )]
    no_trunc: bool,

    #[command(flatten)]
    instance_select: InstanceSelectArgs,

    #[command(subcommand)]
    sub_command: SubCommand,
}

#[derive(Subcommand, Debug)]
enum SubCommand {
    #[command(about = "启动 PeerSend 网络连接")]
    Start(StartArgs),
    #[command(about = "停止 PeerSend 网络连接")]
    Stop,
    #[command(about = "查看 PeerSend 状态")]
    Status,
    #[command(about = "show peers info")]
    Peer(PeerArgs),
    #[command(about = "manage connectors")]
    Connector(ConnectorArgs),
    #[command(about = "do stun test")]
    Stun,
    #[command(about = "show route info")]
    Route(RouteArgs),
    #[command(about = "show global peers info")]
    PeerCenter,
    #[command(about = "show vpn portal (wireguard) info")]
    VpnPortal,
    #[command(about = "inspect self easytier-core status")]
    Node(NodeArgs),
    #[command(about = "show tcp/kcp proxy status")]
    Proxy,
    #[command(about = "show statistics information")]
    Stats(StatsArgs),
}

#[derive(clap::ValueEnum, Debug, Clone, PartialEq)]
enum OutputFormat {
    Table,
    Json,
}

#[derive(Parser, Debug)]
struct InstanceSelectArgs {
    #[arg(short = 'i', long = "instance-id", help = "the instance id")]
    id: Option<String>,

    #[arg(short = 'n', long = "instance-name", help = "the instance name")]
    name: Option<String>,
}

impl From<&InstanceSelectArgs> for InstanceIdentifier {
    fn from(args: &InstanceSelectArgs) -> Self {
        InstanceIdentifier {
            selector: match &args.id {
                Some(id) => {
                    if let Ok(uuid) = Uuid::parse_str(id) {
                        Some(Selector::Id(uuid.into()))
                    } else {
                        Some(Selector::InstanceSelector(InstanceSelector {
                            name: args.name.clone(),
                        }))
                    }
                }
                None => Some(Selector::InstanceSelector(InstanceSelector {
                    name: args.name.clone(),
                })),
            },
        }
    }
}

#[derive(Args, Debug)]
struct PeerArgs {
    #[command(subcommand)]
    sub_command: Option<PeerSubCommand>,
}

#[derive(Subcommand, Debug)]
enum PeerSubCommand {
    Add,
    Remove,
    List,
    ListForeign,
    ListGlobalForeign,
}

#[derive(Args, Debug)]
struct RouteArgs {
    #[command(subcommand)]
    sub_command: Option<RouteSubCommand>,
}

#[derive(Subcommand, Debug)]
enum RouteSubCommand {
    List,
    Dump,
}

#[derive(Args, Debug)]
struct ConnectorArgs {
    #[arg(short, long)]
    ipv4: Option<String>,

    #[arg(short, long)]
    peers: Vec<String>,

    #[command(subcommand)]
    sub_command: Option<ConnectorSubCommand>,
}

#[derive(Subcommand, Debug)]
enum ConnectorSubCommand {
    Add,
    Remove,
    List,
}

#[derive(Subcommand, Debug)]
enum NodeSubCommand {
    #[command(about = "show node info")]
    Info,
    #[command(about = "show node config")]
    Config,
}

#[derive(Args, Debug)]
struct NodeArgs {
    #[command(subcommand)]
    sub_command: Option<NodeSubCommand>,
}

#[derive(Args, Debug)]
struct StatsArgs {
    #[command(subcommand)]
    sub_command: Option<StatsSubCommand>,
}

#[derive(Subcommand, Debug)]
enum StatsSubCommand {
    /// Show general statistics
    Show,
    /// Show statistics in Prometheus format
    Prometheus,
}

struct CommandHandler<'a> {
    client: tokio::sync::Mutex<RpcClient>,
    verbose: bool,
    output_format: &'a OutputFormat,
    no_trunc: bool,
    instance_selector: InstanceIdentifier,
}

impl CommandHandler<'_> {
    async fn get_peer_manager_client(
        &self,
    ) -> Result<Box<dyn PeerManageRpc<Controller = BaseController>>, Error> {
        Ok(self
            .client
            .lock()
            .await
            .scoped_client::<PeerManageRpcClientFactory<BaseController>>("".to_string())
            .await
            .with_context(|| "failed to get peer manager client")?)
    }

    async fn get_connector_manager_client(
        &self,
    ) -> Result<Box<dyn ConnectorManageRpc<Controller = BaseController>>, Error> {
        Ok(self
            .client
            .lock()
            .await
            .scoped_client::<ConnectorManageRpcClientFactory<BaseController>>("".to_string())
            .await
            .with_context(|| "failed to get connector manager client")?)
    }

    async fn get_peer_center_client(
        &self,
    ) -> Result<Box<dyn PeerManageRpc<Controller = BaseController>>, Error> {
        Ok(self
            .client
            .lock()
            .await
            .scoped_client::<PeerManageRpcClientFactory<BaseController>>("".to_string())
            .await
            .with_context(|| "failed to get peer center client")?)
    }

    async fn get_vpn_portal_client(
        &self,
    ) -> Result<Box<dyn VpnPortalRpc<Controller = BaseController>>, Error> {
        Ok(self
            .client
            .lock()
            .await
            .scoped_client::<VpnPortalRpcClientFactory<BaseController>>("".to_string())
            .await
            .with_context(|| "failed to get vpn portal client")?)
    }

    async fn get_tcp_proxy_client(
        &self,
        transport_type: &str,
    ) -> Result<Box<dyn TcpProxyRpc<Controller = BaseController>>, Error> {
        Ok(self
            .client
            .lock()
            .await
            .scoped_client::<TcpProxyRpcClientFactory<BaseController>>(transport_type.to_string())
            .await
            .with_context(|| "failed to get tcp proxy client")?)
    }

    async fn get_stats_client(
        &self,
    ) -> Result<Box<dyn StatsRpc<Controller = BaseController>>, Error> {
        Ok(self
            .client
            .lock()
            .await
            .scoped_client::<StatsRpcClientFactory<BaseController>>("".to_string())
            .await
            .with_context(|| "failed to get stats client")?)
    }

    async fn get_config_client(
        &self,
    ) -> Result<Box<dyn ConfigRpc<Controller = BaseController>>, Error> {
        Ok(self
            .client
            .lock()
            .await
            .scoped_client::<ConfigRpcClientFactory<BaseController>>("".to_string())
            .await
            .with_context(|| "failed to get config client")?)
    }

    async fn list_peers(&self) -> Result<ListPeerResponse, Error> {
        let client = self.get_peer_manager_client().await?;
        let request = ListPeerRequest {
            instance: Some(self.instance_selector.clone()),
        };
        let response = client.list_peer(BaseController::default(), request).await?;
        Ok(response)
    }

    async fn list_routes(&self) -> Result<ListRouteResponse, Error> {
        let client = self.get_peer_manager_client().await?;
        let request = ListRouteRequest {
            instance: Some(self.instance_selector.clone()),
        };
        let response = client
            .list_route(BaseController::default(), request)
            .await?;
        Ok(response)
    }

    async fn list_peer_route_pair(&self) -> Result<Vec<PeerRoutePair>, Error> {
        let peers = self.list_peers().await?.peer_infos;
        let routes = self.list_routes().await?.routes;
        Ok(list_peer_route_pair(peers, routes))
    }

    async fn handle_peer_list(&self) -> Result<(), Error> {
        #[derive(tabled::Tabled, serde::Serialize)]
        struct PeerTableItem {
            #[tabled(rename = "ipv4")]
            cidr: String,
            #[tabled(skip)]
            ipv4: String,
            hostname: String,
            cost: String,
            #[tabled(rename = "lat(ms)")]
            lat_ms: String,
            #[tabled(rename = "loss")]
            loss_rate: String,
            #[tabled(rename = "rx")]
            rx_bytes: String,
            #[tabled(rename = "tx")]
            tx_bytes: String,
            #[tabled(rename = "tunnel")]
            tunnel_proto: String,
            #[tabled(rename = "NAT")]
            nat_type: String,
            #[tabled(skip)]
            id: String,
            version: String,
        }

        impl From<PeerRoutePair> for PeerTableItem {
            fn from(p: PeerRoutePair) -> Self {
                let route = p.route.clone().unwrap_or_default();
                let lat_ms = if route.cost == 1 {
                    p.get_latency_ms().unwrap_or(0.0)
                } else {
                    route.path_latency_latency_first() as f64
                };
                PeerTableItem {
                    cidr: route.ipv4_addr.map(|ip| ip.to_string()).unwrap_or_default(),
                    ipv4: route
                        .ipv4_addr
                        .map(|ip: easytier::proto::common::Ipv4Inet| ip.address.unwrap_or_default())
                        .map(|ip| ip.to_string())
                        .unwrap_or_default(),
                    hostname: route.hostname.clone(),
                    cost: cost_to_str(route.cost),
                    lat_ms: format!("{:.2}", lat_ms),
                    loss_rate: format!("{:.1}%", p.get_loss_rate().unwrap_or(0.0) * 100.0),
                    rx_bytes: format_size(p.get_rx_bytes().unwrap_or(0), humansize::DECIMAL),
                    tx_bytes: format_size(p.get_tx_bytes().unwrap_or(0), humansize::DECIMAL),
                    tunnel_proto: p.get_conn_protos().unwrap_or_default().join(","),
                    nat_type: p.get_udp_nat_type(),
                    id: route.peer_id.to_string(),
                    version: if route.version.is_empty() {
                        "unknown".to_string()
                    } else {
                        route.version
                    },
                }
            }
        }

        impl From<easytier::proto::api::instance::NodeInfo> for PeerTableItem {
            fn from(p: easytier::proto::api::instance::NodeInfo) -> Self {
                PeerTableItem {
                    cidr: p.ipv4_addr.clone(),
                    ipv4: Ipv4Inet::from_str(&p.ipv4_addr)
                        .map(|ip| ip.address().to_string())
                        .unwrap_or_default(),
                    hostname: p.hostname.clone(),
                    cost: "Local".to_string(),
                    lat_ms: "-".to_string(),
                    loss_rate: "-".to_string(),
                    rx_bytes: "-".to_string(),
                    tx_bytes: "-".to_string(),
                    tunnel_proto: "-".to_string(),
                    nat_type: if let Some(info) = p.stun_info {
                        info.udp_nat_type().as_str_name().to_string()
                    } else {
                        "Unknown".to_string()
                    },
                    id: p.peer_id.to_string(),
                    version: p.version,
                }
            }
        }

        let mut items: Vec<PeerTableItem> = vec![];
        let peer_routes = self.list_peer_route_pair().await?;
        if self.verbose {
            println!("{}", serde_json::to_string_pretty(&peer_routes)?);
            return Ok(());
        }

        let client = self.get_peer_manager_client().await?;
        let node_info = client
            .show_node_info(
                BaseController::default(),
                ShowNodeInfoRequest {
                    instance: Some(self.instance_selector.clone()),
                },
            )
            .await?
            .node_info
            .ok_or(anyhow::anyhow!("node info not found"))?;
        items.push(node_info.into());

        for p in peer_routes {
            items.push(p.into());
        }

        items.sort_by(|a, b| {
            use std::net::{IpAddr, Ipv4Addr};
            let a_is_local = a.cost == "Local";
            let b_is_local = b.cost == "Local";
            if a_is_local != b_is_local {
                return if a_is_local {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                };
            }
            let a_is_public = a.hostname.starts_with(peers::PUBLIC_SERVER_HOSTNAME_PREFIX);
            let b_is_public = b.hostname.starts_with(peers::PUBLIC_SERVER_HOSTNAME_PREFIX);
            if a_is_public != b_is_public {
                return if a_is_public {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                };
            }
            let a_ip = IpAddr::from_str(&a.ipv4).unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
            let b_ip = IpAddr::from_str(&b.ipv4).unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
            match a_ip.cmp(&b_ip) {
                std::cmp::Ordering::Equal => a.hostname.cmp(&b.hostname),
                other => other,
            }
        });

        print_output(
            &items,
            self.output_format,
            &["tunnel", "version"],
            &["version", "tunnel", "nat", "tx", "rx", "loss", "lat(ms)"],
            self.no_trunc,
        )?;

        Ok(())
    }

    async fn handle_connector_list(&self) -> Result<(), Error> {
        let client = self.get_connector_manager_client().await?;
        let request = ListConnectorRequest {
            instance: Some(self.instance_selector.clone()),
        };
        let response = client
            .list_connector(BaseController::default(), request)
            .await?;
        if self.verbose || *self.output_format == OutputFormat::Json {
            println!("{}", serde_json::to_string_pretty(&response.connectors)?);
            return Ok(());
        }
        println!("{:#?}", response);
        Ok(())
    }

    async fn handle_route_list(&self) -> Result<(), Error> {
        #[derive(tabled::Tabled, serde::Serialize)]
        struct RouteTableItem {
            ipv4: String,
            hostname: String,
            proxy_cidrs: String,
            next_hop_ipv4: String,
            next_hop_hostname: String,
            next_hop_lat: f64,
            path_len: i32,
            path_latency: i32,
            next_hop_ipv4_lat_first: String,
            next_hop_hostname_lat_first: String,
            path_len_lat_first: i32,
            path_latency_lat_first: i32,
            version: String,
        }

        let mut items: Vec<RouteTableItem> = vec![];
        let client = self.get_peer_manager_client().await?;
        let node_info = client
            .show_node_info(
                BaseController::default(),
                ShowNodeInfoRequest {
                    instance: Some(self.instance_selector.clone()),
                },
            )
            .await?
            .node_info
            .ok_or(anyhow::anyhow!("node info not found"))?;
        let peer_routes = self.list_peer_route_pair().await?;

        if self.verbose {
            #[derive(serde::Serialize)]
            struct VerboseItem {
                node_info: easytier::proto::api::instance::NodeInfo,
                peer_routes: Vec<PeerRoutePair>,
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&VerboseItem {
                    node_info,
                    peer_routes
                })?
            );
            return Ok(());
        }

        items.push(RouteTableItem {
            ipv4: node_info.ipv4_addr.clone(),
            hostname: node_info.hostname.clone(),
            proxy_cidrs: node_info.proxy_cidrs.join(", "),
            next_hop_ipv4: "-".to_string(),
            next_hop_hostname: "Local".to_string(),
            next_hop_lat: 0.0,
            path_len: 0,
            path_latency: 0,
            next_hop_ipv4_lat_first: "-".to_string(),
            next_hop_hostname_lat_first: "Local".to_string(),
            path_len_lat_first: 0,
            path_latency_lat_first: 0,
            version: node_info.version.clone(),
        });
        for p in peer_routes.iter() {
            let Some(next_hop_pair) = peer_routes.iter().find(|pair| {
                pair.route.clone().unwrap_or_default().peer_id
                    == p.route.clone().unwrap_or_default().next_hop_peer_id
            }) else {
                continue;
            };

            let next_hop_pair_latency_first = peer_routes.iter().find(|pair| {
                pair.route.clone().unwrap_or_default().peer_id
                    == p.route
                        .clone()
                        .unwrap_or_default()
                        .next_hop_peer_id_latency_first
                        .unwrap_or_default()
            });

            let route = p.route.clone().unwrap_or_default();
            items.push(RouteTableItem {
                ipv4: route.ipv4_addr.map(|ip| ip.to_string()).unwrap_or_default(),
                hostname: route.hostname.clone(),
                proxy_cidrs: route.proxy_cidrs.clone().join(",").to_string(),
                next_hop_ipv4: if route.cost == 1 {
                    "DIRECT".to_string()
                } else {
                    next_hop_pair
                        .route
                        .clone()
                        .unwrap_or_default()
                        .ipv4_addr
                        .map(|ip| ip.to_string())
                        .unwrap_or_default()
                },
                next_hop_hostname: if route.cost == 1 {
                    "DIRECT".to_string()
                } else {
                    next_hop_pair
                        .route
                        .clone()
                        .unwrap_or_default()
                        .hostname
                        .clone()
                },
                next_hop_lat: next_hop_pair.get_latency_ms().unwrap_or(0.0),
                path_len: route.cost,
                path_latency: route.path_latency,
                next_hop_ipv4_lat_first: if route.cost_latency_first.unwrap_or_default() == 1 {
                    "DIRECT".to_string()
                } else {
                    next_hop_pair_latency_first
                        .map(|pair| pair.route.clone().unwrap_or_default().ipv4_addr)
                        .unwrap_or_default()
                        .map(|ip| ip.to_string())
                        .unwrap_or_default()
                },
                next_hop_hostname_lat_first: if route.cost_latency_first.unwrap_or_default() == 1 {
                    "DIRECT".to_string()
                } else {
                    next_hop_pair_latency_first
                        .map(|pair| pair.route.clone().unwrap_or_default().hostname)
                        .unwrap_or_default()
                        .clone()
                },
                path_latency_lat_first: route.path_latency_latency_first.unwrap_or_default(),
                path_len_lat_first: route.cost_latency_first.unwrap_or_default(),
                version: if route.version.is_empty() {
                    "unknown".to_string()
                } else {
                    route.version.to_string()
                },
            });
        }

        print_output(
            &items,
            self.output_format,
            &["proxy_cidrs", "version"],
            &["proxy_cidrs", "version"],
            self.no_trunc,
        )?;

        Ok(())
    }
}

fn print_output<T>(
    items: &[T],
    format: &OutputFormat,
    optional_columns: &[&str],
    drop_columns: &[&str],
    no_trunc: bool,
) -> Result<(), Error>
where
    T: tabled::Tabled + serde::Serialize,
{
    match format {
        OutputFormat::Table => {
            let mut table = tabled::Table::new(items);
            table.with(Style::markdown());
            if no_trunc {
                println!("{}", table);
                return Ok(());
            }
            let headers = T::headers()
                .iter()
                .map(|header| header.as_ref().to_string())
                .collect::<Vec<_>>();
            let col_widths = compute_column_widths(items);
            let terminal_width = terminal_table_width();
            let drop_indices = header_indices(&headers, drop_columns);
            let optional_indices = header_indices(&headers, optional_columns);
            let (active, drop_indices, total_width) =
                select_columns_to_drop(terminal_width, &drop_indices, &col_widths);
            apply_column_drops(&mut table, &drop_indices);
            apply_optional_column_truncation(
                &mut table,
                terminal_width,
                &headers,
                &optional_indices,
                &col_widths,
                &active,
                total_width,
            );
            println!("{}", table);
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(items)?);
        }
    }
    Ok(())
}

fn terminal_table_width() -> Option<usize> {
    let (TerminalWidth(width), _) = terminal_size()?;
    let width = width as usize;
    width.checked_sub(1)
}

fn apply_optional_column_truncation(
    table: &mut tabled::Table,
    terminal_width: Option<usize>,
    headers: &[String],
    optional_indices: &[usize],
    col_widths: &[usize],
    active: &[bool],
    total_width: usize,
) {
    let Some(terminal_width) = terminal_width else {
        return;
    };
    if optional_indices.is_empty() || total_width <= terminal_width {
        return;
    }

    let targets = optional_column_targets(terminal_width, optional_indices, col_widths, active);
    for (index, width) in targets {
        if let Some(name) = headers.get(index) {
            table.with(
                Modify::new(ByColumnName::new(name)).with(Width::truncate(width).suffix("...")),
            );
        }
    }
}

fn apply_column_drops(table: &mut tabled::Table, drop_indices: &[usize]) {
    let mut indices = drop_indices.to_vec();
    indices.sort_unstable_by(|a, b| b.cmp(a));
    for index in indices {
        table.with(Disable::column(Columns::single(index)));
    }
}

fn compute_column_widths<T>(items: &[T]) -> Vec<usize>
where
    T: tabled::Tabled,
{
    let mut widths = vec![0usize; T::LENGTH];
    for (idx, header) in T::headers().iter().enumerate() {
        widths[idx] = widths[idx].max(text_width(header.as_ref()));
    }
    for item in items {
        for (idx, field) in item.fields().iter().enumerate() {
            widths[idx] = widths[idx].max(text_width(field.as_ref()));
        }
    }
    widths
}

fn text_width(text: &str) -> usize {
    text.split('\n')
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0)
}

fn header_indices(headers: &[String], names: &[&str]) -> Vec<usize> {
    let mut indices = Vec::new();
    for name in names {
        if let Some(index) = headers
            .iter()
            .position(|header| header.eq_ignore_ascii_case(name))
        {
            if !indices.contains(&index) {
                indices.push(index);
            }
        }
    }
    indices
}

fn select_columns_to_drop(
    terminal_width: Option<usize>,
    drop_indices: &[usize],
    col_widths: &[usize],
) -> (Vec<bool>, Vec<usize>, usize) {
    let mut active = vec![true; col_widths.len()];
    let Some(terminal_width) = terminal_width else {
        let total = table_total_width(col_widths, &active);
        return (active, vec![], total);
    };

    let mut total = table_total_width(col_widths, &active);
    if total <= terminal_width {
        return (active, vec![], total);
    }

    let mut dropped = vec![];
    for &index in drop_indices {
        if total <= terminal_width {
            break;
        }
        if active[index] {
            active[index] = false;
            dropped.push(index);
            total = table_total_width(col_widths, &active);
        }
    }

    (active, dropped, total)
}

fn table_total_width(col_widths: &[usize], active: &[bool]) -> usize {
    let col_count = active.iter().filter(|value| **value).count();
    if col_count == 0 {
        return 0;
    }
    let content_width = col_widths
        .iter()
        .zip(active.iter())
        .filter_map(|(width, keep)| keep.then_some(*width))
        .sum::<usize>();
    content_width + 3 * col_count + 1
}

fn optional_column_targets(
    terminal_width: usize,
    optional_indices: &[usize],
    col_widths: &[usize],
    active: &[bool],
) -> Vec<(usize, usize)> {
    if optional_indices.is_empty() {
        return vec![];
    }

    let mut is_optional = vec![false; col_widths.len()];
    for &index in optional_indices {
        if let Some(flag) = is_optional.get_mut(index) {
            *flag = true;
        }
    }

    let optional_indices = optional_indices
        .iter()
        .copied()
        .filter(|idx| active.get(*idx).copied().unwrap_or(false))
        .collect::<Vec<_>>();
    if optional_indices.is_empty() {
        return vec![];
    }

    let col_count = active.iter().filter(|value| **value).count();
    let overhead = 3 * col_count + 1;
    let mut required_width = overhead;
    for (idx, width) in col_widths.iter().enumerate() {
        if active.get(idx).copied().unwrap_or(false) && !is_optional[idx] {
            required_width += *width;
        }
    }

    let remaining = terminal_width.saturating_sub(required_width);
    let min_width = 6usize;
    let per_column = if remaining == 0 {
        min_width
    } else {
        (remaining / optional_indices.len()).clamp(min_width, 24)
    };

    optional_indices
        .into_iter()
        .map(|idx| (idx, col_widths[idx].min(per_column)))
        .collect()
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    // 处理不需要 RPC 连接的命令
    match &cli.sub_command {
        SubCommand::Start(args) => {
            let rpc_portal = args.rpc_portal.unwrap_or_else(|| {
                "127.0.0.1:15888".parse().unwrap()
            });
            let daemon = EasyTierDaemon::new(Some(rpc_portal));

            let config = NetworkConfig {
                network_name: args.network_name.clone(),
                network_secret: args.network_secret.clone(),
                peers: args.peers.clone(),
                dhcp: args.dhcp,
                ipv4: args.ipv4.clone(),
                enable_wg: args.enable_wg,
                rpc_portal,
            };

            daemon.start(&config).await?;
            println!("PeerSend 网络已启动");
            return Ok(());
        }
        SubCommand::Stop => {
            let daemon = EasyTierDaemon::new(None);
            daemon.stop().await?;
            println!("PeerSend 网络已停止");
            return Ok(());
        }
        SubCommand::Status => {
            let daemon = EasyTierDaemon::new(None);
            let status = daemon.status().await;
            println!("状态: {}", if status.running { "运行中" } else { "已停止" });
            if let Some(pid) = status.pid {
                println!("PID: {}", pid);
            }
            println!("对等点数量: {}", status.peer_count);
            println!("网络名称: {}", status.network_name);
            return Ok(());
        }
        _ => {}
    }

    // 其他命令需要 RPC 连接
    let client = RpcClient::new(TcpTunnelConnector::new(
        format!("tcp://{}:{}", cli.rpc_portal.ip(), cli.rpc_portal.port())
            .parse()
            .unwrap(),
    ));
    let handler = CommandHandler {
        client: tokio::sync::Mutex::new(client),
        verbose: cli.verbose,
        output_format: &cli.output_format,
        no_trunc: cli.no_trunc,
        instance_selector: (&cli.instance_select).into(),
    };

    match cli.sub_command {
        SubCommand::Start(_) | SubCommand::Stop | SubCommand::Status => {
            // 已经在前面处理过了
        }
        SubCommand::Peer(peer_args) => match &peer_args.sub_command {
            Some(PeerSubCommand::Add) => {
                println!("add peer");
            }
            Some(PeerSubCommand::Remove) => {
                println!("remove peer");
            }
            Some(PeerSubCommand::List) => {
                handler.handle_peer_list().await?;
            }
            Some(PeerSubCommand::ListForeign) => {
                println!("list foreign network - not implemented");
            }
            Some(PeerSubCommand::ListGlobalForeign) => {
                println!("list global foreign network - not implemented");
            }
            None => {
                handler.handle_peer_list().await?;
            }
        },
        SubCommand::Connector(conn_args) => match conn_args.sub_command {
            Some(ConnectorSubCommand::Add) => {
                println!("add connector");
            }
            Some(ConnectorSubCommand::Remove) => {
                println!("remove connector");
            }
            Some(ConnectorSubCommand::List) | None => {
                handler.handle_connector_list().await?;
            }
        },
        SubCommand::Route(route_args) => match route_args.sub_command {
            Some(RouteSubCommand::List) | None => handler.handle_route_list().await?,
            Some(RouteSubCommand::Dump) => {
                println!("route dump - not implemented");
            }
        },
        SubCommand::Stun => {
            println!("stun test - not implemented");
        }
        SubCommand::PeerCenter => {
            println!("peer center - not implemented");
        }
        SubCommand::VpnPortal => {
            let client = handler.get_vpn_portal_client().await?;
            let resp = client
                .get_vpn_portal_info(
                    BaseController::default(),
                    GetVpnPortalInfoRequest {
                        instance: Some((&cli.instance_select).into()),
                    },
                )
                .await?
                .vpn_portal_info
                .unwrap_or_default();
            println!("portal_name: {}", resp.vpn_type);
            println!(
                r#"
############### client_config_start ###############
{}
############### client_config_end ###############
"#,
                resp.client_config
            );
            println!("connected_clients:\n{:#?}", resp.connected_clients);
        }
        SubCommand::Node(sub_cmd) => {
            let client = handler.get_peer_manager_client().await?;
            let node_info = client
                .show_node_info(
                    BaseController::default(),
                    ShowNodeInfoRequest {
                        instance: Some((&cli.instance_select).into()),
                    },
                )
                .await?
                .node_info
                .ok_or(anyhow::anyhow!("node info not found"))?;
            match sub_cmd.sub_command {
                Some(NodeSubCommand::Info) | None => {
                    if cli.verbose || cli.output_format == OutputFormat::Json {
                        println!("{}", serde_json::to_string_pretty(&node_info)?);
                        return Ok(());
                    }

                    let stun_info = node_info.stun_info.clone().unwrap_or_default();
                    let ip_list = node_info.ip_list.clone().unwrap_or_default();

                    let mut builder = tabled::builder::Builder::default();
                    builder.push_record(vec!["Virtual IP", &node_info.ipv4_addr]);
                    builder.push_record(vec!["Hostname", &node_info.hostname]);
                    builder.push_record(vec!["Proxy CIDRs", &node_info.proxy_cidrs.join(", ")]);
                    builder.push_record(vec!["Peer ID", &node_info.peer_id.to_string()]);
                    stun_info.public_ip.iter().for_each(|ip| {
                        let Ok(ip) = ip.parse::<IpAddr>() else {
                            return;
                        };
                        if ip.is_ipv4() {
                            builder.push_record(vec!["Public IPv4", &ip.to_string()]);
                        } else {
                            builder.push_record(vec!["Public IPv6", &ip.to_string()]);
                        }
                    });
                    builder.push_record(vec!["UDP Stun Type", &format!("{:?}", stun_info.udp_nat_type())]);
                    ip_list.interface_ipv4s.iter().for_each(|ip| {
                        builder.push_record(vec!["Interface IPv4", &ip.to_string()]);
                    });
                    ip_list.interface_ipv6s.iter().for_each(|ip| {
                        builder.push_record(vec!["Interface IPv6", &ip.to_string()]);
                    });
                    for (idx, l) in node_info.listeners.iter().enumerate() {
                        if l.starts_with("ring") {
                            continue;
                        }
                        builder.push_record(vec![&format!("Listener {}", idx), l]);
                    }

                    println!("{}", builder.build().with(Style::markdown()));
                }
                Some(NodeSubCommand::Config) => {
                    println!("{}", node_info.config);
                }
            }
        }
        SubCommand::Proxy => {
            let mut entries = vec![];

            for client_type in &["tcp", "kcp_src", "kcp_dst", "quic_src", "quic_dst"] {
                let client = handler.get_tcp_proxy_client(client_type).await?;
                let ret = client
                    .list_tcp_proxy_entry(BaseController::default(), Default::default())
                    .await;
                entries.extend(ret.unwrap_or_default().entries);
            }

            if cli.verbose {
                println!("{}", serde_json::to_string_pretty(&entries)?);
                return Ok(());
            }

            #[derive(tabled::Tabled, serde::Serialize)]
            struct TableItem {
                src: String,
                dst: String,
                start_time: String,
                state: String,
                transport_type: String,
            }

            let table_rows = entries
                .iter()
                .map(|e| TableItem {
                    src: SocketAddr::from(e.src.unwrap_or_default()).to_string(),
                    dst: SocketAddr::from(e.dst.unwrap_or_default()).to_string(),
                    start_time: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
                        (e.start_time * 1000) as i64,
                    )
                    .unwrap()
                    .with_timezone(&chrono::Local)
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string(),
                    state: format!("{:?}", TcpProxyEntryState::try_from(e.state).unwrap()),
                    transport_type: format!(
                        "{:?}",
                        TcpProxyEntryTransportType::try_from(e.transport_type).unwrap()
                    ),
                })
                .collect::<Vec<_>>();

            print_output(
                &table_rows,
                &cli.output_format,
                &["start_time", "state", "transport_type"],
                &["start_time", "state", "transport_type"],
                cli.no_trunc,
            )?;
        }
        SubCommand::Stats(stats_args) => match &stats_args.sub_command {
            Some(StatsSubCommand::Show) | None => {
                let client = handler.get_stats_client().await?;
                let request = GetStatsRequest {
                    instance: Some((&cli.instance_select).into()),
                };
                let response = client.get_stats(BaseController::default(), request).await?;

                if cli.output_format == OutputFormat::Json {
                    println!("{}", serde_json::to_string_pretty(&response.metrics)?);
                } else {
                    #[derive(tabled::Tabled, serde::Serialize)]
                    struct StatsTableRow {
                        #[tabled(rename = "Metric Name")]
                        name: String,
                        #[tabled(rename = "Value")]
                        value: String,
                        #[tabled(rename = "Labels")]
                        labels: String,
                    }

                    let table_rows: Vec<StatsTableRow> = response
                        .metrics
                        .iter()
                        .map(|metric| {
                            let labels_str = if metric.labels.is_empty() {
                                "-".to_string()
                            } else {
                                metric
                                    .labels
                                    .iter()
                                    .map(|(k, v)| format!("{}={}", k, v))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            };

                            let formatted_value = if metric.name.contains("bytes") {
                                format_size(metric.value, humansize::BINARY)
                            } else if metric.name.contains("duration") {
                                format!("{} ms", metric.value)
                            } else {
                                metric.value.to_string()
                            };

                            StatsTableRow {
                                name: metric.name.clone(),
                                value: formatted_value,
                                labels: labels_str,
                            }
                        })
                        .collect();

                    print_output(
                        &table_rows,
                        &cli.output_format,
                        &["labels"],
                        &["labels"],
                        cli.no_trunc,
                    )?;
                }
            }
            Some(StatsSubCommand::Prometheus) => {
                println!("prometheus format - not implemented");
            }
        },
    }

    Ok(())
}
