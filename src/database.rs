use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseKind {
    Postgres,
    Mysql,
    Mssql,
}

impl DatabaseKind {
    pub fn port(self) -> u16 {
        match self {
            DatabaseKind::Postgres => 5432,
            DatabaseKind::Mysql => 3306,
            DatabaseKind::Mssql => 1433,
        }
    }

    pub fn dir(self) -> &'static str {
        match self {
            DatabaseKind::Postgres => "benchmarks_db/pg",
            DatabaseKind::Mysql => "benchmarks_db/mysql",
            DatabaseKind::Mssql => "benchmarks_db/mssql",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            DatabaseKind::Postgres => "pg",
            DatabaseKind::Mysql => "mysql",
            DatabaseKind::Mssql => "mssql",
        }
    }
}

impl TryFrom<&str> for DatabaseKind {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "pg" | "postgres" | "postgresql" => Ok(DatabaseKind::Postgres),
            "mysql" => Ok(DatabaseKind::Mysql),
            "mssql" | "sqlserver" | "microsoft sql server" => Ok(DatabaseKind::Mssql),
            other => Err(format!("Unsupported database: {}", other)),
        }
    }
}

impl std::fmt::Display for DatabaseKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
