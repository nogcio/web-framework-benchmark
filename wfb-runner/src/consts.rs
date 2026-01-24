pub const BENCHMARK_DURATION_PER_TEST_SECS: u64 = 60 * 4;
pub const BENCHMARK_WARMUP_DURATION_SECS: u64 = 30;
pub const BENCHMARK_WARMUP_MAX_VUS: u64 = 4;

// Verification runs: keep short/light; correctness-focused.
pub const VERIFY_DURATION_SECS: u64 = 3;
pub const VERIFY_MAX_VUS: u64 = 4;
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

// Scripts executed by the external `nogcio/wrkr` container (mounted via -v ./scripts:/scripts).
pub const SCRIPT_PLAINTEXT: &str = "/scripts/wfb_plaintext.lua";
pub const SCRIPT_JSON: &str = "/scripts/wfb_json_aggregate.lua";
pub const SCRIPT_STATIC: &str = "/scripts/wfb_static_files.lua";
pub const SCRIPT_DB_COMPLEX: &str = "/scripts/wfb_db_complex.lua";
pub const SCRIPT_GRPC_AGGREGATE: &str = "/scripts/wfb_grpc_aggregate.lua";

pub const UVS_PLAINTEXT: u64 = 1024;
pub const UVS_JSON: u64 = 512;
pub const UVS_GRPC: u64 = 512;
pub const UVS_DB_COMPLEX: u64 = 128;
pub const UVS_STATIC: u64 = 128;

pub const CONTAINER_HEALTH_RETRIES: u32 = 30;
pub const CONTAINER_HEALTH_INTERVAL_SECS: u64 = 1;
