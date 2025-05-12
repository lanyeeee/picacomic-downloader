use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;

use crate::utils::filename_filter;

use super::Comic;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChapterInfo {
    pub chapter_id: String,
    pub chapter_title: String,
    pub order: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_downloaded: Option<bool>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub chapter_dir_name: String,
}

impl ChapterInfo {
    pub fn get_temp_download_dir(&self, app: &AppHandle, comic: &Comic) -> PathBuf {
        let comic_download_dir = Comic::get_comic_download_dir(app, &comic.title, &comic.author);

        let order = self.order;
        let chapter_title = filename_filter(&self.chapter_title);
        let prefixed_chapter_title = format!("{order} {chapter_title}");
        comic_download_dir.join(format!(".下载中-{prefixed_chapter_title}")) // 以 `.下载中-` 开头，表示是临时目录
    }

    pub fn get_chapter_download_dir(&self, app: &AppHandle, comic: &Comic) -> PathBuf {
        let comic_download_dir = Comic::get_comic_download_dir(app, &comic.title, &comic.author);

        let order = self.order;
        let chapter_title = filename_filter(&self.chapter_title);
        let prefixed_chapter_title = format!("{order} {chapter_title}");
        comic_download_dir.join(prefixed_chapter_title)
    }

    pub fn get_is_downloaded(
        app: &AppHandle,
        comic_title: &str,
        chapter_title: &str,
        author: &str,
        order: i64,
    ) -> bool {
        let comic_download_dir = Comic::get_comic_download_dir(app, comic_title, author);

        let chapter_title = filename_filter(chapter_title);
        let prefixed_chapter_title = format!("{order} {chapter_title}");
        comic_download_dir.join(prefixed_chapter_title).exists()
    }
}
