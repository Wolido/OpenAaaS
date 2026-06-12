FROM node:22-slim

# Set pip domestic mirror
ENV PIP_INDEX_URL=https://pypi.tuna.tsinghua.edu.cn/simple

# Bootstrap ca-certificates (required for HTTPS mirrors) using default sources
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Replace apt source with Tsinghua mirror
RUN sed -i 's|URIs: http://deb.debian.org|URIs: https://mirrors.tuna.tsinghua.edu.cn|g' /etc/apt/sources.list.d/debian.sources

# Install system dependencies
RUN apt-get update && apt-get install -y \
    jq \
    git \
    curl \
    python3 \
    python3-pip \
    && rm -rf /var/lib/apt/lists/*

# Configure pip domestic mirror
RUN printf '[global]\nindex-url = https://pypi.tuna.tsinghua.edu.cn/simple\n' > /etc/pip.conf && \
    chmod 644 /etc/pip.conf

# Set npm registry to domestic mirror
RUN npm config set registry https://registry.npmmirror.com

# Install pi-coding-agent globally
RUN npm install -g --ignore-scripts @earendil-works/pi-coding-agent

# Create executor user
RUN useradd -m -s /bin/bash executor

# Copy project configurations
COPY pi/ /home/executor/.pi/
RUN mkdir -p /home/executor/.pi/agent/sessions && chown -R executor:executor /home/executor/.pi
COPY main-agent.md /opt/main-agent.md
COPY run.sh /opt/run.sh
RUN chmod +x /opt/run.sh
COPY entrypoint.sh /opt/entrypoint.sh
RUN chmod +x /opt/entrypoint.sh

# Set user and workdir
USER executor
WORKDIR /workspace

ENTRYPOINT ["/opt/entrypoint.sh"]
