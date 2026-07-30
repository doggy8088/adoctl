use adoctl::{
    access_level::AccessLevel,
    ado::{client::AdoClient, users},
    auth::Authenticator,
    config::Organization,
    credentials::StoredCredential,
    identity::UserIdentifier,
};
use assert_cmd::Command;
use predicates::str::contains;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header_exists, method, path, query_param},
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

fn user_payload(id: usize) -> serde_json::Value {
    serde_json::json!({
        "id": id.to_string(),
        "user": {
            "principalName": format!("user{id}@example.com"),
            "displayName": format!("User {id}")
        },
        "accessLevel": {
            "accountLicenseType": "express",
            "licenseDisplayName": "Basic",
            "status": "active"
        }
    })
}

#[tokio::test]
async fn cli_uses_azure_devops_ext_pat_env_without_saved_login() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/_apis/userentitlements"))
        .and(query_param("api-version", "7.1-preview.4"))
        .and(header_exists("authorization"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [
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
            ],
            "count": 1
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
            "user",
            "list",
        ])
        .assert()
        .success()
        .stdout(contains("will@example.com"));
}

#[tokio::test]
async fn debug_flag_writes_verbose_logs_to_stderr() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/_apis/userentitlements"))
        .and(query_param("api-version", "7.1-preview.4"))
        .and(header_exists("authorization"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [
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
            ],
            "count": 1
        })))
        .mount(&server)
        .await;

    let mut command = Command::cargo_bin("adoctl").unwrap();
    command
        .env("AZURE_DEVOPS_EXT_PAT", "fake-token")
        .env_remove("ADOCTL_PAT")
        .args([
            "--debug",
            "--org",
            "my-org",
            "--base-url",
            &server.uri(),
            "--output",
            "json",
            "user",
            "list",
        ])
        .assert()
        .success()
        .stdout(contains("\"principalName\": \"will@example.com\""))
        .stderr(contains("HTTP 要求"))
        .stderr(contains("HTTP JSON 摘要"));
}

#[tokio::test]
async fn list_users_filters_by_access_level_and_search() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/_apis/userentitlements"))
        .and(query_param("api-version", "7.1-preview.4"))
        .and(header_exists("authorization"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [
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
                },
                {
                    "id": "2",
                    "user": {
                        "principalName": "other@example.com",
                        "displayName": "Other"
                    },
                    "accessLevel": {
                        "accountLicenseType": "stakeholder",
                        "licenseDisplayName": "Stakeholder",
                        "status": "active"
                    }
                }
            ],
            "count": 2
        })))
        .mount(&server)
        .await;

    let result = users::list_users(
        &test_client(&server),
        Some(AccessLevel::Basic),
        Some("will"),
    )
    .await
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].upn(), "will@example.com");
}

#[tokio::test]
async fn list_users_reads_all_pages_before_filtering() {
    let server = MockServer::start().await;
    let first_page = (0..100).map(user_payload).collect::<Vec<_>>();
    let second_page = vec![user_payload(100)];

    Mock::given(method("GET"))
        .and(path("/_apis/userentitlements"))
        .and(query_param("api-version", "7.1-preview.4"))
        .and(query_param("top", "100"))
        .and(query_param("skip", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": first_page,
            "count": 100
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/_apis/userentitlements"))
        .and(query_param("api-version", "7.1-preview.4"))
        .and(query_param("top", "100"))
        .and(query_param("skip", "100"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": second_page,
            "count": 1
        })))
        .mount(&server)
        .await;

    let result = users::list_users(&test_client(&server), None, Some("user100"))
        .await
        .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].upn(), "user100@example.com");
}

#[tokio::test]
async fn get_user_by_upn_requires_exact_match() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/_apis/userentitlements"))
        .and(query_param("api-version", "7.1-preview.4"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [
                {
                    "id": "1",
                    "user": {
                        "principalName": "will@example.com",
                        "displayName": "Will"
                    },
                    "accessLevel": {
                        "accountLicenseType": "express"
                    }
                }
            ],
            "count": 1
        })))
        .mount(&server)
        .await;

    let result = users::get_user(
        &test_client(&server),
        &UserIdentifier::Upn("will@example.com".into()),
    )
    .await
    .unwrap();

    assert_eq!(result.id, "1");
}

#[tokio::test]
async fn set_access_level_uses_json_patch() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/_apis/userentitlements/1"))
        .and(query_param("api-version", "7.1-preview.4"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "1",
            "user": {
                "principalName": "will@example.com",
                "descriptor": "aad.user"
            },
            "accessLevel": {
                "accountLicenseType": "express"
            }
        })))
        .mount(&server)
        .await;

    Mock::given(method("PATCH"))
        .and(path("/_apis/userentitlements/1"))
        .and(query_param("api-version", "7.1-preview.4"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "1",
            "user": {
                "principalName": "will@example.com"
            },
            "accessLevel": {
                "accountLicenseType": "stakeholder"
            }
        })))
        .mount(&server)
        .await;

    let result = users::set_access_level(
        &test_client(&server),
        &UserIdentifier::Id("1".into()),
        AccessLevel::Stakeholder,
    )
    .await
    .unwrap();

    assert_eq!(result.access_level.account_license_type, "stakeholder");
}
