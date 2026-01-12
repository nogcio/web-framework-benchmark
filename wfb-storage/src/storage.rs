use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::benchmark::{Benchmark, BenchmarkManifest, BenchmarkTests};
use crate::environment::Environment;
use crate::error::Result;
use crate::lang::Lang;
use crate::testcase::{TestCaseSummary, TestCaseRaw};

// RunId -> Environment -> Language -> BenchmarkName -> BenchmarkResult
pub type StorageData =
    HashMap<String, HashMap<String, HashMap<String, HashMap<String, BenchmarkResult>>>>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RunManifest {
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone)]
pub struct Storage {
    pub base_path: PathBuf,
    pub data: Arc<RwLock<StorageData>>,
    pub runs: Arc<RwLock<HashMap<String, RunManifest>>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BenchmarkResult {
    pub manifest: BenchmarkManifest,
    pub test_cases: HashMap<String, TestCaseSummary>,
    #[serde(skip)]
    pub raw_data: HashMap<String, Vec<TestCaseRaw>>,
}

impl Storage {
    pub fn new(base_path: impl Into<PathBuf>) -> Result<Self> {
        let base_path = base_path.into();
        let (data, runs) = Self::load_all(&base_path)?;
        Ok(Self {
            base_path,
            data: Arc::new(RwLock::new(data)),
            runs: Arc::new(RwLock::new(runs)),
        })
    }

    #[allow(clippy::collapsible_if)]
    fn load_all(base_path: &Path) -> Result<(StorageData, HashMap<String, RunManifest>)> {
        let mut data = HashMap::new();
        let mut runs = HashMap::new();
        if !base_path.exists() {
            return Ok((data, runs));
        }

        for run_entry in fs::read_dir(base_path)? {
            let run_entry = run_entry?;
            if !run_entry.file_type()?.is_dir() {
                continue;
            }
            let run_id = run_entry.file_name().to_string_lossy().to_string();

            // Load run manifest
            let manifest_path = run_entry.path().join("manifest.yaml");
            if manifest_path.exists() {
                if let Ok(file) = fs::File::open(&manifest_path) {
                    if let Ok(manifest) = serde_yaml::from_reader(file) {
                        runs.insert(run_id.clone(), manifest);
                    }
                }
            }

            let run_data = data.entry(run_id.clone()).or_default();

            for env_entry in fs::read_dir(run_entry.path())? {
                let env_entry = env_entry?;
                if !env_entry.file_type()?.is_dir() {
                    continue;
                }
                let environment = env_entry.file_name().to_string_lossy().to_string();
                let env_data = run_data.entry(environment.clone()).or_default();

                for lang_entry in fs::read_dir(env_entry.path())? {
                    let lang_entry = lang_entry?;
                    if !lang_entry.file_type()?.is_dir() {
                        continue;
                    }
                    let language = lang_entry.file_name().to_string_lossy().to_string();
                    let lang_results = env_data.entry(language.clone()).or_default();

                    for bench_entry in fs::read_dir(lang_entry.path())? {
                        let bench_entry = bench_entry?;
                        if !bench_entry.file_type()?.is_dir() {
                            continue;
                        }
                        let benchmark_name = bench_entry.file_name().to_string_lossy().to_string();
                        let benchmark_path = bench_entry.path();

                        // Load manifest
                        let manifest_path = benchmark_path.join("manifest.yaml");
                        if !manifest_path.exists() {
                            continue;
                        }
                        let manifest_file = fs::File::open(&manifest_path)?;
                        let manifest: BenchmarkManifest = match serde_yaml::from_reader(manifest_file) {
                            Ok(m) => m,
                            Err(_) => continue,
                        };

                        let mut test_cases = HashMap::new();
                        let mut raw_data = HashMap::new();

                        // Load test cases
                        if let Ok(entries) = fs::read_dir(&benchmark_path) {
                            for file_entry in entries {
                                if let Ok(file_entry) = file_entry {
                                    let path = file_entry.path();
                                    if let Some(extension) = path.extension().and_then(|s| s.to_str()) {
                                        let file_stem =
                                            path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                                        if file_stem == "manifest" {
                                            continue;
                                        }

                                        if extension == "yaml" {
                                            if let Ok(summary_file) = fs::File::open(&path) {
                                                if let Ok(summary) = serde_yaml::from_reader::<_, TestCaseSummary>(summary_file) {
                                                    test_cases.insert(file_stem.to_string(), summary);
                                                }
                                            }
                                        } else if extension == "jsonl" && file_stem.ends_with("_raw") {
                                            let test_name = file_stem.trim_end_matches("_raw");
                                            if let Ok(file) = fs::File::open(&path) {
                                                let reader = std::io::BufReader::new(file);
                                                let mut results = Vec::new();
                                                use std::io::BufRead;
                                                for line in reader.lines() {
                                                    if let Ok(line) = line {
                                                        if line.is_empty() {
                                                            continue;
                                                        }
                                                        if let Ok(item) = serde_json::from_str::<TestCaseRaw>(&line) {
                                                            results.push(item);
                                                        }
                                                    }
                                                }
                                                raw_data.insert(test_name.to_string(), results);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        lang_results.insert(
                            benchmark_name,
                            BenchmarkResult {
                                manifest,
                                test_cases,
                                raw_data,
                            },
                        );
                    }
                }
            }
        }
        Ok((data, runs))
    }

    fn get_benchmark_path(
        &self,
        run_id: &str,
        environment: &Environment,
        language: &Lang,
        benchmark: &Benchmark,
    ) -> PathBuf {
        self.base_path
            .join(run_id)
            .join(environment.name())
            .join(&language.name)
            .join(&benchmark.name)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn save_benchmark_result(
        &self,
        run_id: &str,
        environment: &Environment,
        language: &Lang,
        benchmark: &Benchmark,
        testcase: BenchmarkTests,
        manifest: &BenchmarkManifest,
        summary: &TestCaseSummary,
        raw_data: &[TestCaseRaw],
    ) -> Result<()> {
        // Update memory
        {
            let mut data = self.data.write().unwrap();
            let run_data = data.entry(run_id.to_string()).or_default();
            let env_data = run_data.entry(environment.name().to_string()).or_default();
            let lang_data = env_data.entry(language.name.to_string()).or_default();

            let bench_result = lang_data
                .entry(benchmark.name.to_string())
                .or_insert_with(|| BenchmarkResult {
                    manifest: manifest.clone(),
                    test_cases: HashMap::new(),
                    raw_data: HashMap::new(),
                });

            // Update manifest in case it changed (though usually it shouldn't for same benchmark)
            bench_result.manifest = manifest.clone();
            bench_result
                .test_cases
                .insert(testcase.to_string(), summary.clone());
            bench_result.raw_data.insert(testcase.to_string(), raw_data.to_vec());
        }

        // Save to disk
        let benchmark_path = self.get_benchmark_path(run_id, environment, language, benchmark);
        fs::create_dir_all(&benchmark_path)?;

        // Save run manifest if it doesn't exist
        let run_path = self.base_path.join(run_id);
        let run_manifest_path = run_path.join("manifest.yaml");
        if !run_manifest_path.exists() {
            let manifest = RunManifest {
                created_at: chrono::Utc::now(),
            };
            let file = fs::File::create(&run_manifest_path)?;
            serde_yaml::to_writer(file, &manifest)?;

            // Update memory
            self.runs
                .write()
                .unwrap()
                .insert(run_id.to_string(), manifest);
        }

        // Save manifest if it doesn't exist
        let manifest_path = benchmark_path.join("manifest.yaml");
        if !manifest_path.exists() {
            let manifest_file = fs::File::create(&manifest_path)?;
            serde_yaml::to_writer(manifest_file, manifest)?;
        }

        // Save summary
        let summary_path = benchmark_path.join(format!("{}.yaml", testcase));
        let summary_file = fs::File::create(&summary_path)?;
        serde_yaml::to_writer(summary_file, summary)?;

        // Save raw data
        let raw_path = benchmark_path.join(format!("{}_raw.jsonl", testcase));
        let mut raw_file = fs::File::create(&raw_path)?;
        for item in raw_data {
            serde_json::to_writer(&mut raw_file, item)?;
            raw_file.write_all(b"\n")?;
        }

        Ok(())
    }

    pub fn get_raw_data(
        &self,
        run_id: &str,
        environment: &str,
        language: &str,
        benchmark: &str,
        testcase: &str,
    ) -> Option<Vec<TestCaseRaw>> {
        let data = self.data.read().unwrap();
        data.get(run_id)
            .and_then(|env| env.get(environment))
            .and_then(|lang| lang.get(language))
            .and_then(|bench| bench.get(benchmark))
            .and_then(|result| result.raw_data.get(testcase))
            .cloned()
    }

    pub fn load_run(
        &self,
        run_id: &str,
        environment: &Environment,
    ) -> Result<HashMap<String, HashMap<String, BenchmarkResult>>> {
        let data = self.data.read().unwrap();
        if let Some(env_data) = data
            .get(run_id)
            .and_then(|run_data| run_data.get(environment.name()))
        {
            return Ok(env_data.clone());
        }
        Ok(HashMap::new())
    }

    pub fn has_test_result(
        &self,
        run_id: &str,
        environment: &Environment,
        language: &Lang,
        benchmark: &Benchmark,
        testcase: BenchmarkTests,
    ) -> bool {
        let data = self.data.read().unwrap();
        data.get(run_id)
            .and_then(|run_data| run_data.get(environment.name()))
            .and_then(|env_data| env_data.get(&language.name))
            .and_then(|lang_data| lang_data.get(&benchmark.name))
            .map(|bench_result| bench_result.test_cases.contains_key(&testcase.to_string()))
            .unwrap_or(false)
    }

    pub fn reload(&self) -> Result<()> {
        let (new_data, new_runs) = Self::load_all(&self.base_path)?;

        let mut data = self.data.write().unwrap();
        *data = new_data;

        let mut runs = self.runs.write().unwrap();
        *runs = new_runs;

        Ok(())
    }
}
