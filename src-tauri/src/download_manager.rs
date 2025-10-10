use std::collections::HashMap;
use std::io::Cursor;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context};
use bytes::Bytes;
use image::codecs::png::{self, PngEncoder};
use image::ImageFormat;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use tokio::sync::{watch, Semaphore, SemaphorePermit};
use tokio::task::JoinSet;
use tokio::time::sleep;

use crate::config::Config;
use crate::events::{DownloadSleepingEvent, DownloadSpeedEvent, DownloadTaskEvent};
use crate::extensions::AnyhowErrorToStringChain;
use crate::pica_client::PicaClient;
use crate::types::{ChapterInfo, Comic, DownloadFormat};
use crate::utils::filename_filter;

/// 用于管理下载任务
///
/// 克隆 `DownloadManager` 的开销极小，性能开销几乎可以忽略不计。
/// 可以放心地在多个线程中传递和使用它的克隆副本。
///
/// 具体来说：
/// - `app`的克隆开销很小。
/// - 其他字段都被 `Arc` 包裹，这些字段的克隆操作仅仅是增加引用计数。
#[derive(Clone)]
pub struct DownloadManager {
    app: AppHandle,
    chapter_sem: Arc<Semaphore>,
    img_sem: Arc<Semaphore>,
    byte_per_sec: Arc<AtomicU64>,
    download_tasks: Arc<RwLock<HashMap<String, DownloadTask>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum DownloadTaskState {
    Pending,
    Downloading,
    Paused,
    Cancelled,
    Completed,
    Failed,
}

impl DownloadManager {
    pub fn new(app: AppHandle) -> Self {
        let (chapter_concurrency, img_concurrency) = {
            let config = app.state::<RwLock<Config>>();
            let config = config.read();
            (config.chapter_concurrency, config.img_concurrency)
        };

        let manager = DownloadManager {
            app,
            chapter_sem: Arc::new(Semaphore::new(chapter_concurrency)),
            img_sem: Arc::new(Semaphore::new(img_concurrency)),
            byte_per_sec: Arc::new(AtomicU64::new(0)),
            download_tasks: Arc::new(RwLock::new(HashMap::new())),
        };

        tauri::async_runtime::spawn(manager.clone().emit_download_speed_loop());

        manager
    }

    pub fn create_download_task(&self, comic: Comic, chapter_id: String) -> anyhow::Result<()> {
        use DownloadTaskState::{Downloading, Paused, Pending};
        let mut tasks = self.download_tasks.write();
        if let Some(task) = tasks.get(&chapter_id) {
            let state = *task.state_sender.borrow();
            if matches!(state, Pending | Downloading | Paused) {
                return Err(anyhow!("章节ID为`{chapter_id}`的下载任务已存在"));
            }
        }
        tasks.remove(&chapter_id);
        let task = DownloadTask::new(self.app.clone(), comic, &chapter_id)
            .context(format!("创建章节ID为`{chapter_id}`的下载任务失败"))?;
        tauri::async_runtime::spawn(task.clone().process());
        tasks.insert(chapter_id, task);
        Ok(())
    }

    pub fn pause_download_task(&self, chapter_id: &str) -> anyhow::Result<()> {
        let tasks = self.download_tasks.read();
        let Some(task) = tasks.get(chapter_id) else {
            return Err(anyhow!("未找到章节ID为`{chapter_id}`的下载任务"));
        };
        task.set_state(DownloadTaskState::Paused);
        Ok(())
    }

    pub fn resume_download_task(&self, chapter_id: &str) -> anyhow::Result<()> {
        let tasks = self.download_tasks.read();
        let Some(task) = tasks.get(chapter_id) else {
            return Err(anyhow!("未找到章节ID为`{chapter_id}`的下载任务"));
        };
        task.set_state(DownloadTaskState::Pending);
        Ok(())
    }

    pub fn cancel_download_task(&self, chapter_id: &str) -> anyhow::Result<()> {
        let tasks = self.download_tasks.read();
        let Some(task) = tasks.get(chapter_id) else {
            return Err(anyhow!("未找到章节ID为`{chapter_id}`的下载任务"));
        };
        task.set_state(DownloadTaskState::Cancelled);
        Ok(())
    }

    async fn emit_download_speed_loop(self) {
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        loop {
            interval.tick().await;
            let byte_per_sec = self.byte_per_sec.swap(0, Ordering::Relaxed);
            #[allow(clippy::cast_precision_loss)]
            let mega_byte_per_sec = byte_per_sec as f64 / 1024.0 / 1024.0;
            let speed = format!("{mega_byte_per_sec:.2}MB/s");
            let _ = DownloadSpeedEvent { speed }.emit(&self.app);
        }
    }
}

#[derive(Clone)]
struct DownloadTask {
    app: AppHandle,
    download_manager: DownloadManager,
    comic: Arc<Comic>,
    chapter_info: Arc<ChapterInfo>,
    state_sender: watch::Sender<DownloadTaskState>,
    downloaded_img_count: Arc<AtomicU32>,
    total_img_count: Arc<AtomicU32>,
}

impl DownloadTask {
    pub fn new(app: AppHandle, mut comic: Comic, chapter_id: &str) -> anyhow::Result<Self> {
        comic
            .update_download_dir_fields_by_fmt(&app)
            .context(format!("漫画`{}`更新`download_dir`字段失败", comic.title))?;

        let chapter_info = comic
            .chapter_infos
            .iter()
            .find(|chapter| chapter.chapter_id == chapter_id)
            .cloned()
            .context(format!("未找到章节ID为`{chapter_id}`的章节信息"))?;

        let download_manager = app.state::<DownloadManager>().inner().clone();
        let (state_sender, _) = watch::channel(DownloadTaskState::Pending);

        let task = Self {
            app,
            download_manager,
            comic: Arc::new(comic),
            chapter_info: Arc::new(chapter_info),
            state_sender,
            downloaded_img_count: Arc::new(AtomicU32::new(0)),
            total_img_count: Arc::new(AtomicU32::new(0)),
        };

        Ok(task)
    }

    async fn process(self) {
        self.emit_download_task_create_event();

        let download_comic_task = self.download_chapter();
        tokio::pin!(download_comic_task);

        let mut state_receiver = self.state_sender.subscribe();
        let mut permit = None;
        loop {
            let state_is_downloading = *state_receiver.borrow() == DownloadTaskState::Downloading;
            let state_is_pending = *state_receiver.borrow() == DownloadTaskState::Pending;
            tokio::select! {
                () = &mut download_comic_task, if state_is_downloading && permit.is_some() => break,
                control_flow = self.acquire_chapter_permit(&mut permit), if state_is_pending => {
                    match control_flow {
                        ControlFlow::Continue(()) => continue,
                        ControlFlow::Break(()) => break,
                    }
                },
                _ = state_receiver.changed() => {
                    match self.handle_state_change(&mut permit, &mut state_receiver) {
                        ControlFlow::Continue(()) => continue,
                        ControlFlow::Break(()) => break,
                    }
                }
            }
        }
    }

    async fn download_chapter(&self) {
        let comic_title = &self.comic.title;
        let chapter_title = &self.chapter_info.chapter_title;
        if let Err(err) = self.save_comic_metadata() {
            let err_title = format!("`{comic_title}`保存元数据失败");
            let string_chain = err.to_string_chain();
            tracing::error!(err_title, message = string_chain);

            self.set_state(DownloadTaskState::Failed);
            self.emit_download_task_update_event();

            return;
        }
        // 获取图片链接
        let img_urls = match self.get_img_urls().await {
            Ok(img_urls) => img_urls,
            Err(err) => {
                let err_title = format!("`{comic_title} - {chapter_title}`获取图片链接失败");
                let string_chain = err.to_string_chain();
                tracing::error!(err_title, message = string_chain);

                self.set_state(DownloadTaskState::Failed);
                self.emit_download_task_update_event();

                return;
            }
        };
        // 记录总共需要下载的图片数量
        #[allow(clippy::cast_possible_truncation)]
        self.total_img_count
            .fetch_add(img_urls.len() as u32, Ordering::Relaxed);
        // 创建临时下载目录
        let Some(temp_download_dir) = self.create_temp_download_dir() else {
            return;
        };
        // 清理临时下载目录中与`config.download_format`对不上的文件
        self.clean_temp_download_dir(&temp_download_dir);

        let mut join_set = JoinSet::new();
        for (i, url) in img_urls.into_iter().enumerate() {
            // 创建下载任务
            let temp_download_dir = temp_download_dir.clone();
            let download_img_task = DownloadImgTask::new(self, url, i, temp_download_dir);
            join_set.spawn(download_img_task.process());
        }
        // 等待所有图片下载任务完成
        join_set.join_all().await;
        tracing::trace!(comic_title, chapter_title, "所有图片下载任务完成");
        // 检查此章节的图片是否全部下载成功
        let downloaded_img_count = self.downloaded_img_count.load(Ordering::Relaxed);
        let total_img_count = self.total_img_count.load(Ordering::Relaxed);
        if downloaded_img_count != total_img_count {
            // 此章节的图片未全部下载成功
            let err_title = format!("`{comic_title} - {chapter_title}`下载不完整");
            let err_msg =
                format!("总共有`{total_img_count}`张图片，但只下载了`{downloaded_img_count}`张");
            tracing::error!(err_title, message = err_msg);

            self.set_state(DownloadTaskState::Failed);
            self.emit_download_task_update_event();

            return;
        }
        // 至此，章节的图片全部下载成功
        if let Err(err) = self.rename_temp_download_dir(&temp_download_dir) {
            let err_title = format!("`{comic_title} - {chapter_title}`重命名临时下载目录失败");
            let string_chain = err.to_string_chain();
            tracing::error!(err_title, message = string_chain);

            self.set_state(DownloadTaskState::Failed);
            self.emit_download_task_update_event();

            return;
        };

        if let Err(err) = self.save_chapter_metadata() {
            let err_title = format!("`{comic_title} - {chapter_title}`保存元数据失败");
            let string_chain = err.to_string_chain();
            tracing::error!(err_title, message = string_chain);
        }

        self.sleep_between_chapter().await;
        tracing::info!(comic_title, chapter_title, "章节下载成功");

        self.set_state(DownloadTaskState::Completed);
        self.emit_download_task_update_event();
    }

    fn create_temp_download_dir(&self) -> Option<PathBuf> {
        let comic_title = &self.comic.title;
        let chapter_title = &self.chapter_info.chapter_title;

        let temp_download_dir = match self.chapter_info.get_temp_download_dir() {
            Ok(temp_download_dir) => temp_download_dir,
            Err(err) => {
                let err_title = format!("`{comic_title} - {chapter_title}`获取临时下载目录失败");
                let string_chain = err.to_string_chain();
                tracing::error!(err_title, message = string_chain);

                self.set_state(DownloadTaskState::Failed);
                self.emit_download_task_update_event();

                return None;
            }
        };

        if let Err(err) = std::fs::create_dir_all(&temp_download_dir).map_err(anyhow::Error::from) {
            let err_title = format!(
                "`{comic_title} - {chapter_title}`创建临时下载目录`{temp_download_dir:?}`失败"
            );
            let string_chain = err.to_string_chain();
            tracing::error!(err_title, message = string_chain);

            self.set_state(DownloadTaskState::Failed);
            self.emit_download_task_update_event();

            return None;
        };

        tracing::trace!(
            comic_title,
            chapter_title,
            "创建临时下载目录`{temp_download_dir:?}`成功"
        );

        Some(temp_download_dir)
    }

    async fn get_img_urls(&self) -> anyhow::Result<Vec<String>> {
        let comic_title = &self.comic.title;
        let chapter_title = &self.chapter_info.chapter_title;
        let comic_id = &self.comic.id;
        let chapter_order = self.chapter_info.order;

        let pica_client = self.pica_client();

        let first_page = pica_client
            .get_chapter_img(comic_id, chapter_order, 1)
            .await
            .context("获取第`1`页图片链接失败")?;

        let total_pages = first_page.pages;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let mut page_imgs_pairs = Vec::with_capacity(total_pages as usize);
        page_imgs_pairs.push((1, first_page.docs));

        let mut join_set = JoinSet::new();
        for page in 2..=total_pages {
            let pica_client = pica_client.clone();
            let comic_id = comic_id.clone();

            join_set.spawn(async move {
                let img_page = pica_client
                    .get_chapter_img(&comic_id, chapter_order, page)
                    .await
                    .context(format!("获取第`{page}`页图片链接失败"))?;

                Ok::<_, anyhow::Error>((page, img_page.docs))
            });
        }

        // 逐个处理完成的任务，如果有任务失败，则返回None
        while let Some(join_result) = join_set.join_next().await {
            match join_result {
                Ok(Ok(pair)) => {
                    page_imgs_pairs.push(pair);
                }
                Ok(Err(err)) => return Err(err),
                Err(err) => return Err(anyhow::Error::from(err)),
            }
        }

        page_imgs_pairs.sort_by_key(|(page, _)| *page);
        let img_urls: Vec<String> = page_imgs_pairs
            .into_iter()
            .flat_map(|(_, imgs)| imgs)
            .map(|img| (img.media.file_server, img.media.path))
            .map(|(file_server, path)| format!("{file_server}/static/{path}"))
            .collect();

        tracing::trace!(comic_title, chapter_title, "获取图片链接成功");

        Ok(img_urls)
    }

    fn rename_temp_download_dir(&self, temp_download_dir: &PathBuf) -> anyhow::Result<()> {
        let comic_title = &self.comic.title;
        let chapter_title = &self.chapter_info.chapter_title;
        let chapter_download_dir = self
            .chapter_info
            .chapter_download_dir
            .as_ref()
            .context("`chapter_download_dir`字段为`None`")?;

        if chapter_download_dir.exists() {
            std::fs::remove_dir_all(&chapter_download_dir)
                .context(format!("删除 {chapter_download_dir:?} 失败"))?;
        }

        std::fs::rename(temp_download_dir, &chapter_download_dir).context(format!(
            "将 {temp_download_dir:?} 重命名为 {chapter_download_dir:?} 失败"
        ))?;

        tracing::trace!(
            comic_title,
            chapter_title,
            "重命名临时下载目录`{temp_download_dir:?}`为`{chapter_download_dir:?}`成功"
        );

        Ok(())
    }

    /// 删除临时下载目录中与`config.download_format`对不上的文件
    fn clean_temp_download_dir(&self, temp_download_dir: &Path) {
        let comic_title = &self.comic.title;
        let chapter_title = &self.chapter_info.chapter_title;

        let entries = match std::fs::read_dir(temp_download_dir).map_err(anyhow::Error::from) {
            Ok(entries) => entries,
            Err(err) => {
                let err_title = format!(
                    "`{comic_title}`读取临时下载目录`{}`失败",
                    temp_download_dir.display()
                );
                let string_chain = err.to_string_chain();
                tracing::error!(err_title, message = string_chain);
                return;
            }
        };

        let download_format = self.app.state::<RwLock<Config>>().read().download_format;
        let extension = download_format.extension();
        for path in entries.filter_map(Result::ok).map(|entry| entry.path()) {
            // path有扩展名，且能转换为utf8，并与`config.download_format`一致或是gif，则保留
            let should_keep = path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| Some(ext) == extension);
            if should_keep {
                continue;
            }
            // 否则删除文件
            if let Err(err) = std::fs::remove_file(&path).map_err(anyhow::Error::from) {
                let err_title =
                    format!("`{comic_title}`删除临时下载目录的`{}`失败", path.display());
                let string_chain = err.to_string_chain();
                tracing::error!(err_title, message = string_chain);
            }
        }

        tracing::trace!(
            comic_title,
            chapter_title,
            "清理临时下载目录`{}`成功",
            temp_download_dir.display()
        );
    }

    fn save_comic_metadata(&self) -> anyhow::Result<()> {
        let mut comic = self.comic.as_ref().clone();
        // 将漫画的is_downloaded和comic_download_dir字段设置为None
        // 这样能使这些字段在序列化时被忽略
        comic.is_downloaded = None;
        comic.comic_download_dir = None;
        for chapter in &mut comic.chapter_infos {
            // 将章节的is_downloaded和chapter_download_dir字段设置为None
            // 这样能使这些字段在序列化时被忽略
            chapter.is_downloaded = None;
            chapter.chapter_download_dir = None;
        }

        let comic_download_dir = self
            .comic
            .comic_download_dir
            .as_ref()
            .context("`comic_download_dir`字段为`None`")?;
        let metadata_path = comic_download_dir.join("元数据.json");

        std::fs::create_dir_all(&comic_download_dir)
            .context(format!("创建目录`{comic_download_dir:?}`失败"))?;

        let comic_json = serde_json::to_string_pretty(&comic).context("将Comic序列化为json失败")?;

        std::fs::write(&metadata_path, comic_json)
            .context(format!("写入文件`{metadata_path:?}`失败"))?;

        Ok(())
    }

    fn save_chapter_metadata(&self) -> anyhow::Result<()> {
        let mut chapter_info = self.chapter_info.as_ref().clone();
        // 将is_downloaded和chapter_download_dir字段设置为None
        // 这样能使这些字段在序列化时被忽略
        chapter_info.is_downloaded = None;
        chapter_info.chapter_download_dir = None;

        let chapter_download_dir = self
            .chapter_info
            .chapter_download_dir
            .as_ref()
            .context("`chapter_download_dir`字段为`None`")?;
        let metadata_path = chapter_download_dir.join("章节元数据.json");

        std::fs::create_dir_all(&chapter_download_dir)
            .context(format!("创建目录`{chapter_download_dir:?}`失败"))?;

        let chapter_json =
            serde_json::to_string_pretty(&chapter_info).context("将ChapterInfo序列化为json失败")?;

        std::fs::write(&metadata_path, chapter_json)
            .context(format!("写入文件`{metadata_path:?}`失败"))?;

        Ok(())
    }

    async fn acquire_chapter_permit<'a>(
        &'a self,
        permit: &mut Option<SemaphorePermit<'a>>,
    ) -> ControlFlow<()> {
        let comic_title = &self.comic.title;
        let chapter_title = &self.chapter_info.chapter_title;

        tracing::debug!(comic_title, chapter_title, "章节开始排队");

        self.emit_download_task_update_event();

        *permit = match permit.take() {
            // 如果有permit，则直接用
            Some(permit) => Some(permit),
            // 如果没有permit，则获取permit
            None => match self
                .download_manager
                .chapter_sem
                .acquire()
                .await
                .map_err(anyhow::Error::from)
            {
                Ok(permit) => Some(permit),
                Err(err) => {
                    let err_title =
                        format!("`{comic_title} - {chapter_title}`获取下载章节的permit失败");
                    let string_chain = err.to_string_chain();
                    tracing::error!(err_title, message = string_chain);

                    self.set_state(DownloadTaskState::Failed);
                    self.emit_download_task_update_event();

                    return ControlFlow::Break(());
                }
            },
        };
        // 如果当前任务状态不是`Pending`，则不将任务状态设置为`Downloading`
        if *self.state_sender.borrow() != DownloadTaskState::Pending {
            return ControlFlow::Continue(());
        }
        // 将任务状态设置为`Downloading`
        if let Err(err) = self
            .state_sender
            .send(DownloadTaskState::Downloading)
            .map_err(anyhow::Error::from)
        {
            let err_title = format!("`{comic_title} - {chapter_title}`发送状态`Downloading`失败");
            let string_chain = err.to_string_chain();
            tracing::error!(err_title, message = string_chain);
            return ControlFlow::Break(());
        }
        ControlFlow::Continue(())
    }

    fn handle_state_change<'a>(
        &'a self,
        permit: &mut Option<SemaphorePermit<'a>>,
        state_receiver: &mut watch::Receiver<DownloadTaskState>,
    ) -> ControlFlow<()> {
        let comic_title = &self.comic.title;
        let chapter_title = &self.chapter_info.chapter_title;

        self.emit_download_task_update_event();
        let state = *state_receiver.borrow();
        match state {
            DownloadTaskState::Paused => {
                tracing::debug!(comic_title, chapter_title, "章节暂停中");
                if let Some(permit) = permit.take() {
                    drop(permit);
                };
                ControlFlow::Continue(())
            }
            DownloadTaskState::Cancelled => {
                tracing::debug!(comic_title, chapter_title, "章节取消下载");
                ControlFlow::Break(())
            }
            _ => ControlFlow::Continue(()),
        }
    }

    async fn sleep_between_chapter(&self) {
        let id = &self.chapter_info.chapter_id;
        let mut remaining_sec = self
            .app
            .state::<RwLock<Config>>()
            .read()
            .chapter_download_interval_sec;
        while remaining_sec > 0 {
            // 发送章节休眠事件
            let _ = DownloadSleepingEvent {
                id: id.clone(),
                remaining_sec,
            }
            .emit(&self.app);
            sleep(Duration::from_secs(1)).await;
            remaining_sec -= 1;
        }
    }
    fn set_state(&self, state: DownloadTaskState) {
        let comic_title = &self.comic.title;
        let chapter_title = &self.chapter_info.chapter_title;

        if let Err(err) = self.state_sender.send(state).map_err(anyhow::Error::from) {
            let err_title = format!("`{comic_title} - {chapter_title}`发送状态`{state:?}`失败");
            let string_chain = err.to_string_chain();
            tracing::error!(err_title, message = string_chain);
        }
    }

    fn emit_download_task_update_event(&self) {
        let _ = DownloadTaskEvent::Update {
            chapter_id: self.chapter_info.chapter_id.clone(),
            state: *self.state_sender.borrow(),
            downloaded_img_count: self.downloaded_img_count.load(Ordering::Relaxed),
            total_img_count: self.total_img_count.load(Ordering::Relaxed),
        }
        .emit(&self.app);
    }

    fn emit_download_task_create_event(&self) {
        let _ = DownloadTaskEvent::Create {
            state: *self.state_sender.borrow(),
            comic: Box::new(self.comic.as_ref().clone()),
            chapter_info: Box::new(self.chapter_info.as_ref().clone()),
            downloaded_img_count: self.downloaded_img_count.load(Ordering::Relaxed),
            total_img_count: self.total_img_count.load(Ordering::Relaxed),
        }
        .emit(&self.app);
    }

    fn pica_client(&self) -> PicaClient {
        self.app.state::<PicaClient>().inner().clone()
    }
}

#[derive(Clone)]
struct DownloadImgTask {
    app: AppHandle,
    download_manager: DownloadManager,
    download_task: DownloadTask,
    url: String,
    index: usize,
    temp_download_dir: PathBuf,
}

impl DownloadImgTask {
    pub fn new(
        download_task: &DownloadTask,
        url: String,
        index: usize,
        temp_download_dir: PathBuf,
    ) -> Self {
        Self {
            app: download_task.app.clone(),
            download_manager: download_task.download_manager.clone(),
            download_task: download_task.clone(),
            url,
            index,
            temp_download_dir,
        }
    }

    async fn process(self) {
        let download_img_task = self.download_img();
        tokio::pin!(download_img_task);

        let mut state_receiver = self.download_task.state_sender.subscribe();
        state_receiver.mark_changed();
        let mut permit = None;

        loop {
            let state_is_downloading = *state_receiver.borrow() == DownloadTaskState::Downloading;
            tokio::select! {
                () = &mut download_img_task, if state_is_downloading && permit.is_some() => break,
                control_flow = self.acquire_img_permit(&mut permit), if state_is_downloading && permit.is_none() => {
                    match control_flow {
                        ControlFlow::Continue(()) => continue,
                        ControlFlow::Break(()) => break,
                    }
                },
                _ = state_receiver.changed() => {
                    match self.handle_state_change(&mut permit, &mut state_receiver) {
                        ControlFlow::Continue(()) => continue,
                        ControlFlow::Break(()) => break,
                    }
                }
            }
        }
    }

    async fn download_img(&self) {
        let url = &self.url;
        let comic_title = &self.download_task.comic.title;
        let chapter_title = &self.download_task.chapter_info.chapter_title;

        let download_format = self.app.state::<RwLock<Config>>().read().download_format;
        if let Some(extension) = download_format.extension() {
            // 如果图片已存在，则跳过下载
            let save_path = self
                .temp_download_dir
                .join(format!("{:03}.{extension}", self.index + 1));
            if save_path.exists() {
                tracing::trace!(url, comic_title, chapter_title, "图片已存在，跳过下载");
                self.download_task
                    .downloaded_img_count
                    .fetch_add(1, Ordering::Relaxed);
                self.download_task.emit_download_task_update_event();
                return;
            }
        }

        tracing::trace!(url, comic_title, chapter_title, "开始下载图片");

        let (img_data, img_format) = match self.pica_client().get_img_data_and_format(url).await {
            Ok(data) => data,
            Err(err) => {
                let err_title = format!("下载图片`{url}`失败");
                let string_chain = err.to_string_chain();
                tracing::error!(err_title, message = string_chain);
                return;
            }
        };
        let img_data_len = img_data.len() as u64;

        tracing::trace!(url, comic_title, chapter_title, "图片成功下载到内存");

        // 获取图片格式的扩展名
        let Some(extension) = download_format.extension().or(match img_format {
            ImageFormat::Jpeg => Some("jpg"),
            ImageFormat::Png => Some("png"),
            ImageFormat::WebP => Some("webp"),
            _ => None,
        }) else {
            let err_title = format!("保存图片`{url}`失败");
            let err_msg = format!("`{img_format:?}`格式不支持");
            tracing::error!(err_title, message = err_msg);
            return;
        };

        let save_path = self
            .temp_download_dir
            .join(format!("{:03}.{extension}", self.index + 1));

        // 保存图片
        if let Err(err) = save_img(&save_path, download_format, img_data).await {
            let err_title = format!("保存图片`{}`失败", save_path.display());
            let string_chain = err.to_string_chain();
            tracing::error!(err_title, message = string_chain);
            return;
        }

        tracing::trace!(
            url,
            comic_title,
            chapter_title,
            "图片成功保存到`{save_path:?}`"
        );

        // 记录下载字节数
        self.download_manager
            .byte_per_sec
            .fetch_add(img_data_len, Ordering::Relaxed);

        self.download_task
            .downloaded_img_count
            .fetch_add(1, Ordering::Relaxed);

        self.download_task.emit_download_task_update_event();

        let img_download_interval_sec = self
            .app
            .state::<RwLock<Config>>()
            .read()
            .img_download_interval_sec;
        sleep(Duration::from_secs(img_download_interval_sec)).await;
    }

    async fn acquire_img_permit<'a>(
        &'a self,
        permit: &mut Option<SemaphorePermit<'a>>,
    ) -> ControlFlow<()> {
        let url = &self.url;
        let comic_title = &self.download_task.comic.title;
        let chapter_title = &self.download_task.chapter_info.chapter_title;

        tracing::trace!(comic_title, chapter_title, url, "图片开始排队");

        *permit = match permit.take() {
            // 如果有permit，则直接用
            Some(permit) => Some(permit),
            // 如果没有permit，则获取permit
            None => match self
                .download_manager
                .img_sem
                .acquire()
                .await
                .map_err(anyhow::Error::from)
            {
                Ok(permit) => Some(permit),
                Err(err) => {
                    let err_title =
                        format!("`{comic_title} - {chapter_title}`获取下载图片的permit失败");
                    let string_chain = err.to_string_chain();
                    tracing::error!(err_title, message = string_chain);
                    return ControlFlow::Break(());
                }
            },
        };
        ControlFlow::Continue(())
    }

    fn handle_state_change<'a>(
        &'a self,
        permit: &mut Option<SemaphorePermit<'a>>,
        state_receiver: &mut watch::Receiver<DownloadTaskState>,
    ) -> ControlFlow<()> {
        let url = &self.url;
        let comic_title = &self.download_task.comic.title;
        let chapter_title = &self.download_task.chapter_info.chapter_title;

        let state = *state_receiver.borrow();
        match state {
            DownloadTaskState::Paused => {
                tracing::trace!(comic_title, chapter_title, url, "图片暂停下载");
                if let Some(permit) = permit.take() {
                    drop(permit);
                };
                ControlFlow::Continue(())
            }
            DownloadTaskState::Cancelled => {
                tracing::trace!(comic_title, chapter_title, url, "图片取消下载");
                ControlFlow::Break(())
            }
            _ => ControlFlow::Continue(()),
        }
    }

    fn pica_client(&self) -> PicaClient {
        self.app.state::<PicaClient>().inner().clone()
    }
}

async fn save_img(
    save_path: &Path,
    download_format: DownloadFormat,
    src_img_data: Bytes,
) -> anyhow::Result<()> {
    if DownloadFormat::Original == download_format {
        // 如果下载格式是Original，直接保存
        std::fs::write(save_path, &src_img_data)
            .context(format!("保存图片`{}`失败", save_path.display()))?;
        return Ok(());
    }

    // 图像处理的闭包
    let save_path = save_path.to_path_buf();
    let process_img = move || -> anyhow::Result<()> {
        let src_img = image::load_from_memory(&src_img_data)
            .context("解码图片失败")?
            .to_rgb8();
        // 用来存图片编码后的数据
        let mut dst_img_data = Vec::new();
        match download_format {
            DownloadFormat::Jpeg => {
                src_img.write_to(&mut Cursor::new(&mut dst_img_data), ImageFormat::Jpeg)?;
            }
            DownloadFormat::Png => {
                let encoder = PngEncoder::new_with_quality(
                    Cursor::new(&mut dst_img_data),
                    png::CompressionType::Best,
                    png::FilterType::default(),
                );
                src_img.write_with_encoder(encoder)?;
            }
            DownloadFormat::Webp => {
                src_img.write_to(&mut Cursor::new(&mut dst_img_data), ImageFormat::WebP)?;
            }
            DownloadFormat::Original => {
                return Err(anyhow!("这里不应该出现这个下载格式: `{download_format:?}`"));
            }
        }
        // 保存编码后的图片数据
        std::fs::write(&save_path, dst_img_data)
            .context(format!("保存图片`{}`失败", save_path.display()))?;
        Ok(())
    };

    // 因为图像处理是CPU密集型操作，所以使用rayon并发处理
    let (sender, receiver) = tokio::sync::oneshot::channel::<anyhow::Result<()>>();
    rayon::spawn(move || {
        let _ = sender.send(process_img());
    });
    // 在tokio任务中等待rayon任务的完成，避免阻塞worker threads
    receiver.await?
}

#[derive(Default, Debug, PartialEq, Clone, Serialize, Deserialize, Type)]
pub struct ComicDirNameFmtParams {
    pub comic_id: String,
    pub comic_title: String,
    pub author: String,
}

impl Comic {
    /// 根据fmt更新`comic_download_dir`和`chapter_infos.chapter_download_dir`字段
    fn update_download_dir_fields_by_fmt(&mut self, app: &AppHandle) -> anyhow::Result<()> {
        if self.chapter_infos.is_empty() {
            return Err(anyhow!("没有章节信息，无法更新下载目录字段"));
        }

        let mut first_chapter_download_dir = None;

        for chapter_info in &mut self.chapter_infos {
            let chapter_title = &chapter_info.chapter_title;

            let dir_fmt_params = DirFmtParams {
                comic_id: self.id.clone(),
                comic_title: self.title.clone(),
                author: self.author.clone(),
                chapter_id: chapter_info.chapter_id.clone(),
                chapter_title: chapter_info.chapter_title.clone(),
                order: chapter_info.order,
            };

            let chapter_download_dir =
                ChapterInfo::get_chapter_download_dir_by_fmt(app, &dir_fmt_params)
                    .context(format!("章节`{chapter_title}`根据fmt获取章节下载目录失败"))?;

            if first_chapter_download_dir.is_none() {
                first_chapter_download_dir = Some(chapter_download_dir.clone());
            }

            chapter_info.chapter_download_dir = Some(chapter_download_dir);
        }

        let Some(first_chapter_download_dir) = first_chapter_download_dir else {
            return Err(anyhow!(
                "处理完所有章节后first_chapter_download_dir仍然为None"
            ));
        };

        let comic_download_dir = first_chapter_download_dir.parent().context(format!(
            "第一个章节下载目录`{first_chapter_download_dir:?}`没有父目录"
        ))?;

        self.comic_download_dir = Some(comic_download_dir.to_path_buf());

        Ok(())
    }
}

#[derive(Default, Debug, PartialEq, Clone, Serialize, Deserialize, Type)]
pub struct DirFmtParams {
    pub comic_id: String,
    pub comic_title: String,
    pub author: String,
    pub chapter_id: String,
    pub chapter_title: String,
    pub order: i64,
}

impl ChapterInfo {
    fn get_chapter_download_dir_by_fmt(
        app: &AppHandle,
        fmt_params: &DirFmtParams,
    ) -> anyhow::Result<PathBuf> {
        use strfmt::strfmt;

        let json_value =
            serde_json::to_value(fmt_params).context("将DirFmtParams转为serde_json::Value失败")?;

        let json_map = json_value.as_object().context("DirFmtParams不是JSON对象")?;

        let vars: HashMap<String, String> = json_map
            .into_iter()
            .map(|(k, v)| {
                let key = k.clone();
                let value = match v {
                    serde_json::Value::String(s) => s.clone(),
                    _ => v.to_string(),
                };
                (key, value)
            })
            .collect();

        let (download_dir, dir_fmt) = {
            let config = app.state::<RwLock<Config>>();
            let config = config.read();
            (config.download_dir.clone(), config.dir_fmt.clone())
        };

        let dir_fmt_parts: Vec<&str> = dir_fmt.split('/').collect();

        let mut dir_names = Vec::new();
        for fmt in dir_fmt_parts {
            let dir_name = strfmt(fmt, &vars).context("格式化目录名失败")?;
            let dir_name = filename_filter(&dir_name);
            if !dir_name.is_empty() {
                dir_names.push(dir_name);
            }
        }

        if dir_names.len() < 2 {
            let err_msg =
                "配置中的下载目录格式至少要有两个层级，例如：{comic_title}/{chapter_title}";
            return Err(anyhow!(err_msg));
        }
        // 将格式化后的目录名拼接成完整的目录路径
        let mut chapter_download_dir = download_dir;
        for dir_name in dir_names {
            chapter_download_dir = chapter_download_dir.join(dir_name);
        }

        Ok(chapter_download_dir)
    }

    fn get_temp_download_dir(&self) -> anyhow::Result<PathBuf> {
        let chapter_download_dir = self
            .chapter_download_dir
            .as_ref()
            .context("`chapter_download_dir`字段为`None`")?;

        let chapter_download_dir_name = self
            .get_chapter_download_dir_name()
            .context("获取章节下载目录名失败")?;

        let parent = chapter_download_dir
            .parent()
            .context(format!("`{chapter_download_dir:?}`的父目录不存在"))?;

        let temp_download_dir = parent.join(format!(".下载中-{chapter_download_dir_name}"));
        Ok(temp_download_dir)
    }
}
