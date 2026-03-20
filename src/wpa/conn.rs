#![allow(dead_code)]

use super::*;
use anyhow::anyhow;
use std::net::Shutdown;
use std::path::PathBuf;
use tokio::net::UnixDatagram;
use tokio::time::Duration;

pub struct Conn<const N: usize> {
    tmp_dir: tempfile::TempDir,
    pub socket: UnixDatagram,
    pub buffer: [u8; N],
}

impl<const N: usize> Conn<N> {
    pub(crate) async fn connect(server_path: &PathBuf, label: &str) -> AppResult<Self> {
        #[cfg(target_os = "linux")]
        {
            let tmp_dir = tempfile::tempdir()?;
            let client_path = tmp_dir.path().join(label);
            let socket = UnixDatagram::bind(client_path)?;
            socket.connect(&server_path)?;
            Ok(Self {
                socket,
                tmp_dir,
                buffer: [0; N],
            })
        }

        #[cfg(target_os = "windows")]
        return Err(anyhow!("Implemented"));
    }

    pub fn shutdown(&self) -> AppResult {
        #[cfg(target_os = "linux")]
        {
            self.socket
                .shutdown(Shutdown::Both)
                .map_err(|e| anyhow!("{}", e))
        }

        #[cfg(target_os = "windows")]
        return Err(anyhow!("Implemented"));
    }
    /// 发送命令 并判断是否返回ok
    pub async fn send_cmd_ok(&mut self, cmd: &[u8]) -> AppResult {
        #[cfg(target_os = "linux")]
        {
            let n = self.socket.send(cmd).await?;
            if n != cmd.len() {
                return Err(anyhow!("socket send err: ok: {n}, cmd_len: {}", cmd.len()));
            }
            self.revc_ok_with_timeout(Duration::from_secs(3)).await
        }
        #[cfg(target_os = "windows")]
        return Err(anyhow!("Implemented"));
    }

    /// 发送命令
    pub async fn send_cmd_result(&mut self, cmd: &[u8]) -> AppResult<String> {
        #[cfg(target_os = "linux")]
        {
            let n = self.socket.send(cmd).await?;
            if n != cmd.len() {
                return Err(anyhow!("socket send err: ok: {n}, cmd_len: {}", cmd.len()));
            }
            self.revc_with_timeout(Duration::from_secs(3)).await
        }
        #[cfg(target_os = "windows")]
        return Err(anyhow!("Implemented"));
    }

    //接收消息超时
    async fn revc_with_timeout(&mut self, timeout: Duration) -> AppResult<String> {
        tokio::select!(
            res = self.revc_string() => res,
            _ =tokio::time::sleep(timeout) => Err(anyhow!("socket recv timeout"))
        )
    }

    async fn revc_string(&mut self) -> AppResult<String> {
        #[cfg(target_os = "linux")]
        {
            match self.socket.recv(&mut self.buffer).await {
                Ok(n) => {
                    let data_str = std::str::from_utf8(&self.buffer[..n])?.trim_end();
                    Ok(data_str.to_owned())
                }
                Err(e) => Err(anyhow!("socket recv err::{}", e)),
            }
        }
        #[cfg(target_os = "windows")]
        return Err(anyhow!("Implemented"));
    }

    //判断是否成功
    async fn revc_is_ok(&mut self) -> AppResult {
        #[cfg(target_os = "linux")]
        {
            match self.socket.recv(&mut self.buffer).await {
                Ok(n) => {
                    let data_str = std::str::from_utf8(&self.buffer[..n])?.trim_end();
                    if data_str.trim() == "OK" {
                        Ok(())
                    } else {
                        log::error!("[wpa] revc_is_ok err: {} ", data_str);
                        Err(anyhow!("{}", data_str))
                    }
                }
                Err(e) => Err(anyhow!("socket recv err::{}", e)),
            }
        }
        #[cfg(target_os = "windows")]
        return Err(anyhow!("Implemented"));
    }

    //接收消息超时
    async fn revc_ok_with_timeout(&mut self, timeout: Duration) -> AppResult {
        tokio::select!(
            res = self.revc_is_ok() => res,
            _ =tokio::time::sleep(timeout) => Err(anyhow!("socket recv timeout"))
        )
    }
}
