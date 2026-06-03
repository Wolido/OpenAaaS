"""Tests for Config class."""

from __future__ import annotations

from typing import Any

import pytest

from pyopenaaas.config import DEFAULT_SERVER_URL, Config
from pyopenaaas.exceptions import AuthenticationError


class TestDefaults:
    """Default configuration tests."""

    def test_default_server_url(self) -> None:
        cfg = Config()
        assert cfg.server_url == DEFAULT_SERVER_URL

    def test_default_api_key_empty(self) -> None:
        cfg = Config()
        assert cfg.api_key == ""

    def test_default_timeout(self) -> None:
        cfg = Config()
        assert cfg.timeout == 30.0

    def test_default_client_id_and_name(self) -> None:
        cfg = Config()
        assert cfg.client_id == ""
        assert cfg.name == ""

    def test_repr_masks_api_key(self) -> None:
        cfg = Config(api_key="secret")
        r = repr(cfg)
        assert "***" in r
        assert "secret" not in r

    def test_require_api_key_raises_when_empty(self) -> None:
        cfg = Config()
        with pytest.raises(AuthenticationError):
            cfg.require_api_key()

    def test_require_api_key_returns_key(self) -> None:
        cfg = Config(api_key="my-key")
        assert cfg.require_api_key() == "my-key"


class TestKwargsPriority:
    """kwargs should override everything."""

    def test_kwargs_override_defaults(self) -> None:
        cfg = Config(server_url="https://custom.com", api_key="k", timeout=10.0)
        assert cfg.server_url == "https://custom.com"
        assert cfg.api_key == "k"
        assert cfg.timeout == 10.0

    def test_kwargs_override_env(self, monkeypatch: Any) -> None:
        """Even with env present, kwargs win."""
        monkeypatch.setenv("OPENAAAS_SERVER_URL", "https://env.com")
        monkeypatch.setenv("OPENAAAS_API_KEY", "env-key")

        cfg = Config(server_url="https://kwarg.com", api_key="kwarg-key")
        assert cfg.server_url == "https://kwarg.com"
        assert cfg.api_key == "kwarg-key"

    def test_trailing_slash_stripped(self) -> None:
        cfg = Config(server_url="https://api.com/")
        assert cfg.server_url == "https://api.com"

    def test_http_url_with_port_kept(self) -> None:
        cfg = Config(server_url="http://localhost:8080/")
        assert cfg.server_url == "http://localhost:8080"


class TestEnvPriority:
    """Environment variables override defaults."""

    def test_env_override_defaults(self, monkeypatch: Any) -> None:
        monkeypatch.setenv("OPENAAAS_SERVER_URL", "https://env.com")
        monkeypatch.setenv("OPENAAAS_API_KEY", "env-key")
        cfg = Config()
        assert cfg.server_url == "https://env.com"
        assert cfg.api_key == "env-key"

    def test_partial_env(self, monkeypatch: Any) -> None:
        monkeypatch.setenv("OPENAAAS_API_KEY", "env-key")
        cfg = Config()
        assert cfg.server_url == DEFAULT_SERVER_URL
        assert cfg.api_key == "env-key"


class TestThreeLevelPriority:
    """Full priority chain: kwargs > env > defaults."""

    def test_all_three_levels(self, monkeypatch: Any) -> None:
        """Only kwargs should survive when all three levels are present."""
        monkeypatch.setenv("OPENAAAS_SERVER_URL", "https://env.com")
        monkeypatch.setenv("OPENAAAS_API_KEY", "env-key")

        cfg = Config(server_url="https://kwarg.com", api_key="kwarg-key")
        assert cfg.server_url == "https://kwarg.com"
        assert cfg.api_key == "kwarg-key"

    def test_env_beats_defaults(self, monkeypatch: Any) -> None:
        monkeypatch.setenv("OPENAAAS_API_KEY", "env-key")
        cfg = Config()
        assert cfg.server_url == DEFAULT_SERVER_URL
        assert cfg.api_key == "env-key"


# --- helpers ---

@pytest.fixture(autouse=True)
def _clear_env(monkeypatch: Any) -> None:
    """Remove env vars before every test so they don't leak."""
    for key in ("OPENAAAS_SERVER_URL", "OPENAAAS_API_KEY"):
        monkeypatch.delenv(key, raising=False)
