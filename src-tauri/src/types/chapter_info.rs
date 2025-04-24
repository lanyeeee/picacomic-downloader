use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChapterInfo {
    pub chapter_id: String,
    pub chapter_title: String,
    pub comic_id: String,
    pub comic_title: String,
    pub author: String,
    pub is_downloaded: bool,
    pub order: i64,
}
