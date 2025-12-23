use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseKind {
    Postgres,
    Mysql,
    Mariadb,
    Mssql,
    Mongodb,
}

impl DatabaseKind {
    pub fn port(self) -> u16 {
        match self {
            DatabaseKind::Postgres => 5432,
            DatabaseKind::Mysql => 3306,
            DatabaseKind::Mariadb => 3306,
            DatabaseKind::Mssql => 1433,
            DatabaseKind::Mongodb => 27017,
        }
    }

    pub fn dir(self) -> &'static str {
        match self {
            DatabaseKind::Postgres => "benchmarks_db/pg",
            DatabaseKind::Mysql => "benchmarks_db/mysql",
            DatabaseKind::Mariadb => "benchmarks_db/mariadb",
            DatabaseKind::Mssql => "benchmarks_db/mssql",
            DatabaseKind::Mongodb => "benchmarks_db/mongodb",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            DatabaseKind::Postgres => "pg",
            DatabaseKind::Mysql => "mysql",
            DatabaseKind::Mariadb => "mariadb",
            DatabaseKind::Mssql => "mssql",
            DatabaseKind::Mongodb => "mongodb",
        }
    }
}

impl TryFrom<&str> for DatabaseKind {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "pg" | "postgres" | "postgresql" => Ok(DatabaseKind::Postgres),
            "mysql" => Ok(DatabaseKind::Mysql),
            "mariadb" => Ok(DatabaseKind::Mariadb),
            "mssql" | "sqlserver" | "microsoft sql server" => Ok(DatabaseKind::Mssql),
            "mongo" | "mongodb" => Ok(DatabaseKind::Mongodb),
            other => Err(format!("Unsupported database: {}", other)),
        }
    }
}

impl std::fmt::Display for DatabaseKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
