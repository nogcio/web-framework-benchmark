use crate::benchmark_environment::config::EnvironmentFile;
use crate::db::{benchmarks, frameworks, languages, runs};
use clap::ValueEnum;
use serde::Serialize;

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum AnalysisLanguage {
    En,
    Ru,
}

impl AnalysisLanguage {
    pub fn as_str(&self) -> &'static str {
        match self {
            AnalysisLanguage::En => "en",
            AnalysisLanguage::Ru => "ru",
        }
    }

    pub fn prompt_instruction(&self) -> &'static str {
        match self {
            AnalysisLanguage::En => "The output must be in English",
            AnalysisLanguage::Ru => "The output must be in Russian",
        }
    }
}

#[derive(Serialize)]
pub struct AnalysisContext<'a> {
    pub framework: &'a str,
    pub language: &'a str,
    pub test: &'a str,
    pub test_description: &'a str,
    pub framework_info: Option<&'a frameworks::FrameworkRecord>,
    pub language_info: Option<&'a languages::LanguageRecord>,
    pub benchmark_config: Option<&'a benchmarks::BenchmarkRecord>,
    pub environment: Option<&'a EnvironmentFile>,
    pub results: &'a runs::RunDataRecord,
}
