//! 共享测试工具模块
//!
//! 提供测试所需的共享工具函数，如数据库连接池设置等。
//! 仅在测试模式下可用。

use crate::db::Database;
use sqlx::{Pool, Sqlite};

/// 设置测试数据库连接池
///
/// 创建一个内存 SQLite 数据库，并初始化所有表。
/// 适用于单元测试和集成测试。
///
/// # 示例
///
/// ```rust,ignore
/// #[sqlx::test]
/// async fn test_something() {
///     let pool = setup_test_db().await;
///     // 使用 pool 进行测试...
/// }
/// ```
pub async fn setup_test_db() -> Pool<Sqlite> {
    let db = Database::new("sqlite::memory:").await.unwrap();

    db.init_tables().await.unwrap();

    // 创建默认 admin 用户（旧迁移会插入，保持测试兼容性）
    sqlx::query("INSERT OR IGNORE INTO users (id, api_key, name, role) VALUES (?, ?, ?, ?)")
        .bind("admin")
        .bind("ak_admin_default_key")
        .bind("Administrator")
        .bind("admin")
        .execute(db.pool())
        .await
        .unwrap();

    db.pool().clone()
}
