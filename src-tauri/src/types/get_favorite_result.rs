use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;

use crate::{
    responses::{ComicInFavoriteRespData, GetFavoriteRespData, ImageRespData, Pagination},
    utils,
};

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
    pub fn from_resp_data(
        app: &AppHandle,
        resp_data: GetFavoriteRespData,
    ) -> anyhow::Result<GetFavoriteResult> {
        let id_to_dir_map =
            utils::create_id_to_dir_map(app).context("创建漫画ID到下载目录映射失败")?;

        let mut docs = Vec::new();
        for comic in resp_data.comics.docs {
            let comic = ComicInFavorite::from_resp_data(comic, &id_to_dir_map);
            docs.push(comic);
        }

        let pagination = Pagination {
            total: resp_data.comics.total,
            limit: resp_data.comics.limit,
            page: resp_data.comics.page,
            pages: resp_data.comics.pages,
            docs,
        };

        let result = GetFavoriteResult(pagination);

        Ok(result)
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
    pub comic_download_dir: PathBuf,
}

impl ComicInFavorite {
    pub fn from_resp_data(
        resp_data: ComicInFavoriteRespData,
        id_to_dir_map: &HashMap<String, PathBuf>,
    ) -> ComicInFavorite {
        let mut comic = ComicInFavorite {
            id: resp_data.id,
            title: resp_data.title,
            author: resp_data.author,
            pages_count: resp_data.pages_count,
            eps_count: resp_data.eps_count,
            finished: resp_data.finished,
            categories: resp_data.categories,
            thumb: resp_data.thumb,
            likes_count: resp_data.likes_count,
            is_downloaded: false,
            comic_download_dir: PathBuf::new(),
        };

        comic.update_fields(id_to_dir_map);

        comic
    }

    pub fn update_fields(&mut self, id_to_dir_map: &HashMap<String, PathBuf>) {
        if let Some(comic_download_dir) = id_to_dir_map.get(&self.id) {
            self.comic_download_dir = comic_download_dir.clone();
            self.is_downloaded = true;
        }
    }
}
