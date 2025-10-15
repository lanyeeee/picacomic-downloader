use serde::{Deserialize, Serialize};
use specta::Type;

use crate::responses::ImageRespData;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct GetRankRespData {
    pub comics: Vec<ComicInRankRespData>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct ComicInRankRespData {
    #[serde(rename = "_id")]
    pub id: String,
    pub title: String,
    pub author: String,
    #[serde(rename = "totalViews")]
    pub total_views: i64,
    #[serde(rename = "totalLikes")]
    pub total_likes: i64,
    #[serde(rename = "pagesCount")]
    pub pages_count: i64,
    #[serde(rename = "epsCount")]
    pub eps_count: i64,
    pub finished: bool,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub thumb: ImageRespData,
    #[serde(rename = "leaderboardCount")]
    pub leaderboard_count: i64,
    #[serde(rename = "viewsCount")]
    pub views_count: i64,
}
