use std::collections::HashMap;

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkTests {
    PlainText,
    JsonAggregate,
    StaticFiles,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseKind {
    Postgres,
    Mysql,
    Mariadb,
    Mssql,
    Mongodb,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Benchmark {
    pub name: String,
    pub language: String,
    pub language_version: String,
    pub framework: String,
    pub framework_version: String,
    pub path: String,
    #[serde(default)]
    pub tests: Vec<BenchmarkTests>,
    #[serde(default)]
    pub tags: HashMap<String, String>,
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