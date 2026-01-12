#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Framework {
    pub name: String,
    pub language: String,
    pub url: String,
}
