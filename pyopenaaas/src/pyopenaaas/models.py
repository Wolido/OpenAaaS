"""Pydantic models for OpenAaaS SDK."""

from __future__ import annotations

from datetime import datetime
from typing import Any, ClassVar

from pydantic import BaseModel, ConfigDict, Field

from ._utils import _parse_iso_time


class _ReprMixin:
    """Mixin for a concise __repr__ showing class name and key fields."""

    _repr_fields: ClassVar[tuple[str, ...]] = ()

    def __repr__(self) -> str:
        cls = self.__class__.__name__
        fields = self._repr_fields or tuple(self.model_fields.keys())
        parts = []
        for k in fields:
            v = getattr(self, k, None)
            if v is not None:
                parts.append(f"{k}={v!r}")
        return f"{cls}({', '.join(parts)})"


class ServerInfo(_ReprMixin, BaseModel):
    """Server discovery information."""

    model_config = ConfigDict(extra="allow", populate_by_name=True)

    version: str = "unknown"
    base_url: str = ""
    authentication: str | dict[str, Any] = "Bearer Token"
    endpoints: list[dict[str, Any]] = Field(default_factory=list)
    services: list[dict[str, Any]] = Field(default_factory=list)

    _repr_fields: ClassVar[tuple[str, ...]] = ("version", "base_url")


class Service(_ReprMixin, BaseModel):
    """An available Agent service."""

    model_config = ConfigDict(extra="allow", populate_by_name=True)

    id: str = ""
    name: str = "未命名"
    description: str = ""
    agent_status: str = "unknown"
    access_type: str = "unknown"
    has_permission: bool = False
    registration_status: str | None = None

    _repr_fields: ClassVar[tuple[str, ...]] = ("id", "name", "agent_status")


class ServiceUsage(_ReprMixin, BaseModel):
    """Detailed usage instructions for a service."""

    model_config = ConfigDict(extra="allow", populate_by_name=True)

    name: str = "未命名"
    usage: str = ""

    _repr_fields: ClassVar[tuple[str, ...]] = ("name",)


class ResultFile(_ReprMixin, BaseModel):
    """A result file associated with a task."""

    model_config = ConfigDict(extra="allow", populate_by_name=True)

    id: str = Field(default="", alias="file_id")
    filename: str = Field(default="", alias="name")
    size: int | None = Field(default=None, alias="file_size")

    _repr_fields: ClassVar[tuple[str, ...]] = ("id", "filename", "size")


class TaskResult(_ReprMixin, BaseModel):
    """任务执行结果（来自 Server 的 output 字段）。

    注意：此字段仅包含执行元数据（stdout、文件列表等）。
    实际结果文件请通过 ``client.list_files()`` 和 ``client.download_all_files()`` 获取。
    """

    model_config = ConfigDict(extra="allow", populate_by_name=True)

    files: list[str] = Field(default_factory=list)  # Server output.files
    stdout: str | None = None                        # Server output.stdout
    raw: dict[str, Any] = Field(default_factory=dict)  # 兜底，捕获所有额外字段

    _repr_fields: ClassVar[tuple[str, ...]] = ("files", "stdout")


class Task(_ReprMixin, BaseModel):
    """A submitted task."""

    model_config = ConfigDict(extra="allow", populate_by_name=True)

    id: str = Field(default="", alias="task_id")
    status: str = "unknown"
    service_id: str = ""
    task_prompt: str = ""
    output_prompt: str = ""
    session_id: str = ""
    started_at: str | None = None
    completed_at: str | None = None
    created_at: str | None = None
    result: TaskResult | None = Field(
        default=None,
        alias="output",
        description=(
            "Server 返回的 output 字段，仅包含执行元数据（stdout、文件列表等）。"
            "实际结果文件请通过 client.download_all_files() 获取。"
        ),
    )

    _repr_fields: ClassVar[tuple[str, ...]] = ("id", "status")

    def is_done(self) -> bool:
        """Return True if the task has reached a terminal state."""
        return self.status in ("completed", "failed", "cancelled")

    def is_success(self) -> bool:
        """Return True if the task completed successfully."""
        return self.status == "completed"

    @property
    def duration_seconds(self) -> float | None:
        """Compute elapsed seconds if both start and end times are available."""
        start = _parse_iso_time(self.started_at or self.created_at)
        end = _parse_iso_time(self.completed_at)
        if start and end:
            return (end - start).total_seconds()
        return None
