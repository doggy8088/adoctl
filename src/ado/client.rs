use reqwest::{Method, header::CONTENT_TYPE};
use serde::{Serialize, de::DeserializeOwned};

use crate::{
    auth::Authenticator,
    config::{AdoService, Organization},
    debug,
    error::{AdoctlError, Result},
};

#[derive(Clone)]
pub struct AdoClient {
    http: reqwest::Client,
    organization: Organization,
    auth: Authenticator,
}

impl AdoClient {
    pub fn new(organization: Organization, auth: Authenticator) -> Self {
        Self {
            http: reqwest::Client::new(),
            organization,
            auth,
        }
    }

    pub async fn get_json<T>(
        &self,
        service: AdoService,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.send_json(Method::GET, service, path, query, Option::<&()>::None, None)
            .await
    }

    pub async fn get_json_with_headers<T>(
        &self,
        service: AdoService,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<(T, reqwest::header::HeaderMap)>
    where
        T: DeserializeOwned,
    {
        let response = self
            .request(Method::GET, service, path, query, Option::<&()>::None, None)
            .await?;
        let headers = response.headers().clone();
        let payload = parse_json_response(response).await?;
        Ok((payload, headers))
    }

    pub async fn patch_json<T, B>(
        &self,
        service: AdoService,
        path: &str,
        query: &[(&str, String)],
        body: &B,
        content_type: Option<&str>,
    ) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        self.send_json(
            Method::PATCH,
            service,
            path,
            query,
            Some(body),
            content_type,
        )
        .await
    }

    pub async fn put_empty(
        &self,
        service: AdoService,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<()> {
        self.send_empty(Method::PUT, service, path, query).await
    }

    pub async fn delete_empty(
        &self,
        service: AdoService,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<()> {
        self.send_empty(Method::DELETE, service, path, query).await
    }

    pub async fn check_connection(&self) -> Result<()> {
        let _: serde_json::Value = self
            .get_json(
                AdoService::Core,
                "_apis/connectionData",
                &[("api-version", "7.1-preview.1".into())],
            )
            .await?;
        Ok(())
    }

    async fn send_json<T, B>(
        &self,
        method: Method,
        service: AdoService,
        path: &str,
        query: &[(&str, String)],
        body: Option<&B>,
        content_type: Option<&str>,
    ) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let response = self
            .request(method, service, path, query, body, content_type)
            .await?;
        parse_json_response(response).await
    }

    async fn send_empty(
        &self,
        method: Method,
        service: AdoService,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<()> {
        let response = self
            .request(method, service, path, query, Option::<&()>::None, None)
            .await?;
        let status = response.status();
        let text = response.text().await?;
        log_response(status, &text);

        if status.is_success() {
            Ok(())
        } else {
            Err(AdoctlError::Api {
                status,
                message: extract_api_message(&text),
            })
        }
    }

    async fn request<B>(
        &self,
        method: Method,
        service: AdoService,
        path: &str,
        query: &[(&str, String)],
        body: Option<&B>,
        content_type: Option<&str>,
    ) -> Result<reqwest::Response>
    where
        B: Serialize + ?Sized,
    {
        let mut url = self
            .organization
            .service_base(service)?
            .join(path.trim_start_matches('/'))?;

        {
            let mut pairs = url.query_pairs_mut();
            for (key, value) in query {
                pairs.append_pair(key, value);
            }
        }

        let method_name = method.as_str().to_owned();
        debug::log(format!("HTTP 要求：{} {}", method_name, url));

        let mut request = self.http.request(method, url).header(
            reqwest::header::AUTHORIZATION,
            self.auth.authorization_header().await?,
        );

        if let Some(body) = body {
            request = request.json(body);
            if let Some(content_type) = content_type {
                request = request.header(CONTENT_TYPE, content_type);
            }
        }

        Ok(request.send().await?)
    }
}

pub(crate) fn continuation_token_from_headers(
    headers: &reqwest::header::HeaderMap,
) -> Option<String> {
    headers
        .get("x-ms-continuationtoken")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

async fn parse_json_response<T>(response: reqwest::Response) -> Result<T>
where
    T: DeserializeOwned,
{
    let status = response.status();
    let text = response.text().await?;
    log_response(status, &text);

    if !status.is_success() {
        return Err(AdoctlError::Api {
            status,
            message: extract_api_message(&text),
        });
    }

    if debug::is_enabled()
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
    {
        return Ok(serde_json::from_value(value)?);
    }

    Ok(serde_json::from_str(&text)?)
}

fn log_response(status: reqwest::StatusCode, text: &str) {
    if !debug::is_enabled() {
        return;
    }

    debug::log(format!(
        "HTTP 回應：status={}，body_bytes={}",
        status,
        text.len()
    ));

    if text.trim().is_empty() {
        debug::log("HTTP 回應內容為空。");
        return;
    }

    match serde_json::from_str::<serde_json::Value>(text) {
        Ok(value) => debug::log(format!("HTTP JSON 摘要：{}", debug::summarize_json(&value))),
        Err(_) => debug::log(format!("HTTP 回應文字：{}", debug::preview_text(text, 200))),
    }
}

fn extract_api_message(text: &str) -> String {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|value| {
            value
                .get("message")
                .or_else(|| value.get("error_description"))
                .and_then(|message| message.as_str())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| text.trim().to_owned())
}
