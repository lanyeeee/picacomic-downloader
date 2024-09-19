use std::sync::RwLock;
use std::time::Duration;

use anyhow::anyhow;
use chrono::Local;
use hmac::{Hmac, Mac};
use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::{Jitter, RetryTransientMiddleware};
use serde_json::json;
use sha2::Sha256;
use tauri::{AppHandle, Manager};
use tauri::http::StatusCode;

use crate::config::Config;
use crate::extensions::IgnoreRwLockPoison;
use crate::responses::{
    Comic, ComicInSearch, ComicResponseData, ComicSearchResponseData, Episode, EpisodeImage,
    EpisodeImageResponseData, EpisodeResponseData, LoginResponseData, Pagination, PicaResponse,
    UserProfile, UserProfileResponseData,
};
use crate::types::Sort;

const HOST_URL: &str = "https://picaapi.picacomic.com/";
const API_KEY: &str = "C69BAF41DA5ABD1FFEDC6D2FEA56B";
const NONCE: &str = "ptxdhmjzqtnrtwndhbxcpkjamb33w837";
const DIGEST_KEY: &str = r#"~d}$Q7$eIni=V)9\RK/P.RM4;9[7|@/CA}b~OW!3?EV`:<>M7pddUBL5n|0/*Cn"#;

#[derive(Clone)]
pub struct PicaClient {
    app: AppHandle,
}

impl PicaClient {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

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
        let token = self
            .app
            .state::<RwLock<Config>>()
            .read_or_panic()
            .token
            .clone();

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
            None => request.send().await?,
            Some(body) => request.body(serde_json::to_string(&body)?).send().await?,
        };

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

        let http_resp = self.pica_post("auth/sign-in", payload).await?;

        let status = http_resp.status();
        if status == StatusCode::BAD_REQUEST {
            let text = http_resp.text().await.map_err(anyhow::Error::from)?;
            return Err(anyhow!("登录失败，用户名或密码错误({status}): {text}"));
        } else if status != StatusCode::OK {
            let text = http_resp.text().await.map_err(anyhow::Error::from)?;
            return Err(anyhow!("登录失败，预料之外的状态码({status}): {text}"));
        }

        let pica_resp: PicaResponse = http_resp.json().await?;
        if pica_resp.code != 200 {
            return Err(anyhow!("登录失败，预料之外的code: {pica_resp:?}"));
        }

        let Some(data) = pica_resp.data else {
            return Err(anyhow!("登录失败，data字段不存在: {pica_resp:?}"));
        };
        let data: LoginResponseData = serde_json::from_value(data)?;

        self.app.state::<RwLock<Config>>().write_or_panic().token = data.token.clone();
        Ok(data.token)
    }

    pub async fn get_user_profile(&self) -> anyhow::Result<UserProfile> {
        let http_resp = self.pica_get("users/profile").await?;

        let status = http_resp.status();
        if status == StatusCode::UNAUTHORIZED {
            let text = http_resp.text().await.map_err(anyhow::Error::from)?;
            return Err(anyhow!("获取用户信息失败，未登录({status}): {text}"));
        } else if status != StatusCode::OK {
            let text = http_resp.text().await.map_err(anyhow::Error::from)?;
            return Err(anyhow!(
                "获取用户信息失败，预料之外的状态码({status}): {text}"
            ));
        }

        let pica_resp: PicaResponse = http_resp.json().await?;
        if pica_resp.code != 200 {
            return Err(anyhow!("获取用户信息失败，预料之外的code: {pica_resp:?}"));
        }

        let Some(data) = pica_resp.data else {
            return Err(anyhow!("获取用户信息失败，data字段不存在: {pica_resp:?}"));
        };
        let data: UserProfileResponseData = serde_json::from_value(data)?;

        Ok(data.user)
    }

    pub async fn search_comic(
        &self,
        keyword: &str,
        sort: Sort,
        page: i32,
        categories: Vec<String>,
    ) -> anyhow::Result<Pagination<ComicInSearch>> {
        let payload = json!({
            "keyword": keyword,
            "sort": sort.as_str(),
            "categories": categories,
        });

        let path = format!("comics/advanced-search?page={page}");
        let http_resp = self.pica_post(&path, payload).await?;

        let status = http_resp.status();
        if status == StatusCode::UNAUTHORIZED {
            let text = http_resp.text().await.map_err(anyhow::Error::from)?;
            return Err(anyhow!("搜索漫画失败，未登录({status}): {text}"));
        } else if http_resp.status() != StatusCode::OK {
            let text = http_resp.text().await.map_err(anyhow::Error::from)?;
            return Err(anyhow!("搜索漫画失败，预料之外的状态码({status}): {text}"));
        }

        let pica_resp: PicaResponse = http_resp.json().await?;
        if pica_resp.code != 200 {
            return Err(anyhow!("搜索漫画失败，预料之外的code: {pica_resp:?}"));
        }

        let Some(data) = pica_resp.data else {
            return Err(anyhow!("搜索漫画失败，data字段不存在: {pica_resp:?}"));
        };
        let data: ComicSearchResponseData = serde_json::from_value(data)?;

        Ok(data.comics)
    }

    pub async fn get_comic(&self, comic_id: &str) -> anyhow::Result<Comic> {
        let path = format!("comics/{comic_id}");
        let http_resp = self.pica_get(&path).await?;

        let status = http_resp.status();
        if status == StatusCode::UNAUTHORIZED {
            let text = http_resp.text().await.map_err(anyhow::Error::from)?;
            //TODO: 改为 "获取漫画`{comic_id}`的信息失败，...."
            return Err(anyhow!(
                "获取ID为 {comic_id} 的漫画失败，未登录({status}): {text}"
            ));
        } else if status != StatusCode::OK {
            let text = http_resp.text().await.map_err(anyhow::Error::from)?;
            return Err(anyhow!(
                "获取ID为 {comic_id} 的漫画失败，预料之外的状态码({status}): {text}"
            ));
        }

        let pica_resp: PicaResponse = http_resp.json().await?;
        if pica_resp.code != 200 {
            return Err(anyhow!(
                "获取ID为 {comic_id} 的漫画失败，预料之外的code: {pica_resp:?}"
            ));
        }

        let Some(data) = pica_resp.data else {
            return Err(anyhow!(
                "获取ID为 {comic_id} 的漫画失败，data字段不存在: {pica_resp:?}"
            ));
        };
        let data: ComicResponseData = serde_json::from_value(data)?;

        Ok(data.comic)
    }

    pub async fn get_episode(
        &self,
        comic_id: &str,
        page: i64,
    ) -> anyhow::Result<Pagination<Episode>> {
        let path = format!("comics/{comic_id}/eps?page={page}");
        let http_resp = self.pica_get(&path).await?;

        let status = http_resp.status();
        if status == StatusCode::UNAUTHORIZED {
            let text = http_resp.text().await.map_err(anyhow::Error::from)?;
            return Err(anyhow!(
                "获取漫画`{comic_id}`的章节分页`{page}`失败，未登录({status}): {text}"
            ));
        } else if status != StatusCode::OK {
            let text = http_resp.text().await.map_err(anyhow::Error::from)?;
            return Err(anyhow!(
                "获取漫画`{comic_id}`的章节分页`{page}`失败，预料之外的状态码({status}): {text}"
            ));
        }

        let pica_res: PicaResponse = http_resp.json().await?;
        if pica_res.code != 200 {
            return Err(anyhow!(
                "获取漫画`{comic_id}`的章节分页`{page}`失败，预料之外的code: {pica_res:?}"
            ));
        }

        let Some(data) = pica_res.data else {
            return Err(anyhow!(
                "获取漫画`{comic_id}`的章节分页`{page}`失败，data字段不存在: {pica_res:?}"
            ));
        };
        let data: EpisodeResponseData = serde_json::from_value(data)?;

        Ok(data.eps)
    }

    pub async fn get_episode_image(
        &self,
        comic_id: &str,
        ep_order: i64,
        page: i64,
    ) -> anyhow::Result<Pagination<EpisodeImage>> {
        let path = format!("comics/{comic_id}/order/{ep_order}/pages?page={page}");
        let http_resp = self.pica_get(&path).await?;

        let status = http_resp.status();
        if status == StatusCode::UNAUTHORIZED {
            let text = http_resp.text().await.map_err(anyhow::Error::from)?;
            return Err(anyhow!(
                "获取漫画`{comic_id}`章节`{ep_order}`的图片分页`{page}`失败，未登录({status}): {text}"
            ));
        } else if status != StatusCode::OK {
            let text = http_resp.text().await.map_err(anyhow::Error::from)?;
            return Err(anyhow!(
                "获取漫画`{comic_id}`章节`{ep_order}`的图片分页`{page}`失败，预料之外的状态码({status}): {text}"
            ));
        }

        let pica_res: PicaResponse = http_resp.json().await?;
        if pica_res.code != 200 {
            return Err(anyhow!(
                "获取漫画`{comic_id}`章节`{ep_order}`的图片分页`{page}`失败，预料之外的code: {pica_res:?}"
            ));
        }

        let Some(data) = pica_res.data else {
            return Err(anyhow!(
                "获取漫画`{comic_id}`章节`{ep_order}`的图片分页`{page}`失败，data字段不存在: {pica_res:?}"
            ));
        };
        let data: EpisodeImageResponseData = serde_json::from_value(data)?;

        Ok(data.pages)
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
