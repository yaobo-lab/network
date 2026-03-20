use super::dto::*;
use super::IWifi;
use crate::wpa::{Config, client::Client};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use std::{path::Path, sync::Arc};
use std::fmt::Debug;
 
pub struct WpaSupplicant {
    cli: Arc<Client>,
}

impl Debug for WpaSupplicant{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "WpaSupplicant: linux wifi manager")?;     
        Ok(())
    }
}

//指定路径
async fn new_with_iface(iface: &str, event: Option<fn(String)>) -> Result<Client> {
    let iface = format!("/var/run/wpa_supplicant/{}", iface);
    if !Path::new(&iface).exists() {
        return Err(anyhow!("wpa_supplicant socket 文件不存在:{}", iface));
    }
    let cfg = Config {
        server_socket_path: iface.clone(),
        event_callback: event,
        ..Default::default()
    };
    let client = crate::wpa::connect(cfg).await?;
    log::info!("wifi iface:{} setup ok", iface);
    Ok(client)
}

impl WpaSupplicant {
    //wifi 自动检测 初始化
    pub async fn auto_setup(event: Option<fn(String)>) -> Result<Self> {
        let mut wifi_iface = "".to_string();
        let ifaces = crate::utils::ifaces();
        for iface in ifaces.iter() {
            if iface.is_wifi {
                wifi_iface = iface.name.to_owned();
                break;
            }
        }

        if wifi_iface.is_empty() {
            return Err(anyhow!(" wifi ifaces not found, skip wifi setup"));
        }

        let cli = new_with_iface(&wifi_iface, event).await?;
        let cli = Arc::new(cli);
        Ok(Self { cli })
    }
    pub async fn new(iface: &str, event: Option<fn(String)>) -> Result<Self> {
        let cli = new_with_iface(iface, event).await?;
        let cli = Arc::new(cli);
        Ok(WpaSupplicant { cli })
    }
}

#[async_trait]
impl IWifi for WpaSupplicant {
    //扫描 wifi、并等待结果
    async fn scan(&self) -> Result<Vec<ScanResult>> {
        let list = self.cli.scan().await?;
        let mut wifi = vec![];
        for item in list.iter() {
            wifi.push(ScanResult {
                bissid_mac: item.bissid_mac.clone(),
                frequency: item.frequency.clone(),
                ssid_name: item.ssid_name.clone(),
                signal: item.signal,
                flags: item.flags.clone(),
                is_encrypted: item.is_encrypted,
            });
        }
        Ok(wifi)
    }
    //获取已扫描结果
    async fn get_scan_result(&self) -> Result<Vec<ScanResult>> {
        let list = self.cli.get_scan_result().await?;
        Ok(list)
    }
    //获取已保存的wifi
    async fn get_networks(&self) -> Result<Vec<NetworkListResult>> {
        let networks = self.cli.get_networks().await?;
        Ok(networks)
    }

    //移除网络
    async fn remove(&self, network_id: usize) -> Result<()> {
        self.cli.remove_network(network_id).await?;
        // 保存配置
        if let Err(e) = self.cli.save_config().await {
            log::warn!("wifi save config err:{}", e);
        }
        Ok(())
    }
    //断开连接
    async fn disconnect(&self) -> Result<()> {
        self.cli.disconnect().await?;
        Ok(())
    }
    //重连
    async fn reconnect(&self) -> Result<()> {
        self.cli.reconnect().await?;
        Ok(())
    }
    //强制连接id
    async fn select_id(&self, network_id: usize) -> Result<SelectResult> {
        let res = self.cli.select_network(network_id).await?;
        Ok(res)
    }
    //获取状态
    async fn status(&self) -> Result<Status> {
        let res = self.cli.get_status().await?;
        Ok(res)
    }
    //连接wifi
    async fn connect(&self, ssid: String, passwd: String) -> Result<()> {
        let networks = self.cli.get_networks().await?;

        let mut network_id = super::utils::get_networkd_id(&ssid, &networks);

        // 添加网络
        if network_id.is_none() {
            let id = self.cli.add_network().await?;
            network_id = Some(id);
        }

        if network_id.is_none() {
            return Err(anyhow!("无法添加网络"));
        }
        let network_id = network_id.unwrap();

        let req_clone = self.cli.clone();
        let conn: Result<()> = async move {
            // 设置 SSID
            req_clone.set_network_ssid(network_id, ssid.clone()).await?;

            let passwd = passwd.trim().to_owned();

            //设置wifi 密码 psk
            if passwd.is_empty() {
                req_clone
                    .set_network_keymgmt(network_id, KeyMgmt::None)
                    .await?;
            } else {
                req_clone
                    .set_network_psk(network_id, passwd)
                    .await
                    .map_err(|e| {
                        log::warn!("wifi 密码不正确:{}", e);
                        anyhow!("wifi-密码不正确")
                    })?;
            }
            // 启用网络
            if let SelectResult::WrongPsk = req_clone.select_network(network_id).await? {
                return Err(anyhow!("wifi 密码不正确"));
            }

            Ok(())
        }
        .await;

        if let Err(e) = conn {
            let _ = self.cli.remove_network(network_id).await;
            return Err(e);
        }

        // 保存配置
        if let Err(e) = self.cli.save_config().await {
            log::warn!("wifi save config err:{}", e);
        }
        Ok(())
    }
}
