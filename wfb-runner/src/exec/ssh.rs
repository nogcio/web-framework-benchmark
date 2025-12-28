use super::Executor;
use anyhow::{Context, Result};
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use ssh2::Session;

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
        // Create remote directory
        // Use sftp for mkdir
        let sftp = sess.sftp().context("Failed to init SFTP")?;
        // Ignore error if directory exists (or check first, but mkdir -p behavior is hard with sftp)
        // SFTP mkdir fails if exists.
        // We can try to stat it first.
        match sftp.stat(dst) {
            Ok(_) => {} // Exists
            Err(_) => {
                // Try to create. Note: sftp.mkdir doesn't do -p.
                // For simplicity, we assume parent exists or we don't care about -p here strictly,
                // but user might expect it.
                // To do -p properly via SFTP is tedious.
                // Alternatively, we can execute "mkdir -p" via channel exec before uploading?
                // But we are inside a loop.
                // Let's just try mkdir and ignore failure?
                // Or better: use sftp.mkdir with 0o755.
                let _ = sftp.mkdir(dst, 0o755);
            }
        }
        
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let entry_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            upload_recursive_sync(sess, &entry_path, &dst_path, total_size, copied, on_progress)?;
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
        remote_file.wait_close().context("Failed to wait for close")?;
    }
    Ok(())
}

impl Executor for SshExecutor {
    async fn execute<F1, F2>(&self, script: &str, on_stdout: F1, on_stderr: F2) -> Result<String>
    where
        F1: Fn(&str) + Send + 'static,
        F2: Fn(&str) + Send + 'static,
    {
        let host = self.host.clone();
        let port = self.port;
        let username = self.username.clone();
        let private_key_path = self.private_key_path.clone();
        let script = script.to_string();

        tokio::task::spawn_blocking(move || {
            let tcp = TcpStream::connect((host.as_str(), port)).context("Failed to connect to SSH host")?;
            let mut sess = Session::new().context("Failed to create SSH session")?;
            sess.set_tcp_stream(tcp);
            sess.handshake().context("SSH handshake failed")?;

            sess.userauth_pubkey_file(&username, None, &private_key_path, None)
                .context("SSH authentication failed")?;

            let mut channel = sess.channel_session().context("Failed to create SSH channel")?;
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
                                on_stderr(&line);
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
                on_stdout(&s);
                output.push_str(&s);
            }
            if let Some(s) = stderr_buffer.flush() {
                on_stderr(&s);
            }

            channel.wait_close()?;
            let exit_status = channel.exit_status()?;
            if exit_status != 0 {
                return Err(anyhow::anyhow!("Command failed with status: {}", exit_status));
            }

            Ok(output)
        })
        .await?
    }

    async fn mkdir(&self, path: &str) -> Result<(), anyhow::Error> {
        let noop = |_: &str| {};
        self.execute(&format!("mkdir -p {}", path), noop, noop)
            .await
            .map(|_| ())
    }

    async fn rm(&self, path: &str) -> Result<(), anyhow::Error> {
        let noop = |_: &str| {};
        self.execute(&format!("rm -rf {}", path), noop, noop)
            .await
            .map(|_| ())
    }

    async fn cp<F>(&self, src: &str, dst: &str, on_progress: F) -> Result<(), anyhow::Error>
    where
        F: Fn(&str, u64, u64) + Send + 'static,
    {
        let host = self.host.clone();
        let port = self.port;
        let username = self.username.clone();
        let private_key_path = self.private_key_path.clone();
        let src = src.to_string();
        let dst = dst.to_string();

        tokio::task::spawn_blocking(move || {
            let tcp = TcpStream::connect((host.as_str(), port)).context("Failed to connect to SSH host")?;
            let mut sess = Session::new().context("Failed to create SSH session")?;
            sess.set_tcp_stream(tcp);
            sess.handshake().context("SSH handshake failed")?;
            sess.userauth_pubkey_file(&username, None, &private_key_path, None)
                .context("SSH authentication failed")?;

            let src_path = Path::new(&src);
            let dst_path = Path::new(&dst);
            
            let total_size = get_dir_size_sync(src_path).context("Failed to get size")?;
            let copied = AtomicU64::new(0);

            // Ensure parent directory exists on remote?
            // We can't easily do `mkdir -p` via SFTP or SCP without executing a command.
            // But we can assume the user has prepared the environment or we can try to create it.
            // For now, let's just start uploading.
            
            upload_recursive_sync(&sess, src_path, dst_path, total_size, &copied, &on_progress)
        })
        .await?
    }
}


