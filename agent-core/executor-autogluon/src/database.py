"""SQLite 持久化模块."""

import json
import sqlite3
from datetime import datetime
from typing import Any, Dict, List, Optional

from .config import DB_PATH
from .utils import log


def init_db() -> None:
    """初始化 SQLite 数据库（若不存在则创建表）."""
    DB_PATH.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS tasks (
            task_id TEXT PRIMARY KEY,
            status TEXT NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            completed_at TIMESTAMP,
            result TEXT,
            error_message TEXT,
            model_path TEXT,
            label TEXT,
            feature_columns TEXT
        )
    """)
    conn.commit()
    conn.close()
    log(f"数据库已初始化: {DB_PATH}")


def save_task_record(
    task_id: str,
    status: str,
    result: Optional[Dict[str, Any]] = None,
    error: Optional[str] = None,
    model_path: Optional[str] = None,
    label: Optional[str] = None,
    features: Optional[List[str]] = None,
) -> None:
    """保存或更新任务记录到 SQLite."""
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    cursor.execute("""
        INSERT OR REPLACE INTO tasks 
        (task_id, status, completed_at, result, error_message,
         model_path, label, feature_columns)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    """, (
        task_id,
        status,
        datetime.now().isoformat(),
        json.dumps(result, ensure_ascii=False) if result else None,
        error,
        model_path,
        label,
        json.dumps(features) if features else None,
    ))
    conn.commit()
    conn.close()