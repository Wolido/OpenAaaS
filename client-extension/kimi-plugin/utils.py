#!/usr/bin/env python3
"""
OpenAaaS 插件 - 公共工具模块
提供配置管理、HTTP 请求等通用功能
"""

import base64
import gzip
import json
import os
import sys
import urllib.request
import urllib.error
import urllib.parse
import zlib

try:
    import brotli
    _has_brotli = True
except ImportError:
    _has_brotli = False


def load_config():
    """加载配置文件（支持多服务器格式并兼容旧格式，迁移自动持久化）"""
    config_path = os.path.join(os.path.dirname(__file__), "config.json")
    migrated = False
    try:
        with open(config_path, "r", encoding="utf-8") as f:
            config = json.load(f)
            if not isinstance(config, dict):
                return {
                    "error": f"config.json 格式错误: 期望 JSON 对象，实际为 {type(config).__name__}",
                    "server_url": "https://api.open-aaas.com",
                    "api_key": "",
                    "client_id": "",
                    "servers": {
                        "default": {
                            "server_url": "https://api.open-aaas.com",
                            "api_key": "",
                            "client_id": ""
                        }
                    },
                    "default_server": "default"
                }
    except Exception as e:
        return {
            "error": f"无法读取 config.json: {str(e)}",
            "server_url": "https://api.open-aaas.com",
            "api_key": "",
            "client_id": "",
            "servers": {
                "default": {
                    "server_url": "https://api.open-aaas.com",
                    "api_key": "",
                    "client_id": ""
                }
            },
            "default_server": "default"
        }

    # 兼容旧格式：单服务器配置
    if "servers" not in config and "server_url" in config:
        config = {
            "server_url": config.get("server_url", "https://api.open-aaas.com"),
            "api_key": config.get("api_key", ""),
            "client_id": config.get("client_id", ""),
            "servers": {
                "default": {
                    "server_url": config.get("server_url", "https://api.open-aaas.com"),
                    "api_key": config.get("api_key", ""),
                    "client_id": config.get("client_id", "")
                }
            },
            "default_server": config.get("default_server", "default")
        }
        migrated = True

    # 持久化迁移后的配置
    if migrated:
        if not save_config(config):
            print("警告: 配置迁移后保存失败，请检查磁盘空间或文件权限", file=sys.stderr)

    return config


def save_config(config):
    """保存配置到文件，失败时返回 False"""
    config_path = os.path.join(os.path.dirname(__file__), "config.json")
    try:
        with open(config_path, "w", encoding="utf-8") as f:
            json.dump(config, f, ensure_ascii=False, indent=2)
        return True
    except Exception:
        return False


def _decompress_body(raw_body, headers):
    """根据 Content-Encoding 解压响应体，失败时返回原始 bytes"""
    if not raw_body:
        return raw_body
    encoding = (headers or {}).get("Content-Encoding", "").lower().strip()
    if not encoding:
        return raw_body
    _excepted = (OSError, zlib.error, EOFError)
    if _has_brotli:
        _excepted = _excepted + (brotli.error,)
    try:
        if encoding == "gzip":
            return gzip.decompress(raw_body)
        elif encoding == "deflate":
            try:
                return zlib.decompress(raw_body)
            except zlib.error:
                try:
                    return zlib.decompress(raw_body, -15)
                except zlib.error:
                    return raw_body
        elif encoding == "br":
            if _has_brotli:
                return brotli.decompress(raw_body)
            else:
                # brotli not available, return raw and let caller handle
                return raw_body
    except _excepted:
        # decompression failed, return raw body for best-effort handling
        return raw_body
    return raw_body


class _NoRedirectHandler(urllib.request.HTTPRedirectHandler):
    """禁用自动重定向，让调用方手动处理 3xx"""
    def http_error_302(self, req, fp, code, msg, headers):
        return fp
    http_error_301 = http_error_303 = http_error_307 = http_error_308 = http_error_302

_no_redirect_opener = urllib.request.build_opener(_NoRedirectHandler())


def safe_request(url, headers=None, data=None, method="GET", timeout=30, max_redirects=3):
    """
    发送 HTTP 请求并安全处理响应（手动处理 3xx 重定向，保持原 HTTP 方法）
    """
    current_url = url
    remaining_redirects = max_redirects
    original_host = urllib.parse.urlparse(url).hostname

    try:
        while remaining_redirects >= 0:
            req_headers = dict(headers or {})
            current_host = urllib.parse.urlparse(current_url).hostname
            if current_host != original_host:
                req_headers.pop("Authorization", None)

            req = urllib.request.Request(current_url, data=data, headers=req_headers, method=method)
            with _no_redirect_opener.open(req, timeout=timeout) as response:
                status = response.getcode()
                if 300 <= status < 400:
                    location = response.headers.get("Location")
                    if location:
                        if remaining_redirects > 0:
                            current_url = urllib.parse.urljoin(current_url, location)
                            remaining_redirects -= 1
                            continue
                        return False, "请求被多次重定向，请直接使用 HTTPS URL", status

                raw_body = response.read()
                raw_body = _decompress_body(raw_body, response.headers)
                try:
                    body = raw_body.decode("utf-8")
                except UnicodeDecodeError:
                    truncated = raw_body[:512]
                    return False, f"响应体解码失败（非 UTF-8 文本，前 {len(truncated)} 字节 base64）: {base64.b64encode(truncated).decode('ascii')}...", status
                result = json.loads(body)
                return True, result, status

    except urllib.error.HTTPError as e:
        error_raw = e.read()
        error_raw = _decompress_body(error_raw, e.headers)
        try:
            error_body = error_raw.decode("utf-8")
        except UnicodeDecodeError:
            truncated = error_raw[:512]
            error_body = f"响应体解码失败（非 UTF-8 文本，前 {len(truncated)} 字节 base64）: {base64.b64encode(truncated).decode('ascii')}..."
        try:
            error_data = json.loads(error_body)
            error_msg = error_data.get("error") or error_data.get("message") or error_body
        except Exception:
            error_msg = error_body or e.reason
        return False, error_msg, e.code
    except urllib.error.URLError as e:
        return False, f"连接失败: {e.reason}，请检查服务端地址是否正确", None
    except json.JSONDecodeError as e:
        return False, f"JSON 解析错误: {str(e)}", None
    except Exception as e:
        return False, f"请求失败: {str(e)}", None
