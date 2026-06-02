"""数据模型定义."""

from dataclasses import dataclass
from typing import Any, Dict, List, Optional


@dataclass
class TaskConfig:
    """OpenAaaS 下发的任务配置."""

    task_id: str
    mode: str                      # 'split' | 'predefined'
    train_path: Optional[str] = None
    test_path: Optional[str] = None
    data_path: Optional[str] = None
    model_name: Optional[str] = None
    time_limit: int = 6000
    presets: str = "medium"


@dataclass
class TrainingResult:
    """训练结果数据结构."""

    status: str
    best_model: str
    problem_type: str
    eval_metric: str
    performance: Dict[str, Any]
    label: str
    train_samples: int
    test_samples: int
    feature_count: int
    model_path: str
    model_name: str
    training_duration_sec: Optional[float] = None