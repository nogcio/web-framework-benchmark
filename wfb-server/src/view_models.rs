use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct RunView {
    pub id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Clone)]
pub struct TestView {
    pub id: String,
    pub name: String,
    pub icon: String,
}

#[derive(Serialize, Clone)]
pub struct EnvironmentView {
    pub name: String,
    pub title: String,
    pub icon: String,
    pub spec: Option<String>,
}
