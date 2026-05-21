"""Tests for tools.py MCP tool functions."""

import json
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

from openaaas_mcp_adapter.http_client import OpenAaaSError
from openaaas_mcp_adapter.tools import register_tools


class MockMCP:
    """Mock FastMCP that captures registered tools."""

    def __init__(self):
        self.tools = {}

    def tool(self):
        def decorator(f):
            self.tools[f.__name__] = f
            return f

        return decorator


@pytest.fixture
def mock_mcp():
    return MockMCP()


@pytest.fixture
def tools(mock_mcp):
    register_tools(mock_mcp)
    return mock_mcp.tools


@pytest.fixture
def default_config():
    return {
        "servers": {
            "default": {
                "server_url": "https://api.example.com",
                "api_key": "test_key",
                "client_id": "test_client",
                "name": "test_user",
            }
        },
        "default_server": "default",
    }


class TestDiscover:
    """Tests for discover tool."""

    def test_success(self, tools):
        with patch(
            "openaaas_mcp_adapter.tools.safe_request",
            return_value={
                "api": {"version": "1.0.0", "base_url": "https://api.example.com"},
                "endpoints": [{"name": "tasks", "method": "GET", "path": "/tasks"}],
                "services": [{"name": "agent"}],
            },
        ):
            result = tools["discover"]("https://api.example.com")
        assert "成功获取" in result
        assert "1.0.0" in result

    def test_failure(self, tools):
        def raise_error(*args, **kwargs):
            raise OpenAaaSError("connection failed")

        with patch("openaaas_mcp_adapter.tools.safe_request", side_effect=raise_error):
            result = tools["discover"]("https://api.example.com")
        assert "发现失败" in result


class TestSetServerUrl:
    """Tests for set_server_url tool."""

    def test_valid_url(self, tools, tmp_path, monkeypatch):
        monkeypatch.setattr(Path, "home", lambda: tmp_path)
        with patch("openaaas_mcp_adapter.tools.load_config", return_value={
            "servers": {}, "default_server": "default"
        }):
            with patch("openaaas_mcp_adapter.tools.save_config") as mock_save:
                result = tools["set_server_url"]("https://new.example.com")
        assert "设置成功" in result
        mock_save.assert_called_once()

    def test_invalid_url(self, tools):
        result = tools["set_server_url"]("ftp://invalid.com")
        assert "参数错误" in result

    def test_empty_url(self, tools):
        result = tools["set_server_url"]("  ")
        assert "缺少必填参数" in result

    def test_existing_api_key_rejects_change(self, tools):
        config = {
            "servers": {
                "default": {
                    "server_url": "https://old.example.com",
                    "api_key": "existing_key",
                    "client_id": "",
                    "name": "",
                }
            },
            "default_server": "default",
        }
        with patch("openaaas_mcp_adapter.tools.load_config", return_value=config):
            result = tools["set_server_url"]("https://new.example.com")
        assert "已有注册信息" in result

    def test_set_server_url_uses_configured_default_server(self, tools, tmp_path, monkeypatch):
        """当 default_server 为自定义 alias 且调用 set_server_url 时不传 server 参数，应写入该 alias"""
        monkeypatch.setattr(Path, "home", lambda: tmp_path)
        config = {
            "servers": {
                "default": {"server_url": "https://a.com", "api_key": "", "client_id": "", "name": ""},
                "prod": {"server_url": "", "api_key": "", "client_id": "", "name": ""},
            },
            "default_server": "prod",
        }
        with patch("openaaas_mcp_adapter.tools.load_config", return_value=config):
            with patch("openaaas_mcp_adapter.tools.save_config") as mock_save:
                result = tools["set_server_url"]("https://new.example.com")
        assert "设置成功" in result
        # 验证 save_config 被调用时，prod 的 server_url 被更新
        saved = mock_save.call_args[0][0]
        assert saved["servers"]["prod"]["server_url"] == "https://new.example.com"


class TestRegister:
    """Tests for register tool."""

    def test_success(self, tools, default_config):
        with patch("openaaas_mcp_adapter.tools.load_config", return_value=default_config):
            with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
                "alias": "default",
                "server_url": "https://api.example.com",
                "api_key": "",
                "client_id": "",
                "name": "",
            }):
                with patch("openaaas_mcp_adapter.tools.safe_request", return_value={
                    "api_key": "new_key",
                    "client_id": "new_client",
                }):
                    with patch("openaaas_mcp_adapter.tools.save_config"):
                        result = tools["register"]("testuser")
        assert "注册成功" in result

    def test_already_registered(self, tools, default_config):
        config = default_config.copy()
        config["servers"]["default"]["api_key"] = "existing"
        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "existing",
            "client_id": "cid",
            "name": "uname",
        }):
            result = tools["register"]("testuser")
        assert "已注册" in result

    def test_empty_name(self, tools):
        result = tools["register"]("  ")
        assert "缺少必填参数" in result

    def test_too_long_name(self, tools):
        result = tools["register"]("x" * 65)
        assert "长度不能超过 64" in result

    def test_illegal_chars(self, tools):
        result = tools["register"]("name/with/slash")
        assert "非法字符" in result


class TestListServices:
    """Tests for list_services tool."""

    def test_success(self, tools):
        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "key",
        }):
            with patch("openaaas_mcp_adapter.tools.require_api_key", return_value="key"):
                with patch("openaaas_mcp_adapter.tools.safe_request", return_value={
                    "services": [
                        {"id": "svc1", "name": "Agent", "agent_status": "online", "access_type": "public", "has_permission": True, "description": "desc"}
                    ]
                }):
                    result = tools["list_services"]()
        assert "Agent" in result
        assert "online" in result

    def test_no_services(self, tools):
        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "key",
        }):
            with patch("openaaas_mcp_adapter.tools.require_api_key", return_value="key"):
                with patch("openaaas_mcp_adapter.tools.safe_request", return_value={"services": []}):
                    result = tools["list_services"]()
        assert "暂无" in result


class TestGetServiceUsage:
    """Tests for get_service_usage tool."""

    def test_success(self, tools):
        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "key",
        }):
            with patch("openaaas_mcp_adapter.tools.require_api_key", return_value="key"):
                with patch("openaaas_mcp_adapter.tools.safe_request", return_value={
                    "name": "Agent",
                    "usage": "Use this agent for coding tasks.",
                }):
                    result = tools["get_service_usage"]("svc1")
        assert "Agent" in result
        assert "coding tasks" in result

    def test_empty_service_id(self, tools):
        result = tools["get_service_usage"]("")
        assert "缺少必填参数" in result


class TestSubmitTask:
    """Tests for submit_task tool."""

    def test_success(self, tools):
        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "key",
        }):
            with patch("openaaas_mcp_adapter.tools.require_api_key", return_value="key"):
                with patch("openaaas_mcp_adapter.tools.safe_request", return_value={
                    "id": "task-123",
                    "status": "pending",
                }):
                    result = tools["submit_task"]("svc1", "do something")
        assert "提交成功" in result
        assert "task-123" in result

    def test_missing_params(self, tools):
        result = tools["submit_task"]("", "")
        assert "缺少必填参数" in result

    def test_file_upload_success(self, tools, tmp_path, monkeypatch):
        monkeypatch.chdir(tmp_path)
        test_file = tmp_path / "upload.txt"
        test_file.write_text("content")

        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "key",
        }):
            with patch("openaaas_mcp_adapter.tools.require_api_key", return_value="key"):
                with patch("openaaas_mcp_adapter.tools.safe_request", return_value={
                    "id": "task-124",
                    "status": "pending",
                }) as mock_req:
                    result = tools["submit_task"]("svc1", "do something", input_files=[str(test_file)])
        assert "提交成功" in result


class TestGetTask:
    """Tests for get_task tool."""

    def test_completed(self, tools):
        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "key",
        }):
            with patch("openaaas_mcp_adapter.tools.require_api_key", return_value="key"):
                with patch("openaaas_mcp_adapter.tools.safe_request", return_value={
                    "id": "task-1",
                    "status": "completed",
                    "started_at": "2024-01-01T12:00:00Z",
                    "completed_at": "2024-01-01T12:01:00Z",
                    "result": {"summary": "Done"},
                }):
                    result = tools["get_task"]("task-1")
        assert "已完成" in result
        assert "Done" in result

    def test_failed(self, tools):
        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "key",
        }):
            with patch("openaaas_mcp_adapter.tools.require_api_key", return_value="key"):
                with patch("openaaas_mcp_adapter.tools.safe_request", return_value={
                    "id": "task-1",
                    "status": "failed",
                    "started_at": "2024-01-01T12:00:00Z",
                    "result": {"error": "something broke"},
                }):
                    result = tools["get_task"]("task-1")
        assert "失败" in result
        assert "something broke" in result

    def test_running(self, tools):
        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "key",
        }):
            with patch("openaaas_mcp_adapter.tools.require_api_key", return_value="key"):
                with patch("openaaas_mcp_adapter.tools.safe_request", return_value={
                    "id": "task-1",
                    "status": "running",
                    "started_at": "2024-01-01T12:00:00Z",
                }):
                    result = tools["get_task"]("task-1")
        assert "执行中" in result

    def test_pending(self, tools):
        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "key",
        }):
            with patch("openaaas_mcp_adapter.tools.require_api_key", return_value="key"):
                with patch("openaaas_mcp_adapter.tools.safe_request", return_value={
                    "id": "task-1",
                    "status": "pending",
                }):
                    result = tools["get_task"]("task-1")
        assert "等待中" in result


class TestCancelTask:
    """Tests for cancel_task tool."""

    def test_success_cancelled(self, tools):
        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "key",
        }):
            with patch("openaaas_mcp_adapter.tools.require_api_key", return_value="key"):
                with patch("openaaas_mcp_adapter.tools.safe_request", return_value={
                    "status": "cancelled",
                }):
                    result = tools["cancel_task"]("task-1")
        assert "已取消" in result

    def test_cancelling(self, tools):
        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "key",
        }):
            with patch("openaaas_mcp_adapter.tools.require_api_key", return_value="key"):
                with patch("openaaas_mcp_adapter.tools.safe_request", return_value={
                    "status": "cancelling",
                }):
                    result = tools["cancel_task"]("task-1")
        assert "取消中" in result


class TestListFiles:
    """Tests for list_files tool."""

    def test_has_files(self, tools):
        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "key",
        }):
            with patch("openaaas_mcp_adapter.tools.require_api_key", return_value="key"):
                with patch("openaaas_mcp_adapter.tools.safe_request", return_value={
                    "files": [{"filename": "result.zip", "id": "f1", "size": 1024}]
                }):
                    result = tools["list_files"]("task-1")
        assert "result.zip" in result

    def test_no_files(self, tools):
        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "key",
        }):
            with patch("openaaas_mcp_adapter.tools.require_api_key", return_value="key"):
                with patch("openaaas_mcp_adapter.tools.safe_request", return_value={"files": []}):
                    result = tools["list_files"]("task-1")
        assert "没有结果文件" in result


class TestListServers:
    """Tests for list_servers tool."""

    def test_has_servers(self, tools):
        with patch("openaaas_mcp_adapter.tools.load_config", return_value={
            "servers": {
                "default": {"server_url": "https://a.com", "api_key": "k1"},
                "prod": {"server_url": "https://b.com", "api_key": ""},
            },
            "default_server": "default",
        }):
            result = tools["list_servers"]()
        assert "default" in result
        assert "prod" in result

    def test_no_servers(self, tools):
        with patch("openaaas_mcp_adapter.tools.load_config", return_value={
            "servers": {},
            "default_server": "default",
        }):
            result = tools["list_servers"]()
        assert "尚未配置" in result


class TestSetDefaultServer:
    """Tests for set_default_server tool."""

    def test_success(self, tools):
        with patch("openaaas_mcp_adapter.tools.load_config", return_value={
            "servers": {"default": {}, "prod": {}},
            "default_server": "default",
        }):
            with patch("openaaas_mcp_adapter.tools.save_config") as mock_save:
                result = tools["set_default_server"]("prod")
        assert "已切换" in result
        mock_save.assert_called_once()

    def test_alias_not_found(self, tools):
        with patch("openaaas_mcp_adapter.tools.load_config", return_value={
            "servers": {"default": {}},
            "default_server": "default",
        }):
            result = tools["set_default_server"]("missing")
        assert "不存在" in result


class TestRemoveServer:
    """Tests for remove_server tool."""

    def test_success(self, tools):
        with patch("openaaas_mcp_adapter.tools.load_config", return_value={
            "servers": {"default": {}, "prod": {}},
            "default_server": "default",
        }):
            with patch("openaaas_mcp_adapter.tools.save_config") as mock_save:
                result = tools["remove_server"]("prod")
        assert "已删除" in result
        mock_save.assert_called_once()

    def test_cannot_remove_default(self, tools):
        with patch("openaaas_mcp_adapter.tools.load_config", return_value={
            "servers": {"default": {}, "prod": {}},
            "default_server": "default",
        }):
            result = tools["remove_server"]("default")
        assert "不能删除默认服务器" in result

    def test_alias_not_found(self, tools):
        with patch("openaaas_mcp_adapter.tools.load_config", return_value={
            "servers": {"default": {}},
            "default_server": "default",
        }):
            result = tools["remove_server"]("missing")
        assert "不存在" in result
