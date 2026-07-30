use adoctl::{
    ado::{client::AdoClient, pools},
    auth::Authenticator,
    config::Organization,
    credentials::StoredCredential,
    error::AdoctlError,
    pool_type::PoolTypeFilter,
};
use assert_cmd::Command;
use predicates::str::contains;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header_exists, method, path, query_param, query_param_is_missing},
};

fn test_client(server: &MockServer) -> AdoClient {
    let org = Organization::parse("my-org", Some(server.uri())).unwrap();
    let auth = Authenticator::new(
        StoredCredential::Pat {
            token: "secret".into(),
        },
        None,
        None,
    );
    AdoClient::new(org, auth)
}

#[tokio::test]
async fn list_agent_pools_filters_by_pool_type() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/_apis/distributedtask/pools"))
        .and(query_param("api-version", "7.1"))
        .and(query_param("poolType", "automation"))
        .and(header_exists("authorization"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "count": 1,
            "value": [
                {
                    "id": 42,
                    "name": "Default",
                    "poolType": "automation",
                    "isHosted": false,
                    "size": 2,
                    "targetSize": null,
                    "autoSize": false,
                    "autoProvision": true
                }
            ]
        })))
        .mount(&server)
        .await;

    let result = pools::list_agent_pools(&test_client(&server), Some(PoolTypeFilter::Automation))
        .await
        .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].id, 42);
    assert_eq!(result[0].pool_type, "automation");
}

#[tokio::test]
async fn list_agents_resolves_pool_name_and_includes_status() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/_apis/distributedtask/pools"))
        .and(query_param("api-version", "7.1"))
        .and(query_param("poolName", "Linux Pool"))
        .and(query_param_is_missing("poolType"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "count": 1,
            "value": [
                {
                    "id": 42,
                    "name": "Linux Pool",
                    "poolType": "automation"
                }
            ]
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/_apis/distributedtask/pools/42/agents"))
        .and(query_param("includeAssignedRequest", "true"))
        .and(query_param("api-version", "7.1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "count": 1,
            "value": [
                {
                    "id": 7,
                    "name": "linux-agent-01",
                    "enabled": true,
                    "status": "online",
                    "version": "4.255.0",
                    "osDescription": "Ubuntu 24.04",
                    "statusChangedOn": "2026-07-30T01:00:00Z",
                    "assignedRequest": {
                        "requestId": 1001,
                        "jobName": "Build"
                    }
                }
            ]
        })))
        .mount(&server)
        .await;

    let result = pools::list_agents(&test_client(&server), "Linux Pool")
        .await
        .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].status, "online");
    assert_eq!(
        result[0]
            .assigned_request
            .as_ref()
            .unwrap()
            .job_name
            .as_deref(),
        Some("Build")
    );
}

#[tokio::test]
async fn list_jobs_reads_all_pages_using_continuation_token() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/_apis/distributedtask/pools/42/jobrequests"))
        .and(query_param("$top", "100"))
        .and(query_param("api-version", "7.1-preview.1"))
        .and(query_param_is_missing("continuationToken"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-ms-continuationtoken", "token-1")
                .set_body_json(serde_json::json!({
                    "count": 1,
                    "value": [
                        {
                            "requestId": 1001,
                            "poolId": 42,
                            "jobName": "Build",
                            "queueTime": "2026-07-30T01:00:00Z"
                        }
                    ]
                })),
        )
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/_apis/distributedtask/pools/42/jobrequests"))
        .and(query_param("$top", "100"))
        .and(query_param("api-version", "7.1-preview.1"))
        .and(query_param("continuationToken", "token-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "requestId": 1000,
                "poolId": 42,
                "jobName": "Test",
                "queueTime": "2026-07-29T01:00:00Z",
                "finishTime": "2026-07-29T01:05:00Z",
                "result": "succeeded",
                "reservedAgent": {
                    "id": 7,
                    "name": "linux-agent-01",
                    "enabled": true,
                    "status": "online"
                }
            }
        ])))
        .mount(&server)
        .await;

    let result = pools::list_jobs(&test_client(&server), "42").await.unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].request_id, 1001);
    assert_eq!(result[1].request_id, 1000);
    assert_eq!(result[1].result.as_deref(), Some("succeeded"));
}

#[tokio::test]
async fn list_agents_reports_pool_not_found_for_unknown_name() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/_apis/distributedtask/pools"))
        .and(query_param("api-version", "7.1"))
        .and(query_param("poolName", "Missing Pool"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "count": 0,
            "value": []
        })))
        .mount(&server)
        .await;

    let error = pools::list_agents(&test_client(&server), "Missing Pool")
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        AdoctlError::PoolNotFound(ref name) if name == "Missing Pool"
    ));
}

#[tokio::test]
async fn list_agents_maps_api_404_to_pool_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/_apis/distributedtask/pools/99/agents"))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_json(serde_json::json!({"message": "Pool not found"})),
        )
        .mount(&server)
        .await;

    let error = pools::list_agents(&test_client(&server), "99")
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        AdoctlError::PoolNotFound(ref name) if name == "99"
    ));
}

#[tokio::test]
async fn list_jobs_preserves_api_errors() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/_apis/distributedtask/pools/42/jobrequests"))
        .respond_with(
            ResponseTemplate::new(500)
                .set_body_json(serde_json::json!({"message": "service unavailable"})),
        )
        .mount(&server)
        .await;

    let error = pools::list_jobs(&test_client(&server), "42")
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        AdoctlError::Api { status, .. } if status.as_u16() == 500
    ));
}

#[tokio::test]
async fn cli_renders_agent_status_in_traditional_chinese() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/_apis/distributedtask/pools/42/agents"))
        .and(query_param("includeAssignedRequest", "true"))
        .and(query_param("api-version", "7.1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "count": 1,
            "value": [
                {
                    "id": 7,
                    "name": "linux-agent-01",
                    "enabled": true,
                    "status": "online",
                    "version": "4.255.0",
                    "osDescription": "Ubuntu 24.04"
                }
            ]
        })))
        .mount(&server)
        .await;

    let mut command = Command::cargo_bin("adoctl").unwrap();
    command
        .env("AZURE_DEVOPS_EXT_PAT", "fake-token")
        .env_remove("ADOCTL_PAT")
        .args([
            "--org",
            "my-org",
            "--base-url",
            &server.uri(),
            "pool",
            "agents",
            "--pool",
            "42",
        ])
        .assert()
        .success()
        .stdout(contains("linux-agent-01"))
        .stdout(contains("上線"));
}

#[tokio::test]
async fn cli_outputs_stable_agent_pool_json() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/_apis/distributedtask/pools"))
        .and(query_param("api-version", "7.1"))
        .and(query_param_is_missing("poolType"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "count": 1,
            "value": [
                {
                    "id": 43,
                    "name": "Deployment Pool",
                    "poolType": "deployment",
                    "isHosted": false,
                    "size": 3,
                    "targetSize": null,
                    "autoSize": false,
                    "autoProvision": false
                }
            ]
        })))
        .mount(&server)
        .await;

    let mut command = Command::cargo_bin("adoctl").unwrap();
    command
        .env("AZURE_DEVOPS_EXT_PAT", "fake-token")
        .env_remove("ADOCTL_PAT")
        .args([
            "--org",
            "my-org",
            "--base-url",
            &server.uri(),
            "--output",
            "json",
            "pool",
            "list",
        ])
        .assert()
        .success()
        .stdout(contains("\"id\": 43"))
        .stdout(contains("\"poolType\": \"deployment\""))
        .stdout(contains("\"targetSize\": null"));
}
