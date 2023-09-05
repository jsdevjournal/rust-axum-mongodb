use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Serialize)]
pub struct GenericResponse {
    pub status: String,
    pub message: String,
}

#[allow(non_snake_case)]
#[derive(Serialize, Debug)]
pub struct PostResponse {
    pub id: String,
    pub title: String,
    pub body: String,
    pub author: String,
    pub published: bool,
    pub createdAt: DateTime<Utc>,
    pub updatedAt: DateTime<Utc>,
}

#[derive(Serialize, Debug)]
pub struct PostData {
    pub post: PostResponse,
}

#[derive(Serialize, Debug)]
pub struct SinglePostResponse {
    pub status: &'static str,
    pub data: PostData,
}

#[derive(Serialize, Debug)]
pub struct PostListResponse {
    pub status: &'static str,
    pub results: usize,
    pub posts: Vec<PostResponse>,
}
