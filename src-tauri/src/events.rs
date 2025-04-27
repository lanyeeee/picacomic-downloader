use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(tag = "event", content = "data")]
pub enum DownloadEvent {
    #[serde(rename_all = "camelCase")]
    ChapterPending { chapter_id: String, title: String },

    #[serde(rename_all = "camelCase")]
    ChapterStart {
        chapter_id: String,
        title: String,
        total: u32,
    },

    #[serde(rename_all = "camelCase")]
    ImageSuccess {
        chapter_id: String,
        url: String,
        downloaded_count: u32,
    },

    #[serde(rename_all = "camelCase")]
    ImageError {
        chapter_id: String,
        url: String,
        err_msg: String,
    },

    #[serde(rename_all = "camelCase")]
    ChapterEnd {
        chapter_id: String,
        err_msg: Option<String>,
    },

    #[serde(rename_all = "camelCase")]
    OverallUpdate {
        downloaded_image_count: u32,
        total_image_count: u32,
        percentage: f64,
    },

    #[serde(rename_all = "camelCase")]
    Speed { speed: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(tag = "event", content = "data")]
pub enum ExportCbzEvent {
    #[serde(rename_all = "camelCase")]
    Start {
        uuid: String,
        comic_title: String,
        total: u32,
    },
    #[serde(rename_all = "camelCase")]
    Progress { uuid: String, current: u32 },
    #[serde(rename_all = "camelCase")]
    Error { uuid: String },
    #[serde(rename_all = "camelCase")]
    End { uuid: String },
}
