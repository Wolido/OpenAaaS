#!/usr/bin/env python3
"""
OpenAaaS AutoGluon Executor
============================
被 agent-core 调用，执行单次自动机器学习任务。
运行逻辑：加载任务 -> 初始化环境 -> 执行训练 -> 保存结果 -> 退出。
"""

import sys
import threading
import time
import traceback
import json
from datetime import datetime
from typing import Any, Dict, Optional
from src.config import (
    DB_PATH,
    DEFAULT_TIME_LIMIT,
    HEARTBEAT_FILE,
    RESULT_FILE,
    TASK_FILE,
)
from src.database import init_db, save_task_record
from src.models import TaskConfig
from src.training import AutoGluonTrainer
from src.utils import load_task_from_env_or_file, log, resolve_path, save_json


def start_heartbeat(file_path, task_id: str, interval: int = 30) -> None:
    """
    后台线程：定期写入心跳文件。
    供 agent-core 确认长任务（20分钟+）仍在存活，避免被误判为僵死。
    """

    def _beat() -> None:
        while True:
            try:
                save_json(
                    {
                        "task_id": task_id,
                        "timestamp": datetime.now().isoformat(),
                        "status": "running",
                    },
                    file_path,
                )
            except Exception:
                pass
            time.sleep(interval)

    t = threading.Thread(target=_beat, daemon=True)
    t.start()
    log(f"心跳线程已启动（间隔 {interval} 秒）")


def parse_task(raw: Dict[str, Any]) -> TaskConfig:
    """
    从 OpenAaaS 标准 task.json 解析 AutoGluon 参数。
    
    OpenAaaS 的 Task 结构体只有固定字段：task_id, prompt, output_prompt, 
    session_id, input_files。AutoGluon 的自定义参数通过 prompt 字段以 
    JSON 字符串传递。
    
    参数来源优先级：
    1. prompt 字段中的 JSON 对象（OpenAaaS 标准方式）
    2. 顶层字段（直接兼容模式，便于本地测试）
    """
    task_id = raw.get("task_id", f"agl-{int(time.time())}")
    
    # 尝试从 prompt 字段解析 JSON 参数
    prompt = raw.get("prompt", "")
    params: Dict[str, Any] = {}
    if prompt and isinstance(prompt, str) and prompt.strip().startswith("{"):
        try:
            params = json.loads(prompt)
            log(f"从 prompt 字段解析到参数: {params}")
        except json.JSONDecodeError:
            log("prompt 字段不是有效 JSON，视为纯文本")
    
    # 合并参数：prompt 中的 JSON 优先，其次顶层字段（兼容本地直接测试）
    def get_param(key: str, default=None):
        return params.get(key, raw.get(key, default))
    
    return TaskConfig(
        task_id=task_id,
        mode=get_param("mode", "predefined"),
        train_path=str(rp) if (rp := resolve_path(get_param("train_path"))) else None,
        test_path=str(rp) if (rp := resolve_path(get_param("test_path"))) else None,
        data_path=str(rp) if (rp := resolve_path(get_param("data_path"))) else None,
        model_name=get_param("model_name"),
        time_limit=get_param("time_limit", DEFAULT_TIME_LIMIT),
        presets=get_param("presets", "medium"),
    )


def main() -> int:
    """主入口：流程控制."""
    log("=" * 60)
    log("OpenAaaS AutoGluon Executor 启动")
    log("=" * 60)

    raw_task: Optional[Dict[str, Any]] = None
    config: Optional[TaskConfig] = None

    try:
        # 1. 初始化持久化
        init_db()

        # 2. 加载任务（环境变量优先，其次挂载文件）
        raw_task = load_task_from_env_or_file(TASK_FILE)
        config = parse_task(raw_task)
        log(f"任务ID: {config.task_id} | 模式: {config.mode}")

        # 3. 启动心跳（解决 20 分钟+ 长任务超时问题）
        start_heartbeat(HEARTBEAT_FILE, config.task_id)

        # 4. 执行训练（所有技术细节在 training.py 中）
        trainer = AutoGluonTrainer(config)
        result = trainer.run()

        # 5. 构造结果字典
        result_dict: Dict[str, Any] = {
            "status": result.status,
            "最佳模型": result.best_model,
            "问题类型": result.problem_type,
            "评估指标": result.eval_metric,
            "模型性能": result.performance,
            "目标列": result.label,
            "训练样本数": result.train_samples,
            "测试样本数": result.test_samples,
            "特征数量": result.feature_count,
            "模型保存路径": result.model_path,
            "模型名称": result.model_name,
            "训练耗时秒": round(result.training_duration_sec, 2)
            if result.training_duration_sec
            else None,
        }

        # 6. 持久化：SQLite + result.json
        save_task_record(
            task_id=config.task_id,
            status="completed",
            result=result_dict,
            model_path=result.model_path,
            label=result.label,
            features=trainer.feature_columns,
        )
        save_json(result_dict, RESULT_FILE)

        log(f"✅ 任务完成，结果已保存: {RESULT_FILE}")
        log(f"   模型路径: {result.model_path}")
        return 0

    except Exception as e:
        error_msg = f"{str(e)}\n{traceback.format_exc()}"
        log(f"❌ 执行失败: {error_msg}")

        # 尽最大努力保存错误结果
        error_result = {
            "status": "failed",
            "error": str(e),
            "traceback": traceback.format_exc(),
        }
        try:
            save_json(error_result, RESULT_FILE)
        except Exception:
            pass

        # 尽最大努力记录到数据库
        try:
            task_id = (
                config.task_id
                if config
                else (raw_task.get("task_id", "unknown") if raw_task else "unknown")
            )
            save_task_record(task_id=task_id, status="failed", error=error_msg)
        except Exception:
            pass

        return 1


if __name__ == "__main__":
    sys.exit(main())