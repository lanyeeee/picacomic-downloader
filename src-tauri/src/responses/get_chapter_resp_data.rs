use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::Pagination;

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
