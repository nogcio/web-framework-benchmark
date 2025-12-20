use std::path::Path;
use tokio::process::Command;

use crate::{parsers::parse_mem, prelude::*};

#[derive(Debug)]
pub struct DockerStatsResult {
    pub memory_usage: u64,
}

pub async fn exec_build(path: &Path, tag: &str) -> Result<()> {
    let mut cmd = Command::new("docker");
    cmd.current_dir(path)
        .arg("build")
        .arg("-t")
        .arg(tag)
        .arg(".");
    exec(&mut cmd).await?;
    Ok(())
}

pub async fn exec_stats(container_id: &str) -> Result<DockerStatsResult> {
    let mut cmd = Command::new("docker");
    cmd.arg("stats")
        .arg("--no-stream")
        .arg("--format")
        .arg("{{.MemUsage}}")
        .arg(container_id);

    let stdout = exec(&mut cmd).await?;
    let mem_usage_part = stdout.split('/').next().unwrap_or("").trim();
    let memory_usage =
        parse_mem(mem_usage_part).ok_or_else(|| Error::DockerStatsParseError(stdout))?;

    Ok(DockerStatsResult { memory_usage })
}

pub async fn exec_run_container(
    container_id: &str,
    tag: &str,
    ports: Option<impl Into<String>>,
    cpus: u32,
    memory: u32,
    link: Option<impl Into<String>>,
    mount: Option<impl Into<String>>,
) -> Result<()> {
    let mut cmd = Command::new("docker");
    cmd.arg("run")
        .arg("--rm")
        .arg("--cpus")
        .arg(cpus.to_string())
        .arg("--memory")
        .arg(format!("{}m", memory))
        .arg("--name")
        .arg(container_id)
        .arg("-d");
    if let Some(ports_str) = ports {
        cmd.arg("-p").arg(ports_str.into());
    }
    if let Some(link_str) = link {
        cmd.arg("--link").arg(link_str.into());
    }
    if let Some(mount_str) = mount {
        cmd.arg("-v").arg(mount_str.into());
    }
    cmd.arg(tag);
    exec(&mut cmd).await?;
    Ok(())
}

pub async fn exec_stop_container(container_id: &str) -> Result<()> {
    let mut cmd = Command::new("docker");
    cmd.arg("stop").arg(container_id);
    exec(&mut cmd).await?;
    Ok(())
}
