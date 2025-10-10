use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};
use walkdir::WalkDir;

use crate::{
    config::Config,
    extensions::WalkDirEntryExt,
    responses::{ChapterRespData, ComicRespData},
    utils,
};

use super::ChapterInfo;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_field_names)]
pub struct Comic {
    pub id: String,
    pub title: String,
    pub author: String,
    pub pages_count: i64,
    pub chapter_infos: Vec<ChapterInfo>,
    pub chapter_count: i64,
    pub finished: bool,
    pub categories: Vec<String>,
    pub thumb: Image,
    pub likes_count: i64,
    pub creator: Creator,
    pub description: String,
    pub chinese_team: String,
    pub tags: Vec<String>,
    pub updated_at: DateTime<Utc>,
    pub created_at: String,
    pub allow_download: bool,
    pub views_count: i64,
    pub is_liked: bool,
    pub comments_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_downloaded: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comic_download_dir: Option<PathBuf>,
}

impl Comic {
    // TODO: 改名为`from_resp_data`
    pub fn from(
        app: &AppHandle,
        comic: ComicRespData,
        chapters: Vec<ChapterRespData>,
    ) -> anyhow::Result<Comic> {
        let chapter_infos = chapters
            .into_iter()
            .map(|chapter| ChapterInfo {
                chapter_id: chapter.id,
                chapter_title: chapter.title,
                order: chapter.order,
                is_downloaded: None,
                chapter_download_dir: None,
            })
            .collect();

        let thumb = Image {
            original_name: comic.thumb.original_name,
            path: comic.thumb.path,
            file_server: comic.thumb.file_server,
        };

        let creator = Creator {
            id: comic.creator.id,
            gender: comic.creator.gender,
            name: comic.creator.name,
            title: comic.creator.title,
            verified: comic.creator.verified,
            exp: comic.creator.exp,
            level: comic.creator.level,
            characters: comic.creator.characters,
            avatar: Image {
                original_name: comic.creator.avatar.original_name,
                path: comic.creator.avatar.path,
                file_server: comic.creator.avatar.file_server,
            },
            slogan: comic.creator.slogan,
            role: comic.creator.role,
            character: comic.creator.character,
        };

        let mut comic = Self {
            id: comic.id,
            title: comic.title,
            author: comic.author,
            pages_count: comic.pages_count,
            chapter_infos,
            chapter_count: comic.eps_count,
            finished: comic.finished,
            categories: comic.categories,
            thumb,
            likes_count: comic.likes_count,
            creator,
            description: comic.description,
            chinese_team: comic.chinese_team,
            tags: comic.tags,
            updated_at: comic.updated_at,
            created_at: comic.created_at,
            allow_download: comic.allow_download,
            views_count: comic.views_count,
            is_liked: comic.is_liked,
            comments_count: comic.comments_count,
            is_downloaded: None,
            comic_download_dir: None,
        };

        let id_to_dir_map =
            utils::create_id_to_dir_map(app).context("创建漫画ID到下载目录映射失败")?;

        comic
            .update_fields(&id_to_dir_map)
            .context(format!("`{}`更新Comic的字段失败", comic.title))?;

        Ok(comic)
    }

    pub fn update_fields(
        &mut self,
        id_to_dir_map: &HashMap<String, PathBuf>,
    ) -> anyhow::Result<()> {
        if let Some(download_dir) = id_to_dir_map.get(&self.id) {
            self.comic_download_dir = Some(download_dir.clone());
            self.is_downloaded = Some(true);

            self.update_chapter_infos_fields()
                .context("更新章节信息字段失败")?;
        }

        Ok(())
    }

    pub fn from_metadata(metadata_path: &Path) -> anyhow::Result<Comic> {
        let comic_json = std::fs::read_to_string(metadata_path)
            .context(format!("读取`{metadata_path:?}`失败"))?;
        let mut comic = serde_json::from_str::<Comic>(&comic_json)
            .context(format!("将`{metadata_path:?}`反序列化为Comic失败"))?;
        // 来自元数据的章节信息没有`comic_download_dir`和`is_downloaded`字段，需要更新
        let parent = metadata_path
            .parent()
            .context(format!("`{}`没有父目录", metadata_path.display()))?;
        comic.comic_download_dir = Some(parent.to_path_buf());
        comic.is_downloaded = Some(true);

        comic
            .update_chapter_infos_fields()
            .context("更新章节信息字段失败")?;

        Ok(comic)
    }

    pub fn get_comic_download_dir_name(&self) -> anyhow::Result<String> {
        let comic_download_dir = self
            .comic_download_dir
            .as_ref()
            .context("`comic_download_dir`字段为`None`")?;

        let comic_download_dir_name = comic_download_dir
            .file_name()
            .context(format!("获取`{comic_download_dir:?}`的目录名失败"))?
            .to_string_lossy()
            .to_string();

        Ok(comic_download_dir_name)
    }

    pub fn get_comic_export_dir(&self, app: &AppHandle) -> anyhow::Result<PathBuf> {
        let (download_dir, export_dir) = {
            let config = app.state::<RwLock<Config>>();
            let config = config.read();
            (config.download_dir.clone(), config.export_dir.clone())
        };

        let Some(comic_download_dir) = self.comic_download_dir.clone() else {
            return Err(anyhow!("`comic_download_dir`字段为`None`"));
        };

        let relative_dir = comic_download_dir
            .strip_prefix(&download_dir)
            .context(format!(
                "无法从路径`{comic_download_dir:?}`中移除前缀`{download_dir:?}`"
            ))?;

        let comic_export_dir = export_dir.join(relative_dir);
        Ok(comic_export_dir)
    }

    fn update_chapter_infos_fields(&mut self) -> anyhow::Result<()> {
        let Some(comic_download_dir) = &self.comic_download_dir else {
            return Err(anyhow!("`comic_download_dir`字段为`None`"));
        };

        if !comic_download_dir.exists() {
            return Ok(());
        }

        for entry in WalkDir::new(comic_download_dir)
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.is_chapter_metadata() {
                continue;
            }

            let metadata_path = entry.path();

            let metadata_str = std::fs::read_to_string(metadata_path)
                .context(format!("读取`{}`失败", metadata_path.display()))?;

            let chapter_json: serde_json::Value =
                serde_json::from_str(&metadata_str).context(format!(
                    "将`{}`反序列化为serde_json::Value失败",
                    metadata_path.display()
                ))?;

            let chapter_id = chapter_json
                .get("chapterId")
                .and_then(|id| id.as_str())
                .context(format!("`{}`没有`chapterId`字段", metadata_path.display()))?
                .to_string();

            if let Some(chapter_info) = self
                .chapter_infos
                .iter_mut()
                .find(|chapter| chapter.chapter_id == chapter_id)
            {
                let parent = metadata_path
                    .parent()
                    .context(format!("`{}`没有父目录", metadata_path.display()))?;
                chapter_info.chapter_download_dir = Some(parent.to_path_buf());
                chapter_info.is_downloaded = Some(true);
            }
        }
        Ok(())
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Creator {
    pub id: String,
    pub gender: String,
    pub name: String,
    pub title: String,
    pub verified: Option<bool>,
    pub exp: i64,
    pub level: i64,
    pub characters: Vec<String>,
    pub avatar: Image,
    pub slogan: String,
    pub role: String,
    pub character: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    pub original_name: String,
    pub path: String,
    pub file_server: String,
}
