use std::sync::Arc;

use crate::{
    ado::client::AdoClient,
    auth::{
        Authenticator, azure_cli_access_token, expires_at_from_now, pat_create_url,
        pat_token_from_env, poll_device_code_token, request_device_code,
    },
    cli::{AuthMethodArg, LoginArgs},
    config::AppConfig,
    credentials::{CredentialKey, CredentialStore, StoredCredential},
    error::{AdoctlError, Result},
};

pub async fn run(
    config: AppConfig,
    args: LoginArgs,
    store: Arc<dyn CredentialStore>,
) -> Result<()> {
    let key = CredentialKey::new(config.organization.name.clone(), config.profile.clone());
    let credential = match args.method {
        AuthMethodArg::Pat => login_with_pat(&config, &args).await?,
        AuthMethodArg::AzureCli => login_with_azure_cli(&config).await?,
        AuthMethodArg::DeviceCode => login_with_device_code(&config, &args).await?,
    };

    if !args.no_store {
        store.save(&key, &credential)?;
    }

    println!(
        "登入成功：organization={}，profile={}，方式={}{}",
        config.organization.name,
        config.profile,
        credential.summary(),
        if args.no_store { "（未保存）" } else { "" }
    );
    Ok(())
}

async fn login_with_pat(config: &AppConfig, args: &LoginArgs) -> Result<StoredCredential> {
    let pat_create_url = pat_create_url(&config.organization);
    if args.open_browser {
        if webbrowser::open(&pat_create_url).is_err() {
            eprintln!("無法自動開啟瀏覽器，請手動開啟：{pat_create_url}");
        }
    } else {
        eprintln!("如需建立 PAT，請開啟：{pat_create_url}");
    }
    eprintln!(
        "建議 PAT scope 至少包含 Member Entitlement Management、Graph 與 Project/Team 管理所需權限。"
    );

    let token = match &args.pat {
        Some(token) => token.trim().to_owned(),
        None => match pat_token_from_env() {
            Some(token) => token,
            None => rpassword::prompt_password("請貼上 Azure DevOps PAT（輸入不會顯示）：")?
                .trim()
                .to_owned(),
        },
    };

    if token.is_empty() {
        return Err(AdoctlError::Authentication("PAT 不可為空。".into()));
    }

    let credential = StoredCredential::Pat { token };
    validate_credential(config, credential.clone()).await?;
    Ok(credential)
}

async fn login_with_azure_cli(config: &AppConfig) -> Result<StoredCredential> {
    azure_cli_access_token().await?;
    let credential = StoredCredential::AzureCli;
    validate_credential(config, credential.clone()).await?;
    Ok(credential)
}

async fn login_with_device_code(config: &AppConfig, args: &LoginArgs) -> Result<StoredCredential> {
    let client_id = args.device_client_id.as_deref().ok_or_else(|| {
        AdoctlError::Authentication(
            "device-code 登入需要 --device-client-id 或 ADOCTL_DEVICE_CLIENT_ID。".into(),
        )
    })?;

    let http = reqwest::Client::new();
    let device_code = request_device_code(&http, &args.tenant, client_id).await?;
    if let Some(message) = &device_code.message {
        println!("{message}");
    } else {
        println!(
            "請開啟 {}，並輸入代碼 {} 完成登入。",
            device_code.verification_uri, device_code.user_code
        );
    }

    let token = poll_device_code_token(
        &http,
        &args.tenant,
        client_id,
        &device_code.device_code,
        device_code.interval.unwrap_or(5),
        device_code.expires_in,
    )
    .await?;

    let credential = StoredCredential::DeviceCode {
        client_id: client_id.to_owned(),
        tenant: args.tenant.clone(),
        refresh_token: token.refresh_token.clone().ok_or_else(|| {
            AdoctlError::Authentication("device-code 登入沒有回傳 refresh token。".into())
        })?,
        access_token: Some(token.access_token),
        expires_at: expires_at_from_now(token.expires_in),
    };
    validate_credential(config, credential.clone()).await?;
    Ok(credential)
}

async fn validate_credential(config: &AppConfig, credential: StoredCredential) -> Result<()> {
    let auth = Authenticator::new(credential, None, None);
    let client = AdoClient::new(config.organization.clone(), auth);
    client.check_connection().await
}
