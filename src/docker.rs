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

pub struct ContainerOptions {
    pub ports: Option<String>,
    pub cpus: Option<u32>,
    pub memory: Option<u32>,
    pub link: Option<String>,
    pub mount: Option<String>,
    pub envs: Option<Vec<(String, String)>>,
    pub args: Option<Vec<String>>,
}

pub async fn exec_run_container(
    container_id: &str,
    tag: &str,
    options: ContainerOptions,
) -> Result<()> {
    let mut cmd = Command::new("docker");
    cmd.arg("run")
        .arg("--rm")
        .arg("--name")
        .arg(container_id)
        .arg("-d");
    if let Some(cpus) = options.cpus {
        cmd.arg("--cpus").arg(cpus.to_string());
    }
    if let Some(memory) = options.memory {
        cmd.arg("--memory").arg(format!("{}m", memory));
    }
    if let Some(ports_str) = options.ports {
        cmd.arg("-p").arg(ports_str);
    }
    if let Some(link_str) = options.link {
        cmd.arg("--link").arg(link_str);
    }
    if let Some(mount_str) = options.mount {
        cmd.arg("-v").arg(mount_str);
    }
    if let Some(env_vars) = options.envs {
        for (key, value) in env_vars {
            cmd.arg("-e").arg(format!("{}={}", key, value));
        }
    }
    cmd.arg(tag);
    if let Some(args) = options.args {
        for arg in args {
            cmd.arg(arg);
        }
    }
    exec(&mut cmd).await?;
    Ok(())
}

pub async fn exec_stop_container(container_id: &str) -> Result<()> {
    let mut cmd = Command::new("docker");
    cmd.arg("stop").arg(container_id);
    exec(&mut cmd).await?;
    Ok(())
}
