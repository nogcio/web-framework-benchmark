use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageRecord {
    pub name: String,
    pub url: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Language {
    pub name: String,
    pub url: String,
    pub color: String,
}

impl From<&LanguageRecord> for Language {
    fn from(record: &LanguageRecord) -> Self {
        Self {
            name: record.name.clone(),
            url: record.url.clone(),
            color: record.color.clone().unwrap_or_else(|| "#808080".to_string()),
        }
    }
}

pub fn parse_languages<P: AsRef<Path>>(path: P) -> Result<Vec<LanguageRecord>> {
    let content = fs::read_to_string(path)?;
    let languages: Vec<LanguageRecord> = serde_yaml::from_str(&content)?;
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
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();
        let path = temp_file.path();

        let languages = parse_languages(path).unwrap();
        assert_eq!(languages.len(), 1);
        let lang: Language = (&languages[0]).into();
        assert_eq!(lang.name, "Go");
        assert_eq!(lang.url, "https://golang.org");
    }
}
