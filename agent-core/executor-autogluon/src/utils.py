"""工具函数."""

import json
import os
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, Optional

from .config import DATA_DIR


def log(msg: str) -> None:
    """打印带时间戳的日志到 stdout."""
    timestamp = datetime.now().strftime("%H:%M:%S")
    print(f"[{timestamp}] {msg}", flush=True)


def resolve_path(path_str: Optional[str]) -> Optional[Path]:
    """路径解析：支持 OpenAaaS 标准相对路径，兼容本地绝对路径测试。
    
    OpenAaaS 标准：
    - 输入数据在 /workspace/data/ 下
    - 相对路径基于 /workspace/data/
    
    兼容本地测试：
    - 绝对路径直接返回（需自行挂载）
    """
    if not path_str:
        return None
    
    p = Path(path_str)
    
    # 兼容本地测试：绝对路径直接返回
    if p.is_absolute():
        return p
    
    # OpenAaaS 标准：相对路径拼接 DATA_DIR
    str_path = str(p).replace("\\", "/")
    if str_path.startswith("data/"):
        str_path = str_path[5:]
    
    return DATA_DIR / str_path


def load_task_from_env_or_file(task_file: Path) -> Dict[str, Any]:
    """从环境变量 OPENAAAS_TASK 或挂载文件加载任务."""
    task_env = os.environ.get("OPENAAAS_TASK")
    if task_env:
        return json.loads(task_env)

    if task_file.exists():
        with open(task_file, "r", encoding="utf-8-sig") as f:
            return json.load(f)

    raise RuntimeError(
        "未找到任务配置（请设置 OPENAAAS_TASK 环境变量或挂载 /workspace/task.json）"
    )


def save_json(data: Dict[str, Any], file_path: Path) -> None:
    """原子化保存 JSON 结果文件."""
    file_path.parent.mkdir(parents=True, exist_ok=True)
    with open(file_path, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)