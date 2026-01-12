use diesel::prelude::*;
use chrono::NaiveDateTime;
use serde::Serialize;
use crate::schema::{users, posts};

#[derive(Queryable, Selectable, Identifiable, Debug)]
#[diesel(table_name = users)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub created_at: NaiveDateTime,
    pub last_login: Option<NaiveDateTime>,
    pub settings: serde_json::Value,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug)]
#[diesel(belongs_to(User))]
#[diesel(table_name = posts)]
pub struct Post {
    pub id: i32,
    pub user_id: i32,
    pub title: String,
    pub content: String,
    pub views: i32,
    pub created_at: NaiveDateTime,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    pub username: String,
    pub email: String,
    pub created_at: String,
    pub last_login: Option<String>,
    pub settings: serde_json::Value,
    pub posts: Vec<PostResponse>,
    pub trending: Vec<PostResponse>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostResponse {
    pub id: i32,
    pub title: String,
    pub content: String,
    pub views: i32,
    pub created_at: String,
}

impl From<Post> for PostResponse {
    fn from(post: Post) -> Self {
        Self {
            id: post.id,
            title: post.title,
            content: post.content,
            views: post.views,
            created_at: format!("{}Z", post.created_at),
        }
    }
}
