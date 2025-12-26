use std::io;
use std::process::ExitStatus;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Configuration error: key must be set {0}")]
    EnvVarError(&'static str),
    #[error("Command execution error: '{cmd}' exited with status {status}")]
    ExecError { cmd: String, status: ExitStatus },
    #[error("Docker stats parse error: {0}")]
    DockerStatsParseError(String),
    #[error("Server start timeout error")]
    ServerStartTimeoutError,
    #[error("HTTP request error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Parse error: {0}")]
    WrkParseError(String),
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
    #[error("YAML parsing error: {0}")]
    YamlError(#[from] serde_yaml::Error),
    #[error("Task join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Benchmark environment not prepared")]
    EnvironmentNotPrepared,
    #[error("Lock poisoned")]
    PoisonError,
    #[error("Invalid environment type: {0}")]
    InvalidEnvironment(String),
    #[error("Invalid test type: {0}")]
    InvalidTest(String),
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
    #[error("System error: {0}")]
    System(String),
}
