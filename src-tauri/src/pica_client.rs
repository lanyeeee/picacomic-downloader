use std::time::Duration;

use anyhow::{anyhow, Context};
use bytes::Bytes;
use chrono::Local;
use hmac::{Hmac, Mac};
use image::ImageFormat;
use parking_lot::RwLock;
use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::{Jitter, RetryTransientMiddleware};
use serde_json::json;
use sha2::Sha256;
use tauri::http::StatusCode;
use tauri::{AppHandle, Manager};

use crate::config::{Config, ProxyConfig, ProxyType};
use crate::responses::{
    ChapterImageRespData, ChapterRespData, ComicRespData, GetChapterImageRespData,
    GetChapterRespData, GetComicRespData, GetFavoriteRespData, LoginRespData, Pagination, PicaResp,
    SearchRespData, UserProfileDetailRespData, UserProfileRespData,
};
use crate::types::{GetFavoriteSort, SearchSort};

const HOST_URL: &str = "https://picaapi.picacomic.com/";
const API_KEY: &str = "C69BAF41DA5ABD1FFEDC6D2FEA56B";
const NONCE: &str = "ptxdhmjzqtnrtwndhbxcpkjamb33w837";
const DIGEST_KEY: &str = r"~d}$Q7$eIni=V)9\RK/P.RM4;9[7|@/CA}b~OW!3?EV`:<>M7pddUBL5n|0/*Cn";

#[derive(Clone)]
pub struct PicaClient {
    app: AppHandle,
    api_client: ClientWithMiddleware,
    img_client: ClientWithMiddleware,
}

impl PicaClient {
    pub fn new(app: AppHandle) -> Self {
        let config = app.state::<RwLock<Config>>().read().clone();
        let api_client = create_api_client_with_proxy(config.proxy.as_ref());
        let img_client = create_img_client_with_proxy(config.proxy.as_ref());
        Self {
            app,
            api_client,
            img_client,
        }
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

        let request = self
            .api_client
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
        sort: SearchSort,
        page: i32,
        categories: Vec<String>,
    ) -> anyhow::Result<SearchRespData> {
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

        Ok(search_resp_data)
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

    pub async fn get_chapter_img(
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

    pub async fn get_favorite(
        &self,
        sort: GetFavoriteSort,
        page: i64,
    ) -> anyhow::Result<GetFavoriteRespData> {
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

        Ok(get_favorite_resp_data)
    }

    pub async fn get_img_data_and_format(&self, url: &str) -> anyhow::Result<(Bytes, ImageFormat)> {
        let http_resp = self.img_client.get(url).send().await?;

        let status = http_resp.status();
        if status != StatusCode::OK {
            let text = http_resp.text().await?;
            let err = anyhow!("下载图片`{url}`失败，预料之外的状态码: {text}");
            return Err(err);
        }
        let image_data = http_resp.bytes().await?;

        let format = image::guess_format(&image_data)
            .context("无法从图片数据中猜测出图片格式，可能图片数据不完整或已损坏")?;

        Ok((image_data, format))
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

fn create_api_client_with_proxy(proxy_config: Option<&ProxyConfig>) -> ClientWithMiddleware {
    let retry_policy = ExponentialBackoff::builder()
        .base(1)
        .jitter(Jitter::Bounded)
        .build_with_total_retry_duration(Duration::from_secs(3));

    let mut client_builder = reqwest::ClientBuilder::new().timeout(Duration::from_secs(2));

    if let Some(proxy) = proxy_config {
        // 只有当主机地址和端口不为空时才设置代理
        if !proxy.host.trim().is_empty() && proxy.port > 0 {
            let proxy_url = match proxy.proxy_type {
                ProxyType::Http => format!("http://{}:{}", proxy.host, proxy.port),
                ProxyType::Socks5 => format!("socks5://{}:{}", proxy.host, proxy.port),
            };

            match reqwest::Proxy::all(&proxy_url) {
                Ok(proxy_config) => {
                    client_builder = client_builder.proxy(proxy_config);
                }
                Err(e) => {
                    eprintln!("无效的代理配置: {}", e);
                }
            }
        }
    }

    let client = client_builder.build().unwrap();

    reqwest_middleware::ClientBuilder::new(client)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build()
}

fn create_img_client_with_proxy(proxy_config: Option<&ProxyConfig>) -> ClientWithMiddleware {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);

    let mut client_builder = reqwest::ClientBuilder::new();

    if let Some(proxy) = proxy_config {
        // 只有当主机地址不为空时才设置代理
        if !proxy.host.trim().is_empty() && proxy.port > 0 {
            let proxy_url = match proxy.proxy_type {
                ProxyType::Http => format!("http://{}:{}", proxy.host, proxy.port),
                ProxyType::Socks5 => format!("socks5://{}:{}", proxy.host, proxy.port),
            };

            match reqwest::Proxy::all(&proxy_url) {
                Ok(proxy_config) => {
                    client_builder = client_builder.proxy(proxy_config);
                }
                Err(e) => {
                    eprintln!("无效的代理配置: {}", e);
                }
            }
        }
    }

    let client = client_builder.build().unwrap();

    reqwest_middleware::ClientBuilder::new(client)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build()
}
