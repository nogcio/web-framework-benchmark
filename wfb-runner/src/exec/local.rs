use super::Executor;
use anyhow::{Context, Result};
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

#[derive(Default, Clone)]
pub struct LocalExecutor;

impl LocalExecutor {
    pub fn new() -> Self {
        Self
    }
}

async fn get_dir_size(path: &Path) -> Result<u64> {
    let meta = fs::metadata(path).await?;
    if meta.is_file() {
        return Ok(meta.len());
    }
    let mut size = 0;
    let mut entries = fs::read_dir(path).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if entry.metadata().await?.is_dir() {
            size += Box::pin(get_dir_size(&path)).await?;
        } else {
            size += entry.metadata().await?.len();
        }
    }
    Ok(size)
}

async fn copy_recursive<F>(
    src: &Path,
    dst: &Path,
    total_size: u64,
    copied: Arc<AtomicU64>,
    on_progress: &F,
) -> Result<()>
where
    F: Fn(&str, u64, u64) + Send + 'static,
{
    if src.is_dir() {
        fs::create_dir_all(dst).await?;
        let mut entries = fs::read_dir(src).await?;
        while let Some(entry) = entries.next_entry().await? {
            let entry_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            Box::pin(copy_recursive(
                &entry_path,
                &dst_path,
                total_size,
                copied.clone(),
                on_progress,
            ))
            .await?;
        }
    } else {
        let mut src_file = fs::File::open(src).await.context("Failed to open source file")?;
        let mut dst_file = fs::File::create(dst)
            .await
            .context("Failed to create destination file")?;

        let mut buffer = [0u8; 8192];

        loop {
            let n = src_file
                .read(&mut buffer)
                .await
                .context("Failed to read from source")?;
            if n == 0 {
                break;
            }
            dst_file
                .write_all(&buffer[..n])
                .await
                .context("Failed to write to destination")?;
            let c = copied.fetch_add(n as u64, Ordering::Relaxed) + n as u64;
            on_progress(&src.to_string_lossy(), c, total_size);
        }
    }
    Ok(())
}

impl Executor for LocalExecutor {
    async fn execute<F1, F2>(&self, script: &str, on_stdout: F1, on_stderr: F2) -> Result<String>
    where
        F1: Fn(&str) + Send + 'static,
        F2: Fn(&str) + Send + 'static,
    {
        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("powershell");
            c.arg("-Command").arg(script);
            c
        } else {
            let mut c = Command::new("sh");
            c.arg("-c").arg(script);
            c
        };

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().context("Failed to spawn command")?;

        let stdout = child.stdout.take().context("Failed to open stdout")?;
        let stderr = child.stderr.take().context("Failed to open stderr")?;

        let stdout_task = tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            let mut output = String::new();
            while let Ok(Some(line)) = reader.next_line().await {
                on_stdout(&line);
                output.push_str(&line);
                output.push('\n');
            }
            output
        });

        let stderr_task = tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                on_stderr(&line);
            }
        });

        let (stdout_res, stderr_res) = tokio::join!(stdout_task, stderr_task);
        let stdout_str = stdout_res.context("Stdout task failed")?;
        stderr_res.context("Stderr task failed")?;

        let status = child.wait().await.context("Failed to wait for child")?;

        if !status.success() {
            return Err(anyhow::anyhow!("Command failed with status: {}", status));
        }

        Ok(stdout_str)
    }

    async fn mkdir(&self, path: &str) -> Result<(), anyhow::Error> {
        fs::create_dir_all(path).await.context("Failed to create directory")
    }

    async fn rm(&self, path: &str) -> Result<(), anyhow::Error> {
        match fs::metadata(path).await {
            Ok(meta) => {
                if meta.is_dir() {
                    fs::remove_dir_all(path).await.context("Failed to remove directory")
                } else {
                    fs::remove_file(path).await.context("Failed to remove file")
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e).context("Failed to get metadata"),
        }
    }

    async fn cp<F>(&self, src: &str, dst: &str, on_progress: F) -> Result<(), anyhow::Error>
    where
        F: Fn(&str, u64, u64) + Send + 'static,
    {
        let src_path = Path::new(src);
        let dst_path = Path::new(dst);

        let total_size = get_dir_size(src_path).await.context("Failed to get size")?;
        let copied = Arc::new(AtomicU64::new(0));

        copy_recursive(src_path, dst_path, total_size, copied, &on_progress).await
    }
}

