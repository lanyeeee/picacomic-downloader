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
