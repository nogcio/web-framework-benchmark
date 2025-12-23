pub mod benchmarks;
pub mod frameworks;
pub mod languages;
pub mod runs;
pub mod environments;

use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use crate::{
    benchmark::{BenchmarkResults, BenchmarkTests},
    benchmark_environment::config::EnvironmentFile,
    prelude::*,
};

#[derive(Clone)]
pub struct Db {
    inner: Arc<RwLock<DbInner>>,
}

struct DbInner {
    languages: Vec<languages::LanguageRecord>,
    frameworks: Vec<frameworks::FrameworkRecord>,
    benchmarks: Vec<benchmarks::BenchmarkRecord>,
    runs: Vec<runs::RunRecord>,
    environments: Vec<environments::EnvironmentRecord>,
}

impl Db {
    pub fn open() -> Result<Self> {
        let languages = languages::parse_languages("config/languages.yaml")?;
        let frameworks = frameworks::parse_frameworks("config/frameworks.yaml")?;
        let benchmarks = benchmarks::parse_benchmarks("config/benchmarks.yaml")?;
        let runs = runs::load_runs("data")?;
        let environments = environments::load_environments("config/environments")?;
        let inner = DbInner {
            languages,
            frameworks,
            benchmarks,
            runs,
            environments,
        };
        Ok(Db {
            inner: Arc::new(RwLock::new(inner)),
        })
    }

    pub fn get_environment(&self, id: &str) -> Result<Option<EnvironmentFile>> {
        let inner = self.inner.read().map_err(|_| Error::PoisonError)?;
        Ok(inner.environments.iter().find(|e| e.id == id).map(|e| e.config.clone()))
    }

    pub fn get_languages(&self) -> Result<Vec<languages::Language>> {
        let inner = self.inner.read().map_err(|_| Error::PoisonError)?;
        Ok(inner.languages.iter().map(|l| l.into()).collect())
    }

    pub fn get_runs(&self) -> Result<Vec<runs::RunSummary>> {
        let inner = self.inner.read().map_err(|_| Error::PoisonError)?;
        Ok(inner.runs.iter().map(runs::RunSummary::from).collect())
    }

    pub fn get_full_runs(&self) -> Result<Vec<runs::RunRecord>> {
        let inner = self.inner.read().map_err(|_| Error::PoisonError)?;
        Ok(inner.runs.clone())
    }

    pub fn get_framework_records(&self) -> Result<Vec<frameworks::FrameworkRecord>> {
        let inner = self.inner.read().map_err(|_| Error::PoisonError)?;
        Ok(inner.frameworks.clone())
    }

    pub fn get_language_records(&self) -> Result<Vec<languages::LanguageRecord>> {
        let inner = self.inner.read().map_err(|_| Error::PoisonError)?;
        Ok(inner.languages.clone())
    }

    pub fn get_benchmark_records(&self) -> Result<Vec<benchmarks::BenchmarkRecord>> {
        let inner = self.inner.read().map_err(|_| Error::PoisonError)?;
        Ok(inner.benchmarks.clone())
    }

    pub fn get_frameworks(&self) -> Result<Vec<frameworks::Framework>> {
        let inner = self.inner.read().map_err(|_| Error::PoisonError)?;
        Ok(inner.frameworks.iter().map(|f| f.into()).collect())
    }

    pub fn get_benchmarks(&self) -> Result<Vec<benchmarks::Benchmark>> {
        let inner = self.inner.read().map_err(|_| Error::PoisonError)?;
        Ok(inner.benchmarks.iter().map(|b| b.into()).collect())
    }

    pub fn get_tag_keys(&self) -> Result<Vec<String>> {
        let inner = self.inner.read().map_err(|_| Error::PoisonError)?;
        let mut keys = std::collections::HashSet::new();
        for benchmark in &inner.benchmarks {
            for key in benchmark.tags.keys() {
                keys.insert(key.clone());
            }
        }
        Ok(keys.into_iter().collect())
    }

    pub fn get_environments(&self) -> Result<Vec<String>> {
        let inner = self.inner.read().map_err(|_| Error::PoisonError)?;
        let mut environments = std::collections::HashSet::new();
        for run in &inner.runs {
            for fw_run in &run.frameworks {
                environments.insert(fw_run.environment.clone());
            }
        }
        let mut result: Vec<String> = environments.into_iter().collect();
        result.sort();
        Ok(result)
    }

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
                    let manifest = &fw_run.manifest;
                    let path = inner
                        .benchmarks
                        .iter()
                        .find(|b| b.name == fw_run.framework)
                        .map(|b| b.path.clone());

                    let transcript_path = std::path::Path::new("data")
                        .join(run_id.to_string())
                        .join(&fw_run.environment)
                        .join(&manifest.language)
                        .join(&fw_run.framework);
                    
                    let test_str = test.to_string();
                    let has_transcript = transcript_path.join(format!("{}.md", test_str)).exists() || {
                        if let Ok(entries) = std::fs::read_dir(&transcript_path) {
                            entries.flatten().any(|entry| {
                                if let Some(name) = entry.file_name().to_str() {
                                    name.starts_with(&test_str) && name.ends_with(".md") && name[test_str.len()..].starts_with('.')
                                } else {
                                    false
                                }
                            })
                        } else {
                            false
                        }
                    };

                    results.push(runs::RunResult {
                        name: fw_run.framework.clone(),
                        language: manifest.language.clone(),
                        language_version: manifest.language_version.clone(),
                        framework: manifest.framework.clone(),
                        framework_version: manifest.framework_version.clone(),
                        database: manifest.database.clone(),
                        path,
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
                        tags: manifest.tags.clone(),
                        has_transcript,
                    });
                }
            }
        }
        Ok(results)
    }

    pub fn get_completed_tests(
        &self,
        run_id: u32,
        environment: &str,
        language: &str,
        benchmark_name: &str,
    ) -> Result<Vec<BenchmarkTests>> {
        let inner = self.inner.read().map_err(|_| Error::PoisonError)?;
        if let Some(run) = inner.runs.iter().find(|r| r.id == run_id)
            && let Some(fw_run) = run.frameworks.iter().find(|fw| {
                fw.environment == environment
                    && fw.language == language
                    && fw.framework == benchmark_name
            })
        {
            return Ok(fw_run.results.keys().cloned().collect());
        }
        Ok(Vec::new())
    }

    pub fn save_run(
        &self,
        run_id: u32,
        environment: &str,
        benchmark: &benchmarks::Benchmark,
        benchmark_results: &BenchmarkResults,
    ) -> Result<()> {
        runs::save_run("data", run_id, environment, benchmark, benchmark_results)
    }

    pub fn get_transcript(
        &self,
        run_id: u32,
        environment: &str,
        test: &str,
        framework: &str,
        lang: Option<&str>,
    ) -> Result<Option<std::path::PathBuf>> {
        let inner = self.inner.read().map_err(|_| Error::PoisonError)?;
        if let Some(run) = inner.runs.iter().find(|r| r.id == run_id) {
            for fw_run in &run.frameworks {
                if fw_run.environment == environment && fw_run.framework == framework {
                    let base_path = std::path::Path::new("data")
                        .join(run_id.to_string())
                        .join(environment)
                        .join(&fw_run.language)
                        .join(framework);

                    let requested_lang = lang.unwrap_or("en");

                    // Try specific language
                    let lang_path = base_path.join(format!("{}.{}.md", test, requested_lang));
                    if lang_path.exists() {
                        return Ok(Some(lang_path));
                    }

                    // Try English fallback if requested lang wasn't en
                    if requested_lang != "en" {
                        let en_path = base_path.join(format!("{}.en.md", test));
                        if en_path.exists() {
                            return Ok(Some(en_path));
                        }
                    }

                    // Try default file
                    let default_path = base_path.join(format!("{}.md", test));
                    if default_path.exists() {
                        return Ok(Some(default_path));
                    }

                    return Ok(None);
                }
            }
        }
        Ok(None)
    }
}
