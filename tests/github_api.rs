use api_cli::{ApiClientRequest, CollectionModel, RequestModel};
use reqwest::StatusCode;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_get_user_repositories() {
    let server = spawn_mock_server().await;

    let c = load_test_collection();
    let r = load_test_request("Repository/GetUserRepositories");

    let variables = HashMap::from([("host".to_string(), server.base_url)]);

    let req = ApiClientRequest::new(c, r).with_override_variables(variables);

    Mock::given(matchers::method("GET"))
        .and(matchers::path("/users/mathieu-lemay/repos"))
        .and(matchers::query_param("per_page", "10"))
        .and(matchers::header("X-GitHub-Api-Version", "2022-11-28"))
        .and(matchers::header("Accept", "application/vnd.github+json"))
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .expect(1)
        .mount(&server.mock)
        .await;

    req.execute().await.unwrap();
}

#[tokio::test]
async fn test_create_repository() {
    let server = spawn_mock_server().await;

    let c = load_test_collection();
    let r = load_test_request("Repository/CreateRepositoryForAuthenticatedUser");

    let token = Uuid::new_v4();
    let variables = HashMap::from([
        ("host".to_string(), server.base_url),
        ("authToken".to_string(), token.to_string()),
    ]);

    let req = ApiClientRequest::new(c, r).with_override_variables(variables);

    let expected_body = json!(
        {
          "name": "Test Repo",
          "description": "Test Repo from API Cli",
          "homepage": "https://github.com/mathieu-lemay/api-cli",
          "private": true,
          "is_template": false,
        }
    );
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/user/repos"))
        .and(matchers::body_json(expected_body))
        .and(matchers::header(
            "Authorization",
            format!("Bearer {}", token),
        ))
        .and(matchers::header("X-GitHub-Api-Version", "2022-11-28"))
        .and(matchers::header("Accept", "application/vnd.github+json"))
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .expect(1)
        .mount(&server.mock)
        .await;

    req.execute().await.unwrap();
}

#[tokio::test]
async fn test_get_user() {
    let server = spawn_mock_server().await;

    let c = load_test_collection();
    let r = load_test_request("User/GetUser");

    let variables = HashMap::from([("host".to_string(), server.base_url)]);

    let req = ApiClientRequest::new(c, r).with_override_variables(variables);

    Mock::given(matchers::method("GET"))
        .and(matchers::path("/users/mathieu-lemay"))
        .and(matchers::header("X-GitHub-Api-Version", "2022-11-28"))
        .and(matchers::header("Accept", "application/vnd.github+json"))
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .expect(1)
        .mount(&server.mock)
        .await;

    req.execute().await.unwrap();
}

#[tokio::test]
async fn test_get_authenticated_user() {
    let server = spawn_mock_server().await;

    let c = load_test_collection();
    let r = load_test_request("User/GetAuthenticatedUser");

    let token = Uuid::new_v4();
    let variables = HashMap::from([
        ("host".to_string(), server.base_url),
        ("authToken".to_string(), token.to_string()),
    ]);

    let req = ApiClientRequest::new(c, r).with_override_variables(variables);

    Mock::given(matchers::method("GET"))
        .and(matchers::path("/user"))
        .and(matchers::header(
            "Authorization",
            format!("Bearer {}", token),
        ))
        .and(matchers::header("X-GitHub-Api-Version", "2022-11-28"))
        .and(matchers::header("Accept", "application/vnd.github+json"))
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .expect(1)
        .mount(&server.mock)
        .await;

    req.execute().await.unwrap();
}

#[tokio::test]
async fn test_get_user_and_repositories() {
    let server = spawn_mock_server().await;

    let c = load_test_collection();
    let r = load_test_request("GraphQL/GetUserAndRepos");

    let token = Uuid::new_v4();
    let variables = HashMap::from([
        ("host".to_string(), server.base_url),
        ("authToken".to_string(), token.to_string()),
    ]);

    let req = ApiClientRequest::new(c, r).with_override_variables(variables);

    let expected_body = json!({
      "query": "query ($login: String!) {\n  user(login: $login) {\n    login\n    name\n    company\n    location\n    followers(first: 10) {\n      nodes {\n        login\n        name\n        company\n        location\n      }\n    }\n    repositories(first: 100) {\n      totalCount\n      totalDiskUsage\n      pageInfo {\n        hasNextPage\n      }\n      nodes {\n        name\n        diskUsage\n        languages(first: 10) {\n          nodes {\n            name\n            color\n          }\n        }\n      }\n    }\n  }\n}\n",
      "variables": {
        "login": "mathieu-lemay"
      }
    });
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/graphql"))
        .and(matchers::body_json(expected_body))
        .and(matchers::header(
            "Authorization",
            format!("Bearer {}", token),
        ))
        .and(matchers::header("X-GitHub-Api-Version", "2022-11-28"))
        .and(matchers::header("Accept", "application/vnd.github+json"))
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .expect(1)
        .mount(&server.mock)
        .await;

    req.execute().await.unwrap();
}

pub struct TestServer {
    pub mock: MockServer,
    pub base_url: String,
}

pub async fn spawn_mock_server() -> TestServer {
    let mock = MockServer::start().await;
    let base_url = mock.uri();

    TestServer { mock, base_url }
}

fn load_test_collection() -> CollectionModel {
    let fp = Path::new("./tests/collections/GitHub/collection.yaml");

    let data = fs::read(fp).unwrap();

    serde_yaml::from_slice(&data).unwrap()
}

fn load_test_request(name: &str) -> RequestModel {
    let mut fp = PathBuf::from("./tests/collections/GitHub");
    fp.push(format!("{}.yaml", name));

    let data = fs::read(fp).unwrap();

    serde_yaml::from_slice(&data).unwrap()
}
