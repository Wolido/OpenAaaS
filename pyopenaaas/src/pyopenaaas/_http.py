"""HTTP transport layer (synchronous).

Core logic reused from openaaas_mcp_adapter/http_client.py with
SDK-specific exception subclasses.
"""

from __future__ import annotations

import json
import uuid
from typing import Any, Iterator

import httpx

from .exceptions import (
    AuthenticationError,
    ConflictError,
    NetworkError,
    NotFoundError,
    OpenAaaSError,
    RequestTimeoutError,
    RequestValidationError,
)

DEFAULT_TIMEOUT = 30.0
UPLOAD_TIMEOUT = 60.0
DOWNLOAD_TIMEOUT = 60.0


def _extract_error_message(response: httpx.Response) -> str:
    """Extract a human-readable error message from an HTTP response."""
    text = response.text or response.reason_phrase
    try:
        data = json.loads(text)
        if isinstance(data, dict):
            return str(data.get("error") or data.get("message") or text)
    except json.JSONDecodeError:
        pass
    return text


def _escape_quotes(value: str) -> str:
    """Escape backslashes and double quotes for multipart headers."""
    return value.replace("\\", "\\\\").replace('"', '\\"')


def _build_multipart_body(
    data: dict[str, str],
    files: list[tuple[str, tuple[str, bytes, str]]],
) -> tuple[bytes, str]:
    """Manually build a multipart/form-data request body.

    Returns:
        (body_bytes, boundary_string)
    """
    boundary = f"----pyopenaaas-{uuid.uuid4().hex}"
    lines: list[bytes] = []

    def add_field(key: str, value: str) -> None:
        escaped_key = _escape_quotes(key)
        lines.append(f"--{boundary}\r\n".encode("utf-8"))
        lines.append(
            f'Content-Disposition: form-data; name="{escaped_key}"\r\n\r\n'.encode(
                "utf-8"
            )
        )
        lines.append(value.encode("utf-8"))
        lines.append(b"\r\n")

    for key, value in data.items():
        add_field(key, value)

    for field_name, (filename, payload, content_type) in files:
        escaped_field = _escape_quotes(field_name)
        escaped_filename = _escape_quotes(filename)
        lines.append(f"--{boundary}\r\n".encode("utf-8"))
        lines.append(
            (
                f'Content-Disposition: form-data; name="{escaped_field}"; '
                f'filename="{escaped_filename}"\r\n'
            ).encode("utf-8")
        )
        lines.append(f"Content-Type: {content_type}\r\n\r\n".encode("utf-8"))
        lines.append(payload)
        lines.append(b"\r\n")

    lines.append(f"--{boundary}--\r\n".encode("utf-8"))
    body = b"".join(lines)
    return body, boundary


def _map_exception(exc: Exception, url: str) -> OpenAaaSError:
    """Map a low-level exception to an SDK exception subclass."""
    if isinstance(exc, httpx.ConnectError):
        return NetworkError(f"Connection failed: unable to reach {url}")
    if isinstance(exc, httpx.TimeoutException):
        return RequestTimeoutError(f"Request timed out: {url}")
    if isinstance(exc, httpx.NetworkError):
        return NetworkError(f"Network error: unable to reach {url}")
    if isinstance(exc, httpx.InvalidURL):
        return NetworkError(f"Invalid URL: {url}")
    if isinstance(exc, httpx.HTTPStatusError):
        status = exc.response.status_code
        error_msg = _extract_error_message(exc.response)
        if status == 401:
            return AuthenticationError(
                f"Authentication failed (401): invalid API key — {error_msg}"
            )
        if status == 403:
            return AuthenticationError(
                f"Permission denied (403): {error_msg}"
            )
        if status == 404:
            return NotFoundError(f"Not found (404): {error_msg}")
        if status == 409:
            return ConflictError(f"Conflict (409): {error_msg}")
        if status == 400:
            return RequestValidationError(f"Validation error (400): {error_msg}")
        return OpenAaaSError(f"HTTP {status}: {error_msg}")
    return OpenAaaSError(f"Request failed: {exc}")


def safe_request(
    method: str,
    url: str,
    headers: dict[str, str] | None = None,
    data: dict[str, Any] | None = None,
    files: list[tuple[str, tuple[str, bytes, str]]] | None = None,
    timeout: float | None = None,
    max_redirects: int = 3,
) -> Any:
    """Send an HTTP request and return parsed JSON.

    Manually follows 3xx redirects while preserving the original HTTP method.

    Raises:
        SDK exception subclasses on any error.
    """
    headers = headers or {}
    timeout_val = timeout or DEFAULT_TIMEOUT
    current_url = url
    remaining_redirects = max_redirects
    original_host = httpx.URL(url).host

    try:
        with httpx.Client(timeout=timeout_val, follow_redirects=False) as client:
            while remaining_redirects >= 0:
                req_headers = dict(headers)
                current_host = httpx.URL(current_url).host
                if current_host != original_host:
                    # Strip auth on cross-host redirects
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
                    response = client.request(
                        method, current_url, headers=req_headers, content=body
                    )
                elif data is not None:
                    if "Content-Type" not in req_headers:
                        req_headers = {
                            **req_headers,
                            "Content-Type": "application/json",
                        }
                    response = client.request(
                        method, current_url, headers=req_headers, json=data
                    )
                else:
                    response = client.request(
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


def stream_download(
    url: str,
    headers: dict[str, str] | None = None,
    timeout: float | None = None,
    chunk_size: int = 8192,
) -> Iterator[bytes]:
    """Yield response chunks for a streaming download.

    Yields:
        bytes chunks from the response body.

    Raises:
        SDK exception subclasses on any error.
    """
    headers = headers or {}
    timeout_val = timeout or DOWNLOAD_TIMEOUT
    try:
        with httpx.Client(timeout=timeout_val, follow_redirects=True) as client:
            with client.stream("GET", url, headers=headers) as response:
                response.raise_for_status()
                for chunk in response.iter_bytes(chunk_size=chunk_size):
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
