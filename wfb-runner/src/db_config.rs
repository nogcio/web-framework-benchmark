use wfb_storage::DatabaseKind;

use crate::consts;

pub struct DatabaseConfig {
    pub image_name: &'static str,
    pub build_path: &'static str,
    pub port: u16,
    pub env_vars: Vec<(&'static str, &'static str)>,
}

pub fn get_db_config(db_kind: &DatabaseKind) -> DatabaseConfig {
    match db_kind {
        DatabaseKind::Postgres => DatabaseConfig {
            image_name: "postgres",
            build_path: "benchmarks_db/pg",
            port: 5432,
            env_vars: vec![
                ("POSTGRES_PASSWORD", consts::DB_PASS),
                ("POSTGRES_USER", consts::DB_USER),
                ("POSTGRES_DB", consts::DB_NAME),
            ],
        },
        DatabaseKind::Mysql => DatabaseConfig {
            image_name: "mysql",
            build_path: "benchmarks_db/mysql",
            port: 3306,
            env_vars: vec![
                ("MYSQL_ROOT_PASSWORD", consts::DB_PASS),
                ("MYSQL_DATABASE", consts::DB_NAME),
                ("MYSQL_USER", consts::DB_USER),
                ("MYSQL_PASSWORD", consts::DB_PASS),
            ],
        },
        DatabaseKind::Mongodb => DatabaseConfig {
            image_name: "mongodb",
            build_path: "benchmarks_db/mongodb",
            port: 27017,
            env_vars: vec![
                ("MONGO_INITDB_ROOT_USERNAME", consts::DB_USER),
                ("MONGO_INITDB_ROOT_PASSWORD", consts::DB_PASS),
                ("MONGO_INITDB_DATABASE", consts::DB_NAME),
            ],
        },
        DatabaseKind::Mssql => DatabaseConfig {
            image_name: "mssql",
            build_path: "benchmarks_db/mssql",
            port: 1433,
            env_vars: vec![
                ("ACCEPT_EULA", "Y"),
                ("MSSQL_SA_PASSWORD", "Benchmark!12345"),
                ("MSSQL_PID", "Developer"),
            ],
        },
        DatabaseKind::Mariadb => DatabaseConfig {
            image_name: "mariadb",
            build_path: "benchmarks_db/mariadb",
            port: 3306,
            env_vars: vec![
                ("MARIADB_ROOT_PASSWORD", consts::DB_PASS),
                ("MARIADB_DATABASE", consts::DB_NAME),
                ("MARIADB_USER", consts::DB_USER),
                ("MARIADB_PASSWORD", consts::DB_PASS),
            ],
        },
    }
}
