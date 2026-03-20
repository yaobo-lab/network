#![allow(unused_assignments)]
use super::*;
use crate::wifi::dto::*;
use anyhow::{Ok, anyhow};
use std::sync::Arc;

//事件与消息枚举
#[derive(Debug)]
pub(crate) enum EventOrCmd {
    Event(Option<Event>),
    CmdMsg(Option<CmdMsg>),
}

pub struct Wifi {
    pub(crate) cmd_recv: mpsc::Receiver<CmdMsg>,
    #[allow(dead_code)]
    pub(crate) cmd_sender: mpsc::Sender<CmdMsg>,
    pub event_callback: Option<fn(String)>,
   
}

impl Wifi {
    pub(crate) async fn listening(
        mut self,
        mut evt: mpsc::Receiver<Event>,
        mut cli: Conn<10240>,
    ) -> AppResult {
        let mut scan_cmds = Vec::new();
        let mut select_request = None;
        log::debug!("[wpa] cli listening...");

        loop {
            let msg = tokio::select!(
                evt_msg = evt.recv() => {
                    EventOrCmd::Event(evt_msg)
                },
                cmd = self.cmd_recv.recv() => {
                    EventOrCmd::CmdMsg(cmd)
                },
            );
            match msg {
                EventOrCmd::Event(event) => match event {
                    Some(evt) => {
                        if let Err(e) = Self::handle_event(
                            &mut cli,
                            evt,
                            &mut scan_cmds,
                            &self.event_callback,
                            &mut select_request,
                        )
                        .await
                        {
                            log::error!("[wpa] handle_event err: {}", e);
                        }
                    }
                    None => return Err(anyhow!("EventChannelClosed")),
                },
                EventOrCmd::CmdMsg(msg) => match msg {
                    Some(CmdMsg::Shutdown) => {
                        return Ok(());
                    }
                    Some(msg) => {
                        if let Err(e) = self
                            .handle_msg(&mut cli, msg, &mut scan_cmds, &mut select_request)
                            .await
                        {
                            log::error!("[wpa] handle_msg err: {}", e);
                        }
                    }
                    None => return Err(anyhow!("CmdChannelClosed")),
                },
            }
        }
    }

    async fn handle_event<const N: usize>(
        cli: &mut Conn<N>,
        event: Event,
        scan_cmds: &mut Vec<oneshot::Sender<AppResult<Arc<Vec<ScanResult>>>>>,
        callback: &Option<fn(String)>,
        select_request: &mut Option<oneshot::Sender<AppResult<SelectResult>>>,
    ) -> AppResult {
        #[cfg(target_os = "linux")]
        {
            match event {
                Event::ScanComplete => {
                    if scan_cmds.len() == 0 {
                        return Ok(());
                    }

                    let resp = cli.send_cmd_result(b"SCAN_RESULTS").await;
                    if resp.is_err() {
                        let err_msg = resp.err().unwrap().to_string();

                        while let Some(cmd) = scan_cmds.pop() {
                            cmd.send(Err(anyhow!("{}", err_msg)))
                                .map_err(|_| anyhow!("[wpa] resp channel send err"))?;
                        }
                        return Err(anyhow!("[wpa] cmd SCAN_RESULTS err"));
                    }

                    let data_str = resp.unwrap();

                    log::trace!("[wpa] scan_result: {}", data_str);

                    let resp = ScanResult::from_str(&data_str);
                    if resp.is_err() {
                        let err_msg = resp.err().unwrap().to_string();

                        while let Some(cmd) = scan_cmds.pop() {
                            cmd.send(Err(anyhow!("{}", err_msg)))
                                .map_err(|_| anyhow!("[wpa] resp channel send err"))?;
                        }
                        return Err(anyhow!("[wpa] cmd SCAN_RESULTS from_str err"));
                    }

                    let mut scan_wifis = resp.unwrap();

                    scan_wifis.sort_by(|a, b| b.signal.cmp(&a.signal));

                    log::debug!(
                        "handle_event EVENT: SCAN_RESULTS Len:{},scan_cmd_len:{}",
                        scan_wifis.len(),
                        scan_cmds.len()
                    );

                    let results = Arc::new(scan_wifis);

                    while let Some(cmd) = scan_cmds.pop() {
                        cmd.send(Ok(results.clone()))
                            .map_err(|_| anyhow!("[wpa] resp channel send err"))?;
                    }
                }
                Event::WrongPsk => {
                    if let Some(sender) = select_request.take() {
                        sender
                            .send(Ok(SelectResult::WrongPsk))
                            .map_err(|_| anyhow!("[wpa] resp channel send err"))?;
                    }
                }
                Event::Connected => {
                    if let Some(sender) = select_request.take() {
                        sender
                            .send(Ok(SelectResult::Success))
                            .map_err(|_| anyhow!("[wpa] resp channel send err"))?;
                    }
                }
                _ => {
                    if let Some(cb) = callback {
                        cb(event.to_string());
                    }
                }
            }
        }
        Ok(())
    }

    //请求channel 处理
    async fn handle_msg<const N: usize>(
        &self,
        cli: &mut Conn<N>,
        cmd: CmdMsg,
        scan_cmds: &mut Vec<oneshot::Sender<AppResult<Arc<Vec<ScanResult>>>>>,
        select_request: &mut Option<oneshot::Sender<AppResult<SelectResult>>>,
    ) -> AppResult {
        #[cfg(target_os = "linux")]
        {
            match cmd {
                CmdMsg::Custom(custom, resp_channel) => {
                    log::debug!("[wpa] custom cmd: \"{custom}\"");
                    let resp = cli.send_cmd_result(custom.as_bytes()).await;
                    let is_err = resp.is_err();
                    resp_channel
                        .send(resp)
                        .map_err(|_| anyhow!("[wpa] resp channel send err"))?;

                    if is_err {
                        return Err(anyhow!("[wpa] custom cmd err"));
                    }
                }

                CmdMsg::Scan(resp_channel) => {
                    log::debug!("[wpa] cmd SCAN");
                    let resp = cli.send_cmd_ok(b"SCAN").await;

                    if let Err(e) = resp {
                        resp_channel
                            .send(Err(anyhow!("{}", e)))
                            .map_err(|_| anyhow!("[wpa] resp channel send err"))?;

                        return Err(anyhow!("[wpa] cmd SCAN err: {}", e));
                    } else {
                        //等待结果返回
                        scan_cmds.push(resp_channel);
                    }
                }
                CmdMsg::ScanResult(resp_channel) => {
                    log::debug!("[wpa] cmd SCAN_RESULTS");
                    let resp = cli.send_cmd_result(b"SCAN_RESULTS").await;
                    if resp.is_err() {
                        let err_msg = resp.err().unwrap().to_string();

                        while let Some(cmd) = scan_cmds.pop() {
                            cmd.send(Err(anyhow!("{}", err_msg)))
                                .map_err(|_| anyhow!("[wpa] resp channel send err"))?;
                        }
                        return Err(anyhow!("[wpa] cmd SCAN_RESULTS err"));
                    }

                    let data_str = resp.unwrap();

                    let resp = ScanResult::from_str(&data_str);
                    if resp.is_err() {
                        let err_msg = resp.err().unwrap().to_string();

                        while let Some(cmd) = scan_cmds.pop() {
                            cmd.send(Err(anyhow!("{}", err_msg)))
                                .map_err(|_| anyhow!("[wpa] resp channel send err"))?;
                        }
                        return Err(anyhow!("[wpa] cmd SCAN_RESULTS from_str err"));
                    }

                    let mut scan_wifis = resp.unwrap();

                    scan_wifis.sort_by(|a, b| b.signal.cmp(&a.signal));

                    log::debug!("handle_msg EVENT: SCAN_RESULTS Len:{}", scan_wifis.len());

                    resp_channel
                        .send(Ok(scan_wifis))
                        .map_err(|_| anyhow!("[wpa] resp channel send err"))?;
                }

                CmdMsg::Disconnect(resp_channel) => {
                    log::debug!("[wpa] cmd DISCONNECT");
                    let resp = cli.send_cmd_ok(b"DISCONNECT").await;

                    let is_err = resp.is_err();
                    resp_channel
                        .send(resp)
                        .map_err(|_| anyhow!("[wpa] resp channel send err"))?;

                    if is_err {
                        return Err(anyhow!("[wpa] cmd DISCONNECT err"));
                    }
                }
                CmdMsg::Reconnnect(resp_channel) => {
                    log::debug!("[wpa] cmd RECONNECT");
                    let resp = cli.send_cmd_ok(b"RECONNECT").await;
                    let is_err = resp.is_err();
                    resp_channel
                        .send(resp)
                        .map_err(|_| anyhow!("[wpa] resp channel send err"))?;

                    if is_err {
                        return Err(anyhow!("[wpa] cmd RECONNECT err"));
                    }
                }
                CmdMsg::Networks(resp_channel) => {
                    log::debug!("[wpa] cmd LIST_NETWORKS");
                    let resp = cli.send_cmd_result(b"LIST_NETWORKS").await;

                    let is_err = resp.is_err();
                    if is_err {
                        resp_channel
                            .send(Err(anyhow!("{}", resp.err().unwrap())))
                            .map_err(|_| anyhow!("[wpa] resp channel send err"))?;
                        return Err(anyhow!("[wpa] cmd LIST_NETWORKS err"));
                    }

                    let data_str = resp.unwrap();
                    let resp = NetworkListResult::from_str(&data_str).await;
                    let is_err = resp.is_err();
                    if is_err {
                        resp_channel
                            .send(Err(anyhow!("{}", resp.err().unwrap())))
                            .map_err(|_| anyhow!("[wpa] resp channel send err"))?;
                        return Err(anyhow!("[wpa] cmd LIST_NETWORKS from_str err"));
                    }
                    let resp = resp.unwrap();
                    resp_channel
                        .send(Ok(resp))
                        .map_err(|_| anyhow!("[wpa] resp channel send err"))?;
                }
                CmdMsg::Status(resp_channel) => {
                    log::debug!("[wpa] cmd get_status");

                    let resp = cli.send_cmd_result(b"STATUS").await;
                    if resp.is_err() {
                        resp_channel
                            .send(Err(anyhow!("{}", resp.err().unwrap())))
                            .map_err(|_| anyhow!("[wpa] resp channel send err"))?;
                        return Err(anyhow!("[wpa] cmd STATUS err"));
                    }
                    let data_str = resp.unwrap();
                    let resp = parse_status(&data_str);
                    if resp.is_err() {
                        resp_channel
                            .send(Err(anyhow!("{}", resp.err().unwrap())))
                            .map_err(|_| anyhow!("[wpa] resp channel send err"))?;
                        return Err(anyhow!("[wpa] cmd STATUS parse_status err"));
                    }

                    let status = resp.unwrap();
                    resp_channel
                        .send(Ok(status))
                        .map_err(|_| anyhow!("[wpa] resp channel send err"))?;
                }
                CmdMsg::AddNetwork(resp_channel) => {
                    log::debug!("[wpa] cmd ADD_NETWORK");
                    let resp = cli.send_cmd_result(b"ADD_NETWORK").await;
                    if resp.is_err() {
                        resp_channel
                            .send(Err(anyhow!("{}", resp.err().unwrap())))
                            .map_err(|_| anyhow!("[wpa] resp channel send err"))?;
                        return Err(anyhow!("[wpa] cmd ADD_NETWORK err"));
                    }
                    let data_str = resp.unwrap();
                    let resp = usize::from_str(&data_str);
                    if resp.is_err() {
                        resp_channel
                            .send(Err(anyhow!("{}", resp.err().unwrap())))
                            .map_err(|_| anyhow!("[wpa] resp channel send err"))?;
                        return Err(anyhow!("[wpa] cmd ADD_NETWORK from_str err"));
                    }
                    let network_id = resp.unwrap();
                    resp_channel
                        .send(Ok(network_id))
                        .map_err(|_| anyhow!("[wpa] resp channel send err"))?;
                }
                CmdMsg::SetNetwork(id, param, resp_channel) => {
                    let cmd = format!(
                        "SET_NETWORK {id} {}",
                        match param {
                            SetNetwork::Ssid(ssid) => format!("ssid \"{ssid}\""),
                            SetNetwork::Bssid(bssid) => format!("bssid \"{bssid}\""),
                            SetNetwork::Psk(psk) => format!("psk \"{psk}\""),
                            SetNetwork::KeyMgmt(mgmt) => format!("key_mgmt {}", mgmt),
                        }
                    );
                    log::debug!("[wpa] cmd SET_NETWORK: \"{cmd}\"");

                    let bytes = cmd.into_bytes();
                    let resp = cli.send_cmd_ok(&bytes).await;
                    let is_err = resp.is_err();
                    resp_channel
                        .send(resp)
                        .map_err(|_| anyhow!("[wpa] resp channel send err"))?;

                    if is_err {
                        return Err(anyhow!("[wpa] cmd SET_NETWORK err"));
                    }
                }
                CmdMsg::SaveConfig(resp_channel) => {
                    log::debug!("[wpa] cmd SAVE_CONFIG");
                    let resp = cli.send_cmd_ok(b"SAVE_CONFIG").await;
                    let is_err = resp.is_err();
                    resp_channel
                        .send(resp)
                        .map_err(|_| anyhow!("[wpa] resp channel send err"))?;

                    if is_err {
                        return Err(anyhow!("[wpa] cmd SAVE_CONFIG err"));
                    }
                }
                CmdMsg::RemoveNetwork(remove_network, resp_channel) => {
                    log::debug!("[wpa] cmd REMOVE_NETWORK");
                    let str = match remove_network {
                        RemoveNetwork::All => "all".to_string(),
                        RemoveNetwork::Id(id) => id.to_string(),
                    };
                    let cmd = format!("REMOVE_NETWORK {str}");
                    log::debug!("[wpa] cmd {cmd}");
                    let bytes = cmd.into_bytes();

                    let resp = cli.send_cmd_ok(&bytes).await;
                    let is_err = resp.is_err();
                    resp_channel
                        .send(resp)
                        .map_err(|_| anyhow!("[wpa] resp channel send err"))?;

                    if is_err {
                        return Err(anyhow!("[wpa] cmd REMOVE_NETWORK err"));
                    }
                }
                CmdMsg::EnableNetwork(networkd_id, resp_channel) => {
                    let cmd = format!("ENABLE_NETWORK {}", networkd_id);
                    log::debug!("[wpa] cmd {cmd}");
                    let bytes = cmd.into_bytes();
                    let resp = cli.send_cmd_ok(&bytes).await;
                    let is_err = resp.is_err();
                    resp_channel
                        .send(resp)
                        .map_err(|_| anyhow!("[wpa] resp channel send err"))?;

                    if is_err {
                        return Err(anyhow!("[wpa] cmd ENABLE_NETWORK err"));
                    }
                }

                CmdMsg::DisableNetwork(networkd_id, resp_channel) => {
                    let cmd = format!("DISABLE_NETWORK {}", networkd_id);
                    log::debug!("[wpa] cmd {cmd}");
                    let bytes = cmd.into_bytes();
                    let resp = cli.send_cmd_ok(&bytes).await;
                    let is_err = resp.is_err();
                    resp_channel
                        .send(resp)
                        .map_err(|_| anyhow!("[wpa] resp channel send err"))?;

                    if is_err {
                        return Err(anyhow!("[wpa] cmd DISABLE_NETWORK err"));
                    }
                }

                CmdMsg::SelectNetwork(id, resp_channel) => {
                    let cmd = format!("SELECT_NETWORK {id}");
                    log::debug!("[wpa] cmd {cmd}");
                    let bytes = cmd.into_bytes();
                    let resp = cli.send_cmd_ok(&bytes).await;
                    if resp.is_err() {
                        resp_channel
                            .send(Err(anyhow!("不合法的id")))
                            .map_err(|_| anyhow!("[wpa] resp channel send err"))?;
                        return Err(anyhow!("[wpa] cmd SELECT_NETWORK err"));
                    }

                    log::debug!("[wpa] cmd SELECT_NETWORK {id}");
                    let status = Self::get_status(cli).await?;
                    if let Some(current_id) = status.get("id") {
                        if current_id == &id.to_string() {
                            let _ = resp_channel.send(Ok(SelectResult::Success));
                            return Ok(());
                        } else {
                            *select_request = Some(resp_channel);
                        }
                    } else {
                        *select_request = Some(resp_channel);
                    }
                }
                CmdMsg::Shutdown => (),
            }
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    async fn get_status<const N: usize>(socket_handle: &mut Conn<N>) -> AppResult<Status> {
        let _n = socket_handle.socket.send(b"STATUS").await?;
        let n = socket_handle.socket.recv(&mut socket_handle.buffer).await?;
        let data_str = std::str::from_utf8(&socket_handle.buffer[..n])?.trim_end();
        parse_status(data_str)
    }
}
