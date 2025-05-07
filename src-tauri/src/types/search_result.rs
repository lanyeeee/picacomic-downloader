use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;

use crate::responses::{ComicInSearchRespData, ImageRespData, Pagination, SearchRespData};

use super::Comic;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult(pub Pagination<ComicInSearch>);

impl Deref for SearchResult {
    type Target = Pagination<ComicInSearch>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SearchResult {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl SearchResult {
    pub fn from_resp_data(app: &AppHandle, resp_data: SearchRespData) -> SearchResult {
        let pagination = Pagination {
            total: resp_data.comics.total,
            limit: resp_data.comics.limit,
            page: resp_data.comics.page,
            pages: resp_data.comics.pages,
            docs: resp_data
                .comics
                .docs
                .into_iter()
                .map(|comic| ComicInSearch::from_resp_data(app, comic))
                .collect(),
        };

        SearchResult(pagination)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ComicInSearch {
    pub id: String,
    pub author: String,
    pub categories: Vec<String>,
    pub chinese_team: String,
    pub created_at: String,
    pub description: String,
    pub finished: bool,
    pub likes_count: i64,
    pub tags: Vec<String>,
    pub thumb: ImageRespData,
    pub title: String,
    pub total_likes: Option<i64>,
    pub total_views: Option<i64>,
    pub updated_at: String,
    pub is_downloaded: bool,
}

impl ComicInSearch {
    pub fn from_resp_data(app: &AppHandle, resp_data: ComicInSearchRespData) -> ComicInSearch {
        let is_downloaded = Comic::get_is_downloaded(app, &resp_data.title, &resp_data.author);

        ComicInSearch {
            id: resp_data.id,
            author: resp_data.author,
            categories: resp_data.categories,
            chinese_team: resp_data.chinese_team,
            created_at: resp_data.created_at,
            description: resp_data.description,
            finished: resp_data.finished,
            likes_count: resp_data.likes_count,
            tags: resp_data.tags,
            thumb: resp_data.thumb,
            title: resp_data.title,
            total_likes: resp_data.total_likes,
            total_views: resp_data.total_views,
            updated_at: resp_data.updated_at,
            is_downloaded,
        }
    }
}
