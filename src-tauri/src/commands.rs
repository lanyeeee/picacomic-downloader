use std::sync::RwLock;

use tauri::State;

use crate::config::Config;
use crate::pica_client::PicaClient;

#[tauri::command]
#[specta::specta]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command(async)]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_config(config: State<RwLock<Config>>) -> Config {
    config.read().unwrap().clone()
}

#[tauri::command(async)]
#[specta::specta]
pub async fn login(
    pica_client: State<'_, PicaClient>,
    email: String,
    password: String,
) -> Result<String, ()> {
    let token = pica_client.login(&email, &password).await.unwrap();
    Ok(token)
}
