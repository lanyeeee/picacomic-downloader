#![allow(clippy::used_underscore_binding)]
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use anyhow::anyhow;
use path_slash::PathBufExt;
use tauri::{AppHandle, Manager, State};
use tokio::task::JoinSet;

use crate::config::Config;
use crate::download_manager::DownloadManager;
use crate::errors::CommandResult;
use crate::extensions::IgnoreRwLockPoison;
use crate::pica_client::PicaClient;
use crate::responses::{Comic, ComicInSearch, ComicSimple, EpisodeImage, Pagination, UserProfile};
use crate::types;
use crate::types::{Episode, Sort};
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
    config.read_or_panic().clone()
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
    app: AppHandle,
    pica_client: State<'_, PicaClient>,
    comic_id: String,
) -> CommandResult<Vec<types::Episode>> {
    let pica_client = pica_client.inner().clone();

    let comic_task = pica_client.get_comic(&comic_id);
    let first_page_task = pica_client.get_episode(&comic_id, 1);
    let (comic, first_page) = tokio::try_join!(comic_task, first_page_task)?;

    let episodes = Arc::new(Mutex::new(vec![]));
    episodes.lock().unwrap().extend(first_page.docs);

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
    let download_dir = app
        .state::<RwLock<Config>>()
        .read_or_panic()
        .download_dir
        .clone();

    let episodes = episodes
        .into_iter()
        .map(|ep| {
            let episode_dir = download_dir.join(&comic_title).join(&ep.title);
            types::Episode {
                ep_id: ep.id,
                ep_title: utils::filename_filter(&ep.title),
                comic_id: comic.id.clone(),
                comic_title: comic_title.clone(),
                is_downloaded: episode_dir.exists(),
                order: ep.order,
            }
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

#[tauri::command(async)]
#[specta::specta]
pub async fn download_comic(
    app: AppHandle,
    pica_client: State<'_, PicaClient>,
    download_manager: State<'_, DownloadManager>,
    comic_id: String,
) -> CommandResult<()> {
    let episodes: Vec<Episode> = get_episodes(app, pica_client, comic_id)
        .await?
        .into_iter()
        .filter(|ep| !ep.is_downloaded)
        .collect();
    if episodes.is_empty() {
        // TODO: 错误提示里添加漫画名
        return Err(anyhow!("该漫画的所有章节都已存在于下载目录，无需重复下载").into());
    }
    download_episodes(download_manager, episodes).await?;
    Ok(())
}

#[tauri::command(async)]
#[specta::specta]
pub fn show_path_in_file_manager(path: &str) -> CommandResult<()> {
    let path = PathBuf::from_slash(path);
    if !path.exists() {
        return Err(anyhow!("路径`{path:?}`不存在").into());
    }
    showfile::show_path_in_file_manager(path);
    Ok(())
}

#[tauri::command(async)]
#[specta::specta]
pub async fn get_favourite_comics(
    pica_client: State<'_, PicaClient>,
    sort: Sort,
    page: i64,
) -> CommandResult<Pagination<ComicSimple>> {
    let favourite_comics = pica_client.get_favourite_comics(sort, page).await?;
    Ok(favourite_comics)
}
