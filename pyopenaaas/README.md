<p align="right">中文 | <a href="./README.en.md">English</a></p>

# OpenAaaS Python SDK

面向 **OpenAaaS** 智能体编排网络的 Python SDK，专为希望从纯 Python 或 Jupyter Notebook 中提交计算任务、管理结果并与 AI 智能体交互的研究人员和科学家设计。

## 安装

```bash
pip install pyopenaaas
# 或使用 uv
uv add pyopenaaas
```

开发依赖（可选）：

```bash
pip install pyopenaaas[dev]   # 带测试依赖
# 或
uv add --dev pyopenaaas       # uv 直接加 dev 依赖
```

## 快速开始

### 1. 注册与配置

```python
import pyopenaaas

# 选项 A：显式凭据
client = pyopenaaas.Client(
    server_url="https://api.open-aaas.com",
    api_key="your-api-key",
)

# 选项 B：从环境变量加载（OPENAAAS_SERVER_URL, OPENAAAS_API_KEY）
client = pyopenaaas.Client()
```

### 2. 发现服务

```python
with client:
    info = client.discover()
    print(info)

    services = client.list_services()
    for svc in services:
        print(svc.id, svc.name, svc.agent_status)
```

### 3. 提交任务

```python
with client:
    task = client.submit_task(
        service_id="quantum-chemistry-v1",
        task_prompt="Calculate the ground state energy of H2O using CCSD(T)/cc-pVTZ",
        output_prompt="Return the total energy in Hartree and a brief summary",
        input_files=["h2o.xyz"],
    )
    print(task)
```

### 4. 等待完成并下载结果

```python
with client:
    task = client.wait_for_task(task.id, poll_interval=10.0)
    if task.is_success():
        # 获取结果文件（实际结果在这里）
        paths = client.download_all_files(task.id, extract_zip=True)
        for p in paths:
            print("Saved:", p)

        # task.result 包含执行元数据（如 stdout、文件列表）
        if task.result:
            print("Output files:", task.result.files)
            print("Stdout:", task.result.stdout)
    else:
        print("Task failed:", task.result)
```

> **说明：** `task.result` 仅包含执行元数据（`stdout`、`files` 等）。completed 任务的**实际结果内容**需要通过 `client.list_files()` 和 `client.download_all_files()` 获取。

## 异步支持

所有方法都通过 :class:`AsyncClient` 提供异步版本：

```python
async with pyopenaaas.AsyncClient(
    server_url="https://api.open-aaas.com",
    api_key="your-api-key",
) as client:
    services = await client.list_services()
    task = await client.submit_task("svc-1", "Run MD simulation")
    task = await client.wait_for_task(task.id)
    paths = await client.download_all_files(task.id)
```

在 Jupyter Notebook 或 REPL 中，您也可以使用便捷助手：

```python
info = pyopenaaas.run(
    pyopenaaas.AsyncClient(
        server_url="https://api.open-aaas.com",
        api_key="your-api-key",
    ).discover()
)
```

> **Jupyter Notebook 用户：** `pyopenaaas.run()` 在底层使用 `asyncio.run()`，
> 无法嵌套在已有的事件循环中（Jupyter 默认存在）。
> 请改用 `await` 配合 :class:`AsyncClient`：
>
> ```python
> client = pyopenaaas.AsyncClient(
>     server_url="https://api.open-aaas.com",
>     api_key="your-api-key",
> )
> info = await client.discover()
> services = await client.list_services()
> ```

## 典型研究工作流

```python
import pyopenaaas

# 1. 初始化（从环境变量读取 OPENAAAS_API_KEY）
client = pyopenaaas.Client(server_url="https://api.open-aaas.com")

# 2. 选择服务
services = client.list_services()
ml_service = next(s for s in services if "ml" in s.name.lower())

# 3. 获取使用说明
usage = client.get_service_usage(ml_service.id)
print(usage.usage)

# 4. 提交一批实验
session_id = "batch-2024-06-02"
tasks = []
for params in ["exp1.in", "exp2.in", "exp3.in"]:
    t = client.submit_task(
        service_id=ml_service.id,
        task_prompt="Train a GNN on the attached dataset",
        input_files=[params],
        session_id=session_id,
    )
    tasks.append(t)

# 5. 等待所有任务完成
for t in tasks:
    final = client.wait_for_task(t.id, poll_interval=5.0)
    print(f"{final.id}: {final.status}")

# 6. 收集结果
for t in tasks:
    if client.get_task(t.id).is_success():
        client.download_all_files(t.id, save_dir=f"results/{t.id}")
```

## 配置优先级

SDK 按以下顺序解析设置（优先级从高到低）：

1. **代码关键字参数** — `Client(server_url="...", api_key="...")`
2. **环境变量** — `OPENAAAS_SERVER_URL`, `OPENAAAS_API_KEY`
3. **默认值** — `https://api.open-aaas.com`

## 异常层次结构

```
OpenAaaSError
├── AuthenticationError  (401 / 403)
├── NotFoundError        (404)
├── ConflictError        (409)
├── RequestValidationError      (400)
├── NetworkError         (connectivity)
└── RequestTimeoutError         (request timeout)
```

捕获基类以处理任何 SDK 错误：

```python
from pyopenaaas.exceptions import OpenAaaSError

try:
    client.submit_task("bad-id", "...")
except OpenAaaSError as exc:
    print("SDK error:", exc)
```

## 许可证

MIT
