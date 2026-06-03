"""Shared base class for sync and async clients."""

from __future__ import annotations

import mimetypes
import os
import re
import unicodedata
import uuid
from pathlib import Path
from typing import Any
from urllib.parse import quote

from .._utils import (
    _check_file_in_working_dir,
    _get_download_dir,
    _safe_extract_zip,
    _sanitize_filename,
)
from ..config import Config
from ..exceptions import RequestValidationError
from ..models import ResultFile, ServerInfo, Service, Task

MAX_UPLOAD_SIZE = 100 * 1024 * 1024  # 100 MB


class ClientBase:
    """Shared logic: URL construction, auth headers, validation, parsing."""

    def __init__(self, config: Config | None = None, **kwargs) -> None:
        self._config = config or Config(**kwargs)

    # ------------------------------------------------------------------
    # URL / headers
    # ------------------------------------------------------------------

    def _url(self, path: str) -> str:
        """Build a fully-qualified URL from a path."""
        base = self._config.server_url
        return f"{base}{path}"

    def _headers(self) -> dict[str, str]:
        """Return headers with Bearer authorization."""
        return {"Authorization": f"Bearer {self._config.require_api_key()}"}

    @staticmethod
    def _quote_id(value: str) -> str:
        """URL-quote an identifier for safe use in paths."""
        return quote(value, safe="")

    # ------------------------------------------------------------------
    # Validation
    # ------------------------------------------------------------------

    @staticmethod
    def _validate_name(name: str) -> str:
        """Sanitize and validate a user name.

        Raises:
            RequestValidationError: if the name is invalid.
        """
        name = name.strip()
        if not name:
            raise RequestValidationError("name cannot be empty")
        if len(name) > 64:
            raise RequestValidationError("name cannot exceed 64 characters")
        if re.search(r'[\x00-\x1f/\\<>|&;$]', name):
            raise RequestValidationError("name contains illegal characters")
        if any(unicodedata.category(c).startswith("C") for c in name):
            raise RequestValidationError("name contains Unicode control characters")
        return name

    @staticmethod
    def _generate_random_name() -> str:
        return f"user-{uuid.uuid4().hex[:8]}"

    @staticmethod
    def _validate_task_id(task_id: str) -> str:
        if not task_id or not task_id.strip():
            raise RequestValidationError("task_id cannot be empty")
        return task_id.strip()

    @staticmethod
    def _validate_service_id(service_id: str) -> str:
        if not service_id or not service_id.strip():
            raise RequestValidationError("service_id cannot be empty")
        return service_id.strip()

    # ------------------------------------------------------------------
    # Shared helpers
    # ------------------------------------------------------------------

    def _prepare_files_sync(
        self, file_path: str | Path
    ) -> list[tuple[str, tuple[str, bytes, str]]]:
        """Validate and read a local file for upload.

        Returns:
            A list with a single file tuple suitable for *files* in
            :func:`safe_request`.
        """
        fp = Path(file_path)
        if not fp.is_absolute():
            fp = Path(os.getcwd()) / fp

        _check_file_in_working_dir(fp)
        if fp.is_symlink():
            raise RequestValidationError(f"Symlinks are not allowed: {fp}")
        if not fp.exists():
            raise RequestValidationError(f"File not found: {fp}")
        if not fp.is_file():
            raise RequestValidationError(f"Not a file: {fp}")

        try:
            size = fp.stat().st_size
        except OSError as e:
            raise RequestValidationError(f"Cannot stat file: {e}")
        if size > MAX_UPLOAD_SIZE:
            raise RequestValidationError(
                f"File too large: {size} bytes (max {MAX_UPLOAD_SIZE} bytes)"
            )

        content = fp.read_bytes()
        if len(content) > MAX_UPLOAD_SIZE:
            raise RequestValidationError(
                f"File too large: {len(content)} bytes (max {MAX_UPLOAD_SIZE} bytes)"
            )

        mime_type = mimetypes.guess_type(str(fp))[0] or "application/octet-stream"
        return [("files", (fp.name, content, mime_type))]

    @staticmethod
    def _build_submit_fields(
        service_id: str,
        task_prompt: str,
        output_prompt: str,
        session_id: str,
    ) -> dict[str, str]:
        """Build the form fields for submit_task."""
        fields: dict[str, str] = {
            "service_id": service_id,
            "task_prompt": task_prompt,
            "output_prompt": output_prompt or "",
        }
        if session_id:
            fields["session_id"] = session_id
        return fields

    def _resolve_download_path(
        self,
        identifier: str,
        save_path: str | Path | None,
        default_name: str = "result.download",
    ) -> Path:
        """Resolve the final file path for a download."""
        if save_path is None:
            return _get_download_dir(identifier) / default_name
        sp = Path(save_path)
        if sp.is_dir():
            return sp / default_name
        return sp

    @staticmethod
    def _process_downloaded_file_sync(
        file_path: Path,
        extract_zip: bool = True,
    ) -> Path:
        """Optionally extract a zip and clean up the archive.

        Returns:
            Path to the extracted directory (if zip and extract_zip=True)
            or the original file path.
        """
        if extract_zip and file_path.suffix.lower() == ".zip":
            extract_dir = file_path.parent / file_path.stem
            _safe_extract_zip(file_path, extract_dir)
            try:
                file_path.unlink()
            except OSError:
                pass
            return extract_dir
        return file_path

    # ------------------------------------------------------------------
    # Response parsing
    # ------------------------------------------------------------------

    @staticmethod
    def _parse_server_info(data: dict, base_url: str) -> ServerInfo:
        api_info = data.get("api", {}) if isinstance(data, dict) else {}
        return ServerInfo(
            version=api_info.get("version", "unknown"),
            base_url=api_info.get("base_url", base_url),
            authentication=data.get("authentication")
            or data.get("auth")
            or "Bearer Token",
            endpoints=data.get("endpoints", []),
            services=data.get("services", []),
        )

    @staticmethod
    def _parse_services(data: Any) -> list[Service]:
        services = data if isinstance(data, list) else data.get("services", [])
        return [Service.model_validate(s) for s in services if isinstance(s, dict)]

    @staticmethod
    def _parse_task(data: dict) -> Task:
        return Task.model_validate(data)

    @staticmethod
    def _parse_result_files(data: Any) -> list[ResultFile]:
        files = data if isinstance(data, list) else data.get("files", [])
        return [ResultFile.model_validate(f) for f in files if isinstance(f, dict)]

    def __repr__(self) -> str:
        return f"{self.__class__.__name__}(server_url={self._config.server_url!r})"
