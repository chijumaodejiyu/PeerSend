//! Windows 平台特定功能
//!
//! 复制自 easytier-core/src/arch/windows.rs
//! 包含防火墙规则管理等功能

use std::{io, mem::ManuallyDrop, net::SocketAddr, os::windows::io::AsRawSocket};

use anyhow::Context;
use windows::{
    core::BSTR,
    Win32::{
        Foundation::{BOOL, FALSE},
        NetworkManagement::WindowsFirewall::{
            INetFwPolicy2, INetFwRule, NET_FW_ACTION_ALLOW, NET_FW_PROFILE2_DOMAIN,
            NET_FW_PROFILE2_PRIVATE, NET_FW_PROFILE2_PUBLIC, NET_FW_RULE_DIR_IN,
            NET_FW_RULE_DIR_OUT,
        },
        Networking::WinSock::{
            htonl, setsockopt, WSAGetLastError, WSAIoctl, IPPROTO_IP, IPPROTO_IPV6,
            IPV6_UNICAST_IF, IP_UNICAST_IF, SIO_UDP_CONNRESET, SOCKET, SOCKET_ERROR,
        },
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
        },
        System::Ole::{SafeArrayCreateVector, SafeArrayPutElement},
        System::Variant::{VARENUM, VARIANT, VT_ARRAY, VT_BSTR, VT_VARIANT},
    },
};

/// 禁用 UDP 连接重置错误
///
/// 在 Windows 上，UdpSocket 的 recv_from 可能因为发送到的目标不存在而返回 WSAECONNRESET 错误。
/// 这在某些情况下是正常的，可以忽略。
pub fn disable_connection_reset<S: AsRawSocket>(socket: &S) -> io::Result<()> {
    let handle = SOCKET(socket.as_raw_socket() as usize);

    unsafe {
        let mut bytes_returned: u32 = 0;
        let enable: BOOL = FALSE;

        let ret = WSAIoctl(
            handle,
            SIO_UDP_CONNRESET,
            Some(&enable as *const _ as *const std::ffi::c_void),
            std::mem::size_of_val(&enable) as u32,
            None,
            0,
            &mut bytes_returned as *mut _,
            None,
            None,
        );

        if ret == SOCKET_ERROR {
            let err_code = WSAGetLastError();
            return Err(std::io::Error::from_raw_os_error(err_code.0));
        }
    }

    Ok(())
}

/// 获取网络接口数量
pub fn interface_count() -> io::Result<usize> {
    let ifaces = network_interface::NetworkInterface::show().map_err(|e| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Failed to get interfaces. error: {}", e),
        )
    })?;
    Ok(ifaces.len())
}

/// 查找网络接口索引
pub fn find_interface_index(iface_name: &str) -> io::Result<u32> {
    let ifaces = network_interface::NetworkInterface::show().map_err(|e| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Failed to find interface {}, error: {}", iface_name, e),
        )
    })?;
    if let Some(iface) = ifaces.iter().find(|iface| iface.name == iface_name) {
        return Ok(iface.index);
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        iface_name.to_string(),
    ))
}

/// 设置套接字的单播接口
pub fn set_ip_unicast_if<S: AsRawSocket>(
    socket: &S,
    addr: &SocketAddr,
    iface: &str,
) -> io::Result<()> {
    let handle = SOCKET(socket.as_raw_socket() as usize);
    let if_index = find_interface_index(iface)?;

    unsafe {
        let ret = match addr {
            SocketAddr::V4(..) => {
                let if_index = htonl(if_index);
                let if_index_bytes = if_index.to_ne_bytes();
                setsockopt(handle, IPPROTO_IP.0, IP_UNICAST_IF, Some(&if_index_bytes))
            }
            SocketAddr::V6(..) => {
                let if_index_bytes = if_index.to_ne_bytes();
                setsockopt(
                    handle,
                    IPPROTO_IPV6.0,
                    IPV6_UNICAST_IF,
                    Some(&if_index_bytes),
                )
            }
        };

        if ret == SOCKET_ERROR {
            let err = std::io::Error::from_raw_os_error(WSAGetLastError().0);
            return Err(err);
        }
    }

    Ok(())
}

/// 为 Windows 套接字设置必要的选项
pub fn setup_socket_for_win<S: AsRawSocket>(
    socket: &S,
    bind_addr: &SocketAddr,
    bind_dev: Option<String>,
    is_udp: bool,
) -> io::Result<()> {
    if is_udp {
        disable_connection_reset(socket)?;
    }

    if let Some(iface) = bind_dev {
        set_ip_unicast_if(socket, bind_addr, iface.as_str())?;
    }

    Ok(())
}

struct ComInitializer;

impl ComInitializer {
    fn new() -> windows::core::Result<Self> {
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED)? };
        Ok(Self)
    }
}

impl Drop for ComInitializer {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

/// 将当前程序添加到防火墙允许列表
pub fn add_self_to_firewall_allowlist(inbound: bool) -> anyhow::Result<()> {
    let _com = ComInitializer::new()?;
    let policy: INetFwPolicy2 = unsafe {
        CoCreateInstance(
            &windows::Win32::NetworkManagement::WindowsFirewall::NetFwPolicy2,
            None,
            CLSCTX_ALL,
        )
    }?;

    let rule: INetFwRule = unsafe {
        CoCreateInstance(
            &windows::Win32::NetworkManagement::WindowsFirewall::NetFwRule,
            None,
            CLSCTX_ALL,
        )
    }?;

    let exe_path = std::env::current_exe()
        .with_context(|| "Failed to get current executable path")?
        .to_string_lossy()
        .replace(r"\\?\", "");

    let name = BSTR::from(format!(
        "PeerSend {} ({})",
        exe_path,
        if inbound { "Inbound" } else { "Outbound" }
    ));
    let desc = BSTR::from("Allow PeerSend for subnet proxy and kcp proxy");
    let app_path = BSTR::from(&exe_path);

    unsafe {
        rule.SetName(&name)?;
        rule.SetDescription(&desc)?;
        rule.SetApplicationName(&app_path)?;
        rule.SetAction(NET_FW_ACTION_ALLOW)?;
        if inbound {
            rule.SetDirection(NET_FW_RULE_DIR_IN)?;
        } else {
            rule.SetDirection(NET_FW_RULE_DIR_OUT)?;
        }
        rule.SetEnabled(windows::Win32::Foundation::VARIANT_TRUE)?;
        rule.SetProfiles(
            NET_FW_PROFILE2_PRIVATE.0 | NET_FW_PROFILE2_PUBLIC.0 | NET_FW_PROFILE2_DOMAIN.0,
        )?;
        rule.SetGrouping(&BSTR::from("PeerSend"))?;

        let rules = policy.Rules()?;
        rules.Remove(&name)?;
        rules.Add(&rule)?;
    }

    Ok(())
}

/// 添加程序到防火墙（入站和出站）
pub fn add_program_to_firewall() -> anyhow::Result<()> {
    add_self_to_firewall_allowlist(true)?;
    add_self_to_firewall_allowlist(false)?;
    Ok(())
}

/// 为指定网络接口添加防火墙规则
pub fn add_interface_to_firewall_allowlist(interface_name: &str) -> anyhow::Result<()> {
    let _com = ComInitializer::new()?;
    let policy: INetFwPolicy2 = unsafe {
        CoCreateInstance(
            &windows::Win32::NetworkManagement::WindowsFirewall::NetFwPolicy2,
            None,
            CLSCTX_ALL,
        )
    }?;

    add_protocol_firewall_rules(&policy, interface_name, "TCP", Some(6))?;
    add_protocol_firewall_rules(&policy, interface_name, "UDP", Some(17))?;
    add_protocol_firewall_rules(&policy, interface_name, "ICMP", Some(1))?;
    add_protocol_firewall_rules(&policy, interface_name, "ALL", None)?;

    Ok(())
}

/// 添加特定协议的防火墙规则
fn add_protocol_firewall_rules(
    policy: &INetFwPolicy2,
    interface_name: &str,
    protocol_name: &str,
    protocol_number: Option<i32>,
) -> anyhow::Result<()> {
    for (is_inbound, direction_name) in [(true, "Inbound"), (false, "Outbound")] {
        let rule: INetFwRule = unsafe {
            CoCreateInstance(
                &windows::Win32::NetworkManagement::WindowsFirewall::NetFwRule,
                None,
                CLSCTX_ALL,
            )
        }?;

        let rule_name = format!(
            "PeerSend {} - {} Protocol ({})",
            interface_name, protocol_name, direction_name
        );
        let description = format!(
            "Allow {} traffic on PeerSend interface {}",
            protocol_name, interface_name
        );

        let name_bstr = BSTR::from(&rule_name);
        let desc_bstr = BSTR::from(&description);

        unsafe {
            rule.SetName(&name_bstr)?;
            rule.SetDescription(&desc_bstr)?;
            if let Some(protocol_number) = protocol_number {
                rule.SetProtocol(protocol_number)?;
            }
            rule.SetAction(NET_FW_ACTION_ALLOW)?;

            if is_inbound {
                rule.SetDirection(NET_FW_RULE_DIR_IN)?;
            } else {
                rule.SetDirection(NET_FW_RULE_DIR_OUT)?;
            }

            rule.SetEnabled(windows::Win32::Foundation::VARIANT_TRUE)?;
            rule.SetProfiles(
                NET_FW_PROFILE2_PRIVATE.0 | NET_FW_PROFILE2_PUBLIC.0 | NET_FW_PROFILE2_DOMAIN.0,
            )?;
            rule.SetGrouping(&BSTR::from("PeerSend"))?;

            let interface_bstr = BSTR::from(interface_name);
            let interface_array = SafeArrayCreateVector(VT_VARIANT, 0, 1);
            if interface_array.is_null() {
                return Err(anyhow::anyhow!("Failed to create SAFEARRAY"));
            }

            let index = 0i32;
            let mut variant_interface = VARIANT::default();
            (*variant_interface.Anonymous.Anonymous).vt = VT_BSTR;
            (*variant_interface.Anonymous.Anonymous).Anonymous.bstrVal =
                ManuallyDrop::new(interface_bstr);

            SafeArrayPutElement(
                interface_array,
                &index as *const _ as *const i32,
                &variant_interface as *const _ as *const std::ffi::c_void,
            )?;

            let mut interface_variant = VARIANT::default();
            (*interface_variant.Anonymous.Anonymous).vt = VARENUM(VT_ARRAY.0 | VT_VARIANT.0);
            (*interface_variant.Anonymous.Anonymous).Anonymous.parray = interface_array;

            rule.SetInterfaces(interface_variant)?;

            let rules = policy.Rules()?;
            rules.Remove(&name_bstr)?;
            rules.Add(&rule)?;
        }
    }

    Ok(())
}

/// 移除指定接口的防火墙规则
pub fn remove_interface_firewall_rules(interface_name: &str) -> anyhow::Result<()> {
    let _com = ComInitializer::new()?;
    let policy: INetFwPolicy2 = unsafe {
        CoCreateInstance(
            &windows::Win32::NetworkManagement::WindowsFirewall::NetFwPolicy2,
            None,
            CLSCTX_ALL,
        )
    }?;

    let rules = unsafe { policy.Rules()? };

    for protocol_name in ["TCP", "UDP", "ICMP", "ALL"] {
        for direction in ["Inbound", "Outbound"] {
            let rule_name = format!(
                "PeerSend {} - {} Protocol ({})",
                interface_name, protocol_name, direction
            );
            let name_bstr = BSTR::from(&rule_name);
            unsafe {
                let _ = rules.Remove(&name_bstr);
            }
        }
    }

    Ok(())
}

/// 移除程序相关的防火墙规则
pub fn remove_program_from_firewall() -> anyhow::Result<()> {
    let _com = ComInitializer::new()?;
    let policy: INetFwPolicy2 = unsafe {
        CoCreateInstance(
            &windows::Win32::NetworkManagement::WindowsFirewall::NetFwPolicy2,
            None,
            CLSCTX_ALL,
        )
    }?;

    let rules = unsafe { policy.Rules()? };

    for direction in ["Inbound", "Outbound"] {
        let rule_name = format!("PeerSend * ({})", direction);
        let name_bstr = BSTR::from(&rule_name);
        unsafe {
            let _ = rules.Remove(&name_bstr);
        }
    }

    Ok(())
}
