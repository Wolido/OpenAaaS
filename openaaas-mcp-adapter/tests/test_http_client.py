"""Tests for http_client module."""

import json
from unittest.mock import MagicMock

import httpx
import pytest

from openaaas_mcp_adapter.http_client import (
    OpenAaaSError,
    _extract_error_message,
    _map_exception,
    safe_request,
)


class TestExtractErrorMessage:
    """Tests for _extract_error_message."""

    def test_json_error(self):
        """Test extracting error from JSON response."""
        response = MagicMock()
        response.text = json.dumps({"error": "something went wrong"})
        response.reason_phrase = "Bad Request"
        assert _extract_error_message(response) == "something went wrong"

    def test_json_message_field(self):
        """Test extracting 'message' field from JSON."""
        response = MagicMock()
        response.text = json.dumps({"message": "msg field"})
        response.reason_phrase = "OK"
        assert _extract_error_message(response) == "msg field"

    def test_plain_text_error(self):
        """Test extracting error from plain text response."""
        response = MagicMock()
        response.text = "plain text error"
        response.reason_phrase = "Bad Request"
        assert _extract_error_message(response) == "plain text error"

    def test_empty_response(self):
        """Test extracting error from empty response."""
        response = MagicMock()
        response.text = ""
        response.reason_phrase = "Internal Server Error"
        assert _extract_error_message(response) == "Internal Server Error"


class TestMapException:
    """Tests for _map_exception."""

    def test_connect_error(self):
        exc = httpx.ConnectError("connection failed")
        result = _map_exception(exc, "http://example.com")
        assert isinstance(result, OpenAaaSError)
        assert "连接失败" in str(result)

    def test_timeout_exception(self):
        exc = httpx.TimeoutException("timed out")
        result = _map_exception(exc, "http://example.com")
        assert isinstance(result, OpenAaaSError)
        assert "请求超时" in str(result)

    def test_network_error(self):
        exc = httpx.NetworkError("network down")
        result = _map_exception(exc, "http://example.com")
        assert isinstance(result, OpenAaaSError)
        assert "网络错误" in str(result)

    def test_invalid_url(self):
        exc = httpx.InvalidURL("bad url")
        result = _map_exception(exc, "http://example.com")
        assert isinstance(result, OpenAaaSError)
        assert "地址格式错误" in str(result)

    def test_http_status_error_401(self):
        response = MagicMock()
        response.status_code = 401
        response.text = ""
        response.reason_phrase = "Unauthorized"
        exc = httpx.HTTPStatusError("401", request=MagicMock(), response=response)
        result = _map_exception(exc, "http://example.com")
        assert isinstance(result, OpenAaaSError)
        assert "认证失败 (401)" in str(result)

    def test_http_status_error_403(self):
        response = MagicMock()
        response.status_code = 403
        response.text = ""
        response.reason_phrase = "Forbidden"
        exc = httpx.HTTPStatusError("403", request=MagicMock(), response=response)
        result = _map_exception(exc, "http://example.com")
        assert isinstance(result, OpenAaaSError)
        assert "权限不足 (403)" in str(result)

    def test_http_status_error_404(self):
        response = MagicMock()
        response.status_code = 404
        response.text = ""
        response.reason_phrase = "Not Found"
        exc = httpx.HTTPStatusError("404", request=MagicMock(), response=response)
        result = _map_exception(exc, "http://example.com")
        assert isinstance(result, OpenAaaSError)
        assert "资源不存在 (404)" in str(result)

    def test_http_status_error_409(self):
        response = MagicMock()
        response.status_code = 409
        response.text = json.dumps({"error": "conflict detail"})
        response.reason_phrase = "Conflict"
        exc = httpx.HTTPStatusError("409", request=MagicMock(), response=response)
        result = _map_exception(exc, "http://example.com")
        assert isinstance(result, OpenAaaSError)
        assert "冲突 (409)" in str(result)

    def test_http_status_error_400(self):
        response = MagicMock()
        response.status_code = 400
        response.text = json.dumps({"error": "bad request detail"})
        response.reason_phrase = "Bad Request"
        exc = httpx.HTTPStatusError("400", request=MagicMock(), response=response)
        result = _map_exception(exc, "http://example.com")
        assert isinstance(result, OpenAaaSError)
        assert "请求参数错误 (400)" in str(result)

    def test_http_status_error_500(self):
        response = MagicMock()
        response.status_code = 500
        response.text = "server crashed"
        response.reason_phrase = "Internal Server Error"
        exc = httpx.HTTPStatusError("500", request=MagicMock(), response=response)
        result = _map_exception(exc, "http://example.com")
        assert isinstance(result, OpenAaaSError)
        assert "请求失败 (HTTP 500)" in str(result)


class TestSafeRequest:
    """Tests for safe_request."""

    def _mock_client(self, monkeypatch, responses):
        """Helper to mock httpx.Client.request with a sequence of responses."""
        mock_client = MagicMock()
        mock_client.request.side_effect = responses
        mock_client.__enter__.return_value = mock_client
        mock_client.__exit__.return_value = False

        def mock_client_factory(*args, **kwargs):
            return mock_client

        monkeypatch.setattr("httpx.Client", mock_client_factory)
        return mock_client

    def _make_response(self, status_code, json_data=None, text=None, headers=None):
        response = MagicMock()
        response.status_code = status_code
        response.headers = headers or {}
        if json_data is not None:
            response.json.return_value = json_data
            response.text = json.dumps(json_data)
        elif text is not None:
            response.text = text
            response.json.side_effect = json.JSONDecodeError("msg", text, 0)
        else:
            response.text = ""
            response.json.return_value = {}
        response.content = response.text.encode("utf-8")
        response.raise_for_status = MagicMock()
        return response

    def test_get_success(self, monkeypatch):
        """Test GET request success."""
        response = self._make_response(200, json_data={"result": "ok"})
        self._mock_client(monkeypatch, [response])

        result = safe_request("GET", "http://example.com/api")
        assert result == {"result": "ok"}

    def test_post_success(self, monkeypatch):
        """Test POST request success."""
        response = self._make_response(201, json_data={"id": "123"})
        self._mock_client(monkeypatch, [response])

        result = safe_request("POST", "http://example.com/api", data={"key": "value"})
        assert result == {"id": "123"}

    def test_post_with_file_upload(self, monkeypatch):
        """Test POST with file upload."""
        response = self._make_response(200, json_data={"uploaded": True})
        mock_client = self._mock_client(monkeypatch, [response])

        files = [("files", ("test.txt", b"content", "text/plain"))]
        result = safe_request(
            "POST", "http://example.com/upload", data={"name": "test"}, files=files
        )
        assert result == {"uploaded": True}
        # Verify multipart body was built manually and sent via content
        call_kwargs = mock_client.request.call_args[1]
        assert "content" in call_kwargs
        assert "data" not in call_kwargs
        assert "files" not in call_kwargs
        req_headers = call_kwargs.get("headers", {})
        content_type = req_headers.get("Content-Type", "")
        assert "multipart/form-data" in content_type
        assert "boundary=----openaaas-" in content_type
        body = call_kwargs["content"]
        body_str = body.decode("utf-8")
        assert 'Content-Disposition: form-data; name="name"' in body_str
        assert 'Content-Disposition: form-data; name="files"; filename="test.txt"' in body_str
        assert "Content-Type: text/plain" in body_str

    def test_post_multipart_no_files(self, monkeypatch):
        """Test POST multipart with empty files list still sends multipart."""
        response = self._make_response(200, json_data={"ok": True})
        mock_client = self._mock_client(monkeypatch, [response])

        result = safe_request(
            "POST", "http://example.com/upload", data={"name": "test"}, files=[]
        )
        assert result == {"ok": True}
        call_kwargs = mock_client.request.call_args[1]
        req_headers = call_kwargs.get("headers", {})
        content_type = req_headers.get("Content-Type", "")
        assert "multipart/form-data" in content_type
        body = call_kwargs["content"]
        body_str = body.decode("utf-8")
        assert 'Content-Disposition: form-data; name="name"' in body_str
        assert "test" in body_str
        assert "--" in body_str

    def test_post_multipart_multiple_files(self, monkeypatch):
        """Test POST with multiple files."""
        response = self._make_response(200, json_data={"uploaded": True})
        mock_client = self._mock_client(monkeypatch, [response])

        files = [
            ("files", ("a.txt", b"content_a", "text/plain")),
            ("files", ("b.txt", b"content_b", "text/plain")),
        ]
        result = safe_request(
            "POST", "http://example.com/upload", data={"name": "multi"}, files=files
        )
        assert result == {"uploaded": True}
        call_kwargs = mock_client.request.call_args[1]
        body = call_kwargs["content"]
        body_str = body.decode("utf-8")
        assert 'filename="a.txt"' in body_str
        assert 'filename="b.txt"' in body_str
        assert "content_a" in body_str
        assert "content_b" in body_str

    def test_post_multipart_special_chars_in_filename(self, monkeypatch):
        """Test POST with filename containing quotes and backslashes."""
        response = self._make_response(200, json_data={"uploaded": True})
        mock_client = self._mock_client(monkeypatch, [response])

        files = [("files", ('say "hello".txt', b"content", "text/plain"))]
        result = safe_request(
            "POST", "http://example.com/upload", data={"name": "special"}, files=files
        )
        assert result == {"uploaded": True}
        call_kwargs = mock_client.request.call_args[1]
        body = call_kwargs["content"]
        body_str = body.decode("utf-8")
        assert 'filename="say \\"hello\\".txt"' in body_str

    def test_post_multipart_backslash_in_filename(self, monkeypatch):
        """Test POST with filename containing backslashes."""
        response = self._make_response(200, json_data={"uploaded": True})
        mock_client = self._mock_client(monkeypatch, [response])

        files = [("files", ("path\\\\to\\\\file.txt", b"content", "text/plain"))]
        result = safe_request(
            "POST", "http://example.com/upload", data={"name": "backslash"}, files=files
        )
        assert result == {"uploaded": True}
        call_kwargs = mock_client.request.call_args[1]
        body = call_kwargs["content"]
        body_str = body.decode("utf-8")
        assert 'filename="path\\\\\\\\to\\\\\\\\file.txt"' in body_str

    def test_redirect_handling(self, monkeypatch):
        """Test following a redirect."""
        redirect = self._make_response(302, headers={"Location": "http://example.com/new"})
        final = self._make_response(200, json_data={"result": "ok"})
        self._mock_client(monkeypatch, [redirect, final])

        result = safe_request("GET", "http://example.com/api")
        assert result == {"result": "ok"}

    def test_redirect_exceeded(self, monkeypatch):
        """Test redirect limit exceeded raises OpenAaaSError."""
        redirect = self._make_response(302, headers={"Location": "http://example.com/new"})
        self._mock_client(monkeypatch, [redirect, redirect, redirect, redirect])

        with pytest.raises(OpenAaaSError, match="重定向"):
            safe_request("GET", "http://example.com/api", max_redirects=2)

    def test_4xx_error(self, monkeypatch):
        """Test 4xx error raises OpenAaaSError."""
        response = self._make_response(404, text="not found")
        response.raise_for_status.side_effect = httpx.HTTPStatusError(
            "404", request=MagicMock(), response=response
        )
        self._mock_client(monkeypatch, [response])

        with pytest.raises(OpenAaaSError, match="资源不存在"):
            safe_request("GET", "http://example.com/api")

    def test_json_parse_error(self, monkeypatch):
        """Test JSON parse error raises OpenAaaSError."""
        response = self._make_response(200, text="not json")
        self._mock_client(monkeypatch, [response])

        with pytest.raises(OpenAaaSError, match="JSON 解析错误"):
            safe_request("GET", "http://example.com/api")

