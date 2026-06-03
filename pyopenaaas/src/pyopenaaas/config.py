"""Configuration management for OpenAaaS SDK.

Priority (high → low):
1. Code kwargs
2. Environment variables
3. Defaults
"""

from __future__ import annotations

import os

from pyopenaaas.exceptions import AuthenticationError

DEFAULT_SERVER_URL = "https://api.open-aaas.com"


class Config:
    """Unified configuration for OpenAaaS SDK.

    Args:
        server_url: OpenAaaS server base URL.
        api_key: API key for authentication.
        timeout: Default request timeout in seconds.
    """

    def __init__(
        self,
        server_url: str | None = None,
        api_key: str | None = None,
        timeout: float = 30.0,
    ) -> None:
        self._timeout = timeout

        # kwargs > env > defaults
        self.server_url = self._strip_trailing_slash(
            server_url
            if server_url is not None and server_url != ""
            else (os.environ.get("OPENAAAS_SERVER_URL") or DEFAULT_SERVER_URL)
        )
        self.api_key = (
            api_key
            if api_key is not None and api_key != ""
            else (os.environ.get("OPENAAAS_API_KEY") or "")
        )
        self.client_id = ""
        self.name = ""

    @property
    def timeout(self) -> float:
        return self._timeout

    def require_api_key(self) -> str:
        """Return the API key or raise an AuthenticationError if missing."""
        if not self.api_key:
            raise AuthenticationError(
                "API Key is missing. Please register first or set OPENAAAS_API_KEY."
            )
        return self.api_key

    @staticmethod
    def _strip_trailing_slash(url: str) -> str:
        stripped = url.rstrip("/")
        # Preserve protocol root path like "http://" (don't strip to "http:")
        if stripped.endswith(":") and stripped.lower().startswith("http"):
            return url
        return stripped

    def __repr__(self) -> str:
        masked = "***" if self.api_key else ""
        return (
            f"Config(server_url={self.server_url!r}, "
            f"api_key={masked!r})"
        )
