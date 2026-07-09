FROM python:3.11-slim

ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1

WORKDIR /app

# Install package, create non-root user, fix permissions in one layer
RUN pip install --no-cache-dir openaaas-mcp-adapter==0.3.1 && \
    useradd -m -u 1000 appuser && \
    chown -R appuser:appuser /app

USER appuser

ENTRYPOINT ["openaaas-mcp-adapter"]
