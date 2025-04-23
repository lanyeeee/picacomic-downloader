use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PicaResp {
    pub code: i64,
    pub error: Option<String>,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub detail: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRespData {
    pub token: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfileRespData {
    pub user: UserProfileDetailRespData,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UserProfileDetailRespData {
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
    pub avatar: ImageRespData,
    pub birthday: String,
    pub email: String,
    #[serde(rename = "created_at")]
    pub created_at: DateTime<Utc>,
    pub is_punched: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchRespData {
    pub comics: Pagination<ComicInSearchRespData>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ComicInSearchRespData {
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(default)]
    pub author: String,
    pub categories: Vec<String>,
    #[serde(default)]
    pub chinese_team: String,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(default)]
    pub description: String,
    pub finished: bool,
    #[serde(deserialize_with = "string_to_i64")]
    pub likes_count: i64,
    pub tags: Vec<String>,
    pub thumb: ImageRespData,
    pub title: String,
    #[serde(default, deserialize_with = "string_to_option_i64")]
    pub total_likes: Option<i64>,
    #[serde(default, deserialize_with = "string_to_option_i64")]
    pub total_views: Option<i64>,
    #[serde(rename = "updated_at")]
    pub updated_at: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetComicRespData {
    pub comic: ComicRespData,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ComicRespData {
    #[serde(rename = "_id")]
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub author: String,
    pub pages_count: i64,
    pub eps_count: i64,
    pub finished: bool,
    pub categories: Vec<String>,
    pub thumb: ImageRespData,
    #[serde(deserialize_with = "string_to_i64")]
    pub likes_count: i64,
    #[serde(rename = "_creator")]
    pub creator: CreatorRespData,
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

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetChapterRespData {
    pub eps: Pagination<ChapterRespData>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChapterRespData {
    #[serde(rename = "_id")]
    pub id: String,
    pub title: String,
    pub order: i64,
    #[serde(rename = "updated_at")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetChapterImageRespData {
    pub pages: Pagination<ChapterImageRespData>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChapterImageRespData {
    #[serde(rename = "_id")]
    pub id: String,
    pub media: ImageRespData,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFavoriteRespData {
    pub comics: Pagination<ComicInFavoriteRespData>,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ComicInFavoriteRespData {
    #[serde(rename = "_id")]
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub author: String,
    pub pages_count: i64,
    pub eps_count: i64,
    pub finished: bool,
    pub categories: Vec<String>,
    pub thumb: ImageRespData,
    #[serde(deserialize_with = "string_to_i64")]
    pub likes_count: i64,
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
pub struct ImageRespData {
    pub original_name: String,
    pub path: String,
    pub file_server: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreatorRespData {
    #[serde(rename = "_id")]
    pub id: String,
    pub gender: String,
    pub name: String,
    pub title: String,
    pub verified: Option<bool>,
    pub exp: i64,
    pub level: i64,
    pub characters: Vec<String>,
    #[serde(default)]
    pub avatar: ImageRespData,
    #[serde(default)]
    pub slogan: String,
    pub role: String,
    #[serde(default)]
    pub character: String,
}

fn string_to_i64<'de, D>(d: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde_json::Value;
    let value: Value = serde::Deserialize::deserialize(d)?;

    match value {
        #[allow(clippy::cast_possible_truncation)]
        Value::Number(n) => Ok(n.as_i64().unwrap_or(0)),
        Value::String(s) => Ok(s.parse().unwrap_or(0)),
        _ => Err(serde::de::Error::custom(
            "`string_to_i64` 失败，value类型不是 `Number` 或 `String`",
        )),
    }
}

fn string_to_option_i64<'de, D>(d: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde_json::Value;
    let value: Value = serde::Deserialize::deserialize(d)?;

    match value {
        #[allow(clippy::cast_possible_truncation)]
        Value::Number(n) => Ok(Some(n.as_i64().unwrap_or(0))),
        Value::String(s) => Ok(Some(s.parse().unwrap_or(0))),
        _ => Err(serde::de::Error::custom(
            "`string_to_option_i64` 失败，value类型不是 `Number` 或 `String`",
        )),
    }
}
