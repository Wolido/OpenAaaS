# OpenAaaS Python SDK

A Pythonic SDK for the **OpenAaaS** Agent orchestration network, designed for
researchers and scientists who want to submit compute tasks, manage results,
and interact with AI agents from plain Python or Jupyter notebooks.

## Installation

```bash
pip install pyopenaaas
# or with uv
uv add pyopenaaas
```

Development dependencies (optional):

```bash
pip install pyopenaaas[dev]   # includes test dependencies
# or
uv add --dev pyopenaaas       # add dev dependencies directly with uv
```

## Quick Start

### 1. Register & configure

```python
import pyopenaaas

# Option A: explicit credentials
client = pyopenaaas.Client(
    server_url="https://api.open-aaas.com",
    api_key="your-api-key",
)

# Option B: load from environment variables (OPENAAAS_SERVER_URL, OPENAAAS_API_KEY)
client = pyopenaaas.Client()
```

### 2. Discover services

```python
with client:
    info = client.discover()
    print(info)

    services = client.list_services()
    for svc in services:
        print(svc.id, svc.name, svc.agent_status)
```

### 3. Submit a task

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

### 4. Wait for completion & download results

```python
with client:
    task = client.wait_for_task(task.id, poll_interval=10.0)
    if task.is_success():
        # Get result files (actual results are here)
        paths = client.download_all_files(task.id, extract_zip=True)
        for p in paths:
            print("Saved:", p)

        # task.result contains execution metadata (e.g. stdout, file list)
        if task.result:
            print("Output files:", task.result.files)
            print("Stdout:", task.result.stdout)
    else:
        print("Task failed:", task.result)
```

> **Note:** `task.result` only contains execution metadata (`stdout`, `files`, etc.). The **actual result content** of a completed task must be retrieved via `client.list_files()` and `client.download_all_files()`.

## Async Support

All methods have async counterparts via :class:`AsyncClient`:

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

In a Jupyter notebook or REPL you can also use the convenience helper:

```python
info = pyopenaaas.run(
    pyopenaaas.AsyncClient(
        server_url="https://api.open-aaas.com",
        api_key="your-api-key",
    ).discover()
)
```

> **Jupyter Notebook users:** `pyopenaaas.run()` uses `asyncio.run()` under the hood,
> which cannot be nested inside an existing event loop (the default in Jupyter).
> Instead, use `await` directly with :class:`AsyncClient`:
>
> ```python
> client = pyopenaaas.AsyncClient(
>     server_url="https://api.open-aaas.com",
>     api_key="your-api-key",
> )
> info = await client.discover()
> services = await client.list_services()
> ```

## Typical Research Workflow

```python
import pyopenaaas

# 1. Initialise (reads OPENAAAS_API_KEY from env)
client = pyopenaaas.Client(server_url="https://api.open-aaas.com")

# 2. Pick a service
services = client.list_services()
ml_service = next(s for s in services if "ml" in s.name.lower())

# 3. Get usage instructions
usage = client.get_service_usage(ml_service.id)
print(usage.usage)

# 4. Submit a batch of experiments
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

# 5. Wait for all tasks
for t in tasks:
    final = client.wait_for_task(t.id, poll_interval=5.0)
    print(f"{final.id}: {final.status}")

# 6. Collect results
for t in tasks:
    if client.get_task(t.id).is_success():
        client.download_all_files(t.id, save_dir=f"results/{t.id}")
```

## Configuration Priority

The SDK resolves settings in the following order (highest first):

1. **Code kwargs** — `Client(server_url="...", api_key="...")`
2. **Environment variables** — `OPENAAAS_SERVER_URL`, `OPENAAAS_API_KEY`
3. **Defaults** — `https://api.open-aaas.com`

## Exception Hierarchy

```
OpenAaaSError
├── AuthenticationError  (401 / 403)
├── NotFoundError        (404)
├── ConflictError        (409)
├── RequestValidationError      (400)
├── NetworkError         (connectivity)
└── RequestTimeoutError         (request timeout)
```

Catch the base class to handle any SDK error:

```python
from pyopenaaas.exceptions import OpenAaaSError

try:
    client.submit_task("bad-id", "...")
except OpenAaaSError as exc:
    print("SDK error:", exc)
```

## License

MIT
