use std::path::{Path, PathBuf};

use anyhow::Context;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};

use crate::{
    config::Config,
    responses::{ChapterRespData, ComicRespData},
    utils::filename_filter,
};

use super::ChapterInfo;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
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
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub comic_dir_name: String,
}

impl Comic {
    pub fn from(app: &AppHandle, comic: ComicRespData, chapters: Vec<ChapterRespData>) -> Comic {
        let is_downloaded =
            Comic::get_comic_download_dir(app, &comic.title, &comic.author).exists();

        let chapter_infos: Vec<ChapterInfo> = chapters
            .into_iter()
            .map(|chapter_resp_data| {
                let is_downloaded = ChapterInfo::get_is_downloaded(
                    app,
                    &comic.title,
                    &chapter_resp_data.title,
                    &comic.author,
                    chapter_resp_data.order,
                );
                ChapterInfo {
                    chapter_id: chapter_resp_data.id,
                    chapter_title: chapter_resp_data.title,
                    order: chapter_resp_data.order,
                    is_downloaded: Some(is_downloaded),
                    chapter_dir_name: String::new(),
                }
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

        Self {
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
            is_downloaded: Some(is_downloaded),
            comic_dir_name: String::new(),
        }
    }

    pub fn from_metadata(app: &AppHandle, metadata_path: &Path) -> anyhow::Result<Comic> {
        let comic_json = std::fs::read_to_string(metadata_path).context(format!(
            "从元数据转为Comic失败，读取元数据文件 {metadata_path:?} 失败"
        ))?;
        let mut comic = serde_json::from_str::<Comic>(&comic_json).context(format!(
            "从元数据转为Comic失败，将 {metadata_path:?} 反序列化为Comic失败"
        ))?;
        // 来自metadata的Comic的is_downloaded字段都是None，需要重新计算
        let comic_is_downloaded = Comic::get_is_downloaded(app, &comic.title, &comic.author);
        comic.is_downloaded = Some(comic_is_downloaded);
        // 来自metadata的ChapterInfo的is_downloaded字段都是None，需要重新计算
        for chapter_info in &mut comic.chapter_infos {
            let chapter_is_downloaded = ChapterInfo::get_is_downloaded(
                app,
                &comic.title,
                &chapter_info.chapter_title,
                &comic.author,
                chapter_info.order,
            );
            chapter_info.is_downloaded = Some(chapter_is_downloaded);
        }
        Ok(comic)
    }

    pub fn get_comic_download_dir(app: &AppHandle, comic_title: &str, author: &str) -> PathBuf {
        let comic_dir_name = Self::comic_dir_name(app, comic_title, author);
        app.state::<RwLock<Config>>()
            .read()
            .download_dir
            .join(comic_dir_name)
    }

    pub fn get_comic_export_dir(app: &AppHandle, comic_title: &str, author: &str) -> PathBuf {
        let comic_dir_name = Self::comic_dir_name(app, comic_title, author);
        app.state::<RwLock<Config>>()
            .read()
            .export_dir
            .join(comic_dir_name)
    }

    pub fn get_is_downloaded(app: &AppHandle, comic_title: &str, author: &str) -> bool {
        let comic_download_dir = Self::get_comic_download_dir(app, comic_title, author);
        comic_download_dir.exists()
    }

    fn comic_dir_name(app: &AppHandle, comic_title: &str, author: &str) -> String {
        let author = filename_filter(author);
        let comic_title = filename_filter(comic_title);

        let download_with_author = app.state::<RwLock<Config>>().read().download_with_author;
        if download_with_author {
            format!("[{author}] {comic_title}")
        } else {
            comic_title
        }
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
