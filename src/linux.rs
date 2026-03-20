use anyhow::{Result, anyhow};
use std::process::Stdio;
use tokio::process::Command;
//停启用网络
pub async fn enable(name: &str, is_up: bool) -> Result<()> {
    let is_up_cmd = if is_up { "up" } else { "down" };
    let output = Command::new("ip")
        .arg("link")
        .arg("set")
        .arg(name)
        .arg(is_up_cmd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr_str = String::from_utf8_lossy(&output.stderr);
        Err(anyhow!(
            "can not set network interface: {} status: {} ,err: {}",
            name,
            is_up_cmd,
            stderr_str
        ))
    }
}
