use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

pub mod prelude {
    pub use crate::events::{
        DownloadEpisodeEndEvent, DownloadEpisodePendingEvent, DownloadEpisodeStartEvent,
        DownloadImageErrorEvent, DownloadImageSuccessEvent, DownloadSpeedEvent,
        UpdateOverallDownloadProgressEvent,
    };
}

#[derive(Serialize, Deserialize, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub struct DownloadEpisodePendingEventPayload {
    pub ep_id: String,
    pub title: String,
}
#[derive(Serialize, Deserialize, Clone, Type, Event)]
pub struct DownloadEpisodePendingEvent(pub DownloadEpisodePendingEventPayload);

#[derive(Serialize, Deserialize, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub struct DownloadEpisodeStartEventPayload {
    pub ep_id: String,
    pub title: String,
    pub total: u32,
}
#[derive(Serialize, Deserialize, Clone, Type, Event)]
pub struct DownloadEpisodeStartEvent(pub DownloadEpisodeStartEventPayload);

#[derive(Serialize, Deserialize, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub struct DownloadImageSuccessEventPayload {
    pub ep_id: String,
    pub url: String,
    pub downloaded_count: u32,
}
#[derive(Serialize, Deserialize, Clone, Type, Event)]
pub struct DownloadImageSuccessEvent(pub DownloadImageSuccessEventPayload);

#[derive(Serialize, Deserialize, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub struct DownloadImageErrorEventPayload {
    pub ep_id: String,
    pub url: String,
    pub err_msg: String,
}
#[derive(Serialize, Deserialize, Clone, Type, Event)]
pub struct DownloadImageErrorEvent(pub DownloadImageErrorEventPayload);

#[derive(Serialize, Deserialize, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub struct DownloadEpisodeEndEventPayload {
    pub ep_id: String,
    pub err_msg: Option<String>,
}
#[derive(Serialize, Deserialize, Clone, Type, Event)]
pub struct DownloadEpisodeEndEvent(pub DownloadEpisodeEndEventPayload);

#[derive(Serialize, Deserialize, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOverallDownloadProgressEventPayload {
    pub downloaded_image_count: u32,
    pub total_image_count: u32,
    pub percentage: f64,
}
#[derive(Serialize, Deserialize, Clone, Type, Event)]
pub struct UpdateOverallDownloadProgressEvent(pub UpdateOverallDownloadProgressEventPayload);

#[derive(Serialize, Deserialize, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub struct DownloadSpeedEventPayload {
    pub speed: String,
}
#[derive(Serialize, Deserialize, Clone, Type, Event)]
pub struct DownloadSpeedEvent(pub DownloadSpeedEventPayload);
