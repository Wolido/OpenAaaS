"""OpenAaaS SDK exceptions."""


class OpenAaaSError(Exception):
    """Base exception for all OpenAaaS SDK errors."""

    pass


class AuthenticationError(OpenAaaSError):
    """Raised when API key is invalid or missing (HTTP 401/403)."""

    pass


class NotFoundError(OpenAaaSError):
    """Raised when a resource is not found (HTTP 404)."""

    pass


class ConflictError(OpenAaaSError):
    """Raised when there is a conflict (HTTP 409)."""

    pass


class RequestValidationError(OpenAaaSError):
    """Raised when request parameters are invalid (HTTP 400)."""

    pass


class NetworkError(OpenAaaSError):
    """Raised on network connectivity issues."""

    pass


class RequestTimeoutError(OpenAaaSError):
    """Raised when a request times out."""

    pass
