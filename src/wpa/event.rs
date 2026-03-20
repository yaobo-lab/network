#![allow(dead_code)]
#![allow(unused_mut)]
use super::*;
use crate::wifi::dto::*;
use anyhow::anyhow;
use std::io;
use std::path::PathBuf;

pub(crate) struct EventConn {
    conn: Conn<1024>,
    sender: mpsc::Sender<Event>,
}

impl EventConn {
    pub(crate) async fn new(server_path: &PathBuf) -> AppResult<(Self, mpsc::Receiver<Event>)> {
        let conn = Conn::connect(server_path, "wap_cli_evt.sock").await?;
        let (evt_sender, evt_receiver) = mpsc::channel(32);
        Ok((
            Self {
                conn,
                sender: evt_sender,
            },
            evt_receiver,
        ))
    }

    async fn send(&self, event: Event) -> AppResult {
        self.sender
            .send(event)
            .await
            .map_err(|_| anyhow!("EventChannelClosed"))?;
        Ok(())
    }

    //启动鉴听进入sokcet 的 事件
    pub(crate) async fn listening(mut self) -> AppResult {
        log::debug!("[wpa] event listening...");

        #[cfg(target_os = "linux")]
        {
            //进入事件
            self.conn.socket.send(b"ATTACH").await?;

            loop {
                self.conn.socket.readable().await?;

                match self.conn.socket.try_recv(&mut self.conn.buffer) {
                    Ok(n) => {
                        let msg = std::str::from_utf8(&self.conn.buffer[..n])?.trim_end();

                        log::trace!("[wpa] revc event:{}", msg);

                        //扫描有结果了
                        if msg.ends_with("CTRL-EVENT-SCAN-RESULTS") {
                            log::debug!("[wpa] revc event:{}", msg);
                            self.send(Event::ScanComplete).await?;
                            continue;
                        }

                        //成功连接 AP
                        if msg.contains("CTRL-EVENT-CONNECTED") {
                            log::debug!("[wpa] revc event:{}", msg);
                            self.send(Event::Connected).await?;
                            continue;
                        }

                        //断开连接
                        if msg.contains("CTRL-EVENT-DISCONNECTED") {
                            log::debug!("[wpa] revc event:{}", msg);
                            self.send(Event::Disconnected).await?;
                            continue;
                        }

                        //关联请求被拒绝
                        //认证失败
                        //  (msg.contains("CTRL-EVENT-SSID-TEMP-DISABLED")&& msg.contains("reason=WRONG_KEY"))
                        if msg.contains("CTRL-EVENT-ASSOC-REJECT")
                            || msg.contains("CTRL-EVENT-SSID-TEMP-DISABLED")
                            || msg.contains("CTRL-EVENT-AUTH-REJECT")
                            || msg.contains("CTRL-EVENT-EAP-FAILURE")
                        {
                            log::debug!("[wpa] revc event:{}", msg);
                            self.send(Event::WrongPsk).await?;
                            continue;
                        }

                        // if msg.contains("CTRL-EVENT-NETWORK-NOT-FOUND") {
                        //     self.send(Event::NetworkNotFound).await?;
                        //     continue;
                        // }

                        // self.send(Event::Unknown(msg.into())).await?;
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        continue;
                    }
                    Err(e) => {
                        log::warn!("[wpa] event revc err:{}", e);
                        return Err(anyhow!("[wpa] event revc err:({})", e));
                    }
                }
            }
        }
        #[cfg(target_os = "windows")]
        return Err(anyhow!("Implemented"));
    }
}
