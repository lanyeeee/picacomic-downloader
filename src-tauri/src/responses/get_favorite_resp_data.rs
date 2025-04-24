use serde::{Deserialize, Serialize};
use specta::Type;

use super::{string_to_i64, ImageRespData, Pagination};

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
