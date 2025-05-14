use std::ops::{Deref, DerefMut};

use anyhow::Context;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};

use crate::{
    config::Config,
    responses::{ComicInSearchRespData, ImageRespData, Pagination, SearchRespData},
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
        let mut docs = Vec::new();
        for comic in resp_data.comics.docs {
            let comic = ComicInSearch::from_resp_data(app, comic)
                .context("从RespData转为ComicInSearch失败")?;
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
    pub comic_dir_name: String,
}

impl ComicInSearch {
    pub fn from_resp_data(
        app: &AppHandle,
        resp_data: ComicInSearchRespData,
    ) -> anyhow::Result<ComicInSearch> {
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
            comic_dir_name: String::new(),
        };

        comic.update_fields(app).context("更新字段失败")?;

        Ok(comic)
    }

    /// 根据下载目录中的元数据文件更新字段
    ///
    /// 修改字段及逻辑：
    /// - `comic_dir_name`: 通过匹配当前漫画id，更新为元数据文件所在目录名
    /// - `is_downloaded`: 若找到对应漫画元数据，设为 true
    ///
    /// 仅当元数据文件存在且id匹配时才会更新字段
    pub fn update_fields(&mut self, app: &AppHandle) -> anyhow::Result<()> {
        let download_dir = app.state::<RwLock<Config>>().read().download_dir.clone();
        if !download_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(&download_dir)
            .context(format!("读取下载目录`{download_dir:?}`失败"))?
            .filter_map(Result::ok)
        {
            let metadata_path = entry.path().join("元数据.json");
            if !metadata_path.exists() {
                continue;
            }

            let metadata_str = std::fs::read_to_string(&metadata_path)
                .context(format!("读取`{metadata_path:?}`失败"))?;

            let comic_json: serde_json::Value = serde_json::from_str(&metadata_str).context(
                format!("将`{metadata_path:?}`反序列化为serde_json::Value失败"),
            )?;

            let id = comic_json
                .get("id")
                .and_then(|id| id.as_str())
                .context(format!("`{metadata_path:?}`没有`id`字段"))?
                .to_string();

            if id != self.id {
                continue;
            }

            self.comic_dir_name = entry.file_name().to_string_lossy().to_string();
            self.is_downloaded = true;
            break;
        }

        Ok(())
    }
}
