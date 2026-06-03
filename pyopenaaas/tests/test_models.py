"""Tests for Pydantic v2 models."""

from __future__ import annotations

import pytest
from pydantic import ValidationError

from pyopenaaas.models import (
    ResultFile,
    ServerInfo,
    Service,
    ServiceUsage,
    Task,
    TaskResult,
)


class TestServerInfo:
    """ServerInfo model tests."""

    def test_defaults(self) -> None:
        info = ServerInfo()
        assert info.version == "unknown"
        assert info.base_url == ""
        assert info.authentication == "Bearer Token"
        assert info.endpoints == []
        assert info.services == []

    def test_full_construction(self) -> None:
        info = ServerInfo(
            version="1.0",
            base_url="https://api.example.com",
            authentication="API Key",
            endpoints=[{"path": "/v1"}],
            services=[{"id": "svc-1"}],
        )
        assert info.version == "1.0"
        assert info.base_url == "https://api.example.com"

    def test_extra_fields_allowed(self) -> None:
        info = ServerInfo(custom_field="hello")
        assert info.model_dump()["custom_field"] == "hello"

    def test_repr(self) -> None:
        info = ServerInfo(version="1.0", base_url="https://x.com")
        r = repr(info)
        assert "ServerInfo(" in r
        assert "version='1.0'" in r
        assert "base_url='https://x.com'" in r


class TestService:
    """Service model tests."""

    def test_defaults(self) -> None:
        svc = Service()
        assert svc.name == "未命名"
        assert svc.agent_status == "unknown"
        assert svc.access_type == "unknown"
        assert svc.has_permission is False
        assert svc.registration_status is None

    def test_full_construction(self) -> None:
        svc = Service(
            id="svc-1",
            name="Agent A",
            description="Does things",
            agent_status="running",
            access_type="public",
            has_permission=True,
            registration_status="approved",
        )
        assert svc.id == "svc-1"
        assert svc.name == "Agent A"

    def test_extra_fields(self) -> None:
        svc = Service(extra="value")
        assert svc.model_dump()["extra"] == "value"

    def test_repr(self) -> None:
        svc = Service(id="svc-1", name="Agent A", agent_status="running")
        r = repr(svc)
        assert "Service(" in r
        assert "id='svc-1'" in r
        assert "name='Agent A'" in r
        assert "agent_status='running'" in r


class TestServiceUsage:
    """ServiceUsage model tests."""

    def test_defaults(self) -> None:
        su = ServiceUsage()
        assert su.name == "未命名"
        assert su.usage == ""

    def test_full_construction(self) -> None:
        su = ServiceUsage(name="Agent A", usage="Use it like this")
        assert su.name == "Agent A"
        assert su.usage == "Use it like this"

    def test_repr(self) -> None:
        su = ServiceUsage(name="Agent A")
        r = repr(su)
        assert "ServiceUsage(" in r
        assert "name='Agent A'" in r


class TestResultFile:
    """ResultFile model tests."""

    def test_defaults(self) -> None:
        rf = ResultFile()
        assert rf.id == ""
        assert rf.filename == ""
        assert rf.size is None

    def test_alias_construction(self) -> None:
        """Field aliases file_id, name, file_size should work."""
        rf = ResultFile(file_id="f1", name="data.csv", file_size=1024)
        assert rf.id == "f1"
        assert rf.filename == "data.csv"
        assert rf.size == 1024

    def test_by_field_name(self) -> None:
        """Construction by real field names should also work."""
        rf = ResultFile(id="f1", filename="data.csv", size=1024)
        assert rf.id == "f1"
        assert rf.size == 1024

    def test_extra_fields(self) -> None:
        rf = ResultFile(extra="value")
        assert rf.model_dump()["extra"] == "value"

    def test_repr(self) -> None:
        rf = ResultFile(id="f1", filename="data.csv", size=1024)
        r = repr(rf)
        assert "ResultFile(" in r
        assert "id='f1'" in r
        assert "filename='data.csv'" in r
        assert "size=1024" in r


class TestTaskResult:
    """TaskResult model tests."""

    def test_defaults(self) -> None:
        tr = TaskResult()
        assert tr.files == []
        assert tr.stdout is None
        assert tr.raw == {}

    def test_full_construction(self) -> None:
        tr = TaskResult(
            files=["result.json", "plot.png"],
            stdout="Calculation completed.",
            raw={"extra_key": "extra_value"},
        )
        assert tr.files == ["result.json", "plot.png"]
        assert tr.stdout == "Calculation completed."
        assert tr.raw == {"extra_key": "extra_value"}

    def test_from_server_dict(self) -> None:
        """TaskResult should parse Server output dict with files/stdout."""
        tr = TaskResult.model_validate(
            {
                "files": ["result.json"],
                "stdout": "Done",
                "unknown_field": "captured_by_raw",
            }
        )
        assert tr.files == ["result.json"]
        assert tr.stdout == "Done"
        # Pydantic extra='allow' captures unknown fields in model_dump
        dumped = tr.model_dump()
        assert "unknown_field" in dumped

    def test_repr(self) -> None:
        tr = TaskResult(files=["a.txt"], stdout="ok")
        r = repr(tr)
        assert "TaskResult(" in r
        assert "files=['a.txt']" in r
        assert "stdout='ok'" in r


class TestTask:
    """Task model tests."""

    def test_defaults(self) -> None:
        t = Task()
        assert t.id == ""
        assert t.status == "unknown"
        assert t.service_id == ""
        assert t.task_prompt == ""
        assert t.result is None

    def test_alias_construction(self) -> None:
        t = Task(task_id="t-123")
        assert t.id == "t-123"

    def test_full_construction(self) -> None:
        t = Task(
            id="t-123",
            status="completed",
            service_id="svc-1",
            task_prompt="Compute",
            output_prompt="JSON",
            session_id="sess-1",
            started_at="2024-01-01T00:00:00Z",
            completed_at="2024-01-01T00:01:00Z",
            created_at="2024-01-01T00:00:00Z",
            result=TaskResult(files=["result.json"], stdout="Done"),
        )
        assert t.status == "completed"
        assert t.result is not None
        assert t.result.files == ["result.json"]
        assert t.result.stdout == "Done"

    def test_result_alias_from_output(self) -> None:
        """Server returns 'output' instead of 'result'; alias should map it."""
        t = Task(
            id="t-456",
            status="completed",
            output={"files": ["result.json"], "stdout": "Done"},
        )
        assert t.result is not None
        assert t.result.files == ["result.json"]
        assert t.result.stdout == "Done"

    def test_result_by_field_name(self) -> None:
        """Construction by real field name 'result' should still work."""
        t = Task(
            id="t-789",
            status="completed",
            result=TaskResult(files=["a.txt"]),
        )
        assert t.result is not None
        assert t.result.files == ["a.txt"]

    def test_extra_fields(self) -> None:
        t = Task(unknown="value")
        assert t.model_dump()["unknown"] == "value"

    def test_repr(self) -> None:
        t = Task(id="t-123", status="running")
        r = repr(t)
        assert "Task(" in r
        assert "id='t-123'" in r
        assert "status='running'" in r

    # --- is_done ---

    @pytest.mark.parametrize(
        "status, expected",
        [
            ("pending", False),
            ("running", False),
            ("completed", True),
            ("failed", True),
            ("cancelled", True),
            ("unknown", False),
        ],
    )
    def test_is_done(self, status: str, expected: bool) -> None:
        t = Task(status=status)
        assert t.is_done() is expected

    # --- is_success ---

    @pytest.mark.parametrize(
        "status, expected",
        [
            ("pending", False),
            ("running", False),
            ("completed", True),
            ("failed", False),
            ("cancelled", False),
            ("unknown", False),
        ],
    )
    def test_is_success(self, status: str, expected: bool) -> None:
        t = Task(status=status)
        assert t.is_success() is expected

    # --- duration_seconds ---

    def test_duration_seconds_with_start_and_end(self) -> None:
        t = Task(
            started_at="2024-01-01T00:00:00Z",
            completed_at="2024-01-01T00:01:30Z",
        )
        assert t.duration_seconds == 90.0

    def test_duration_seconds_fallback_to_created_at(self) -> None:
        t = Task(
            created_at="2024-01-01T00:00:00Z",
            completed_at="2024-01-01T00:00:05Z",
        )
        assert t.duration_seconds == 5.0

    def test_duration_seconds_missing_end(self) -> None:
        t = Task(started_at="2024-01-01T00:00:00Z")
        assert t.duration_seconds is None

    def test_duration_seconds_missing_start(self) -> None:
        t = Task(completed_at="2024-01-01T00:00:00Z")
        assert t.duration_seconds is None
