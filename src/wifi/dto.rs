#![allow(dead_code)]

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use std::str;
use std::str::FromStr;
use std::sync::Arc;

//wifi 加密方式
#[derive(Debug)]
pub enum KeyMgmt {
    None,
    WpaPsk,
    WpaEap,
    IEEE8021X,
}
impl Display for KeyMgmt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            KeyMgmt::None => "NONE".to_string(),
            KeyMgmt::WpaPsk => "WPA-PSK".to_string(),
            KeyMgmt::WpaEap => "WPA-EAP".to_string(),
            KeyMgmt::IEEE8021X => "IEEE8021X".to_string(),
        };
        write!(f, "{}", str)
    }
}

/// 网络信息
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetworkListResult {
    pub network_id: usize,
    pub ssid: String,
    pub flags: String,
    pub bssid: String,
}

impl NetworkListResult {
    pub async fn from_str(response: &str) -> Result<Vec<NetworkListResult>> {
        let mut results = Vec::new();
        let split = response.split('\n').skip(1);
        for line in split {
            let mut line_split = line.split_whitespace();

            let Some(network_id) = line_split.next() else {
                return Err(anyhow!("network_id unfound"));
            };
            let network_id = usize::from_str(network_id)?;

            let Some(ssid) = line_split.next() else {
                return Err(anyhow!("ssid unfound"));
            };
            let ssid = super::utils::chniese_filter(ssid);

            let bssid = line_split.next().map(|v| v.to_owned()).unwrap_or_default();

            let flags = line_split.next().map(|v| v.to_owned()).unwrap_or_default();

            results.push(NetworkListResult {
                flags,
                ssid,
                network_id,
                bssid,
            });
        }
        Ok(results)
    }

    pub fn is_connected(&self) -> bool {
        self.flags.to_uppercase().contains("CURRENT")
    }

    pub fn is_disable(&self) -> bool {
        self.flags.to_uppercase().contains("DISABLED")
    }
}

//SCAN_RESULTS 查看wifi扫描结果
pub type ScanResults = Arc<Vec<ScanResult>>;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScanResult {
    //mac 地址
    //BSSID（基本服务集标识）即是基站无线界面所使用的 MAC 地址
    pub bissid_mac: String,
    //frequency指的是无线网络使用的无线电频率 单位为Hz
    // 例如2412即2.412GHz，就是频道1，2437即2.437GHz，则是频道6
    pub frequency: String,
    //signal_level 指无线网络信号强度 通常以分贝毫瓦（dBm）为单位 信号强度越高，表示连接越稳定
    //-60dBm的信号强度比-65dBm的信号强度要强
    pub signal: isize,
    // 包含一系列标志或状态信息，用于描述Wi-Fi网络的当前状态或配置
    pub flags: String,
    //是否加密
    pub is_encrypted: bool,
    //wifi ssid 名称
    //SSID，是让网管人员为服务组合(service set)指定的识别码
    pub ssid_name: String,
}

impl Default for ScanResult {
    fn default() -> Self {
        ScanResult {
            bissid_mac: "".to_string(),
            frequency: "".to_string(),
            signal: 0,
            flags: "".to_string(),
            ssid_name: "".to_string(),
            is_encrypted: false,
        }
    }
}

impl ScanResult {
    pub fn from_str(response: &str) -> Result<Vec<ScanResult>> {
        let mut results = Vec::new();
        let lines = response.lines().skip(1);
        let mut exist = vec![];

        for line in lines {
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() != 5 {
                continue;
            }
            let mac = parts.get(0);
            let frequency = parts.get(1);
            let signal = parts.get(2);
            let flags = parts.get(3);
            let ssid = parts.get(4);

            log::trace!(
                "[wpa] mac:{:?} ssid:{:?} frequency:{:?} signal:{:?} flags:{:?}",
                mac,
                ssid,
                frequency,
                signal,
                flags
            );

            if mac.is_none() || ssid.is_none() {
                continue;
            }

            let mut model = ScanResult::default();

            if let Some(ssid) = ssid {
                let ssid_name = super::utils::chniese_filter(ssid);
                if ssid_name.is_empty() {
                    continue;
                }
                if exist.contains(&ssid_name) {
                    continue;
                } else {
                    exist.push(ssid_name.clone());
                }

                model.ssid_name = ssid_name;
            }

            if let Some(signal) = signal {
                let Ok(signal) = isize::from_str(signal) else {
                    continue;
                };
                // 过滤掉信号强度小于-80dBm的信号
                if signal <= -90 {
                    log::debug!(
                        "[wpa] ssid:{} weak signal:{} [skip]",
                        model.ssid_name,
                        model.signal
                    );
                    continue;
                }
                model.signal = signal;
            }

            if let Some(frequency) = frequency {
                model.frequency = frequency.to_string();
            }

            if let Some(flags) = flags {
                model.flags = flags.to_string();
                model.is_encrypted = super::utils::flags_is_encrypted(flags);
            }

            if let Some(mac) = mac {
                model.bissid_mac = mac.to_string();
            }

            results.push(model);
        }
        Ok(results)
    }

    /*
        强信号：-30dBm至-60dBm：在此范围内，WiFi连接通常非常稳定，适合进行高清视频流媒体播放、在线游戏等高带宽需求的活动。
        良好信号：-60dBm至-70dBm：虽然可能偶尔会遇到轻微的延迟或缓冲，但大多数日常活动如浏览网页、观看视频等都不会受到太大影响。
        可接受信号：-70dBm至-80dBm：此时可能会遇到一些问题，如网页加载速度变慢、视频缓冲频繁等。
        弱信号：-80dBm至-100dBm：在这个范围内，信号非常弱，用户可能会遇到连接中断、数据传输速度下降等严重问题
    */
    pub fn to_level(&self) -> i32 {
        super::utils::signal_to_level(self.signal)
    }
}

// 移除网络
#[derive(Debug)]
pub(crate) enum RemoveNetwork {
    Id(usize),
    All,
}

//设置网络
#[derive(Debug)]
pub(crate) enum SetNetwork {
    Ssid(String),
    Bssid(String),
    Psk(String),
    KeyMgmt(KeyMgmt),
}

//获取wifi 状态
pub type Status = HashMap<String, String>;
#[cfg(feature = "wpa")]
pub(crate) fn parse_status(response: &str) -> Result<Status> {
    use config::{Config, File, FileFormat};
    let config = Config::builder()
        .add_source(File::from_str(response, FileFormat::Ini))
        .build()
        .map_err(|e| anyhow!("{e}"))?;
    Ok(config.try_deserialize::<HashMap<String, String>>().unwrap())
}

#[derive(Debug, Clone)]
pub enum Event {
    ScanComplete,
    Connected,
    Disconnected,
    NetworkNotFound,
    WrongPsk,
    Ready,
    Unknown(String),
}
impl Event {
    pub fn to_string(&self) -> String {
        match self {
            Event::Connected => "connected".to_string(),
            Event::Disconnected => "disconnected".to_string(),
            Event::NetworkNotFound => "network_not_found".to_string(),
            Event::WrongPsk => "wrong_psk".to_string(),
            Event::Ready => "ready".to_string(),
            Event::ScanComplete => "scan_complete".to_string(),
            Event::Unknown(msg) => format!("unknown_msg_{}", msg),
        }
    }
}

#[derive(Debug)]
pub enum SelectResult {
    Success,
    WrongPsk,
}
use std::fmt;
impl fmt::Display for SelectResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            SelectResult::Success => "success",
            SelectResult::WrongPsk => "wrong_psk",
        };
        write!(f, "{s}")
    }
}

#[cfg(test)]
mod test {

    use super::*;
    #[test]
    fn scan_result() {
        let data_str = r#"bssid / frequency / signal level / flags / ssid
        82:ae:54:a5:f2:a7       2462    -79     [WPA-PSK-CCMP][WPA2-PSK-CCMP][ESS]
        68:77:24:24:db:53       2412    -87     [WPA-PSK-CCMP][WPA2-PSK-CCMP][ESS]      Work_24_1_2_0
        7e:45:5a:07:78:5d       2437    -85     [WPA2-PSK-CCMP][ESS]    cjj
        70:79:90:ce:44:81       2437    -92     [WPA2-PSK-CCMP][ESS]    Guest
        70:79:90:ce:49:e0       2412    -95     [WPA2-PSK-CCMP][ESS]    AAM
        70:79:90:ce:49:e1       2412    -95     [WPA2-PSK-CCMP][ESS]    Guest
        00:5a:13:0e:27:a4       2452    -90     [WPA2-PSK-CCMP][ESS]    aimei_test_huawei
        70:79:90:ce:4a:01       2412    -93     [WPA2-PSK-CCMP][ESS]    Guest
        80:ae:54:a5:f2:a7       2462    -78     [ESS]   TP-LINK_F2A7
        70:79:90:ce:4a:00       2412    -96     [WPA2-PSK-CCMP][ESS]    AAM
        34:f7:16:44:88:81       2447    -87     [WPA-PSK-CCMP][WPA2-PSK-CCMP][ESS]      Work_24_8
        70:79:90:ce:49:a0       2447    -94     [WPA2-PSK-CCMP][ESS]    AAM
        ac:ad:4b:54:7f:a9       2412    -98     [WPA-PSK-CCMP][WPA2-PSK-CCMP][ESS]
        70:79:90:ce:44:80       2437    -92     [WPA2-PSK-CCMP][ESS]    AAM
        70:79:90:ce:49:a1       2447    -94     [WPA2-PSK-CCMP][ESS]    Guest
        50:fa:84:8f:a9:82       2472    -96     [WPA-PSK-CCMP][WPA2-PSK-CCMP][ESS]      TPL
        00:4b:f3:73:69:e9       2462    -94     [WPA-PSK-CCMP][WPA2-PSK-CCMP][ESS]      AAM_Live"#;

        let result = ScanResult::from_str(&data_str);

        let list = result.unwrap();

        println!("ok list:{}", list.len());
    }

    #[test]
    fn decode() {
        let escaped_str = "\\xe5\\xb9\\xbf\\xe5\\xb7\\x9e\\xe7\\xbe\\x8e\\xe8\\xbd\\xae";
        // 将转义序列替换为对应的字节
        let byte_str = escaped_str.replace("\\x", "");
        // 将16进制字符串解析成向量
        let bytes: Vec<u8> = (0..byte_str.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&byte_str[i..i + 2], 16).unwrap())
            .collect();

        // 将字节转换为字符串
        match String::from_utf8(bytes) {
            Ok(msg) => {
                println!("Decoded string: {}", msg);
            }
            Err(_) => {}
        }
    }

    #[test]
    fn test_decode_hex_string() {
        let bytes = "\0\0\0\0\0\0\0\0\0\0".as_bytes();
        let all_zeros = bytes.iter().all(|&b| b == 0);
        if all_zeros {
            println!("All zeros");
        } else {
            println!("Not all zeros");
        }

        let str_with_nulls = String::from_utf8_lossy(b"hello world");
        let a = str_with_nulls.to_string();
        println!("Empty string b:{}", a);

        let str_with_nulls = String::from_utf8_lossy(b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00");

        let a = str_with_nulls.trim().to_string();
        println!("Empty string a:{}", a);
        if a == "\"" {
            println!("a is empty");
        }

        let bytes: Vec<u8> = str_with_nulls
            .as_bytes()
            .iter()
            .map(|&b| if b == 0 { b' ' as u8 } else { b })
            .collect();
        let str_without_nulls: String = String::from_utf8(bytes).unwrap(); // 这里是安全的，因为我们只替换了空字符

        println!(
            "String after replacing nulls with spaces: '{}'",
            str_without_nulls
        );

        let c = str_with_nulls.trim();
        println!("c is empty:{}", c.is_empty());

        println!(
            "String with nulls (will appear blank): '{}'",
            str_with_nulls
        );

        println!("str_with_nulls.is_empty() :{} ", str_with_nulls.is_empty());

        let input = "\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        let all_zeros = input.chars().all(|c| c == '\0');
        println!("字符串全部由零组成: {}", all_zeros); // 输出: true
    }

    #[test]
    fn test_decode_hex_string_2() {
        use encoding_rs::*;
        let raw_bytes = "     \\xe4\\xb8\\xad\\xe5\\x9b\\xbd\\xe7\\x94\\xb7\\xe4\\xba\\xba   @v";

        // let raw_bytes = b"\\xe4\\xb8\\xad\\xe5\\x9b\\xbd\\xe7\\x94\\xb7\\xe4\\xba\\xba";
        //let raw_bytes = b"\xE4\xB8\xAD\xE5\x9B\xBD\xE7\x94\xB7\xE4\xBA\xBA";
        let (result, _encoding, _had_errors) = UTF_8.decode(raw_bytes.as_bytes());
        println!("{}", result); // 输出: "中国男人"
    }

    #[test]
    fn test_decode_hex_string_3() {
        let raw_str = "     \\xe4\\xb8\\xad\\xe5\\x9b\\xbd\\xe7\\x94\\xb7\\xe4\\xba\\xba   @v";
        // 创建一个字节向量来存储解码后的字节
        let mut bytes = Vec::new();
        let mut chars = raw_str.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '\\' {
                // 检查是否是以 \x 开头的转义序列
                if let Some('x') = chars.peek() {
                    chars.next(); // 跳过 'x'
                    // 提取接下来的两个字符作为十六进制字节
                    let hex_part: String = chars.by_ref().take(2).collect();
                    if hex_part.len() == 2 {
                        if let Ok(byte) = u8::from_str_radix(&hex_part, 16) {
                            bytes.push(byte);
                            continue;
                        }
                    }
                    // 如果解析失败，保留原始 \x 和后续字符
                    bytes.extend("\\x".bytes());
                    bytes.extend(hex_part.bytes());
                } else {
                    // 如果不是 \x，保留反斜杠
                    bytes.push(c as u8);
                }
            } else {
                // 将普通字符按其 UTF-8 编码添加到字节向量
                bytes.extend(c.to_string().as_bytes());
            }
        }

        // 将字节向量解码为 UTF-8 字符串
        let result = String::from_utf8(bytes).unwrap();
        println!("{}", result); // 输出: "     中国男人   @v"
    }
}
