FROM python:3.10-slim

WORKDIR /app

# 安装编译依赖（AutoGluon 部分底层包需要 C 编译器）
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    libgomp1 \
    && rm -rf /var/lib/apt/lists/*

# 安装 Python 依赖
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# 复制源码：技术细节模块 + 主入口
COPY src/ ./src/
COPY main.py .

# OpenAaaS 标准工作区（agent-core 会通过 -v 挂载此目录）
VOLUME ["/workspace"]

ENTRYPOINT ["python", "main.py"]