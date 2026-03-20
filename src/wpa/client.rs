#![allow(dead_code)]
use super::AppResult;
use crate::wifi::dto::*;
use anyhow::anyhow;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::oneshot::{self, Sender};
use tokio::time::sleep;
use tokio::time::timeout;

// 请求逻辑
#[derive(Debug)]
pub(crate) enum CmdMsg {
    Custom(String, Sender<AppResult<String>>),
    Status(Sender<AppResult<Status>>),
    Networks(Sender<AppResult<Vec<NetworkListResult>>>),
    Scan(Sender<AppResult<ScanResults>>),
    ScanResult(Sender<AppResult<Vec<ScanResult>>>),
    AddNetwork(Sender<AppResult<usize>>),
    SetNetwork(usize, SetNetwork, Sender<AppResult>),
    SaveConfig(Sender<AppResult>),
    RemoveNetwork(RemoveNetwork, Sender<AppResult>),

    //强制切换到指定 ID 的网络配置（即使已有其他连接）
    SelectNetwork(usize, Sender<AppResult<SelectResult>>),
    //启用指定 ID 的网络配置，但不强制切换（如果当前已有更优连接，可能不会切换）
    EnableNetwork(usize, Sender<AppResult>),
    DisableNetwork(usize, Sender<AppResult>),
    Disconnect(Sender<AppResult>),
    Reconnnect(Sender<AppResult>),
    Shutdown,
}

#[derive(Clone)]
pub struct Client {
    //发送请求channel
    sender: mpsc::Sender<CmdMsg>,
}

impl Client {
    pub fn empty() -> Self {
        let (tx, _rx) = mpsc::channel(1);
        Self { sender: tx }
    }
    pub(crate) fn new(sender: mpsc::Sender<CmdMsg>) -> Client {
        Client { sender }
    }

    fn send(&self, recv: CmdMsg) -> AppResult {
        self.sender
            .try_send(recv)
            .map_err(|_| anyhow!("RequestChannelClosed"))?;
        Ok(())
    }

    pub async fn send_custom(&self, custom: String) -> AppResult<String> {
        let (send, recv) = oneshot::channel();
        self.send(CmdMsg::Custom(custom, send))?;
        match timeout(Duration::from_secs(3), recv).await {
            Ok(Ok(Ok(res))) => {
                log::debug!("[wpa] client cmd: custom recv ok");
                return Ok(res);
            }
            Ok(Ok(Err(err))) => {
                log::error!("[wpa] client cmd:custom recv ok err:{}", err);
                return Err(anyhow!("[wpa] custom ok err: {}", err.to_string()));
            }
            Ok(Err(err)) => {
                log::error!("[wpa] client cmd: custom recv err:{}", err);
                Err(anyhow!("[wpa] custom err: {}", err.to_string()))
            }
            Err(_) => {
                log::error!("[wpa] client cmd: custom recv timeout");
                Err(anyhow!("操作超时"))
            }
        }
    }

    ///触发扫描 并等待获取结果
    pub async fn scan(&self) -> AppResult<Arc<Vec<ScanResult>>> {
        let (send, recv) = oneshot::channel();
        self.send(CmdMsg::Scan(send))?;
        match timeout(Duration::from_secs(6), recv).await {
            Ok(Ok(Ok(res))) => {
                log::debug!("[wpa] client cmd: scan recv ok");
                return Ok(res);
            }
            Ok(Ok(Err(err))) => {
                log::error!("[wpa] client cmd:scan recv ok err:{}", err);
                return Err(anyhow!("[wpa] scan ok err: {}", err.to_string()));
            }
            Ok(Err(err)) => {
                log::error!("[wpa] client cmd: scan recv err:{}", err);
                Err(anyhow!("[wpa] scan err: {}", err.to_string()))
            }
            Err(_) => {
                log::error!("[wpa] client cmd: scan recv timeout");
                Err(anyhow!("操作超时"))
            }
        }
    }

    ///获取扫描结果
    pub async fn get_scan_result(&self) -> AppResult<Vec<ScanResult>> {
        let (send, recv) = oneshot::channel();
        self.send(CmdMsg::ScanResult(send))?;
        match timeout(Duration::from_secs(3), recv).await {
            Ok(Ok(Ok(res))) => {
                log::debug!("[wpa] client cmd: scan result recv ok");
                return Ok(res);
            }
            Ok(Ok(Err(err))) => {
                log::error!("[wpa] client cmd: scan result recv ok err:{}", err);
                return Err(anyhow!("[wpa] scan result ok err: {}", err.to_string()));
            }
            Ok(Err(err)) => {
                log::error!("[wpa] client cmd: scan result recv err:{}", err);
                Err(anyhow!("[wpa] scan result err: {}", err.to_string()))
            }
            Err(_) => {
                log::error!("[wpa] client cmd: scan result recv timeout");
                Err(anyhow!("操作超时"))
            }
        }
    }

    ///查看网络列表及其配置
    pub async fn get_networks(&self) -> AppResult<Vec<NetworkListResult>> {
        let (send, recv) = oneshot::channel();
        self.send(CmdMsg::Networks(send))?;
        match timeout(Duration::from_secs(3), recv).await {
            Ok(Ok(Ok(res))) => {
                log::debug!("[wpa] client cmd: get_networks recv ok");
                return Ok(res);
            }
            Ok(Ok(Err(err))) => {
                log::error!("[wpa] client cmd: get_networks recv ok err:{}", err);
                return Err(anyhow!("[wpa] get_networks ok err: {}", err.to_string()));
            }
            Ok(Err(err)) => {
                log::error!("[wpa] client cmd: get_networks recv err:{}", err);
                Err(anyhow!("[wpa] get_networks err: {}", err.to_string()))
            }
            Err(_) => {
                log::error!("[wpa] client cmd: get_networks recv timeout");
                Err(anyhow!("操作超时"))
            }
        }
    }

    ///查看当前连接状态
    pub async fn get_status(&self) -> AppResult<Status> {
        let (send, recv) = oneshot::channel();
        self.send(CmdMsg::Status(send))?;
        match timeout(Duration::from_secs(3), recv).await {
            Ok(Ok(Ok(res))) => {
                log::debug!("[wpa] client cmd: get_status recv ok");
                return Ok(res);
            }
            Ok(Ok(Err(err))) => {
                log::error!("[wpa] client cmd: get_status recv ok err:{}", err);
                return Err(anyhow!("[wpa] get_status ok err: {}", err.to_string()));
            }
            Ok(Err(err)) => {
                log::error!("[wpa] client cmd: get_status recv err:{}", err);
                Err(anyhow!("[wpa] get_status err: {}", err.to_string()))
            }
            Err(_) => {
                log::error!("[wpa] client cmd: get_status recv timeout");
                Err(anyhow!("操作超时"))
            }
        }
    }

    ///Creates ENABLE_NETWORK request
    pub async fn add_network(&self) -> AppResult<usize> {
        let (send, recv) = oneshot::channel();
        self.send(CmdMsg::AddNetwork(send))?;
        match timeout(Duration::from_secs(3), recv).await {
            Ok(Ok(Ok(res))) => {
                log::debug!("[wpa] client cmd: add_network recv ok");
                return Ok(res);
            }
            Ok(Ok(Err(err))) => {
                log::error!("[wpa] client cmd: add_network recv ok err:{}", err);
                return Err(anyhow!("[wpa] add_network ok err: {}", err.to_string()));
            }
            Ok(Err(err)) => {
                log::error!("[wpa] client cmd: add_network recv err:{}", err);
                Err(anyhow!("[wpa] add_network err: {}", err.to_string()))
            }
            Err(_) => {
                log::error!("[wpa] client cmd: add_network recv timeout");
                Err(anyhow!("操作超时"))
            }
        }
    }
    ///断开网络连接
    pub async fn disconnect(&self) -> AppResult {
        let (send, recv) = oneshot::channel();
        self.send(CmdMsg::Disconnect(send))?;
        match timeout(Duration::from_secs(3), recv).await {
            Ok(Ok(Ok(res))) => {
                log::debug!("[wpa] client cmd: disconnect recv ok");
                return Ok(res);
            }
            Ok(Ok(Err(err))) => {
                log::error!("[wpa] client cmd: disconnect recv ok err:{}", err);
                return Err(anyhow!("[wpa] disconnect ok err: {}", err.to_string()));
            }
            Ok(Err(err)) => {
                log::error!("[wpa] client cmd: disconnect recv err:{}", err);
                Err(anyhow!("[wpa] disconnect err: {}", err.to_string()))
            }
            Err(_) => {
                log::error!("[wpa] client cmd: disconnect recv timeout");
                Err(anyhow!("操作超时"))
            }
        }
    }
    ///重连网络连接
    pub async fn reconnect(&self) -> AppResult {
        let (send, recv) = oneshot::channel();
        self.send(CmdMsg::Reconnnect(send))?;
        match timeout(Duration::from_secs(3), recv).await {
            Ok(Ok(Ok(res))) => {
                log::debug!("[wpa] client cmd: reconnect recv ok");
                return Ok(res);
            }
            Ok(Ok(Err(err))) => {
                log::error!("[wpa] client cmd: reconnect recv ok err:{}", err);
                return Err(anyhow!("[wpa] reconnect ok err: {}", err.to_string()));
            }
            Ok(Err(err)) => {
                log::error!("[wpa] client cmd: reconnect recv err:{}", err);
                Err(anyhow!("[wpa] reconnect err: {}", err.to_string()))
            }
            Err(_) => {
                log::error!("[wpa] client cmd: reconnect recv timeout");
                Err(anyhow!("操作超时"))
            }
        }
    }

    ///启用网络
    /// 启用指定 ID 的网络配置，但不强制切换（如果当前已有更优连接，可能不会切换）
    pub async fn enable_network(&self, network_id: usize) -> AppResult {
        let (send, recv) = oneshot::channel();
        self.send(CmdMsg::EnableNetwork(network_id, send))?;
        match timeout(Duration::from_secs(3), recv).await {
            Ok(Ok(Ok(res))) => {
                log::debug!("[wpa] client cmd: enable_network recv ok");
                return Ok(res);
            }
            Ok(Ok(Err(err))) => {
                log::error!("[wpa] client cmd: enable_network recv ok err:{}", err);
                return Err(anyhow!("[wpa] enable_network ok err: {}", err.to_string()));
            }
            Ok(Err(err)) => {
                log::error!("[wpa] client cmd: enable_network recv err:{}", err);
                Err(anyhow!("[wpa] enable_network err: {}", err.to_string()))
            }
            Err(_) => {
                log::error!("[wpa] client cmd: enable_network recv timeout");
                Err(anyhow!("操作超时"))
            }
        }
    }

    ///禁用网络
    pub async fn disable_network(&self, network_id: usize) -> AppResult {
        let (send, recv) = oneshot::channel();
        self.send(CmdMsg::DisableNetwork(network_id, send))?;
        match timeout(Duration::from_secs(3), recv).await {
            Ok(Ok(Ok(res))) => {
                log::debug!("[wpa] client cmd: disable_network recv ok");
                return Ok(res);
            }
            Ok(Ok(Err(err))) => {
                log::error!("[wpa] client cmd: disable_network recv ok err:{}", err);
                return Err(anyhow!("[wpa] disable_network ok err: {}", err.to_string()));
            }
            Ok(Err(err)) => {
                log::error!("[wpa] client cmd: disable_network recv err:{}", err);
                Err(anyhow!("[wpa] disable_network err: {}", err.to_string()))
            }
            Err(_) => {
                log::error!("[wpa] client cmd: disable_network recv timeout");
                Err(anyhow!("操作超时"))
            }
        }
    }

    pub async fn set_network_psk(&self, network_id: usize, psk: String) -> AppResult {
        let (send, recv) = oneshot::channel();
        self.send(CmdMsg::SetNetwork(network_id, SetNetwork::Psk(psk), send))?;
        match timeout(Duration::from_secs(3), recv).await {
            Ok(Ok(Ok(res))) => {
                log::debug!("[wpa] client cmd: set_network_psk recv ok");
                return Ok(res);
            }
            Ok(Ok(Err(err))) => {
                log::error!("[wpa] client cmd: set_network_psk recv ok err:{}", err);
                return Err(anyhow!("[wpa] set_network_psk ok err: {}", err.to_string()));
            }
            Ok(Err(err)) => {
                log::error!("[wpa] client cmd: set_network_psk recv err:{}", err);
                Err(anyhow!("[wpa] set_network_psk err: {}", err.to_string()))
            }
            Err(_) => {
                log::error!("[wpa] client cmd: set_network_psk recv timeout");
                Err(anyhow!("操作超时"))
            }
        }
    }

    pub async fn set_network_ssid(&self, network_id: usize, ssid: String) -> AppResult {
        let (send, recv) = oneshot::channel();
        self.send(CmdMsg::SetNetwork(network_id, SetNetwork::Ssid(ssid), send))?;
        match timeout(Duration::from_secs(3), recv).await {
            Ok(Ok(Ok(res))) => {
                log::debug!("[wpa] client cmd: set_network_ssid recv ok");
                return Ok(res);
            }
            Ok(Ok(Err(err))) => {
                log::error!("[wpa] client cmd: set_network_ssid recv ok err:{}", err);
                return Err(anyhow!(
                    "[wpa] set_network_ssid ok err: {}",
                    err.to_string()
                ));
            }
            Ok(Err(err)) => {
                log::error!("[wpa] client cmd: set_network_ssid recv err:{}", err);
                Err(anyhow!("[wpa] set_network_ssid err: {}", err.to_string()))
            }
            Err(_) => {
                log::error!("[wpa] client cmd: set_network_ssid recv timeout");
                Err(anyhow!("操作超时"))
            }
        }
    }

    pub async fn set_network_bssid(&self, network_id: usize, bssid: String) -> AppResult {
        let (send, recv) = oneshot::channel();
        self.send(CmdMsg::SetNetwork(
            network_id,
            SetNetwork::Bssid(bssid),
            send,
        ))?;
        match timeout(Duration::from_secs(3), recv).await {
            Ok(Ok(Ok(res))) => {
                log::debug!("[wpa] client cmd: set_network_bssid recv ok");
                return Ok(res);
            }
            Ok(Ok(Err(err))) => {
                log::error!("[wpa] client cmd: set_network_bssid recv ok err:{}", err);
                return Err(anyhow!(
                    "[wpa] set_network_bssid ok err: {}",
                    err.to_string()
                ));
            }
            Ok(Err(err)) => {
                log::error!("[wpa] client cmd: set_network_bssid recv err:{}", err);
                Err(anyhow!("[wpa] set_network_bssid err: {}", err.to_string()))
            }
            Err(_) => {
                log::error!("[wpa] client cmd: set_network_bssid recv timeout");
                Err(anyhow!("操作超时"))
            }
        }
    }

    pub async fn set_network_keymgmt(&self, network_id: usize, mgmt: KeyMgmt) -> AppResult {
        let (send, recv) = oneshot::channel();
        self.send(CmdMsg::SetNetwork(
            network_id,
            SetNetwork::KeyMgmt(mgmt),
            send,
        ))?;
        match timeout(Duration::from_secs(3), recv).await {
            Ok(Ok(Ok(res))) => {
                log::debug!("[wpa] client cmd: set_network_keymgmt recv ok");
                return Ok(res);
            }
            Ok(Ok(Err(err))) => {
                log::error!("[wpa] client cmd: set_network_keymgmt recv ok err:{}", err);
                return Err(anyhow!(
                    "[wpa] set_network_keymgmt ok err: {}",
                    err.to_string()
                ));
            }
            Ok(Err(err)) => {
                log::error!("[wpa] client cmd: set_network_keymgmt recv err:{}", err);
                Err(anyhow!(
                    "[wpa] set_network_keymgmt err: {}",
                    err.to_string()
                ))
            }
            Err(_) => {
                log::error!("[wpa] client cmd: set_network_keymgmt recv timeout");
                Err(anyhow!("操作超时"))
            }
        }
    }

    //保存配置
    pub async fn save_config(&self) -> AppResult {
        let (send, recv) = oneshot::channel();
        self.send(CmdMsg::SaveConfig(send))?;
        match timeout(Duration::from_secs(3), recv).await {
            Ok(Ok(Ok(res))) => {
                log::debug!("[wpa] client cmd: save_config recv ok");
                return Ok(res);
            }
            Ok(Ok(Err(err))) => {
                log::error!("[wpa] client cmd: save_config recv ok err:{}", err);
                return Err(anyhow!("[wpa] save_config ok err: {}", err.to_string()));
            }
            Ok(Err(err)) => {
                log::error!("[wpa] client cmd: save_config recv err:{}", err);
                Err(anyhow!("[wpa] save_config err: {}", err.to_string()))
            }
            Err(_) => {
                log::error!("[wpa] client cmd: save_config recv timeout");
                Err(anyhow!("操作超时"))
            }
        }
    }

    //移除网络
    pub async fn remove_network(&self, id: usize) -> AppResult {
        let (send, recv) = oneshot::channel();
        self.send(CmdMsg::RemoveNetwork(RemoveNetwork::Id(id), send))?;
        match timeout(Duration::from_secs(3), recv).await {
            Ok(Ok(Ok(res))) => {
                log::debug!("[wpa] client cmd: remove_network recv ok");
                return Ok(res);
            }
            Ok(Ok(Err(err))) => {
                log::error!("[wpa] client cmd: remove_network recv ok err:{}", err);
                return Err(anyhow!("[wpa] remove_network ok err: {}", err.to_string()));
            }
            Ok(Err(err)) => {
                log::error!("[wpa] client cmd: remove_network recv err:{}", err);
                Err(anyhow!("[wpa] remove_network err: {}", err.to_string()))
            }
            Err(_) => {
                log::error!("[wpa] client cmd: remove_network recv timeout");
                Err(anyhow!("操作超时"))
            }
        }
    }

    //从列表中移除所有网络
    pub async fn remove_all_networks(&self) -> AppResult {
        let (send, recv) = oneshot::channel();
        self.send(CmdMsg::RemoveNetwork(RemoveNetwork::All, send))?;
        match timeout(Duration::from_secs(3), recv).await {
            Ok(Ok(Ok(res))) => {
                log::debug!("[wpa] client cmd: remove_all_networks recv ok");
                return Ok(res);
            }
            Ok(Ok(Err(err))) => {
                log::error!("[wpa] client cmd: remove_all_networks recv ok err:{}", err);
                return Err(anyhow!(
                    "[wpa] remove_all_networks ok err: {}",
                    err.to_string()
                ));
            }
            Ok(Err(err)) => {
                log::error!("[wpa] client cmd: remove_all_networks recv err:{}", err);
                Err(anyhow!(
                    "[wpa] remove_all_networks err: {}",
                    err.to_string()
                ))
            }
            Err(_) => {
                log::error!("[wpa] client cmd: remove_all_networks recv timeout");
                Err(anyhow!("操作超时"))
            }
        }
    }

    ///选择一个网络进行连接
    /// 强制切换到指定 ID 的网络配置（即使已有其他连接）
    pub async fn select_network(&self, network_id: usize) -> AppResult<SelectResult> {
        let (send, recv) = oneshot::channel();
        self.send(CmdMsg::SelectNetwork(network_id, send))?;
        match timeout(Duration::from_secs(5), recv).await {
            Ok(Ok(Ok(res))) => {
                log::debug!("[wpa] client cmd: select_network recv ok");
                return Ok(res);
            }
            Ok(Ok(Err(err))) => {
                log::error!("[wpa] client cmd: select_network recv ok err:{}", err);
                return Err(anyhow!("[wpa] select_network ok err: {}", err.to_string()));
            }
            Ok(Err(err)) => {
                log::error!("[wpa] client cmd: select_network recv err:{}", err);
                Err(anyhow!("[wpa] select_network err: {}", err.to_string()))
            }
            Err(_) => {
                log::error!("[wpa] client cmd: select_network recv timeout");
                Err(anyhow!("操作超时"))
            }
        }
    }

    pub async fn shutdown(&self) -> AppResult {
        self.send(CmdMsg::Shutdown)?;
        //等待 wap 释放
        sleep(Duration::from_secs(3)).await;
        log::debug!("[wpa] shutdown ok");
        Ok(())
    }
}
