#![allow(clippy::used_underscore_binding)]
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context};
use parking_lot::{Mutex, RwLock};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use tauri_specta::Event;
use tokio::task::JoinSet;
use tokio::time::sleep;

use crate::config::Config;
use crate::download_manager::DownloadManager;
use crate::errors::{CommandError, CommandResult};
use crate::events::DownloadAllFavoritesEvent;
use crate::extensions::AnyhowErrorToStringChain;
use crate::pica_client::PicaClient;
use crate::responses::{ChapterImageRespData, Pagination, UserProfileDetailRespData};
use crate::types::{
    ChapterInfo, Comic, ComicInFavorite, ComicInSearch, GetFavoriteResult, GetFavoriteSort,
    SearchResult, SearchSort,
};
use crate::{export, logger, utils};

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
    // TODO: 把变量名改为search_resp_data
    let comic_in_search_pagination = pica_client
        .search_comic(&keyword, sort, page, categories)
        .await
        .map_err(|err| CommandError::from("搜索漫画失败", err))?;

    let search_result = SearchResult::from_resp_data(&app, comic_in_search_pagination)
        .map_err(|err| CommandError::from("搜索漫画失败", err))?;

    Ok(search_result)
}

#[tauri::command(async)]
#[specta::specta]
pub async fn get_comic(
    app: AppHandle,
    pica_client: State<'_, PicaClient>,
    comic_id: String,
) -> CommandResult<Comic> {
    let comic = utils::get_comic(&app, pica_client.inner(), &comic_id)
        .await
        .map_err(|err| CommandError::from("获取漫画详情失败", err))?;

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

    let get_favorite_result = GetFavoriteResult::from_resp_data(&app, get_favorite_resp_data)
        .map_err(|err| CommandError::from("获取收藏的漫画失败", err))?;

    Ok(get_favorite_result)
}

#[allow(clippy::cast_possible_wrap)]
#[tauri::command(async)]
#[specta::specta]
pub async fn download_all_favorites(
    app: AppHandle,
    config: State<'_, RwLock<Config>>,
    pica_client: State<'_, PicaClient>,
    download_manager: State<'_, DownloadManager>,
) -> CommandResult<()> {
    let pica_client = pica_client.inner().clone();
    let favorite_comics = Arc::new(Mutex::new(vec![]));
    let _ = DownloadAllFavoritesEvent::GettingFavorites.emit(&app);
    // 获取收藏夹第一页
    let first_page = pica_client
        .get_favorite(GetFavoriteSort::TimeNewest, 1)
        .await
        .map_err(|err| CommandError::from("下载收藏夹失败", err))?;
    // 先把第一页的收藏放进去
    favorite_comics.lock().extend(first_page.comics.docs);
    let page_count = first_page.comics.pages;
    // 获取收藏夹剩余页
    let mut join_set = JoinSet::new();
    for page in 2..=page_count {
        let pica_client = pica_client.clone();
        let favorite_comics = favorite_comics.clone();
        join_set.spawn(async move {
            let favorite_page = pica_client
                .get_favorite(GetFavoriteSort::TimeNewest, page)
                .await?;
            favorite_comics.lock().extend(favorite_page.comics.docs);
            Ok::<(), anyhow::Error>(())
        });
    }
    // 等待所有请求完成
    while let Some(Ok(get_favorite_result)) = join_set.join_next().await {
        // 如果有请求失败，直接返回错误
        get_favorite_result.map_err(|err| CommandError::from("下载收藏夹失败", err))?;
    }
    // 至此，收藏夹已经全部获取完毕
    let favorite_comics = std::mem::take(&mut *favorite_comics.lock());
    let total = favorite_comics.len() as i64;
    // 获取收藏夹漫画的详细信息
    let interval_sec = config.read().download_all_favorites_interval_sec;
    for (i, favorite_comic) in favorite_comics.into_iter().enumerate() {
        let comic = match utils::get_comic(&app, &pica_client, &favorite_comic.id).await {
            Ok(comic) => comic,
            Err(err) => {
                let err_title = format!("获取`{}`漫画详情失败，已跳过", favorite_comic.title);
                let err = err.context("可能是频率太高，请手动去`配置`里调整`下载整个收藏夹时，每处理完一个收藏夹中的漫画后休息`");
                tracing::error!(err_title, message = err.to_string_chain());
                sleep(Duration::from_secs(interval_sec)).await;
                continue;
            }
        };

        let current = (i + 1) as i64;
        let _ = DownloadAllFavoritesEvent::GettingComics { current, total }.emit(&app);

        // 给每个漫画未下载的章节创建下载任务
        let chapter_infos: Vec<&ChapterInfo> = comic
            .chapter_infos
            .iter()
            .filter(|chapter_info| chapter_info.is_downloaded != Some(true))
            .collect();
        if chapter_infos.is_empty() {
            sleep(Duration::from_secs(interval_sec)).await;
            continue;
        }

        let _ = DownloadAllFavoritesEvent::StartCreateDownloadTasks {
            comic_id: comic.id.clone(),
            comic_title: comic.title.clone(),
            current: 0,
            total: chapter_infos.len() as i64,
        }
        .emit(&app);

        for (current, chapter_info) in chapter_infos.into_iter().enumerate() {
            let chapter_id = chapter_info.chapter_id.clone();
            let current = current as i64 + 1;
            let _ = download_manager.create_download_task(comic.clone(), chapter_id);

            let _ = DownloadAllFavoritesEvent::CreatingDownloadTask {
                comic_id: comic.id.clone(),
                current,
            }
            .emit(&app);

            sleep(Duration::from_millis(100)).await;
        }

        let _ = DownloadAllFavoritesEvent::EndCreateDownloadTasks {
            comic_id: comic.id.clone(),
        }
        .emit(&app);

        sleep(Duration::from_secs(interval_sec)).await;
    }
    // 至此，收藏夹漫画的详细信息已经全部获取完毕
    let _ = DownloadAllFavoritesEvent::EndGetComics.emit(&app);

    Ok(())
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
                    let err_title = "从元数据转为Comic失败";
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
pub fn export_cbz(app: AppHandle, mut comic: Comic) -> CommandResult<()> {
    comic
        .update_fields(&app)
        .context(format!("`{}`更新Comic的字段失败", comic.title))
        .map_err(|err| CommandError::from("导出cbz失败", err))?;

    let comic_title = &comic.title;
    export::cbz(&app, &comic)
        .context(format!("漫画`{comic_title}`导出cbz失败"))
        .map_err(|err| CommandError::from("导出cbz失败", err))?;
    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn export_pdf(app: AppHandle, mut comic: Comic) -> CommandResult<()> {
    comic
        .update_fields(&app)
        .context(format!("`{}`更新Comic的字段失败", comic.title))
        .map_err(|err| CommandError::from("导出pdf失败", err))?;

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

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn get_synced_comic(app: AppHandle, mut comic: Comic) -> CommandResult<Comic> {
    comic
        .update_fields(&app)
        .map_err(|err| CommandError::from(&format!("`{}`更新Comic的字段失败", comic.title), err))?;

    Ok(comic)
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn get_synced_comic_in_favorite(
    app: AppHandle,
    mut comic: ComicInFavorite,
) -> CommandResult<ComicInFavorite> {
    comic.update_fields(&app).map_err(|err| {
        let err_title = format!("`{}`更新ComicInFavorite的字段失败", comic.title);
        CommandError::from(&err_title, err)
    })?;

    Ok(comic)
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn get_synced_comic_in_search(
    app: AppHandle,
    mut comic: ComicInSearch,
) -> CommandResult<ComicInSearch> {
    comic.update_fields(&app).map_err(|err| {
        let err_title = format!("`{}`更新ComicInSearch的字段失败", comic.title);
        CommandError::from(&err_title, err)
    })?;

    Ok(comic)
}
