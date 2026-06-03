"""Tests for utility functions."""

from __future__ import annotations

import os
import zipfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import pytest

from pyopenaaas._utils import (
    MAX_FILE_COUNT,
    MAX_SINGLE_FILE_SIZE,
    MAX_TOTAL_EXTRACT_SIZE,
    MAX_ZIP_RATIO,
    ProgressCallback,
    _check_file_in_working_dir,
    _format_duration,
    _get_download_dir,
    _parse_iso_time,
    _safe_extract_zip,
    _sanitize_filename,
    _zipinfo_is_symlink,
)
from pyopenaaas.exceptions import OpenAaaSError


class TestCheckFileInWorkingDir:
    """Tests for _check_file_in_working_dir."""

    def test_file_in_cwd_passes(self, tmp_path: Path, monkeypatch: Any) -> None:
        monkeypatch.chdir(tmp_path)
        file = tmp_path / "data.txt"
        file.write_text("hello")
        # Should not raise
        _check_file_in_working_dir(file)

    def test_file_in_subdir_passes(self, tmp_path: Path, monkeypatch: Any) -> None:
        monkeypatch.chdir(tmp_path)
        sub = tmp_path / "sub"
        sub.mkdir()
        file = sub / "data.txt"
        file.write_text("hello")
        _check_file_in_working_dir(file)

    def test_parent_dir_rejected(self, tmp_path: Path, monkeypatch: Any) -> None:
        wd = tmp_path / "wd"
        wd.mkdir()
        monkeypatch.chdir(wd)
        file = tmp_path / "secret.txt"
        file.write_text("secret")
        with pytest.raises(OpenAaaSError) as exc_info:
            _check_file_in_working_dir(file)
        assert "only files under the working directory" in str(exc_info.value)

    def test_nonexistent_path_in_cwd_allowed(self, tmp_path: Path, monkeypatch: Any) -> None:
        """Nonexistent paths under cwd do not raise."""
        monkeypatch.chdir(tmp_path)
        missing = tmp_path / "missing.txt"
        # Mock resolve to avoid platform-specific behavior for nonexistent paths
        monkeypatch.setattr(Path, "resolve", lambda self, strict=False: self.absolute())
        _check_file_in_working_dir(missing)

    def test_symlink_outside_cwd_rejected(self, tmp_path: Path, monkeypatch: Any) -> None:
        """A symlink pointing outside the working dir should be rejected."""
        monkeypatch.chdir(tmp_path)
        outside = tmp_path.parent / "outside.txt"
        outside.write_text("x")
        link = tmp_path / "link.txt"
        link.symlink_to(outside)
        with pytest.raises(OpenAaaSError):
            _check_file_in_working_dir(link)


class TestSafeExtractZip:
    """Tests for _safe_extract_zip."""

    def test_normal_extraction(self, tmp_path: Path) -> None:
        zip_path = tmp_path / "archive.zip"
        extract_dir = tmp_path / "out"
        with zipfile.ZipFile(zip_path, "w") as zf:
            zf.writestr("hello.txt", "world")
        result = _safe_extract_zip(zip_path, extract_dir)
        assert result == extract_dir
        assert (extract_dir / "hello.txt").read_text() == "world"

    def test_path_traversal_rejected(self, tmp_path: Path) -> None:
        zip_path = tmp_path / "bad.zip"
        with zipfile.ZipFile(zip_path, "w") as zf:
            zf.writestr("../../etc/passwd", "root")
        with pytest.raises(OpenAaaSError) as exc_info:
            _safe_extract_zip(zip_path, tmp_path / "out")
        assert "illegal path" in str(exc_info.value)

    def test_too_many_files_rejected(self, tmp_path: Path) -> None:
        zip_path = tmp_path / "bomb.zip"
        with zipfile.ZipFile(zip_path, "w") as zf:
            for i in range(MAX_FILE_COUNT + 1):
                zf.writestr(f"f{i}.txt", "x")
        with pytest.raises(OpenAaaSError) as exc_info:
            _safe_extract_zip(zip_path, tmp_path / "out")
        assert "too many files" in str(exc_info.value)

    def test_total_size_too_large_rejected(self, tmp_path: Path) -> None:
        zip_path = tmp_path / "big.zip"
        with zipfile.ZipFile(zip_path, "w") as zf:
            # Write a single file larger than MAX_TOTAL_EXTRACT_SIZE
            zf.writestr("big.bin", "x" * (MAX_TOTAL_EXTRACT_SIZE + 1))
        with pytest.raises(OpenAaaSError) as exc_info:
            _safe_extract_zip(zip_path, tmp_path / "out")
        assert "Extracted size too large" in str(exc_info.value)

    def test_single_file_too_large_rejected(self, tmp_path: Path) -> None:
        zip_path = tmp_path / "huge.zip"
        with zipfile.ZipFile(zip_path, "w") as zf:
            zf.writestr("huge.bin", "x" * (MAX_SINGLE_FILE_SIZE + 1))
        with pytest.raises(OpenAaaSError) as exc_info:
            _safe_extract_zip(zip_path, tmp_path / "out")
        assert "oversized file" in str(exc_info.value)

    def test_compression_ratio_rejected(self, tmp_path: Path) -> None:
        """A zip with extreme compression ratio is a zip bomb."""
        zip_path = tmp_path / "ratio.zip"
        with zipfile.ZipFile(zip_path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
            # Highly compressible data to get a huge ratio
            zf.writestr("bomb.txt", "0" * (MAX_ZIP_RATIO * 1000 + 1))
        with pytest.raises(OpenAaaSError) as exc_info:
            _safe_extract_zip(zip_path, tmp_path / "out")
        assert "compression ratio" in str(exc_info.value)

    def test_symlink_in_zip_rejected(self, tmp_path: Path) -> None:
        """Zip entries that are symlinks must be rejected."""
        zip_path = tmp_path / "symlink.zip"
        with zipfile.ZipFile(zip_path, "w") as zf:
            info = zipfile.ZipInfo("link")
            info.create_system = 3  # Unix
            info.external_attr = (0o120777 << 16) | (0xA << 28)
            zf.writestr(info, "/etc/passwd")
        with pytest.raises(OpenAaaSError) as exc_info:
            _safe_extract_zip(zip_path, tmp_path / "out")
        assert "symlink" in str(exc_info.value)

    def test_bad_zipfile_raises(self, tmp_path: Path) -> None:
        bad = tmp_path / "not_a_zip.zip"
        bad.write_text("this is not a zip")
        with pytest.raises(OpenAaaSError) as exc_info:
            _safe_extract_zip(bad, tmp_path / "out")
        assert "Corrupted zip file" in str(exc_info.value)


class TestFormatDuration:
    """Tests for _format_duration."""

    def test_no_started_at(self) -> None:
        assert _format_duration(None, "2024-01-01T00:01:00Z", "completed") == ""

    def test_running_uses_now(self) -> None:
        """For running status, end time should be approximated as 'now'."""
        result = _format_duration(
            datetime.now(timezone.utc).isoformat(), None, "running"
        )
        assert "s" in result or result == ""

    def test_completed_with_both_times(self) -> None:
        result = _format_duration(
            "2024-01-01T00:00:00Z",
            "2024-01-01T00:01:05Z",
            "completed",
        )
        assert result == "1m 5s"

    def test_hours(self) -> None:
        result = _format_duration(
            "2024-01-01T00:00:00Z",
            "2024-01-01T02:30:45Z",
            "completed",
        )
        assert result == "2h 30m 45s"

    def test_seconds_only(self) -> None:
        result = _format_duration(
            "2024-01-01T00:00:00Z",
            "2024-01-01T00:00:05Z",
            "completed",
        )
        assert result == "5s"

    def test_negative_returns_empty(self) -> None:
        result = _format_duration(
            "2024-01-01T00:01:00Z",
            "2024-01-01T00:00:00Z",
            "completed",
        )
        assert result == ""


class TestParseIsoTime:
    """Tests for _parse_iso_time."""

    def test_z_suffix(self) -> None:
        dt = _parse_iso_time("2024-01-01T00:00:00Z")
        assert dt is not None
        assert dt.year == 2024
        assert dt.tzinfo is not None

    def test_no_z_suffix(self) -> None:
        dt = _parse_iso_time("2024-01-01T00:00:00")
        assert dt is not None
        assert dt.year == 2024

    def test_with_microseconds(self) -> None:
        dt = _parse_iso_time("2024-01-01T00:00:00.123456Z")
        assert dt is not None
        assert dt.microsecond == 123456

    def test_offset_format(self) -> None:
        dt = _parse_iso_time("2024-01-01T00:00:00+08:00")
        assert dt is not None

    def test_none_returns_none(self) -> None:
        assert _parse_iso_time(None) is None

    def test_empty_string_returns_none(self) -> None:
        assert _parse_iso_time("") is None

    def test_invalid_string_returns_none(self) -> None:
        assert _parse_iso_time("not-a-date") is None


class TestSanitizeFilename:
    """Tests for _sanitize_filename."""

    def test_basename_only(self) -> None:
        assert _sanitize_filename("/etc/passwd") == "passwd"

    def test_null_bytes_removed(self) -> None:
        assert _sanitize_filename("file\x00name.txt") == "filename.txt"

    def test_empty_fallback(self) -> None:
        assert _sanitize_filename("") == "result.download"

    def test_dot_fallback(self) -> None:
        assert _sanitize_filename(".") == "result.download"

    def test_custom_fallback_ext(self) -> None:
        assert _sanitize_filename("", fallback_ext="csv") == "result.csv"


class TestGetDownloadDir:
    """Tests for _get_download_dir."""

    def test_returns_subdirectory(self, tmp_path: Path, monkeypatch: Any) -> None:
        monkeypatch.chdir(tmp_path)
        d = _get_download_dir("task-123")
        assert d == tmp_path / ".OpenAaaS" / "downloads" / "task-123"

    def test_sanitizes_path_separators(self, tmp_path: Path, monkeypatch: Any) -> None:
        monkeypatch.chdir(tmp_path)
        d = _get_download_dir("task/123\\456")
        assert "task_123_456" in str(d)

    def test_sanitizes_dotdot(self, tmp_path: Path, monkeypatch: Any) -> None:
        monkeypatch.chdir(tmp_path)
        d = _get_download_dir("..")
        assert d.name == "_"


class TestProgressCallback:
    """Tests for ProgressCallback."""

    def test_no_callback_no_crash(self) -> None:
        cb = ProgressCallback()
        cb.update(1024)
        assert cb._received == 1024
        assert cb._chunk_num == 1

    def test_callback_invoked(self) -> None:
        calls: list[tuple[int, int]] = []

        def on_chunk(chunk_num: int, total: int) -> None:
            calls.append((chunk_num, total))

        cb = ProgressCallback(callback=on_chunk, total_size=4096)
        cb.update(1024)
        cb.update(1024)
        assert calls == [(1, 1024), (2, 2048)]

    def test_total_size_optional(self) -> None:
        calls: list[tuple[int, int]] = []

        def on_chunk(chunk_num: int, total: int) -> None:
            calls.append((chunk_num, total))

        cb = ProgressCallback(callback=on_chunk)
        cb.update(512)
        assert calls == [(1, 512)]


class TestZipinfoIsSymlink:
    """Direct tests for _zipinfo_is_symlink."""

    def test_regular_file(self) -> None:
        info = zipfile.ZipInfo("file.txt")
        assert _zipinfo_is_symlink(info) is False

    def test_unix_symlink(self) -> None:
        info = zipfile.ZipInfo("link")
        info.create_system = 3
        info.external_attr = (0o120777 << 16) | (0xA << 28)
        assert _zipinfo_is_symlink(info) is True
