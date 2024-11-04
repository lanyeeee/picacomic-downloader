use std::path::PathBuf;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub token: String,
    pub download_dir: PathBuf,
    pub episode_download_interval: u64,
    pub download_with_author: bool,
}

impl Config {
    pub fn new(app: &AppHandle) -> anyhow::Result<Self> {
        let resource_dir = app.path().app_data_dir()?;
        let config_path = resource_dir.join("config.json");
        let default_config = Config {
            token: String::new(),
            download_dir: app.path().app_data_dir()?.join("漫画下载"),
            episode_download_interval: 0,
            download_with_author: false,
        };
        // 如果配置文件存在且能够解析，则使用配置文件中的配置，否则使用默认配置
        let config = if config_path.exists() {
            let config_string = std::fs::read_to_string(config_path)?;
            serde_json::from_str(&config_string).unwrap_or(default_config)
        } else {
            default_config
        };
        if !config.download_dir.exists() {
            std::fs::create_dir_all(&config.download_dir).context(format!(
                "创建下载目录`{}`失败",
                config.download_dir.display()
            ))?;
        }
        config.save(app)?;
        Ok(config)
    }

    pub fn save(&self, app: &AppHandle) -> anyhow::Result<()> {
        let resource_dir = app.path().app_data_dir()?;
        let config_path = resource_dir.join("config.json");
        let config_string = serde_json::to_string_pretty(self)?;
        std::fs::write(config_path, config_string)?;
        Ok(())
    }
}
