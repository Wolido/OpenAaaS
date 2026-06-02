"""OpenAaaS AutoGluon Executor 配置模块."""

import os
from pathlib import Path

# OpenAaaS 标准工作区（agent-core 会挂载此目录）
WORKSPACE = Path(os.environ.get("OPENAAAS_WORKSPACE", "/workspace"))

# 任务输入 / 结果输出 / 进度心跳
TASK_FILE = WORKSPACE / "task.json"
RESULT_FILE = WORKSPACE / "result.json"
PROGRESS_FILE = WORKSPACE / "progress.json"
HEARTBEAT_FILE = WORKSPACE / "heartbeat.json"

# 数据与模型持久化目录
DATA_DIR = WORKSPACE / "data"
MODELS_DIR = WORKSPACE / "models"

# SQLite 数据库（跨任务保留历史）
DB_PATH = WORKSPACE / "autogluon.db"

# 训练默认参数（可通过环境变量覆盖）
DEFAULT_TIME_LIMIT = int(os.environ.get("AUTOGLUON_TIME_LIMIT", "6000"))
DEFAULT_PRESETS = os.environ.get("AUTOGLUON_PRESETS", "medium")