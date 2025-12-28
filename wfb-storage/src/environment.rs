
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "executor", rename_all = "snake_case")]
pub enum Environment {
    Local(Box<LocalEnvironment>),
    Ssh(Box<SshEnvironment>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LocalEnvironment {
    pub name: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SshConnection {
    pub public_ip: String,
    pub port: u16,
    pub private_ip: String,
    pub username: String,
    pub private_key_path: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SshEnvironment {
    pub name: String,
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
}