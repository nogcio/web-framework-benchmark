use std::{path::PathBuf, sync::Arc};

use serde::Deserialize;
use walkdir::WalkDir;

use crate::{Benchmark, Environment, EnvironmentSecrets, Error, Framework, Lang, Result};

#[derive(Debug, Clone)]
pub struct Config {
    inner: Arc<ConfigInner>,
}

#[derive(Debug)]
struct ConfigInner {
    langs: Vec<Lang>,
    frameworks: Vec<Framework>,
    benchmarks: Vec<Benchmark>,
    environments: Vec<Environment>,
}

struct ConfigAccumulator {
    langs: Vec<Lang>,
    frameworks: Vec<Framework>,
    benchmarks: Vec<Benchmark>,
    environments: Vec<Environment>,
    secrets: Vec<EnvironmentSecrets>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ConfigFile {
    Language(Box<Lang>),
    Framework(Box<Framework>),
    Benchmark(Box<Benchmark>),
    Environment(Box<Environment>),
    EnvironmentSecrets(Box<EnvironmentSecrets>),
}

impl Config {
    pub fn load(dir: &PathBuf) -> Result<Self> {
        WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "yaml" || ext == "yml" || ext == "json" || ext == "jsonl")
                    .unwrap_or(false)
            })
            .try_fold(
                ConfigAccumulator {
                    langs: Vec::new(),
                    frameworks: Vec::new(),
                    benchmarks: Vec::new(),
                    environments: Vec::new(),
                    secrets: Vec::new(),
                },
                |mut acc, entry| -> Result<_> {
                    let path = entry.path();
                    let content = std::fs::read_to_string(path)?;
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

                    let items: Vec<ConfigFile> = match ext {
                        "json" => {
                            let value: serde_json::Value = serde_json::from_str(&content)?;
                            if value.is_array() {
                                serde_json::from_value(value)?
                            } else {
                                vec![serde_json::from_value(value)?]
                            }
                        }
                        "jsonl" => content
                            .lines()
                            .filter(|line| !line.trim().is_empty())
                            .map(|line| serde_json::from_str(line).map_err(Error::Serialize))
                            .collect::<Result<Vec<_>>>()?,
                        "yaml" | "yml" => {
                            let mut items = Vec::new();
                            for doc in serde_yaml::Deserializer::from_str(&content) {
                                let value = serde_yaml::Value::deserialize(doc)?;
                                if let Ok(list) =
                                    serde_yaml::from_value::<Vec<ConfigFile>>(value.clone())
                                {
                                    items.extend(list);
                                } else {
                                    items.push(serde_yaml::from_value(value)?);
                                }
                            }
                            items
                        }
                        _ => Vec::new(),
                    };

                    for config_file in items {
                        match config_file {
                            ConfigFile::Language(lang) => acc.langs.push(*lang),
                            ConfigFile::Framework(framework) => acc.frameworks.push(*framework),
                            ConfigFile::Benchmark(benchmark) => acc.benchmarks.push(*benchmark),
                            ConfigFile::Environment(environment) => {
                                acc.environments.push(*environment)
                            }
                            ConfigFile::EnvironmentSecrets(secret) => acc.secrets.push(*secret),
                        }
                    }

                    Ok(acc)
                },
            )
            .map(|mut acc| {
                for env in &mut acc.environments {
                    if let Environment::Ssh(ssh_env) = env
                        && let Some(secret) = acc.secrets.iter().find(|s| s.name == ssh_env.name)
                    {
                        ssh_env.merge_secrets(secret.clone());
                    }
                }

                Config {
                    inner: Arc::new(ConfigInner {
                        langs: acc.langs,
                        frameworks: acc.frameworks,
                        benchmarks: acc.benchmarks,
                        environments: acc.environments,
                    }),
                }
            })
    }

    pub fn get_benchmarks(&self) -> Vec<&Benchmark> {
        let has_only = self.inner.benchmarks.iter().any(|b| b.only);
        if has_only {
            self.inner
                .benchmarks
                .iter()
                .filter(|b| b.only)
                .collect::<Vec<_>>()
        } else {
            self.inner
                .benchmarks
                .iter()
                .filter(|b| !b.disabled)
                .collect::<Vec<_>>()
        }
    }

    pub fn get_environments(&self) -> Vec<&Environment> {
        self.inner.environments.iter().collect()
    }

    pub fn get_environment(&self, name: &str) -> Option<&Environment> {
        self.inner
            .environments
            .iter()
            .find(|env| env.name() == name)
    }

    pub fn languages(&self) -> &[Lang] {
        &self.inner.langs
    }

    pub fn frameworks(&self) -> &[Framework] {
        &self.inner.frameworks
    }

    pub fn benchmarks(&self) -> &[Benchmark] {
        &self.inner.benchmarks
    }

    pub fn environments(&self) -> &[Environment] {
        &self.inner.environments
    }

    pub fn get_lang(&self, name: &str) -> Option<&Lang> {
        self.inner.langs.iter().find(|l| l.name == name)
    }

    pub fn reload(&mut self, dir: &PathBuf) -> Result<()> {
        let new_config = Self::load(dir)?;
        self.inner = new_config.inner;
        Ok(())
    }
}
