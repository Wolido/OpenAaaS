"""Tests for HTTP transport layer."""

from __future__ import annotations

import json
from typing import Any
from unittest.mock import patch

import httpx
import pytest
import respx
from respx import MockRouter

from pyopenaaas._http import (
    _build_multipart_body,
    _extract_error_message,
    _map_exception,
    safe_request,
    stream_download,
)
from pyopenaaas.exceptions import (
    AuthenticationError,
    ConflictError,
    NetworkError,
    NotFoundError,
    OpenAaaSError,
    RequestTimeoutError,
    RequestValidationError,
)


class TestSafeRequest:
    """Tests for safe_request."""

    @respx.mock
    def test_get_success_json(self) -> None:
        route = respx.get("https://api.example.com/v1/data").mock(
            return_value=httpx.Response(200, json={"ok": True})
        )
        result = safe_request("GET", "https://api.example.com/v1/data")
        assert result == {"ok": True}
        assert route.called

    @respx.mock
    def test_post_success_json(self) -> None:
        route = respx.post("https://api.example.com/v1/data").mock(
            return_value=httpx.Response(201, json={"id": "123"})
        )
        result = safe_request(
            "POST", "https://api.example.com/v1/data", data={"name": "test"}
        )
        assert result == {"id": "123"}
        assert route.called
        assert json.loads(route.calls.last.request.content) == {"name": "test"}

    @respx.mock
    def test_safe_request_does_not_override_existing_content_type(self) -> None:
        route = respx.post("https://api.example.com/v1/data").mock(
            return_value=httpx.Response(200, json={"ok": True})
        )
        result = safe_request(
            "POST",
            "https://api.example.com/v1/data",
            data={"name": "test"},
            headers={"Content-Type": "application/vnd.api+json"},
        )
        assert result == {"ok": True}
        assert route.called
        req = route.calls.last.request
        assert req.headers["Content-Type"] == "application/vnd.api+json"

    @respx.mock
    def test_204_empty_response(self) -> None:
        route = respx.delete("https://api.example.com/v1/data").mock(
            return_value=httpx.Response(204)
        )
        result = safe_request("DELETE", "https://api.example.com/v1/data")
        assert result == {}
        assert route.called

    @respx.mock
    def test_301_redirect_followed(self) -> None:
        """3xx should be followed up to max_redirects times."""
        respx.get("https://api.example.com/v1/old").mock(
            return_value=httpx.Response(301, headers={"Location": "/v1/new"})
        )
        route = respx.get("https://api.example.com/v1/new").mock(
            return_value=httpx.Response(200, json={"new": True})
        )
        result = safe_request("GET", "https://api.example.com/v1/old")
        assert result == {"new": True}
        assert route.called

    @respx.mock
    def test_401_raises_authentication_error(self) -> None:
        respx.get("https://api.example.com/v1/protected").mock(
            return_value=httpx.Response(401, json={"error": "bad key"})
        )
        with pytest.raises(AuthenticationError) as exc_info:
            safe_request("GET", "https://api.example.com/v1/protected")
        assert "401" in str(exc_info.value)
        assert "bad key" in str(exc_info.value)

    @respx.mock
    def test_403_raises_authentication_error(self) -> None:
        respx.get("https://api.example.com/v1/protected").mock(
            return_value=httpx.Response(403, text="Forbidden")
        )
        with pytest.raises(AuthenticationError) as exc_info:
            safe_request("GET", "https://api.example.com/v1/protected")
        assert "403" in str(exc_info.value)

    @respx.mock
    def test_404_raises_not_found_error(self) -> None:
        respx.get("https://api.example.com/v1/missing").mock(
            return_value=httpx.Response(404, json={"message": "not here"})
        )
        with pytest.raises(NotFoundError) as exc_info:
            safe_request("GET", "https://api.example.com/v1/missing")
        assert "404" in str(exc_info.value)

    @respx.mock
    def test_409_raises_conflict_error(self) -> None:
        respx.post("https://api.example.com/v1/resource").mock(
            return_value=httpx.Response(409, json={"error": "exists"})
        )
        with pytest.raises(ConflictError) as exc_info:
            safe_request("POST", "https://api.example.com/v1/resource")
        assert "409" in str(exc_info.value)

    @respx.mock
    def test_400_raises_request_validation_error(self) -> None:
        respx.post("https://api.example.com/v1/resource").mock(
            return_value=httpx.Response(400, json={"error": "invalid"})
        )
        with pytest.raises(RequestValidationError) as exc_info:
            safe_request("POST", "https://api.example.com/v1/resource")
        assert "400" in str(exc_info.value)

    @respx.mock
    def test_500_raises_openaaas_error(self) -> None:
        respx.get("https://api.example.com/v1/boom").mock(
            return_value=httpx.Response(500, text="Internal Server Error")
        )
        with pytest.raises(OpenAaaSError) as exc_info:
            safe_request("GET", "https://api.example.com/v1/boom")
        assert "500" in str(exc_info.value)

    def test_connect_error_maps_to_network_error(self) -> None:
        with patch(
            "httpx.Client.request",
            side_effect=httpx.ConnectError("Connection refused"),
        ):
            with pytest.raises(NetworkError) as exc_info:
                safe_request("GET", "https://api.example.com/v1/data")
            assert "Connection failed" in str(exc_info.value)

    def test_timeout_maps_to_request_timeout_error(self) -> None:
        with patch(
            "httpx.Client.request",
            side_effect=httpx.TimeoutException("Timed out"),
        ):
            with pytest.raises(RequestTimeoutError) as exc_info:
                safe_request("GET", "https://api.example.com/v1/data")
            assert "timed out" in str(exc_info.value).lower()

    def test_invalid_url_maps_to_network_error(self) -> None:
        with patch(
            "httpx.Client.request",
            side_effect=httpx.InvalidURL("bad url"),
        ):
            with pytest.raises(NetworkError) as exc_info:
                safe_request("GET", "not-a-url")
            assert "Invalid URL" in str(exc_info.value)

    @respx.mock
    def test_cross_host_redirect_strips_auth(self) -> None:
        """Auth header should be stripped when redirecting to a different host."""
        respx.get("https://a.com/v1/data").mock(
            return_value=httpx.Response(302, headers={"Location": "https://b.com/v1/data"})
        )
        route = respx.get("https://b.com/v1/data").mock(
            return_value=httpx.Response(200, json={"ok": True})
        )
        safe_request(
            "GET",
            "https://a.com/v1/data",
            headers={"Authorization": "Bearer secret"},
        )
        assert route.called
        req = route.calls.last.request
        assert "Authorization" not in req.headers

    @respx.mock
    def test_too_many_redirects_raises(self) -> None:
        respx.get("https://api.example.com/v1/data").mock(
            return_value=httpx.Response(302, headers={"Location": "/v1/data"})
        )
        with pytest.raises(OpenAaaSError) as exc_info:
            safe_request(
                "GET", "https://api.example.com/v1/data", max_redirects=1
            )
        assert "Too many redirects" in str(exc_info.value)

    @respx.mock
    def test_exception_chaining_from_jsondecode(self) -> None:
        respx.get("https://api.example.com/v1/data").mock(
            return_value=httpx.Response(200, text="not json")
        )
        with pytest.raises(OpenAaaSError) as exc_info:
            safe_request("GET", "https://api.example.com/v1/data")
        assert exc_info.value.__cause__ is not None
        assert isinstance(exc_info.value.__cause__, json.JSONDecodeError)


class TestBuildMultipartBody:
    """Tests for _build_multipart_body."""

    def test_contains_boundary(self) -> None:
        body, boundary = _build_multipart_body({"key": "value"}, [])
        assert boundary.startswith("----pyopenaaas-")
        assert boundary.encode() in body

    def test_form_fields_present(self) -> None:
        body, _ = _build_multipart_body(
            {"service_id": "s1", "task_prompt": "hello"},
            [],
        )
        text = body.decode("utf-8")
        assert 'name="service_id"' in text
        assert "s1" in text
        assert 'name="task_prompt"' in text
        assert "hello" in text

    def test_file_fields_present(self) -> None:
        body, _ = _build_multipart_body(
            {},
            [("files", ("test.txt", b"content", "text/plain"))],
        )
        text = body.decode("utf-8")
        assert 'filename="test.txt"' in text
        assert "Content-Type: text/plain" in text
        assert b"content".decode() in text

    def test_escapes_quotes(self) -> None:
        body, _ = _build_multipart_body(
            {'key"with\\quotes': "val"},
            [],
        )
        text = body.decode("utf-8")
        assert 'key\\"with\\\\quotes' in text


class TestStreamDownload:
    """Tests for stream_download."""

    @respx.mock
    def test_yields_chunks(self) -> None:
        respx.get("https://api.example.com/v1/file").mock(
            return_value=httpx.Response(200, content=b"abcd" * 1024)
        )
        chunks = list(
            stream_download(
                "https://api.example.com/v1/file", chunk_size=512
            )
        )
        assert len(chunks) > 1
        assert b"".join(chunks) == b"abcd" * 1024

    def test_404_raises_not_found(self) -> None:
        """stream_download uses streaming client; respx streaming response doesn't
        allow .text access after raise_for_status. We mock at httpx level."""
        response = httpx.Response(404, content=b"Not Found")
        request = httpx.Request("GET", "https://api.example.com/v1/file")
        error = httpx.HTTPStatusError("404", request=request, response=response)

        with patch("httpx.Client.stream", side_effect=error):
            with pytest.raises(NotFoundError):
                next(stream_download("https://api.example.com/v1/file"))

    def test_connect_error_maps_to_network_error(self) -> None:
        with patch(
            "httpx.Client.stream",
            side_effect=httpx.ConnectError("refused"),
        ):
            with pytest.raises(NetworkError) as exc_info:
                next(stream_download("https://api.example.com/v1/file"))
            assert "Connection failed" in str(exc_info.value)


class TestMapException:
    """Direct tests for _map_exception."""

    def test_connect_error(self) -> None:
        exc = _map_exception(httpx.ConnectError("x"), "https://a.com")
        assert isinstance(exc, NetworkError)

    def test_timeout_exception(self) -> None:
        exc = _map_exception(httpx.TimeoutException("x"), "https://a.com")
        assert isinstance(exc, RequestTimeoutError)

    def test_invalid_url(self) -> None:
        exc = _map_exception(httpx.InvalidURL("x"), "bad")
        assert isinstance(exc, NetworkError)
        assert "Invalid URL" in str(exc)

    def test_generic_exception(self) -> None:
        exc = _map_exception(ValueError("oops"), "https://a.com")
        assert isinstance(exc, OpenAaaSError)
        assert "Request failed" in str(exc)

    @respx.mock
    def test_http_status_error_401(self) -> None:
        response = httpx.Response(401, text="Unauthorized")
        httpx_error = httpx.HTTPStatusError(
            "401", request=httpx.Request("GET", "https://a.com"), response=response
        )
        exc = _map_exception(httpx_error, "https://a.com")
        assert isinstance(exc, AuthenticationError)

    @respx.mock
    def test_http_status_error_404(self) -> None:
        response = httpx.Response(404, json={"message": "missing"})
        httpx_error = httpx.HTTPStatusError(
            "404", request=httpx.Request("GET", "https://a.com"), response=response
        )
        exc = _map_exception(httpx_error, "https://a.com")
        assert isinstance(exc, NotFoundError)
        assert "missing" in str(exc)

    @respx.mock
    def test_http_status_error_409(self) -> None:
        response = httpx.Response(409, text="Conflict")
        httpx_error = httpx.HTTPStatusError(
            "409", request=httpx.Request("GET", "https://a.com"), response=response
        )
        exc = _map_exception(httpx_error, "https://a.com")
        assert isinstance(exc, ConflictError)

    @respx.mock
    def test_http_status_error_400(self) -> None:
        response = httpx.Response(400, json={"error": "bad"})
        httpx_error = httpx.HTTPStatusError(
            "400", request=httpx.Request("GET", "https://a.com"), response=response
        )
        exc = _map_exception(httpx_error, "https://a.com")
        assert isinstance(exc, RequestValidationError)


class TestExtractErrorMessage:
    """Direct tests for _extract_error_message."""

    @respx.mock
    def test_json_dict_with_error(self) -> None:
        response = httpx.Response(400, json={"error": "bad request"})
        assert _extract_error_message(response) == "bad request"

    @respx.mock
    def test_json_dict_with_message(self) -> None:
        response = httpx.Response(400, json={"message": "oops"})
        assert _extract_error_message(response) == "oops"

    @respx.mock
    def test_plain_text_fallback(self) -> None:
        response = httpx.Response(400, text="plain error")
        assert _extract_error_message(response) == "plain error"

    @respx.mock
    def test_empty_uses_reason_phrase(self) -> None:
        response = httpx.Response(400, text="")
        assert _extract_error_message(response) == "Bad Request"
