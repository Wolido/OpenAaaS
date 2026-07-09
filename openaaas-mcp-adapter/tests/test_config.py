"""Tests for config module."""

import json
from pathlib import Path
from unittest.mock import patch

import pytest

from openaaas_mcp_adapter.config import (
    DEFAULT_CONFIG,
    _deep_copy,
    get_config_dir,
    get_config_path,
    get_server_config,
    load_config,
    require_api_key,
    save_config,
    strip_trailing_slash,
)


class TestGetConfigDir:
    """Tests for get_config_dir."""

    def test_returns_path(self, tmp_path, monkeypatch):
        """Test that get_config_dir returns a Path."""
        monkeypatch.setattr(Path, "home", lambda: tmp_path)
        config_dir = get_config_dir()
        assert isinstance(config_dir, Path)

    def test_directory_exists(self, tmp_path, monkeypatch):
        """Test that the returned directory exists."""
        monkeypatch.setattr(Path, "home", lambda: tmp_path)
        config_dir = get_config_dir()
        assert config_dir.exists()
        assert config_dir.is_dir()


class TestStripTrailingSlash:
    """Tests for strip_trailing_slash."""

    def test_http_with_trailing_slash(self):
        assert strip_trailing_slash("http://example.com/") == "http://example.com"

    def test_https_with_trailing_slash(self):
        assert strip_trailing_slash("https://example.com/") == "https://example.com"

    def test_with_path(self):
        assert strip_trailing_slash("https://example.com/api/") == "https://example.com/api"

    def test_without_trailing_slash(self):
        assert strip_trailing_slash("https://example.com/api") == "https://example.com/api"

    def test_http_root(self):
        """Preserve http:// without making it http:"""
        assert strip_trailing_slash("http://") == "http://"

    def test_https_root(self):
        """Preserve https:// without making it https:"""
        assert strip_trailing_slash("https://") == "https://"

    def test_multiple_trailing_slashes(self):
        assert strip_trailing_slash("https://example.com///") == "https://example.com"


class TestLoadConfig:
    """Tests for load_config."""

    def test_default_config_when_file_missing(self, tmp_path, monkeypatch):
        """Test that missing file returns default config."""
        monkeypatch.setattr(Path, "home", lambda: tmp_path)
        config = load_config()
        assert config == DEFAULT_CONFIG

    def test_valid_config(self, tmp_path, monkeypatch):
        """Test loading a valid config file."""
        monkeypatch.setattr(Path, "home", lambda: tmp_path)
        config_path = get_config_path()
        config_path.parent.mkdir(parents=True, exist_ok=True)
        custom_config = {
            "servers": {
                "default": {
                    "server_url": "https://custom.example.com",
                    "api_key": "custom_key",
                    "client_id": "custom_client",
                    "name": "custom",
                }
            },
            "default_server": "default",
        }
        config_path.write_text(json.dumps(custom_config), encoding="utf-8")

        config = load_config()
        assert config["servers"]["default"]["server_url"] == "https://custom.example.com"
        assert config["servers"]["default"]["api_key"] == "custom_key"

    def test_old_format_migration(self, tmp_path, monkeypatch):
        """Test migration from old single-server format."""
        monkeypatch.setattr(Path, "home", lambda: tmp_path)
        config_path = get_config_path()
        config_path.parent.mkdir(parents=True, exist_ok=True)
        old_config = {
            "server_url": "https://old.example.com",
            "api_key": "old_key",
            "client_id": "old_client",
            "default_server": "default",
        }
        config_path.write_text(json.dumps(old_config), encoding="utf-8")

        config = load_config()
        assert "servers" in config
        assert config["servers"]["default"]["server_url"] == "https://old.example.com"
        assert config["servers"]["default"]["api_key"] == "old_key"

    def test_json_decode_error(self, tmp_path, monkeypatch):
        """Test JSON decode error raises RuntimeError."""
        monkeypatch.setattr(Path, "home", lambda: tmp_path)
        config_path = get_config_path()
        config_path.parent.mkdir(parents=True, exist_ok=True)
        config_path.write_text("not valid json", encoding="utf-8")

        with pytest.raises(RuntimeError, match="JSON 格式错误"):
            load_config()

    def test_format_error_not_dict(self, tmp_path, monkeypatch):
        """Test non-dict config raises RuntimeError."""
        monkeypatch.setattr(Path, "home", lambda: tmp_path)
        config_path = get_config_path()
        config_path.parent.mkdir(parents=True, exist_ok=True)
        config_path.write_text(json.dumps([1, 2, 3]), encoding="utf-8")

        with pytest.raises(RuntimeError, match="期望 JSON 对象"):
            load_config()


class TestSaveConfig:
    """Tests for save_config."""

    def test_normal_save(self, tmp_path, monkeypatch):
        """Test normal config save."""
        monkeypatch.setattr(Path, "home", lambda: tmp_path)
        config = _deep_copy(DEFAULT_CONFIG)
        config["default_server"] = "test"
        save_config(config)

        config_path = get_config_path()
        assert config_path.exists()
        saved = json.loads(config_path.read_text(encoding="utf-8"))
        assert saved["default_server"] == "test"

    def test_atomic_write_no_tmp_leftover(self, tmp_path, monkeypatch):
        """Test atomic write does not leave .tmp file behind."""
        monkeypatch.setattr(Path, "home", lambda: tmp_path)
        config = _deep_copy(DEFAULT_CONFIG)
        save_config(config)

        config_path = get_config_path()
        tmp_files = list(config_path.parent.glob("*.tmp.*"))
        assert len(tmp_files) == 0


class TestGetServerConfig:
    """Tests for get_server_config."""

    def test_default_server(self, tmp_path, monkeypatch):
        """Test getting default server config."""
        monkeypatch.setattr(Path, "home", lambda: tmp_path)
        config = get_config_path()
        config.parent.mkdir(parents=True, exist_ok=True)
        config.write_text(json.dumps(DEFAULT_CONFIG), encoding="utf-8")

        server = get_server_config()
        assert server["alias"] == "default"

    def test_specific_alias(self, tmp_path, monkeypatch):
        """Test getting specific alias config."""
        monkeypatch.setattr(Path, "home", lambda: tmp_path)
        custom = _deep_copy(DEFAULT_CONFIG)
        custom["servers"]["prod"] = {
            "server_url": "https://prod.example.com",
            "api_key": "prod_key",
            "client_id": "",
            "name": "",
        }
        config = get_config_path()
        config.parent.mkdir(parents=True, exist_ok=True)
        config.write_text(json.dumps(custom), encoding="utf-8")

        server = get_server_config("prod")
        assert server["alias"] == "prod"
        assert server["server_url"] == "https://prod.example.com"

    def test_alias_not_found(self, tmp_path, monkeypatch):
        """Test non-existent alias raises RuntimeError."""
        monkeypatch.setattr(Path, "home", lambda: tmp_path)
        config = get_config_path()
        config.parent.mkdir(parents=True, exist_ok=True)
        config.write_text(json.dumps(DEFAULT_CONFIG), encoding="utf-8")

        with pytest.raises(RuntimeError, match='服务器别名 "missing" 不存在'):
            get_server_config("missing")

    def test_get_server_config_empty_alias_fallback_to_default_server(self, tmp_path, monkeypatch):
        """当 alias 为空字符串时，应 fallback 到配置中的 default_server"""
        monkeypatch.setattr(Path, "home", lambda: tmp_path)
        config = {
            "servers": {
                "default": {"server_url": "https://a.com", "api_key": "k1"},
                "prod": {"server_url": "https://b.com", "api_key": "k2"},
            },
            "default_server": "prod",
        }
        save_config(config)
        sc = get_server_config("")  # 空字符串
        assert sc["alias"] == "prod"
        assert sc["server_url"] == "https://b.com"


class TestRequireApiKey:
    """Tests for require_api_key."""

    def test_api_key_exists(self, tmp_path, monkeypatch):
        """Test that existing api_key is returned."""
        monkeypatch.setattr(Path, "home", lambda: tmp_path)
        custom = _deep_copy(DEFAULT_CONFIG)
        custom["servers"]["default"]["api_key"] = "secret_key"
        config = get_config_path()
        config.parent.mkdir(parents=True, exist_ok=True)
        config.write_text(json.dumps(custom), encoding="utf-8")

        key = require_api_key()
        assert key == "secret_key"

    def test_api_key_missing(self, tmp_path, monkeypatch):
        """Test missing api_key raises RuntimeError."""
        monkeypatch.setattr(Path, "home", lambda: tmp_path)
        config = get_config_path()
        config.parent.mkdir(parents=True, exist_ok=True)
        config.write_text(json.dumps(DEFAULT_CONFIG), encoding="utf-8")

        with pytest.raises(RuntimeError, match="缺少 API Key"):
            require_api_key()
