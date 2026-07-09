"""Tests for tools.py helper functions."""

import io
import os
import zipfile
from datetime import datetime, timezone
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

from openaaas_mcp_adapter.http_client import OpenAaaSError
from openaaas_mcp_adapter.tools import (
    MAX_FILE_COUNT,
    MAX_SINGLE_FILE_SIZE,
    MAX_TOTAL_EXTRACT_SIZE,
    MAX_ZIP_RATIO,
    _check_file_in_working_dir,
    _format_duration,
    _get_download_dir,
    _parse_iso_time,
    _safe_extract_zip,
    _sanitize_filename,
    _zipinfo_is_symlink,
)


class TestSanitizeFilename:
    """Tests for _sanitize_filename."""

    def test_normal_filename(self):
        assert _sanitize_filename("report.txt") == "report.txt"

    def test_path_traversal(self):
        assert _sanitize_filename("../../../etc/passwd") == "passwd"

    def test_empty_string(self):
        assert _sanitize_filename("") == "result.download"

    def test_special_chars(self):
        # null bytes should be stripped by _sanitize_filename
        assert _sanitize_filename("file\x00name.txt") == "filename.txt"

    def test_dot_only(self):
        assert _sanitize_filename(".") == "result.download"
        assert _sanitize_filename("..") == "result.download"


class TestFormatDuration:
    """Tests for _format_duration."""

    def test_no_started_at(self):
        assert _format_duration(None, None, "completed") == ""

    def test_running_status(self):
        start = datetime(2024, 1, 1, 12, 0, 0, tzinfo=timezone.utc).isoformat()
        result = _format_duration(start, None, "running")
        assert "小时" in result or "分钟" in result or "秒" in result

    def test_pending_status_no_completed(self):
        start = datetime(2024, 1, 1, 12, 0, 0, tzinfo=timezone.utc).isoformat()
        assert _format_duration(start, None, "pending") == ""

    def test_completed_with_times(self):
        start = "2024-01-01T12:00:00Z"
        end = "2024-01-01T12:01:30Z"
        assert _format_duration(start, end, "completed") == "1分钟30秒"

    def test_hours_minutes_seconds(self):
        start = "2024-01-01T10:00:00Z"
        end = "2024-01-01T12:05:03Z"
        assert _format_duration(start, end, "completed") == "2小时5分钟3秒"

    def test_negative_duration(self):
        start = "2024-01-01T12:00:00Z"
        end = "2024-01-01T11:59:00Z"
        assert _format_duration(start, end, "completed") == ""


class TestParseIsoTime:
    """Tests for _parse_iso_time."""

    def test_standard_format(self):
        dt = _parse_iso_time("2024-01-01T12:00:00Z")
        assert dt is not None
        assert dt.year == 2024

    def test_with_z(self):
        dt = _parse_iso_time("2024-01-01T12:00:00Z")
        assert dt is not None
        assert dt.tzinfo == timezone.utc

    def test_without_z(self):
        dt = _parse_iso_time("2024-01-01T12:00:00")
        assert dt is not None
        assert dt.tzinfo == timezone.utc

    def test_with_milliseconds(self):
        dt = _parse_iso_time("2024-01-01T12:00:00.123Z")
        assert dt is not None
        assert dt.year == 2024

    def test_invalid_format(self):
        assert _parse_iso_time("not a date") is None

    def test_empty_string(self):
        assert _parse_iso_time("") is None


class TestZipinfoIsSymlink:
    """Tests for _zipinfo_is_symlink."""

    def test_normal_file(self):
        info = MagicMock()
        info.is_symlink = lambda: False
        assert _zipinfo_is_symlink(info) is False

    def test_symlink_with_hasattr(self):
        info = MagicMock()
        info.is_symlink = lambda: True
        assert _zipinfo_is_symlink(info) is True

    def test_unix_symlink_external_attr(self):
        info = MagicMock()
        del info.is_symlink  # remove is_symlink
        info.create_system = 3
        info.external_attr = 0xA0000000
        assert _zipinfo_is_symlink(info) is True


class TestSafeExtractZip:
    """Tests for _safe_extract_zip."""

    def _create_zip(self, files, compression=zipfile.ZIP_DEFLATED):
        """Create a zip file in memory.

        files: list of (name, content_bytes)
        """
        buf = io.BytesIO()
        with zipfile.ZipFile(buf, "w", compression=compression) as zf:
            for name, content in files:
                zf.writestr(name, content)
        buf.seek(0)
        return buf

    def test_normal_extract(self, tmp_path):
        zip_buf = self._create_zip([("hello.txt", b"Hello World")])
        zip_path = tmp_path / "test.zip"
        zip_path.write_bytes(zip_buf.read())
        extract_dir = tmp_path / "extracted"

        result = _safe_extract_zip(zip_path, extract_dir)
        assert result == extract_dir
        assert (extract_dir / "hello.txt").read_text() == "Hello World"

    def test_too_many_files(self, tmp_path):
        files = [(f"file{i}.txt", b"x") for i in range(MAX_FILE_COUNT + 1)]
        zip_buf = self._create_zip(files)
        zip_path = tmp_path / "test.zip"
        zip_path.write_bytes(zip_buf.read())

        with pytest.raises(OpenAaaSError, match="zip 炸弹"):
            _safe_extract_zip(zip_path, tmp_path / "extracted")

    def test_total_size_too_large(self, tmp_path):
        files = [("big.txt", b"x" * (MAX_TOTAL_EXTRACT_SIZE + 1))]
        zip_buf = self._create_zip(files)
        zip_path = tmp_path / "test.zip"
        zip_path.write_bytes(zip_buf.read())

        with pytest.raises(OpenAaaSError, match="总大小"):
            _safe_extract_zip(zip_path, tmp_path / "extracted")

    def test_single_file_too_large(self, tmp_path):
        files = [("huge.txt", b"x" * (MAX_SINGLE_FILE_SIZE + 1))]
        zip_buf = self._create_zip(files)
        zip_path = tmp_path / "test.zip"
        zip_path.write_bytes(zip_buf.read())

        with pytest.raises(OpenAaaSError, match="过大文件"):
            _safe_extract_zip(zip_path, tmp_path / "extracted")

    def test_compression_ratio_bomb(self, tmp_path):
        """Test zip bomb detection via compression ratio."""
        # Create a deterministic high-compression payload (all zeros)
        files = [("bomb.txt", b"\x00" * 500000)]
        zip_buf = self._create_zip(files, compression=zipfile.ZIP_DEFLATED)
        zip_path = tmp_path / "test.zip"
        zip_path.write_bytes(zip_buf.read())

        with pytest.raises(OpenAaaSError, match="压缩比"):
            _safe_extract_zip(zip_path, tmp_path / "extracted")

    def test_symlink_rejected(self, tmp_path):
        buf = io.BytesIO()
        with zipfile.ZipFile(buf, "w") as zf:
            info = zipfile.ZipInfo("link.txt")
            info.create_system = 3
            info.external_attr = 0xA0000000
            zf.writestr(info, b"target")
        buf.seek(0)

        zip_path = tmp_path / "test.zip"
        zip_path.write_bytes(buf.read())

        with pytest.raises(OpenAaaSError, match="符号链接"):
            _safe_extract_zip(zip_path, tmp_path / "extracted")

    def test_path_traversal_rejected(self, tmp_path):
        zip_buf = self._create_zip([("../../outside.txt", b"bad")])
        zip_path = tmp_path / "test.zip"
        zip_path.write_bytes(zip_buf.read())

        with pytest.raises(OpenAaaSError, match="非法路径|路径穿越"):
            _safe_extract_zip(zip_path, tmp_path / "extracted")


class TestGetDownloadDir:
    """Tests for _get_download_dir."""

    def test_returns_correct_path_format(self, monkeypatch, tmp_path):
        monkeypatch.chdir(tmp_path)
        result = _get_download_dir("task-123")
        assert result == tmp_path / ".OpenAaaS" / "downloads" / "task-123"

    def test_special_chars_sanitized(self, monkeypatch, tmp_path):
        monkeypatch.chdir(tmp_path)
        result = _get_download_dir("task/\\..")
        assert "_" in str(result)
        assert ".." not in str(result.name)

    def test_dot_replaced(self, monkeypatch, tmp_path):
        monkeypatch.chdir(tmp_path)
        result = _get_download_dir(".")
        assert result.name == "_"


class TestCheckFileInWorkingDir:
    """Tests for _check_file_in_working_dir."""

    def test_file_in_working_dir(self, monkeypatch, tmp_path):
        monkeypatch.chdir(tmp_path)
        file_path = tmp_path / "data.txt"
        file_path.write_text("test")
        # Should not raise
        _check_file_in_working_dir(file_path)

    def test_file_above_working_dir(self, monkeypatch, tmp_path):
        monkeypatch.chdir(tmp_path)
        outside_file = tmp_path.parent / "outside.txt"
        outside_file.write_text("secret")
        with pytest.raises(OpenAaaSError, match="只能上传当前工作目录"):
            _check_file_in_working_dir(outside_file)

    def test_symlink_check(self, monkeypatch, tmp_path):
        monkeypatch.chdir(tmp_path)
        real_file = tmp_path / "real.txt"
        real_file.write_text("test")
        symlink = tmp_path / "link.txt"
        symlink.symlink_to(real_file)
        # Symlinks that stay within working dir are resolved and checked
        _check_file_in_working_dir(symlink)
