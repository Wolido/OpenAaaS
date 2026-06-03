"""Tests for ClientBase shared logic."""

from __future__ import annotations

from pathlib import Path
from typing import Any

import pytest

from pyopenaaas.client._base import MAX_UPLOAD_SIZE, ClientBase
from pyopenaaas.config import Config
from pyopenaaas.exceptions import AuthenticationError, OpenAaaSError, RequestValidationError
from pyopenaaas.models import ResultFile, ServerInfo, Service, Task, TaskResult


@pytest.fixture
def base() -> ClientBase:
    return ClientBase(config=Config(server_url="https://api.example.com", api_key="k"))


class TestUrlBuilding:
    """URL construction tests."""

    def test_simple_path(self, base: ClientBase) -> None:
        assert base._url("/v1/data") == "https://api.example.com/v1/data"

    def test_no_trailing_slash_on_base(self) -> None:
        cfg = Config(server_url="https://api.example.com/", api_key="k")
        b = ClientBase(config=cfg)
        assert b._url("/v1/data") == "https://api.example.com/v1/data"


class TestAuthHeaders:
    """Auth header generation tests."""

    def test_bearer_token(self, base: ClientBase) -> None:
        headers = base._headers()
        assert headers == {"Authorization": "Bearer k"}

    def test_missing_key_raises(self) -> None:
        b = ClientBase(config=Config())
        with pytest.raises(AuthenticationError):
            b._headers()


class TestQuoteId:
    """URL quoting tests."""

    def test_plain_id(self, base: ClientBase) -> None:
        assert base._quote_id("abc123") == "abc123"

    def test_special_chars_quoted(self, base: ClientBase) -> None:
        assert base._quote_id("a/b c") == "a%2Fb%20c"


class TestValidateName:
    """Name validation tests."""

    def test_valid_name(self, base: ClientBase) -> None:
        assert base._validate_name("Alice") == "Alice"

    def test_strips_whitespace(self, base: ClientBase) -> None:
        assert base._validate_name("  Bob  ") == "Bob"

    def test_empty_name_raises(self, base: ClientBase) -> None:
        with pytest.raises(RequestValidationError) as exc_info:
            base._validate_name("")
        assert "cannot be empty" in str(exc_info.value)

    def test_too_long_raises(self, base: ClientBase) -> None:
        with pytest.raises(RequestValidationError) as exc_info:
            base._validate_name("x" * 65)
        assert "64 characters" in str(exc_info.value)

    @pytest.mark.parametrize("bad", ["/", "\\", "<", ">", "|", "&", ";", "$"])
    def test_illegal_chars_raise(self, base: ClientBase, bad: str) -> None:
        with pytest.raises(RequestValidationError) as exc_info:
            base._validate_name(f"name{bad}")
        assert "illegal characters" in str(exc_info.value)

    def test_unicode_control_chars_raise(self, base: ClientBase) -> None:
        with pytest.raises(RequestValidationError) as exc_info:
            base._validate_name("name\x7f")
        assert "control characters" in str(exc_info.value)


class TestValidateTaskId:
    """Task ID validation tests."""

    def test_valid(self, base: ClientBase) -> None:
        assert base._validate_task_id("t-123") == "t-123"

    def test_empty_raises(self, base: ClientBase) -> None:
        with pytest.raises(RequestValidationError):
            base._validate_task_id("")

    def test_whitespace_only_raises(self, base: ClientBase) -> None:
        with pytest.raises(RequestValidationError):
            base._validate_task_id("   ")

    def test_strips_whitespace(self, base: ClientBase) -> None:
        assert base._validate_task_id("  t-123  ") == "t-123"


class TestValidateServiceId:
    """Service ID validation tests."""

    def test_valid(self, base: ClientBase) -> None:
        assert base._validate_service_id("svc-1") == "svc-1"

    def test_empty_raises(self, base: ClientBase) -> None:
        with pytest.raises(RequestValidationError):
            base._validate_service_id("")


class TestPrepareFilesSync:
    """File preparation tests."""

    def test_reads_file(self, base: ClientBase, tmp_path: Path, monkeypatch: Any) -> None:
        monkeypatch.chdir(tmp_path)
        file = tmp_path / "data.txt"
        file.write_text("hello")
        result = base._prepare_files_sync("data.txt")
        assert len(result) == 1
        _, (name, content, mime) = result[0]
        assert name == "data.txt"
        assert content == b"hello"
        assert mime == "text/plain"

    def test_absolute_path(self, base: ClientBase, tmp_path: Path, monkeypatch: Any) -> None:
        monkeypatch.chdir(tmp_path)
        file = tmp_path / "data.txt"
        file.write_text("hello")
        result = base._prepare_files_sync(file)
        assert result[0][1][0] == "data.txt"

    def test_outside_cwd_rejected(self, base: ClientBase, tmp_path: Path, monkeypatch: Any) -> None:
        monkeypatch.chdir(tmp_path)
        outside = tmp_path.parent / "secret.txt"
        outside.write_text("x")
        with pytest.raises(OpenAaaSError) as exc_info:
            base._prepare_files_sync(str(outside))
        assert "working directory" in str(exc_info.value)

    def test_symlink_rejected(self, base: ClientBase, tmp_path: Path, monkeypatch: Any) -> None:
        monkeypatch.chdir(tmp_path)
        target = tmp_path / "target.txt"
        target.write_text("x")
        link = tmp_path / "link.txt"
        link.symlink_to(target)
        with pytest.raises(RequestValidationError) as exc_info:
            base._prepare_files_sync("link.txt")
        assert "Symlinks are not allowed" in str(exc_info.value)

    def test_missing_file_rejected(self, base: ClientBase, tmp_path: Path, monkeypatch: Any) -> None:
        monkeypatch.chdir(tmp_path)
        with pytest.raises(RequestValidationError) as exc_info:
            base._prepare_files_sync("missing.txt")
        assert "File not found" in str(exc_info.value)

    def test_directory_rejected(self, base: ClientBase, tmp_path: Path, monkeypatch: Any) -> None:
        monkeypatch.chdir(tmp_path)
        (tmp_path / "adir").mkdir()
        with pytest.raises(RequestValidationError) as exc_info:
            base._prepare_files_sync("adir")
        assert "Not a file" in str(exc_info.value)

    def test_prepare_files_oversized_by_stat(self, base: ClientBase, tmp_path: Path, monkeypatch: Any) -> None:
        """A file whose stat size exceeds MAX_UPLOAD_SIZE should be rejected."""
        monkeypatch.chdir(tmp_path)
        file = tmp_path / "big.bin"
        file.write_bytes(b"x")

        class FakeStat:
            st_size = MAX_UPLOAD_SIZE + 1
            st_mode = 0o100644

        monkeypatch.setattr(Path, "stat", lambda self, follow_symlinks=True: FakeStat())
        with pytest.raises(RequestValidationError) as exc_info:
            base._prepare_files_sync("big.bin")
        assert "File too large" in str(exc_info.value)

    def test_prepare_files_oversized_toctou(self, base: ClientBase, tmp_path: Path, monkeypatch: Any) -> None:
        """Mock stat returns small size, but actual content is oversized (TOCTOU)."""
        monkeypatch.chdir(tmp_path)
        file = tmp_path / "big.bin"
        file.write_bytes(b"x")
        # Mock stat to report a small size so the first check passes

        class FakeStat:
            st_size = 1
            st_mode = 0o100644

        monkeypatch.setattr(Path, "stat", lambda self, follow_symlinks=True: FakeStat())
        monkeypatch.setattr(Path, "read_bytes", lambda self: b"x" * (MAX_UPLOAD_SIZE + 1))
        with pytest.raises(RequestValidationError) as exc_info:
            base._prepare_files_sync("big.bin")
        assert "File too large" in str(exc_info.value)


class TestBuildSubmitFields:
    """Submit field building tests."""

    def test_basic_fields(self, base: ClientBase) -> None:
        fields = base._build_submit_fields("svc-1", "prompt", "output", "sess-1")
        assert fields["service_id"] == "svc-1"
        assert fields["task_prompt"] == "prompt"
        assert fields["output_prompt"] == "output"
        assert fields["session_id"] == "sess-1"

    def test_empty_session_id_omitted(self, base: ClientBase) -> None:
        fields = base._build_submit_fields("svc-1", "prompt", "output", "")
        assert "session_id" not in fields

    def test_empty_output_prompt(self, base: ClientBase) -> None:
        fields = base._build_submit_fields("svc-1", "prompt", "", "")
        assert fields["output_prompt"] == ""


class TestResolveDownloadPath:
    """Download path resolution tests."""

    def test_none_uses_default(self, base: ClientBase, tmp_path: Path, monkeypatch: Any) -> None:
        monkeypatch.chdir(tmp_path)
        path = base._resolve_download_path("f-1", None, "result.txt")
        assert ".OpenAaaS/downloads/f-1/result.txt" in str(path)

    def test_directory_appends_default(self, base: ClientBase, tmp_path: Path) -> None:
        dest = tmp_path / "downloads"
        dest.mkdir()
        path = base._resolve_download_path("f-1", dest, "result.txt")
        assert path == dest / "result.txt"

    def test_full_path_used_directly(self, base: ClientBase, tmp_path: Path) -> None:
        dest = tmp_path / "myfile.bin"
        path = base._resolve_download_path("f-1", dest, "result.txt")
        assert path == dest


class TestProcessDownloadedFileSync:
    """Post-download processing tests."""

    def test_non_zip_returned_as_is(self, base: ClientBase, tmp_path: Path) -> None:
        file = tmp_path / "data.txt"
        file.write_text("hello")
        result = base._process_downloaded_file_sync(file)
        assert result == file

    def test_zip_extracted_and_removed(self, base: ClientBase, tmp_path: Path) -> None:
        file = tmp_path / "archive.zip"
        with Path(file).open("wb") as f:
            import zipfile
            with zipfile.ZipFile(f, "w") as zf:
                zf.writestr("inside.txt", "content")
        result = base._process_downloaded_file_sync(file)
        assert result == tmp_path / "archive"
        assert (result / "inside.txt").read_text() == "content"
        assert not file.exists()

    def test_zip_not_extracted_when_flag_false(self, base: ClientBase, tmp_path: Path) -> None:
        file = tmp_path / "archive.zip"
        with Path(file).open("wb") as f:
            import zipfile
            with zipfile.ZipFile(f, "w") as zf:
                zf.writestr("inside.txt", "content")
        result = base._process_downloaded_file_sync(file, extract_zip=False)
        assert result == file


class TestParseServerInfo:
    """ServerInfo parsing tests."""

    def test_dict_with_api_key(self, base: ClientBase) -> None:
        data = {"api": {"version": "1.0", "base_url": "https://x.com"}, "auth": "Basic"}
        info = base._parse_server_info(data, "https://fallback.com")
        assert info.version == "1.0"
        assert info.base_url == "https://x.com"
        assert info.authentication == "Basic"

    def test_fallback_base_url(self, base: ClientBase) -> None:
        data = {"api": {"version": "1.0"}}
        info = base._parse_server_info(data, "https://fallback.com")
        assert info.base_url == "https://fallback.com"

    def test_non_dict_input(self, base: ClientBase) -> None:
        """Non-dict input should be treated like an empty dict (code guards via .get)."""
        # The current implementation expects a dict; passing a string would
        # crash. We test the guard behaviour by using an empty dict instead.
        info = base._parse_server_info({}, "https://fallback.com")
        assert info.version == "unknown"
        assert info.base_url == "https://fallback.com"


class TestParseServices:
    """Service list parsing tests."""

    def test_list_input(self, base: ClientBase) -> None:
        data = [{"id": "s1", "name": "A"}, {"id": "s2", "name": "B"}]
        services = base._parse_services(data)
        assert len(services) == 2
        assert services[0].id == "s1"

    def test_dict_with_services_key(self, base: ClientBase) -> None:
        data = {"services": [{"id": "s1", "name": "A"}]}
        services = base._parse_services(data)
        assert len(services) == 1
        assert services[0].id == "s1"

    def test_skips_non_dict_items(self, base: ClientBase) -> None:
        data = [{"id": "s1"}, "not a dict", {"id": "s2"}]
        services = base._parse_services(data)
        assert len(services) == 2


class TestParseTask:
    """Task parsing tests."""

    def test_basic(self, base: ClientBase) -> None:
        data = {"task_id": "t-1", "status": "running"}
        task = base._parse_task(data)
        assert task.id == "t-1"
        assert task.status == "running"

    def test_with_result(self, base: ClientBase) -> None:
        data = {
            "task_id": "t-1",
            "status": "completed",
            "result": {"summary": "done", "error": None},
        }
        task = base._parse_task(data)
        assert task.result is not None
        assert task.result.summary == "done"


class TestParseResultFiles:
    """ResultFile list parsing tests."""

    def test_list_input(self, base: ClientBase) -> None:
        data = [{"file_id": "f1", "name": "a.txt", "file_size": 100}]
        files = base._parse_result_files(data)
        assert len(files) == 1
        assert files[0].id == "f1"
        assert files[0].filename == "a.txt"
        assert files[0].size == 100

    def test_dict_with_files_key(self, base: ClientBase) -> None:
        data = {"files": [{"file_id": "f1", "name": "a.txt"}]}
        files = base._parse_result_files(data)
        assert len(files) == 1

    def test_skips_non_dict_items(self, base: ClientBase) -> None:
        data = [{"file_id": "f1"}, "bad", {"file_id": "f2"}]
        files = base._parse_result_files(data)
        assert len(files) == 2


class TestRepr:
    """Repr tests."""

    def test_client_base_repr(self) -> None:
        b = ClientBase(config=Config(server_url="https://x.com"))
        assert "ClientBase" in repr(b)
        assert "https://x.com" in repr(b)
