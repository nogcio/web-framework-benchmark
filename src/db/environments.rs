use crate::benchmark_environment::config::EnvironmentFile;
use crate::prelude::*;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct EnvironmentRecord {
    pub id: String,
    pub config: EnvironmentFile,
}

pub fn load_environments<P: AsRef<Path>>(path: P) -> Result<Vec<EnvironmentRecord>> {
    let mut environments = Vec::new();
    let path = path.as_ref();
    if !path.exists() {
        return Ok(environments);
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            let content = fs::read_to_string(&path)?;
            let config: EnvironmentFile = serde_yaml::from_str(&content)?;
            environments.push(EnvironmentRecord { id, config });
        }
    }
    Ok(environments)
}
