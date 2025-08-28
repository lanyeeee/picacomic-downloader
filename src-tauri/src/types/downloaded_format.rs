use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Default, Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Type)]
pub enum DownloadFormat {
    #[default]
    Jpeg,
    Png,
    Webp,
    Original,
}

impl DownloadFormat {
    pub fn extension(self) -> Option<&'static str> {
        match self {
            DownloadFormat::Jpeg => Some("jpg"),
            DownloadFormat::Png => Some("png"),
            DownloadFormat::Webp => Some("webp"),
            DownloadFormat::Original => None,
        }
    }
}
