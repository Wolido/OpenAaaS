"""Utility functions reused from the MCP adapter."""

from __future__ import annotations

import os
import re
import zipfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Callable

from .exceptions import OpenAaaSError

# Security limits (same as MCP adapter)
MAX_ZIP_RATIO = 500
MAX_TOTAL_EXTRACT_SIZE = 100 * 1024 * 1024  # 100 MB
MAX_FILE_COUNT = 1000
MAX_SINGLE_FILE_SIZE = 50 * 1024 * 1024  # 50 MB


def _check_file_in_working_dir(file_path: Path) -> None:
    """Ensure *file_path* resides under the current working directory.

    Raises:
        OpenAaaSError: if the file is outside the working directory.
    """
    try:
        cwd = Path(os.getcwd()).resolve()
        real_path = file_path.resolve()
        try:
            real_path.relative_to(cwd)
        except ValueError as e:
            raise OpenAaaSError(
                f"File upload failed: only files under the working directory are allowed: {file_path}"
            ) from e
    except OSError as e:
        raise OpenAaaSError(f"File upload failed: cannot resolve path: {e}")


def _zipinfo_is_symlink(info: zipfile.ZipInfo) -> bool:
    """Detect whether a zip entry is a symbolic link."""
    if hasattr(info, "is_symlink"):
        return info.is_symlink()
    # Unix symlink: high nibble of external_attr is 0xA
    return info.create_system == 3 and (info.external_attr >> 28) == 0xA


def _safe_extract_zip(zip_path: str | Path, extract_dir: str | Path) -> Path:
    """Safely extract a zip file with zip-bomb protection.

    Args:
        zip_path: Path to the zip archive.
        extract_dir: Directory into which files will be extracted.

    Returns:
        The extraction directory.

    Raises:
        OpenAaaSError: on any security or I/O issue.
    """
    zip_path = Path(zip_path)
    extract_dir = Path(extract_dir)
    extract_dir.mkdir(parents=True, exist_ok=True)
    real_extract_dir = extract_dir.resolve()

    try:
        with zipfile.ZipFile(zip_path, "r") as zf:
            infolist = zf.infolist()

            if len(infolist) > MAX_FILE_COUNT:
                raise OpenAaaSError(
                    f"Zip contains too many files ({len(infolist)} > {MAX_FILE_COUNT}) — possible zip bomb"
                )

            total_size = sum(info.file_size for info in infolist)
            if total_size > MAX_TOTAL_EXTRACT_SIZE:
                raise OpenAaaSError(
                    f"Extracted size too large ({total_size} bytes > {MAX_TOTAL_EXTRACT_SIZE} bytes)"
                )

            for info in infolist:
                if info.file_size > MAX_SINGLE_FILE_SIZE:
                    raise OpenAaaSError(
                        f"Zip contains oversized file: {info.filename} ({info.file_size} bytes)"
                    )

            zip_size = zip_path.stat().st_size
            if zip_size > 0 and total_size / zip_size > MAX_ZIP_RATIO:
                raise OpenAaaSError(
                    f"Suspicious compression ratio ({total_size / zip_size:.1f} > {MAX_ZIP_RATIO}) — possible zip bomb"
                )

            for info in infolist:
                if _zipinfo_is_symlink(info):
                    raise OpenAaaSError(
                        f"Zip contains symlink: {info.filename} — extraction refused"
                    )

                extracted_path = extract_dir / info.filename
                real_extracted_path = extracted_path.resolve()
                try:
                    real_extracted_path.relative_to(real_extract_dir)
                except ValueError:
                    raise OpenAaaSError(
                        f"Zip contains illegal path: {info.filename}"
                    )

                zf.extract(info, extract_dir)

                # TOCTOU / symlink-after-extraction defence
                if extracted_path.exists():
                    real_after = extracted_path.resolve()
                    try:
                        real_after.relative_to(real_extract_dir)
                    except ValueError:
                        raise OpenAaaSError(
                            f"Zip contains path traversal: {info.filename}"
                        )

        return extract_dir
    except zipfile.BadZipFile as e:
        raise OpenAaaSError(f"Corrupted zip file: {e}") from e
    except OpenAaaSError:
        raise
    except Exception as e:
        raise OpenAaaSError(f"Extraction failed: {e}") from e


def _format_duration(
    started_at: str | None,
    completed_at: str | None,
    status: str,
) -> str:
    """Format a human-readable duration string (Chinese-friendly)."""
    if not started_at:
        return ""

    start = _parse_iso_time(started_at)
    if not start:
        return ""

    if status in ("running", "cancelling"):
        end = datetime.now(timezone.utc)
    elif completed_at:
        end = _parse_iso_time(completed_at)
    else:
        return ""

    if not end:
        return ""

    total_seconds = int((end - start).total_seconds())
    if total_seconds < 0:
        return ""

    hours = total_seconds // 3600
    minutes = (total_seconds % 3600) // 60
    seconds = total_seconds % 60

    if hours > 0:
        return f"{hours}h {minutes}m {seconds}s"
    if minutes > 0:
        return f"{minutes}m {seconds}s"
    return f"{seconds}s"


def _parse_iso_time(ts: str) -> datetime | None:
    """Parse an ISO-8601 timestamp string."""
    if not ts:
        return None
    # Append Z if missing
    if re.match(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?$", ts):
        ts = ts + "Z"
    try:
        ts = ts.replace("Z", "+00:00")
        dt = datetime.fromisoformat(ts)
        if dt.tzinfo is None:
            dt = dt.replace(tzinfo=timezone.utc)
        return dt
    except (ValueError, TypeError):
        return None


def _sanitize_filename(filename: str, fallback_ext: str = "download") -> str:
    """Sanitise a filename to prevent path traversal."""
    safe = os.path.basename(filename)
    safe = safe.replace("\x00", "")
    if not safe or safe in (".", "..", "/", "\\"):
        safe = f"result.{fallback_ext}"
    return safe


def _get_download_dir(task_id: str) -> Path:
    """Return a task-specific download directory under ``.OpenAaaS/downloads/``."""
    safe_task_id = re.sub(r"[\\/]", "_", task_id)
    safe_task_id = safe_task_id.replace("..", "_")
    if safe_task_id in (".", ".."):
        safe_task_id = "_"
    if not safe_task_id:
        safe_task_id = "_"
    return Path(os.getcwd()) / ".OpenAaaS" / "downloads" / safe_task_id


class ProgressCallback:
    """Simple wrapper to turn a callback into a progress reporter for downloads.

    Example::

        def on_chunk(chunk_num, total_bytes):
            print(f"Downloaded {total_bytes} bytes")

        cb = ProgressCallback(on_chunk)
        for chunk in response.iter_bytes():
            cb.update(len(chunk))
    """

    def __init__(
        self,
        callback: Callable[[int, int], None] | None = None,
        total_size: int | None = None,
    ) -> None:
        self.callback = callback
        self.total_size = total_size
        self._received = 0
        self._chunk_num = 0

    def update(self, chunk_size: int) -> None:
        self._received += chunk_size
        self._chunk_num += 1
        if self.callback:
            self.callback(self._chunk_num, self._received)
