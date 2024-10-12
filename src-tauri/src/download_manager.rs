use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use anyhow::{anyhow, Context};
use bytes::Bytes;
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
use crate::events;
use crate::events::{DownloadSpeedEvent, DownloadSpeedEventPayload};
use crate::extensions::{AnyhowErrorToStringChain, IgnoreLockPoison, IgnoreRwLockPoison};
use crate::pica_client::PicaClient;
use crate::types::Episode;

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
    sender: Arc<mpsc::Sender<Episode>>,
    ep_sem: Arc<Semaphore>,
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

        let (sender, receiver) = mpsc::channel::<Episode>(32);
        let manager = DownloadManager {
            client,
            app,
            sender: Arc::new(sender),
            ep_sem: Arc::new(Semaphore::new(3)),
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

    pub async fn submit_episode(&self, ep: Episode) -> anyhow::Result<()> {
        Ok(self.sender.send(ep).await?)
    }

    #[allow(clippy::cast_precision_loss)]
    async fn log_download_speed(self) {
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        loop {
            interval.tick().await;
            let byte_per_sec = self.byte_per_sec.swap(0, Ordering::Relaxed);
            let mega_byte_per_sec = byte_per_sec as f64 / 1024.0 / 1024.0;
            let speed = format!("{mega_byte_per_sec:.2}MB/s");
            emit_download_speed_event(&self.app, speed);
        }
    }

    async fn receiver_loop(self, mut receiver: Receiver<Episode>) {
        while let Some(ep) = receiver.recv().await {
            let manager = self.clone();
            tokio::spawn(manager.process_episode(ep));
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    //TODO: 这里不能用anyhow::Result<()>和`?`，否则会导致错误信息被忽略
    async fn process_episode(self, ep: Episode) -> anyhow::Result<()> {
        emit_pending_event(&self.app, ep.ep_id.clone(), ep.ep_title.clone());

        let pica_client = self.app.state::<PicaClient>().inner().clone();
        let images = Arc::new(Mutex::new(vec![]));
        let first_page = pica_client
            .get_episode_image(&ep.comic_id, ep.order, 1)
            .await
            .unwrap();
        images.lock_or_panic().push((1, first_page.docs));

        let total_pages = first_page.pages;
        let mut join_set = JoinSet::new();

        for page in 2..=total_pages {
            let pica_client = pica_client.clone();
            let images = images.clone();
            let comic_id = ep.comic_id.clone();
            let ep_order = ep.order;
            join_set.spawn(async move {
                let image_page = pica_client
                    .get_episode_image(&comic_id, ep_order, page)
                    .await
                    .unwrap();
                images.lock_or_panic().push((page, image_page.docs));
            });
        }
        // 等待所有章节图片链接获取完成
        join_set.join_all().await;
        let mut images = std::mem::take(&mut *images.lock().unwrap());
        images.sort_by_key(|(page, _)| *page);
        // 构造图片下载链接
        let urls: Vec<String> = images
            .into_iter()
            .flat_map(|(_, images)| images)
            .map(|image| (image.media.file_server, image.media.path))
            .map(|(file_server, path)| format!("{file_server}/static/{path}"))
            .collect();
        // 创建临时下载目录
        let temp_download_dir = get_temp_download_dir(&self.app, &ep);
        std::fs::create_dir_all(&temp_download_dir)
            .context(format!("创建目录`{temp_download_dir:?}`失败"))?;

        let total = urls.len() as u32;
        // 记录总共需要下载的图片数量
        self.total_image_count.fetch_add(total, Ordering::Relaxed);
        let downloaded_count = Arc::new(AtomicU32::new(0));
        let mut join_set = JoinSet::new();
        // 限制同时下载的章节数量
        let permit = self.ep_sem.acquire().await?;
        emit_start_event(&self.app, ep.ep_id.clone(), ep.ep_title.clone(), total);
        for (i, url) in urls.iter().enumerate() {
            let manager = self.clone();
            let ep_id = ep.ep_id.clone();
            let save_path = temp_download_dir.join(format!("{:03}.jpg", i + 1));
            let url = url.clone();
            let downloaded_count = downloaded_count.clone();
            // 创建下载任务
            join_set.spawn(manager.download_image(url, save_path, ep_id, downloaded_count));
        }
        // 逐一处理完成的下载任务
        while let Some(completed_task) = join_set.join_next().await {
            completed_task?;
            self.downloaded_image_count.fetch_add(1, Ordering::Relaxed);
            let downloaded_image_count = self.downloaded_image_count.load(Ordering::Relaxed);
            let total_image_count = self.total_image_count.load(Ordering::Relaxed);
            // 更新下载进度
            emit_update_overall_progress_event(
                &self.app,
                downloaded_image_count,
                total_image_count,
            );
        }
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
        if downloaded_count == total {
            // 下载成功，则把临时目录重命名为正式目录
            if let Some(parent) = temp_download_dir.parent() {
                let download_dir = parent.join(&ep.ep_title);
                std::fs::rename(&temp_download_dir, &download_dir).context(format!(
                    "将`{temp_download_dir:?}`重命名为`{download_dir:?}`失败"
                ))?;
            }
            emit_end_event(&self.app, ep.ep_id.clone(), None);
        } else {
            let ep_title = &ep.ep_title;
            let err_msg = Some(format!(
                "`{ep_title}`总共有`{total}`张图片，但只下载了`{downloaded_count}`张"
            ));
            emit_end_event(&self.app, ep.ep_id.clone(), err_msg);
        };
        Ok(())
    }

    async fn download_image(
        self,
        url: String,
        save_path: PathBuf,
        ep_id: String,
        downloaded_count: Arc<AtomicU32>,
    ) {
        // 下载图片
        let permit = match self.img_sem.acquire().await.map_err(anyhow::Error::from) {
            Ok(permit) => permit,
            Err(err) => {
                let err = err.context("获取下载图片的semaphore失败");
                emit_error_event(&self.app, ep_id, url, err.to_string_chain());
                return;
            }
        };
        let image_data = match self.get_image_bytes(&url).await {
            Ok(data) => data,
            Err(err) => {
                let err = err.context(format!("下载图片`{url}`失败"));
                emit_error_event(&self.app, ep_id, url, err.to_string_chain());
                return;
            }
        };
        drop(permit);
        // 保存图片
        if let Err(err) = std::fs::write(&save_path, &image_data).map_err(anyhow::Error::from) {
            let err = err.context(format!("保存图片`{save_path:?}`失败"));
            emit_error_event(&self.app, ep_id, url, err.to_string_chain());
            return;
        }
        // 记录下载字节数
        self.byte_per_sec
            .fetch_add(image_data.len() as u64, Ordering::Relaxed);
        // 更新章节下载进度
        let downloaded_count = downloaded_count.fetch_add(1, Ordering::Relaxed) + 1;
        let save_path = save_path.to_string_lossy().to_string();
        emit_success_event(&self.app, ep_id, save_path, downloaded_count);
    }

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

fn get_temp_download_dir(app: &AppHandle, ep: &Episode) -> PathBuf {
    app.state::<RwLock<Config>>()
        .read_or_panic()
        .download_dir
        .join(&ep.comic_title)
        .join(format!(".下载中-{}", ep.ep_title)) // 以 `.下载中-` 开头，表示是临时目录
}

fn emit_start_event(app: &AppHandle, ep_id: String, title: String, total: u32) {
    let payload = events::DownloadEpisodeStartEventPayload {
        ep_id,
        title,
        total,
    };
    let event = events::DownloadEpisodeStartEvent(payload);
    let _ = event.emit(app);
}

fn emit_pending_event(app: &AppHandle, ep_id: String, title: String) {
    let payload = events::DownloadEpisodePendingEventPayload { ep_id, title };
    let event = events::DownloadEpisodePendingEvent(payload);
    let _ = event.emit(app);
}

fn emit_success_event(app: &AppHandle, ep_id: String, url: String, downloaded_count: u32) {
    let payload = events::DownloadImageSuccessEventPayload {
        ep_id,
        url,
        downloaded_count,
    };
    let event = events::DownloadImageSuccessEvent(payload);
    let _ = event.emit(app);
}

fn emit_error_event(app: &AppHandle, ep_id: String, url: String, err_msg: String) {
    let payload = events::DownloadImageErrorEventPayload {
        ep_id,
        url,
        err_msg,
    };
    let event = events::DownloadImageErrorEvent(payload);
    let _ = event.emit(app);
}

fn emit_end_event(app: &AppHandle, ep_id: String, err_msg: Option<String>) {
    let payload = events::DownloadEpisodeEndEventPayload { ep_id, err_msg };
    let event = events::DownloadEpisodeEndEvent(payload);
    let _ = event.emit(app);
}

#[allow(clippy::cast_lossless)]
fn emit_update_overall_progress_event(
    app: &AppHandle,
    downloaded_image_count: u32,
    total_image_count: u32,
) {
    let percentage: f64 = downloaded_image_count as f64 / total_image_count as f64 * 100.0;
    let payload = events::UpdateOverallDownloadProgressEventPayload {
        downloaded_image_count,
        total_image_count,
        percentage,
    };
    let event = events::UpdateOverallDownloadProgressEvent(payload);
    let _ = event.emit(app);
}

fn emit_download_speed_event(app: &AppHandle, speed: String) {
    let payload = DownloadSpeedEventPayload { speed };
    let event = DownloadSpeedEvent(payload);
    let _ = event.emit(app);
}
