use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Language {
    pub name: String,
    pub url: String,
    pub frameworks: Vec<Framework>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Framework {
    pub name: String,
    pub path: String,
    pub url: String,
    pub tags: HashMap<String, String>,
}

pub fn parse_languages<P: AsRef<Path>>(path: P) -> Result<Vec<Language>> {
    let content = fs::read_to_string(path)?;
    let languages: Vec<Language> = serde_yaml::from_str(&content)?;
    Ok(languages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_languages() {
        let yaml_content = r#"
- name: Go
  url: https://golang.org
  frameworks:
    - name: stdlib
      path: benchmarks/go/std
      url: https://golang.org/pkg/net/http/
      tags:
        go: "1.21"
        platform: go
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();
        let path = temp_file.path();

        let languages = parse_languages(path).unwrap();
        assert_eq!(languages.len(), 1);
        let lang = &languages[0];
        assert_eq!(lang.name, "Go");
        assert_eq!(lang.url, "https://golang.org");
        assert_eq!(lang.frameworks.len(), 1);
        let fw = &lang.frameworks[0];
        assert_eq!(fw.name, "stdlib");
        assert_eq!(fw.path, "benchmarks/go/std");
        assert_eq!(fw.url, "https://golang.org/pkg/net/http/");
        let mut expected_tags = HashMap::new();
        expected_tags.insert("go".to_string(), "1.21".to_string());
        expected_tags.insert("platform".to_string(), "go".to_string());
        assert_eq!(fw.tags, expected_tags);
    }
}
