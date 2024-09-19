use std::sync::{Arc, RwLock};
use std::time::Duration;

use anyhow::anyhow;
use chrono::Local;
use hmac::{Hmac, Mac};
use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::{Jitter, RetryTransientMiddleware};
use serde_json::json;
use sha2::Sha256;
use tauri::http::StatusCode;

use crate::responses::{LoginResponseData, PicaResponse};

const HOST_URL: &str = "https://picaapi.picacomic.com/";
const API_KEY: &str = "C69BAF41DA5ABD1FFEDC6D2FEA56B";
const NONCE: &str = "ptxdhmjzqtnrtwndhbxcpkjamb33w837";
const DIGEST_KEY: &str = r#"~d}$Q7$eIni=V)9\RK/P.RM4;9[7|@/CA}b~OW!3?EV`:<>M7pddUBL5n|0/*Cn"#;

#[derive(Clone)]
pub struct PicaClient {
    client: ClientWithMiddleware,
    token: Arc<RwLock<String>>,
}

impl PicaClient {
    pub fn new() -> Self {
        let retry_policy = reqwest_retry::policies::ExponentialBackoff::builder()
            .base(1) // 指数为1，保证重试间隔为1秒不变
            .jitter(Jitter::Bounded) // 重试间隔在1秒左右波动
            .build_with_total_retry_duration(Duration::from_secs(3)); // 重试总时长为3秒
        let client = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(2)) // 每个请求超过2秒就超时
            .build()
            .unwrap();
        let client = reqwest_middleware::ClientBuilder::new(client)
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();
        Self {
            client,
            token: Arc::new(RwLock::new(String::new())),
        }
    }

    pub fn client(&self) -> &ClientWithMiddleware {
        &self.client
    }

    pub fn set_token(&self, token: &str) {
        token.clone_into(&mut self.token.write().unwrap());
    }

    pub fn token(&self) -> String {
        self.token.read().unwrap().clone()
    }

    async fn pica_request(
        &self,
        method: reqwest::Method,
        path: &str,
        payload: Option<serde_json::Value>,
    ) -> anyhow::Result<reqwest::Response> {
        let time = Local::now().timestamp().to_string();
        let signature = create_signature(path, &method, &time)?;

        let request = self
            .client()
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
            .header("authorization", self.token())
            .header("image-quality", "original")
            .header("signature", signature);

        let http_resp = match payload {
            None => request.send().await?,
            Some(body) => request.body(serde_json::to_string(&body)?).send().await?,
        };

        Ok(http_resp)
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

        self.set_token(&data.token);
        Ok(data.token)
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
