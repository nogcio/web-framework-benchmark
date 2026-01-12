pub const BENCHMARK_DURATION_PER_TEST_SECS: u64 = 60 * 4;
pub const BENCHMARK_WARMUP_DURATION_SECS: u64 = 30;
pub const BENCHMARK_STEP_CONNECTIONS_PLAINTEXT: &str = "32,64,128,256,512,1024";
pub const BENCHMARK_STEP_CONNECTIONS_JSON: &str = "32,64,128,256,512";
pub const BENCHMARK_STEP_CONNECTIONS_STATIC: &str = "16,32,64,128,256";
pub const BENCHMARK_STEP_CONNECTIONS_DB_COMPLEX: &str = "32,64,128,256,512";
pub const BENCHMARK_STEP_DURATION_SECS: u64 = 20;
pub const WRKR_IMAGE: &str = "nogcio/wrkr";
pub const BENCHMARK_DATA: &str = "benchmarks_data";

pub const DB_PORT_EXTERNAL: u16 = 54350;
pub const APP_PORT_EXTERNAL: u16 = 54320;
pub const APP_PORT_INTERNAL: u16 = 8080;

pub const DB_USER: &str = "user";
pub const DB_PASS: &str = "password";
pub const DB_NAME: &str = "hello_world";

pub const REMOTE_APP_PATH: &str = "/tmp/wfb/app";
pub const REMOTE_DB_PATH: &str = "/tmp/wfb/database";
pub const REMOTE_WRKR_PATH: &str = "/tmp/wfb/wrkr";

pub const SCRIPT_PLAINTEXT: &str = "scripts/wrkr_plaintext.lua";
pub const SCRIPT_JSON: &str = "scripts/wrkr_json_aggregate.lua";
pub const SCRIPT_STATIC: &str = "scripts/wrkr_static_files.lua";
pub const SCRIPT_DB_COMPLEX: &str = "scripts/wrkr_db_complex.lua";

pub const CONTAINER_HEALTH_RETRIES: u32 = 30;
pub const CONTAINER_HEALTH_INTERVAL_SECS: u64 = 1;
