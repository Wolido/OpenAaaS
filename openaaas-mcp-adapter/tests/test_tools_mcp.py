"""Tests for tools.py MCP tool functions."""

from pathlib import Path
from unittest.mock import patch

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


def _progress_header(text: str) -> str:
    """定位进度块首行（输出首行是「任务状态: …」，进度标题行以「进度」开头）。"""
    for line in text.splitlines():
        if line.strip().startswith("进度"):
            return line.strip()
    return ""


def _progress_block(text: str) -> str:
    """取进度块（进度标题行 + 其后正文）；找不到标题行时返回空串。"""
    lines = text.splitlines()
    for index, line in enumerate(lines):
        if line.strip().startswith("进度"):
            return "\n".join(lines[index:]).strip()
    return ""


def _assert_no_deprecated_stale_wording_and_no_source(text: str) -> None:
    """协议已删 source 字段，且 stale 文案改为「可能不是最新」。"""
    assert "可能已过期" not in text, (
        f"输出不得出现「可能已过期」（stale 文案应为「可能不是最新」）: {text!r}"
    )
    assert "source" not in text, (
        f"输出不得出现「source」字样（该字段已从协议删除）: {text!r}"
    )


def _assert_no_facts_word(text: str) -> None:
    """新协议已删除 facts：任何情况下输出都不得出现该字样。"""
    assert "facts" not in text, f"输出不得出现「facts」字样（该字段已从协议删除）: {text!r}"


def _assert_no_time_words_or_placeholders(text: str) -> None:
    """时间维度已从协议删除：不得出现时间字样与 None/undefined 占位。"""
    for forbidden in ("已跑", "空闲", "秒前", "小时", "None", "undefined", "NaN"):
        assert forbidden not in text, f"输出不应含「{forbidden}」: {text!r}"


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

    def test_running_with_live_progress(self, tools):
        """非 stale：进度块首行严格为「进度：」，其后只展示 text。"""
        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "key",
        }):
            with patch("openaaas_mcp_adapter.tools.require_api_key", return_value="key"):
                with patch("openaaas_mcp_adapter.tools.safe_request", return_value={
                    "id": "task-1",
                    "status": "running",
                    "live": {
                        "text": (
                            "phase=step1 开始解析\n"
                            "note=已解析完 12 个 CSV"
                        ),
                        "stale": False,
                        "source": "live",
                        "truncated": False,
                    },
                }):
                    result = tools["get_task"]("task-1")
        first_line = _progress_header(result)
        assert first_line == "进度：", f"非 stale 首行应严格为「进度：」: {first_line!r}"
        # text 的多行内容原样出现
        assert "phase=step1 开始解析\nnote=已解析完 12 个 CSV" in result
        _assert_no_facts_word(result)
        _assert_no_time_words_or_placeholders(result)

    def test_running_with_live_progress_stale(self, tools):
        """stale 为 true：首行严格为「进度（可能不是最新）：」，仍展示 text。"""
        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "key",
        }):
            with patch("openaaas_mcp_adapter.tools.require_api_key", return_value="key"):
                with patch("openaaas_mcp_adapter.tools.safe_request", return_value={
                    "id": "task-1",
                    "status": "running",
                    "live": {
                        "text": (
                            "phase=step1 开始解析\n"
                            "note=已解析完 12 个 CSV"
                        ),
                        "stale": True,
                        "source": "live",
                        "truncated": False,
                    },
                }):
                    result = tools["get_task"]("task-1")
        first_line = _progress_header(result)
        assert first_line == "进度（可能不是最新）：", (
            f"stale 首行应严格为「进度（可能不是最新）：」: {first_line!r}"
        )
        assert "phase=step1 开始解析" in result
        assert "note=已解析完 12 个 CSV" in result
        _assert_no_facts_word(result)
        _assert_no_time_words_or_placeholders(result)
        # 不得出现旧文案，也不得展示负载里的 source 字段
        _assert_no_deprecated_stale_wording_and_no_source(result)

    def test_running_with_long_live_text_truncates_by_code_point(self, tools):
        """live.text 超过 800 字符：按码点截断，总长 ≤ 800，以「…」结尾，emoji 完整。"""
        long_text = "x" * 793 + "🙂🙂" + "y" * 200
        assert len(long_text) > 800, "用例前置条件：live.text 字符数应明显超过 800"
        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "key",
        }):
            with patch("openaaas_mcp_adapter.tools.require_api_key", return_value="key"):
                with patch("openaaas_mcp_adapter.tools.safe_request", return_value={
                    "id": "task-1",
                    "status": "running",
                    "live": {
                        "text": long_text,
                        "stale": False,
                        "source": "live",
                        "truncated": True,
                    },
                }):
                    result = tools["get_task"]("task-1")
        block = _progress_block(result)
        assert len(block) <= 800, (
            f"进度块应按码点截断到 ≤ 800 个字符，实际 {len(block)} 个（块尾: {block[-20:]!r}）"
        )
        assert block.endswith("…"), (
            f"截断后的进度块应以「…」结尾（块尾: {block[-20:]!r}）"
        )
        assert not any(0xD800 <= ord(ch) <= 0xDFFF for ch in block), (
            f"进度块不得出现半个代理对（块尾: {block[-20:]!r}）"
        )
        assert "🙂🙂" in block, f"emoji 应完整保留，不得被截断（块尾: {block[-20:]!r}）"
        _assert_no_facts_word(result)
        _assert_no_time_words_or_placeholders(result)

    def test_running_without_live_progress(self, tools):
        """live 缺席：输出应保持原有状态行且不含任何进度块。"""
        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "key",
        }):
            with patch("openaaas_mcp_adapter.tools.require_api_key", return_value="key"):
                with patch("openaaas_mcp_adapter.tools.safe_request", return_value={
                    "id": "task-1",
                    "status": "running",
                }):
                    result = tools["get_task"]("task-1")
        assert "任务状态: 执行中" in result
        assert "任务 ID: task-1" in result
        assert _progress_header(result) == ""
        _assert_no_facts_word(result)
        _assert_no_time_words_or_placeholders(result)

    def test_running_with_empty_live_text(self, tools):
        """live.text 为空串：不显示进度块，即使负载仍带着 facts。"""
        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "key",
        }):
            with patch("openaaas_mcp_adapter.tools.require_api_key", return_value="key"):
                with patch("openaaas_mcp_adapter.tools.safe_request", return_value={
                    "id": "task-1",
                    "status": "running",
                    "live": {
                        "text": "",
                        "stale": False,
                        "source": "live",
                        "truncated": False,
                        "facts": "阶段 step1 · 输出 3 个文件 · 容器在跑",
                    },
                }):
                    result = tools["get_task"]("task-1")
        assert _progress_header(result) == ""
        assert "阶段 step1 · 输出 3 个文件 · 容器在跑" not in result
        assert "任务状态: 执行中" in result
        _assert_no_facts_word(result)
        _assert_no_time_words_or_placeholders(result)

    def test_running_with_missing_live_text(self, tools):
        """live.text 缺省：不显示进度块，即使负载仍带着 facts。"""
        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "key",
        }):
            with patch("openaaas_mcp_adapter.tools.require_api_key", return_value="key"):
                with patch("openaaas_mcp_adapter.tools.safe_request", return_value={
                    "id": "task-1",
                    "status": "running",
                    "live": {
                        "stale": False,
                        "source": "live",
                        "truncated": False,
                        "facts": "阶段 step1 · 输出 3 个文件 · 容器在跑",
                    },
                }):
                    result = tools["get_task"]("task-1")
        assert _progress_header(result) == ""
        assert "阶段 step1 · 输出 3 个文件 · 容器在跑" not in result
        _assert_no_facts_word(result)
        _assert_no_time_words_or_placeholders(result)

    def test_running_with_live_progress_legacy_payload(self, tools):
        """旧 server 负载带 facts / updated_at 等历史字段：不报错，只展示 text。"""
        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "key",
        }):
            with patch("openaaas_mcp_adapter.tools.require_api_key", return_value="key"):
                with patch("openaaas_mcp_adapter.tools.safe_request", return_value={
                    "id": "task-1",
                    "status": "running",
                    "live": {
                        "text": (
                            "phase=step1 开始解析\n"
                            "note=已解析完 12 个 CSV"
                        ),
                        "stale": False,
                        "source": "live",
                        "truncated": False,
                        "facts": "阶段 step1 · 输出 3 个文件 · 容器在跑",
                        "updated_at": "2024-01-01T12:00:00Z",
                    },
                }):
                    result = tools["get_task"]("task-1")
        first_line = _progress_header(result)
        assert first_line == "进度：", f"旧 server 负载也应输出「进度：」首行: {first_line!r}"
        assert "note=已解析完 12 个 CSV" in result
        assert "阶段 step1 · 输出 3 个文件 · 容器在跑" not in result
        assert "2024-01-01T12:00:00Z" not in result
        _assert_no_facts_word(result)
        _assert_no_time_words_or_placeholders(result)

class TestPollTask:
    """Tests for poll_task tool."""

    def test_completed_immediately(self, tools):
        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "key",
        }):
            with patch("openaaas_mcp_adapter.tools.require_api_key", return_value="key"):
                with patch("openaaas_mcp_adapter.tools.safe_request", return_value={
                    "id": "task-1",
                    "status": "completed",
                    "result": {"summary": "Done"},
                }):
                    with patch("openaaas_mcp_adapter.tools.time.sleep") as mock_sleep:
                        result = tools["poll_task"]("task-1")
        assert "轮询结束" in result
        assert "已完成" in result
        assert "Done" in result
        mock_sleep.assert_not_called()

    def test_polls_until_completed(self, tools):
        responses = [
            {"id": "task-1", "status": "pending"},
            {"id": "task-1", "status": "running"},
            {"id": "task-1", "status": "completed", "result": {"summary": "Done"}},
        ]
        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "key",
        }):
            with patch("openaaas_mcp_adapter.tools.require_api_key", return_value="key"):
                with patch("openaaas_mcp_adapter.tools.safe_request", side_effect=responses):
                    with patch("openaaas_mcp_adapter.tools.time.sleep") as mock_sleep:
                        result = tools["poll_task"]("task-1")
        assert "轮询结束" in result
        assert "已完成" in result
        assert "Done" in result
        assert "轮询次数: 3" in result
        assert mock_sleep.call_count == 2
        mock_sleep.assert_called_with(20)

    def test_timeout(self, tools):
        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "key",
        }):
            with patch("openaaas_mcp_adapter.tools.require_api_key", return_value="key"):
                with patch("openaaas_mcp_adapter.tools.safe_request", return_value={
                    "id": "task-1",
                    "status": "running",
                }):
                    with patch("openaaas_mcp_adapter.tools.time.sleep"):
                        with patch(
                            "openaaas_mcp_adapter.tools.time.monotonic",
                            side_effect=[0.0, 5.0, 55.0],
                        ):
                            result = tools["poll_task"]("task-1", timeout_seconds=30)
        assert "轮询超时" in result
        assert "30 秒" in result
        assert "running" in result

    def test_failed_stops_polling(self, tools):
        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "key",
        }):
            with patch("openaaas_mcp_adapter.tools.require_api_key", return_value="key"):
                with patch("openaaas_mcp_adapter.tools.safe_request", return_value={
                    "id": "task-1",
                    "status": "failed",
                    "result": {"error": "boom"},
                }):
                    with patch("openaaas_mcp_adapter.tools.time.sleep") as mock_sleep:
                        result = tools["poll_task"]("task-1")
        assert "轮询结束" in result
        assert "失败" in result
        assert "boom" in result
        mock_sleep.assert_not_called()

    def test_cancelled_stops_polling(self, tools):
        with patch("openaaas_mcp_adapter.tools.get_server_config", return_value={
            "alias": "default",
            "server_url": "https://api.example.com",
            "api_key": "key",
        }):
            with patch("openaaas_mcp_adapter.tools.require_api_key", return_value="key"):
                with patch("openaaas_mcp_adapter.tools.safe_request", return_value={
                    "id": "task-1",
                    "status": "cancelled",
                }):
                    with patch("openaaas_mcp_adapter.tools.time.sleep") as mock_sleep:
                        result = tools["poll_task"]("task-1")
        assert "轮询结束" in result
        assert "已取消" in result
        mock_sleep.assert_not_called()

    def test_empty_task_id(self, tools):
        result = tools["poll_task"]("")
        assert "缺少必填参数" in result


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
