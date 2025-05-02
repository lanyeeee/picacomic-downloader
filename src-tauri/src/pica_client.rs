use std::io::Cursor;
use std::time::Duration;

use anyhow::{anyhow, Context};
use bytes::Bytes;
use chrono::Local;
use hmac::{Hmac, Mac};
use image::ImageFormat;
use parking_lot::RwLock;
use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::{Jitter, RetryTransientMiddleware};
use serde_json::json;
use sha2::Sha256;
use tauri::http::StatusCode;
use tauri::{AppHandle, Manager};

use crate::config::Config;
use crate::responses::{
    ChapterImageRespData, ChapterRespData, ComicInFavoriteRespData, ComicInSearchRespData,
    ComicRespData, GetChapterImageRespData, GetChapterRespData, GetComicRespData,
    GetFavoriteRespData, LoginRespData, Pagination, PicaResp, SearchRespData,
    UserProfileDetailRespData, UserProfileRespData,
};
use crate::types::{DownloadFormat, Sort};

const HOST_URL: &str = "https://picaapi.picacomic.com/";
const API_KEY: &str = "C69BAF41DA5ABD1FFEDC6D2FEA56B";
const NONCE: &str = "ptxdhmjzqtnrtwndhbxcpkjamb33w837";
const DIGEST_KEY: &str = r#"~d}$Q7$eIni=V)9\RK/P.RM4;9[7|@/CA}b~OW!3?EV`:<>M7pddUBL5n|0/*Cn"#; //TODO: 去除没必要的#号

#[derive(Clone)]
pub struct PicaClient {
    app: AppHandle,
}

impl PicaClient {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    // TODO: 用api_client和img_client分别处理api请求和图片请求，避免每次请求都创建client
    pub fn client() -> ClientWithMiddleware {
        // TODO: 可以将retry_policy缓存起来，避免每次请求都创建
        let retry_policy = reqwest_retry::policies::ExponentialBackoff::builder()
            .base(1) // 指数为1，保证重试间隔为1秒不变
            .jitter(Jitter::Bounded) // 重试间隔在1秒左右波动
            .build_with_total_retry_duration(Duration::from_secs(3)); // 重试总时长为3秒
        let client = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(2)) // 每个请求超过2秒就超时
            .build()
            .unwrap();
        reqwest_middleware::ClientBuilder::new(client)
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build()
    }

    async fn pica_request(
        &self,
        method: reqwest::Method,
        path: &str,
        payload: Option<serde_json::Value>,
    ) -> anyhow::Result<reqwest::Response> {
        let time = Local::now().timestamp().to_string();
        let signature = create_signature(path, &method, &time)?;
        let token = self.app.state::<RwLock<Config>>().read().token.clone();

        let request = Self::client()
            .request(method.clone(), format!("{HOST_URL}{path}").as_str())
            .header("api-key", API_KEY)
            .header("accept", "application/vnd.picacomic.com.v1+json")
            .header("app-channel", "2")
            .header("time", time)
            .header("nonce", NONCE)
            .header("app-version", "2.2.1.2.3.3")
            .header("app-uuid", "defaultUuid")
            .header("app-platform", "android")
            .header("app-build-version", "44")
            .header("Content-Type", "application/json; charset=UTF-8")
            .header("User-Agent", "okhttp/3.8.1")
            .header("authorization", token)
            .header("image-quality", "original")
            .header("signature", signature);

        let http_resp = match payload {
            Some(body) => request.json(&body).send().await,
            None => request.send().await,
        }
        .map_err(|e| {
            if e.is_timeout() {
                anyhow::Error::from(e).context("连接超时，请使用代理或换条线路重试")
            } else {
                anyhow::Error::from(e)
            }
        })?;

        Ok(http_resp)
    }

    async fn pica_get(&self, path: &str) -> anyhow::Result<reqwest::Response> {
        self.pica_request(reqwest::Method::GET, path, None).await
    }

    async fn pica_post(
        &self,
        path: &str,
        payload: serde_json::Value,
    ) -> anyhow::Result<reqwest::Response> {
        self.pica_request(reqwest::Method::POST, path, Some(payload))
            .await
    }

    pub async fn login(&self, email: &str, password: &str) -> anyhow::Result<String> {
        let payload = json!({
            "email": email,
            "password": password,
        });
        // 发送登录请求
        let http_resp = self.pica_post("auth/sign-in", payload).await?;
        // 检查http响应状态码
        let status = http_resp.status();
        let body = http_resp.text().await?;
        if status == StatusCode::BAD_REQUEST {
            return Err(anyhow!("用户名或密码错误({status}): {body}"));
        } else if status != StatusCode::OK {
            return Err(anyhow!("预料之外的状态码({status}): {body}"));
        }
        // 尝试将body解析为PicaResp
        let pica_resp = serde_json::from_str::<PicaResp>(&body)
            .context(format!("将body解析为PicaResp失败: {body}"))?;
        // 检查PicaResp的code字段
        if pica_resp.code != 200 {
            return Err(anyhow!("预料之外的code: {pica_resp:?}"));
        }
        // 检查BiliResp的data是否存在
        let Some(data) = pica_resp.data else {
            return Err(anyhow!("data字段不存在: {pica_resp:?}"));
        };
        // 尝试将data解析为LoginRespData
        let data_str = data.to_string();
        let login_resp_data = serde_json::from_str::<LoginRespData>(&data_str)
            .context(format!("将data解析为LoginRespData失败: {data_str}"))?;

        Ok(login_resp_data.token)
    }

    pub async fn get_user_profile(&self) -> anyhow::Result<UserProfileDetailRespData> {
        // 发送获取用户信息请求
        let http_resp = self.pica_get("users/profile").await?;
        // 检查http响应状态码
        let status = http_resp.status();
        let body = http_resp.text().await?;
        if status == StatusCode::UNAUTHORIZED {
            return Err(anyhow!(
                "Authorization无效或已过期，请重新登录({status}): {body}"
            ));
        } else if status != StatusCode::OK {
            return Err(anyhow!("预料之外的状态码({status}): {body}"));
        }
        // 尝试将body解析为PicaResp
        let pica_resp = serde_json::from_str::<PicaResp>(&body)
            .context(format!("将body解析为PicaResp失败: {body}"))?;
        // 检查PicaResp的code字段
        if pica_resp.code != 200 {
            return Err(anyhow!("预料之外的code: {pica_resp:?}"));
        }
        // 检查PicaResp的data是否存在
        let Some(data) = pica_resp.data else {
            return Err(anyhow!("data字段不存在: {pica_resp:?}"));
        };
        // 尝试将data解析为UserProfileRespData
        let data_str = data.to_string();
        let user_profile_resp_data = serde_json::from_str::<UserProfileRespData>(&data_str)
            .context(format!("将data解析为UserProfileRespData失败: {data_str}"))?;

        Ok(user_profile_resp_data.user)
    }

    pub async fn search_comic(
        &self,
        keyword: &str,
        sort: Sort,
        page: i32,
        categories: Vec<String>,
    ) -> anyhow::Result<Pagination<ComicInSearchRespData>> {
        let payload = json!({
            "keyword": keyword,
            "sort": sort.as_str(),
            "categories": categories,
        });
        // 发送搜索漫画请求
        let path = format!("comics/advanced-search?page={page}");
        let http_resp = self.pica_post(&path, payload).await?;
        // 检查http响应状态码
        let status = http_resp.status();
        let body = http_resp.text().await?;
        if status == StatusCode::UNAUTHORIZED {
            return Err(anyhow!(
                "Authorization无效或已过期，请重新登录({status}): {body}"
            ));
        } else if status != StatusCode::OK {
            return Err(anyhow!("预料之外的状态码({status}): {body}"));
        }
        // 尝试将body解析为PicaResp
        let pica_resp = serde_json::from_str::<PicaResp>(&body)
            .context(format!("将body解析为PicaResp失败: {body}"))?;
        // 检查PicaResp的code字段
        if pica_resp.code != 200 {
            return Err(anyhow!("预料之外的code: {pica_resp:?}"));
        }
        // 检查PicaResp的data是否存在
        let Some(data) = pica_resp.data else {
            return Err(anyhow!("data字段不存在: {pica_resp:?}"));
        };
        // 尝试将data解析为SearchRespData
        let data_str = data.to_string();
        let search_resp_data = serde_json::from_str::<SearchRespData>(&data_str)
            .context(format!("将data解析为SearchRespData失败: {data_str}"))?;

        Ok(search_resp_data.comics)
    }

    pub async fn get_comic(&self, comic_id: &str) -> anyhow::Result<ComicRespData> {
        // 发送获取漫画请求
        let path = format!("comics/{comic_id}");
        let http_resp = self.pica_get(&path).await?;
        // 检查http响应状态码
        let status = http_resp.status();
        let body = http_resp.text().await?;
        if status == StatusCode::UNAUTHORIZED {
            return Err(anyhow!(
                "Authorization无效或已过期，请重新登录({status}): {body}"
            ));
        } else if status != StatusCode::OK {
            return Err(anyhow!("预料之外的状态码({status}): {body}"));
        }
        // 尝试将body解析为PicaResp
        let pica_resp = serde_json::from_str::<PicaResp>(&body)
            .context(format!("将body解析为PicaResp失败: {body}"))?;
        // 检查PicaResp的code字段
        if pica_resp.code != 200 {
            return Err(anyhow!("预料之外的code: {pica_resp:?}"));
        }
        // 检查PicaResp的data是否存在
        let Some(data) = pica_resp.data else {
            return Err(anyhow!("data字段不存在: {pica_resp:?}"));
        };
        // 尝试将data解析为GetComicRespData
        let data_str = data.to_string();
        let get_comic_resp_data = serde_json::from_str::<GetComicRespData>(&data_str)
            .context(format!("将data解析为GetComicRespData失败: {data_str}"))?;

        Ok(get_comic_resp_data.comic)
    }

    pub async fn get_chapter(
        &self,
        comic_id: &str,
        page: i64,
    ) -> anyhow::Result<Pagination<ChapterRespData>> {
        // 发送获取漫画章节分页请求
        let path = format!("comics/{comic_id}/eps?page={page}");
        let http_resp = self.pica_get(&path).await?;
        // 检查http响应状态码
        let status = http_resp.status();
        let body = http_resp.text().await?;
        if status == StatusCode::UNAUTHORIZED {
            return Err(anyhow!(
                "Authorization无效或已过期，请重新登录({status}): {body}"
            ));
        } else if status != StatusCode::OK {
            return Err(anyhow!("预料之外的状态码({status}): {body}"));
        }
        // 尝试将body解析为PicaResp
        let pica_resp = serde_json::from_str::<PicaResp>(&body)
            .context(format!("将body解析为PicaResp失败: {body}"))?;
        // 检查PicaResp的code字段
        if pica_resp.code != 200 {
            return Err(anyhow!("预料之外的code: {pica_resp:?}"));
        }
        // 检查PicaResp的data是否存在
        let Some(data) = pica_resp.data else {
            return Err(anyhow!("data字段不存在: {pica_resp:?}"));
        };
        // 尝试将data解析为GetChapterRespData
        let data_str = data.to_string();
        let get_chapter_resp_data = serde_json::from_str::<GetChapterRespData>(&data_str)
            .context(format!("将data解析为GetChapterRespData失败: {data_str}"))?;

        Ok(get_chapter_resp_data.eps)
    }

    pub async fn get_chapter_image(
        &self,
        comic_id: &str,
        chapter_order: i64,
        page: i64,
    ) -> anyhow::Result<Pagination<ChapterImageRespData>> {
        // 发送获取漫画章节的图片分页请求
        let path = format!("comics/{comic_id}/order/{chapter_order}/pages?page={page}");
        let http_resp = self.pica_get(&path).await?;
        // 检查http响应状态码
        let status = http_resp.status();
        let body = http_resp.text().await?;
        if status == StatusCode::UNAUTHORIZED {
            return Err(anyhow!(
                "Authorization无效或已过期，请重新登录({status}): {body}"
            ));
        } else if status != StatusCode::OK {
            return Err(anyhow!("预料之外的状态码({status}): {body}"));
        }
        // 尝试将body解析为PicaResp
        let pica_resp = serde_json::from_str::<PicaResp>(&body)
            .context(format!("将body解析为PicaResp失败: {body}"))?;
        // 检查PicaResp的code字段
        if pica_resp.code != 200 {
            return Err(anyhow!("预料之外的code: {pica_resp:?}"));
        }
        // 检查PicaResp的data是否存在
        let Some(data) = pica_resp.data else {
            return Err(anyhow!("data字段不存在: {pica_resp:?}"));
        };
        // 尝试将data解析为GetChapterImageRespData
        let data_str = data.to_string();
        let get_chapter_image_resp_data =
            serde_json::from_str::<GetChapterImageRespData>(&data_str).context(format!(
                "将data解析为GetChapterImageRespData失败: {data_str}"
            ))?;

        Ok(get_chapter_image_resp_data.pages)
    }

    pub async fn get_favorite_comics(
        &self,
        sort: Sort,
        page: i64,
    ) -> anyhow::Result<Pagination<ComicInFavoriteRespData>> {
        // 发送获取收藏的漫画请求
        let sort = sort.as_str();
        let path = format!("users/favourite?s={sort}&page={page}");
        let http_resp = self.pica_get(&path).await?;
        // 检查http响应状态码
        let status = http_resp.status();
        let body = http_resp.text().await?;
        if status == StatusCode::UNAUTHORIZED {
            return Err(anyhow!(
                "Authorization无效或已过期，请重新登录({status}): {body}"
            ));
        } else if status != StatusCode::OK {
            return Err(anyhow!("预料之外的状态码({status}): {body}"));
        }
        // 尝试将body解析为PicaResp
        let pica_resp: PicaResp =
            serde_json::from_str(&body).context(format!("将body解析为PicaResp失败: {body}"))?;
        // 检查PicaResp的code字段
        if pica_resp.code != 200 {
            return Err(anyhow!("预料之外的code: {pica_resp:?}"));
        }
        // 检查PicaResp的data是否存在
        let Some(data) = pica_resp.data else {
            return Err(anyhow!("data字段不存在: {pica_resp:?}"));
        };
        // 尝试将data解析为GetFavoriteRespData
        let data_str = data.to_string();
        let get_favorite_resp_data = serde_json::from_str::<GetFavoriteRespData>(&data_str)
            .context(format!("将data解析为GetFavoriteRespData失败: {data_str}"))?;

        Ok(get_favorite_resp_data.comics)
    }

    pub async fn get_image_data(&self, url: &str) -> anyhow::Result<Bytes> {
        let http_resp = Self::client().get(url).send().await?;

        let status = http_resp.status();
        if status != StatusCode::OK {
            let text = http_resp.text().await?;
            let err = anyhow!("下载图片`{url}`失败，预料之外的状态码: {text}");
            return Err(err);
        }
        // 获取 resp headers 的 content-type 字段
        let content_type = http_resp
            .headers()
            .get("content-type")
            .ok_or(anyhow!("响应中没有content-type字段"))?
            .to_str()
            .context("响应中的content-type字段不是utf-8字符串")?
            .to_string();
        let image_data = http_resp.bytes().await?;
        // 确定原始格式
        let original_format = match content_type.as_str() {
            "image/jpeg" => ImageFormat::Jpeg,
            "image/png" => ImageFormat::Png,
            _ => return Err(anyhow!("原图出现了意料之外的格式: {content_type}")),
        };
        // 确定目标格式
        let download_format = self.app.state::<RwLock<Config>>().read().download_format;
        let target_format = match download_format {
            DownloadFormat::Jpeg => ImageFormat::Jpeg,
            DownloadFormat::Png => ImageFormat::Png,
            DownloadFormat::Original => original_format,
        };
        // 如果原始格式与目标格式相同，直接返回
        if original_format == target_format {
            return Ok(image_data);
        }
        // 否则需要将图片转换为目标格式
        let img =
            image::load_from_memory(&image_data).context("将图片数据转换为DynamicImage失败")?;
        let mut converted_data = Vec::new();
        match target_format {
            ImageFormat::Jpeg => img
                .to_rgb8()
                .write_to(&mut Cursor::new(&mut converted_data), target_format),
            ImageFormat::Png | ImageFormat::WebP => img
                .to_rgba8()
                .write_to(&mut Cursor::new(&mut converted_data), target_format),
            _ => return Err(anyhow!("这里不应该出现目标格式`{target_format:?}`")),
        }
        .context(format!(
            "将`{original_format:?}`转换为`{target_format:?}`失败"
        ))?;

        Ok(Bytes::from(converted_data))
    }
}

fn create_signature(path: &str, method: &reqwest::Method, time: &str) -> anyhow::Result<String> {
    let method = method.as_str();
    let data = format!("{path}{time}{NONCE}{method}{API_KEY}").to_lowercase();

    let signature = hmac_hex(DIGEST_KEY, &data)?;
    Ok(signature)
}

fn hmac_hex(key: &str, data: &str) -> anyhow::Result<String> {
    let key = key.as_bytes();
    let mut mac = Hmac::<Sha256>::new_from_slice(key)?;
    mac.update(data.as_bytes());
    let result = hex::encode(mac.finalize().into_bytes().as_slice());
    Ok(result)
}
