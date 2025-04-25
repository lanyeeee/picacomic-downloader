#![allow(clippy::used_underscore_binding)]
use std::sync::Arc;

use anyhow::{anyhow, Context};
use parking_lot::{Mutex, RwLock};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use tokio::task::JoinSet;

use crate::config::Config;
use crate::download_manager::DownloadManager;
use crate::errors::CommandResult;
use crate::pica_client::PicaClient;
use crate::responses::{
    ChapterImageRespData, ComicInFavoriteRespData, ComicInSearchRespData, Pagination,
    UserProfileDetailRespData,
};
use crate::types::{ChapterInfo, Comic, Sort};

#[tauri::command]
#[specta::specta]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command(async)]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_config(config: State<RwLock<Config>>) -> Config {
    config.read().clone()
}

#[tauri::command(async)]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn save_config(
    app: AppHandle,
    config_state: State<RwLock<Config>>,
    config: Config,
) -> CommandResult<()> {
    let mut config_state = config_state.write();
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
pub async fn get_user_profile(
    pica_client: State<'_, PicaClient>,
) -> CommandResult<UserProfileDetailRespData> {
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
) -> CommandResult<Pagination<ComicInSearchRespData>> {
    let comic_in_search_pagination = pica_client
        .search_comic(&keyword, sort, page, categories)
        .await?;
    Ok(comic_in_search_pagination)
}

#[tauri::command(async)]
#[specta::specta]
pub async fn get_comic(
    app: AppHandle,
    pica_client: State<'_, PicaClient>,
    comic_id: String,
) -> CommandResult<Comic> {
    let pica_client = pica_client.inner().clone();
    // 获取漫画详情和章节的第一页
    let comic_task = pica_client.get_comic(&comic_id);
    let first_page_task = pica_client.get_chapter(&comic_id, 1);
    let (comic, first_page) = tokio::try_join!(comic_task, first_page_task)?;
    // 准备根据章节的第一页获取所有章节
    // 先把第一页的章节放进去
    let chapters = Arc::new(Mutex::new(vec![]));
    chapters.lock().extend(first_page.docs);
    // 获取剩下的章节
    let total_pages = first_page.pages;
    let mut join_set = JoinSet::new();
    for page in 2..=total_pages {
        let pica_client = pica_client.clone();
        let chapters = chapters.clone();
        let comic_id = comic_id.clone();
        // 创建获取章节的任务
        join_set.spawn(async move {
            let chapter_page = pica_client.get_chapter(&comic_id, page).await.unwrap();
            chapters.lock().extend(chapter_page.docs);
        });
    }
    // 等待所有章节获取完毕
    join_set.join_all().await;
    // 按章节顺序排序
    let chapters = {
        let mut chapters = chapters.lock();
        chapters.sort_by_key(|chapter| chapter.order);
        std::mem::take(&mut *chapters)
    };
    let comic = Comic::from(&app, comic, chapters);

    Ok(comic)
}

#[tauri::command(async)]
#[specta::specta]
pub async fn get_chapter_image(
    pica_client: State<'_, PicaClient>,
    comic_id: String,
    chapter_order: i64,
    page: i64,
) -> CommandResult<Pagination<ChapterImageRespData>> {
    let chapter_image_pagination = pica_client
        .get_chapter_image(&comic_id, chapter_order, page)
        .await?;
    Ok(chapter_image_pagination)
}

#[tauri::command(async)]
#[specta::specta]
pub async fn download_chapters(
    download_manager: State<'_, DownloadManager>,
    chapters: Vec<ChapterInfo>,
) -> CommandResult<()> {
    for chapter in chapters {
        download_manager.submit_chapter(chapter).await?;
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
    let comic = get_comic(app, pica_client, comic_id).await?;
    let chapter_infos: Vec<ChapterInfo> = comic
        .chapter_infos
        .into_iter()
        .filter(|chapter_info| chapter_info.is_downloaded != Some(true))
        .collect();
    if chapter_infos.is_empty() {
        let comic_title = comic.title;
        return Err(
            anyhow!("漫画`{comic_title}`的所有章节都已存在于下载目录，无需重复下载").into(),
        );
    }
    download_chapters(download_manager, chapter_infos).await?;
    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn show_path_in_file_manager(app: AppHandle, path: &str) -> CommandResult<()> {
    app.opener()
        .reveal_item_in_dir(path)
        .context(format!("在文件管理器中打开`{path}`失败"))?;
    Ok(())
}

#[tauri::command(async)]
#[specta::specta]
pub async fn get_favorite_comics(
    pica_client: State<'_, PicaClient>,
    sort: Sort,
    page: i64,
) -> CommandResult<Pagination<ComicInFavoriteRespData>> {
    let favorite_comics = pica_client.get_favorite_comics(sort, page).await?;
    Ok(favorite_comics)
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn save_metadata(app: AppHandle, mut comic: Comic) -> CommandResult<()> {
    // 将所有章节的is_downloaded字段设置为None，这样能使is_downloaded字段在序列化时被忽略
    for chapter in &mut comic.chapter_infos {
        chapter.is_downloaded = None;
    }

    let comic_title = comic.title.clone();
    let comic_json = serde_json::to_string_pretty(&comic).context(format!(
        "`{comic_title}`的元数据保存失败，将Comic序列化为json失败"
    ))?;
    let comic_download_dir = Comic::get_comic_download_dir(&app, &comic_title, &comic.author);
    let metadata_path = comic_download_dir.join("元数据.json");

    std::fs::create_dir_all(&comic_download_dir)
        .context(format!("创建目录`{comic_download_dir:?}`失败"))?;

    std::fs::write(&metadata_path, comic_json)
        .context(format!("写入文件`{metadata_path:?}`失败"))?;

    Ok(())
}
