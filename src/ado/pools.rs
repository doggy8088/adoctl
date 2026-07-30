use std::collections::HashSet;

use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};

use crate::{
    ado::client::{AdoClient, continuation_token_from_headers},
    config::AdoService,
    debug,
    error::{AdoctlError, Result},
    pool_type::PoolTypeFilter,
};

const DISTRIBUTED_TASK_API_VERSION: &str = "7.1";
const JOB_REQUESTS_API_VERSION: &str = "7.1-preview.1";
const JOB_REQUESTS_PAGE_SIZE: usize = 100;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPool {
    #[serde(default)]
    pub id: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub pool_type: String,
    #[serde(default)]
    pub is_hosted: bool,
    #[serde(default)]
    pub size: u32,
    #[serde(default)]
    pub target_size: Option<u32>,
    #[serde(default)]
    pub auto_size: bool,
    #[serde(default)]
    pub auto_provision: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Agent {
    #[serde(default)]
    pub id: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub os_description: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub status_changed_on: String,
    #[serde(default)]
    pub provisioning_state: String,
    #[serde(default)]
    pub assigned_request: Option<JobRequest>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentReference {
    #[serde(default)]
    pub id: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationOwner {
    #[serde(default)]
    pub id: u32,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobRequest {
    #[serde(default)]
    pub request_id: u64,
    #[serde(default)]
    pub pool_id: u32,
    #[serde(default)]
    pub queue_id: u32,
    #[serde(default)]
    pub job_id: Option<String>,
    #[serde(default)]
    pub job_name: Option<String>,
    #[serde(default)]
    pub plan_id: Option<String>,
    #[serde(default)]
    pub plan_type: Option<String>,
    #[serde(default)]
    pub queue_time: Option<String>,
    #[serde(default)]
    pub assign_time: Option<String>,
    #[serde(default)]
    pub receive_time: Option<String>,
    #[serde(default)]
    pub finish_time: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub status_message: Option<String>,
    #[serde(default)]
    pub reserved_agent: Option<AgentReference>,
    #[serde(default)]
    pub definition: Option<OrchestrationOwner>,
    #[serde(default)]
    pub owner: Option<OrchestrationOwner>,
}

impl JobRequest {
    pub fn state(&self) -> JobRequestState {
        if self
            .finish_time
            .as_deref()
            .is_some_and(|value| !value.is_empty())
            || self
                .result
                .as_deref()
                .is_some_and(|value| !value.is_empty())
        {
            JobRequestState::Completed
        } else if self
            .receive_time
            .as_deref()
            .is_some_and(|value| !value.is_empty())
            || self
                .assign_time
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            || self.reserved_agent.is_some()
        {
            JobRequestState::Running
        } else {
            JobRequestState::Queued
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobRequestState {
    Queued,
    Running,
    Completed,
}

#[derive(Debug, Default, Deserialize)]
struct AgentPoolListResponse {
    #[serde(default, alias = "items", alias = "members")]
    value: Vec<AgentPool>,
}

#[derive(Debug, Default, Deserialize)]
struct AgentListResponse {
    #[serde(default, alias = "items", alias = "members")]
    value: Vec<Agent>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum JobRequestListResponse {
    Wrapped {
        #[serde(default, alias = "items", alias = "members")]
        value: Vec<JobRequest>,
    },
    Direct(Vec<JobRequest>),
}

impl JobRequestListResponse {
    fn into_vec(self) -> Vec<JobRequest> {
        match self {
            Self::Wrapped { value } | Self::Direct(value) => value,
        }
    }
}

pub async fn list_agent_pools(
    client: &AdoClient,
    pool_type: Option<PoolTypeFilter>,
) -> Result<Vec<AgentPool>> {
    get_agent_pools(client, pool_type, None).await
}

pub async fn list_agents(client: &AdoClient, pool: &str) -> Result<Vec<Agent>> {
    let pool_id = resolve_pool_id(client, pool).await?;
    debug::log(format!("列出代理程式：pool={}，pool_id={pool_id}", pool));

    let response: AgentListResponse = client
        .get_json(
            AdoService::Core,
            &format!("_apis/distributedtask/pools/{pool_id}/agents"),
            &[
                ("includeAssignedRequest", "true".into()),
                ("api-version", DISTRIBUTED_TASK_API_VERSION.into()),
            ],
        )
        .await
        .map_err(|error| map_pool_not_found(error, pool))?;

    debug::log(format!("代理程式清單：{} 筆。", response.value.len()));
    Ok(response.value)
}

pub async fn list_jobs(client: &AdoClient, pool: &str) -> Result<Vec<JobRequest>> {
    let pool_id = resolve_pool_id(client, pool).await?;
    debug::log(format!("列出工作：pool={}，pool_id={pool_id}", pool));

    let mut all_jobs = Vec::new();
    let mut continuation_token: Option<String> = None;
    let mut seen_tokens = HashSet::new();

    loop {
        let mut query = vec![
            ("$top", JOB_REQUESTS_PAGE_SIZE.to_string()),
            ("api-version", JOB_REQUESTS_API_VERSION.into()),
        ];
        if let Some(token) = continuation_token.as_deref() {
            query.push(("continuationToken", token.to_owned()));
        }

        let (response, headers): (JobRequestListResponse, HeaderMap) = client
            .get_json_with_headers(
                AdoService::Core,
                &format!("_apis/distributedtask/pools/{pool_id}/jobrequests"),
                &query,
            )
            .await
            .map_err(|error| map_pool_not_found(error, pool))?;
        let jobs = response.into_vec();
        let page_len = jobs.len();
        all_jobs.extend(jobs);

        continuation_token = continuation_token_from_headers(&headers);
        debug::log(format!(
            "工作分頁：page_len={}，has_more={}",
            page_len,
            continuation_token.is_some()
        ));

        match continuation_token.as_ref() {
            Some(token) if !seen_tokens.insert(token.clone()) => {
                return Err(AdoctlError::Pagination(
                    "工作清單 API 重複回傳相同 continuation token。".into(),
                ));
            }
            Some(_) => {}
            None => break,
        }
    }

    debug::log(format!("工作清單：共 {} 筆。", all_jobs.len()));
    Ok(all_jobs)
}

async fn get_agent_pools(
    client: &AdoClient,
    pool_type: Option<PoolTypeFilter>,
    pool_name: Option<&str>,
) -> Result<Vec<AgentPool>> {
    debug::log(format!(
        "列出代理程式集區：pool_type={}，pool_name={}",
        pool_type.map(|value| value.api_value()).unwrap_or("<全部>"),
        pool_name.unwrap_or("<全部>")
    ));

    let mut query = vec![("api-version", DISTRIBUTED_TASK_API_VERSION.into())];
    if let Some(pool_type) = pool_type {
        query.push(("poolType", pool_type.api_value().into()));
    }
    if let Some(pool_name) = pool_name {
        query.push(("poolName", pool_name.to_owned()));
    }

    let response: AgentPoolListResponse = client
        .get_json(AdoService::Core, "_apis/distributedtask/pools", &query)
        .await?;

    debug::log(format!("代理程式集區清單：{} 筆。", response.value.len()));
    Ok(response.value)
}

async fn resolve_pool_id(client: &AdoClient, pool: &str) -> Result<u32> {
    let pool = pool.trim();
    if let Ok(pool_id) = pool.parse::<u32>() {
        return Ok(pool_id);
    }

    let matches = get_agent_pools(client, None, Some(pool))
        .await?
        .into_iter()
        .filter(|candidate| candidate.name.eq_ignore_ascii_case(pool))
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [matched] => Ok(matched.id),
        [] => Err(AdoctlError::PoolNotFound(pool.to_owned())),
        _ => Err(AdoctlError::AmbiguousPool(pool.to_owned())),
    }
}

fn map_pool_not_found(error: AdoctlError, pool: &str) -> AdoctlError {
    match error {
        AdoctlError::Api { status, .. } if status.as_u16() == 404 => {
            AdoctlError::PoolNotFound(pool.to_owned())
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::{JobRequest, JobRequestState};

    #[test]
    fn derives_job_request_state_from_timestamps_and_result() {
        assert_eq!(JobRequest::default().state(), JobRequestState::Queued);
        assert_eq!(
            JobRequest {
                assign_time: Some("2026-07-30T01:00:00Z".into()),
                ..JobRequest::default()
            }
            .state(),
            JobRequestState::Running
        );
        assert_eq!(
            JobRequest {
                result: Some("succeeded".into()),
                ..JobRequest::default()
            }
            .state(),
            JobRequestState::Completed
        );
    }

    #[test]
    fn accepts_null_values_for_incomplete_job_requests() {
        let request: JobRequest = serde_json::from_value(serde_json::json!({
            "requestId": 1001,
            "jobName": "Build",
            "assignTime": null,
            "receiveTime": null,
            "finishTime": null,
            "result": null
        }))
        .unwrap();

        assert_eq!(request.state(), JobRequestState::Queued);
    }
}
