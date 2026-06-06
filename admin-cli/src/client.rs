use std::time::Duration;

use reqwest::{Client, Response};

use crate::config::Config;
use crate::error::{AppError, Result};
use crate::models::*;

pub struct ApiClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ApiClient {
    pub fn new(config: &Config) -> Result<Self> {
        let base_url = config.require_server_url()?;
        let api_key = config.require_api_key()?;
        Ok(Self {
            client: Client::builder().timeout(Duration::from_secs(30)).build()?,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        })
    }

    pub fn new_without_auth(config: &Config) -> Result<Self> {
        let base_url = config.require_server_url()?;
        Ok(Self {
            client: Client::builder().timeout(Duration::from_secs(30)).build()?,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: String::new(),
        })
    }

    fn auth_header(&self) -> Option<(&str, String)> {
        if self.api_key.is_empty() {
            None
        } else {
            Some(("Authorization", format!("Bearer {}", self.api_key)))
        }
    }

    async fn handle_response<T: serde::de::DeserializeOwned>(res: Response) -> Result<T> {
        let status = res.status();
        if status.is_success() {
            let body = res.json::<T>().await?;
            Ok(body)
        } else {
            let body_text = res.text().await.unwrap_or_default();
            Err(AppError::extract_api_error(status.as_u16(), &body_text))
        }
    }

    async fn handle_empty_response(res: Response) -> Result<()> {
        let status = res.status();
        if status.is_success() {
            Ok(())
        } else {
            let body_text = res.text().await.unwrap_or_default();
            Err(AppError::extract_api_error(status.as_u16(), &body_text))
        }
    }

    // ========================================================================
    // Health (no auth)
    // ========================================================================

    pub async fn health_check(&self) -> Result<HealthResponse> {
        let url = format!("{}/health", self.base_url);
        let res = self.client.get(&url).send().await?;
        Self::handle_response(res).await
    }

    // ========================================================================
    // Services
    // ========================================================================

    pub async fn list_services(&self) -> Result<Vec<ServiceListItem>> {
        let url = format!("{}/api/v1/services", self.base_url);
        let mut req = self.client.get(&url);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let res = req.send().await?;
        Self::handle_response(res).await
    }

    pub async fn get_service(&self, id: &str) -> Result<ServiceResponse> {
        let url = format!("{}/api/v1/services/{}", self.base_url, id);
        let mut req = self.client.get(&url);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let res = req.send().await?;
        Self::handle_response(res).await
    }

    pub async fn get_service_usage(&self, id: &str) -> Result<ServiceUsageResponse> {
        let url = format!("{}/api/v1/client/services/{}/usage", self.base_url, id);
        let mut req = self.client.get(&url);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let res = req.send().await?;
        Self::handle_response(res).await
    }

    pub async fn create_service(
        &self,
        req_body: &CreateServiceRequest,
    ) -> Result<CreateServiceResponse> {
        let url = format!("{}/api/v1/services", self.base_url);
        let mut req = self.client.post(&url).json(req_body);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let res = req.send().await?;
        Self::handle_response(res).await
    }

    pub async fn update_service(
        &self,
        id: &str,
        req_body: &UpdateServiceRequest,
    ) -> Result<ServiceResponse> {
        let url = format!("{}/api/v1/services/{}", self.base_url, id);
        let mut req = self.client.put(&url).json(req_body);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let res = req.send().await?;
        Self::handle_response(res).await
    }

    pub async fn delete_service(&self, id: &str, force: bool) -> Result<DeleteServiceResponse> {
        let url = format!("{}/api/v1/services/{}", self.base_url, id);
        let mut req = self.client.delete(&url);
        if force {
            req = req.query(&[("force", "true")]);
        }
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let res = req.send().await?;
        Self::handle_response(res).await
    }

    // ========================================================================
    // Users
    // ========================================================================

    pub async fn list_users(&self) -> Result<Vec<UserResponse>> {
        let url = format!("{}/api/v1/admin/users", self.base_url);
        let mut req = self.client.get(&url);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let res = req.send().await?;
        Self::handle_response(res).await
    }

    pub async fn delete_user(&self, id: &str) -> Result<()> {
        let url = format!("{}/api/v1/admin/users/{}", self.base_url, id);
        let mut req = self.client.delete(&url);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let res = req.send().await?;
        Self::handle_empty_response(res).await
    }

    // ========================================================================
    // Permissions
    // ========================================================================

    pub async fn list_service_users(&self, service_id: &str) -> Result<ServiceUsersList> {
        let url = format!(
            "{}/api/v1/admin/services/{}/users",
            self.base_url, service_id
        );
        let mut req = self.client.get(&url);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let res = req.send().await?;
        Self::handle_response(res).await
    }

    pub async fn list_user_permissions(
        &self,
        user_id: &str,
    ) -> Result<Vec<UserPermissionResponse>> {
        let url = format!(
            "{}/api/v1/admin/users/{}/permissions",
            self.base_url, user_id
        );
        let mut req = self.client.get(&url);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let res = req.send().await?;
        Self::handle_response(res).await
    }

    /// Note: This endpoint uses `/api/v1/client/...` prefix by design,
    /// matching the Python SDK `OaaSClient.grant_service_permission` behavior.
    pub async fn grant_permission(&self, service_id: &str, user_id: &str) -> Result<()> {
        let url = format!(
            "{}/api/v1/client/services/{}/grant",
            self.base_url, service_id
        );
        let body = GrantPermissionRequest {
            user_id: user_id.to_string(),
        };
        let mut req = self.client.post(&url).json(&body);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let res = req.send().await?;
        Self::handle_empty_response(res).await
    }

    pub async fn revoke_permission(&self, service_id: &str, user_id: &str) -> Result<()> {
        let url = format!(
            "{}/api/v1/admin/services/{}/users/{}",
            self.base_url, service_id, user_id
        );
        let mut req = self.client.delete(&url);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let res = req.send().await?;
        Self::handle_empty_response(res).await
    }

    pub async fn check_admin(&self) -> Result<()> {
        let url = format!("{}/api/v1/admin/users", self.base_url);
        let mut req = self.client.get(&url).query(&[("limit", "1")]);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let res = req.send().await?;
        let status = res.status();
        if status.is_success() {
            Ok(())
        } else {
            let body_text = res.text().await.unwrap_or_default();
            Err(AppError::extract_api_error(status.as_u16(), &body_text))
        }
    }

    // ========================================================================
    // Tasks / Stats
    // ========================================================================

    pub async fn list_admin_tasks(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AdminTaskResponse>> {
        let url = format!("{}/api/v1/admin/tasks", self.base_url);
        let mut req = self
            .client
            .get(&url)
            .query(&[("limit", limit.to_string()), ("offset", offset.to_string())]);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let res = req.send().await?;
        Self::handle_response(res).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json, header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_config(url: &str) -> Config {
        Config {
            server_url: Some(url.to_string()),
            api_key: Some("test_key".to_string()),
        }
    }

    #[tokio::test]
    async fn test_health_check() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": "ok",
                "version": "0.1.0",
                "timestamp": "2024-01-01T00:00:00Z"
            })))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new_without_auth(&config).unwrap();
        let health = client.health_check().await.unwrap();
        assert_eq!(health.status, "ok");
    }

    #[tokio::test]
    async fn test_list_services() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/services"))
            .and(header("Authorization", "Bearer test_key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "id": "svc-1",
                    "name": "Test Service",
                    "description": "Desc",
                    "agent_status": "online",
                    "registration_status": "active",
                    "access_type": "public",
                    "has_permission": true
                }
            ])))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        let services = client.list_services().await.unwrap();
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].id, "svc-1");
    }

    #[tokio::test]
    async fn test_api_error_extraction() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/services"))
            .and(header("Authorization", "Bearer test_key"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "message": "bad request"
            })))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        let err = client.list_services().await.unwrap_err();
        match err {
            AppError::ApiError { status, message } => {
                assert_eq!(status, 400);
                assert_eq!(message, "bad request");
            }
            _ => panic!("Expected ApiError, got {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_create_service() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/services"))
            .and(header("Authorization", "Bearer test_key"))
            .and(body_json(serde_json::json!({
                "name": "New Service",
                "description": "A new service",
                "usage": "Usage info",
                "is_public": true
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "svc-new",
                "name": "New Service",
                "description": "A new service",
                "usage": "Usage info",
                "registration_status": "pending",
                "registration_token": "rt_abc123",
                "created_at": "2024-01-01T00:00:00Z"
            })))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        let req = CreateServiceRequest {
            name: "New Service".to_string(),
            description: "A new service".to_string(),
            usage: "Usage info".to_string(),
            is_public: true,
        };
        let resp = client.create_service(&req).await.unwrap();
        assert_eq!(resp.id, "svc-new");
        assert_eq!(resp.registration_token, "rt_abc123");
    }

    #[tokio::test]
    async fn test_delete_user_error() {
        let mock_server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v1/admin/users/u1"))
            .and(header("Authorization", "Bearer test_key"))
            .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
                "message": "不能删除当前登录的管理员账户"
            })))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        let err = client.delete_user("u1").await.unwrap_err();
        match err {
            AppError::ApiError { status, message } => {
                assert_eq!(status, 403);
                assert!(message.contains("不能删除"));
            }
            _ => panic!("Expected ApiError"),
        }
    }

    #[tokio::test]
    async fn test_get_service() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/services/svc-1"))
            .and(header("Authorization", "Bearer test_key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "svc-1",
                "name": "Test Service",
                "description": "Desc",
                "usage": "Usage info",
                "agent_status": "online",
                "registration_status": "active",
                "agent_capacity": 10,
                "agent_current_load": 2,
                "is_public": true,
                "created_at": "2024-01-01T00:00:00Z"
            })))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        let svc = client.get_service("svc-1").await.unwrap();
        assert_eq!(svc.id, "svc-1");
        assert_eq!(svc.name, "Test Service");
        assert_eq!(svc.agent_capacity, 10);
        assert_eq!(svc.agent_current_load, 2);
        assert!(svc.is_public);
    }

    #[tokio::test]
    async fn test_update_service() {
        let mock_server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/api/v1/services/svc-1"))
            .and(header("Authorization", "Bearer test_key"))
            .and(body_json(serde_json::json!({
                "name": "Updated Service",
                "description": "Updated desc"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "svc-1",
                "name": "Updated Service",
                "description": "Updated desc",
                "usage": "Usage info",
                "agent_status": "online",
                "registration_status": "active",
                "agent_capacity": 10,
                "agent_current_load": 2,
                "is_public": true,
                "created_at": "2024-01-01T00:00:00Z"
            })))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        let req = UpdateServiceRequest {
            name: Some("Updated Service".to_string()),
            description: Some("Updated desc".to_string()),
            usage: None,
            is_public: None,
        };
        let svc = client.update_service("svc-1", &req).await.unwrap();
        assert_eq!(svc.name, "Updated Service");
        assert_eq!(svc.description, "Updated desc");
    }

    #[tokio::test]
    async fn test_delete_service() {
        let mock_server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v1/services/svc-1"))
            .and(header("Authorization", "Bearer test_key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "deleted": true,
                "tasks_cancelled": 0,
                "tasks_retained": 0
            })))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        let resp = client.delete_service("svc-1", false).await.unwrap();
        assert!(resp.deleted);
        assert_eq!(resp.tasks_cancelled, 0);
        assert_eq!(resp.tasks_retained, 0);
    }

    #[tokio::test]
    async fn test_delete_service_force() {
        let mock_server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v1/services/svc-1"))
            .and(query_param("force", "true"))
            .and(header("Authorization", "Bearer test_key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "deleted": true,
                "tasks_cancelled": 3,
                "tasks_retained": 1
            })))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        let resp = client.delete_service("svc-1", true).await.unwrap();
        assert!(resp.deleted);
        assert_eq!(resp.tasks_cancelled, 3);
        assert_eq!(resp.tasks_retained, 1);
    }

    #[tokio::test]
    async fn test_list_users() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/admin/users"))
            .and(header("Authorization", "Bearer test_key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "id": "u1",
                    "name": "Alice",
                    "api_key": "ak_abc",
                    "role": "admin",
                    "created_at": "2024-01-01T00:00:00Z"
                }
            ])))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        let users = client.list_users().await.unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].id, "u1");
        assert_eq!(users[0].name, "Alice");
        assert_eq!(users[0].role, "admin");
    }

    #[tokio::test]
    async fn test_delete_user() {
        let mock_server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v1/admin/users/u1"))
            .and(header("Authorization", "Bearer test_key"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        client.delete_user("u1").await.unwrap();
    }

    #[tokio::test]
    async fn test_check_admin() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/admin/users"))
            .and(query_param("limit", "1"))
            .and(header("Authorization", "Bearer test_key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "id": "u1" }
            ])))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        client.check_admin().await.unwrap();
    }

    #[tokio::test]
    async fn test_check_admin_failure() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/admin/users"))
            .and(query_param("limit", "1"))
            .and(header("Authorization", "Bearer test_key"))
            .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
                "message": "Forbidden"
            })))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        let err = client.check_admin().await.unwrap_err();
        match err {
            AppError::ApiError { status, message } => {
                assert_eq!(status, 403);
                assert_eq!(message, "Forbidden");
            }
            _ => panic!("Expected ApiError"),
        }
    }

    #[tokio::test]
    async fn test_grant_permission() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/client/services/svc-1/grant"))
            .and(header("Authorization", "Bearer test_key"))
            .and(body_json(serde_json::json!({
                "user_id": "u1"
            })))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        client.grant_permission("svc-1", "u1").await.unwrap();
    }

    #[tokio::test]
    async fn test_revoke_permission() {
        let mock_server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v1/admin/services/svc-1/users/u1"))
            .and(header("Authorization", "Bearer test_key"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        client.revoke_permission("svc-1", "u1").await.unwrap();
    }

    #[tokio::test]
    async fn test_list_user_permissions() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/admin/users/u1/permissions"))
            .and(header("Authorization", "Bearer test_key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "service_id": "svc-1",
                    "service_name": "Test Service",
                    "granted_at": "2024-01-01T00:00:00Z"
                }
            ])))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        let perms = client.list_user_permissions("u1").await.unwrap();
        assert_eq!(perms.len(), 1);
        assert_eq!(perms[0].service_id, "svc-1");
        assert_eq!(perms[0].service_name, "Test Service");
    }

    #[tokio::test]
    async fn test_list_service_users() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/admin/services/svc-1/users"))
            .and(header("Authorization", "Bearer test_key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "is_public": false,
                "users": [
                    {
                        "user_id": "u1",
                        "user_name": "Alice",
                        "role": "user",
                        "granted_at": "2024-01-01T00:00:00Z"
                    }
                ]
            })))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        let result = client.list_service_users("svc-1").await.unwrap();
        assert!(!result.is_public);
        assert_eq!(result.users.len(), 1);
        assert_eq!(result.users[0].user_id, "u1");
        assert_eq!(result.users[0].user_name, Some("Alice".to_string()));
        assert_eq!(result.users[0].role, "user");
    }

    #[tokio::test]
    async fn test_list_service_users_empty() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/admin/services/svc-1/users"))
            .and(header("Authorization", "Bearer test_key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "is_public": false,
                "users": []
            })))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        let result = client.list_service_users("svc-1").await.unwrap();
        assert!(!result.is_public);
        assert!(result.users.is_empty());
    }

    #[tokio::test]
    async fn test_list_service_users_not_found() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/admin/services/svc-99/users"))
            .and(header("Authorization", "Bearer test_key"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "message": "Service not found"
            })))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        let err = client.list_service_users("svc-99").await.unwrap_err();
        match err {
            AppError::ApiError { status, message } => {
                assert_eq!(status, 404);
                assert_eq!(message, "Service not found");
            }
            _ => panic!("Expected ApiError, got {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_list_admin_tasks() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/admin/tasks"))
            .and(query_param("limit", "10"))
            .and(query_param("offset", "0"))
            .and(header("Authorization", "Bearer test_key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "id": "task-1",
                    "user_id": "u1",
                    "user_name": "Alice",
                    "service_id": "svc-1",
                    "status": "running",
                    "input": null,
                    "output": null,
                    "error_message": null,
                    "session_id": "sess-1",
                    "retry_count": 0,
                    "created_at": "2024-01-01T00:00:00Z",
                    "assigned_at": null,
                    "started_at": null,
                    "completed_at": null
                }
            ])))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        let tasks = client.list_admin_tasks(10, 0).await.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "task-1");
        assert_eq!(tasks[0].status, crate::models::TaskStatus::Running);
    }

    #[tokio::test]
    async fn test_list_services_empty() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/services"))
            .and(header("Authorization", "Bearer test_key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        let services = client.list_services().await.unwrap();
        assert!(services.is_empty());
    }

    #[tokio::test]
    async fn test_list_users_empty() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/admin/users"))
            .and(header("Authorization", "Bearer test_key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        let users = client.list_users().await.unwrap();
        assert!(users.is_empty());
    }

    #[tokio::test]
    async fn test_server_error_500() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/services"))
            .and(header("Authorization", "Bearer test_key"))
            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
                "message": "internal server error"
            })))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        let err = client.list_services().await.unwrap_err();
        match err {
            AppError::ApiError { status, .. } => {
                assert_eq!(status, 500);
            }
            _ => panic!("Expected ApiError, got {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_malformed_json() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/services"))
            .and(header("Authorization", "Bearer test_key"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        let err = client.list_services().await.unwrap_err();
        match err {
            AppError::Parse(_) | AppError::Network(_) => {}
            _ => panic!("Expected Parse or Network error, got {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_get_service_usage() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/client/services/svc-1/usage"))
            .and(header("Authorization", "Bearer test_key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "svc-1",
                "name": "Test Service",
                "usage": "Usage info"
            })))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        let usage = client.get_service_usage("svc-1").await.unwrap();
        assert_eq!(usage.id, "svc-1");
        assert_eq!(usage.name, "Test Service");
        assert_eq!(usage.usage, "Usage info");
    }

    #[tokio::test]
    async fn test_list_admin_tasks_empty() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/admin/tasks"))
            .and(query_param("limit", "10"))
            .and(query_param("offset", "0"))
            .and(header("Authorization", "Bearer test_key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        let tasks = client.list_admin_tasks(10, 0).await.unwrap();
        assert!(tasks.is_empty());
    }

    #[tokio::test]
    async fn test_delete_user_not_found() {
        let mock_server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v1/admin/users/u99"))
            .and(header("Authorization", "Bearer test_key"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "message": "User not found"
            })))
            .mount(&mock_server)
            .await;

        let config = test_config(&mock_server.uri());
        let client = ApiClient::new(&config).unwrap();
        let err = client.delete_user("u99").await.unwrap_err();
        match err {
            AppError::ApiError { status, message } => {
                assert_eq!(status, 404);
                assert_eq!(message, "User not found");
            }
            _ => panic!("Expected ApiError, got {:?}", err),
        }
    }
}
