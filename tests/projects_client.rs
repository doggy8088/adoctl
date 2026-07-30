use adoctl::{
    ado::{client::AdoClient, projects},
    auth::Authenticator,
    config::Organization,
    credentials::StoredCredential,
    project_state::ProjectStateFilter,
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

fn project_payload(id: usize) -> serde_json::Value {
    serde_json::json!({
        "id": format!("project-{id}"),
        "name": format!("Project {id}"),
        "description": format!("Description {id}"),
        "state": "wellFormed",
        "visibility": "private",
        "lastUpdateTime": format!("2025-01-{day:02}T00:00:00Z", day = (id % 28) + 1)
    })
}

#[tokio::test]
async fn cli_lists_projects_with_azure_devops_ext_pat_env() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/_apis/projects"))
        .and(query_param("api-version", "7.1"))
        .and(query_param("$top", "100"))
        .and(query_param("stateFilter", "all"))
        .and(header_exists("authorization"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "count": 1,
            "value": [
                {
                    "id": "project-1",
                    "name": "Platform",
                    "description": "Core platform",
                    "state": "wellFormed",
                    "visibility": "private",
                    "lastUpdateTime": "2025-01-01T00:00:00Z"
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
            "project",
            "list",
        ])
        .assert()
        .success()
        .stdout(contains("Platform"));
}

#[tokio::test]
async fn list_projects_filters_by_search_and_state() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/_apis/projects"))
        .and(query_param("api-version", "7.1"))
        .and(query_param("$top", "100"))
        .and(query_param("stateFilter", "wellFormed"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "count": 2,
            "value": [
                {
                    "id": "project-1",
                    "name": "Legacy Migration",
                    "description": "Legacy workloads",
                    "state": "wellFormed",
                    "visibility": "private",
                    "lastUpdateTime": "2025-01-01T00:00:00Z"
                },
                {
                    "id": "project-2",
                    "name": "Platform",
                    "description": "Core platform",
                    "state": "wellFormed",
                    "visibility": "private",
                    "lastUpdateTime": "2025-01-02T00:00:00Z"
                }
            ]
        })))
        .mount(&server)
        .await;

    let result = projects::list_projects(
        &test_client(&server),
        ProjectStateFilter::WellFormed,
        Some("legacy"),
    )
    .await
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "Legacy Migration");
}

#[tokio::test]
async fn list_projects_reads_all_pages_using_continuation_token() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/_apis/projects"))
        .and(query_param("api-version", "7.1"))
        .and(query_param("$top", "100"))
        .and(query_param("stateFilter", "all"))
        .and(query_param_is_missing("continuationToken"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-ms-continuationtoken", "token-1")
                .set_body_json(serde_json::json!({
                    "count": 1,
                    "value": [project_payload(1)]
                })),
        )
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/_apis/projects"))
        .and(query_param("api-version", "7.1"))
        .and(query_param("$top", "100"))
        .and(query_param("stateFilter", "all"))
        .and(query_param("continuationToken", "token-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "count": 1,
            "value": [project_payload(2)]
        })))
        .mount(&server)
        .await;

    let result = projects::list_projects(&test_client(&server), ProjectStateFilter::All, None)
        .await
        .unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].id, "project-1");
    assert_eq!(result[1].id, "project-2");
}
