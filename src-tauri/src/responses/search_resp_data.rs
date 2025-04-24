use serde::{Deserialize, Serialize};
use specta::Type;

use super::{string_to_i64, string_to_option_i64, ImageRespData, Pagination};

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
