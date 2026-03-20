# network

一个面向 Rust 的网络工具库，主要提供以下能力：

- 枚举本机物理网卡与基础网络状态
- 获取本机 IP、MAC、默认联网网卡等信息
- 监听网卡上下线变化
- 在 Linux 下启用/禁用网卡
- 通过可选 `feature` 提供 Wi-Fi 管理能力
- 在 Linux 下基于 `wpa_supplicant` 管理无线网络

这个项目目前更偏向库（library）而不是独立命令行程序。

## 特性概览

### 通用网络能力

默认特性下即可使用：

- 获取网卡列表：`utils::ifaces`
- 获取原始网卡信息：`utils::get_source_ifaces`
- 获取本机 IP：`utils::get_local_ip` / `utils::get_local_ips`
- 获取 MAC 地址：`utils::get_mac_addr` / `utils::get_physical_mac`
- 判断当前是否联网：`utils::check_online` / `utils::is_online`
- 获取局域网连接与在线状态：`utils::lan_is_ok`
- 监听网卡事件：`utils::listen_change`
- Linux 下启用/禁用网卡：`utils::enable`

### Wi-Fi 能力

启用 `wifi` feature 后可使用统一的 Wi-Fi trait 和数据结构：

- `wifi::IWifi`
- `wifi::ScanResult`
- `wifi::NetworkListResult`
- `wifi::Status`

### WPA / wpa_supplicant 能力

启用 `wpa` feature 且运行在 Linux 时，可通过 Unix Domain Socket 与 `wpa_supplicant` 通信：

- 自动发现无线网卡并初始化：`wifi::linux::WpaSupplicant::auto_setup`
- 指定网卡初始化：`wifi::linux::WpaSupplicant::new`
- 扫描附近 Wi-Fi
- 查看已保存网络
- 连接 / 断开 / 重连 Wi-Fi
- 选择指定 network id
- 保存或移除 `wpa_supplicant` 配置

## 平台说明

- 通用网卡信息读取依赖 `netdev`、`if-watch`，核心逻辑以跨平台接口为主
- `linux` 模块仅在 `target_os = "linux"` 下编译
- `wpa` 模块仅在 `Linux + feature = "wpa"` 下可用
- 当前 Wi-Fi 管理实现实际依赖 Linux 的 `wpa_supplicant` socket，Windows 下未实现对应控制逻辑

## Feature 说明

`Cargo.toml` 中当前提供两个可选 feature：

```toml
[features]
default = []
wifi = ["async-trait"]
wpa = ["libc", "config", "tempfile"]
```

建议按场景启用：

- 只使用基础网络能力：不需要额外 feature
- 只依赖 Wi-Fi trait / DTO：`wifi`
- 使用 Linux `wpa_supplicant` 控制能力：`wifi,wpa`

示例：

```bash
cargo add network
```

如果以本地路径依赖：

```toml
[dependencies]
network = { path = ".", features = ["wifi", "wpa"] }
```

## 依赖环境

当你使用 `wpa` 能力时，需要保证：

- 系统为 Linux
- 已安装并启动 `wpa_supplicant`
- 存在对应网卡的控制 socket，例如 `/var/run/wpa_supplicant/wlan0`
- 当前进程对该 socket 具有访问权限

项目内部默认会尝试连接类似下面的路径：

```text
/var/run/wpa_supplicant/<iface>
```

## 快速开始

### 1. 获取网卡列表

```rust
use network::utils;

fn main() {
    let ifaces = utils::ifaces();
    for iface in ifaces {
        println!(
            "{} {} wifi={} up={} ipv4={}",
            iface.name, iface.friendly_name, iface.is_wifi, iface.is_up, iface.ipv4_addr
        );
    }
}
```

### 2. 获取本机 IP 与 MAC

```rust
use network::utils;

fn main() -> anyhow::Result<()> {
    let ip = utils::get_local_ip()?;
    let mac = utils::get_mac_addr()?;

    println!("local ip: {ip}");
    println!("mac: {mac}");
    Ok(())
}
```

### 3. 监听网络变化

```rust
use network::utils;

fn on_change(is_up: bool) {
    println!("network changed, is_up={is_up}");
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _handle = utils::listen_change(on_change)?;

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}
```

### 4. Linux 下启用或禁用网卡

```rust
#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    network::utils::enable("eth0", true).await?;
    Ok(())
}
```

## Wi-Fi 使用示例

下面示例需要启用 `features = ["wifi", "wpa"]`，并运行在 Linux。

### 初始化 `wpa_supplicant` 客户端

```rust
use network::wifi::{IWifi, linux::WpaSupplicant};

fn on_wifi_event(evt: String) {
    println!("wifi event: {evt}");
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let wifi = WpaSupplicant::auto_setup(Some(on_wifi_event)).await?;

    let status = wifi.status().await?;
    println!("{status:?}");
    Ok(())
}
```

### 扫描附近 Wi-Fi

```rust
use network::wifi::{IWifi, linux::WpaSupplicant};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let wifi = WpaSupplicant::auto_setup(None).await?;
    let results = wifi.scan().await?;

    for item in results {
        println!(
            "ssid={} signal={} level={} encrypted={}",
            item.ssid_name,
            item.signal,
            item.to_level(),
            item.is_encrypted
        );
    }

    Ok(())
}
```

### 连接 Wi-Fi

```rust
use network::wifi::{IWifi, linux::WpaSupplicant};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let wifi = WpaSupplicant::auto_setup(None).await?;

    wifi.connect("MyWiFi".to_string(), "12345678".to_string()).await?;

    Ok(())
}
```

开放网络可传空密码：

```rust
wifi.connect("OpenWiFi".to_string(), "".to_string()).await?;
```

### 查看已保存网络

```rust
use network::wifi::{IWifi, linux::WpaSupplicant};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let wifi = WpaSupplicant::auto_setup(None).await?;
    let networks = wifi.get_networks().await?;

    for item in networks {
        println!(
            "id={} ssid={} connected={} disabled={}",
            item.network_id,
            item.ssid,
            item.is_connected(),
            item.is_disable()
        );
    }

    Ok(())
}
```

### 切换、断开与移除网络

```rust
use network::wifi::{IWifi, linux::WpaSupplicant};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let wifi = WpaSupplicant::auto_setup(None).await?;

    let _ = wifi.select_id(0).await?;
    wifi.disconnect().await?;
    wifi.reconnect().await?;
    wifi.remove(0).await?;

    Ok(())
}
```

## 主要数据结构

### `IfaceDto`

表示网卡的基础信息：

- `index`: 网卡索引
- `name`: 系统网卡名，如 `eth0`、`wlan0`
- `friendly_name`: 友好名称
- `is_wifi`: 是否为无线网卡
- `mac_addr`: MAC 地址
- `ipv4_addr`: IPv4 地址
- `ipv6_addr`: IPv6 地址
- `has_dns`: 是否存在 DNS 配置
- `cur_used`: 是否为当前默认使用网卡
- `is_up`: 是否启用

### `NetInfoDto`

- `ifaces`: 网卡列表
- `connected`: 是否已连接网络

### `ScanResult`

- `bissid_mac`: AP 的 BSSID / MAC
- `frequency`: 频段频率
- `signal`: 信号强度（dBm）
- `flags`: 扫描标志
- `is_encrypted`: 是否加密
- `ssid_name`: Wi-Fi 名称

### `NetworkListResult`

- `network_id`: `wpa_supplicant` 中的网络 ID
- `ssid`: Wi-Fi 名称
- `flags`: 状态标记
- `bssid`: 关联 BSSID

## 设计说明

- 底层网卡枚举基于 `netdev`
- 网络变化监听基于 `if-watch`
- 异步运行时基于 `tokio`
- Wi-Fi 控制通过异步 channel + `wpa_supplicant` socket 完成
- 扫描结果和事件结果会经过简单封装，便于上层业务直接使用

## 注意事项

- `utils::check_online()` 当前通过 UDP 连接 `114.114.114.114:53` 判断外网连通性，这更像“是否可以访问外部网络”的近似判断
- `utils::ifaces()` 会过滤回环网卡、Docker 网卡、`p2p0` 以及非物理网卡
- `WpaSupplicant::auto_setup()` 会选择扫描到的第一个无线网卡
- 使用 `wpa` 能力时，运行权限、socket 权限和系统网络配置都会影响结果
- README 中的示例基于当前代码接口编写，如后续 API 调整请同步更新

## 开发与检查

```bash
cargo check
cargo check --features wifi
cargo check --features "wifi wpa"
```

## 许可证

本项目基于 [MIT License](./LICENSE) 发布。
