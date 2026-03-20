pub mod client;
pub mod conn;
pub mod event;
pub mod wifi;
use crate::wifi::dto::Event;
pub use client::*;
use conn::*;
use event::*;
use std::path::PathBuf;
use std::str::FromStr;
use tokio::sync::{mpsc, oneshot};
use tokio::time::Duration;
pub use wifi::*;

pub type AppResult<T = ()> = std::result::Result<T, anyhow::Error>;
#[derive(Clone)]
pub struct Config {
    //请求命令队列长度
    pub cmd_channel_len: usize,
    //wifi socket 路径
    pub server_socket_path: String,
    //连接超时
    pub conn_timeout: Duration,
    //事件回调
    pub event_callback: Option<fn(String)>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cmd_channel_len: 32,
            server_socket_path: "/var/run/wpa_supplicant/wlan0".to_string(),
            conn_timeout: Duration::from_secs(5),
            event_callback: None,
        }
    }
}
#[allow(dead_code)]
impl Config {
    pub fn set_socket_path(&mut self, path: String) {
        self.server_socket_path = path;
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.conn_timeout = timeout;
    }

    pub fn set_iface_name(&mut self, name: String) {
        self.server_socket_path = format!("/var/run/wpa_supplicant/{}", name);
    }

    pub fn set_event_callback(&mut self, callback: fn(String)) {
        self.event_callback = Some(callback);
    }
}

pub async fn connect(cfg: Config) -> AppResult<Client> {
    let (cmd_sender, cmd_recv) = mpsc::channel(cfg.cmd_channel_len);

    let cmd = Client::new(cmd_sender.clone());

    let wpa = Wifi {
        event_callback: cfg.event_callback, 
        cmd_recv,
        cmd_sender,
    };

    let socket_path: PathBuf = cfg.server_socket_path.clone().into();
    //client cli 发收
    let cli = Conn::<10240>::connect(&socket_path, "wpa_cli.sock").await?;
    //ATTACH 事件
    let (event, evt_recv) = EventConn::new(&socket_path).await?;

    if let Some(cb) = &wpa.event_callback {
        cb(Event::Ready.to_string());
    }

    tokio::spawn(async move {
        let res = tokio::select!(
            res = event.listening() => res,
            resp = wpa.listening(evt_recv, cli) => resp,
        );
        if res.is_err() {
            log::error!("[wap] listening exit by err:{:?}", res);
        } else {
            log::debug!("[wap] listening exit");
        }
    });

    Ok(cmd)
}
