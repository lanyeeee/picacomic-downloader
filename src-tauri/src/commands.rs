use std::sync::{Arc, Mutex, RwLock};

use tauri::{AppHandle, State};
use tokio::task::JoinSet;

use crate::config::Config;
use crate::download_manager::DownloadManager;
use crate::errors::CommandResult;
use crate::extensions::{IgnoreLockPoison, IgnoreRwLockPoison};
use crate::pica_client::PicaClient;
use crate::responses::{Comic, ComicInSearch, EpisodeImage, Pagination, UserProfile};
use crate::types;
use crate::types::Sort;
use crate::utils;

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
pub async fn get_episodes(
    pica_client: State<'_, PicaClient>,
    comic_id: String,
) -> CommandResult<Vec<types::Episode>> {
    let pica_client = pica_client.inner().clone();
    // TODO: 漫画获取和第一个章节获取可以并行
    let comic = pica_client.get_comic(&comic_id).await?;
    let episodes = Arc::new(Mutex::new(vec![]));
    let first_page = pica_client.get_episode(&comic_id, 1).await?;
    episodes.lock_or_panic().extend(first_page.docs);

    let total_pages = first_page.pages;
    let mut join_set = JoinSet::new();

    for page in 2..=total_pages {
        let pica_client = pica_client.clone();
        let episodes = episodes.clone();
        let comic_id = comic_id.clone();
        join_set.spawn(async move {
            let episode_page = pica_client.get_episode(&comic_id, page).await.unwrap();
            episodes.lock().unwrap().extend(episode_page.docs);
        });
    }

    join_set.join_all().await;

    let episodes = {
        let mut episodes = episodes.lock().unwrap();
        episodes.sort_by_key(|ep| ep.order);
        std::mem::take(&mut *episodes)
    };

    let comic_title = utils::filename_filter(&comic.title);
    let episodes = episodes
        .into_iter()
        .map(|ep| types::Episode {
            ep_id: ep.id,
            ep_title: utils::filename_filter(&ep.title),
            comic_id: comic.id.clone(),
            comic_title: comic_title.clone(),
            is_downloaded: false,
            order: ep.order,
        })
        .collect();

    Ok(episodes)
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

#[tauri::command(async)]
#[specta::specta]
pub async fn download_episodes(
    download_manager: State<'_, DownloadManager>,
    episodes: Vec<types::Episode>,
) -> CommandResult<()> {
    for ep in episodes {
        download_manager.submit_episode(ep).await?;
    }
    Ok(())
}
