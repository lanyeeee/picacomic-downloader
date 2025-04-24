use serde::{Deserialize, Serialize};
use specta::Type;

use super::{ImageRespData, Pagination};

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
