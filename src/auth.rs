use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::header::HeaderValue;
use serde::Deserialize;
use tokio::{process::Command, sync::Mutex, time::sleep};

use crate::{
    cli::AuthMethodArg,
    config::Organization,
    credentials::{CredentialKey, CredentialStore, StoredCredential},
    debug,
    error::{AdoctlError, Result},
};

pub const AZURE_DEVOPS_RESOURCE: &str = "499b84ac-1321-427f-aa17-267ca6975798";
pub const AZURE_DEVOPS_SCOPE: &str = "499b84ac-1321-427f-aa17-267ca6975798/.default offline_access";
pub const ADOCTL_PAT_ENV: &str = "ADOCTL_PAT";
pub const AZURE_DEVOPS_EXT_PAT_ENV: &str = "AZURE_DEVOPS_EXT_PAT";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PatEnvironmentSource {
    AdoctlPat,
    AzureDevopsExtPat,
}

impl PatEnvironmentSource {
    fn env_name(self) -> &'static str {
        match self {
            Self::AdoctlPat => ADOCTL_PAT_ENV,
            Self::AzureDevopsExtPat => AZURE_DEVOPS_EXT_PAT_ENV,
        }
    }
}

#[derive(Clone)]
pub struct Authenticator {
    credential: Arc<Mutex<StoredCredential>>,
    store: Option<Arc<dyn CredentialStore>>,
    key: Option<CredentialKey>,
    http: reqwest::Client,
}

impl Authenticator {
    pub fn new(
        credential: StoredCredential,
        store: Option<Arc<dyn CredentialStore>>,
        key: Option<CredentialKey>,
    ) -> Self {
        Self {
            credential: Arc::new(Mutex::new(credential)),
            store,
            key,
            http: reqwest::Client::new(),
        }
    }

    pub async fn authorization_header(&self) -> Result<HeaderValue> {
        let mut credential = self.credential.lock().await;
        match &mut *credential {
            StoredCredential::Pat { token } => {
                debug::log("使用 PAT 認證建立 Authorization header。");
                basic_pat_header(token)
            }
            StoredCredential::AzureCli => {
                debug::log("使用 Azure CLI 取得 access token。");
                let token = azure_cli_access_token().await?;
                bearer_header(&token)
            }
            StoredCredential::DeviceCode {
                client_id,
                tenant,
                refresh_token,
                access_token,
                expires_at,
            } => {
                if let (Some(token), Some(stored_expires_at)) =
                    (access_token.as_ref(), expires_at.as_ref())
                    && now_epoch_seconds() + 60 < *stored_expires_at
                {
                    debug::log("使用已快取的 device code access token。");
                    return bearer_header(token);
                }

                debug::log("device code access token 即將過期，正在刷新。");
                let refreshed = refresh_device_code_token(
                    &self.http,
                    tenant,
                    client_id,
                    refresh_token.as_str(),
                )
                .await?;

                *refresh_token = refreshed
                    .refresh_token
                    .clone()
                    .unwrap_or_else(|| refresh_token.clone());
                *access_token = Some(refreshed.access_token.clone());
                *expires_at = refreshed
                    .expires_in
                    .map(|seconds| now_epoch_seconds() + seconds);

                if let (Some(store), Some(key)) = (&self.store, &self.key) {
                    store.save(key, &credential)?;
                }

                bearer_header(&refreshed.access_token)
            }
        }
    }
}

pub fn load_stored_or_env_credential(
    org: &Organization,
    profile: &str,
    method: Option<AuthMethodArg>,
    store: Arc<dyn CredentialStore>,
) -> Result<(StoredCredential, CredentialKey)> {
    let key = CredentialKey::new(org.name.clone(), profile.to_owned());

    if let Some((source, token)) = pat_token_from_env_with_source() {
        debug::log(format!("認證來源：環境變數 {}。", source.env_name()));
        return Ok((StoredCredential::Pat { token }, key));
    }

    if matches!(method, Some(AuthMethodArg::AzureCli)) {
        debug::log("認證來源：命令列指定 Azure CLI。");
        return Ok((StoredCredential::AzureCli, key));
    }

    if let Some(credential) = store.load(&key)? {
        debug::log(format!(
            "認證來源：已保存的登入資訊（{}）。",
            credential.summary()
        ));
        if let Some(expected_method) = method
            && !stored_credential_matches_method(&credential, expected_method)
        {
            return Err(AdoctlError::Authentication(format!(
                "已保存的登入方式是 {}，但本次指定的是 {:?}。請重新執行 adoctl login。",
                credential.summary(),
                expected_method
            )));
        }
        return Ok((credential, key));
    }

    debug::log("找不到環境變數 PAT 或已保存的登入資訊。");
    Err(AdoctlError::NotLoggedIn)
}

pub fn pat_token_from_env() -> Option<String> {
    pat_token_from_env_with_source().map(|(_, token)| token)
}

fn pat_token_from_env_with_source() -> Option<(PatEnvironmentSource, String)> {
    pat_token_from_values(
        std::env::var(ADOCTL_PAT_ENV).ok(),
        std::env::var(AZURE_DEVOPS_EXT_PAT_ENV).ok(),
    )
}

fn pat_token_from_values(
    adoctl_pat: Option<String>,
    azure_devops_ext_pat: Option<String>,
) -> Option<(PatEnvironmentSource, String)> {
    [
        (PatEnvironmentSource::AdoctlPat, adoctl_pat),
        (
            PatEnvironmentSource::AzureDevopsExtPat,
            azure_devops_ext_pat,
        ),
    ]
    .into_iter()
    .filter_map(|(source, token)| token.map(|token| (source, token)))
    .map(|(source, token)| (source, token.trim().to_owned()))
    .find(|(_, token)| !token.is_empty())
}

fn stored_credential_matches_method(credential: &StoredCredential, method: AuthMethodArg) -> bool {
    matches!(
        (credential, method),
        (StoredCredential::Pat { .. }, AuthMethodArg::Pat)
            | (StoredCredential::AzureCli, AuthMethodArg::AzureCli)
            | (
                StoredCredential::DeviceCode { .. },
                AuthMethodArg::DeviceCode
            )
    )
}

pub async fn azure_cli_access_token() -> Result<String> {
    debug::log("正在呼叫 Azure CLI 取得 Azure DevOps access token。");
    let output = Command::new("az")
        .args([
            "account",
            "get-access-token",
            "--resource",
            AZURE_DEVOPS_RESOURCE,
            "--query",
            "accessToken",
            "--output",
            "tsv",
        ])
        .output()
        .await
        .map_err(|error| AdoctlError::AzureCli(error.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AdoctlError::AzureCli(stderr.trim().to_owned()));
    }

    let token = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if token.is_empty() {
        return Err(AdoctlError::AzureCli(
            "Azure CLI 沒有回傳 access token，請先執行 az login。".into(),
        ));
    }

    debug::log("Azure CLI 已成功回傳 access token。");
    Ok(token)
}

pub async fn request_device_code(
    http: &reqwest::Client,
    tenant: &str,
    client_id: &str,
) -> Result<DeviceCodeResponse> {
    let url = format!("https://login.microsoftonline.com/{tenant}/oauth2/v2.0/devicecode");
    let response = http
        .post(url)
        .form(&[("client_id", client_id), ("scope", AZURE_DEVOPS_SCOPE)])
        .send()
        .await?;
    parse_auth_response(response).await
}

pub async fn poll_device_code_token(
    http: &reqwest::Client,
    tenant: &str,
    client_id: &str,
    device_code: &str,
    interval: u64,
    expires_in: u64,
) -> Result<TokenResponse> {
    let url = format!("https://login.microsoftonline.com/{tenant}/oauth2/v2.0/token");
    let started_at = now_epoch_seconds();
    let mut interval = interval.max(5);

    loop {
        if now_epoch_seconds().saturating_sub(started_at) > expires_in {
            return Err(AdoctlError::Authentication(
                "device code 登入逾時，請重新執行 login。".into(),
            ));
        }

        let response = http
            .post(&url)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", client_id),
                ("device_code", device_code),
            ])
            .send()
            .await?;

        if response.status().is_success() {
            return parse_auth_response(response).await;
        }

        let status = response.status();
        let error = response.json::<OAuthError>().await?;
        match error.error.as_str() {
            "authorization_pending" => sleep(Duration::from_secs(interval)).await,
            "slow_down" => {
                interval += 5;
                sleep(Duration::from_secs(interval)).await;
            }
            _ => {
                return Err(AdoctlError::Api {
                    status,
                    message: error
                        .error_description
                        .unwrap_or(error.error)
                        .trim()
                        .to_owned(),
                });
            }
        }
    }
}

async fn refresh_device_code_token(
    http: &reqwest::Client,
    tenant: &str,
    client_id: &str,
    refresh_token: &str,
) -> Result<TokenResponse> {
    let url = format!("https://login.microsoftonline.com/{tenant}/oauth2/v2.0/token");
    let response = http
        .post(url)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", client_id),
            ("refresh_token", refresh_token),
            ("scope", AZURE_DEVOPS_SCOPE),
        ])
        .send()
        .await?;
    parse_auth_response(response).await
}

fn basic_pat_header(token: &str) -> Result<HeaderValue> {
    let encoded = STANDARD.encode(format!(":{token}"));
    header_value(format!("Basic {encoded}"))
}

fn bearer_header(token: &str) -> Result<HeaderValue> {
    header_value(format!("Bearer {token}"))
}

fn header_value(value: String) -> Result<HeaderValue> {
    HeaderValue::from_str(&value)
        .map_err(|error| AdoctlError::Authentication(format!("無法建立認證 header：{error}")))
}

async fn parse_auth_response<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
) -> Result<T> {
    let status = response.status();
    let text = response.text().await?;
    if status.is_success() {
        return Ok(serde_json::from_str(&text)?);
    }

    let message = serde_json::from_str::<OAuthError>(&text)
        .ok()
        .and_then(|error| error.error_description.or(Some(error.error)))
        .unwrap_or(text);
    Err(AdoctlError::Api { status, message })
}

fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn expires_at_from_now(expires_in: Option<u64>) -> Option<u64> {
    expires_in.map(|seconds| now_epoch_seconds() + seconds)
}

pub fn pat_create_url(organization: &Organization) -> String {
    format!(
        "https://dev.azure.com/{}/_usersSettings/tokens",
        organization.name
    )
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: Option<u64>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct OAuthError {
    error: String,
    error_description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{
        AZURE_DEVOPS_EXT_PAT_ENV, PatEnvironmentSource, pat_create_url, pat_token_from_values,
    };
    use crate::config::Organization;

    #[test]
    fn pat_create_url_uses_parsed_organization_name() {
        let org = Organization::parse("willh", None).unwrap();
        assert_eq!(
            pat_create_url(&org),
            "https://dev.azure.com/willh/_usersSettings/tokens"
        );

        let org = Organization::parse("https://dev.azure.com/willh", None).unwrap();
        assert_eq!(
            pat_create_url(&org),
            "https://dev.azure.com/willh/_usersSettings/tokens"
        );
    }

    #[test]
    fn pat_token_uses_azure_devops_ext_pat_when_adoctl_pat_is_absent() {
        assert_eq!(
            pat_token_from_values(None, Some(" azure-devops-ext-token ".into())),
            Some((
                PatEnvironmentSource::AzureDevopsExtPat,
                "azure-devops-ext-token".into(),
            ))
        );
        assert_eq!(
            PatEnvironmentSource::AzureDevopsExtPat.env_name(),
            AZURE_DEVOPS_EXT_PAT_ENV
        );
    }

    #[test]
    fn pat_token_prefers_adoctl_pat_over_azure_devops_ext_pat() {
        assert_eq!(
            pat_token_from_values(Some("adoctl-token".into()), Some("azure-token".into())),
            Some((PatEnvironmentSource::AdoctlPat, "adoctl-token".into()))
        );
    }

    #[test]
    fn pat_token_ignores_empty_values() {
        assert_eq!(
            pat_token_from_values(Some("   ".into()), Some("azure-token".into())),
            Some((
                PatEnvironmentSource::AzureDevopsExtPat,
                "azure-token".into(),
            ))
        );
        assert_eq!(pat_token_from_values(Some("".into()), None), None);
    }
}
