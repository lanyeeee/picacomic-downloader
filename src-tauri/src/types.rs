use crate::config::Config;
use crate::extensions::IgnoreRwLockPoison;
use crate::responses::{ChapterRespData, ComicRespData};
use crate::utils::filename_filter;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::RwLock;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub enum Sort {
    Default,
    TimeNewest,
    TimeOldest,
    LikeMost,
    ViewMost,
}

impl Sort {
    pub fn as_str(&self) -> &'static str {
        match self {
            Sort::Default => "ua",
            Sort::TimeNewest => "dd",
            Sort::TimeOldest => "da",
            Sort::LikeMost => "ld",
            Sort::ViewMost => "vd",
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Comic {
    #[serde(rename = "_id")]
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub author: String,
    pub pages_count: i64,
    pub chapters: Vec<ChapterInfo>,
    pub chapter_count: i64,
    pub finished: bool,
    pub categories: Vec<String>,
    pub thumb: Image,
    pub likes_count: i64,
    #[serde(rename = "_creator")]
    pub creator: Creator,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub chinese_team: String,
    pub tags: Vec<String>,
    #[serde(rename = "updated_at")]
    pub updated_at: DateTime<Utc>,
    #[serde(rename = "created_at")]
    pub created_at: String,
    pub allow_download: bool,
    pub views_count: i64,
    pub is_liked: bool,
    pub comments_count: i64,
}
impl Comic {
    pub fn from(app: &AppHandle, comic: ComicRespData, chapters: Vec<ChapterRespData>) -> Self {
        let comic_title = filename_filter(&comic.title);
        let author = filename_filter(&comic.author);

        let chapters: Vec<ChapterInfo> = chapters
            .into_iter()
            .map(|chapter| {
                let chapter_title = filename_filter(&chapter.title);
                let is_downloaded =
                    Self::get_is_downloaded(app, &comic_title, &chapter_title, &author);
                ChapterInfo {
                    chapter_id: chapter.id,
                    chapter_title,
                    comic_id: comic.id.clone(),
                    comic_title: comic_title.clone(),
                    author: author.clone(),
                    is_downloaded,
                    order: chapter.order,
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
            chapters,
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
        }
    }

    fn get_is_downloaded(
        app: &AppHandle,
        comic_title: &str,
        chapter_title: &str,
        author: &str,
    ) -> bool {
        let download_with_author = app
            .state::<RwLock<Config>>()
            .read_or_panic()
            .download_with_author;
        let comic_title = if download_with_author {
            &format!("[{author}] {comic_title}")
        } else {
            comic_title
        };
        app.state::<RwLock<Config>>()
            .read_or_panic()
            .download_dir
            .join(comic_title)
            .join(chapter_title)
            .exists()
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChapterInfo {
    pub chapter_id: String,
    pub chapter_title: String,
    pub comic_id: String,
    pub comic_title: String,
    pub author: String,
    pub is_downloaded: bool,
    pub order: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Creator {
    #[serde(rename = "_id")]
    pub id: String,
    pub gender: String,
    pub name: String,
    pub title: String,
    pub verified: Option<bool>,
    pub exp: i64,
    pub level: i64,
    pub characters: Vec<String>,
    #[serde(default)]
    pub avatar: Image,
    #[serde(default)]
    pub slogan: String,
    pub role: String,
    #[serde(default)]
    pub character: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    pub original_name: String,
    pub path: String,
    pub file_server: String,
}
