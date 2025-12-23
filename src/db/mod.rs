pub mod benchmarks;
pub mod frameworks;
pub mod languages;
pub mod runs;

use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use crate::{
    benchmark::{BenchmarkResults, BenchmarkTests},
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
}

impl Db {
    pub fn open() -> Result<Self> {
        let languages = languages::parse_languages("config/languages.yaml")?;
        let frameworks = frameworks::parse_frameworks("config/frameworks.yaml")?;
        let benchmarks = benchmarks::parse_benchmarks("config/benchmarks.yaml")?;
        let runs = runs::load_runs("data")?;
        let inner = DbInner {
            languages,
            frameworks,
            benchmarks,
            runs,
        };
        Ok(Db {
            inner: Arc::new(RwLock::new(inner)),
        })
    }

    pub fn get_languages(&self) -> Result<Vec<languages::Language>> {
        let inner = self.inner.read().map_err(|_| Error::PoisonError)?;
        Ok(inner.languages.iter().map(|l| l.into()).collect())
    }

    pub fn get_runs(&self) -> Result<Vec<runs::RunSummary>> {
        let inner = self.inner.read().map_err(|_| Error::PoisonError)?;
        Ok(inner.runs.iter().map(runs::RunSummary::from).collect())
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
        benchmark_name: &str,
    ) -> Result<bool> {
        let inner = self.inner.read().map_err(|_| Error::PoisonError)?;
        if let Some(run) = inner.runs.iter().find(|r| r.id == run_id)
            && let Some(fw_run) = run.frameworks.iter().find(|fw| {
                fw.environment == environment
                    && fw.language == language
                    && fw.framework == benchmark_name
            })
        {
            return Ok(!fw_run.results.is_empty());
        }
        Ok(false)
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
}
