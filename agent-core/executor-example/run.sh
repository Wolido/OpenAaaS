#!/bin/bash
set -euo pipefail

TASK_JSON="/workspace/task.json"
OUTPUT_DIR="/workspace/output"
mkdir -p "$OUTPUT_DIR"

TASK_PROMPT=$(jq -r '.task_prompt // empty' "$TASK_JSON")
OUTPUT_PROMPT=$(jq -r '.output_prompt // empty' "$TASK_JSON")

if [ -z "$TASK_PROMPT" ]; then
    echo "Error: task_prompt is empty" >&2
    exit 1
fi

# =====================
# Step 1
# =====================
echo "[Step 1] Executing task..."
echo "Prompt preview:"
echo "--------------------------------------------------"
echo "$TASK_PROMPT"
echo "--------------------------------------------------"

STEP1_PROMPT=$(cat <<EOF
请完成以下任务，并将所有输出保存到 /workspace/output/ 目录：

${TASK_PROMPT}

注意：
1. 将主要结果保存到 /workspace/output/response.md
2. 如果有附件或数据文件，也保存到 /workspace/output/ 目录
3. 如果任务涉及代码，保存到 /workspace/output/ 目录
EOF
)

pi -p -c --tools read,write,edit,bash,ls,grep,todo \
   --append-system-prompt /opt/main-agent.md \
   "$STEP1_PROMPT" 2>&1 | tee /workspace/step1.log

echo "[Step 1] Completed"

# =====================
# Step 2
# =====================
echo "[Step 2] Formatting output..."
echo "Requirements preview:"
echo "--------------------------------------------------"
echo "$OUTPUT_PROMPT"
echo "--------------------------------------------------"

STEP2_PROMPT=$(cat <<EOF
请直接整理之前的执行结果，按照以下要求生成最终输出：

要求：${OUTPUT_PROMPT}

请执行以下操作：
1. 读取 /workspace/output/ 目录下的所有文件
2. 按照上述要求整理内容
3. 将最终整理后的内容写入 /workspace/output/response.md
4. 确保输出格式符合要求

如果 /workspace/output/response.md 已存在，请根据上述要求重新整理。

如果整理过程中遇到文件缺失或格式无法处理等问题，请尽量基于已有内容生成合理的最终输出，并说明遇到的问题。
EOF
)

pi -p -c --tools read,write,edit,bash,ls,grep,todo \
   --append-system-prompt /opt/main-agent.md \
   "$STEP2_PROMPT" 2>&1 \
|| { echo "Warning: Step 2 failed" >&2; }

# =====================
# Fallback: ensure response.md exists
# =====================
if [ ! -f "/workspace/output/response.md" ]; then
    echo "Warning: /workspace/output/response.md not found, generating from step1.log" >&2
    cp /workspace/step1.log /workspace/output/response.md
fi

# =====================
# Copy final response to /workspace
# =====================
cp /workspace/output/response.md /workspace/response.md

# =====================
# Final summary banner
# =====================
echo ""
echo "=================================================="
echo "Exit code: 0"
echo "=================================================="
echo ""
echo "Output files in /workspace/output/:"
ls -la /workspace/output/ 2>/dev/null || true
echo ""
echo "Preview of /workspace/response.md:"
echo "--------------------------------------------------"
head -n 50 /workspace/response.md || true
echo "--------------------------------------------------"
