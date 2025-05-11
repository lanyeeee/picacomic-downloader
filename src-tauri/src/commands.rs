#![allow(clippy::used_underscore_binding)]
use std::sync::Arc;

use anyhow::{anyhow, Context};
use parking_lot::{Mutex, RwLock};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use tokio::task::JoinSet;

use crate::config::Config;
use crate::download_manager::DownloadManager;
use crate::errors::{CommandError, CommandResult};
use crate::extensions::AnyhowErrorToStringChain;
use crate::pica_client::PicaClient;
use crate::responses::{ChapterImageRespData, Pagination, UserProfileDetailRespData};
use crate::types::{
    ChapterInfo, Comic, GetFavoriteResult, GetFavoriteSort, SearchResult, SearchSort,
};
use crate::{export, logger};

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
    let enable_file_logger = config.enable_file_logger;
    let enable_file_logger_changed = config_state
        .read()
        .enable_file_logger
        .ne(&enable_file_logger);

    {
        // 包裹在大括号中，以便自动释放写锁
        let mut config_state = config_state.write();
        *config_state = config;
        config_state
            .save(&app)
            .map_err(|err| CommandError::from("保存配置失败", err))?;
        tracing::debug!("保存配置成功");
    }

    if enable_file_logger_changed {
        if enable_file_logger {
            logger::reload_file_logger()
                .map_err(|err| CommandError::from("重新加载文件日志失败", err))?;
        } else {
            logger::disable_file_logger()
                .map_err(|err| CommandError::from("禁用文件日志失败", err))?;
        }
    }

    Ok(())
}

#[tauri::command(async)]
#[specta::specta]
pub async fn login(
    pica_client: State<'_, PicaClient>,
    email: String,
    password: String,
) -> CommandResult<String> {
    let token = pica_client
        .login(&email, &password)
        .await
        .map_err(|err| CommandError::from("登录失败", err))?;
    Ok(token)
}

#[tauri::command(async)]
#[specta::specta]
pub async fn get_user_profile(
    pica_client: State<'_, PicaClient>,
) -> CommandResult<UserProfileDetailRespData> {
    let user_profile = pica_client
        .get_user_profile()
        .await
        .map_err(|err| CommandError::from("获取用户信息失败", err))?;
    Ok(user_profile)
}

#[tauri::command(async)]
#[specta::specta]
pub async fn search_comic(
    app: AppHandle,
    pica_client: State<'_, PicaClient>,
    keyword: String,
    sort: SearchSort,
    page: i32,
    categories: Vec<String>,
) -> CommandResult<SearchResult> {
    let comic_in_search_pagination = pica_client
        .search_comic(&keyword, sort, page, categories)
        .await
        .map_err(|err| CommandError::from("搜索漫画失败", err))?;

    let search_result = SearchResult::from_resp_data(&app, comic_in_search_pagination);
    Ok(search_result)
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
    let (comic, first_page) = tokio::try_join!(
        pica_client.get_comic(&comic_id),
        pica_client.get_chapter(&comic_id, 1)
    )
    .map_err(|err| {
        let err_title = format!("获取漫画`{comic_id}的详情或章节的第一页失败`");
        CommandError::from(&err_title, err)
    })?;
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
            let chapter_page = match pica_client.get_chapter(&comic_id, page).await {
                Ok(chapter_page) => chapter_page,
                Err(err) => {
                    let err_title = format!("获取漫画`{comic_id}`章节的第{page}页章节失败");
                    let string_chain = err.to_string_chain();
                    tracing::error!(err_title, message = string_chain);
                    return;
                }
            };
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
        .get_chapter_img(&comic_id, chapter_order, page)
        .await
        .map_err(|err| CommandError::from("获取章节图片失败", err))?;
    Ok(chapter_image_pagination)
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn create_download_task(
    download_manager: State<DownloadManager>,
    comic: Comic,
    chapter_id: String,
) -> CommandResult<()> {
    let comic_title = comic.title.clone();
    download_manager
        .create_download_task(comic, chapter_id.clone())
        .map_err(|err| {
            let err_title = format!("`{comic_title}`的章节ID为`{chapter_id}`的下载任务创建失败");
            CommandError::from(&err_title, err)
        })?;
    tracing::debug!("下载任务创建成功");
    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn pause_download_task(
    download_manager: State<DownloadManager>,
    chapter_id: String,
) -> CommandResult<()> {
    download_manager
        .pause_download_task(&chapter_id)
        .map_err(|err| CommandError::from(&format!("暂停章节ID为`{chapter_id}`的下载任务"), err))?;
    tracing::debug!("暂停章节ID为`{chapter_id}`的下载任务成功");
    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn resume_download_task(
    download_manager: State<DownloadManager>,
    chapter_id: String,
) -> CommandResult<()> {
    download_manager
        .resume_download_task(&chapter_id)
        .map_err(|err| CommandError::from(&format!("恢复章节ID为`{chapter_id}`的下载任务"), err))?;
    tracing::debug!("恢复章节ID为`{chapter_id}`的下载任务成功");
    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn cancel_download_task(
    download_manager: State<DownloadManager>,
    chapter_id: String,
) -> CommandResult<()> {
    download_manager
        .cancel_download_task(&chapter_id)
        .map_err(|err| CommandError::from(&format!("取消章节ID为`{chapter_id}`的下载任务"), err))?;
    tracing::debug!("取消章节ID为`{chapter_id}`的下载任务成功");
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
    let chapter_infos: Vec<&ChapterInfo> = comic
        .chapter_infos
        .iter()
        .filter(|chapter_info| chapter_info.is_downloaded != Some(true))
        .collect();
    if chapter_infos.is_empty() {
        let comic_title = &comic.title;
        return Err(CommandError::from(
            "一键下载漫画失败",
            anyhow!("漫画`{comic_title}`的所有章节都已存在于下载目录，无需重复下载"),
        ));
    }
    for chapter_info in chapter_infos {
        download_manager
            .create_download_task(comic.clone(), chapter_info.chapter_id.clone())
            .map_err(|err| CommandError::from("一键下载漫画失败", err))?;
    }
    tracing::debug!("一键下载漫画成功，已为所有需要下载的章节创建下载任务");
    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn show_path_in_file_manager(app: AppHandle, path: &str) -> CommandResult<()> {
    app.opener()
        .reveal_item_in_dir(path)
        .context(format!("在文件管理器中打开`{path}`失败"))
        .map_err(|err| CommandError::from("在文件管理器中打开失败", err))?;
    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn show_comic_download_dir_in_file_manager(
    app: AppHandle,
    comic_title: String,
    author: String,
) -> CommandResult<()> {
    let comic_download_dir = Comic::get_comic_download_dir(&app, &comic_title, &author);
    app.opener()
        .reveal_item_in_dir(&comic_download_dir)
        .context(format!("在文件管理器中打开`{comic_download_dir:?}`失败"))
        .map_err(|err| CommandError::from("在文件管理器中打开失败", err))?;
    Ok(())
}

#[tauri::command(async)]
#[specta::specta]
pub async fn get_favorite(
    app: AppHandle,
    pica_client: State<'_, PicaClient>,
    sort: GetFavoriteSort,
    page: i64,
) -> CommandResult<GetFavoriteResult> {
    let get_favorite_resp_data = pica_client
        .get_favorite(sort, page)
        .await
        .map_err(|err| CommandError::from("获取收藏的漫画失败", err))?;

    let get_favorite_result = GetFavoriteResult::from_resp_data(&app, get_favorite_resp_data);
    Ok(get_favorite_result)
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn get_downloaded_comics(
    app: AppHandle,
    config: State<RwLock<Config>>,
) -> CommandResult<Vec<Comic>> {
    let download_dir = config.read().download_dir.clone();
    // 遍历下载目录，获取所有元数据文件的路径和修改时间
    let mut metadata_path_with_modify_time = std::fs::read_dir(&download_dir)
        .context(format!(
            "获取已下载的漫画失败，读取下载目录 {download_dir:?} 失败"
        ))
        .map_err(|err| CommandError::from("获取已下载的漫画失败", err))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let metadata_path = entry.path().join("元数据.json");
            if !metadata_path.exists() {
                return None;
            }
            let modify_time = metadata_path.metadata().ok()?.modified().ok()?;
            Some((metadata_path, modify_time))
        })
        .collect::<Vec<_>>();
    // 按照文件修改时间排序，最新的排在最前面
    metadata_path_with_modify_time.sort_by(|(_, a), (_, b)| b.cmp(a));
    let downloaded_comics: Vec<Comic> = metadata_path_with_modify_time
        .iter()
        .filter_map(
            |(metadata_path, _)| match Comic::from_metadata(&app, metadata_path) {
                Ok(comic) => Some(comic),
                Err(err) => {
                    let err_title = format!("读取元数据文件`{metadata_path:?}`失败");
                    let string_chain = err.to_string_chain();
                    tracing::error!(err_title, message = string_chain);
                    None
                }
            },
        )
        .collect();

    // 根据comicId去重
    let mut comic_id_set = std::collections::HashSet::new();
    let downloaded_comics = downloaded_comics
        .into_iter()
        .filter(|comic| comic_id_set.insert(comic.id.clone()))
        .collect();
    Ok(downloaded_comics)
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn export_cbz(app: AppHandle, comic: Comic) -> CommandResult<()> {
    let comic_title = &comic.title;
    export::cbz(&app, &comic)
        .context(format!("漫画`{comic_title}`导出cbz失败"))
        .map_err(|err| CommandError::from("导出cbz失败", err))?;
    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn export_pdf(app: AppHandle, comic: Comic) -> CommandResult<()> {
    let comic_title = &comic.title;
    export::pdf(&app, &comic)
        .context(format!("漫画`{comic_title}`导出pdf失败"))
        .map_err(|err| CommandError::from("导出pdf失败", err))?;
    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn get_logs_dir_size(app: AppHandle) -> CommandResult<u64> {
    let logs_dir = logger::logs_dir(&app)
        .context("获取日志目录失败")
        .map_err(|err| CommandError::from("获取日志目录大小失败", err))?;
    let logs_dir_size = std::fs::read_dir(&logs_dir)
        .context(format!("读取日志目录`{logs_dir:?}`失败"))
        .map_err(|err| CommandError::from("获取日志目录大小失败", err))?
        .filter_map(Result::ok)
        .filter_map(|entry| entry.metadata().ok())
        .map(|metadata| metadata.len())
        .sum::<u64>();
    tracing::debug!("获取日志目录大小成功");
    Ok(logs_dir_size)
}
