"""Tests for synchronous Client."""

from __future__ import annotations

import json
import time
from pathlib import Path
from typing import Any
from unittest.mock import patch

import httpx
import pytest
import respx
from respx import MockRouter

from pyopenaaas.client.sync import Client
from pyopenaaas.config import Config
from pyopenaaas.exceptions import (
    AuthenticationError,
    NotFoundError,
    RequestTimeoutError,
    RequestValidationError,
)
from pyopenaaas.models import ResultFile, ServerInfo, Service, ServiceUsage, Task


@pytest.fixture
def client() -> Client:
    return Client(server_url="https://api.example.com", api_key="test-key")


@pytest.fixture
def base_url() -> str:
    return "https://api.example.com"


class TestContextManager:
    """Context manager tests."""

    def test_enter_returns_self(self) -> None:
        client = Client(server_url="https://x.com", api_key="k")
        with client as c:
            assert c is client

    def test_exit_does_not_suppress(self) -> None:
        client = Client(server_url="https://x.com", api_key="k")
        with pytest.raises(ValueError):
            with client:
                raise ValueError("boom")


class TestDiscover:
    """discover() tests."""

    @respx.mock
    def test_returns_server_info(self, client: Client, base_url: str) -> None:
        route = respx.get(f"{base_url}/api/v1/discovery").mock(
            return_value=httpx.Response(
                200,
                json={
                    "api": {"version": "1.0", "base_url": base_url},
                    "authentication": "Bearer",
                    "endpoints": [{"path": "/v1"}],
                    "services": [{"id": "s1"}],
                },
            )
        )
        info = client.discover()
        assert isinstance(info, ServerInfo)
        assert info.version == "1.0"
        assert route.called


class TestRegister:
    """register() tests."""

    @respx.mock
    def test_success_saves_api_key(self, client: Client, base_url: str) -> None:
        route = respx.post(f"{base_url}/api/v1/client/auth/register").mock(
            return_value=httpx.Response(
                200,
                json={
                    "api_key": "new-key",
                    "client_id": "c-1",
                    "name": "Alice",
                },
            )
        )
        result = client.register("Alice")
        assert result["api_key"] == "new-key"
        assert client._config.api_key == "new-key"
        assert client._config.client_id == "c-1"
        assert client._config.name == "Alice"
        assert route.called
        assert json.loads(route.calls.last.request.content) == {"name": "Alice"}

    @respx.mock
    def test_token_fallback(self, client: Client, base_url: str) -> None:
        respx.post(f"{base_url}/api/v1/client/auth/register").mock(
            return_value=httpx.Response(
                200,
                json={"token": "tok-1", "id": "c-1"},
            )
        )
        client.register("Bob")
        assert client._config.api_key == "tok-1"
        assert client._config.client_id == "c-1"

    def test_invalid_name_raises(self, client: Client) -> None:
        with pytest.raises(RequestValidationError):
            client.register("a" * 65)

    @respx.mock
    def test_register_without_name(self, client: Client, base_url: str) -> None:
        route = respx.post(f"{base_url}/api/v1/client/auth/register").mock(
            return_value=httpx.Response(
                200,
                json={
                    "api_key": "new-key",
                    "client_id": "c-1",
                    "name": "placeholder",
                },
            )
        )
        result = client.register()
        assert result["api_key"] == "new-key"
        assert client._config.api_key == "new-key"
        assert client._config.name.startswith("user-")
        assert route.called
        sent = json.loads(route.calls.last.request.content)
        assert sent["name"].startswith("user-")


class TestUpdateProfile:
    """update_profile() tests."""

    @respx.mock
    def test_success(self, client: Client, base_url: str) -> None:
        route = respx.put(f"{base_url}/api/v1/client/profile").mock(
            return_value=httpx.Response(200, json={"name": "Alice"})
        )
        result = client.update_profile("Alice")
        assert result["name"] == "Alice"
        assert client._config.name == "Alice"
        assert route.called

    def test_invalid_name_raises(self, client: Client) -> None:
        with pytest.raises(RequestValidationError):
            client.update_profile("")


class TestListServices:
    """list_services() tests."""

    @respx.mock
    def test_returns_service_list(self, client: Client, base_url: str) -> None:
        route = respx.get(f"{base_url}/api/v1/client/services").mock(
            return_value=httpx.Response(
                200,
                json={
                    "services": [
                        {"id": "s1", "name": "Agent A", "agent_status": "running"}
                    ]
                },
            )
        )
        services = client.list_services()
        assert len(services) == 1
        assert services[0].id == "s1"
        assert route.called

    @respx.mock
    def test_list_input_also_works(self, client: Client, base_url: str) -> None:
        respx.get(f"{base_url}/api/v1/client/services").mock(
            return_value=httpx.Response(
                200,
                json=[{"id": "s1", "name": "Agent A"}],
            )
        )
        services = client.list_services()
        assert len(services) == 1


class TestGetServiceUsage:
    """get_service_usage() tests."""

    @respx.mock
    def test_returns_usage(self, client: Client, base_url: str) -> None:
        route = respx.get(
            f"{base_url}/api/v1/client/services/s1/usage"
        ).mock(
            return_value=httpx.Response(
                200, json={"name": "Agent A", "usage": "Do this"}
            )
        )
        usage = client.get_service_usage("s1")
        assert isinstance(usage, ServiceUsage)
        assert usage.name == "Agent A"
        assert route.called

    def test_empty_service_id_raises(self, client: Client) -> None:
        with pytest.raises(RequestValidationError):
            client.get_service_usage("")

    def test_whitespace_only_service_id_raises(self, client: Client) -> None:
        with pytest.raises(RequestValidationError):
            client.get_service_usage("   ")


class TestSubmitTask:
    """submit_task() tests."""

    @respx.mock
    def test_without_files(self, client: Client, base_url: str) -> None:
        route = respx.post(f"{base_url}/api/v1/client/tasks").mock(
            return_value=httpx.Response(
                200, json={"task_id": "t-1", "status": "pending"}
            )
        )
        task = client.submit_task("svc-1", "Compute something")
        assert isinstance(task, Task)
        assert task.id == "t-1"
        assert task.status == "pending"
        assert route.called

    @respx.mock
    def test_with_files(self, client: Client, base_url: str, tmp_path: Path, monkeypatch: Any) -> None:
        monkeypatch.chdir(tmp_path)
        file1 = tmp_path / "data.txt"
        file1.write_text("hello")
        route = respx.post(f"{base_url}/api/v1/client/tasks").mock(
            return_value=httpx.Response(
                200, json={"task_id": "t-1", "status": "pending"}
            )
        )
        task = client.submit_task(
            "svc-1", "Compute", input_files=["data.txt"], session_id="sess-1"
        )
        assert task.id == "t-1"
        assert route.called
        # Check multipart body contains the file
        content = route.calls.last.request.content
        assert b"data.txt" in content
        assert b"hello" in content

    def test_empty_prompt_raises(self, client: Client) -> None:
        with pytest.raises(RequestValidationError):
            client.submit_task("svc-1", "")

    def test_too_many_files_raises(self, client: Client) -> None:
        with pytest.raises(RequestValidationError) as exc_info:
            client.submit_task("svc-1", "prompt", input_files=["f"] * 11)
        assert "At most 10 files" in str(exc_info.value)

    def test_empty_service_id_raises(self, client: Client) -> None:
        with pytest.raises(RequestValidationError):
            client.submit_task("", "prompt")

    def test_whitespace_only_service_id_raises(self, client: Client) -> None:
        with pytest.raises(RequestValidationError):
            client.submit_task("   ", "prompt")


class TestGetTask:
    """get_task() tests."""

    @respx.mock
    def test_returns_task(self, client: Client, base_url: str) -> None:
        route = respx.get(
            f"{base_url}/api/v1/client/tasks/t-1"
        ).mock(
            return_value=httpx.Response(
                200, json={"task_id": "t-1", "status": "running"}
            )
        )
        task = client.get_task("t-1")
        assert task.id == "t-1"
        assert task.status == "running"
        assert route.called

    @respx.mock
    def test_various_statuses(self, client: Client, base_url: str) -> None:
        for status in ("pending", "running", "completed", "failed", "cancelled"):
            respx.get(f"{base_url}/api/v1/client/tasks/{status}").mock(
                return_value=httpx.Response(
                    200, json={"task_id": status, "status": status}
                )
            )
            task = client.get_task(status)
            assert task.status == status

    def test_empty_task_id_raises(self, client: Client) -> None:
        with pytest.raises(RequestValidationError):
            client.get_task("")


class TestCancelTask:
    """cancel_task() tests."""

    @respx.mock
    def test_returns_cancelled_task(self, client: Client, base_url: str) -> None:
        route = respx.post(
            f"{base_url}/api/v1/client/tasks/t-1/cancel"
        ).mock(
            return_value=httpx.Response(
                200, json={"task_id": "t-1", "status": "cancelled"}
            )
        )
        task = client.cancel_task("t-1")
        assert task.status == "cancelled"
        assert route.called

    def test_empty_task_id_raises(self, client: Client) -> None:
        with pytest.raises(RequestValidationError):
            client.cancel_task("")


class TestWaitForTask:
    """wait_for_task() tests."""

    @respx.mock
    def test_returns_when_done(self, client: Client, base_url: str) -> None:
        call_count = 0

        def handler(request: httpx.Request) -> httpx.Response:
            nonlocal call_count
            call_count += 1
            status = "completed" if call_count >= 2 else "running"
            return httpx.Response(
                200, json={"task_id": "t-1", "status": status}
            )

        respx.get(f"{base_url}/api/v1/client/tasks/t-1").mock(
            side_effect=handler
        )
        task = client.wait_for_task("t-1", poll_interval=0.01)
        assert task.status == "completed"
        assert call_count >= 2

    @respx.mock
    def test_callback_invoked(self, client: Client, base_url: str) -> None:
        states: list[str] = []

        def handler(request: httpx.Request) -> httpx.Response:
            status = "completed" if len(states) >= 1 else "running"
            return httpx.Response(
                200, json={"task_id": "t-1", "status": status}
            )

        respx.get(f"{base_url}/api/v1/client/tasks/t-1").mock(
            side_effect=handler
        )

        def callback(task: Task) -> None:
            states.append(task.status)

        client.wait_for_task("t-1", poll_interval=0.01, callback=callback)
        assert "running" in states
        assert "completed" in states

    @respx.mock
    def test_timeout_raises(self, client: Client, base_url: str) -> None:
        respx.get(f"{base_url}/api/v1/client/tasks/t-1").mock(
            return_value=httpx.Response(
                200, json={"task_id": "t-1", "status": "running"}
            )
        )
        with pytest.raises(RequestTimeoutError) as exc_info:
            client.wait_for_task("t-1", poll_interval=0.05, timeout=0.01)
        assert "Timed out" in str(exc_info.value)

    def test_empty_task_id_raises(self, client: Client) -> None:
        with pytest.raises(RequestValidationError):
            client.wait_for_task("")


class TestListFiles:
    """list_files() tests."""

    @respx.mock
    def test_returns_result_files(self, client: Client, base_url: str) -> None:
        route = respx.get(
            f"{base_url}/api/v1/client/files/list/t-1"
        ).mock(
            return_value=httpx.Response(
                200,
                json={
                    "files": [
                        {"file_id": "f1", "name": "a.txt", "file_size": 100}
                    ]
                },
            )
        )
        files = client.list_files("t-1")
        assert len(files) == 1
        assert files[0].id == "f1"
        assert route.called

    def test_empty_task_id_raises(self, client: Client) -> None:
        with pytest.raises(RequestValidationError):
            client.list_files("")


class TestDownloadFile:
    """download_file() tests."""

    @respx.mock
    def test_downloads_to_path(self, client: Client, base_url: str, tmp_path: Path) -> None:
        route = respx.get(
            f"{base_url}/api/v1/client/files/f-1/download"
        ).mock(
            return_value=httpx.Response(200, content=b"file content")
        )
        dest = tmp_path / "output.txt"
        result = client.download_file("f-1", save_path=dest)
        assert result == dest
        assert dest.read_bytes() == b"file content"
        assert route.called

    @respx.mock
    def test_downloads_to_directory(self, client: Client, base_url: str, tmp_path: Path) -> None:
        respx.get(f"{base_url}/api/v1/client/files/f-1/download").mock(
            return_value=httpx.Response(200, content=b"data")
        )
        dest_dir = tmp_path / "downloads"
        dest_dir.mkdir()
        result = client.download_file("f-1", save_path=dest_dir)
        assert result == dest_dir / "result.download"

    @respx.mock
    def test_auto_extract_zip(self, client: Client, base_url: str, tmp_path: Path) -> None:
        import zipfile
        zip_bytes = tmp_path / "tmp.zip"
        with zipfile.ZipFile(zip_bytes, "w") as zf:
            zf.writestr("inside.txt", "zip content")
        content = zip_bytes.read_bytes()
        respx.get(f"{base_url}/api/v1/client/files/f-1/download").mock(
            return_value=httpx.Response(200, content=content)
        )
        dest = tmp_path / "archive.zip"
        result = client.download_file("f-1", save_path=dest)
        assert result.is_dir()
        assert (result / "inside.txt").read_text() == "zip content"

    def test_empty_file_id_raises(self, client: Client) -> None:
        with pytest.raises(RequestValidationError):
            client.download_file("")


class TestDownloadAllFiles:
    """download_all_files() tests."""

    @respx.mock
    def test_downloads_all(self, client: Client, base_url: str, tmp_path: Path) -> None:
        respx.get(f"{base_url}/api/v1/client/files/list/t-1").mock(
            return_value=httpx.Response(
                200,
                json={
                    "files": [
                        {"file_id": "f1", "name": "a.txt"},
                        {"file_id": "f2", "name": "b.txt"},
                    ]
                },
            )
        )
        respx.get(f"{base_url}/api/v1/client/files/f1/download").mock(
            return_value=httpx.Response(200, content=b"content-a")
        )
        respx.get(f"{base_url}/api/v1/client/files/f2/download").mock(
            return_value=httpx.Response(200, content=b"content-b")
        )
        dest_dir = tmp_path / "out"
        paths = client.download_all_files("t-1", save_dir=dest_dir)
        assert len(paths) == 2
        assert sorted([p.name for p in paths]) == ["a.txt", "b.txt"]

    @respx.mock
    def test_empty_list_returns_empty(self, client: Client, base_url: str, tmp_path: Path) -> None:
        respx.get(f"{base_url}/api/v1/client/files/list/t-1").mock(
            return_value=httpx.Response(200, json={"files": []})
        )
        paths = client.download_all_files("t-1", save_dir=tmp_path / "out")
        assert paths == []

    @respx.mock
    def test_duplicate_names_avoid_collision(self, client: Client, base_url: str, tmp_path: Path) -> None:
        respx.get(f"{base_url}/api/v1/client/files/list/t-1").mock(
            return_value=httpx.Response(
                200,
                json={
                    "files": [
                        {"file_id": "f1", "name": "same.txt"},
                        {"file_id": "f2", "name": "same.txt"},
                    ]
                },
            )
        )
        respx.get(f"{base_url}/api/v1/client/files/f1/download").mock(
            return_value=httpx.Response(200, content=b"a")
        )
        respx.get(f"{base_url}/api/v1/client/files/f2/download").mock(
            return_value=httpx.Response(200, content=b"b")
        )
        paths = client.download_all_files("t-1", save_dir=tmp_path / "out")
        assert len(paths) == 2
        names = [p.name for p in paths]
        assert "same.txt" in names
        assert "same_1.txt" in names

    def test_empty_task_id_raises(self, client: Client) -> None:
        with pytest.raises(RequestValidationError):
            client.download_all_files("")
