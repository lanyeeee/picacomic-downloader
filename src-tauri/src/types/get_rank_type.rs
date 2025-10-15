use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub enum RankType {
    Day,
    Week,
    Month,
}

impl RankType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RankType::Day => "H24",
            RankType::Week => "D7",
            RankType::Month => "D30",
        }
    }
}
