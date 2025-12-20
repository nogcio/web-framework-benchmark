use tokio::process::Command;

use crate::prelude::*;

pub async fn exec(cmd: &mut Command) -> Result<String> {
    let cmd_str = format!("{:?}", cmd);
    debug!("Executing command: {}", cmd_str);

    let output = cmd.output().await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.trim().split("\n") {
        debug!("{}", line);
    }
    let status = output.status;
    debug!("Command completed with status: {}", status);
    if !status.success() {
        for line in String::from_utf8_lossy(&output.stderr).trim().split("\n") {
            error!("{}", line);
        }
        return Err(Error::ExecError {
            cmd: cmd_str,
            status,
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
