use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context};
use bytes::Bytes;
use parking_lot::{Mutex, RwLock};
use reqwest::StatusCode;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::RetryTransientMiddleware;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use tokio::sync::mpsc::Receiver;
use tokio::sync::{mpsc, Semaphore};
use tokio::task::JoinSet;

use crate::config::Config;
use crate::events::DownloadEvent;
use crate::extensions::AnyhowErrorToStringChain;
use crate::pica_client::PicaClient;
use crate::types::ChapterInfo;

/// 用于管理下载任务
///
/// 克隆 `DownloadManager` 的开销极小，性能开销几乎可以忽略不计。
/// 可以放心地在多个线程中传递和使用它的克隆副本。
///
/// 具体来说：
/// - `client`和`app`的克隆开销很小。
/// - 其他字段都被 `Arc` 包裹，这些字段的克隆操作仅仅是增加引用计数。
#[derive(Clone)]
pub struct DownloadManager {
    client: ClientWithMiddleware,
    app: AppHandle,
    sender: Arc<mpsc::Sender<ChapterInfo>>,
    chapter_sem: Arc<Semaphore>,
    img_sem: Arc<Semaphore>,
    byte_per_sec: Arc<AtomicU64>,
    downloaded_image_count: Arc<AtomicU32>,
    total_image_count: Arc<AtomicU32>,
}

impl DownloadManager {
    pub fn new(app: AppHandle) -> Self {
        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(2);
        let client = ClientBuilder::new(reqwest::Client::new())
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();

        let (sender, receiver) = mpsc::channel::<ChapterInfo>(32);
        let manager = DownloadManager {
            client,
            app,
            sender: Arc::new(sender),
            chapter_sem: Arc::new(Semaphore::new(3)),
            img_sem: Arc::new(Semaphore::new(40)),
            byte_per_sec: Arc::new(AtomicU64::new(0)),
            downloaded_image_count: Arc::new(AtomicU32::new(0)),
            total_image_count: Arc::new(AtomicU32::new(0)),
        };

        // TODO: 改用tauri::async_runtime::spawn
        tokio::spawn(manager.clone().log_download_speed());
        tokio::spawn(manager.clone().receiver_loop(receiver));

        manager
    }

    pub async fn submit_chapter(&self, chapter_info: ChapterInfo) -> anyhow::Result<()> {
        Ok(self.sender.send(chapter_info).await?)
    }

    #[allow(clippy::cast_precision_loss)]
    async fn log_download_speed(self) {
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        loop {
            interval.tick().await;
            let byte_per_sec = self.byte_per_sec.swap(0, Ordering::Relaxed);
            let mega_byte_per_sec = byte_per_sec as f64 / 1024.0 / 1024.0;
            let speed = format!("{mega_byte_per_sec:.2}MB/s");
            let _ = DownloadEvent::Speed { speed }.emit(&self.app);
        }
    }

    async fn receiver_loop(self, mut receiver: Receiver<ChapterInfo>) {
        while let Some(chapter) = receiver.recv().await {
            let manager = self.clone();
            tokio::spawn(manager.process_chapter(chapter));
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::too_many_lines)]
    // TODO: 重构这个函数，减少行数
    async fn process_chapter(self, chapter_info: ChapterInfo) {
        // 发送章节排队事件
        let _ = DownloadEvent::ChapterPending {
            chapter_id: chapter_info.chapter_id.clone(),
            title: chapter_info.chapter_title.clone(),
        }
        .emit(&self.app);

        let pica_client = self.app.state::<PicaClient>().inner().clone();
        let images = Arc::new(Mutex::new(vec![]));
        // 先获取该章节的第一页图片
        let first_page = match pica_client
            .get_chapter_image(&chapter_info.comic_id, chapter_info.order, 1)
            .await
        {
            Ok(first_page) => first_page,
            Err(err) => {
                let comic_title = &chapter_info.comic_title;
                let chapter_order = chapter_info.order;
                let chapter_title = &chapter_info.chapter_title;
                let err = err.context(format!(
                    "获取`{comic_title}`第`{chapter_order}`章节`{chapter_title}`的第`1`页图片失败"
                ));
                // 发送章节结束事件
                let _ = DownloadEvent::ChapterEnd {
                    chapter_id: chapter_info.chapter_id.clone(),
                    err_msg: Some(err.to_string_chain()),
                }
                .emit(&self.app);
                return;
            }
        };
        images.lock().push((1, first_page.docs));
        // 根据第一页返回的总页数，创建获取剩下页数图片的任务
        let total_pages = first_page.pages;
        let mut join_set = JoinSet::new();
        // 从第二页开始获取
        for page in 2..=total_pages {
            let pica_client = pica_client.clone();
            let images = images.clone();
            let comic_id = chapter_info.comic_id.clone();
            let comic_title = chapter_info.comic_title.clone();
            let chapter_id = chapter_info.chapter_id.clone();
            let chapter_title = chapter_info.chapter_title.clone();
            let chapter_order = chapter_info.order;
            let app = self.app.clone();
            join_set.spawn(async move {
                let image_page = match pica_client
                    .get_chapter_image(&comic_id, chapter_order, page)
                    .await
                {
                    Ok(image_page) => image_page,
                    Err(err) => {
                        let err = err.context(format!(
                            "获取`{comic_title}`第`{chapter_order}`章`{chapter_title}`的第`{page}`页图片失败"
                        ));
                        // 发送章节结束事件
                        let _ = DownloadEvent::ChapterEnd {
                            chapter_id,
                            err_msg: Some(err.to_string_chain()),
                        }
                        .emit(&app);
                        return;
                    }
                };

                images.lock().push((page, image_page.docs));
            });
        }
        // 等待所有获取图片的任务完成
        join_set.join_all().await;
        let mut images = std::mem::take(&mut *images.lock());
        images.sort_by_key(|(page, _)| *page);
        // 构造图片下载链接
        let urls: Vec<String> = images
            .into_iter()
            .flat_map(|(_, images)| images)
            .map(|image| (image.media.file_server, image.media.path))
            .map(|(file_server, path)| format!("{file_server}/static/{path}"))
            .collect();

        let total = urls.len() as u32;
        // 记录总共需要下载的图片数量
        self.total_image_count.fetch_add(total, Ordering::Relaxed);
        let downloaded_count = Arc::new(AtomicU32::new(0));
        let mut join_set = JoinSet::new();
        // 限制同时下载的章节数量
        let permit = match self
            .chapter_sem
            .acquire()
            .await
            .map_err(anyhow::Error::from)
        {
            Ok(permit) => permit,
            Err(err) => {
                let err = err.context("获取下载章节的semaphore失败");
                // 发送章节结束事件
                let _ = DownloadEvent::ChapterEnd {
                    chapter_id: chapter_info.chapter_id.clone(),
                    err_msg: Some(err.to_string_chain()),
                }
                .emit(&self.app);
                return;
            }
        };
        // 创建临时下载目录
        let chapter_temp_download_dir = chapter_info.get_chapter_temp_download_dir(&self.app);
        if let Err(err) =
            std::fs::create_dir_all(&chapter_temp_download_dir).map_err(anyhow::Error::from)
        {
            let err = err.context(format!("创建目录`{chapter_temp_download_dir:?}`失败"));
            let _ = DownloadEvent::ChapterEnd {
                chapter_id: chapter_info.chapter_id.clone(),
                err_msg: Some(err.to_string_chain()),
            }
            .emit(&self.app);
            return;
        };
        // 发送章节开始下载事件
        let _ = DownloadEvent::ChapterStart {
            chapter_id: chapter_info.chapter_id.clone(),
            title: chapter_info.chapter_title.clone(),
            total,
        }
        .emit(&self.app);
        for (i, url) in urls.iter().enumerate() {
            let manager = self.clone();
            let chapter_id = chapter_info.chapter_id.clone();
            let save_path = chapter_temp_download_dir.join(format!("{:03}.jpg", i + 1));
            let url = url.clone();
            let downloaded_count = downloaded_count.clone();
            // 创建下载任务
            join_set.spawn(manager.download_image(url, save_path, chapter_id, downloaded_count));
        }
        // 逐一处理完成的下载任务
        while let Some(Ok(())) = join_set.join_next().await {
            self.downloaded_image_count.fetch_add(1, Ordering::Relaxed);
            let downloaded_image_count = self.downloaded_image_count.load(Ordering::Relaxed);
            let total_image_count = self.total_image_count.load(Ordering::Relaxed);
            // 发送整体下载进度事件
            #[allow(clippy::cast_lossless)]
            let percentage = downloaded_image_count as f64 / total_image_count as f64 * 100.0;
            let _ = DownloadEvent::OverallUpdate {
                downloaded_image_count,
                total_image_count,
                percentage,
            }
            .emit(&self.app);
        }
        let download_interval = self
            .app
            .state::<RwLock<Config>>()
            .read()
            .chapter_download_interval;
        // 等待一段时间再下载下一章节
        tokio::time::sleep(Duration::from_secs(download_interval)).await;
        drop(permit);
        // 如果DownloadManager所有图片全部都已下载(无论成功或失败)，则清空下载进度
        let downloaded_image_count = self.downloaded_image_count.load(Ordering::Relaxed);
        let total_image_count = self.total_image_count.load(Ordering::Relaxed);
        if downloaded_image_count == total_image_count {
            self.downloaded_image_count.store(0, Ordering::Relaxed);
            self.total_image_count.store(0, Ordering::Relaxed);
        }
        // 检查此章节的图片是否全部下载成功
        let downloaded_count = downloaded_count.load(Ordering::Relaxed);
        if downloaded_count != total {
            // 此章节的图片未全部下载成功
            let comic_title = &chapter_info.comic_title;
            let chapter_title = &chapter_info.chapter_title;
            let err_msg = Some(format!(
                "`{comic_title}`的`{chapter_title}`章节总共有`{total}`张图片，但只下载了`{downloaded_count}`张"
            ));
            // 发送章节结束事件
            let _ = DownloadEvent::ChapterEnd {
                chapter_id: chapter_info.chapter_id.clone(),
                err_msg,
            }
            .emit(&self.app);
            return;
        }
        // 此章节的图片全部下载成功
        let err_msg = match self
            .rename_chapter_temp_download_dir(&chapter_info, &chapter_temp_download_dir)
        {
            Ok(()) => None,
            Err(err) => Some(err.to_string_chain()),
        };
        // 发送章节结束事件
        let _ = DownloadEvent::ChapterEnd {
            chapter_id: chapter_info.chapter_id.clone(),
            err_msg,
        }
        .emit(&self.app);
    }

    fn rename_chapter_temp_download_dir(
        &self,
        chapter_info: &ChapterInfo,
        chapter_temp_download_dir: &PathBuf,
    ) -> anyhow::Result<()> {
        let chapter_download_dir = chapter_info.get_chapter_download_dir(&self.app);

        if chapter_download_dir.exists() {
            std::fs::remove_dir_all(&chapter_download_dir)
                .context(format!("删除 {chapter_download_dir:?} 失败"))?;
        }

        std::fs::rename(chapter_temp_download_dir, &chapter_download_dir).context(format!(
            "将 {chapter_temp_download_dir:?} 重命名为 {chapter_download_dir:?} 失败"
        ))?;

        Ok(())
    }

    async fn download_image(
        self,
        url: String,
        save_path: PathBuf,
        chapter_id: String,
        downloaded_count: Arc<AtomicU32>,
    ) {
        // 下载图片
        let permit = match self.img_sem.acquire().await.map_err(anyhow::Error::from) {
            Ok(permit) => permit,
            Err(err) => {
                let err = err.context("获取下载图片的semaphore失败");
                // 发送下载图片失败事件
                let _ = DownloadEvent::ImageError {
                    chapter_id,
                    url,
                    err_msg: err.to_string_chain(),
                }
                .emit(&self.app);
                return;
            }
        };
        let image_data = match self.get_image_bytes(&url).await {
            Ok(data) => data,
            Err(err) => {
                let err = err.context(format!("下载图片`{url}`失败"));
                // 发送下载图片失败事件
                let _ = DownloadEvent::ImageError {
                    chapter_id,
                    url,
                    err_msg: err.to_string_chain(),
                }
                .emit(&self.app);
                return;
            }
        };
        drop(permit);
        // 保存图片
        if let Err(err) = std::fs::write(&save_path, &image_data).map_err(anyhow::Error::from) {
            let err = err.context(format!("保存图片`{save_path:?}`失败"));
            // 发送下载图片失败事件
            let _ = DownloadEvent::ImageError {
                chapter_id,
                url,
                err_msg: err.to_string_chain(),
            }
            .emit(&self.app);
            return;
        }
        // 记录下载字节数
        self.byte_per_sec
            .fetch_add(image_data.len() as u64, Ordering::Relaxed);
        // 更新章节下载进度
        let downloaded_count = downloaded_count.fetch_add(1, Ordering::Relaxed) + 1;
        let save_path = save_path.to_string_lossy().to_string();
        // 发送下载图片成功事件
        let _ = DownloadEvent::ImageSuccess {
            chapter_id,
            url: save_path,
            downloaded_count,
        }
        .emit(&self.app);
    }

    // TODO: 将发送获取图片请求的逻辑移到PicaClient中
    async fn get_image_bytes(&self, url: &str) -> anyhow::Result<Bytes> {
        let http_res = self.client.get(url).send().await?;

        let status = http_res.status();
        if status != StatusCode::OK {
            let text = http_res.text().await?;
            let err = anyhow!("下载图片`{url}`失败，预料之外的状态码: {text}");
            return Err(err);
        }

        let image_data = http_res.bytes().await?;

        Ok(image_data)
    }
}
