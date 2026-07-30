use reqwest::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdoctlError {
    #[error("缺少 Azure DevOps 組織資訊，請使用 --org 或設定 ADOCTL_ORG。")]
    MissingOrganization,

    #[error("Azure DevOps 組織格式無效：{0}")]
    InvalidOrganization(String),

    #[error("缺少使用者識別資訊，請提供 --upn 或 --id。")]
    MissingUserIdentifier,

    #[error("找不到使用者：{0}")]
    UserNotFound(String),

    #[error("找到多個符合條件的使用者：{0}")]
    AmbiguousUser(String),

    #[error("找不到專案：{0}")]
    ProjectNotFound(String),

    #[error("找不到專案群組：{0}")]
    GroupNotFound(String),

    #[error("找不到代理程式集區：{0}")]
    PoolNotFound(String),

    #[error("找到多個同名代理程式集區：{0}")]
    AmbiguousPool(String),

    #[error("API 分頁處理失敗：{0}")]
    Pagination(String),

    #[error("尚未登入，請先執行 adoctl login --org <organization>。")]
    NotLoggedIn,

    #[error("憑證存取失敗：{0}")]
    CredentialStore(String),

    #[error("驗證失敗：{0}")]
    Authentication(String),

    #[error("Azure CLI 執行失敗：{0}")]
    AzureCli(String),

    #[error("Azure DevOps API 回傳錯誤（{status}）：{message}")]
    Api { status: StatusCode, message: String },

    #[error("HTTP 請求失敗：{0}")]
    Http(#[from] reqwest::Error),

    #[error("URL 處理失敗：{0}")]
    Url(#[from] url::ParseError),

    #[error("JSON 處理失敗：{0}")]
    Json(#[from] serde_json::Error),

    #[error("I/O 失敗：{0}")]
    Io(#[from] std::io::Error),
}

impl AdoctlError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::MissingOrganization
            | Self::InvalidOrganization(_)
            | Self::MissingUserIdentifier
            | Self::NotLoggedIn => 2,
            Self::Authentication(_) | Self::AzureCli(_) | Self::CredentialStore(_) => 3,
            Self::Api { status, .. } if status.is_client_error() => 4,
            Self::UserNotFound(_)
            | Self::AmbiguousUser(_)
            | Self::ProjectNotFound(_)
            | Self::GroupNotFound(_)
            | Self::PoolNotFound(_)
            | Self::AmbiguousPool(_) => 4,
            _ => 1,
        }
    }
}

pub type Result<T> = std::result::Result<T, AdoctlError>;
