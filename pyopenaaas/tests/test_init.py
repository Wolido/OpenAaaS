"""Tests for package-level exports and convenience functions."""

from __future__ import annotations

from typing import Any

import httpx
import pytest
import respx

import pyopenaaas
from pyopenaaas import AsyncClient, Client, Config


class TestExports:
    """Verify public API surface."""

    def test_client_exported(self) -> None:
        assert hasattr(pyopenaaas, "Client")
        assert pyopenaaas.Client is Client

    def test_async_client_exported(self) -> None:
        assert hasattr(pyopenaaas, "AsyncClient")
        assert pyopenaaas.AsyncClient is AsyncClient

    def test_config_exported(self) -> None:
        assert hasattr(pyopenaaas, "Config")
        assert pyopenaaas.Config is Config

    def test_version_exported(self) -> None:
        assert hasattr(pyopenaaas, "__version__")
        assert isinstance(pyopenaaas.__version__, str)
        assert pyopenaaas.__version__ == "0.1.3"

    def test_run_exported(self) -> None:
        assert hasattr(pyopenaaas, "run")
        assert callable(pyopenaaas.run)


class TestRunFunction:
    """Tests for pyopenaaas.run() convenience wrapper."""

    @respx.mock
    def test_run_async_discover(self) -> None:
        respx.get("https://api.example.com/api/v1/discovery").mock(
            return_value=httpx.Response(
                200,
                json={
                    "api": {"version": "1.0", "base_url": "https://api.example.com"},
                    "authentication": "Bearer",
                    "endpoints": [],
                    "services": [],
                },
            )
        )

        async def _discover() -> Any:
            async with AsyncClient(
                server_url="https://api.example.com", api_key="test-key"
            ) as client:
                return await client.discover()

        info = pyopenaaas.run(_discover())
        assert isinstance(info, pyopenaaas.models.ServerInfo)
        assert info.version == "1.0"

    @respx.mock
    def test_run_submit_and_wait_end_to_end(self) -> None:
        """Full flow: submit task -> wait -> get result."""
        base_url = "https://api.example.com"
        respx.post(f"{base_url}/api/v1/client/tasks").mock(
            return_value=httpx.Response(
                200, json={"task_id": "t-1", "status": "pending"}
            )
        )
        call_count = 0

        def task_handler(request: httpx.Request) -> httpx.Response:
            nonlocal call_count
            call_count += 1
            status = "completed" if call_count >= 2 else "running"
            return httpx.Response(
                200, json={"task_id": "t-1", "status": status}
            )

        respx.get(f"{base_url}/api/v1/client/tasks/t-1").mock(
            side_effect=task_handler
        )

        async def flow() -> Any:
            client = AsyncClient(server_url=base_url, api_key="test-key")
            task = await client.submit_task("svc-1", "Compute")
            return await client.wait_for_task(task.id, poll_interval=0.01)

        final_task = pyopenaaas.run(flow())
        assert final_task.status == "completed"
        assert final_task.id == "t-1"


class TestAllPublic:
    """Ensure __all__ matches reality."""

    def test_all_items_exist(self) -> None:
        for name in pyopenaaas.__all__:
            assert hasattr(pyopenaaas, name), f"{name} missing from pyopenaaas"
