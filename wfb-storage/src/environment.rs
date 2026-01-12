use std::path::PathBuf;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "executor", rename_all = "snake_case")]
pub enum Environment {
    Local(Box<LocalEnvironment>),
    Ssh(Box<SshEnvironment>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LocalEnvironment {
    pub name: String,
    pub title: String,
    pub spec: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SshConnection {
    pub ip: String,
    pub internal_ip: String,
    pub user: String,
    pub ssh_key_path: PathBuf,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SshEnvironment {
    pub name: String,
    pub title: String,
    pub spec: Option<String>,
    pub icon: Option<String>,
    pub wrkr: SshConnection,
    pub db: SshConnection,
    pub app: SshConnection,
}

impl Environment {
    pub fn name(&self) -> &str {
        match self {
            Environment::Local(env) => &env.name,
            Environment::Ssh(env) => &env.name,
        }
    }

    pub fn title(&self) -> &str {
        match self {
            Environment::Local(env) => &env.title,
            Environment::Ssh(env) => &env.title,
        }
    }

    pub fn spec(&self) -> Option<&str> {
        match self {
            Environment::Local(env) => env.spec.as_deref(),
            Environment::Ssh(env) => env.spec.as_deref(),
        }
    }

    pub fn icon(&self) -> Option<&str> {
        match self {
            Environment::Local(env) => env.icon.as_deref(),
            Environment::Ssh(env) => env.icon.as_deref(),
        }
    }
}
