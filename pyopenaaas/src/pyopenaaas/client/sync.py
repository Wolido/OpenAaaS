"""Synchronous OpenAaaS client."""

from __future__ import annotations

import time
from pathlib import Path
from typing import Callable

from .._http import DOWNLOAD_TIMEOUT, UPLOAD_TIMEOUT, safe_request, stream_download
from .._utils import _get_download_dir, _sanitize_filename
from ..config import Config
from ..exceptions import RequestTimeoutError, RequestValidationError
from ..models import ResultFile, ServerInfo, Service, ServiceUsage, Task
from ._base import ClientBase


class Client(ClientBase):
    """Synchronous OpenAaaS client.

    Usage::

        with Client(server_url="...", api_key="...") as client:
            services = client.list_services()
            task = client.submit_task("svc-1", "Compute something")
            task = client.wait_for_task(task.id)
            paths = client.download_all_files(task.id)
    """

    def __init__(self, config: Config | None = None, **kwargs) -> None:
        super().__init__(config, **kwargs)

    def __enter__(self) -> Client:
        return self

    def __exit__(self, exc_type, exc_val, exc_tb) -> None:
        pass

    # ------------------------------------------------------------------
    # Discovery & registration
    # ------------------------------------------------------------------

    def discover(self) -> ServerInfo:
        """Discover server API information."""
        url = self._url("/api/v1/discovery")
        data = safe_request("GET", url)
        return self._parse_server_info(data, self._config.server_url)

    def register(self, name: str | None = None) -> dict:
        """Register a new client account and obtain an API key.

        Args:
            name: Desired display name. If *None* or empty, a random name
                ``user-{uuid}`` is generated automatically.

        Returns:
            The raw server response dict (contains ``api_key``, ``client_id``, etc.).
        """
        if not name:
            name = self._generate_random_name()
        name = self._validate_name(name)
        url = self._url("/api/v1/client/auth/register")
        result = safe_request("POST", url, data={"name": name})

        api_key = result.get("api_key") or result.get("token")
        client_id = result.get("client_id") or result.get("id")

        # Update in-memory config so subsequent calls work immediately
        if api_key:
            self._config.api_key = api_key
        if client_id:
            self._config.client_id = client_id
        self._config.name = name

        return result

    def update_profile(self, name: str) -> dict:
        """Update the current client's profile name."""
        name = self._validate_name(name)
        url = self._url("/api/v1/client/profile")
        result = safe_request(
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

    def list_services(self) -> list[Service]:
        """List available Agent services."""
        url = self._url("/api/v1/client/services")
        result = safe_request("GET", url, headers=self._headers())
        return self._parse_services(result)

    def get_service_usage(self, service_id: str) -> ServiceUsage:
        """Get detailed usage instructions for a service."""
        sid = self._validate_service_id(service_id)
        url = self._url(f"/api/v1/client/services/{self._quote_id(sid)}/usage")
        result = safe_request("GET", url, headers=self._headers())
        return ServiceUsage.model_validate(result)

    # ------------------------------------------------------------------
    # Tasks
    # ------------------------------------------------------------------

    def submit_task(
        self,
        service_id: str,
        task_prompt: str,
        output_prompt: str = "",
        input_files: list[str | Path] | None = None,
        session_id: str = "",
    ) -> Task:
        """Submit a task to a remote Agent.

        Args:
            service_id: Target service identifier.
            task_prompt: Main task description.
            output_prompt: Expected output format / constraints.
            input_files: Local file paths to upload (max 10, max 100 MB each).
            session_id: Optional session identifier for grouping tasks.

        Returns:
            A :class:`Task` instance.
        """
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
                files.extend(self._prepare_files_sync(fp))

        result = safe_request(
            "POST",
            url,
            headers=self._headers(),
            data=fields,
            files=files,
            timeout=UPLOAD_TIMEOUT,
        )
        return self._parse_task(result)

    def get_task(self, task_id: str) -> Task:
        """Query the current status and result of a task."""
        tid = self._validate_task_id(task_id)
        url = self._url(f"/api/v1/client/tasks/{self._quote_id(tid)}")
        result = safe_request("GET", url, headers=self._headers())
        return self._parse_task(result)

    def cancel_task(self, task_id: str) -> Task:
        """Cancel an executing task."""
        tid = self._validate_task_id(task_id)
        url = self._url(f"/api/v1/client/tasks/{self._quote_id(tid)}/cancel")
        result = safe_request(
            "POST",
            url,
            headers={
                **self._headers(),
                "Content-Type": "application/json",
            },
        )
        return self._parse_task(result)

    def wait_for_task(
        self,
        task_id: str,
        poll_interval: float = 5.0,
        timeout: float | None = None,
        callback: Callable[[Task], None] | None = None,
    ) -> Task:
        """Poll a task until it reaches a terminal state.

        Args:
            task_id: Task to wait for.
            poll_interval: Seconds between polls.
            timeout: Maximum total seconds to wait (``None`` = unlimited).
            callback: Called with the current :class:`Task` after each poll.

        Returns:
            The final :class:`Task`.

        Raises:
            RequestTimeoutError: if *timeout* is exceeded.
        """
        tid = self._validate_task_id(task_id)
        start = time.monotonic()
        while True:
            task = self.get_task(tid)
            if callback:
                callback(task)
            if task.is_done():
                return task
            elapsed = time.monotonic() - start
            if timeout is not None:
                if elapsed >= timeout:
                    raise RequestTimeoutError(f"Timed out waiting for task {tid}")
                sleep_for = min(poll_interval, timeout - elapsed)
            else:
                sleep_for = poll_interval
            time.sleep(sleep_for)

    # ------------------------------------------------------------------
    # Files
    # ------------------------------------------------------------------

    def list_files(self, task_id: str) -> list[ResultFile]:
        """List result files for a completed task."""
        tid = self._validate_task_id(task_id)
        url = self._url(f"/api/v1/client/files/list/{self._quote_id(tid)}")
        result = safe_request("GET", url, headers=self._headers())
        return self._parse_result_files(result)

    def download_file(
        self,
        file_id: str,
        save_path: str | Path | None = None,
        extract_zip: bool = True,
    ) -> Path:
        """Download a single result file.

        Args:
            file_id: File identifier.
            save_path: Destination path (directory or full file path).
                If ``None``, saves to ``.OpenAaaS/downloads/{file_id}/``.
            extract_zip: Automatically unzip ``.zip`` files after download.

        Returns:
            Path to the saved file (or extracted directory if zip).
        """
        if not file_id:
            raise RequestValidationError("file_id cannot be empty")

        download_url = self._url(
            f"/api/v1/client/files/{self._quote_id(file_id)}/download"
        )

        save_path = self._resolve_download_path(file_id, save_path)
        save_path.parent.mkdir(parents=True, exist_ok=True)

        with open(save_path, "wb") as f:
            for chunk in stream_download(download_url, headers=self._headers()):
                f.write(chunk)

        return self._process_downloaded_file_sync(save_path, extract_zip)

    def download_all_files(
        self,
        task_id: str,
        save_dir: str | Path | None = None,
        extract_zip: bool = True,
    ) -> list[Path]:
        """Download every result file for a task.

        Args:
            task_id: Task identifier.
            save_dir: Destination directory. If ``None``, uses
                ``.OpenAaaS/downloads/{task_id}/``.
            extract_zip: Auto-unzip ``.zip`` files.

        Returns:
            List of saved paths (files or extracted directories).
        """
        tid = self._validate_task_id(task_id)
        files = self.list_files(tid)
        if not files:
            return []

        if save_dir is None:
            save_dir = _get_download_dir(tid)
        save_dir = Path(save_dir)
        save_dir.mkdir(parents=True, exist_ok=True)

        paths: list[Path] = []
        for rf in files:
            filename = rf.filename or f"{rf.id}.download"
            safe_name = _sanitize_filename(filename)
            dest = save_dir / safe_name
            counter = 1
            stem = dest.stem
            suffix = dest.suffix
            while dest.exists():
                dest = save_dir / f"{stem}_{counter}{suffix}"
                counter += 1

            download_url = self._url(
                f"/api/v1/client/files/{self._quote_id(rf.id)}/download"
            )
            with open(dest, "wb") as f:
                for chunk in stream_download(download_url, headers=self._headers()):
                    f.write(chunk)

            paths.append(self._process_downloaded_file_sync(dest, extract_zip))

        return paths
