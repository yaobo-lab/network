use serde::{Deserialize, Serialize};
//网卡信息
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct IfaceDto {
    pub index: i32,
    // 网卡名称
    pub name: String,
    // 网卡显示名称
    pub friendly_name: String,
    /// 使用的网口类型
    pub is_wifi: bool,
    /// MAC 地址
    pub mac_addr: String,
    pub ipv4_addr: String,
    pub ipv6_addr: String,
    /// 是否有dns
    pub has_dns: bool,
    /// 是否默认使用网络 default
    pub cur_used: bool,
    /// 是否已经启动
    pub is_up: bool,
}

//网络接口信息
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NetInfoDto {
    pub ifaces: Vec<IfaceDto>,
    pub connected: bool,
}
