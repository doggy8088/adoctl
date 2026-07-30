use serde::{Deserialize, Serialize};

use crate::{
    access_level::AccessLevel,
    ado::client::AdoClient,
    config::AdoService,
    debug,
    error::{AdoctlError, Result},
    identity::UserIdentifier,
};

const USER_ENTITLEMENTS_API_VERSION: &str = "7.1-preview.4";
const USER_ENTITLEMENTS_PAGE_SIZE: usize = 100;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserEntitlementList {
    #[serde(default, alias = "items", alias = "value")]
    pub members: Vec<UserEntitlement>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserEntitlement {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub user: AdoUser,
    #[serde(default)]
    pub access_level: AccessLevelInfo,
    #[serde(default)]
    pub project_entitlements: Vec<ProjectEntitlement>,
}

impl UserEntitlement {
    pub fn upn(&self) -> String {
        first_non_empty([
            self.user.principal_name.as_str(),
            self.user.mail_address.as_str(),
            self.user.mail.as_str(),
        ])
    }

    pub fn display_name(&self) -> String {
        if self.user.display_name.trim().is_empty() {
            self.upn()
        } else {
            self.user.display_name.clone()
        }
    }

    pub fn descriptor(&self) -> Option<&str> {
        none_if_empty(&self.user.descriptor)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoUser {
    #[serde(default)]
    pub principal_name: String,
    #[serde(default)]
    pub mail_address: String,
    #[serde(default)]
    pub mail: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub descriptor: String,
    #[serde(default)]
    pub origin_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessLevelInfo {
    #[serde(default)]
    pub account_license_type: String,
    #[serde(default)]
    pub license_display_name: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEntitlement {
    #[serde(default)]
    pub project_ref: ProjectRef,
    #[serde(default)]
    pub group: ProjectGroup,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectRef {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectGroup {
    #[serde(default)]
    pub group_type: String,
    #[serde(default)]
    pub display_name: String,
}

pub async fn list_users(
    client: &AdoClient,
    access_level: Option<AccessLevel>,
    search: Option<&str>,
) -> Result<Vec<UserEntitlement>> {
    debug::log(format!(
        "列出使用者：access-level={}，search={}",
        access_level.map(AccessLevel::cli_name).unwrap_or("<全部>"),
        search.unwrap_or("<全部>")
    ));

    let mut all_members = Vec::new();
    let mut skip = 0;

    loop {
        let response: UserEntitlementList = client
            .get_json(
                AdoService::Entitlements,
                "_apis/userentitlements",
                &[
                    ("api-version", USER_ENTITLEMENTS_API_VERSION.into()),
                    ("top", USER_ENTITLEMENTS_PAGE_SIZE.to_string()),
                    ("skip", skip.to_string()),
                ],
            )
            .await?;
        let page_len = response.members.len();
        debug::log(format!(
            "user entitlements 分頁：skip={skip}，page_len={page_len}"
        ));

        all_members.extend(response.members);
        if page_len < USER_ENTITLEMENTS_PAGE_SIZE {
            break;
        }

        skip += USER_ENTITLEMENTS_PAGE_SIZE;
    }

    let total_users = all_members.len();
    let search = search.map(|value| value.trim().to_ascii_lowercase());
    let users = all_members
        .into_iter()
        .filter(|user| {
            access_level.is_none_or(|level| {
                level.matches_api_value(&user.access_level.account_license_type)
            })
        })
        .filter(|user| {
            search.as_ref().is_none_or(|search| {
                [
                    user.user.display_name.as_str(),
                    user.user.principal_name.as_str(),
                    user.user.mail_address.as_str(),
                    user.user.mail.as_str(),
                ]
                .iter()
                .any(|value| value.to_ascii_lowercase().contains(search))
            })
        })
        .collect::<Vec<_>>();

    debug::log(format!(
        "使用者清單：原始 {} 筆，過濾後 {} 筆。",
        total_users,
        users.len()
    ));

    Ok(users)
}

pub async fn get_user(client: &AdoClient, identifier: &UserIdentifier) -> Result<UserEntitlement> {
    match identifier {
        UserIdentifier::Id(id) => get_user_by_id(client, id).await,
        UserIdentifier::Upn(upn) => get_user_by_upn(client, upn).await,
    }
}

pub async fn get_user_by_id(client: &AdoClient, id: &str) -> Result<UserEntitlement> {
    client
        .get_json(
            AdoService::Entitlements,
            &format!("_apis/userentitlements/{id}"),
            &[("api-version", USER_ENTITLEMENTS_API_VERSION.into())],
        )
        .await
}

pub async fn get_user_by_upn(client: &AdoClient, upn: &str) -> Result<UserEntitlement> {
    let users = list_users(client, None, Some(upn)).await?;
    let mut exact_matches = users
        .into_iter()
        .filter(|user| {
            user.user.principal_name.eq_ignore_ascii_case(upn)
                || user.user.mail_address.eq_ignore_ascii_case(upn)
                || user.user.mail.eq_ignore_ascii_case(upn)
        })
        .collect::<Vec<_>>();

    match exact_matches.len() {
        0 => Err(AdoctlError::UserNotFound(upn.to_owned())),
        1 => Ok(exact_matches.remove(0)),
        _ => Err(AdoctlError::AmbiguousUser(upn.to_owned())),
    }
}

pub async fn set_access_level(
    client: &AdoClient,
    identifier: &UserIdentifier,
    access_level: AccessLevel,
) -> Result<UserEntitlement> {
    let user = get_user(client, identifier).await?;
    let patch = vec![JsonPatchOperation {
        op: "replace",
        path: "/accessLevel",
        value: AccessLevelPatch {
            account_license_type: access_level.api_account_license_type(),
            licensing_source: "account",
        },
    }];

    client
        .patch_json(
            AdoService::Entitlements,
            &format!("_apis/userentitlements/{}", user.id),
            &[("api-version", USER_ENTITLEMENTS_API_VERSION.into())],
            &patch,
            Some("application/json-patch+json"),
        )
        .await
}

#[derive(Debug, Serialize)]
struct JsonPatchOperation<'a> {
    op: &'a str,
    path: &'a str,
    value: AccessLevelPatch<'a>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccessLevelPatch<'a> {
    account_license_type: &'a str,
    licensing_source: &'a str,
}

fn first_non_empty<'a>(values: impl IntoIterator<Item = &'a str>) -> String {
    values
        .into_iter()
        .find(|value| !value.trim().is_empty())
        .unwrap_or("")
        .to_owned()
}

fn none_if_empty(value: &str) -> Option<&str> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::UserEntitlementList;

    fn sample_user_list(key: &str) -> UserEntitlementList {
        serde_json::from_value(serde_json::json!({
            key: [
                {
                    "id": "1",
                    "user": {
                        "principalName": "will@example.com",
                        "displayName": "Will"
                    },
                    "accessLevel": {
                        "accountLicenseType": "express",
                        "licenseDisplayName": "Basic",
                        "status": "active"
                    }
                }
            ]
        }))
        .unwrap()
    }

    #[test]
    fn deserializes_items_key_from_user_entitlements_list() {
        let list = sample_user_list("items");
        assert_eq!(list.members.len(), 1);
        assert_eq!(list.members[0].upn(), "will@example.com");
    }

    #[test]
    fn deserializes_members_key_from_user_entitlements_list() {
        let list = sample_user_list("members");
        assert_eq!(list.members.len(), 1);
        assert_eq!(list.members[0].display_name(), "Will");
    }
}
