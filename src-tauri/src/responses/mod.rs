mod get_chapter_image_resp_data;
mod get_chapter_resp_data;
mod get_comic_resp_data;
mod get_favorite_resp_data;
mod login_resp_data;
mod search_resp_data;
mod user_profile_resp_data;

pub use get_chapter_image_resp_data::*;
pub use get_chapter_resp_data::*;
pub use get_comic_resp_data::*;
pub use get_favorite_resp_data::*;
pub use login_resp_data::*;
pub use search_resp_data::*;
pub use user_profile_resp_data::*;

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PicaResp {
    pub code: i64,
    pub error: Option<String>,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub detail: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Pagination<T> {
    pub total: i64,
    pub limit: i64,
    pub page: i64,
    pub pages: i64,
    pub docs: Vec<T>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImageRespData {
    pub original_name: String,
    pub path: String,
    pub file_server: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreatorRespData {
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
    pub avatar: ImageRespData,
    #[serde(default)]
    pub slogan: String,
    pub role: String,
    #[serde(default)]
    pub character: String,
}

fn string_to_i64<'de, D>(d: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde_json::Value;
    let value: Value = serde::Deserialize::deserialize(d)?;

    match value {
        #[allow(clippy::cast_possible_truncation)]
        Value::Number(n) => Ok(n.as_i64().unwrap_or(0)),
        Value::String(s) => Ok(s.parse().unwrap_or(0)),
        _ => Err(serde::de::Error::custom(
            "`string_to_i64` 失败，value类型不是 `Number` 或 `String`",
        )),
    }
}

fn string_to_option_i64<'de, D>(d: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde_json::Value;
    let value: Value = serde::Deserialize::deserialize(d)?;

    match value {
        #[allow(clippy::cast_possible_truncation)]
        Value::Number(n) => Ok(Some(n.as_i64().unwrap_or(0))),
        Value::String(s) => Ok(Some(s.parse().unwrap_or(0))),
        _ => Err(serde::de::Error::custom(
            "`string_to_option_i64` 失败，value类型不是 `Number` 或 `String`",
        )),
    }
}
