pub mod local;
pub mod ssh;

use async_trait::async_trait;
use console::style;
use indicatif::ProgressBar;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

#[async_trait]
pub trait Executor: Send + Sync {
    async fn execute<S>(&self, script: S, pb: &ProgressBar) -> Result<String, anyhow::Error>
    where
        S: std::fmt::Display + Send + Sync;
    async fn execute_with_std_out<S, F>(
        &self,
        script: S,
        on_stdout: F,
        pb: &ProgressBar,
    ) -> Result<String, anyhow::Error>
    where
        F: Fn(&str) + Send + Sync + 'static,
        S: std::fmt::Display + Send + Sync;
    async fn mkdir(&self, path: &str) -> Result<(), anyhow::Error>;
    async fn rm(&self, path: &str) -> Result<(), anyhow::Error>;
    async fn cp(&self, src: &str, dst: &str, pb: &ProgressBar) -> Result<(), anyhow::Error>;
}

#[derive(Clone)]
pub struct OutputLogger {
    pb: ProgressBar,
    cmd: String,
    last_lines: Arc<Mutex<VecDeque<String>>>,
    last_lines_plain: Arc<Mutex<VecDeque<String>>>,
    stderr_log: Arc<Mutex<String>>,
}

impl OutputLogger {
    pub fn new(pb: ProgressBar, cmd: String) -> Self {
        pb.set_message(cmd.clone());
        Self {
            pb,
            cmd,
            last_lines: Arc::new(Mutex::new(VecDeque::with_capacity(6))),
            last_lines_plain: Arc::new(Mutex::new(VecDeque::with_capacity(20))),
            stderr_log: Arc::new(Mutex::new(String::new())),
        }
    }

    pub fn on_stdout(&self, line: &str) {
        {
            let mut lines = self.last_lines.lock().unwrap_or_else(|e| e.into_inner());
            if lines.len() >= 6 {
                lines.pop_front();
            }
            lines.push_back(format!(
                "{}  {}",
                style("===>").black().bright(),
                style(line).black().bright()
            ));

            let mut plain = self
                .last_lines_plain
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if plain.len() >= 20 {
                plain.pop_front();
            }
            plain.push_back(format!("stdout: {}", line));
        }
        self.update_state();
    }

    pub fn on_stderr(&self, line: &str) {
        {
            let mut lines = self.last_lines.lock().unwrap_or_else(|e| e.into_inner());
            if lines.len() >= 6 {
                lines.pop_front();
            }
            lines.push_back(format!(
                "{}  {}",
                style("===>").red(),
                style(line).black().bright()
            ));

            let mut plain = self
                .last_lines_plain
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if plain.len() >= 20 {
                plain.pop_front();
            }
            plain.push_back(format!("stderr: {}", line));

            let mut log = self.stderr_log.lock().unwrap_or_else(|e| e.into_inner());
            log.push_str(line);
            log.push('\n');
        }

        self.update_state();
    }

    pub fn update_state(&self) {
        let gray_lines = self
            .last_lines
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        self.pb.set_message(format!("{}\n{}", self.cmd, gray_lines));
    }

    pub fn get_stderr(&self) -> String {
        self.stderr_log
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn get_last_lines_plain(&self) -> String {
        self.last_lines_plain
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    }
}
