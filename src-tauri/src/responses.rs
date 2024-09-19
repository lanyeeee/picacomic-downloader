use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PicaResponse {
    pub code: i64,
    pub error: Option<String>,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub detail: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponseData {
    pub token: String,
}
