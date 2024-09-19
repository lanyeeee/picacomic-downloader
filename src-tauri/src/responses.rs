use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PicaResponse {
    pub code: i64,
    pub error: Option<String>,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub detail: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponseData {
    pub token: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfileResponseData {
    pub user: UserProfile,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    #[serde(rename = "_id")]
    pub id: String,
    pub gender: String,
    pub name: String,
    pub title: String,
    pub verified: bool,
    pub exp: i64,
    pub level: i64,
    pub characters: Vec<String>,
    #[serde(default)]
    pub avatar: Image,
    pub birthday: String,
    pub email: String,
    #[serde(rename = "created_at")]
    pub created_at: DateTime<Utc>,
    pub is_punched: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComicSearchResponseData {
    pub comics: Pagination<ComicInSearch>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ComicInSearch {
    #[serde(rename = "_id")]
    pub id: String,
    pub author: String,
    pub categories: Vec<String>,
    #[serde(default)]
    pub chinese_team: String,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(default)]
    pub description: String,
    pub finished: bool,
    pub likes_count: i64,
    pub tags: Vec<String>,
    pub thumb: Image,
    pub title: String,
    pub total_likes: Option<i64>,
    pub total_views: Option<i64>,
    #[serde(rename = "updated_at")]
    pub updated_at: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComicResponseData {
    pub comic: Comic,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Comic {
    #[serde(rename = "_id")]
    pub id: String,
    pub title: String,
    pub author: String,
    pub pages_count: i64,
    pub eps_count: i64,
    pub finished: bool,
    pub categories: Vec<String>,
    pub thumb: Image,
    pub likes_count: i64,
    #[serde(rename = "_creator")]
    pub creator: Creator,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub chinese_team: String,
    pub tags: Vec<String>,
    #[serde(rename = "updated_at")]
    pub updated_at: DateTime<Utc>,
    #[serde(rename = "created_at")]
    pub created_at: String,
    pub allow_download: bool,
    pub views_count: i64,
    pub is_liked: bool,
    pub comments_count: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Pagination<T> {
    pub total: i64,
    pub limit: i64,
    pub page: i64,
    pub pages: i64,
    pub docs: Vec<T>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    pub original_name: String,
    pub path: String,
    pub file_server: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Creator {
    #[serde(rename = "_id")]
    pub id: String,
    pub gender: String,
    pub name: String,
    pub title: String,
    pub verified: bool,
    pub exp: i64,
    pub level: i64,
    pub characters: Vec<String>,
    #[serde(default)]
    pub avatar: Image,
    #[serde(default)]
    pub slogan: String,
    pub role: String,
    #[serde(default)]
    pub character: String,
}
