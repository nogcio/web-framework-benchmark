use serde::Serialize;

#[derive(Serialize)]
pub struct RunView {
    pub id: String,
    pub created_at_fmt: String,
}

#[derive(Serialize)]
pub struct TestView {
    pub id: String,
    pub name: String,
    pub icon: String,
}

#[derive(Serialize)]
pub struct EnvironmentView {
    pub name: String,
    pub title: String,
    pub icon: String,
}
