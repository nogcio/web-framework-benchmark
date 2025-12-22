use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path};

use crate::{benchmark::BenchmarkTests, database::DatabaseKind, prelude::*};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BenchmarkRecord {
    pub name: String,
    pub language: String,
    pub language_version: String,
    pub framework: String,
    pub framework_version: String,
    pub tests: Vec<BenchmarkTests>,
    pub tags: HashMap<String, String>,
    pub path: String,
    #[serde(default)]
    pub database: Option<DatabaseKind>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub only: bool,
    #[serde(default)]
    pub arguments: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct Benchmark {
    pub name: String,
    pub language: String,
    pub language_version: String,
    pub framework: String,
    pub framework_version: String,
    pub tests: Vec<BenchmarkTests>,
    pub tags: HashMap<String, String>,
    pub path: String,
    pub database: Option<DatabaseKind>,
    pub disabled: bool,
    pub only: bool,
    pub arguments: Vec<String>,
    pub env: HashMap<String, String>,
}

impl From<&BenchmarkRecord> for Benchmark {
    fn from(record: &BenchmarkRecord) -> Self {
        Self {
            name: record.name.clone(),
            language: record.language.clone(),
            language_version: record.language_version.clone(),
            framework: record.framework.clone(),
            framework_version: record.framework_version.clone(),
            tests: record.tests.clone(),
            tags: record.tags.clone(),
            path: record.path.clone(),
            database: record.database,
            disabled: record.disabled,
            only: record.only,
            arguments: record.arguments.clone(),
            env: record.env.clone(),
        }
    }
}

pub fn parse_benchmarks<P: AsRef<Path>>(path: P) -> Result<Vec<BenchmarkRecord>> {
    let content = fs::read_to_string(path)?;
    let benchmarks: Vec<BenchmarkRecord> = serde_yaml::from_str(&content)?;
    Ok(benchmarks)
}
