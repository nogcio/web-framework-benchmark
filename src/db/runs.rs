use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    benchmark::{BenchmarkResults, BenchmarkTests},
    db::benchmarks,
    prelude::*,
};

fn serialize_duration_as_nanos<S>(
    duration: &std::time::Duration,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u128(duration.as_nanos())
}

fn serialize_latency_distribution<S>(
    dist: &Vec<(u8, std::time::Duration)>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeSeq;
    let mut seq = serializer.serialize_seq(Some(dist.len()))?;
    for (percent, duration) in dist {
        seq.serialize_element(&(percent, duration.as_nanos()))?;
    }
    seq.end()
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RunManifest {
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct FrameworkManifest {
    pub language: String,
    pub language_version: String,
    pub framework: String,
    pub framework_version: String,
    pub database: Option<String>,
    pub tags: HashMap<String, String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RunDataRecord {
    pub requests_per_sec: f64,
    pub transfer_per_sec: u64,
    pub latency_avg: std::time::Duration,
    pub latency_stdev: std::time::Duration,
    pub latency_max: std::time::Duration,
    pub latency_stdev_pct: f64,
    pub latency_distribution: Vec<(u8, std::time::Duration)>,
    pub req_per_sec_avg: f64,
    pub req_per_sec_stdev: f64,
    pub req_per_sec_max: f64,
    pub req_per_sec_stdev_pct: f64,
    pub errors: i64,
    pub memory_usage: u64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RunRecord {
    pub id: u32,
    pub manifest: RunManifest,
    pub frameworks: Vec<FrameworkRunRecord>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FrameworkRunRecord {
    pub environment: String,
    pub language: String,
    pub framework: String,
    pub manifest: FrameworkManifest,
    pub results: HashMap<BenchmarkTests, RunDataRecord>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct RunResult {
    pub name: String,
    pub language: String,
    pub language_version: String,
    pub framework: String,
    pub framework_version: String,
    pub database: Option<String>,
    pub path: Option<String>,
    pub rps: f64,
    pub tps: u64,
    #[serde(serialize_with = "serialize_duration_as_nanos")]
    pub latency_avg: std::time::Duration,
    #[serde(serialize_with = "serialize_duration_as_nanos")]
    pub latency_stdev: std::time::Duration,
    #[serde(serialize_with = "serialize_duration_as_nanos")]
    pub latency_max: std::time::Duration,
    #[serde(serialize_with = "serialize_duration_as_nanos")]
    pub latency50: std::time::Duration,
    #[serde(serialize_with = "serialize_duration_as_nanos")]
    pub latency75: std::time::Duration,
    #[serde(serialize_with = "serialize_duration_as_nanos")]
    pub latency90: std::time::Duration,
    #[serde(serialize_with = "serialize_duration_as_nanos")]
    pub latency99: std::time::Duration,
    pub latency_stdev_pct: f64,
    #[serde(serialize_with = "serialize_latency_distribution")]
    pub latency_distribution: Vec<(u8, std::time::Duration)>,
    pub req_per_sec_avg: f64,
    pub req_per_sec_stdev: f64,
    pub req_per_sec_max: f64,
    pub req_per_sec_stdev_pct: f64,
    pub errors: i64,
    pub memory_usage: u64,
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSummary {
    pub id: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<&RunRecord> for RunSummary {
    fn from(run: &RunRecord) -> Self {
        Self {
            id: run.id,
            created_at: run.manifest.created_at,
        }
    }
}

pub fn load_runs(base: impl AsRef<Path>) -> Result<Vec<RunRecord>> {
    let base = base.as_ref();
    let mut runs = Vec::new();
    for entry in fs::read_dir(base)? {
        let entry = entry?;
        let run_path = entry.path();
        if !run_path.is_dir() {
            continue;
        }
        let Some(name) = run_path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Ok(run_id) = name.parse::<u32>() else {
            continue;
        };

        let run_manifest_path = run_path.join("manifest.yaml");
        let run_manifest: RunManifest =
            serde_yaml::from_str(&fs::read_to_string(run_manifest_path)?)?;

        let mut frameworks = Vec::new();
        for env_entry in fs::read_dir(&run_path)? {
            let env_entry = env_entry?;
            let env_path = env_entry.path();
            if !env_path.is_dir() {
                continue;
            }
            let Some(environment) = env_path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };

            for lang_entry in fs::read_dir(&env_path)? {
                let lang_entry = lang_entry?;
                let lang_path = lang_entry.path();
                if !lang_path.is_dir() {
                    continue;
                }
                let Some(language) = lang_path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };

                for fw_entry in fs::read_dir(&lang_path)? {
                    let fw_entry = fw_entry?;
                    let fw_path = fw_entry.path();
                    if !fw_path.is_dir() {
                        continue;
                    }
                    let Some(framework) = fw_path.file_name().and_then(|n| n.to_str()) else {
                        continue;
                    };

                    let fw_manifest_path = fw_path.join("manifest.yaml");
                    let fw_manifest: FrameworkManifest =
                        serde_yaml::from_str(&fs::read_to_string(fw_manifest_path)?)?;

                    let mut results = HashMap::new();
                    for test_entry in fs::read_dir(&fw_path)? {
                        let test_entry = test_entry?;
                        let test_path = test_entry.path();
                        if !test_path.is_file() {
                            continue;
                        }
                        if test_path.extension() != Some(std::ffi::OsStr::new("yaml")) {
                            continue;
                        }
                        if test_path.file_stem() == Some(std::ffi::OsStr::new("manifest")) {
                            continue;
                        }

                        let test_name_str = test_path.file_stem().unwrap().to_str().unwrap();
                        let test: BenchmarkTests =
                            test_name_str.try_into().map_err(Error::InvalidTest)?;
                        let run_data: RunDataRecord =
                            serde_yaml::from_str(&fs::read_to_string(&test_path)?)?;
                        results.insert(test, run_data);
                    }

                    frameworks.push(FrameworkRunRecord {
                        environment: environment.to_string(),
                        language: language.to_string(),
                        framework: framework.to_string(),
                        manifest: fw_manifest,
                        results,
                    });
                }
            }
        }

        runs.push(RunRecord {
            id: run_id,
            manifest: run_manifest,
            frameworks,
        });
    }

    Ok(runs)
}

pub fn save_run(
    base: impl AsRef<Path>,
    run_id: u32,
    environment: &str,
    benchmark: &benchmarks::Benchmark,
    benchmark_results: &BenchmarkResults,
) -> Result<()> {
    let mut path = PathBuf::from(base.as_ref());
    path.push(run_id.to_string());
    let run_manifest_path = path.join("manifest.yaml");
    path.push(environment);
    path.push(&benchmark.language);
    path.push(&benchmark.name);
    let framework_manifest_path: PathBuf = path.join("manifest.yaml");
    fs::create_dir_all(&path)?;

    if !run_manifest_path.exists() {
        let run_manifest = RunManifest {
            created_at: chrono::Utc::now(),
        };
        let run_manifest_content = serde_yaml::to_string(&run_manifest)?;
        fs::write(&run_manifest_path, run_manifest_content)?;
    }

    if !framework_manifest_path.exists() {
        let mut tags_map = HashMap::new();
        for (key, value) in &benchmark.tags {
            tags_map.insert(key.clone(), value.clone());
        }
        let framework_manifest = FrameworkManifest {
            language: benchmark.language.clone(),
            language_version: benchmark.language_version.clone(),
            framework: benchmark.framework.clone(),
            framework_version: benchmark_results.version.clone(),
            database: benchmark.database.map(|db| db.to_string()),
            tags: tags_map,
        };
        let framework_manifest_content = serde_yaml::to_string(&framework_manifest)?;
        fs::write(&framework_manifest_path, framework_manifest_content)?;
    }

    for (test, result) in &benchmark_results.results {
        let test_path = path.join(format!("{}.yaml", test));
        let run_result = RunDataRecord {
            requests_per_sec: result.wrk_result.requests_per_sec,
            transfer_per_sec: result.wrk_result.transfer_per_sec,
            latency_avg: result.wrk_result.latency_avg,
            latency_stdev: result.wrk_result.latency_stdev,
            latency_max: result.wrk_result.latency_max,
            latency_stdev_pct: result.wrk_result.latency_stdev_pct,
            latency_distribution: result.wrk_result.latency_distribution.clone(),
            req_per_sec_avg: result.wrk_result.req_per_sec_avg,
            req_per_sec_stdev: result.wrk_result.req_per_sec_stdev,
            req_per_sec_max: result.wrk_result.req_per_sec_max,
            req_per_sec_stdev_pct: result.wrk_result.req_per_sec_stdev_pct,
            memory_usage: result.memory_usage,
            errors: result.wrk_result.errors,
        };
        let result_content = serde_yaml::to_string(&run_result)?;
        fs::write(test_path, result_content)?;
    }

    Ok(())
}
