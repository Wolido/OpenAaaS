# AutoGluon Executor for OpenAaaS

基于 AutoGluon Tabular 的自动机器学习执行器。

## 职责

- 接收 OpenAaaS 任务（通过环境变量 `OPENAAAS_TASK` 或 `/workspace/task.json`）
- 在数据本地完成训练（数据零迁移）
- 将结果写入 `/workspace/result.json`
- 模型与训练历史持久化在 `/workspace/models/` 和 `/workspace/autogluon.db`

## 文件树

agent-core/executor-autogluon/
├── Dockerfile
├── requirements.txt
├── main.py              # 运行逻辑（仅流程控制）
├── README.md            # 执行器说明文档
└── src/
    ├── __init__.py
    ├── config.py        # 配置与环境变量
    ├── models.py        # 数据模型
    ├── utils.py         # 工具函数
    ├── database.py      # SQLite 持久化
    └── training.py      # AutoGluon 训练核心

## 项目整体框架

agent-core (Rust, 常驻进程)
    ↓ docker run -v /workspace:/workspace ...
executor-autogluon (Docker 容器, 单次执行)
    ├─ main.py          ← 运行逻辑（流程控制）
    ├─ src/config.py    ← 路径、环境变量
    ├─ src/models.py    ← 数据对象定义
    ├─ src/utils.py     ← 日志、JSON 读写、路径解析
    ├─ src/database.py  ← SQLite 持久化
    └─ src/training.py  ← AutoGluon 训练核心（技术细节）


## 任务参数

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| mode | string | 是 | `split`（自动划分）或 `predefined`（指定训练/测试集） |
| data_path | string | split时 | 完整数据 CSV 路径（相对 `/workspace/data/`） |
| train_path | string | predefined时 | 训练集 CSV 路径 |
| test_path | string | predefined时 | 测试集 CSV 路径 |
| model_name | string | 否 | 自定义模型保存名称 |
| time_limit | int | 否 | 训练时间限制（秒），默认 6000 |
| presets | string | 否 | AutoGluon 预设，默认 `medium` |

## 运行

