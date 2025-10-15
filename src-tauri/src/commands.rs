#![allow(clippy::used_underscore_binding)]
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context};
use indexmap::IndexMap;
use parking_lot::{Mutex, RwLock};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use tauri_specta::Event;
use tokio::task::JoinSet;
use tokio::time::sleep;
use walkdir::WalkDir;

use crate::config::Config;
use crate::download_manager::DownloadManager;
use crate::errors::{CommandError, CommandResult};
use crate::events::DownloadAllFavoritesEvent;
use crate::extensions::{AnyhowErrorToStringChain, WalkDirEntryExt};
use crate::pica_client::PicaClient;
use crate::responses::{ChapterImageRespData, Pagination, UserProfileDetailRespData};
use crate::types::{
    ChapterInfo, Comic, ComicInFavorite, ComicInRank, ComicInSearch, GetFavoriteResult,
    GetFavoriteSort, GetRankResult, RankType, SearchResult, SearchSort,
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
        .context(format!("获取ID为`{comic_id}`的漫画失败"))
        .map_err(|err| CommandError::from("获取漫画失败", err))?;

    Ok(comic)
}

// TODO: 删了这个用不到的command
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
        .context(format!(
            "漫画`{comic_title}`创建章节ID为`{chapter_id}`的下载任务失败"
        ))
        .map_err(|err| CommandError::from("下载任务创建失败", err))?;
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
        .context(format!("暂停章节ID为`{chapter_id}`的下载任务失败"))
        .map_err(|err| CommandError::from("暂停下载任务失败", err))?;
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
        .context(format!("恢复章节ID为`{chapter_id}`的下载任务失败"))
        .map_err(|err| CommandError::from("恢复下载任务失败", err))?;
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
        .context(format!("取消章节ID为`{chapter_id}`的下载任务失败"))
        .map_err(|err| CommandError::from("取消下载任务失败", err))?;
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
    let comic = utils::get_comic(&app, pica_client.inner(), &comic_id)
        .await
        .context(format!("获取ID为`{comic_id}`的漫画失败"))
        .map_err(|err| CommandError::from("一键下载漫画失败", err))?;

    let comic_title = &comic.title;

    let chapter_infos: Vec<&ChapterInfo> = comic
        .chapter_infos
        .iter()
        .filter(|chapter_info| chapter_info.is_downloaded != Some(true))
        .collect();

    if chapter_infos.is_empty() {
        let err = anyhow!("漫画`{comic_title}`的所有章节都已存在于下载目录，无需重复下载");
        return Err(CommandError::from("一键下载漫画失败", err));
    }

    for chapter_info in chapter_infos {
        let chapter_id = &chapter_info.chapter_id;
        download_manager
            .create_download_task(comic.clone(), chapter_id.clone())
            .context(format!(
                "漫画`{comic_title}`创建章节ID为`{chapter_id}`的下载任务失败"
            ))
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

#[tauri::command(async)]
#[specta::specta]
pub async fn get_rank(
    app: AppHandle,
    pica_client: State<'_, PicaClient>,
    rank_type: RankType,
) -> CommandResult<GetRankResult> {
    let get_rank_resp_data = pica_client
        .get_rank(rank_type)
        .await
        .map_err(|err| CommandError::from("获取排行榜失败", err))?;

    let get_rank_result = GetRankResult::from_resp_data(&app, get_rank_resp_data)
        .map_err(|err| CommandError::from("获取排行榜失败", err))?;

    Ok(get_rank_result)
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
    // TODO: 把favorite_page在JoinSet里返回，然后在.join_next()里处理，这样就不用锁了
    let favorite_comics = Arc::new(Mutex::new(vec![]));
    let _ = DownloadAllFavoritesEvent::GettingFavorites.emit(&app);
    // 获取收藏夹第一页
    let first_page = pica_client
        .get_favorite(GetFavoriteSort::TimeNewest, 1)
        .await
        .context("获取收藏夹的第`1`页失败")
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
                .await
                .context(format!("获取收藏夹的第`{page}`页失败"))?;
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
        let comic_title = &favorite_comic.title;
        let comic_id = &favorite_comic.id;

        let comic = match utils::get_comic(&app, &pica_client, comic_id)
            .await
            .context(format!("获取ID为`{comic_id}`的漫画失败"))
        {
            Ok(comic) => comic,
            Err(err) => {
                let err_title = format!("下载收藏夹过程中，获取漫画`{comic_title}`失败，已跳过");
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
#[allow(clippy::too_many_lines)]
#[tauri::command(async)]
#[specta::specta]
pub fn get_downloaded_comics(config: State<RwLock<Config>>) -> Vec<Comic> {
    let download_dir = config.read().download_dir.clone();
    // 遍历下载目录，获取所有漫画元数据文件的路径和修改时间
    let mut metadata_path_with_modify_time = Vec::new();
    for entry in WalkDir::new(&download_dir)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();

        if !entry.is_comic_metadata() {
            continue;
        }

        let metadata = match path
            .metadata()
            .map_err(anyhow::Error::from)
            .context(format!("获取`{}`的metadata失败", path.display()))
        {
            Ok(metadata) => metadata,
            Err(err) => {
                let err_title = "获取已下载漫画的过程中遇到错误，已跳过";
                let string_chain = err.to_string_chain();
                tracing::error!(err_title, message = string_chain);
                continue;
            }
        };

        let modify_time = match metadata
            .modified()
            .map_err(anyhow::Error::from)
            .context(format!("获取`{}`的修改时间失败", path.display()))
        {
            Ok(modify_time) => modify_time,
            Err(err) => {
                let err_title = "获取已下载漫画的过程中遇到错误，已跳过";
                let string_chain = err.to_string_chain();
                tracing::error!(err_title, message = string_chain);
                continue;
            }
        };

        metadata_path_with_modify_time.push((path.to_path_buf(), modify_time));
    }
    // 按照文件修改时间排序，最新的排在最前面
    metadata_path_with_modify_time.sort_by(|(_, a), (_, b)| b.cmp(a));

    let mut downloaded_comics = Vec::new();
    for (metadata_path, _) in metadata_path_with_modify_time {
        match Comic::from_metadata(&metadata_path).context(format!(
            "从元数据`{}`转为Comic失败",
            metadata_path.display()
        )) {
            Ok(comic) => downloaded_comics.push(comic),
            Err(err) => {
                let err_title = "获取已下载漫画的过程中遇到错误，已跳过";
                let string_chain = err.to_string_chain();
                tracing::error!(err_title, message = string_chain);
            }
        }
    }
    // 按照漫画ID分组，以方便去重
    let mut comics_by_id: IndexMap<String, Vec<Comic>> = IndexMap::new();
    for comic in downloaded_comics {
        comics_by_id
            .entry(comic.id.clone())
            .or_default()
            .push(comic);
    }

    let mut unique_comics = Vec::new();
    for (_comic_id, mut comics) in comics_by_id {
        // 该漫画ID对应的所有漫画下载目录，可能有多个版本，所以需要去重
        let comic_download_dirs: Vec<&PathBuf> = comics
            .iter()
            .filter_map(|comic| comic.comic_download_dir.as_ref())
            .collect();

        if comic_download_dirs.is_empty() {
            // 其实这种情况不应该发生，因为漫画元数据文件应该总是有下载目录的
            continue;
        }

        // 选第一个作为保留的漫画
        let chosen_download_dir = comic_download_dirs[0];

        if comics.len() > 1 {
            let dir_paths_string = comic_download_dirs
                .iter()
                .map(|path| format!("`{}`", path.display()))
                .collect::<Vec<String>>()
                .join(", ");
            // 如果有重复的漫画，打印错误信息
            let comic_title = &comics[0].title;
            let err_title = "获取已下载漫画的过程中遇到错误";
            let string_chain = anyhow!("所有版本路径: [{dir_paths_string}]")
                .context(format!(
                    "此次获取已下载漫画的结果中只保留版本`{}`",
                    chosen_download_dir.display()
                ))
                .context(format!(
                    "漫画`{comic_title}`在下载目录里有多个版本，请手动处理，只保留一个版本"
                ))
                .to_string_chain();
            tracing::error!(err_title, message = string_chain);
        }
        // 取第一个作为保留的漫画
        let chosen_comic = comics.remove(0);
        unique_comics.push(chosen_comic);
    }

    unique_comics
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
        .context(format!("读取日志目录`{}`失败", logs_dir.display()))
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
    let id_to_dir_map = utils::create_id_to_dir_map(&app)
        .context("创建漫画ID到下载目录映射失败")
        .map_err(|err| {
            CommandError::from(&format!("漫画`{}`同步Comic的字段失败", comic.title), err)
        })?;

    comic.update_fields(&id_to_dir_map).map_err(|err| {
        CommandError::from(&format!("漫画`{}`同步Comic的字段失败", comic.title), err)
    })?;

    Ok(comic)
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn get_synced_comic_in_favorite(
    app: AppHandle,
    mut comic: ComicInFavorite,
) -> CommandResult<ComicInFavorite> {
    let id_to_dir_map = utils::create_id_to_dir_map(&app)
        .context("创建漫画ID到下载目录映射失败")
        .map_err(|err| {
            let err_title = format!("漫画`{}`同步ComicInFavorite的字段失败", comic.title);
            CommandError::from(&err_title, err)
        })?;

    comic.update_fields(&id_to_dir_map);

    Ok(comic)
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn get_synced_comic_in_search(
    app: AppHandle,
    mut comic: ComicInSearch,
) -> CommandResult<ComicInSearch> {
    let id_to_dir_map = utils::create_id_to_dir_map(&app)
        .context("创建漫画ID到下载目录映射失败")
        .map_err(|err| {
            let err_title = format!("漫画`{}`同步ComicInSearch的字段失败", comic.title);
            CommandError::from(&err_title, err)
        })?;

    comic.update_fields(&id_to_dir_map);

    Ok(comic)
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn get_synced_comic_in_rank(
    app: AppHandle,
    mut comic: ComicInRank,
) -> CommandResult<ComicInRank> {
    let id_to_dir_map = utils::create_id_to_dir_map(&app)
        .context("创建漫画ID到下载目录映射失败")
        .map_err(|err| {
            let err_title = format!("漫画`{}`同步ComicInRank的字段失败", comic.title);
            CommandError::from(&err_title, err)
        })?;

    comic.update_fields(&id_to_dir_map);

    Ok(comic)
}
