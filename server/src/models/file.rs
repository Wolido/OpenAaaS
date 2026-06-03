//! 文件模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::error::AppError;

/// 文件创建者类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum FileCreatedBy {
    /// 客户端创建
    Client,
    /// Agent创建
    Agent,
}

impl Default for FileCreatedBy {
    fn default() -> Self {
        FileCreatedBy::Client
    }
}

impl ToString for FileCreatedBy {
    fn to_string(&self) -> String {
        match self {
            FileCreatedBy::Client => "client".to_string(),
            FileCreatedBy::Agent => "agent".to_string(),
        }
    }
}

/// 任务文件模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TaskFile {
    /// 文件ID
    pub id: String,
    /// 关联的任务ID
    pub task_id: String,
    /// 原始文件名
    pub filename: String,
    /// MIME类型
    pub mime_type: Option<String>,
    /// 文件大小（字节）
    pub size_bytes: i64,
    /// 存储路径（相对于存储根目录）
    pub storage_path: String,
    /// 创建者类型
    pub created_by: FileCreatedBy,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

impl TaskFile {
    /// 创建新文件记录
    pub fn new(
        task_id: impl Into<String>,
        filename: impl Into<String>,
        mime_type: Option<String>,
        size_bytes: i64,
        storage_path: impl Into<String>,
        created_by: FileCreatedBy,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            task_id: task_id.into(),
            filename: filename.into(),
            mime_type,
            size_bytes,
            storage_path: storage_path.into(),
            created_by,
            created_at: Utc::now(),
        }
    }

    /// 构建完整的存储路径（带路径遍历防护）
    pub fn full_storage_path(&self, base_path: &str) -> Result<std::path::PathBuf, AppError> {
        use std::path::Path;

        let base = Path::new(base_path)
            .canonicalize()
            .map_err(|_| AppError::Internal("无效的存储路径".to_string()))?;

        let full = base.join(&self.storage_path);

        // 确保最终路径在 base_path 下（防止路径遍历攻击）
        if !full.starts_with(&base) {
            return Err(AppError::Internal("非法的文件路径".to_string()));
        }

        Ok(full)
    }
}

/// 文件上传响应
#[derive(Debug, Serialize)]
pub struct UploadFileResponse {
    /// 文件ID
    pub file_id: String,
    /// 文件名
    pub filename: String,
    /// MIME类型
    pub mime_type: Option<String>,
    /// 文件大小（字节）
    pub size_bytes: i64,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

impl From<TaskFile> for UploadFileResponse {
    fn from(file: TaskFile) -> Self {
        Self {
            file_id: file.id,
            filename: file.filename,
            mime_type: file.mime_type,
            size_bytes: file.size_bytes,
            created_at: file.created_at,
        }
    }
}

/// 文件信息响应
#[derive(Debug, Serialize)]
pub struct FileInfoResponse {
    /// 文件ID
    pub id: String,
    /// 任务ID
    pub task_id: String,
    /// 文件名
    pub filename: String,
    /// MIME类型
    pub mime_type: Option<String>,
    /// 文件大小（字节）
    pub size_bytes: i64,
    /// 创建者类型
    pub created_by: FileCreatedBy,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

impl From<TaskFile> for FileInfoResponse {
    fn from(file: TaskFile) -> Self {
        Self {
            id: file.id,
            task_id: file.task_id,
            filename: file.filename,
            mime_type: file.mime_type,
            size_bytes: file.size_bytes,
            created_by: file.created_by,
            created_at: file.created_at,
        }
    }
}

/// 文件列表响应
#[derive(Debug, Serialize)]
pub struct FileListResponse {
    /// 文件列表
    pub files: Vec<FileInfoResponse>,
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // ==================== FileCreatedBy 枚举测试 ====================

    #[test]
    fn test_file_created_by_default() {
        let creator: FileCreatedBy = Default::default();
        assert_eq!(creator, FileCreatedBy::Client);
    }

    #[test]
    fn test_file_created_by_to_string() {
        assert_eq!(FileCreatedBy::Client.to_string(), "client");
        assert_eq!(FileCreatedBy::Agent.to_string(), "agent");
    }

    #[test]
    fn test_file_created_by_clone_and_eq() {
        let creator = FileCreatedBy::Agent;
        let cloned = creator.clone();
        assert_eq!(creator, cloned);
        assert_eq!(creator, FileCreatedBy::Agent);
        assert_ne!(creator, FileCreatedBy::Client);
    }

    #[test]
    fn test_file_created_by_serialization() {
        let creator = FileCreatedBy::Agent;
        let json = serde_json::to_string(&creator).unwrap();
        assert_eq!(json, "\"agent\"");

        let deserialized: FileCreatedBy = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, FileCreatedBy::Agent);
    }

    #[test]
    fn test_file_created_by_deserialization() {
        let json = "\"client\"";
        let creator: FileCreatedBy = serde_json::from_str(json).unwrap();
        assert_eq!(creator, FileCreatedBy::Client);

        let json = "\"agent\"";
        let creator: FileCreatedBy = serde_json::from_str(json).unwrap();
        assert_eq!(creator, FileCreatedBy::Agent);
    }

    // ==================== TaskFile 结构体测试 ====================

    #[test]
    fn test_task_file_new() {
        let file = TaskFile::new(
            "task_123",
            "test.txt",
            Some("text/plain".to_string()),
            1024,
            "task_123/test.txt",
            FileCreatedBy::Client,
        );

        assert!(!file.id.is_empty());
        assert_eq!(file.task_id, "task_123");
        assert_eq!(file.filename, "test.txt");
        assert_eq!(file.mime_type, Some("text/plain".to_string()));
        assert_eq!(file.size_bytes, 1024);
        assert_eq!(file.storage_path, "task_123/test.txt");
        assert_eq!(file.created_by, FileCreatedBy::Client);
    }

    #[test]
    fn test_task_file_new_without_mime_type() {
        let file = TaskFile::new(
            "task_456",
            "data.bin",
            None,
            2048,
            "task_456/data.bin",
            FileCreatedBy::Agent,
        );

        assert_eq!(file.task_id, "task_456");
        assert_eq!(file.filename, "data.bin");
        assert!(file.mime_type.is_none());
        assert_eq!(file.size_bytes, 2048);
        assert_eq!(file.created_by, FileCreatedBy::Agent);
    }

    #[test]
    fn test_task_file_new_with_string_refs() {
        let task_id = String::from("task_789");
        let filename = String::from("document.pdf");
        let path = String::from("task_789/document.pdf");

        let file = TaskFile::new(
            &task_id,
            &filename,
            Some("application/pdf".to_string()),
            1024000,
            &path,
            FileCreatedBy::Client,
        );

        assert_eq!(file.task_id, "task_789");
        assert_eq!(file.filename, "document.pdf");
        assert_eq!(file.storage_path, "task_789/document.pdf");
    }

    #[test]
    fn test_task_file_clone() {
        let file = TaskFile::new(
            "task_123",
            "test.txt",
            Some("text/plain".to_string()),
            1024,
            "task_123/test.txt",
            FileCreatedBy::Client,
        );

        let cloned = file.clone();
        assert_eq!(cloned.id, file.id);
        assert_eq!(cloned.task_id, file.task_id);
        assert_eq!(cloned.filename, file.filename);
        assert_eq!(cloned.mime_type, file.mime_type);
        assert_eq!(cloned.size_bytes, file.size_bytes);
        assert_eq!(cloned.storage_path, file.storage_path);
        assert_eq!(cloned.created_by, file.created_by);
        assert_eq!(cloned.created_at, file.created_at);
    }

    #[test]
    fn test_task_file_serialization() {
        let file = TaskFile::new(
            "task_123",
            "test.txt",
            Some("text/plain".to_string()),
            1024,
            "task_123/test.txt",
            FileCreatedBy::Agent,
        );

        let json = serde_json::to_string(&file).unwrap();

        assert!(json.contains("task_123"));
        assert!(json.contains("test.txt"));
        assert!(json.contains("text/plain"));
        assert!(json.contains("1024"));
        assert!(json.contains("agent"));

        let deserialized: TaskFile = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, file.id);
        assert_eq!(deserialized.task_id, file.task_id);
        assert_eq!(deserialized.filename, file.filename);
        assert_eq!(deserialized.size_bytes, file.size_bytes);
        assert_eq!(deserialized.created_by, file.created_by);
    }

    // ==================== TaskFile::full_storage_path 测试 ====================

    #[test]
    fn test_full_storage_path_success() {
        // 创建临时目录
        let temp_dir = std::env::temp_dir().join(format!("test_storage_{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        // 创建子目录模拟任务文件夹
        let task_dir = temp_dir.join("task_123");
        fs::create_dir_all(&task_dir).unwrap();

        // 创建一个测试文件
        let test_file = task_dir.join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let file = TaskFile::new(
            "task_123",
            "test.txt",
            None,
            100,
            "task_123/test.txt",
            FileCreatedBy::Client,
        );

        let result = file.full_storage_path(temp_dir.to_str().unwrap());
        assert!(result.is_ok());

        let full_path = result.unwrap();
        assert!(full_path.exists());
        assert!(full_path.to_string_lossy().contains("task_123"));
        assert!(full_path.to_string_lossy().contains("test.txt"));

        // 清理
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_full_storage_path_invalid_base_path() {
        let file = TaskFile::new(
            "task_123",
            "test.txt",
            None,
            100,
            "task_123/test.txt",
            FileCreatedBy::Client,
        );

        let result = file.full_storage_path("/nonexistent/path/that/does/not/exist");
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Internal(msg) => assert!(msg.contains("无效") || msg.contains("存储路径")),
            _ => panic!("应该是 Internal 错误"),
        }
    }

    #[test]
    fn test_full_storage_path_with_absolute_storage_path() {
        let temp_dir = std::env::temp_dir().join(format!("test_storage_{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        // 使用绝对路径作为 storage_path
        let file = TaskFile::new(
            "task_123",
            "test.txt",
            None,
            100,
            "/etc/passwd",
            FileCreatedBy::Client,
        );

        let result = file.full_storage_path(temp_dir.to_str().unwrap());
        // 这应该返回错误，因为路径不在 base 目录下
        assert!(result.is_err());

        // 清理
        let _ = fs::remove_dir_all(&temp_dir);
    }

    // ==================== UploadFileResponse 测试 ====================

    #[test]
    fn test_upload_file_response_from_task_file() {
        let file = TaskFile::new(
            "task_123",
            "document.pdf",
            Some("application/pdf".to_string()),
            1024000,
            "task_123/document.pdf",
            FileCreatedBy::Client,
        );

        let response: UploadFileResponse = file.clone().into();

        assert_eq!(response.file_id, file.id);
        assert_eq!(response.filename, file.filename);
        assert_eq!(response.mime_type, file.mime_type);
        assert_eq!(response.size_bytes, file.size_bytes);
        assert_eq!(response.created_at, file.created_at);
    }

    #[test]
    fn test_upload_file_response_serialization() {
        let file = TaskFile::new(
            "task_123",
            "test.txt",
            Some("text/plain".to_string()),
            1024,
            "task_123/test.txt",
            FileCreatedBy::Agent,
        );

        let response: UploadFileResponse = file.into();
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("test.txt"));
        assert!(json.contains("text/plain"));
        assert!(json.contains("1024"));
    }

    // ==================== FileInfoResponse 测试 ====================

    #[test]
    fn test_file_info_response_from_task_file() {
        let file = TaskFile::new(
            "task_123",
            "image.png",
            Some("image/png".to_string()),
            512000,
            "task_123/image.png",
            FileCreatedBy::Agent,
        );

        let response: FileInfoResponse = file.clone().into();

        assert_eq!(response.id, file.id);
        assert_eq!(response.task_id, file.task_id);
        assert_eq!(response.filename, file.filename);
        assert_eq!(response.mime_type, file.mime_type);
        assert_eq!(response.size_bytes, file.size_bytes);
        assert_eq!(response.created_by, file.created_by);
        assert_eq!(response.created_at, file.created_at);
    }

    #[test]
    fn test_file_info_response_serialization() {
        let file = TaskFile::new(
            "task_456",
            "data.json",
            Some("application/json".to_string()),
            2048,
            "task_456/data.json",
            FileCreatedBy::Client,
        );

        let response: FileInfoResponse = file.into();
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("task_456"));
        assert!(json.contains("data.json"));
        assert!(json.contains("application/json"));
        assert!(json.contains("2048"));
        assert!(json.contains("client"));
    }

    // ==================== FileListResponse 测试 ====================

    #[test]
    fn test_file_list_response_empty() {
        let response = FileListResponse { files: vec![] };

        let json = serde_json::to_string(&response).unwrap();
        assert_eq!(json, "{\"files\":[]}");
    }

    #[test]
    fn test_file_list_response_with_files() {
        let file1 = TaskFile::new(
            "task_123",
            "file1.txt",
            Some("text/plain".to_string()),
            100,
            "task_123/file1.txt",
            FileCreatedBy::Client,
        );

        let file2 = TaskFile::new(
            "task_123",
            "file2.pdf",
            Some("application/pdf".to_string()),
            1000,
            "task_123/file2.pdf",
            FileCreatedBy::Agent,
        );

        let response = FileListResponse {
            files: vec![file1.into(), file2.into()],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("file1.txt"));
        assert!(json.contains("file2.pdf"));
        assert!(json.contains("client"));
        assert!(json.contains("agent"));
    }

    // ==================== 边界情况测试 ====================

    #[test]
    fn test_task_file_zero_size() {
        let file = TaskFile::new(
            "task_123",
            "empty.txt",
            Some("text/plain".to_string()),
            0,
            "task_123/empty.txt",
            FileCreatedBy::Client,
        );

        assert_eq!(file.size_bytes, 0);
    }

    #[test]
    fn test_task_file_large_size() {
        let large_size = i64::MAX;
        let file = TaskFile::new(
            "task_123",
            "large.bin",
            None,
            large_size,
            "task_123/large.bin",
            FileCreatedBy::Agent,
        );

        assert_eq!(file.size_bytes, large_size);
    }

    #[test]
    fn test_task_file_special_characters_in_filename() {
        let file = TaskFile::new(
            "task_123",
            "file with spaces & symbols!.txt",
            Some("text/plain".to_string()),
            100,
            "task_123/file with spaces & symbols!.txt",
            FileCreatedBy::Client,
        );

        assert_eq!(file.filename, "file with spaces & symbols!.txt");
    }

    #[test]
    fn test_task_file_unicode_filename() {
        let file = TaskFile::new(
            "task_123",
            "中文文件.pdf",
            Some("application/pdf".to_string()),
            1024,
            "task_123/中文文件.pdf",
            FileCreatedBy::Client,
        );

        assert_eq!(file.filename, "中文文件.pdf");
    }

    // ==================== 数据库集成测试 ====================

    use crate::test_utils::setup_test_db;

    #[sqlx::test]
    async fn test_task_file_insert_and_fetch() {
        let pool = setup_test_db().await;

        // 创建外键引用的服务、用户和任务
        sqlx::query("INSERT INTO services (id, name, description, usage) VALUES (?, ?, ?, ?)")
            .bind("service_123")
            .bind("Test Service")
            .bind("")
            .bind("")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO users (id, api_key, name, role) VALUES (?, ?, ?, ?)")
            .bind("user_123")
            .bind("ak_test_key")
            .bind("Test User")
            .bind("client")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO tasks (id, user_id, service_id, session_id) VALUES (?, ?, ?, ?)")
            .bind("task_123")
            .bind("user_123")
            .bind("service_123")
            .bind("session_123")
            .execute(&pool)
            .await
            .unwrap();

        let file = TaskFile::new(
            "task_123",
            "test.txt",
            Some("text/plain".to_string()),
            1024,
            "task_123/test.txt",
            FileCreatedBy::Client,
        );

        // 插入文件记录
        sqlx::query(
            r#"
            INSERT INTO task_files (id, task_id, filename, mime_type, size_bytes, storage_path, created_by, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&file.id)
        .bind(&file.task_id)
        .bind(&file.filename)
        .bind(&file.mime_type)
        .bind(file.size_bytes)
        .bind(&file.storage_path)
        .bind("client")
        .bind(file.created_at)
        .execute(&pool)
        .await
        .unwrap();

        // 查询文件
        let fetched: TaskFile =
            sqlx::query_as::<_, TaskFile>("SELECT * FROM task_files WHERE id = ?")
                .bind(&file.id)
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(fetched.id, file.id);
        assert_eq!(fetched.task_id, file.task_id);
        assert_eq!(fetched.filename, file.filename);
        assert_eq!(fetched.mime_type, file.mime_type);
        assert_eq!(fetched.size_bytes, file.size_bytes);
        assert_eq!(fetched.storage_path, file.storage_path);
        assert_eq!(fetched.created_by, FileCreatedBy::Client);
    }

    #[sqlx::test]
    async fn test_task_file_fetch_by_task_id() {
        let pool = setup_test_db().await;

        // 创建外键引用的服务、用户
        sqlx::query("INSERT INTO services (id, name, description, usage) VALUES (?, ?, ?, ?)")
            .bind("service_456")
            .bind("Test Service")
            .bind("")
            .bind("")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO users (id, api_key, name, role) VALUES (?, ?, ?, ?)")
            .bind("user_456")
            .bind("ak_test_key")
            .bind("Test User")
            .bind("client")
            .execute(&pool)
            .await
            .unwrap();

        // 创建任务（包括 file3 引用的 task_789）
        for tid in ["task_456", "task_789"] {
            sqlx::query(
                "INSERT INTO tasks (id, user_id, service_id, session_id) VALUES (?, ?, ?, ?)",
            )
            .bind(tid)
            .bind("user_456")
            .bind("service_456")
            .bind(format!("session_{}", tid))
            .execute(&pool)
            .await
            .unwrap();
        }

        // 插入多个文件到同一个任务
        let file1 = TaskFile::new(
            "task_456",
            "file1.txt",
            Some("text/plain".to_string()),
            100,
            "task_456/file1.txt",
            FileCreatedBy::Client,
        );

        let file2 = TaskFile::new(
            "task_456",
            "file2.txt",
            Some("text/plain".to_string()),
            200,
            "task_456/file2.txt",
            FileCreatedBy::Agent,
        );

        let file3 = TaskFile::new(
            "task_789",
            "file3.txt",
            Some("text/plain".to_string()),
            300,
            "task_789/file3.txt",
            FileCreatedBy::Client,
        );

        for file in [&file1, &file2, &file3] {
            sqlx::query(
                r#"
                INSERT INTO task_files (id, task_id, filename, mime_type, size_bytes, storage_path, created_by, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(&file.id)
            .bind(&file.task_id)
            .bind(&file.filename)
            .bind(&file.mime_type)
            .bind(file.size_bytes)
            .bind(&file.storage_path)
            .bind(if file.created_by == FileCreatedBy::Client { "client" } else { "agent" })
            .bind(file.created_at)
            .execute(&pool)
            .await
            .unwrap();
        }

        // 按 task_id 查询
        let files: Vec<TaskFile> =
            sqlx::query_as::<_, TaskFile>("SELECT * FROM task_files WHERE task_id = ?")
                .bind("task_456")
                .fetch_all(&pool)
                .await
                .unwrap();

        assert_eq!(files.len(), 2);
    }

    #[sqlx::test]
    async fn test_task_file_delete() {
        let pool = setup_test_db().await;

        // 创建外键引用的服务、用户和任务
        sqlx::query("INSERT INTO services (id, name, description, usage) VALUES (?, ?, ?, ?)")
            .bind("service_123")
            .bind("Test Service")
            .bind("")
            .bind("")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO users (id, api_key, name, role) VALUES (?, ?, ?, ?)")
            .bind("user_123")
            .bind("ak_test_key")
            .bind("Test User")
            .bind("client")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO tasks (id, user_id, service_id, session_id) VALUES (?, ?, ?, ?)")
            .bind("task_123")
            .bind("user_123")
            .bind("service_123")
            .bind("session_123")
            .execute(&pool)
            .await
            .unwrap();

        let file = TaskFile::new(
            "task_123",
            "delete_me.txt",
            None,
            100,
            "task_123/delete_me.txt",
            FileCreatedBy::Client,
        );

        // 插入
        sqlx::query(
            r#"
            INSERT INTO task_files (id, task_id, filename, mime_type, size_bytes, storage_path, created_by, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&file.id)
        .bind(&file.task_id)
        .bind(&file.filename)
        .bind(&file.mime_type)
        .bind(file.size_bytes)
        .bind(&file.storage_path)
        .bind("client")
        .bind(file.created_at)
        .execute(&pool)
        .await
        .unwrap();

        // 删除
        let result = sqlx::query("DELETE FROM task_files WHERE id = ?")
            .bind(&file.id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(result.rows_affected(), 1);

        // 确认删除
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM task_files WHERE id = ?")
            .bind(&file.id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(count.0, 0);
    }

    #[sqlx::test]
    async fn test_task_file_count_by_creator() {
        let pool = setup_test_db().await;

        // 创建外键引用的服务、用户
        sqlx::query("INSERT INTO services (id, name, description, usage) VALUES (?, ?, ?, ?)")
            .bind("service_1")
            .bind("Test Service")
            .bind("")
            .bind("")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO users (id, api_key, name, role) VALUES (?, ?, ?, ?)")
            .bind("user_1")
            .bind("ak_test_key")
            .bind("Test User")
            .bind("client")
            .execute(&pool)
            .await
            .unwrap();

        // 创建任务
        for tid in ["task_1", "task_2", "task_3"] {
            sqlx::query(
                "INSERT INTO tasks (id, user_id, service_id, session_id) VALUES (?, ?, ?, ?)",
            )
            .bind(tid)
            .bind("user_1")
            .bind("service_1")
            .bind(format!("session_{}", tid))
            .execute(&pool)
            .await
            .unwrap();
        }

        // 插入多个文件
        let files = vec![
            TaskFile::new(
                "task_1",
                "f1.txt",
                None,
                100,
                "task_1/f1.txt",
                FileCreatedBy::Client,
            ),
            TaskFile::new(
                "task_2",
                "f2.txt",
                None,
                100,
                "task_2/f2.txt",
                FileCreatedBy::Client,
            ),
            TaskFile::new(
                "task_3",
                "f3.txt",
                None,
                100,
                "task_3/f3.txt",
                FileCreatedBy::Agent,
            ),
        ];

        for file in &files {
            sqlx::query(
                r#"
                INSERT INTO task_files (id, task_id, filename, mime_type, size_bytes, storage_path, created_by, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(&file.id)
            .bind(&file.task_id)
            .bind(&file.filename)
            .bind(&file.mime_type)
            .bind(file.size_bytes)
            .bind(&file.storage_path)
            .bind(if file.created_by == FileCreatedBy::Client { "client" } else { "agent" })
            .bind(file.created_at)
            .execute(&pool)
            .await
            .unwrap();
        }

        // 统计 client 创建的文件
        let client_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM task_files WHERE created_by = ?")
                .bind("client")
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(client_count.0, 2);

        // 统计 agent 创建的文件
        let agent_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM task_files WHERE created_by = ?")
                .bind("agent")
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(agent_count.0, 1);
    }
}
