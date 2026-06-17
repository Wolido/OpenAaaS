"""MCP Tools 注册：OpenAaaS 14 个核心工具"""

import httpx
import json
import mimetypes
import os
import re
import shutil
import time
import unicodedata
import zipfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from urllib.parse import quote, urlparse

from fastmcp import FastMCP

from .config import (
    get_config_dir,
    get_server_config,
    load_config,
    require_api_key,
    save_config,
    strip_trailing_slash,
)
from .http_client import (
    DOWNLOAD_TIMEOUT,
    UPLOAD_TIMEOUT,
    OpenAaaSError,
    safe_request,
)

# 常量
MAX_UPLOAD_SIZE = 100 * 1024 * 1024  # 100MB
MAX_DOWNLOAD_SIZE = 100 * 1024 * 1024  # 100MB
MAX_ZIP_RATIO = 500
MAX_TOTAL_EXTRACT_SIZE = 100 * 1024 * 1024  # 100MB
MAX_FILE_COUNT = 1000
MAX_SINGLE_FILE_SIZE = 50 * 1024 * 1024  # 50MB


def _sanitize_filename(filename: str, fallback_ext: str = "download") -> str:
    """清理文件名，防止路径遍历"""
    safe = os.path.basename(filename)
    safe = safe.replace("\x00", "")
    if not safe or safe in (".", "..", "/", "\\"):
        safe = f"result.{fallback_ext}"
    return safe


def _format_duration(
    started_at: str | None,
    completed_at: str | None,
    status: str,
) -> str:
    """格式化任务运行时长"""
    if not started_at:
        return ""

    start = _parse_iso_time(started_at)
    if not start:
        return ""

    if status in ("running", "cancelling"):
        end = datetime.now(timezone.utc)
    elif completed_at:
        end = _parse_iso_time(completed_at)
    else:
        return ""

    if not end:
        return ""

    total_seconds = int((end - start).total_seconds())
    if total_seconds < 0:
        return ""

    hours = total_seconds // 3600
    minutes = (total_seconds % 3600) // 60
    seconds = total_seconds % 60

    if hours > 0:
        return f"{hours}小时{minutes}分钟{seconds}秒"
    if minutes > 0:
        return f"{minutes}分钟{seconds}秒"
    return f"{seconds}秒"


def _parse_iso_time(ts: str) -> datetime | None:
    """解析 ISO 时间字符串"""
    if not ts:
        return None
    # 补齐缺少 Z 的时间
    if re.match(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?$", ts):
        ts = ts + "Z"
    try:
        ts = ts.replace("Z", "+00:00")
        dt = datetime.fromisoformat(ts)
        if dt.tzinfo is None:
            dt = dt.replace(tzinfo=timezone.utc)
        return dt
    except (ValueError, TypeError):
        return None


def _zipinfo_is_symlink(info: zipfile.ZipInfo) -> bool:
    """检测 zip 条目是否为符号链接"""
    if hasattr(info, "is_symlink"):
        return info.is_symlink()
    # Unix 符号链接: external_attr 高 4 位为 0xA
    return info.create_system == 3 and (info.external_attr >> 28) == 0xA


def _safe_extract_zip(zip_path: str | Path, extract_dir: str | Path) -> Path:
    """
    安全解压 zip 文件（含 zip 炸弹防护）

    Raises:
        OpenAaaSError: 任何安全问题或解压错误
    """
    zip_path = Path(zip_path)
    extract_dir = Path(extract_dir)
    extract_dir.mkdir(parents=True, exist_ok=True)
    real_extract_dir = extract_dir.resolve()

    try:
        with zipfile.ZipFile(zip_path, "r") as zf:
            infolist = zf.infolist()

            if len(infolist) > MAX_FILE_COUNT:
                raise OpenAaaSError(
                    f"zip 文件包含过多文件 ({len(infolist)} > {MAX_FILE_COUNT})，可能存在 zip 炸弹风险"
                )

            total_size = sum(info.file_size for info in infolist)
            if total_size > MAX_TOTAL_EXTRACT_SIZE:
                raise OpenAaaSError(
                    f"zip 文件解压后总大小过大 ({total_size} bytes > {MAX_TOTAL_EXTRACT_SIZE} bytes)"
                )

            for info in infolist:
                if info.file_size > MAX_SINGLE_FILE_SIZE:
                    raise OpenAaaSError(
                        f"zip 文件包含过大文件: {info.filename} ({info.file_size} bytes)"
                    )

            zip_size = zip_path.stat().st_size
            if zip_size > 0 and total_size / zip_size > MAX_ZIP_RATIO:
                raise OpenAaaSError(
                    f"zip 文件压缩比异常 ({total_size / zip_size:.1f} > {MAX_ZIP_RATIO})，可能存在 zip 炸弹风险"
                )

            for info in infolist:
                if _zipinfo_is_symlink(info):
                    raise OpenAaaSError(
                        f"zip 文件包含符号链接: {info.filename}，已拒绝解压"
                    )

                extracted_path = extract_dir / info.filename
                real_extracted_path = extracted_path.resolve()
                try:
                    real_extracted_path.relative_to(real_extract_dir)
                except ValueError:
                    raise OpenAaaSError(
                        f"zip 文件包含非法路径: {info.filename}"
                    )

                zf.extract(info, extract_dir)

                # 二次验证（防御 TOCTOU + 符号链接创建后跟随）
                if extracted_path.exists():
                    real_after = extracted_path.resolve()
                    try:
                        real_after.relative_to(real_extract_dir)
                    except ValueError:
                        try:
                            extracted_path.unlink()
                        except IsADirectoryError:
                            shutil.rmtree(extracted_path, ignore_errors=True)
                        raise OpenAaaSError(
                            f"zip 文件包含路径穿越: {info.filename}"
                        )

        return extract_dir
    except zipfile.BadZipFile as e:
        raise OpenAaaSError(f"zip 文件损坏: {e}")
    except OpenAaaSError:
        raise
    except Exception as e:
        raise OpenAaaSError(f"解压失败: {e}")


def _get_download_dir(task_id: str) -> Path:
    """获取任务下载目录：.OpenAaaS/downloads/{task_id}/"""
    safe_task_id = re.sub(r"[\\/]", "_", task_id)
    safe_task_id = safe_task_id.replace("..", "_")
    if safe_task_id in (".", ".."):
        safe_task_id = "_"
    if not safe_task_id:
        safe_task_id = "_"
    return Path(os.getcwd()) / ".OpenAaaS" / "downloads" / safe_task_id


def _check_file_in_working_dir(file_path: Path) -> None:
    """检查文件是否位于当前工作目录下，防止路径遍历"""
    try:
        cwd = Path(os.getcwd()).resolve()
        real_path = file_path.resolve()
        try:
            relative = real_path.relative_to(cwd)
        except ValueError:
            raise OpenAaaSError(
                f"文件上传失败: 只能上传当前工作目录下的文件: {file_path}"
            )
        # 防御 .. 路径
        parts = relative.parts
        if ".." in parts:
            raise OpenAaaSError(
                f"文件上传失败: 只能上传当前工作目录下的文件: {file_path}"
            )
    except OSError as e:
        raise OpenAaaSError(f"文件上传失败: 无法解析文件路径: {e}")


def register_tools(mcp: FastMCP) -> None:
    """注册所有 OpenAaaS MCP Tools"""

    # ------------------------------------------------------------------
    # 1. discover
    # ------------------------------------------------------------------
    @mcp.tool()
    def discover(server_url: str) -> str:
        """发现服务端 API 信息"""
        url = f"{strip_trailing_slash(server_url)}/api/v1/discovery"
        try:
            result = safe_request("GET", url)
        except OpenAaaSError as e:
            return f"❌ 发现失败: {e}"

        api_info = result.get("api", {}) if isinstance(result, dict) else {}
        version = api_info.get("version", "unknown") if isinstance(api_info, dict) else "unknown"
        base_url = api_info.get("base_url", server_url) if isinstance(api_info, dict) else server_url

        auth = "Bearer Token (通过 register 获取 api_key)"
        for key in ("authentication", "auth"):
            val = result.get(key)
            if val:
                if isinstance(val, str):
                    auth = val
                elif isinstance(val, dict):
                    auth = str(val.get("type") or val.get("method") or json.dumps(val, ensure_ascii=False))
                break

        lines = [
            f"✅ 成功获取服务端 API 信息",
            f"服务端版本: {version}",
            f"Base URL: {base_url}",
            f"认证方式: {auth}",
        ]

        endpoints = result.get("endpoints")
        if isinstance(endpoints, list) and endpoints:
            lines.append(f"\n可用端点 ({len(endpoints)} 个):")
            for ep in endpoints:
                if isinstance(ep, dict):
                    name = ep.get("name", "unnamed")
                    method = ep.get("method", "?")
                    path = ep.get("path", "")
                    lines.append(f"  - {name}: [{method}] {path}")
                else:
                    lines.append(f"  - {ep}")

        services = result.get("services")
        if isinstance(services, list) and services:
            lines.append(f"\n已注册服务 ({len(services)} 个):")
            for svc in services:
                if isinstance(svc, dict):
                    lines.append(f"  - {svc.get('name', svc.get('id', 'unknown'))}")
                else:
                    lines.append(f"  - {svc}")

        return "\n".join(lines)

    # ------------------------------------------------------------------
    # 2. set_server_url
    # ------------------------------------------------------------------
    @mcp.tool()
    def set_server_url(server_url: str, server: str = "") -> str:
        """设置服务器地址并保存到配置文件"""
        server_url = server_url.strip()
        if not server_url:
            return "❌ 缺少必填参数: server_url"

        try:
            parsed = urlparse(server_url)
        except Exception:
            return "❌ 参数错误: server_url 必须是有效的 URL"

        if parsed.scheme not in ("http", "https"):
            return "❌ 参数错误: server_url 必须以 http:// 或 https:// 开头"
        if not parsed.hostname:
            return "❌ 参数错误: server_url 必须包含有效的主机地址"

        config = load_config()
        alias = server or config.get("default_server", "default")
        servers = config.setdefault("servers", {})
        existing = servers.get(alias)

        target_url = strip_trailing_slash(server_url)

        # 已有 api_key 时，若地址变更则拒绝覆盖
        if existing and existing.get("api_key"):
            old_url = strip_trailing_slash(existing.get("server_url", ""))
            if old_url and old_url != target_url:
                return (
                    f'❌ 服务器 "{alias}" 已有注册信息（地址: {existing.get("server_url")}，api_key 存在）。\n'
                    "直接修改会清除原有注册信息。\n\n"
                    "如需添加新服务器，请使用新的 server 别名，例如：\n"
                    f'  server="new-alias" server_url="{server_url}"\n\n'
                    "如需修改该服务器地址，请先使用 remove_server 删除旧配置，或使用其他别名。"
                )

        # 检查是否与其他已注册别名冲突
        for other_alias, other_srv in servers.items():
            if other_alias == alias:
                continue
            other_url = strip_trailing_slash(other_srv.get("server_url", ""))
            if other_url == target_url and other_srv.get("api_key"):
                return (
                    "❌ 该服务器地址已被其他别名注册过。\n"
                    f"服务器别名: {other_alias}\n"
                    f"服务器地址: {other_srv.get('server_url')}\n\n"
                    "如需使用此别名，请先使用 remove_server 删除已有配置。"
                )

        if alias not in servers:
            servers[alias] = {}
        old_url = strip_trailing_slash(servers[alias].get("server_url", ""))
        servers[alias]["server_url"] = target_url

        warning = ""
        if old_url and old_url != target_url and not servers[alias].get("api_key"):
            warning = f'\n\n⚠️ 服务器 "{alias}" 地址已从 {old_url} 更换为 {target_url}。'

        save_config(config)
        return (
            f'✅ 服务器 "{alias}" 地址设置成功！已保存到 config.json: {target_url}{warning}'
        )

    # ------------------------------------------------------------------
    # 3. register
    # ------------------------------------------------------------------
    @mcp.tool()
    def register(name: str, server: str = "") -> str:
        """注册客户端账号，获取 api_key（仅需一次）"""
        name = name.strip()
        if not name:
            return "❌ 缺少必填参数: name"
        if len(name) > 64:
            return "❌ 参数错误: name 长度不能超过 64 字符"
        if re.search(r'[\x00-\x1f/\\<>|&;$]', name):
            return "❌ 参数错误: name 包含非法字符"
        if any(unicodedata.category(c).startswith('C') for c in name):
            return "❌ 参数错误: name 包含非法字符（Unicode 控制字符）"

        try:
            sc = get_server_config(server)
        except RuntimeError as e:
            return f"❌ {e}"

        alias = sc["alias"]
        server_url = strip_trailing_slash(sc.get("server_url", ""))
        if not server_url:
            return "❌ 服务器地址未配置，请先使用 set_server_url 设置"

        # 已注册检查
        if sc.get("api_key"):
            return (
                f"✅ 已注册，无需重复注册。\n"
                f"服务器别名: {alias}\n"
                f"服务器地址: {server_url}\n"
                f"客户端 ID: {sc.get('client_id', 'unknown')}\n"
                f"用户名: {sc.get('name', 'unknown')}\n\n"
                "如需修改用户名，请使用 update_profile。\n"
                "如需切换到其他服务器，请先用 set_server_url 修改地址，再重新注册。"
            )

        # 检查其他 alias 是否已用相同 URL 注册
        config = load_config()
        for other_alias, other_srv in config.get("servers", {}).items():
            if other_alias == alias:
                continue
            other_url = strip_trailing_slash(other_srv.get("server_url", ""))
            if other_url == server_url and other_srv.get("api_key"):
                return (
                    "❌ 该服务器地址已被其他别名注册过。\n"
                    f"服务器别名: {other_alias}\n"
                    f"服务器地址: {other_srv.get('server_url')}\n"
                    f"客户端 ID: {other_srv.get('client_id', 'unknown')}\n"
                    f"用户名: {other_srv.get('name', 'unknown')}\n\n"
                    "如需使用其他别名，请先使用 remove_server 删除已有配置。"
                )

        url = f"{server_url}/api/v1/client/auth/register"
        try:
            result = safe_request("POST", url, data={"name": name})
        except OpenAaaSError as e:
            return f"❌ 注册失败: {e}"

        api_key = result.get("api_key") or result.get("token")
        client_id = result.get("client_id") or result.get("id")

        if api_key:
            config = load_config()
            servers = config.setdefault("servers", {})
            if alias not in servers:
                servers[alias] = {}
            servers[alias]["api_key"] = api_key
            if client_id:
                servers[alias]["client_id"] = client_id
            servers[alias]["name"] = name
            servers[alias]["server_url"] = server_url
            save_config(config)
            saved = True
        else:
            saved = False

        return (
            f"✅ 注册成功！服务器: {alias}，客户端 ID: {client_id}。\n"
            f"API Key 已{'自动保存' if saved else '返回，请手动保存'}到 config.json"
        )

    # ------------------------------------------------------------------
    # 4. update_profile
    # ------------------------------------------------------------------
    @mcp.tool()
    def update_profile(name: str, server: str = "") -> str:
        """修改当前客户端用户名"""
        name = name.strip()
        if not name:
            return "❌ 参数错误: name 不能为空"
        if len(name) > 64:
            return "❌ 参数错误: name 长度不能超过 64 字符"
        if re.search(r'[\x00-\x1f/\\<>|&;$]', name):
            return "❌ 参数错误: name 包含非法字符"
        if any(unicodedata.category(c).startswith('C') for c in name):
            return "❌ 参数错误: name 包含非法字符（Unicode 控制字符）"

        try:
            sc = get_server_config(server)
            api_key = require_api_key(server)
        except RuntimeError as e:
            return f"❌ {e}"

        alias = sc["alias"]
        server_url = strip_trailing_slash(sc.get("server_url", ""))
        if not server_url:
            return "❌ 服务器地址未配置，请先使用 set_server_url 设置"
        url = f"{server_url}/api/v1/client/profile"

        try:
            result = safe_request(
                "PUT",
                url,
                headers={"Authorization": f"Bearer {api_key}"},
                data={"name": name},
            )
        except OpenAaaSError as e:
            return f"❌ 更新失败: {e}"

        client_id = result.get("client_id") or result.get("id")
        updated_name = result.get("name", name)

        config = load_config()
        servers = config.setdefault("servers", {})
        if alias not in servers:
            servers[alias] = {}
        servers[alias]["name"] = updated_name
        save_config(config)

        return f"✅ 用户名更新成功！新用户名: {updated_name}"

    # ------------------------------------------------------------------
    # 5. list_services
    # ------------------------------------------------------------------
    @mcp.tool()
    def list_services(server: str = "") -> str:
        """获取可用的 Agent 服务列表（轻量摘要）"""
        try:
            sc = get_server_config(server)
            api_key = require_api_key(server)
        except RuntimeError as e:
            return f"❌ {e}"

        server_url = strip_trailing_slash(sc.get("server_url", ""))
        url = f"{server_url}/api/v1/client/services"

        try:
            result = safe_request(
                "GET", url, headers={"Authorization": f"Bearer {api_key}"}
            )
        except OpenAaaSError as e:
            return f"❌ 请求失败: {e}"

        services = result if isinstance(result, list) else result.get("services", [])
        if not services:
            return "暂无可用的 Agent 服务"

        status_icon = {"online": "🟢", "offline": "🔴", "unknown": "⚪"}
        lines = [f"找到 {len(services)} 个可用服务："]

        for i, svc in enumerate(services):
            if not isinstance(svc, dict):
                continue
            svc_id = svc.get("id", "unknown")
            svc_name = svc.get("name", "未命名")
            description = svc.get("description", "无描述")
            agent_status = svc.get("agent_status", "unknown")
            access_type = svc.get("access_type", "unknown")
            has_permission = svc.get("has_permission") is True
            reg_status = svc.get("registration_status")

            icon = status_icon.get(agent_status, "⚪")
            perm = "✅ 有权限" if has_permission else "❌ 无权限"

            lines.append(f"{i + 1}. {svc_name}")
            lines.append(f"   ID: {svc_id}")
            lines.append(f"   状态: {icon} {agent_status}")
            lines.append(f"   访问类型: {access_type}")
            lines.append(f"   权限: {perm}")
            lines.append(f"   描述: {description}")
            if reg_status:
                lines.append(f"   注册状态: {reg_status}")
            if i < len(services) - 1:
                lines.append("")

        return "\n".join(lines)

    # ------------------------------------------------------------------
    # 6. get_service_usage
    # ------------------------------------------------------------------
    @mcp.tool()
    def get_service_usage(service_id: str, server: str = "") -> str:
        """获取指定服务的详细用法说明（能力范围、调用规范、返回格式、限制条件）"""
        if not service_id:
            return "❌ 缺少必填参数: service_id"

        try:
            sc = get_server_config(server)
            api_key = require_api_key(server)
        except RuntimeError as e:
            return f"❌ {e}"

        server_url = strip_trailing_slash(sc.get("server_url", ""))
        url = f"{server_url}/api/v1/client/services/{quote(service_id, safe='')}/usage"

        try:
            result = safe_request(
                "GET", url, headers={"Authorization": f"Bearer {api_key}"}
            )
        except OpenAaaSError as e:
            return f"❌ 请求失败: {e}"

        usage = result.get("usage", "无 usage 说明") if isinstance(result, dict) else "无 usage 说明"
        name = result.get("name", "未命名") if isinstance(result, dict) else "未命名"

        return f"服务: {name}\n\nUsage:\n{usage}"

    # ------------------------------------------------------------------
    # 7. submit_task
    # ------------------------------------------------------------------
    @mcp.tool()
    def submit_task(
        service_id: str,
        task_prompt: str,
        output_prompt: str = "",
        input_files: list[str] | None = None,
        session_id: str = "",
        server: str = "",
    ) -> str:
        """
        提交任务到远程 Agent（支持文件上传）

        input_files 为本地文件路径列表，只能上传当前工作目录下的文件，单个文件不超过 100MB。
        """
        if not service_id:
            return "❌ 缺少必填参数: service_id"
        if not task_prompt:
            return "❌ 缺少必填参数: task_prompt"

        try:
            sc = get_server_config(server)
            api_key = require_api_key(server)
        except RuntimeError as e:
            return f"❌ {e}"

        alias = sc["alias"]
        server_url = strip_trailing_slash(sc.get("server_url", ""))
        url = f"{server_url}/api/v1/client/tasks"

        # 表单字段
        fields: dict[str, str] = {
            "service_id": service_id,
            "task_prompt": task_prompt,
            "output_prompt": output_prompt or "",
        }
        if session_id:
            fields["session_id"] = session_id

        # 处理文件上传
        files: list[tuple[str, tuple[str, bytes, str]]] = []
        if input_files:
            if len(input_files) > 10:
                return "❌ 文件上传失败: 最多支持 10 个文件"

            for file_path_str in input_files:
                file_path = Path(file_path_str)
                if not file_path.is_absolute():
                    file_path = Path(os.getcwd()) / file_path

                _check_file_in_working_dir(file_path)
                if file_path.is_symlink():
                    return f"❌ 文件上传失败: 不支持符号链接: {file_path}"

                if not file_path.exists():
                    return f"❌ 文件上传失败: 文件不存在: {file_path}"
                if not file_path.is_file():
                    return f"❌ 文件上传失败: 路径不是文件: {file_path}"

                try:
                    file_size = file_path.stat().st_size
                except OSError as e:
                    return f"❌ 文件上传失败: 无法获取文件信息: {e}"

                if file_size > MAX_UPLOAD_SIZE:
                    return (
                        f"❌ 文件上传失败: 文件过大: {file_size} bytes，"
                        f"超过 {MAX_UPLOAD_SIZE} bytes 限制"
                    )

                try:
                    content = file_path.read_bytes()
                except OSError as e:
                    return f"❌ 文件上传失败: 无法读取文件: {e}"

                # 二次校验（防御 TOCTOU）
                if len(content) > MAX_UPLOAD_SIZE:
                    return (
                        f"❌ 文件上传失败: 文件过大: {len(content)} bytes，"
                        f"超过 {MAX_UPLOAD_SIZE} bytes 限制"
                    )

                mime_type = mimetypes.guess_type(str(file_path))[0] or "application/octet-stream"
                files.append((
                    "files",
                    (file_path.name, content, mime_type),
                ))

        try:
            result = safe_request(
                "POST",
                url,
                headers={"Authorization": f"Bearer {api_key}"},
                data=fields,
                files=files if files is not None else [],
                timeout=UPLOAD_TIMEOUT,
            )
        except OpenAaaSError as e:
            return f"❌ 提交失败: {e}"

        task_id = result.get("id") or result.get("task_id")
        status = result.get("status", "unknown")

        return (
            f"✅ 任务提交成功！\n"
            f"任务 ID: {task_id}\n"
            f"状态: {status}\n\n"
            "⏳ 任务正在后台执行。请询问用户是否需要等待结果。\n"
            "如果用户没有明确说'帮我等结果'或'轮询任务'，不要主动调用 poll_task 或 get_task。"
        )

    # ------------------------------------------------------------------
    # 8. get_task
    # ------------------------------------------------------------------
    @mcp.tool()
    def get_task(task_id: str, server: str = "") -> str:
        """查询任务状态和最终结果（仅在用户明确要求时调用）

        ⚠️ 不要主动轮询。只有在用户明确要求查询任务状态时，才调用此工具。
        """
        if not task_id:
            return "❌ 缺少必填参数: task_id"

        try:
            sc = get_server_config(server)
            api_key = require_api_key(server)
        except RuntimeError as e:
            return f"❌ {e}"

        server_url = strip_trailing_slash(sc.get("server_url", ""))
        url = f"{server_url}/api/v1/client/tasks/{quote(task_id, safe='')}"

        try:
            result = safe_request(
                "GET", url, headers={"Authorization": f"Bearer {api_key}"}
            )
        except OpenAaaSError as e:
            return f"❌ 查询失败: {e}"

        status = result.get("status", "unknown") if isinstance(result, dict) else "unknown"
        status_desc = {
            "pending": "等待中",
            "accepted": "已接受",
            "running": "执行中",
            "completed": "已完成",
            "failed": "失败",
            "cancelled": "已取消",
            "cancelling": "取消中",
        }.get(status, status)

        tid = result.get("id") or result.get("task_id") or task_id
        started_at = result.get("started_at") or result.get("created_at")
        completed_at = result.get("completed_at")

        duration_str = _format_duration(started_at, completed_at, status)

        lines = [f"任务状态: {status_desc}", f"任务 ID: {tid}"]

        if status == "completed":
            lines.append("")
            lines.append("✅ 任务已完成！可以使用 download_result 工具下载结果文件。")
            result_data = result.get("result") if isinstance(result, dict) else None
            if isinstance(result_data, dict):
                lines.append("")
                lines.append("执行结果摘要:")
                if result_data.get("summary"):
                    lines.append(str(result_data["summary"]))
                elif isinstance(result_data.get("output"), str):
                    out = result_data["output"]
                    lines.append(out[:500] + ("..." if len(out) > 500 else ""))
        elif status == "failed":
            lines.append("")
            lines.append("❌ 任务执行失败")
            result_data = result.get("result") if isinstance(result, dict) else None
            if isinstance(result_data, dict):
                err_msg = result_data.get("error") or result_data.get("message")
                if err_msg:
                    lines.append(f"错误信息: {err_msg}")
        elif status in ("pending", "accepted", "running"):
            lines.append("")
            lines.append("⏳ 任务正在执行中，请稍后再次查询...")

        if duration_str:
            lines.append(f"⏱️ 运行时长: {duration_str}")

        return "\n".join(lines)

    # ------------------------------------------------------------------
    # 9. poll_task
    # ------------------------------------------------------------------
    @mcp.tool()
    def poll_task(
        task_id: str,
        timeout_seconds: float | None = None,
        server: str = "",
    ) -> str:
        """轮询任务直到获得最终结果（仅用户明确要求时使用）

        每 20 秒查询一次任务状态，直到任务进入 completed / failed / cancelled 状态。
        默认不设置超时；传入 timeout_seconds 可限制最大轮询时长。

        ⚠️ 重要：Agent 禁止主动调用此工具。ONLY use this tool when the user explicitly
        asks to wait for or poll a task result (e.g. "帮我等结果"、"轮询一下任务").
        如果用户没有明确表达等待意图，应询问用户而不是直接轮询。
        """
        if not task_id:
            return "❌ 缺少必填参数: task_id"

        try:
            sc = get_server_config(server)
            api_key = require_api_key(server)
        except RuntimeError as e:
            return f"❌ {e}"

        server_url = strip_trailing_slash(sc.get("server_url", ""))
        url = f"{server_url}/api/v1/client/tasks/{quote(task_id, safe='')}"

        poll_interval = 20
        start_time = time.monotonic()
        iterations = 0

        while True:
            iterations += 1
            try:
                result = safe_request(
                    "GET", url, headers={"Authorization": f"Bearer {api_key}"}
                )
            except OpenAaaSError as e:
                return f"❌ 查询失败: {e}"

            status = result.get("status", "unknown") if isinstance(result, dict) else "unknown"
            if status in ("completed", "failed", "cancelled"):
                elapsed = int(time.monotonic() - start_time)
                status_desc = {
                    "completed": "已完成",
                    "failed": "失败",
                    "cancelled": "已取消",
                }.get(status, status)

                tid = result.get("id") or result.get("task_id") or task_id
                lines = [
                    f"轮询结束，任务状态: {status_desc}",
                    f"任务 ID: {tid}",
                    f"轮询次数: {iterations}",
                    f"总耗时: {elapsed} 秒",
                ]

                if status == "completed":
                    lines.append("")
                    lines.append("任务已完成，可以使用 download_result 下载结果文件。")
                    result_data = result.get("result") if isinstance(result, dict) else None
                    if isinstance(result_data, dict):
                        lines.append("")
                        lines.append("执行结果摘要:")
                        if result_data.get("summary"):
                            lines.append(str(result_data["summary"]))
                        elif isinstance(result_data.get("output"), str):
                            out = result_data["output"]
                            lines.append(out[:500] + ("..." if len(out) > 500 else ""))
                elif status == "failed":
                    lines.append("")
                    lines.append("❌ 任务执行失败")
                    result_data = result.get("result") if isinstance(result, dict) else None
                    if isinstance(result_data, dict):
                        err_msg = result_data.get("error") or result_data.get("message")
                        if err_msg:
                            lines.append(f"错误信息: {err_msg}")

                return "\n".join(lines)

            if timeout_seconds is not None:
                elapsed = time.monotonic() - start_time
                if elapsed >= timeout_seconds:
                    return (
                        f"⏰ 轮询超时（已等待 {int(elapsed)} 秒，超过 {timeout_seconds} 秒）\n"
                        f"任务 ID: {task_id}\n"
                        f"当前状态: {status}\n"
                        f"轮询次数: {iterations}"
                    )

            time.sleep(poll_interval)

    # ------------------------------------------------------------------
    # 10. cancel_task
    # ------------------------------------------------------------------
    @mcp.tool()
    def cancel_task(task_id: str, server: str = "") -> str:
        """取消执行中的任务"""
        if not task_id:
            return "❌ 缺少必填参数: task_id"

        try:
            sc = get_server_config(server)
            api_key = require_api_key(server)
        except RuntimeError as e:
            return f"❌ {e}"

        server_url = strip_trailing_slash(sc.get("server_url", ""))
        url = f"{server_url}/api/v1/client/tasks/{quote(task_id, safe='')}/cancel"

        try:
            result = safe_request(
                "POST",
                url,
                headers={
                    "Authorization": f"Bearer {api_key}",
                    "Content-Type": "application/json",
                },
            )
        except OpenAaaSError as e:
            return f"❌ 取消失败: {e}"

        status = result.get("status", "unknown") if isinstance(result, dict) else "unknown"

        if status == "cancelled":
            return f"✅ 任务已取消\n任务 ID: {task_id}\n状态: 已取消"
        elif status == "cancelling":
            return (
                f"⏳ 任务正在取消中\n"
                f"任务 ID: {task_id}\n"
                "状态: 取消中（Agent 将收到取消信号）"
            )
        else:
            return f"任务状态: {status}\n任务 ID: {task_id}"

    # ------------------------------------------------------------------
    # 10. list_files
    # ------------------------------------------------------------------
    @mcp.tool()
    def list_files(task_id: str, server: str = "") -> str:
        """列出任务的结果文件"""
        if not task_id:
            return "❌ 缺少必填参数: task_id"

        try:
            sc = get_server_config(server)
            api_key = require_api_key(server)
        except RuntimeError as e:
            return f"❌ {e}"

        server_url = strip_trailing_slash(sc.get("server_url", ""))
        url = f"{server_url}/api/v1/client/files/list/{quote(task_id, safe='')}"

        try:
            result = safe_request(
                "GET", url, headers={"Authorization": f"Bearer {api_key}"}
            )
        except OpenAaaSError as e:
            return f"❌ 获取文件列表失败: {e}"

        files = result if isinstance(result, list) else result.get("files", [])
        if not files:
            return f"任务 {task_id} 没有结果文件"

        lines = [f"任务 {task_id} 共有 {len(files)} 个结果文件："]
        for i, f in enumerate(files):
            if not isinstance(f, dict):
                continue
            filename = f.get("filename") or f.get("name", "unnamed")
            file_id = f.get("id") or f.get("file_id", "")
            size = f.get("size")
            if size is None:
                size = f.get("file_size", "未知")
            lines.append(f"{i + 1}. {filename} (ID: {file_id}, 大小: {size})")

        return "\n".join(lines)

    # ------------------------------------------------------------------
    # 11. download_result
    # ------------------------------------------------------------------
    @mcp.tool()
    def download_result(
        task_id: str,
        file_id: str = "",
        download_all: bool = False,
        server: str = "",
    ) -> str:
        """
        下载任务结果文件并返回每个文件的完整路径

        - file_id: 指定要下载的文件 ID，不指定则默认下载第一个 zip 文件或第一个文件
        - download_all: 是否下载该任务的所有结果文件

        返回结果中会明确列出每个文件的完整路径，读取文件时请直接使用这些路径，不要推测子目录。
        """
        if not task_id:
            return "❌ 缺少必填参数: task_id"

        try:
            sc = get_server_config(server)
            api_key = require_api_key(server)
        except RuntimeError as e:
            return f"❌ {e}"

        server_url = strip_trailing_slash(sc.get("server_url", ""))

        # 1. 获取文件列表
        list_url = f"{server_url}/api/v1/client/files/list/{quote(task_id, safe='')}"
        try:
            list_result = safe_request(
                "GET", list_url, headers={"Authorization": f"Bearer {api_key}"}
            )
        except OpenAaaSError as e:
            return f"❌ 获取文件列表失败: {e}"

        files = (
            list_result
            if isinstance(list_result, list)
            else list_result.get("files", [])
        )
        if not files:
            return f"❌ 任务 {task_id} 没有可下载的结果文件"

        # 2. 确定要下载的文件
        target_files: list[dict[str, Any]] = []
        if download_all:
            target_files = [f for f in files if isinstance(f, dict)]
        elif file_id:
            matched = None
            for f in files:
                if isinstance(f, dict) and (f.get("id") == file_id or f.get("file_id") == file_id):
                    matched = f
                    break
            if not matched:
                return f"❌ 找不到指定的文件 ID: {file_id}"
            target_files = [matched]
        else:
            zip_files = [
                f for f in files
                if isinstance(f, dict) and str(f.get("filename", "")).lower().endswith(".zip")
            ]
            target_files = [zip_files[0]] if zip_files else [files[0]]

        download_dir = _get_download_dir(task_id)
        download_dir.mkdir(parents=True, exist_ok=True)

        downloaded: list[dict[str, str]] = []
        errors: list[str] = []
        extracted_dirs: list[str] = []

        for target_file in target_files:
            if not isinstance(target_file, dict):
                errors.append("文件信息格式错误")
                continue

            fid = target_file.get("id") or target_file.get("file_id")
            filename = target_file.get("filename") or target_file.get("name") or f"{fid}.download"
            if not fid:
                errors.append(f"文件缺少 ID，无法下载: {filename}")
                continue

            safe_name = _sanitize_filename(filename, "zip" if filename.lower().endswith(".zip") else "download")
            # 处理重名
            save_path = download_dir / safe_name
            counter = 1
            stem = save_path.stem
            suffix = save_path.suffix
            while save_path.exists():
                save_path = download_dir / f"{stem}_{counter}{suffix}"
                counter += 1

            # 下载文件
            download_url = f"{server_url}/api/v1/client/files/{quote(fid, safe='')}/download"
            try:
                with httpx.Client(timeout=DOWNLOAD_TIMEOUT, follow_redirects=True) as client:
                    response = client.get(
                        download_url, headers={"Authorization": f"Bearer {api_key}"}
                    )
                    response.raise_for_status()
                    content_length = response.headers.get("content-length")
                    if content_length is not None:
                        try:
                            if int(content_length) > MAX_DOWNLOAD_SIZE:
                                errors.append(
                                    f"{filename}: 文件大小 ({content_length} bytes) 超过下载限制 "
                                    f"({MAX_DOWNLOAD_SIZE} bytes)"
                                )
                                continue
                        except ValueError:
                            pass
                    content = response.content
                    if len(content) > MAX_DOWNLOAD_SIZE:
                        errors.append(
                            f"{filename}: 下载文件过大 ({len(content)} bytes)，"
                            f"超过 {MAX_DOWNLOAD_SIZE} bytes 限制"
                        )
                        continue
                    save_path.write_bytes(content)
            except httpx.HTTPStatusError as e:
                msg = e.response.text or e.response.reason_phrase
                try:
                    err_data = json.loads(msg)
                    msg = str(err_data.get("error") or err_data.get("message") or msg)
                except json.JSONDecodeError:
                    pass
                if e.response.status_code == 401:
                    errors.append(f"{filename}: 认证失败 (401): API Key 无效")
                elif e.response.status_code == 403:
                    errors.append(f"{filename}: 权限不足 (403): 无法下载该文件")
                else:
                    errors.append(f"{filename}: HTTP {e.response.status_code}: {msg}")
                continue
            except Exception as e:
                errors.append(f"{filename}: 下载失败: {e}")
                continue

            downloaded.append({"filename": filename, "path": str(save_path)})

            # 自动解压 zip
            if filename.lower().endswith(".zip"):
                extract_dir = download_dir / save_path.stem
                try:
                    _safe_extract_zip(save_path, extract_dir)
                    extracted_dirs.append(str(extract_dir))
                    # 解压成功后删除原 zip
                    try:
                        save_path.unlink()
                    except OSError:
                        pass
                except OpenAaaSError as e:
                    errors.append(f"{filename} 解压失败: {e}")

        if not downloaded:
            return f"❌ 所有文件下载失败: {'; '.join(errors)}"

        lines = [
            f"✅ 结果下载成功！",
            f"任务 ID: {task_id}",
            f"文件夹路径: {download_dir}",
            f"成功下载: {len(downloaded)} 个文件",
        ]

        lines.append("文件列表（请使用下方列出的完整路径，不要推测子目录）:")
        for item in downloaded:
            lines.append(f"  - {item['filename']}: {item['path']}")

        if extracted_dirs:
            lines.append(f"自动解压目录: {len(extracted_dirs)} 个")
            for d in extracted_dirs:
                try:
                    files_in_dir = os.listdir(d)
                    lines.append(f"  - {d}: {', '.join(files_in_dir)}")
                except OSError:
                    lines.append(f"  - {d}")

        if errors:
            lines.append(f"失败项 ({len(errors)} 个):")
            for err in errors:
                lines.append(f"  - {err}")

        lines.append("\n💡 提示: 同一任务多次下载会覆盖到同一目录；读取文件时请使用上面列出的完整路径。")
        return "\n".join(lines)

    # ------------------------------------------------------------------
    # 12. list_servers
    # ------------------------------------------------------------------
    @mcp.tool()
    def list_servers() -> str:
        """列出所有已配置的服务器"""
        config = load_config()
        servers = config.get("servers", {})
        default_server = config.get("default_server", "default")

        if not servers:
            return "尚未配置任何服务器"

        lines = [f"已配置 {len(servers)} 个服务器："]
        for alias, sc in servers.items():
            is_default = alias == default_server
            has_key = bool(sc.get("api_key"))
            marker = "★ " if is_default else "  "
            reg = " (已注册)" if has_key else " (未注册)"
            lines.append(f"{marker}{alias}: {sc.get('server_url', 'N/A')}{reg}")

        return "\n".join(lines)

    # ------------------------------------------------------------------
    # 13. set_default_server
    # ------------------------------------------------------------------
    @mcp.tool()
    def set_default_server(server: str) -> str:
        """切换默认服务器"""
        alias = server.strip()
        if not alias:
            return "❌ 缺少必填参数: server"

        config = load_config()
        if alias not in config.get("servers", {}):
            available = ", ".join(config.get("servers", {}).keys()) or "无"
            return f'❌ 服务器别名 "{alias}" 不存在。可用服务器: {available}'

        config["default_server"] = alias
        save_config(config)
        return f"✅ 默认服务器已切换为: {alias}"

    # ------------------------------------------------------------------
    # 14. remove_server
    # ------------------------------------------------------------------
    @mcp.tool()
    def remove_server(server: str) -> str:
        """删除指定服务器的配置（不能删除默认服务器）"""
        alias = server.strip()
        if not alias:
            return "❌ 缺少必填参数: server（要删除的服务器别名）"

        config = load_config()
        servers = config.get("servers", {})

        if alias not in servers:
            return f'❌ 服务器别名 "{alias}" 不存在'

        if config.get("default_server") == alias:
            return (
                f'❌ 不能删除默认服务器 "{alias}"，'
                "请先使用 set_default_server 切换默认服务器"
            )

        del servers[alias]
        save_config(config)
        return f'✅ 服务器 "{alias}" 已删除'
