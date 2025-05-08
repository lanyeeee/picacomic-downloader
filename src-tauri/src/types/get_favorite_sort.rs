use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub enum GetFavoriteSort {
    TimeNewest,
    TimeOldest,
}

impl GetFavoriteSort {
    pub fn as_str(&self) -> &'static str {
        match self {
            GetFavoriteSort::TimeNewest => "dd",
            GetFavoriteSort::TimeOldest => "da",
        }
    }
}
