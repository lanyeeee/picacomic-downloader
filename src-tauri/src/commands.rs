use std::sync::RwLock;

use tauri::{AppHandle, State};

use crate::config::Config;
use crate::errors::CommandResult;
use crate::extensions::IgnoreRwLockPoison;
use crate::pica_client::PicaClient;
use crate::responses::{Comic, ComicInSearch, Episode, EpisodeImage, Pagination, UserProfile};
use crate::types::Sort;

#[tauri::command]
#[specta::specta]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command(async)]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_config(config: State<RwLock<Config>>) -> Config {
    // TODO: 改用 read_or_panic
    config.read().unwrap().clone()
}

#[tauri::command(async)]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn save_config(
    app: AppHandle,
    config_state: State<RwLock<Config>>,
    config: Config,
) -> CommandResult<()> {
    let mut config_state = config_state.write_or_panic();
    *config_state = config;
    config_state.save(&app)?;
    Ok(())
}

#[tauri::command(async)]
#[specta::specta]
pub async fn login(
    pica_client: State<'_, PicaClient>,
    email: String,
    password: String,
) -> CommandResult<String> {
    let token = pica_client.login(&email, &password).await?;
    Ok(token)
}

#[tauri::command(async)]
#[specta::specta]
pub async fn get_user_profile(pica_client: State<'_, PicaClient>) -> CommandResult<UserProfile> {
    let user_profile = pica_client.get_user_profile().await?;
    Ok(user_profile)
}

#[tauri::command(async)]
#[specta::specta]
pub async fn search_comic(
    pica_client: State<'_, PicaClient>,
    keyword: String,
    sort: Sort,
    page: i32,
    categories: Vec<String>,
) -> CommandResult<Pagination<ComicInSearch>> {
    let comic_in_search_pagination = pica_client
        .search_comic(&keyword, sort, page, categories)
        .await?;
    Ok(comic_in_search_pagination)
}

#[tauri::command(async)]
#[specta::specta]
pub async fn get_comic(
    pica_client: State<'_, PicaClient>,
    comic_id: String,
) -> CommandResult<Comic> {
    let comic = pica_client.get_comic(&comic_id).await?;
    Ok(comic)
}

#[tauri::command(async)]
#[specta::specta]
pub async fn get_episode(
    pica_client: State<'_, PicaClient>,
    comic_id: String,
    page: i64,
) -> CommandResult<Pagination<Episode>> {
    let episode_pagination = pica_client.get_episode(&comic_id, page).await?;
    Ok(episode_pagination)
}

#[tauri::command(async)]
#[specta::specta]
pub async fn get_episode_image(
    pica_client: State<'_, PicaClient>,
    comic_id: String,
    episode_order: i64,
    page: i64,
) -> CommandResult<Pagination<EpisodeImage>> {
    let episode_image_pagination = pica_client
        .get_episode_image(&comic_id, episode_order, page)
        .await?;
    Ok(episode_image_pagination)
}
