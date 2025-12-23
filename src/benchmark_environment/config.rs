use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WrkConfig {
    pub duration_secs: u64,
    pub threads: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EnvironmentFile {
    pub name: String,
    pub spec: Option<String>,
    pub icon: Option<String>,
    #[serde(flatten)]
    pub kind: EnvironmentKind,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum EnvironmentKind {
    Local(LocalConfig),
    Remote(RemoteConfig),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LocalConfig {
    pub wrk: WrkConfig,
    pub limits: LimitsConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LimitsConfig {
    pub db: Option<ResourceLimitSpec>,
    pub app: Option<ResourceLimitSpec>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ResourceLimitSpec {
    pub cpus: Option<u32>,
    pub memory_mb: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RemoteConfig {
    pub wrk: WrkConfig,
    pub hosts: HashMap<String, RemoteHostConfig>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct RemoteHostConfig {
    pub ip: String,
    pub internal_ip: String,
    pub user: String,
    pub ssh_key_path: String,
}
