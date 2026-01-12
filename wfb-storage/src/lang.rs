#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Lang {
    pub name: String,
    pub url: String,
    pub color: String,
}
