use std::collections::HashMap;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use tokio::sync::{watch, Semaphore, SemaphorePermit};
use tokio::task::JoinSet;

use crate::config::Config;
use crate::events::{DownloadSpeedEvent, DownloadTaskEvent};
use crate::extensions::AnyhowErrorToStringChain;
use crate::pica_client::PicaClient;
use crate::types::{ChapterInfo, Comic};

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
        let manager = DownloadManager {
            app,
            chapter_sem: Arc::new(Semaphore::new(3)),
            img_sem: Arc::new(Semaphore::new(40)),
            byte_per_sec: Arc::new(AtomicU64::new(0)),
            download_tasks: Arc::new(RwLock::new(HashMap::new())),
        };

        tauri::async_runtime::spawn(manager.clone().emit_download_speed_loop());

        manager
    }

    pub fn create_download_task(&self, comic: Comic, chapter_id: String) -> anyhow::Result<()> {
        use DownloadTaskState::{Downloading, Paused, Pending};
        let chapter_info = comic
            .chapter_infos
            .iter()
            .find(|chapter| chapter.chapter_id == chapter_id)
            .cloned()
            .context(format!("未找到章节ID为`{chapter_id}`的章节信息"))?;
        let mut tasks = self.download_tasks.write();
        if let Some(task) = tasks.get(&chapter_id) {
            let state = *task.state_sender.borrow();
            if matches!(state, Pending | Downloading | Paused) {
                return Err(anyhow!("章节ID为`{chapter_id}`的下载任务已存在"));
            }
        }
        tasks.remove(&chapter_id);
        let task = DownloadTask::new(self.app.clone(), comic, chapter_info);
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
    pub fn new(app: AppHandle, comic: Comic, chapter_info: ChapterInfo) -> Self {
        let download_manager = app.state::<DownloadManager>().inner().clone();
        let (state_sender, _) = watch::channel(DownloadTaskState::Pending);
        Self {
            app,
            download_manager,
            comic: Arc::new(comic),
            chapter_info: Arc::new(chapter_info),
            state_sender,
            downloaded_img_count: Arc::new(AtomicU32::new(0)),
            total_img_count: Arc::new(AtomicU32::new(0)),
        }
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
        // 获取保存路径
        let Some(save_paths) = self.get_save_paths(&img_urls, &temp_download_dir) else {
            return;
        };
        // 清理临时下载目录中与`config.download_format`对不上的文件
        if let Err(err) = self.clean_temp_download_dir(&temp_download_dir, &save_paths) {
            let err_title = format!("`{comic_title} - {chapter_title}`清理临时下载目录失败");
            let string_chain = err.to_string_chain();
            tracing::error!(err_title, message = string_chain);

            self.set_state(DownloadTaskState::Failed);
            self.emit_download_task_update_event();

            return;
        }

        let mut join_set = JoinSet::new();
        for (url, save_path) in img_urls.into_iter().zip(save_paths.into_iter()) {
            // 创建下载任务
            let download_img_task = DownloadImgTask::new(self, url, save_path);
            join_set.spawn(download_img_task.process());
        }
        // 等待所有图片下载任务完成
        join_set.join_all().await;
        tracing::trace!(comic_title, chapter_title, "所有图片下载任务完成");
        // 等待一段时间再下载下一章节
        let chapter_download_interval = self
            .app
            .state::<RwLock<Config>>()
            .read()
            .chapter_download_interval;
        tokio::time::sleep(Duration::from_secs(chapter_download_interval)).await;
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

        tracing::info!(comic_title, chapter_title, "章节下载成功");

        self.set_state(DownloadTaskState::Completed);
        self.emit_download_task_update_event();
    }

    fn get_save_paths(&self, urls: &[String], temp_download_dir: &Path) -> Option<Vec<PathBuf>> {
        let comic_title = &self.comic.title;
        let chapter_title = &self.chapter_info.chapter_title;

        let mut save_paths = Vec::with_capacity(urls.len());

        for (i, url) in urls.iter().enumerate() {
            let extension = match self.get_extension_from_url(url) {
                Ok(extension) => extension,
                Err(err) => {
                    let err_title =
                        format!("`{comic_title} - {chapter_title}`获取`{url}`的后缀名失败");
                    let string_chain = err.to_string_chain();
                    tracing::error!(err_title, message = string_chain);

                    self.set_state(DownloadTaskState::Failed);
                    self.emit_download_task_update_event();

                    return None;
                }
            };
            save_paths.push(temp_download_dir.join(format!("{:03}.{extension}", i + 1)));
        }

        tracing::trace!(comic_title, chapter_title, "获取保存路径成功");

        Some(save_paths)
    }

    fn create_temp_download_dir(&self) -> Option<PathBuf> {
        let comic_title = &self.comic.title;
        let chapter_title = &self.chapter_info.chapter_title;

        let temp_download_dir = self
            .chapter_info
            .get_temp_download_dir(&self.app, &self.comic);
        if let Err(err) = std::fs::create_dir_all(&temp_download_dir).map_err(anyhow::Error::from) {
            let err_title =
                format!("`{comic_title} - {chapter_title}`创建目录`{temp_download_dir:?}`失败");
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
            .get_chapter_download_dir(&self.app, &self.comic);

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

    fn get_extension_from_url(&self, url: &str) -> anyhow::Result<String> {
        let download_format = self.app.state::<RwLock<Config>>().read().download_format;
        if let Some(extension) = download_format.extension() {
            // 如果不是Original格式，直接返回
            return Ok(extension.to_string());
        }
        // 如果是Original格式，从url中提取后缀名
        let extension = url
            .rsplit('.')
            .next()
            .context(format!("无法从`{url}`中提取出后缀名"))?
            .to_string();
        Ok(extension)
    }

    /// 删除临时下载目录中与`config.download_format`对不上的文件
    fn clean_temp_download_dir(
        &self,
        temp_download_dir: &Path,
        save_paths: &[PathBuf],
    ) -> anyhow::Result<()> {
        let comic_title = &self.comic.title;
        let chapter_title = &self.chapter_info.chapter_title;

        let entries = std::fs::read_dir(temp_download_dir)
            .context(format!("读取临时下载目录`{temp_download_dir:?}`失败"))?;

        for path in entries.filter_map(Result::ok).map(|entry| entry.path()) {
            if !save_paths.contains(&path) {
                std::fs::remove_file(&path).context(format!("删除临时下载目录的`{path:?}`失败"))?;
            }
        }

        tracing::trace!(
            comic_title,
            chapter_title,
            "清理临时下载目录`{temp_download_dir:?}`成功"
        );

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
    save_path: PathBuf,
}

impl DownloadImgTask {
    pub fn new(download_task: &DownloadTask, url: String, save_path: PathBuf) -> Self {
        Self {
            app: download_task.app.clone(),
            download_manager: download_task.download_manager.clone(),
            download_task: download_task.clone(),
            url,
            save_path,
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
        let save_path = &self.save_path;
        let comic_title = &self.download_task.comic.title;
        let chapter_title = &self.download_task.chapter_info.chapter_title;

        if save_path.exists() {
            // 如果图片已经存在，直接返回
            self.download_task
                .downloaded_img_count
                .fetch_add(1, Ordering::Relaxed);

            tracing::trace!(url, comic_title, chapter_title, "图片已存在，跳过下载");

            return;
        }

        tracing::trace!(url, comic_title, chapter_title, "开始下载图片");

        let img_data = match self.pica_client().get_img_data(url).await {
            Ok(data) => data,
            Err(err) => {
                let err_title = format!("下载图片`{url}`失败");
                let string_chain = err.to_string_chain();
                tracing::error!(err_title, message = string_chain);
                return;
            }
        };

        tracing::trace!(url, comic_title, chapter_title, "图片成功下载到内存");

        // 保存图片
        if let Err(err) = std::fs::write(save_path, &img_data).map_err(anyhow::Error::from) {
            let err_title = format!("保存图片`{save_path:?}`失败");
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
            .fetch_add(img_data.len() as u64, Ordering::Relaxed);

        self.download_task
            .downloaded_img_count
            .fetch_add(1, Ordering::Relaxed);

        self.download_task.emit_download_task_update_event();
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
