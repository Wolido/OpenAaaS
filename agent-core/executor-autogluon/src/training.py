"""AutoGluon 训练核心模块."""

import pickle
import shutil
import time
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

import pandas as pd
from autogluon.tabular import TabularDataset, TabularPredictor
from sklearn.model_selection import train_test_split

from .config import DEFAULT_PRESETS, DEFAULT_TIME_LIMIT, MODELS_DIR
from .models import TaskConfig, TrainingResult
from .utils import log


class AutoGluonTrainer:
    """AutoGluon 自动机器学习训练器."""

    def __init__(self, config: TaskConfig) -> None:
        self.config = config
        self.model_path: Path = self._resolve_model_path()
        self.label: Optional[str] = None
        self.feature_columns: List[str] = []

    def _resolve_model_path(self) -> Path:
        """确定模型保存路径，若存在则清理旧模型."""
        if self.config.model_name:
            path = MODELS_DIR / self.config.model_name
        else:
            path = MODELS_DIR / f"model_{self.config.task_id}"
        path = path.resolve()

        if path.exists():
            shutil.rmtree(path)
            log(f"清理旧模型: {path}")
        return path

    def load_data(self) -> Tuple[pd.DataFrame, pd.DataFrame]:
        """加载并划分训练/测试数据."""
        mode = self.config.mode

        if mode == "split":
            if not self.config.data_path:
                raise ValueError("split 模式需要提供 data_path")
            data = pd.read_csv(self.config.data_path)
            train_df, test_df = train_test_split(
                data, test_size=0.2, random_state=42
            )
            self.label = data.columns[-1]
            log(f"自动划分: 训练集 {len(train_df)} 行, 测试集 {len(test_df)} 行")

        elif mode == "predefined":
            if not self.config.train_path or not self.config.test_path:
                raise ValueError("predefined 模式需要提供 train_path 和 test_path")
            train_df = TabularDataset(self.config.train_path)
            test_df = TabularDataset(self.config.test_path)
            self.label = train_df.columns[-1]
            log(f"加载完成: 训练集 {len(train_df)} 行, 测试集 {len(test_df)} 行")

        else:
            raise ValueError(f"不支持的模式: {mode}")

        if self.label not in train_df.columns:
            raise ValueError(f"目标列 '{self.label}' 不存在")

        self.feature_columns = [
            col for col in train_df.columns if col != self.label
        ]
        return train_df, test_df

    def train(self, train_df: pd.DataFrame) -> TabularPredictor:
        """执行 AutoGluon 训练."""
        log("初始化 AutoGluon...")
        predictor = TabularPredictor(
            label=self.label,
            path=str(self.model_path),
            eval_metric=None,
            verbosity=1,
        )

        log("开始训练模型（请耐心等待）...")
        predictor.fit(
            train_data=train_df,
            presets=self.config.presets,
            time_limit=self.config.time_limit,
        )
        return predictor

    def evaluate(
        self, predictor: TabularPredictor, test_df: pd.DataFrame
    ) -> Dict[str, Any]:
        """评估模型性能."""
        log("评估模型性能...")
        performance = predictor.evaluate(test_df, silent=True)
        best_model = (
            predictor.model_best
            if hasattr(predictor, "model_best")
            else "Unknown"
        )
        return {
            "performance": performance,
            "best_model": best_model,
            "problem_type": predictor.problem_type,
            "eval_metric": str(predictor.eval_metric),
        }

    def save_metadata(self) -> None:
        """保存特征元数据，供后续推理复用."""
        metadata = {
            "label": self.label,
            "feature_columns": self.feature_columns,
            "trained_at": datetime.now().isoformat(),
            "task_id": self.config.task_id,
        }
        meta_path = self.model_path / "feature_metadata.pkl"
        with open(meta_path, "wb") as f:
            pickle.dump(metadata, f)
        log(f"元数据已保存: {meta_path}")

    def run(self) -> TrainingResult:
        """执行完整训练流程并返回结构化结果."""
        start_time = time.time()
        train_df, test_df = self.load_data()
        predictor = self.train(train_df)
        eval_result = self.evaluate(predictor, test_df)
        self.save_metadata()
        duration = time.time() - start_time

        perf = eval_result["performance"]
        return TrainingResult(
            status="completed",
            best_model=eval_result["best_model"],
            problem_type=eval_result["problem_type"],
            eval_metric=eval_result["eval_metric"],
            performance={
                k: round(float(v), 4) if isinstance(v, (int, float)) else str(v)
                for k, v in perf.items()
            },
            label=self.label,
            train_samples=len(train_df),
            test_samples=len(test_df),
            feature_count=len(self.feature_columns),
            model_path=str(self.model_path),
            model_name=self.config.model_name or self.model_path.name,
            training_duration_sec=duration,
        )