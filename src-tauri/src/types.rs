use serde::{Deserialize, Serialize};
use specta::Type;

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
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    pub ep_id: String,
    pub ep_title: String,
    pub comic_id: String,
    pub comic_title: String,
    pub is_downloaded: bool,
}
