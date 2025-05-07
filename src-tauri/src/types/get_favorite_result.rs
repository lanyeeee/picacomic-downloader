use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;

use crate::responses::{ComicInFavoriteRespData, GetFavoriteRespData, ImageRespData, Pagination};

use super::Comic;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GetFavoriteResult(pub Pagination<ComicInFavorite>);

impl Deref for GetFavoriteResult {
    type Target = Pagination<ComicInFavorite>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for GetFavoriteResult {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl GetFavoriteResult {
    pub fn from_resp_data(app: &AppHandle, resp_data: GetFavoriteRespData) -> GetFavoriteResult {
        let pagination = Pagination {
            total: resp_data.comics.total,
            limit: resp_data.comics.limit,
            page: resp_data.comics.page,
            pages: resp_data.comics.pages,
            docs: resp_data
                .comics
                .docs
                .into_iter()
                .map(|comic| ComicInFavorite::from_resp_data(app, comic))
                .collect(),
        };

        GetFavoriteResult(pagination)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ComicInFavorite {
    pub id: String,
    pub title: String,
    pub author: String,
    pub pages_count: i64,
    pub eps_count: i64,
    pub finished: bool,
    pub categories: Vec<String>,
    pub thumb: ImageRespData,
    pub likes_count: i64,
    pub is_downloaded: bool,
}

impl ComicInFavorite {
    pub fn from_resp_data(app: &AppHandle, resp_data: ComicInFavoriteRespData) -> ComicInFavorite {
        let is_downloaded = Comic::get_is_downloaded(app, &resp_data.title, &resp_data.author);

        ComicInFavorite {
            id: resp_data.id,
            title: resp_data.title,
            author: resp_data.author,
            pages_count: resp_data.pages_count,
            eps_count: resp_data.eps_count,
            finished: resp_data.finished,
            categories: resp_data.categories,
            thumb: resp_data.thumb,
            likes_count: resp_data.likes_count,
            is_downloaded,
        }
    }
}
