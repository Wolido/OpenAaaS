"""配置管理模块：原子读写、多服务器配置管理"""

import json
import os
from pathlib import Path
from typing import Any

DEFAULT_CONFIG: dict[str, Any] = {
    "servers": {
        "default": {
            "server_url": "https://api.open-aaas.com",
            "api_key": "",
            "client_id": "",
            "name": "",
        }
    },
    "default_server": "default",
}


def get_config_dir() -> Path:
    """获取配置目录：~/.openaaas-mcp-adapter/"""
    config_dir = Path.home() / ".openaaas-mcp-adapter"
    config_dir.mkdir(parents=True, exist_ok=True)
    return config_dir


def get_config_path() -> Path:
    """获取配置文件路径"""
    return get_config_dir() / "config.json"


def strip_trailing_slash(url: str) -> str:
    """去除 URL 末尾的斜杠"""
    stripped = url.rstrip("/")
    # 保留协议根路径，如 http:// 不变为 http:
    if stripped.endswith(":") and stripped.lower().startswith("http"):
        return url
    return stripped


def load_config() -> dict[str, Any]:
    """加载配置文件，若不存在或格式错误则返回默认配置"""
    config_path = get_config_path()
    if not config_path.exists():
        return _deep_copy(DEFAULT_CONFIG)

    try:
        with open(config_path, "r", encoding="utf-8") as f:
            raw = f.read()
    except OSError as e:
        raise RuntimeError(f"无法读取配置文件: {e}")

    try:
        parsed = json.loads(raw)
    except json.JSONDecodeError:
        raise RuntimeError("配置文件 JSON 格式错误，请检查 ~/.openaaas-mcp-adapter/config.json")

    if not isinstance(parsed, dict):
        raise RuntimeError("配置文件格式错误：期望 JSON 对象")

    # 兼容旧格式（单服务器）
    if "servers" not in parsed and "server_url" in parsed:
        parsed = {
            "servers": {
                "default": {
                    "server_url": parsed.get("server_url", "https://api.open-aaas.com"),
                    "api_key": parsed.get("api_key", ""),
                    "client_id": parsed.get("client_id", ""),
                    "name": parsed.get("name", ""),
                }
            },
            "default_server": parsed.get("default_server", "default"),
        }
        save_config(parsed)

    if "servers" not in parsed:
        parsed["servers"] = _deep_copy(DEFAULT_CONFIG["servers"])
    if "default_server" not in parsed:
        parsed["default_server"] = "default"

    return parsed


def save_config(config: dict[str, Any]) -> None:
    """原子写入配置文件"""
    config_path = get_config_path()
    config_path.parent.mkdir(parents=True, exist_ok=True)
    tmp_path = config_path.with_suffix(f".tmp.{os.getpid()}")

    try:
        with open(tmp_path, "w", encoding="utf-8") as f:
            json.dump(config, f, ensure_ascii=False, indent=2)
            f.flush()
            os.fsync(f.fileno())
        tmp_path.replace(config_path)
    except OSError as e:
        try:
            tmp_path.unlink(missing_ok=True)
        except OSError:
            pass
        raise RuntimeError(f"无法保存配置文件: {e}")


def get_server_config(alias: str | None = None) -> dict[str, Any]:
    """获取指定服务器的配置，不传则取 default_server"""
    config = load_config()
    target = alias or config.get("default_server", "default")
    servers = config.get("servers", {})
    if target not in servers:
        available = ", ".join(servers.keys()) or "无"
        raise RuntimeError(f'服务器别名 "{target}" 不存在。可用服务器: {available}')
    return {"alias": target, **servers[target]}


def require_api_key(alias: str | None = None) -> str:
    """获取指定服务器的 api_key，不存在则报错"""
    sc = get_server_config(alias)
    api_key = sc.get("api_key", "")
    if not api_key:
        raise RuntimeError(
            f'服务器 "{sc["alias"]}" 缺少 API Key，请先运行 register 进行注册'
        )
    return api_key


def _deep_copy(obj: Any) -> Any:
    """深拷贝简单数据结构"""
    return json.loads(json.dumps(obj))
