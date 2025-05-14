use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;

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
    pub fn get_chapter_download_dir(&self, app: &AppHandle, comic: &Comic) -> PathBuf {
        let comic_download_dir = comic.get_comic_download_dir(app);

        comic_download_dir.join(&self.chapter_dir_name)
    }
}
