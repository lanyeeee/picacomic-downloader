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
    responses::{ComicInSearchRespData, ImageRespData, Pagination, SearchRespData},
    utils,
};

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
    pub fn from_resp_data(
        app: &AppHandle,
        resp_data: SearchRespData,
    ) -> anyhow::Result<SearchResult> {
        let id_to_dir_map =
            utils::create_id_to_dir_map(app).context("创建漫画ID到下载目录映射失败")?;

        let mut docs = Vec::new();
        for comic in resp_data.comics.docs {
            let comic = ComicInSearch::from_resp_data(comic, &id_to_dir_map);
            docs.push(comic);
        }

        let pagination = Pagination {
            total: resp_data.comics.total,
            limit: resp_data.comics.limit,
            page: resp_data.comics.page,
            pages: resp_data.comics.pages,
            docs,
        };

        let result = SearchResult(pagination);

        Ok(result)
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
    pub comic_download_dir: PathBuf,
}

impl ComicInSearch {
    pub fn from_resp_data(
        resp_data: ComicInSearchRespData,
        id_to_dir_map: &HashMap<String, PathBuf>,
    ) -> ComicInSearch {
        let mut comic = ComicInSearch {
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
