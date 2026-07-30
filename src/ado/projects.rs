use serde::{Deserialize, Serialize};

use crate::{
    ado::{
        client::{AdoClient, continuation_token_from_headers},
        users,
    },
    config::AdoService,
    debug,
    error::{AdoctlError, Result},
    identity::UserIdentifier,
    project_state::ProjectStateFilter,
};

const CORE_API_VERSION: &str = "7.1";
const GRAPH_API_VERSION: &str = "7.1-preview.1";
const PROJECTS_PAGE_SIZE: usize = 100;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub visibility: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub last_update_time: String,
}

#[derive(Debug, Default, Deserialize)]
struct ProjectListResponse {
    #[serde(default, alias = "items", alias = "members")]
    value: Vec<Project>,
}

#[derive(Debug, Deserialize)]
struct DescriptorResponse {
    value: String,
}

#[derive(Debug, Deserialize)]
struct GraphGroupsResponse {
    #[serde(default)]
    value: Vec<GraphGroup>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphGroup {
    pub descriptor: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub principal_name: String,
}

pub async fn list_projects(
    client: &AdoClient,
    state: ProjectStateFilter,
    search: Option<&str>,
) -> Result<Vec<Project>> {
    debug::log(format!(
        "列出專案：state={}，search={}",
        state,
        search.unwrap_or("<全部>")
    ));

    let mut all_projects = Vec::new();
    let mut continuation_token: Option<String> = None;

    loop {
        let mut query: Vec<(&str, String)> = vec![
            ("api-version", CORE_API_VERSION.to_owned()),
            ("$top", PROJECTS_PAGE_SIZE.to_string()),
            ("stateFilter", state.api_value().to_owned()),
        ];
        if let Some(token) = continuation_token.as_deref() {
            query.push(("continuationToken", token.to_owned()));
        }

        let (response, headers): (ProjectListResponse, _) = client
            .get_json_with_headers(AdoService::Core, "_apis/projects", &query)
            .await?;
        let page_len = response.value.len();
        continuation_token = continuation_token_from_headers(&headers);
        debug::log(format!(
            "projects 分頁：page_len={}，has_more={}",
            page_len,
            continuation_token.is_some()
        ));

        all_projects.extend(response.value);
        if continuation_token.is_none() {
            break;
        }
    }

    let total_projects = all_projects.len();
    let search = search
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_lowercase);
    let projects = all_projects
        .into_iter()
        .filter(|project| {
            search.as_ref().is_none_or(|search| {
                [
                    project.name.as_str(),
                    project.description.as_str(),
                    project.id.as_str(),
                ]
                .iter()
                .any(|value| value.to_lowercase().contains(search))
            })
        })
        .collect::<Vec<_>>();

    debug::log(format!(
        "專案清單：原始 {} 筆，過濾後 {} 筆。",
        total_projects,
        projects.len()
    ));

    Ok(projects)
}

pub async fn add_user_to_project(
    client: &AdoClient,
    project: &str,
    identifier: &UserIdentifier,
    group: &str,
) -> Result<()> {
    let user = users::get_user(client, identifier).await?;
    let user_descriptor = user
        .descriptor()
        .ok_or_else(|| AdoctlError::UserNotFound(identifier.label().to_owned()))?;
    let group = resolve_project_group(client, project, group).await?;

    client
        .put_empty(
            AdoService::Graph,
            &format!(
                "_apis/graph/memberships/{}/{}",
                urlencoding(user_descriptor),
                urlencoding(&group.descriptor)
            ),
            &[("api-version", GRAPH_API_VERSION.into())],
        )
        .await
}

pub async fn remove_user_from_project(
    client: &AdoClient,
    project: &str,
    identifier: &UserIdentifier,
    group: &str,
) -> Result<()> {
    let user = users::get_user(client, identifier).await?;
    let user_descriptor = user
        .descriptor()
        .ok_or_else(|| AdoctlError::UserNotFound(identifier.label().to_owned()))?;
    let group = resolve_project_group(client, project, group).await?;

    client
        .delete_empty(
            AdoService::Graph,
            &format!(
                "_apis/graph/memberships/{}/{}",
                urlencoding(user_descriptor),
                urlencoding(&group.descriptor)
            ),
            &[("api-version", GRAPH_API_VERSION.into())],
        )
        .await
}

async fn resolve_project_group(
    client: &AdoClient,
    project_name_or_id: &str,
    group_name_or_descriptor: &str,
) -> Result<GraphGroup> {
    if group_name_or_descriptor.starts_with("vssgp.")
        || group_name_or_descriptor.starts_with("aadgp.")
        || group_name_or_descriptor.starts_with("svc.")
    {
        return Ok(GraphGroup {
            descriptor: group_name_or_descriptor.to_owned(),
            display_name: group_name_or_descriptor.to_owned(),
            principal_name: String::new(),
        });
    }

    let project = get_project(client, project_name_or_id).await?;
    let descriptor = get_project_descriptor(client, &project.id).await?;
    let groups = list_project_groups(client, &descriptor).await?;

    groups
        .into_iter()
        .find(|group| {
            group
                .display_name
                .eq_ignore_ascii_case(group_name_or_descriptor)
                || group
                    .principal_name
                    .eq_ignore_ascii_case(group_name_or_descriptor)
        })
        .ok_or_else(|| AdoctlError::GroupNotFound(group_name_or_descriptor.to_owned()))
}

async fn get_project(client: &AdoClient, project_name_or_id: &str) -> Result<Project> {
    client
        .get_json(
            AdoService::Core,
            &format!("_apis/projects/{}", urlencoding(project_name_or_id)),
            &[("api-version", CORE_API_VERSION.into())],
        )
        .await
        .map_err(|error| match error {
            AdoctlError::Api { status, .. } if status.as_u16() == 404 => {
                AdoctlError::ProjectNotFound(project_name_or_id.to_owned())
            }
            other => other,
        })
}

async fn get_project_descriptor(client: &AdoClient, project_id: &str) -> Result<String> {
    let response: DescriptorResponse = client
        .get_json(
            AdoService::Graph,
            &format!("_apis/graph/descriptors/{}", urlencoding(project_id)),
            &[("api-version", GRAPH_API_VERSION.into())],
        )
        .await?;
    Ok(response.value)
}

async fn list_project_groups(
    client: &AdoClient,
    scope_descriptor: &str,
) -> Result<Vec<GraphGroup>> {
    let response: GraphGroupsResponse = client
        .get_json(
            AdoService::Graph,
            "_apis/graph/groups",
            &[
                ("scopeDescriptor", scope_descriptor.to_owned()),
                ("api-version", GRAPH_API_VERSION.into()),
            ],
        )
        .await?;
    Ok(response.value)
}

fn urlencoding(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use reqwest::header::{HeaderMap, HeaderValue};

    use crate::ado::client::continuation_token_from_headers;

    #[test]
    fn reads_continuation_token_from_response_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-ms-continuationtoken",
            HeaderValue::from_static("token-123"),
        );

        assert_eq!(
            continuation_token_from_headers(&headers),
            Some("token-123".into())
        );
    }
}
