use super::{Executor, OutputLogger};
use anyhow::{Context, Result};
use async_trait::async_trait;
use indicatif::ProgressBar;
use ssh2::Session;
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone)]
pub struct SshExecutor {
    host: String,
    port: u16,
    username: String,
    private_key_path: PathBuf,
}

impl SshExecutor {
    pub fn new(host: String, port: u16, username: String, private_key_path: PathBuf) -> Self {
        Self {
            host,
            port,
            username,
            private_key_path,
        }
    }

    pub fn from_config(config: &wfb_storage::SshConnection) -> Self {
        SshExecutor::new(
            config.ip.clone(),
            22,
            config.user.clone(),
            config.ssh_key_path.clone(),
        )
    }
}

struct LineBuffer {
    buf: Vec<u8>,
}

impl LineBuffer {
    fn new() -> Self {
        Self { buf: Vec::new() }
    }

    fn push(&mut self, data: &[u8]) -> Vec<String> {
        self.buf.extend_from_slice(data);
        let mut lines = Vec::new();
        while let Some(pos) = self.buf.iter().position(|&b| b == b'\n') {
            let line_bytes: Vec<u8> = self.buf.drain(..=pos).collect();
            let s = String::from_utf8_lossy(&line_bytes[..line_bytes.len() - 1]).to_string();
            lines.push(s);
        }
        lines
    }

    fn flush(&mut self) -> Option<String> {
        if self.buf.is_empty() {
            None
        } else {
            let s = String::from_utf8_lossy(&self.buf).to_string();
            self.buf.clear();
            Some(s)
        }
    }
}

fn get_dir_size_sync(path: &Path) -> Result<u64> {
    let meta = std::fs::metadata(path)?;
    if meta.is_file() {
        return Ok(meta.len());
    }
    let mut size = 0;
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if entry.metadata()?.is_dir() {
            size += get_dir_size_sync(&path)?;
        } else {
            size += entry.metadata()?.len();
        }
    }
    Ok(size)
}

fn upload_recursive_sync<F>(
    sess: &Session,
    src: &Path,
    dst: &Path,
    total_size: u64,
    copied: &AtomicU64,
    on_progress: &F,
) -> Result<()>
where
    F: Fn(&str, u64, u64) + Send + 'static,
{
    if src.is_dir() {
        let sftp = sess.sftp().context("Failed to init SFTP")?;
        match sftp.stat(dst) {
            Ok(_) => {} // Exists
            Err(_) => {
                let _ = sftp.mkdir(dst, 0o755);
            }
        }

        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let entry_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            upload_recursive_sync(
                sess,
                &entry_path,
                &dst_path,
                total_size,
                copied,
                on_progress,
            )?;
        }
    } else {
        let mut src_file = File::open(src).context("Failed to open local source file")?;
        let metadata = src_file.metadata().context("Failed to get metadata")?;
        let file_size = metadata.len();

        let mut remote_file = sess
            .scp_send(dst, 0o644, file_size, None)
            .context("Failed to start SCP send")?;

        let mut buffer = [0u8; 8192];

        loop {
            let n = src_file
                .read(&mut buffer)
                .context("Failed to read from source")?;
            if n == 0 {
                break;
            }
            remote_file
                .write_all(&buffer[..n])
                .context("Failed to write to remote SCP")?;
            let c = copied.fetch_add(n as u64, Ordering::Relaxed) + n as u64;
            on_progress(&src.to_string_lossy(), c, total_size);
        }

        // Close scp channel for this file
        remote_file.send_eof().context("Failed to send EOF")?;
        remote_file.wait_eof().context("Failed to wait for EOF")?;
        remote_file.close().context("Failed to close channel")?;
        remote_file
            .wait_close()
            .context("Failed to wait for close")?;
    }
    Ok(())
}

#[async_trait]
impl Executor for SshExecutor {
    async fn execute<S>(&self, script: S, pb: &ProgressBar) -> Result<String, anyhow::Error>
    where
        S: std::fmt::Display + Send + Sync,
    {
        self.execute_with_std_out(script, |_| {}, pb).await
    }

    async fn execute_with_std_out<S, F>(
        &self,
        script: S,
        on_stdout: F,
        pb: &ProgressBar,
    ) -> Result<String, anyhow::Error>
    where
        F: Fn(&str) + Send + Sync + 'static,
        S: std::fmt::Display + Send + Sync,
    {
        let script = script.to_string();
        let logger = Arc::new(OutputLogger::new(
            pb.clone(),
            format!("ssh {}@{} {}", self.username, self.host, script),
        ));

        let host = self.host.clone();
        let port = self.port;
        let username = self.username.clone();
        let private_key_path = self.private_key_path.clone();

        tokio::task::spawn_blocking(move || {
            let tcp = TcpStream::connect((host.as_str(), port))
                .context("Failed to connect to SSH host")?;
            let mut sess = Session::new().context("Failed to create SSH session")?;
            sess.set_tcp_stream(tcp);
            sess.handshake().context("SSH handshake failed")?;

            sess.userauth_pubkey_file(&username, None, &private_key_path, None)
                .context("SSH authentication failed")?;

            let mut channel = sess
                .channel_session()
                .context("Failed to create SSH channel")?;
            channel.exec(&script).context("Failed to execute script")?;

            sess.set_blocking(false);

            let mut stdout_buf = [0u8; 4096];
            let mut stderr_buf = [0u8; 4096];
            let mut stdout_buffer = LineBuffer::new();
            let mut stderr_buffer = LineBuffer::new();
            let mut output = String::new();

            let mut stdout_closed = false;
            let mut stderr_closed = false;

            while !stdout_closed || !stderr_closed {
                let mut did_work = false;

                if !stdout_closed {
                    match channel.read(&mut stdout_buf) {
                        Ok(0) => stdout_closed = true,
                        Ok(n) => {
                            did_work = true;
                            let lines = stdout_buffer.push(&stdout_buf[..n]);
                            for line in lines {
                                logger.on_stdout(&line);
                                on_stdout(&line);
                                output.push_str(&line);
                                output.push('\n');
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                        Err(e) => return Err(e.into()),
                    }
                }

                if !stderr_closed {
                    match channel.stderr().read(&mut stderr_buf) {
                        Ok(0) => stderr_closed = true,
                        Ok(n) => {
                            did_work = true;
                            let lines = stderr_buffer.push(&stderr_buf[..n]);
                            for line in lines {
                                logger.on_stderr(&line);
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                        Err(e) => return Err(e.into()),
                    }
                }

                if !did_work && (!stdout_closed || !stderr_closed) {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }

            // Flush remaining buffers
            if let Some(s) = stdout_buffer.flush() {
                logger.on_stdout(&s);
                output.push_str(&s);
            }
            if let Some(s) = stderr_buffer.flush() {
                logger.on_stderr(&s);
            }

            channel.wait_close()?;
            let exit_status = channel.exit_status()?;
            if exit_status != 0 {
                let stderr = logger.get_stderr();
                let last_lines = logger.get_last_lines_plain();
                if !stderr.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Command failed with status: {}\nCommand: {}\nStderr:\n{}\nRecent output:\n{}",
                        exit_status,
                        script,
                        stderr,
                        last_lines
                    ));
                }
                return Err(anyhow::anyhow!(
                    "Command failed with status: {}\nCommand: {}\nRecent output:\n{}",
                    exit_status,
                    script,
                    last_lines
                ));
            }

            Ok(output)
        })
        .await?
    }

    async fn mkdir(&self, path: &str) -> Result<(), anyhow::Error> {
        let pb = ProgressBar::hidden();
        self.execute(format!("mkdir -p {}", path), &pb)
            .await
            .map(|_| ())
    }

    async fn rm(&self, path: &str) -> Result<(), anyhow::Error> {
        let pb = ProgressBar::hidden();
        self.execute(format!("rm -rf {}", path), &pb)
            .await
            .map(|_| ())
    }

    async fn cp(&self, src: &str, dst: &str, pb: &ProgressBar) -> Result<(), anyhow::Error> {
        let host = self.host.clone();
        let port = self.port;
        let username = self.username.clone();
        let private_key_path = self.private_key_path.clone();
        let src = src.to_string();
        let dst = dst.to_string();
        let pb_clone = pb.clone();
        pb_clone.set_length(100);
        pb_clone.set_position(0);

        let on_progress = move |filename: &str, current: u64, total: u64| {
            let percentage = if total > 0 {
                (current as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            pb_clone.set_message(format!("copying {}", filename));
            pb_clone.set_position(percentage.round() as u64);
        };

        tokio::task::spawn_blocking(move || {
            let tcp = TcpStream::connect((host.as_str(), port))
                .context("Failed to connect to SSH host")?;
            let mut sess = Session::new().context("Failed to create SSH session")?;
            sess.set_tcp_stream(tcp);
            sess.handshake().context("SSH handshake failed")?;
            sess.userauth_pubkey_file(&username, None, &private_key_path, None)
                .context("SSH authentication failed")?;

            let src_path = Path::new(&src);
            let dst_path = Path::new(&dst);

            let total_size = get_dir_size_sync(src_path).context("Failed to get size")?;
            let copied = AtomicU64::new(0);

            upload_recursive_sync(&sess, src_path, dst_path, total_size, &copied, &on_progress)
        })
        .await?
    }
}
