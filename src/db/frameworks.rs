use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

use crate::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkRecord {
    pub name: String,
    pub language: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct Framework {
    pub name: String,
    pub language: String,
    pub url: String,
}

impl From<&FrameworkRecord> for Framework {
    fn from(record: &FrameworkRecord) -> Self {
        Self {
            name: record.name.clone(),
            language: record.language.clone(),
            url: record.url.clone(),
        }
    }
}

pub fn parse_frameworks<P: AsRef<Path>>(path: P) -> Result<Vec<FrameworkRecord>> {
    let content = fs::read_to_string(path)?;
    let frameworks: Vec<FrameworkRecord> = serde_yaml::from_str(&content)?;
    Ok(frameworks)
}
