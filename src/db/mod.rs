pub mod languages;
pub mod runs;

use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, RwLock},
    time::Duration,
};

use crate::{
    benchmark::{BenchmarkResults, BenchmarkTests},
    prelude::*,
};

fn load_runs() -> Result<Vec<runs::Run>> {
    let mut runs = Vec::new();
    for entry in fs::read_dir("data")? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir()
            && let Some(name) = path.file_name().and_then(|n| n.to_str())
            && let Ok(run_id) = name.parse::<u32>()
        {
            // load run manifest
            let run_manifest_path = path.join("manifest.yaml");
            let run_manifest: runs::RunManifest =
                serde_yaml::from_str(&fs::read_to_string(run_manifest_path)?)?;
            let mut frameworks = Vec::new();
            // recurse into environment dirs
            for env_entry in fs::read_dir(&path)? {
                let env_entry = env_entry?;
                let env_path = env_entry.path();
                if env_path.is_dir() {
                    let environment = env_path.file_name().unwrap().to_str().unwrap().to_string();
                    for lang_entry in fs::read_dir(&env_path)? {
                        let lang_entry = lang_entry?;
                        let lang_path = lang_entry.path();
                        if lang_path.is_dir() {
                            let language =
                                lang_path.file_name().unwrap().to_str().unwrap().to_string();
                            for fw_entry in fs::read_dir(&lang_path)? {
                                let fw_entry = fw_entry?;
                                let fw_path = fw_entry.path();
                                if fw_path.is_dir() {
                                    let framework =
                                        fw_path.file_name().unwrap().to_str().unwrap().to_string();
                                    // load framework manifest
                                    let fw_manifest_path = fw_path.join("manifest.yaml");
                                    let fw_manifest: runs::FrameworkManifest =
                                        serde_yaml::from_str(&fs::read_to_string(
                                            fw_manifest_path,
                                        )?)?;
                                    let mut results = HashMap::new();
                                    for test_entry in fs::read_dir(&fw_path)? {
                                        let test_entry = test_entry?;
                                        let test_path = test_entry.path();
                                        if test_path.is_file()
                                            && test_path.extension()
                                                == Some(std::ffi::OsStr::new("yaml"))
                                            && test_path.file_stem()
                                                != Some(std::ffi::OsStr::new("manifest"))
                                        {
                                            let test_name_str =
                                                test_path.file_stem().unwrap().to_str().unwrap();
                                            let test: BenchmarkTests = test_name_str
                                                .try_into()
                                                .map_err(Error::InvalidTest)?;
                                            let run_data: runs::RunData = serde_yaml::from_str(
                                                &fs::read_to_string(&test_path)?,
                                            )?;
                                            results.insert(test, run_data);
                                        }
                                    }
                                    frameworks.push(runs::FrameworkRun {
                                        environment: environment.clone(),
                                        language: language.clone(),
                                        framework,
                                        manifest: fw_manifest,
                                        results,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            runs.push(runs::Run {
                id: run_id,
                manifest: run_manifest,
                frameworks,
            });
        }
    }
    Ok(runs)
}

#[derive(Clone)]
pub struct Db {
    inner: Arc<RwLock<DbInner>>,
}

struct DbInner {
    languages: Vec<languages::Language>,
    #[allow(dead_code)]
    runs: Vec<runs::Run>,
}

impl Db {
    pub fn open() -> Result<Self> {
        let languages = languages::parse_languages("config/languages.yaml")?;
        let runs = load_runs()?;
        let inner = DbInner { languages, runs };
        Ok(Db {
            inner: Arc::new(RwLock::new(inner)),
        })
    }

    pub fn get_languages(&self) -> Result<Vec<languages::Language>> {
        let inner = self.inner.read().map_err(|_| Error::PoisonError)?;
        Ok(inner.languages.clone())
    }

    #[allow(dead_code)]
    pub fn get_runs(&self) -> Result<Vec<runs::Run>> {
        let inner = self.inner.read().map_err(|_| Error::PoisonError)?;
        Ok(inner.runs.clone())
    }

    #[allow(dead_code)]
    pub fn get_frameworks(&self) -> Result<Vec<runs::FrameworkWithLanguage>> {
        let inner = self.inner.read().map_err(|_| Error::PoisonError)?;
        let mut frameworks = Vec::new();
        for lang in &inner.languages {
            for fw in &lang.frameworks {
                frameworks.push(runs::FrameworkWithLanguage {
                    language: lang.name.clone(),
                    framework: fw.clone(),
                });
            }
        }
        Ok(frameworks)
    }

    #[allow(dead_code)]
    pub fn get_tag_keys(&self) -> Result<Vec<String>> {
        let inner = self.inner.read().map_err(|_| Error::PoisonError)?;
        let mut keys = std::collections::HashSet::new();
        for lang in &inner.languages {
            for fw in &lang.frameworks {
                for key in fw.tags.keys() {
                    keys.insert(key.clone());
                }
            }
        }
        Ok(keys.into_iter().collect())
    }

    #[allow(dead_code)]
    pub fn get_environments(&self) -> Result<Vec<String>> {
        crate::benchmark_environment::list_environments()
    }

    #[allow(dead_code)]
    pub fn get_run_results(
        &self,
        run_id: u32,
        environment: String,
        test: BenchmarkTests,
    ) -> Result<Vec<runs::RunResult>> {
        let inner = self.inner.read().map_err(|_| Error::PoisonError)?;
        let mut results = Vec::new();
        if let Some(run) = inner.runs.iter().find(|r| r.id == run_id) {
            for fw_run in &run.frameworks {
                if fw_run.environment == environment
                    && let Some(result) = fw_run.results.get(&test)
                {
                    results.push(runs::RunResult {
                        language: fw_run.language.clone(),
                        framework: fw_run.framework.clone(),
                        version: fw_run.manifest.version.clone(),
                        rps: result.requests_per_sec,
                        tps: result.transfer_per_sec,
                        latency_avg: result.latency_avg,
                        latency_stdev: result.latency_stdev,
                        latency_max: result.latency_max,
                        latency50: result
                            .latency_distribution
                            .iter()
                            .find(|(p, _)| *p == 50)
                            .map(|(_, d)| *d)
                            .unwrap_or(Duration::ZERO),
                        latency75: result
                            .latency_distribution
                            .iter()
                            .find(|(p, _)| *p == 75)
                            .map(|(_, d)| *d)
                            .unwrap_or(Duration::ZERO),
                        latency90: result
                            .latency_distribution
                            .iter()
                            .find(|(p, _)| *p == 90)
                            .map(|(_, d)| *d)
                            .unwrap_or(Duration::ZERO),
                        latency99: result
                            .latency_distribution
                            .iter()
                            .find(|(p, _)| *p == 99)
                            .map(|(_, d)| *d)
                            .unwrap_or(Duration::ZERO),
                        latency_stdev_pct: result.latency_stdev_pct,
                        latency_distribution: result.latency_distribution.clone(),
                        req_per_sec_avg: result.req_per_sec_avg,
                        req_per_sec_stdev: result.req_per_sec_stdev,
                        req_per_sec_max: result.req_per_sec_max,
                        req_per_sec_stdev_pct: result.req_per_sec_stdev_pct,
                        errors: result.errors,
                        memory_usage: result.memory_usage,
                        tags: fw_run.manifest.tags.clone(),
                    });
                }
            }
        }
        Ok(results)
    }

    pub fn has_framework_results(
        &self,
        run_id: u32,
        environment: &str,
        language: &str,
        framework: &str,
    ) -> Result<bool> {
        let inner = self.inner.read().map_err(|_| Error::PoisonError)?;
        if let Some(run) = inner.runs.iter().find(|r| r.id == run_id)
            && let Some(fw_run) = run.frameworks.iter().find(|fw| {
                fw.environment == environment
                    && fw.language == language
                    && fw.framework == framework
            })
        {
            return Ok(!fw_run.results.is_empty());
        }
        Ok(false)
    }

    pub fn save_run(
        &self,
        run_id: u32,
        environment: &str,
        lang: &languages::Language,
        framework: &languages::Framework,
        benchmark_results: &BenchmarkResults,
    ) -> Result<()> {
        let mut path = PathBuf::new();
        path.push("data");
        path.push(run_id.to_string());
        let run_manifest_path = path.join("manifest.yaml");
        path.push(environment);
        path.push(&lang.name);
        path.push(&framework.name);
        let framework_manifest_path = path.join("manifest.yaml");
        fs::create_dir_all(&path)?;
        if !run_manifest_path.exists() {
            let run_manifest = runs::RunManifest {
                created_at: chrono::Utc::now(),
            };
            let run_manifest_content = serde_yaml::to_string(&run_manifest)?;
            fs::write(run_manifest_path, run_manifest_content)?;
        }
        if !framework_manifest_path.exists() {
            let mut tags_map = std::collections::HashMap::new();
            for (key, value) in &framework.tags {
                tags_map.insert(key.clone(), value.clone());
            }
            let framework_manifest = runs::FrameworkManifest {
                version: benchmark_results.version.clone(),
                tags: tags_map,
            };
            let framework_manifest_content = serde_yaml::to_string(&framework_manifest)?;
            fs::write(framework_manifest_path, framework_manifest_content)?;
        }

        for (test, result) in &benchmark_results.results {
            let test_path = path.join(format!("{}.yaml", test));
            let run_result = runs::RunData {
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
}
