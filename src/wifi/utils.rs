use crate::wifi::NetworkListResult;

pub fn split<const N: usize>(text: &str, ch: char) -> Option<[&str; N]> {
    let mut count = 0;
    let mut result = [""; N];

    for part in text.split(ch) {
        if let Some(elem) = result.get_mut(count) {
            *elem = part.trim();
            count += 1;
        } else {
            return None;
        }
    }

    if count != N { None } else { Some(result) }
}
#[allow(dead_code)]
pub(crate) fn get_networkd_id(ssid: &str, list: &[NetworkListResult]) -> Option<usize> {
    for r in list.iter() {
        if r.ssid.eq(ssid) {
            return Some(r.network_id);
        }
    }
    None
}

// WIFI 中文过滤
pub fn chniese_filter(text: &str) -> String {
    let mut name = text.to_string();

    if text.contains("\\x") {
        // 创建一个字节向量来存储解码后的字节
        let mut bytes = Vec::new();
        let mut chars = text.chars().peekable();

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

        // 将字节转换为字符串
        if let Ok(msg) = String::from_utf8(bytes) {
            name = msg;
        }
    }

    name
}
/*
    强信号：-30dBm至-60dBm：在此范围内，WiFi连接通常非常稳定，适合进行高清视频流媒体播放、在线游戏等高带宽需求的活动。
    良好信号：-60dBm至-70dBm：虽然可能偶尔会遇到轻微的延迟或缓冲，但大多数日常活动如浏览网页、观看视频等都不会受到太大影响。
    可接受信号：-70dBm至-80dBm：此时可能会遇到一些问题，如网页加载速度变慢、视频缓冲频繁等。
    弱信号：-80dBm至-100dBm：在这个范围内，信号非常弱，用户可能会遇到连接中断、数据传输速度下降等严重问题
*/
pub fn signal_to_level(signal: isize) -> i32 {
    if signal >= -60 {
        return 3;
    }

    if signal >= -70 {
        return 2;
    }
    1
}

// 判断 flags 是否包含加密相关的标识
pub(crate) fn flags_is_encrypted(flags: &str) -> bool {
    if flags.contains("WPA") || flags.contains("WEP") || flags.contains("PSK") {
        return true;
    }
    false
}
