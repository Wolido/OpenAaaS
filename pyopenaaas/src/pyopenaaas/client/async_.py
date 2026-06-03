"""Asynchronous OpenAaaS client (httpx.AsyncClient)."""

from __future__ import annotations

import asyncio
import json
from pathlib import Path
from typing import Any, AsyncIterator, Callable

import aiofiles
import httpx

from .._http import (
    DEFAULT_TIMEOUT,
    DOWNLOAD_TIMEOUT,
    UPLOAD_TIMEOUT,
    _build_multipart_body,
    _extract_error_message,
    _map_exception,
)
from .._utils import _get_download_dir, _sanitize_filename
from ..config import Config
from ..exceptions import OpenAaaSError, RequestTimeoutError, RequestValidationError
from ..models import ResultFile, ServerInfo, Service, ServiceUsage, Task
from ._base import ClientBase


async def _async_safe_request(
    method: str,
    url: str,
    headers: dict[str, str] | None = None,
    data: dict[str, Any] | None = None,
    files: list[tuple[str, tuple[str, bytes, str]]] | None = None,
    timeout: float | None = None,
    max_redirects: int = 3,
) -> Any:
    """Async equivalent of :func:`pyopenaaas._http.safe_request`."""
    headers = headers or {}
    timeout_val = timeout or DEFAULT_TIMEOUT
    current_url = url
    remaining_redirects = max_redirects
    original_host = httpx.URL(url).host

    try:
        async with httpx.AsyncClient(
            timeout=timeout_val, follow_redirects=False
        ) as client:
            while remaining_redirects >= 0:
                req_headers = dict(headers)
                current_host = httpx.URL(current_url).host
                if current_host != original_host:
                    for h in list(req_headers.keys()):
                        if h.lower() == "authorization":
                            req_headers.pop(h)

                if files is not None:
                    form_data = {k: str(v) for k, v in (data or {}).items()}
                    body, boundary = _build_multipart_body(form_data, files)
                    for h in list(req_headers.keys()):
                        if h.lower() == "content-type":
                            req_headers.pop(h)
                    req_headers["Content-Type"] = (
                        f"multipart/form-data; boundary={boundary}"
                    )
                    response = await client.request(
                        method, current_url, headers=req_headers, content=body
                    )
                elif data is not None:
                    if "Content-Type" not in req_headers:
                        req_headers = {
                            **req_headers,
                            "Content-Type": "application/json",
                        }
                    response = await client.request(
                        method, current_url, headers=req_headers, json=data
                    )
                else:
                    response = await client.request(
                        method, current_url, headers=req_headers
                    )

                if 300 <= response.status_code < 400:
                    location = response.headers.get("Location")
                    if location:
                        if remaining_redirects > 0:
                            current_url = str(httpx.URL(current_url).join(location))
                            remaining_redirects -= 1
                            continue
                        raise OpenAaaSError(
                            "Too many redirects — please use an HTTPS URL directly"
                        )

                response.raise_for_status()
                if response.status_code == 204 or not response.content:
                    return {}
                return response.json()

    except httpx.HTTPStatusError as e:
        raise _map_exception(e, current_url)
    except (
        httpx.ConnectError,
        httpx.TimeoutException,
        httpx.NetworkError,
        httpx.InvalidURL,
    ) as e:
        raise _map_exception(e, current_url)
    except json.JSONDecodeError as e:
        raise OpenAaaSError(f"JSON decode error in response: {e}") from e
    except OpenAaaSError:
        raise
    except Exception as e:
        raise OpenAaaSError(f"Request failed: {e}") from e


async def _async_stream_download(
    url: str,
    headers: dict[str, str] | None = None,
    timeout: float | None = None,
    chunk_size: int = 8192,
) -> AsyncIterator[bytes]:
    """Async generator yielding response chunks."""
    headers = headers or {}
    timeout_val = timeout or DOWNLOAD_TIMEOUT
    try:
        async with httpx.AsyncClient(
            timeout=timeout_val, follow_redirects=True
        ) as client:
            async with client.stream("GET", url, headers=headers) as response:
                response.raise_for_status()
                async for chunk in response.aiter_bytes(chunk_size=chunk_size):
                    yield chunk
    except httpx.HTTPStatusError as e:
        raise _map_exception(e, url)
    except (
        httpx.ConnectError,
        httpx.TimeoutException,
        httpx.NetworkError,
        httpx.InvalidURL,
    ) as e:
        raise _map_exception(e, url)
    except OpenAaaSError:
        raise
    except Exception as e:
        raise OpenAaaSError(f"Download failed: {e}") from e


class AsyncClient(ClientBase):
    """Asynchronous OpenAaaS client.

    Usage::

        async with AsyncClient(server_url="...", api_key="...") as client:
            services = await client.list_services()
            task = await client.submit_task("svc-1", "Compute something")
            task = await client.wait_for_task(task.id)
            paths = await client.download_all_files(task.id)
    """

    def __init__(self, config: Config | None = None, **kwargs) -> None:
        super().__init__(config, **kwargs)

    async def __aenter__(self) -> AsyncClient:
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb) -> None:
        pass

    # ------------------------------------------------------------------
    # Discovery & registration
    # ------------------------------------------------------------------

    async def discover(self) -> ServerInfo:
        """Discover server API information."""
        url = self._url("/api/v1/discovery")
        data = await _async_safe_request("GET", url)
        return self._parse_server_info(data, self._config.server_url)

    async def register(self, name: str | None = None) -> dict:
        """Register a new client account and obtain an API key.

        Args:
            name: Desired display name. If *None* or empty, a random name
                ``user-{uuid}`` is generated automatically.
        """
        if not name:
            name = self._generate_random_name()
        name = self._validate_name(name)
        url = self._url("/api/v1/client/auth/register")
        result = await _async_safe_request("POST", url, data={"name": name})

        api_key = result.get("api_key") or result.get("token")
        client_id = result.get("client_id") or result.get("id")

        if api_key:
            self._config.api_key = api_key
        if client_id:
            self._config.client_id = client_id
        self._config.name = name

        return result

    async def update_profile(self, name: str) -> dict:
        """Update the current client's profile name."""
        name = self._validate_name(name)
        url = self._url("/api/v1/client/profile")
        result = await _async_safe_request(
            "PUT",
            url,
            headers=self._headers(),
            data={"name": name},
        )
        self._config.name = result.get("name", name)
        return result

    # ------------------------------------------------------------------
    # Services
    # ------------------------------------------------------------------

    async def list_services(self) -> list[Service]:
        """List available Agent services."""
        url = self._url("/api/v1/client/services")
        result = await _async_safe_request("GET", url, headers=self._headers())
        return self._parse_services(result)

    async def get_service_usage(self, service_id: str) -> ServiceUsage:
        """Get detailed usage instructions for a service."""
        sid = self._validate_service_id(service_id)
        url = self._url(f"/api/v1/client/services/{self._quote_id(sid)}/usage")
        result = await _async_safe_request("GET", url, headers=self._headers())
        return ServiceUsage.model_validate(result)

    # ------------------------------------------------------------------
    # Tasks
    # ------------------------------------------------------------------

    async def submit_task(
        self,
        service_id: str,
        task_prompt: str,
        output_prompt: str = "",
        input_files: list[str | Path] | None = None,
        session_id: str = "",
    ) -> Task:
        """Submit a task to a remote Agent."""
        sid = self._validate_service_id(service_id)
        if not task_prompt:
            raise RequestValidationError("task_prompt cannot be empty")

        url = self._url("/api/v1/client/tasks")
        fields = self._build_submit_fields(sid, task_prompt, output_prompt, session_id)

        files: list[tuple[str, tuple[str, bytes, str]]] = []
        if input_files:
            if len(input_files) > 10:
                raise RequestValidationError("At most 10 files can be uploaded")
            for fp in input_files:
                files.extend(await asyncio.to_thread(self._prepare_files_sync, fp))

        result = await _async_safe_request(
            "POST",
            url,
            headers=self._headers(),
            data=fields,
            files=files,
            timeout=UPLOAD_TIMEOUT,
        )
        return self._parse_task(result)

    async def get_task(self, task_id: str) -> Task:
        """Query the current status and result of a task."""
        tid = self._validate_task_id(task_id)
        url = self._url(f"/api/v1/client/tasks/{self._quote_id(tid)}")
        result = await _async_safe_request("GET", url, headers=self._headers())
        return self._parse_task(result)

    async def cancel_task(self, task_id: str) -> Task:
        """Cancel an executing task."""
        tid = self._validate_task_id(task_id)
        url = self._url(f"/api/v1/client/tasks/{self._quote_id(tid)}/cancel")
        result = await _async_safe_request(
            "POST",
            url,
            headers={
                **self._headers(),
                "Content-Type": "application/json",
            },
        )
        return self._parse_task(result)

    async def wait_for_task(
        self,
        task_id: str,
        poll_interval: float = 5.0,
        timeout: float | None = None,
        callback: Callable[[Task], None] | None = None,
    ) -> Task:
        """Poll a task until it reaches a terminal state."""
        tid = self._validate_task_id(task_id)
        start = asyncio.get_running_loop().time()
        while True:
            task = await self.get_task(tid)
            if callback:
                callback(task)
            if task.is_done():
                return task
            elapsed = asyncio.get_running_loop().time() - start
            if timeout is not None:
                if elapsed >= timeout:
                    raise RequestTimeoutError(f"Timed out waiting for task {tid}")
                sleep_for = min(poll_interval, timeout - elapsed)
            else:
                sleep_for = poll_interval
            await asyncio.sleep(sleep_for)

    # ------------------------------------------------------------------
    # Files
    # ------------------------------------------------------------------

    async def list_files(self, task_id: str) -> list[ResultFile]:
        """List result files for a completed task."""
        tid = self._validate_task_id(task_id)
        url = self._url(f"/api/v1/client/files/list/{self._quote_id(tid)}")
        result = await _async_safe_request("GET", url, headers=self._headers())
        return self._parse_result_files(result)

    async def download_file(
        self,
        file_id: str,
        save_path: str | Path | None = None,
        extract_zip: bool = True,
    ) -> Path:
        """Download a single result file."""
        if not file_id:
            raise RequestValidationError("file_id cannot be empty")

        download_url = self._url(
            f"/api/v1/client/files/{self._quote_id(file_id)}/download"
        )

        save_path = await asyncio.to_thread(
            self._resolve_download_path, file_id, save_path
        )
        await asyncio.to_thread(save_path.parent.mkdir, parents=True, exist_ok=True)

        async with aiofiles.open(save_path, "wb") as f:
            async for chunk in _async_stream_download(
                download_url, headers=self._headers()
            ):
                await f.write(chunk)

        return await asyncio.to_thread(
            self._process_downloaded_file_sync, save_path, extract_zip
        )

    async def download_all_files(
        self,
        task_id: str,
        save_dir: str | Path | None = None,
        extract_zip: bool = True,
    ) -> list[Path]:
        """Download every result file for a task."""
        tid = self._validate_task_id(task_id)
        files = await self.list_files(tid)
        if not files:
            return []

        if save_dir is None:
            save_dir = _get_download_dir(tid)
        save_dir = Path(save_dir)
        await asyncio.to_thread(save_dir.mkdir, parents=True, exist_ok=True)

        semaphore = asyncio.Semaphore(3)

        async def _download_one(rf: ResultFile) -> Path:
            async with semaphore:
                filename = rf.filename or f"{rf.id}.download"
                safe_name = _sanitize_filename(filename)
                dest = save_dir / safe_name
                counter = 1
                stem = dest.stem
                suffix = dest.suffix
                while await asyncio.to_thread(dest.exists):
                    dest = save_dir / f"{stem}_{counter}{suffix}"
                    counter += 1

                download_url = self._url(
                    f"/api/v1/client/files/{self._quote_id(rf.id)}/download"
                )
                async with aiofiles.open(dest, "wb") as f:
                    async for chunk in _async_stream_download(
                        download_url, headers=self._headers()
                    ):
                        await f.write(chunk)

                return await asyncio.to_thread(
                    self._process_downloaded_file_sync, dest, extract_zip
                )

        return await asyncio.gather(*[_download_one(rf) for rf in files])
