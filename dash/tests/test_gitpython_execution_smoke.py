"""Smoke tests for the gitpython code path that is really executed at runtime.

Nothing inside ``dash/src`` imports gitpython. The only module that does is
streamlit itself: ``streamlit.git_util.GitRepo.__init__`` performs a lazy
``import git`` and then talks to a repository through ``git.Repo``,
``repo.git.version_info``, ``rev_parse`` and the branch/remote helpers used for
streamlit's "uncommitted changes" banner.

Because of that, bumping gitpython (3.1.58 -> 3.1.59) is invisible to the rest
of the suite: the existing tests mock HTTP with ``responses`` and never import
``git``. These tests create a throwaway repository in a temporary directory and
drive the same API surface streamlit drives, so a broken upgrade is caught here
instead of in a deployed dashboard.

NOTE: these tests are *not* part of the security RED gate. They pass on 3.1.58
as well; they exist to give the GREEN change regression protection.
"""

import re
import shutil
from pathlib import Path

import pytest

# GitPython probes the git binary while ``import git`` runs, so the executable has to
# be present before the module is imported; otherwise the whole file errors out during
# collection instead of reporting a missing system dependency.
if shutil.which("git") is None:  # pragma: no cover - environment guard
    pytest.skip("git executable is required to exercise gitpython", allow_module_level=True)

import git  # noqa: E402

# Identity used for every commit created below; kept explicit so that a
# developer's global git config can never influence the outcome.
TEST_ACTOR = git.Actor("OpenAaaS Dash Tests", "dash-tests@invalid.example")

GITHUB_REMOTE_URL = "https://github.com/openaaas-fixture/openaaas-dash-smoke.git"
SHA1_RE = re.compile(r"^[0-9a-f]{40}$")


@pytest.fixture
def isolated_git_env(tmp_path, monkeypatch):
    """Isolate git from user/system configuration so results are deterministic."""
    empty_config = tmp_path / "empty-gitconfig"
    empty_config.write_text("", encoding="utf-8")
    fake_home = tmp_path / "home"
    (fake_home / ".config").mkdir(parents=True)

    monkeypatch.setenv("HOME", str(fake_home))
    monkeypatch.setenv("USERPROFILE", str(fake_home))
    monkeypatch.setenv("XDG_CONFIG_HOME", str(fake_home / ".config"))
    monkeypatch.setenv("GIT_CONFIG_GLOBAL", str(empty_config))
    monkeypatch.setenv("GIT_CONFIG_SYSTEM", str(empty_config))
    monkeypatch.setenv("GIT_TERMINAL_PROMPT", "0")
    monkeypatch.setenv("GIT_OPTIONAL_LOCKS", "0")
    return fake_home


@pytest.fixture
def temp_git_repo(isolated_git_env, tmp_path):
    """A real one-commit repository in a temporary directory (never the checkout)."""
    repo_dir = tmp_path / "repo"
    repo_dir.mkdir()
    repo = git.Repo.init(str(repo_dir))

    payload = repo_dir / "payload.txt"
    payload.write_text("first\n", encoding="utf-8")
    repo.index.add([str(payload)])
    repo.index.commit("feat: first payload", author=TEST_ACTOR, committer=TEST_ACTOR, skip_hooks=True)
    return repo


@pytest.fixture
def temp_repo_with_github_remote(temp_git_repo):
    """The temp repo plus an origin remote and an upstream ref (no network I/O)."""
    branch = temp_git_repo.active_branch.name
    temp_git_repo.create_remote("origin", GITHUB_REMOTE_URL)
    # Materialise refs/remotes/<branch> offline so the upstream resolves locally.
    temp_git_repo.git.update_ref(
        f"refs/remotes/origin/{branch}", temp_git_repo.head.commit.hexsha
    )
    temp_git_repo.git.branch(f"--set-upstream-to=origin/{branch}", branch)
    return temp_git_repo


class TestGitPythonRepositoryOperations:
    """The plain gitpython API streamlit depends on keeps working."""

    def test_should_expose_head_revision_after_init_add_commit(self, temp_git_repo):
        """should return a real commit object and matching revision after committing."""
        repo = temp_git_repo

        head = repo.head.commit

        assert head.summary == "feat: first payload"
        assert SHA1_RE.match(head.hexsha)
        assert repo.git.rev_parse("HEAD") == head.hexsha
        assert [commit.summary for commit in repo.iter_commits()] == ["feat: first payload"]
        assert repo.is_dirty(untracked_files=True) is False

    def test_should_read_back_committed_blob_content(self, temp_git_repo):
        """should serve file contents from the object database across revisions."""
        repo = temp_git_repo
        payload = Path(repo.working_dir) / "payload.txt"
        payload.write_text("second\n", encoding="utf-8")
        repo.index.add([str(payload)])
        second = repo.index.commit(
            "feat: second payload", author=TEST_ACTOR, committer=TEST_ACTOR, skip_hooks=True
        )

        assert (second.tree / "payload.txt").data_stream.read() == b"second\n"
        assert (second.parents[0].tree / "payload.txt").data_stream.read() == b"first\n"
        assert [commit.summary for commit in repo.iter_commits()] == [
            "feat: second payload",
            "feat: first payload",
        ]

    def test_should_report_modified_and_untracked_files_after_commit(self, temp_git_repo):
        """should report working tree state for a modified tracked and a new file."""
        repo = temp_git_repo
        (Path(repo.working_dir) / "payload.txt").write_text("dirty\n", encoding="utf-8")
        (Path(repo.working_dir) / "scratch.txt").write_text("scratch\n", encoding="utf-8")

        assert [item.a_path for item in repo.index.diff(None)] == ["payload.txt"]
        assert repo.untracked_files == ["scratch.txt"]


class TestStreamlitGitPythonCodePath:
    """streamlit.git_util.GitRepo — the lazy consumer of gitpython — still works.

    GitRepo swallows every exception raised while opening a repository (it falls
    back to ``self.repo = None``), so the assertions below check the values a
    *valid* repository produces, not merely "no error was raised".
    """

    def test_should_report_temp_repo_as_valid(self, temp_git_repo):
        """should consider a real repository valid and compute its repo-relative module."""
        from streamlit.git_util import GitRepo

        git_repo = GitRepo(temp_git_repo.working_dir)

        assert git_repo.is_valid() is True
        assert git_repo.is_head_detached is False
        assert git_repo.module == "."

    def test_should_return_owner_repo_and_branch_for_github_remote(self, temp_repo_with_github_remote):
        """should resolve owner/repo, branch and module from a GitHub style remote."""
        from streamlit.git_util import GitRepo

        branch = temp_repo_with_github_remote.active_branch.name
        git_repo = GitRepo(temp_repo_with_github_remote.working_dir)

        assert git_repo.get_repo_info() == (
            "openaaas-fixture/openaaas-dash-smoke",
            branch,
            ".",
        )

    def test_should_list_untracked_files_of_temp_repo(self, temp_git_repo):
        """should surface untracked files through streamlit's own property."""
        from streamlit.git_util import GitRepo

        (Path(temp_git_repo.working_dir) / "scratch.txt").write_text("scratch\n", encoding="utf-8")

        assert GitRepo(temp_git_repo.working_dir).untracked_files == ["scratch.txt"]

    def test_should_report_non_repository_path_as_invalid(self, tmp_path, isolated_git_env):
        """control: a plain directory must not be reported as a repository.

        Without this, a gitpython failure would look identical to "success"
        because GitRepo turns every error into an invalid repository.
        """
        from streamlit.git_util import GitRepo

        not_a_repo = tmp_path / "plain-directory"
        not_a_repo.mkdir()

        git_repo = GitRepo(str(not_a_repo))

        assert git_repo.is_valid() is False
        assert git_repo.get_repo_info() is None
        assert git_repo.untracked_files is None
