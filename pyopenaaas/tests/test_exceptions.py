"""Tests for the exception hierarchy."""

import pytest

from pyopenaaas.exceptions import (
    AuthenticationError,
    ConflictError,
    NetworkError,
    NotFoundError,
    OpenAaaSError,
    RequestTimeoutError,
    RequestValidationError,
)


class TestExceptionInstantiation:
    """Test that every exception class can be instantiated with a message."""

    @pytest.mark.parametrize(
        "exc_cls",
        [
            OpenAaaSError,
            AuthenticationError,
            NotFoundError,
            ConflictError,
            RequestValidationError,
            NetworkError,
            RequestTimeoutError,
        ],
    )
    def test_exception_instantiation(self, exc_cls: type[Exception]) -> None:
        """Each exception class should be instantiable."""
        exc = exc_cls("something went wrong")
        assert str(exc) == "something went wrong"

    @pytest.mark.parametrize(
        "exc_cls",
        [
            OpenAaaSError,
            AuthenticationError,
            NotFoundError,
            ConflictError,
            RequestValidationError,
            NetworkError,
            RequestTimeoutError,
        ],
    )
    def test_exception_default_message(self, exc_cls: type[Exception]) -> None:
        """Exceptions without a message should produce an empty str()."""
        exc = exc_cls()
        assert str(exc) == ""


class TestExceptionInheritance:
    """Verify the inheritance chain."""

    @pytest.mark.parametrize(
        "exc_cls",
        [
            AuthenticationError,
            NotFoundError,
            ConflictError,
            RequestValidationError,
            NetworkError,
            RequestTimeoutError,
        ],
    )
    def test_all_subclass_openaaas_error(self, exc_cls: type[Exception]) -> None:
        """Every SDK exception must subclass OpenAaaSError."""
        assert issubclass(exc_cls, OpenAaaSError)

    def test_isinstance_base(self) -> None:
        """isinstance(e, OpenAaaSError) works for every leaf exception."""
        exc = AuthenticationError("bad key")
        assert isinstance(exc, OpenAaaSError)
        assert isinstance(exc, Exception)

    def test_catch_all_with_base(self) -> None:
        """A generic except OpenAaaSError catches every subclass."""
        caught = []
        for exc_cls in [
            AuthenticationError,
            NotFoundError,
            ConflictError,
            RequestValidationError,
            NetworkError,
            RequestTimeoutError,
        ]:
            try:
                raise exc_cls("boom")
            except OpenAaaSError as e:
                caught.append(type(e))
        assert len(caught) == 6

    def test_specific_catch_order(self) -> None:
        """Specific subclasses are caught before the generic base."""
        try:
            raise NotFoundError("missing")
        except NotFoundError:
            caught = "specific"
        except OpenAaaSError:
            caught = "generic"
        assert caught == "specific"
