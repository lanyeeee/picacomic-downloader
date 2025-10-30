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
    responses::{ComicInRankRespData, GetRankRespData, ImageRespData},
    utils,
};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct GetRankResult(pub Vec<ComicInRank>);

impl Deref for GetRankResult {
    type Target = Vec<ComicInRank>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for GetRankResult {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl GetRankResult {
    pub fn from_resp_data(
        app: &AppHandle,
        resp_data: GetRankRespData,
    ) -> anyhow::Result<GetRankResult> {
        let id_to_dir_map =
            utils::create_id_to_dir_map(app).context("创建漫画ID到下载目录映射失败")?;

        let comics = resp_data
            .comics
            .into_iter()
            .map(|comic| ComicInRank::from_resp_data(comic, &id_to_dir_map))
            .collect();

        Ok(GetRankResult(comics))
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct ComicInRank {
    pub id: String,
    pub title: String,
    pub author: String,
    pub total_views: i64,
    pub total_likes: i64,
    pub pages_count: i64,
    pub eps_count: i64,
    pub finished: bool,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub thumb: ImageRespData,
    pub leaderboard_count: i64,
    pub views_count: i64,
    pub is_downloaded: bool,
    pub comic_download_dir: PathBuf,
}

impl ComicInRank {
    pub fn from_resp_data(
        resp_data: ComicInRankRespData,
        id_to_dir_map: &HashMap<String, PathBuf>,
    ) -> ComicInRank {
        let mut comic = ComicInRank {
            id: resp_data.id,
            title: resp_data.title,
            author: resp_data.author,
            total_views: resp_data.total_views,
            total_likes: resp_data.total_likes,
            pages_count: resp_data.pages_count,
            eps_count: resp_data.eps_count,
            finished: resp_data.finished,
            categories: resp_data.categories,
            tags: resp_data.tags,
            thumb: resp_data.thumb,
            leaderboard_count: resp_data.leaderboard_count,
            views_count: resp_data.views_count,
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
