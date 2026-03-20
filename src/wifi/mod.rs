/*
 wifi 管理
*/
pub mod dto;
pub use dto::*; 
pub mod utils;
#[cfg(target_os = "linux")]
pub mod linux;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use std::fmt::Debug;

#[async_trait]
pub trait IWifi:Send+Sync+Debug {
    //扫描 wifi、并等待结果
    async fn scan(&self) -> Result<Vec<ScanResult>> {
        Ok(vec![])
    }
    //获取已扫描结果
    async fn get_scan_result(&self) -> Result<Vec<ScanResult>> {
        Ok(vec![])
    }
    //获取已保存的wifi
    async fn get_networks(&self) -> Result<Vec<NetworkListResult>> {
        Ok(vec![])
    }

    //移除网络
    async fn remove(&self, network_id: usize) -> Result<()> {
        Ok(())
    }
    //断开连接
    async fn disconnect(&self) -> Result<()> {
        Ok(())
    }
    //重连
    async fn reconnect(&self) -> Result<()> {
        Ok(())
    }
    //强制连接id
    async fn select_id(&self, network_id: usize) -> Result<SelectResult> {
        Err(anyhow!("unsupported"))
    }
    //获取状态
    async fn status(&self) -> Result<Status> {
        Err(anyhow!("unsupported"))
    }
    //连接wifi
    async fn connect(&self, ssid: String, passwd: String) -> Result<()> {
        Err(anyhow!("unsupported"))
    }
}

//默认未实现wif 
#[derive(Debug)]
pub struct  EmptyWifi;
impl IWifi for EmptyWifi{}