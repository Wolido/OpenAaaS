"""Security floor tests for the gitpython dependency of the dash dashboard.

Background
----------
This PR raises the gitpython floor to 3.1.59, the first release unaffected by
PYSEC-2026-3785 / 3786 / 3787 / 3788 (CVE-2026-78675 .. CVE-2026-78678, one of
them CVSS 9.8). Before the fix, ``main`` (e727c8b) resolved ``gitpython`` to
3.1.58 in ``dash/uv.lock`` and declared only ``gitpython>=3.1.55`` in
``dash/pyproject.toml``, so a vulnerable release was still selectable.

gitpython is not used by ``aaas_dashboard`` directly: it is declared in
``dash/pyproject.toml`` on purpose as an override of streamlit's transitive
dependency, and streamlit imports it lazily inside
``streamlit.git_util.GitRepo.__init__``. So "which gitpython ends up in the
runtime environment" is a property of the dependency declaration plus the lock
file, and that is what these tests pin down.

These tests were written as the RED gate of that fix: they failed while the
resolution could still select a vulnerable release, pass now that the floor is
raised, and turn red again if the floor ever regresses.

This file is a regression guardrail, not a vulnerability database
-----------------------------------------------------------------
``SECURITY_FLOOR``, ``VULNERABLE_VERSION`` and ``ADVISORY`` below are a snapshot
of the PYSEC-2026-3785..3788 range as it stood when this fix was written. Their
role is narrow: keep this specific known-bad resolution (3.1.58, or a declared
floor below 3.1.59) from creeping back in. They will *not* notice a future
advisory that affects a higher release, and a green run here is therefore not a
statement that the dependency tree is vulnerability-free. Authoritative
vulnerability judgements come from ``pip-audit`` in CI (the ``security-audit``
job of ``.github/workflows/ci.yml``, which audits ``uv export`` of
``dash/uv.lock``). When a new advisory lands, remediate it through the normal
dependency update and raise ``SECURITY_FLOOR`` here only deliberately, as an
explicit follow-up that pins the historical regression.
"""

import importlib.metadata
import re
from pathlib import Path

import toml
from packaging.specifiers import SpecifierSet
from packaging.version import Version

# The dash package root (dash/), independent of the current working directory.
DASH_ROOT = Path(__file__).resolve().parents[1]
PYPROJECT_FILE = DASH_ROOT / "pyproject.toml"
UV_LOCK_FILE = DASH_ROOT / "uv.lock"

GITPYTHON_DIST = "gitpython"
# First release that is not affected by PYSEC-2026-3785..3788.
SECURITY_FLOOR = Version("3.1.59")
# Last release that *is* affected; it must not be selectable any more.
VULNERABLE_VERSION = Version("3.1.58")

ADVISORY = "PYSEC-2026-3785..3788 (CVE-2026-78675..78678)"

# name | optional extras | version specifier | optional environment marker
REQUIREMENT_RE = re.compile(
    r"^\s*(?P<name>[A-Za-z0-9._-]+)\s*(?:\[[^\]]*\])?\s*(?P<specifier>[^;]*?)(?:;.*)?$"
)


def _declared_specifier(project_metadata: dict, package: str) -> SpecifierSet:
    """Return the version specifier dash declares for *package* (e.g. ``>=3.1.59``)."""
    requirements = []
    for requirement in project_metadata["project"]["dependencies"]:
        parsed = REQUIREMENT_RE.match(requirement)
        if parsed and parsed.group("name").lower().replace("_", "-") == package:
            requirements.append(parsed.group("specifier").strip())

    assert len(requirements) == 1, f"expected exactly one {package} requirement, got {requirements}"
    return SpecifierSet(requirements[0])


class TestGitPythonResolvedVersion:
    """The version that the runtime actually resolves must not be vulnerable."""

    def test_resolved_gitpython_meets_security_floor(self):
        """should report a gitpython release >= 3.1.59 for the dash environment."""
        resolved = Version(importlib.metadata.version(GITPYTHON_DIST))

        assert resolved >= SECURITY_FLOOR, (
            f"{GITPYTHON_DIST} {resolved} is affected by {ADVISORY}; "
            f"the dashboard environment must resolve >= {SECURITY_FLOOR}"
        )

    def test_imported_git_module_matches_resolved_distribution(self):
        """should import the same gitpython version it reports in metadata.

        Guards the floor above against a shadowed or stale copy of ``git``
        (e.g. a leftover virtualenv), which would make the version check
        vacuous.
        """
        import git

        resolved = importlib.metadata.version(GITPYTHON_DIST)

        assert git.__version__ == resolved, (
            f"importlib.metadata reports {GITPYTHON_DIST} {resolved} but "
            f"``import git`` loaded {git.__version__} from {git.__file__}"
        )


class TestGitPythonDependencyDeclaration:
    """dash's own declaration and lock file must exclude vulnerable releases.

    ``ci.yml`` installs dash with ``pip install -e ".[dev]"``, i.e. *without*
    consuming uv.lock, so the declaration-level floor is the part of the gate
    that is enforced in CI as well.
    """

    def test_declared_gitpython_floor_excludes_vulnerable_release(self):
        """should not admit gitpython 3.1.58 and should admit 3.1.59."""
        project_metadata = toml.loads(PYPROJECT_FILE.read_text(encoding="utf-8"))

        specifier = _declared_specifier(project_metadata, GITPYTHON_DIST)

        assert VULNERABLE_VERSION not in specifier, (
            f"dash declares '{GITPYTHON_DIST}{specifier}', which still installs "
            f"{VULNERABLE_VERSION} ({ADVISORY})"
        )
        assert SECURITY_FLOOR in specifier, (
            f"dash declares '{GITPYTHON_DIST}{specifier}', which would refuse the "
            f"fixed release {SECURITY_FLOOR}"
        )

    def test_uv_lock_resolves_gitpython_at_or_above_security_floor(self):
        """should pin gitpython to a release >= 3.1.59 in uv.lock."""
        assert UV_LOCK_FILE.is_file(), f"expected lock file at {UV_LOCK_FILE}"
        lock = toml.loads(UV_LOCK_FILE.read_text(encoding="utf-8"))

        pinned = [
            Version(package["version"])
            for package in lock["package"]
            if package.get("name") == GITPYTHON_DIST
        ]

        assert len(pinned) == 1, f"expected exactly one gitpython entry in uv.lock, got {pinned}"
        assert pinned[0] >= SECURITY_FLOOR, (
            f"uv.lock pins {GITPYTHON_DIST} {pinned[0]}, which is affected by {ADVISORY}; "
            f"expected >= {SECURITY_FLOOR}"
        )
