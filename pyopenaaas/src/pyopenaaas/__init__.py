"""OpenAaaS Python SDK — Agent orchestration for Science."""

import asyncio
from typing import Any

from ._version import __version__
from .client import AsyncClient, Client
from .config import Config

__all__ = [
    "Client",
    "AsyncClient",
    "Config",
    "__version__",
    "run",
]


def run(coro: Any) -> Any:
    """Run an async coroutine from synchronous code.

    Convenience wrapper around :func:`asyncio.run`. Useful in notebooks
    or scripts where you want to fire off a quick async call without
    writing ``async def main()``.

    Example::

        import pyopenaaas
        client = pyopenaaas.AsyncClient(api_key="...")
        info = pyopenaaas.run(client.discover())
    """
    return asyncio.run(coro)
