pub mod local;
pub mod ssh;

pub trait Executor {
    async fn execute<F1, F2>(&self, script: &str, on_stdout: F1, on_stderr: F2) -> Result<String, anyhow::Error>
    where
        F1: Fn(&str) + Send + 'static,
        F2: Fn(&str) + Send + 'static;

    async fn mkdir(&self, path: &str) -> Result<(), anyhow::Error>;
    async fn rm(&self, path: &str) -> Result<(), anyhow::Error>;
    async fn cp<F>(&self, src: &str, dst: &str, on_progress: F) -> Result<(), anyhow::Error>
    where
        F: Fn(&str, u64, u64) + Send + 'static;
}