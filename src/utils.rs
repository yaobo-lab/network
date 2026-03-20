use crate::IfaceDto;
use anyhow::{Result, anyhow};
use futures::StreamExt;
use if_watch::{IfEvent, tokio::IfWatcher};
use netdev::{Interface, get_interfaces, prelude::InterfaceType};
use std::{
    net::UdpSocket,
    sync::atomic::{AtomicBool, Ordering},
};
use tokio::{
    task::JoinHandle,
    time::{Duration, sleep},
};
//是否能正常上网:true 能正常上班,false 否
static ONLINE: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "linux")]
use crate::linux;

fn iface_set(s: netdev::Interface) -> super::IfaceDto {
    let mut iface = super::IfaceDto {
        index: s.index as i32,
        name: s.name.clone(),
        is_up: s.is_up(),
        is_wifi: s.if_type == InterfaceType::Wireless80211,
        has_dns: !s.dns_servers.is_empty(),
        cur_used: s.default,
        friendly_name: "".to_string(),
        mac_addr: "".to_string(),
        ipv4_addr: "".to_string(),
        ipv6_addr: "".to_string(),
    };

    if let Some(item) = s.mac_addr {
        iface.mac_addr = item.address();
    }
    if !s.ipv4.is_empty() {
        iface.ipv4_addr = s.ipv4[0].addr().to_string();
    }
    if !s.ipv6.is_empty() {
        iface.ipv6_addr = s.ipv6[0].addr().to_string();
    }

    iface
}

//获取网卡列表
pub fn ifaces() -> Vec<IfaceDto> {
    let mut ifaces = vec![];
    let ifacelist = netdev::get_interfaces();
    for s in ifacelist {
        if s.name.is_empty()
            || s.name.eq("p2p0")
            || s.name.eq("docker0")
            || s.is_loopback()
            || !s.is_physical()
        {
            continue;
        }

        if s.if_type != InterfaceType::Wireless80211 && s.if_type != InterfaceType::Ethernet {
            continue;
        }

        ifaces.push(iface_set(s));
    }

    let eth_count = ifaces.iter().filter(|&n| !n.is_wifi).count();
    let wifi_count = ifaces.iter().filter(|&m| m.is_wifi).count();

    let mut wifi_index = 0;
    let mut eth_index = 0;
    for item in ifaces.iter_mut() {
        if item.is_wifi {
            wifi_index += 1;
            item.friendly_name = if wifi_count > 1 {
                format!("无线网络{}", wifi_index)
            } else {
                "无线网络".to_string()
            };
        } else {
            eth_index += 1;
            item.friendly_name = if eth_count > 1 {
                format!("有线网络{}", eth_index)
            } else {
                "有线网络".to_string()
            };
        }
    }
    ifaces.sort_by_key(|p| p.name.clone());
    ifaces
}

//获取原生网卡信息
pub fn get_source_ifaces() -> Vec<Interface> {
    let ifaces = get_interfaces();
    let mut list = vec![];
    for s in ifaces {
        if s.name.is_empty()
            || s.ip_addrs().is_empty()
            || s.name.eq("p2p0")
            || s.name.eq("docker0")
            || s.is_loopback()
            || !s.is_physical()
        {
            continue;
        }

        if s.if_type != InterfaceType::Ethernet && s.if_type != InterfaceType::Wireless80211 {
            continue;
        }
        list.push(s);
    }

    list.sort_by_key(|p| p.name.clone());
    return list;
}

//获取本地ip 地址字符串
pub fn get_local_ips() -> Result<Vec<String>> {
    let mut ips = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    let ifaces = get_source_ifaces();
    for s in ifaces {
        if s.gateway.is_none() {
            continue;
        }
        for ip in s.ipv4 {
            let mut ip_str = ip.to_string();
            if ip_str.contains("/") {
                ip_str = ip_str.split("/").next().unwrap_or_default().to_owned();
            }
            ips.push(ip_str);
        }
    }
    if ips.is_empty() {
        return Err(anyhow!("ip addrss is empty"));
    }
    Ok(ips)
}

//获取本地IP地址
pub fn get_local_ip() -> Result<String> {
    let ips = ["localhost".to_string(), "127.0.0.1".to_string()];
    let ifaces = get_source_ifaces();
    let mut ip_str = String::from("");
    for s in ifaces {
        if s.gateway.is_none() {
            continue;
        }

        if !s.default {
            continue;
        }

        for ip in s.ipv4 {
            ip_str = ip.to_string();
            if ips.contains(&ip_str) {
                continue;
            }

            if ip_str.contains("/") {
                ip_str = ip_str.split("/").next().unwrap_or_default().to_owned();
            }
            break;
        }
    }
    if ip_str.is_empty() {
        return Err(anyhow!("ip address is empty"));
    }

    Ok(ip_str)
}

//获取mac 地址
pub fn get_mac_addr() -> Result<String> {
    let ifaces = get_source_ifaces();
    let mut macs = vec![];
    for s in ifaces.iter() {
        if let Some(mac) = s.mac_addr {
            macs.push(mac.to_string());
        }
    }
    if macs.is_empty() {
        return Err(anyhow::Error::msg("No valid MAC address found"));
    }
    Ok(macs.join("-"))
}

//获取物理网卡mac 地址
pub fn get_physical_mac() -> Result<String> {
    let ifaces = get_source_ifaces();
    let mut macs = vec![];
    for s in ifaces.iter() {
        if s.if_type != InterfaceType::Ethernet {
            continue;
        }
        if let Some(mac) = s.mac_addr {
            macs.push(mac.to_string());
        }
    }
    if macs.is_empty() {
        return Err(anyhow::Error::msg("No valid MAC address found"));
    }
    Ok(macs.join("-"))
}

//停启用网络
pub async fn enable(name: &str, is_up: bool) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        return linux::enable(name, is_up).await;
    }

    #[cfg(target_os = "windows")]
    {
        Err(anyhow!("unsupported windows"))
    }
}

//是否能上
pub fn check_online() -> bool {
    let mut online = false;
    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("114.114.114.114:53").is_ok() {
            online = true;
        }
    }
    if online != ONLINE.load(Ordering::Relaxed) {
        ONLINE.store(online, Ordering::Release);
    }
    online
}

//是否连接上局域网
pub fn lan_is_ok() -> (bool, bool) {
    let online = ONLINE.load(Ordering::Relaxed);
    let mut conn = false;
    let items = ifaces();
    for iface in items.iter() {
        if iface.cur_used {
            conn = true;
            break;
        }
    }
    (conn, online)
}

//打印网卡列表
pub fn ifaces_print() {
    let interfaces = netdev::get_interfaces();
    for interface in interfaces {
        if interface.name.is_empty()
            || interface.name.eq("p2p0")
            || interface.name.eq("docker0")
            || interface.is_loopback()
            || !interface.is_physical()
        {
            continue;
        }
        if interface.if_type != InterfaceType::Wireless80211
            && interface.if_type != InterfaceType::Ethernet
        {
            continue;
        }

        println!("Interface:");
        println!("\tIndex: {}", interface.index);
        println!("\tName: {}", interface.name);
        println!("\tFriendly Name: {:?}", interface.friendly_name);
        println!("\tDescription: {:?}", interface.description);
        println!("\tType: {}", interface.if_type.name());
        println!("\tFlags: {:?}", interface.flags);
        println!("\t\tis UP {}", interface.is_up());
        println!("\t\tis LOOPBACK {}", interface.is_loopback());
        println!("\t\tis MULTICAST {}", interface.is_multicast());
        println!("\t\tis BROADCAST {}", interface.is_broadcast());
        println!("\t\tis POINT TO POINT {}", interface.is_point_to_point());
        println!("\t\tis TUN {}", interface.is_tun());
        println!("\t\tis RUNNING {}", interface.is_running());
        println!("\t\tis PHYSICAL {}", interface.is_physical());
        if let Some(mac_addr) = interface.mac_addr {
            println!("\tMAC Address: {}", mac_addr);
        } else {
            println!("\tMAC Address: (Failed to get mac address)");
        }
        println!("\tIPv4: {:?}", interface.ipv4);
        println!("\tIPv6: {:?}", interface.ipv6);
        println!("\tTransmit Speed: {:?}", interface.transmit_speed);
        println!("\tReceive Speed: {:?}", interface.receive_speed);
        if let Some(gateway) = interface.gateway {
            println!("Gateway");
            println!("\tMAC Address: {}", gateway.mac_addr);
            println!("\tIPv4 Address: {:?}", gateway.ipv4);
            println!("\tIPv6 Address: {:?}", gateway.ipv6);
        } else {
            println!("Gateway: (Not found)");
        }
        println!("DNS Servers: {:?}", interface.dns_servers);
        println!("Default: {}", interface.default);
        println!();
    }
}

//网络初始化
pub async fn wait_ok(mut retry_times: u8) -> Result<()> {
    let mut retry_count = 0;
    if retry_times == 0 {
        retry_times = 3;
    }
    let mut list = vec![];
    while retry_count < retry_times {
        let items = ifaces();
        if items.is_empty() {
            retry_count += 1;
            sleep(Duration::from_secs(2)).await;
            continue;
        }
        list = items;
        break;
    }

    if list.is_empty() {
        return Err(anyhow!("网络硬件初始化失败"));
    } else {
        log::info!("[network] ifaces load ok:{}", list.len());
    }
    let _ = check_online();
    Ok(())
}

pub fn is_online() -> bool {
    ONLINE.load(Ordering::Relaxed)
}

//监听网卡接口变化
pub fn listen_change(callback: fn(bool)) -> Result<JoinHandle<()>> {
    log::info!("netlink listen...");
    let hand = tokio::spawn(async move {
        let mut set = IfWatcher::new().expect("Failed to create IfWatcher");
        let mut ipadd_str = "".to_string();
        loop {
            let event = set.select_next_some().await;

            if event.is_err() {
                continue;
            }

            let mut is_up = true;
            if let Ok(evt) = event {
                let ipnet = match evt {
                    IfEvent::Up(ip) => ip,
                    IfEvent::Down(ip) => {
                        is_up = false;
                        ip
                    }
                };

                if ipnet.addr().is_ipv6() {
                    continue;
                }

                ipadd_str = ipnet.to_string();
            }

            //ipv4 ipv6 本地回环地址 忽略
            if ipadd_str == "127.0.0.1/8" || ipadd_str == "::1/128" {
                continue;
            }

            log::debug!("netlink_listen event {}", ipadd_str);
            callback(is_up);
        }
    });
    Ok(hand)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(dead_code)]
    fn test_interfaces() {
        ifaces_print();
    }

    #[test]
    #[allow(dead_code)]
    fn test_local_ip() {
        let ip = get_local_ip().unwrap();
        log::debug!("local ip: {}", ip);
    }
}
