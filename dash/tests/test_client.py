"""Tests for API client module."""

from datetime import datetime
from unittest.mock import patch

import pytest
import requests
import responses
import json

from aaas_dashboard.client import (
    OaaSClient,
    MockOaaSClient,
    TaskStatus,
    Task,
    User,
)
from pydantic import ValidationError


class TestTaskModelValidation:
    """Test Task model validation."""

    def test_task_model_validation(self, sample_task_data):
        """Test that Task model validates correctly."""
        task = Task.model_validate(sample_task_data)
        
        assert task.id == "task-test-001"
        assert task.service_id == "code-agent"
        assert task.status == TaskStatus.RUNNING
        assert task.session_id == "session-001"
        assert task.retry_count == 0
        assert isinstance(task.created_at, datetime)

    def test_task_model_invalid_status(self):
        """Test that invalid status raises validation error."""
        invalid_data = {
            "id": "task-001",
            "service_id": "agent",
            "status": "invalid_status",
            "session_id": "session-1",
            "created_at": "2024-01-15T10:00:00Z",
        }
        
        with pytest.raises(ValidationError):
            Task.model_validate(invalid_data)

    def test_task_model_missing_required(self):
        """Test that missing required fields raise validation error."""
        incomplete_data = {
            "service_id": "agent",
            "status": "pending",
            "session_id": "session-1",
        }
        
        with pytest.raises(ValidationError):
            Task.model_validate(incomplete_data)

    def test_task_status_enum_values(self):
        """Test that TaskStatus enum has correct values."""
        assert TaskStatus.PENDING.value == "pending"
        assert TaskStatus.RUNNING.value == "running"
        assert TaskStatus.COMPLETED.value == "completed"
        assert TaskStatus.FAILED.value == "failed"
        assert TaskStatus.CANCELLED.value == "cancelled"
        assert TaskStatus.CANCELLING.value == "cancelling"


class TestTaskProgressCalculation:
    """Test Task progress property calculation."""

    def test_task_progress_completed(self, sample_completed_task_data):
        """Test that completed task has 100% progress."""
        task = Task.model_validate(sample_completed_task_data)
        
        assert task.progress == 100.0

    def test_task_progress_failed(self, sample_failed_task_data):
        """Test that failed task has 0% progress."""
        task = Task.model_validate(sample_failed_task_data)
        
        assert task.progress == 0.0

    def test_task_progress_pending(self):
        """Test that pending task has 0% progress."""
        data = {
            "id": "task-001",
            "service_id": "agent",
            "status": "pending",
            "session_id": "session-1",
            "created_at": "2024-01-15T10:00:00Z",
        }
        task = Task.model_validate(data)
        
        assert task.progress == 0.0

    def test_task_progress_running(self, sample_task_data):
        """Test that running task has 50% progress."""
        task = Task.model_validate(sample_task_data)
        
        assert task.progress == 50.0

    def test_task_progress_cancelled(self):
        """Test that cancelled task progress is None."""
        data = {
            "id": "task-001",
            "service_id": "agent",
            "status": "cancelled",
            "session_id": "session-1",
            "created_at": "2024-01-15T10:00:00Z",
        }
        task = Task.model_validate(data)
        
        assert task.progress is None

    def test_task_progress_cancelling(self):
        """Test that cancelling task progress is None."""
        data = {
            "id": "task-001",
            "service_id": "agent",
            "status": "cancelling",
            "session_id": "session-1",
            "created_at": "2024-01-15T10:00:00Z",
        }
        task = Task.model_validate(data)
        
        assert task.progress is None


class TestTaskNameProperty:
    """Test Task name property."""

    def test_task_name_from_short_prompt(self):
        """Test name extraction from short prompt."""
        data = {
            "id": "task-001",
            "service_id": "agent",
            "status": "pending",
            "session_id": "session-1",
            "created_at": "2024-01-15T10:00:00Z",
            "input": {"task_prompt": "Short prompt"},
        }
        task = Task.model_validate(data)
        
        assert task.name == "Short prompt"

    def test_task_name_from_long_prompt(self):
        """Test name truncation from long prompt."""
        long_prompt = "A" * 100
        data = {
            "id": "task-001",
            "service_id": "agent",
            "status": "pending",
            "session_id": "session-1",
            "created_at": "2024-01-15T10:00:00Z",
            "input": {"task_prompt": long_prompt},
        }
        task = Task.model_validate(data)
        
        assert task.name == "A" * 50 + "..."

    def test_task_name_fallback(self):
        """Test name fallback to task ID (first 8 chars)."""
        data = {
            "id": "task-abc123xyz",
            "service_id": "agent",
            "status": "pending",
            "session_id": "session-1",
            "created_at": "2024-01-15T10:00:00Z",
        }
        task = Task.model_validate(data)
        
        # Task.name returns f"Task {self.id[:8]}" - first 8 chars of ID
        assert task.name == "Task task-abc"


class TestClientHealthCheck:
    """Test client health check functionality."""

    @responses.activate
    def test_client_health_check_success(self, client, mock_server_url):
        """Test successful health check."""
        responses.add(
            responses.GET,
            f"{mock_server_url}/health",
            json={"status": "healthy", "version": "0.1.0"},
            status=200
        )
        
        result = client.health_check()
        
        assert result["status"] == "healthy"
        assert result["version"] == "0.1.0"

    @responses.activate
    def test_client_health_check_failure(self, client, mock_server_url):
        """Test health check with server error."""
        responses.add(
            responses.GET,
            f"{mock_server_url}/health",
            status=500
        )
        
        result = client.health_check()
        
        assert result["status"] == "unhealthy"
        assert "error" in result


class TestClientListTasks:
    """Test client list_tasks functionality."""

    @responses.activate
    def test_client_list_tasks(self, client, mock_server_url, sample_task_data):
        """Test listing tasks."""
        responses.add(
            responses.GET,
            f"{mock_server_url}/api/v1/client/tasks",
            json=[sample_task_data],
            status=200
        )
        
        tasks = client.list_tasks()
        
        assert len(tasks) == 1
        assert tasks[0].id == "task-test-001"
        assert tasks[0].status == TaskStatus.RUNNING

    @responses.activate
    def test_client_list_tasks_with_params(self, client, mock_server_url, sample_task_data):
        """Test listing tasks with status filter and pagination."""
        responses.add(
            responses.GET,
            f"{mock_server_url}/api/v1/client/tasks",
            json=[sample_task_data],
            status=200
        )
        
        tasks = client.list_tasks(status=TaskStatus.RUNNING, limit=10, offset=5)
        
        # Verify request was made with correct params
        assert len(responses.calls) == 1
        request = responses.calls[0].request
        assert "status=running" in request.url
        assert "limit=10" in request.url
        assert "offset=5" in request.url

    @responses.activate
    def test_client_list_tasks_empty(self, client, mock_server_url):
        """Test listing tasks returns empty list when no tasks."""
        responses.add(
            responses.GET,
            f"{mock_server_url}/api/v1/client/tasks",
            json=[],
            status=200
        )
        
        tasks = client.list_tasks()
        
        assert tasks == []

    @responses.activate
    def test_client_list_tasks_wrapped_response(self, client, mock_server_url, sample_task_data):
        """Test handling of wrapped response format."""
        responses.add(
            responses.GET,
            f"{mock_server_url}/api/v1/client/tasks",
            json={"tasks": [sample_task_data]},
            status=200
        )
        
        tasks = client.list_tasks()
        
        assert len(tasks) == 1
        assert tasks[0].id == "task-test-001"

    @responses.activate
    def test_client_list_tasks_nested_wrapped_response(self, client, mock_server_url, sample_task_data):
        """Test handling of a response with one extra list layer."""
        responses.add(
            responses.GET,
            f"{mock_server_url}/api/v1/client/tasks",
            json={"tasks": [[sample_task_data]]},
            status=200
        )

        tasks = client.list_tasks()

        assert len(tasks) == 1
        assert tasks[0].id == "task-test-001"

    @responses.activate
    def test_client_list_tasks_data_wrapped_response(self, client, mock_server_url, sample_task_data):
        """Test handling of data-wrapped response format."""
        responses.add(
            responses.GET,
            f"{mock_server_url}/api/v1/client/tasks",
            json={"data": [sample_task_data]},
            status=200
        )

        tasks = client.list_tasks()

        assert len(tasks) == 1
        assert tasks[0].id == "task-test-001"

    @responses.activate
    def test_client_list_tasks_validation_error(self, client, mock_server_url):
        """Test handling of validation errors."""
        responses.add(
            responses.GET,
            f"{mock_server_url}/api/v1/client/tasks",
            json=[{"invalid": "data"}],
            status=200
        )
        
        tasks = client.list_tasks()
        
        assert tasks == []


class TestClientGetTask:
    """Test client get_task functionality."""

    @responses.activate
    def test_client_get_task(self, client, mock_server_url, sample_task_data):
        """Test getting a single task."""
        task_id = "task-test-001"
        responses.add(
            responses.GET,
            f"{mock_server_url}/api/v1/client/tasks/{task_id}",
            json=sample_task_data,
            status=200
        )
        
        task = client.get_task(task_id)
        
        assert task is not None
        assert task.id == task_id
        assert task.status == TaskStatus.RUNNING

    @responses.activate
    def test_client_get_task_not_found(self, client, mock_server_url):
        """Test getting a non-existent task returns None."""
        task_id = "nonexistent"
        responses.add(
            responses.GET,
            f"{mock_server_url}/api/v1/client/tasks/{task_id}",
            status=404
        )
        
        task = client.get_task(task_id)
        
        assert task is None


class TestClientCancelTask:
    """Test client cancel_task functionality."""

    @responses.activate
    def test_client_cancel_task_success(self, client, mock_server_url):
        """Test successful task cancellation."""
        task_id = "task-test-001"
        responses.add(
            responses.POST,
            f"{mock_server_url}/api/v1/client/tasks/{task_id}/cancel",
            status=200
        )
        
        result = client.cancel_task(task_id)
        
        assert result is True

    @responses.activate
    def test_client_cancel_task_failure(self, client, mock_server_url):
        """Test failed task cancellation."""
        task_id = "task-test-001"
        responses.add(
            responses.POST,
            f"{mock_server_url}/api/v1/client/tasks/{task_id}/cancel",
            status=400
        )
        
        result = client.cancel_task(task_id)
        
        assert result is False


class TestClientCreateTask:
    """Test client create_task response parsing."""

    @responses.activate
    def test_client_create_task_direct_response(self, client, mock_server_url, sample_task_data):
        """Test creating a task from a direct task object response."""
        responses.add(
            responses.POST,
            f"{mock_server_url}/api/v1/client/tasks",
            json=sample_task_data,
            status=200,
        )

        task = client.create_task("code-agent", "prompt", "output")

        assert task is not None
        assert task.id == "task-test-001"

    @responses.activate
    def test_client_create_task_list_response(self, client, mock_server_url, sample_task_data):
        """Test creating a task from a single-item list response."""
        responses.add(
            responses.POST,
            f"{mock_server_url}/api/v1/client/tasks",
            json=[sample_task_data],
            status=200,
        )

        task = client.create_task("code-agent", "prompt", "output")

        assert task is not None
        assert task.id == "task-test-001"

    @responses.activate
    def test_client_create_task_wrapped_response(self, client, mock_server_url, sample_task_data):
        """Test creating a task from a wrapped response."""
        responses.add(
            responses.POST,
            f"{mock_server_url}/api/v1/client/tasks",
            json={"data": [sample_task_data]},
            status=200,
        )

        task = client.create_task("code-agent", "prompt", "output")

        assert task is not None
        assert task.id == "task-test-001"

    @responses.activate
    def test_client_create_task_tasks_wrapped_response(self, client, mock_server_url, sample_task_data):
        """Test creating a task from a tasks-wrapped response."""
        responses.add(
            responses.POST,
            f"{mock_server_url}/api/v1/client/tasks",
            json={"tasks": [sample_task_data]},
            status=200,
        )

        task = client.create_task("code-agent", "prompt", "output")

        assert task is not None
        assert task.id == "task-test-001"

    @responses.activate
    def test_client_create_task_multi_item_list_response(self, client, mock_server_url, sample_task_data):
        """Test creating a task rejects a multi-item list response."""
        second_task = dict(sample_task_data)
        second_task["id"] = "task-test-002"
        responses.add(
            responses.POST,
            f"{mock_server_url}/api/v1/client/tasks",
            json=[sample_task_data, second_task],
            status=200,
        )

        task = client.create_task("code-agent", "prompt", "output")

        assert task is None
        assert client.last_error is not None
        assert "Input should be a valid dictionary" in client.last_error

    @responses.activate
    def test_client_create_task_follows_redirect(self, client, mock_server_url, sample_task_data):
        """Test creating a task follows redirect and succeeds."""
        responses.add(
            responses.POST,
            f"{mock_server_url}/api/v1/client/tasks",
            status=301,
            headers={"Location": "https://example.com/api/v1/client/tasks"},
        )
        responses.add(
            responses.POST,
            "https://example.com/api/v1/client/tasks",
            json=sample_task_data,
            status=200,
        )

        task = client.create_task("code-agent", "prompt", "output")

        assert task is not None
        assert task.id == "task-test-001"

    @responses.activate
    def test_client_create_task_redirect_exceeds_max(self, client, mock_server_url):
        """Test creating a task fails after exceeding max redirects."""
        for i in range(4):
            responses.add(
                responses.POST,
                f"{mock_server_url}/api/v1/client/tasks" if i == 0 else f"https://example.com/api/v1/client/tasks/{i}",
                status=301,
                headers={"Location": f"https://example.com/api/v1/client/tasks/{i + 1}"},
            )

        task = client.create_task("code-agent", "prompt", "output")

        assert task is None
        assert client.last_error is not None
        assert "after 3 redirects" in client.last_error
        assert "Use the final HTTPS Server URL directly" in client.last_error

    @responses.activate
    def test_client_create_task_follows_relative_redirect(self, client, mock_server_url, sample_task_data):
        """Test creating a task follows relative redirect and succeeds."""
        responses.add(
            responses.POST,
            f"{mock_server_url}/api/v1/client/tasks",
            status=301,
            headers={"Location": "/api/v1/client/tasks"},
        )
        responses.add(
            responses.POST,
            f"{mock_server_url}/api/v1/client/tasks",
            json=sample_task_data,
            status=200,
        )

        task = client.create_task("code-agent", "prompt", "output")

        assert task is not None
        assert task.id == "task-test-001"

    @responses.activate
    def test_client_create_task_redirect_strips_auth_cross_domain(self, client, mock_server_url, sample_task_data):
        """Test that Authorization header is stripped on cross-domain redirect."""
        responses.add(
            responses.POST,
            f"{mock_server_url}/api/v1/client/tasks",
            status=301,
            headers={"Location": "https://example.com/api/v1/client/tasks"},
        )
        responses.add(
            responses.POST,
            "https://example.com/api/v1/client/tasks",
            json=sample_task_data,
            status=200,
        )

        task = client.create_task("code-agent", "prompt", "output")

        assert task is not None
        assert task.id == "task-test-001"
        # Verify second request (cross-domain redirect) has no Authorization header
        assert len(responses.calls) == 2
        second_request = responses.calls[1].request
        assert "Authorization" not in second_request.headers

    def test_post_multipart_with_redirects_rewinds_files(self, client, mock_server_url, sample_task_data):
        """Test _post_multipart_with_redirects rewinds file handles on redirect."""
        from unittest.mock import MagicMock

        mock_fh = MagicMock()
        mock_fh.seek = MagicMock()

        with responses.RequestsMock() as rsps:
            rsps.add(
                responses.POST,
                f"{mock_server_url}/api/v1/client/tasks",
                status=301,
                headers={"Location": f"{mock_server_url}/api/v1/client/tasks"},
            )
            rsps.add(
                responses.POST,
                f"{mock_server_url}/api/v1/client/tasks",
                json=sample_task_data,
                status=200,
            )

            response, url, redirect_count = client._post_multipart_with_redirects(
                f"{mock_server_url}/api/v1/client/tasks",
                opened_files=[mock_fh],
            )

        assert redirect_count == 1
        assert response.status_code == 200
        assert mock_fh.seek.call_count == 2
        mock_fh.seek.assert_called_with(0)


class TestClientNetworkError:
    """Test client network error handling."""

    @responses.activate
    def test_client_network_error_list_tasks(self, client, mock_server_url):
        """Test network error handling in list_tasks."""
        responses.add(
            responses.GET,
            f"{mock_server_url}/api/v1/client/tasks",
            body=requests.ConnectionError("Connection refused")
        )
        
        tasks = client.list_tasks()
        
        assert tasks == []

    @responses.activate
    def test_client_network_error_get_task(self, client, mock_server_url):
        """Test network error handling in get_task."""
        responses.add(
            responses.GET,
            f"{mock_server_url}/api/v1/client/tasks/test",
            body=requests.Timeout("Request timed out")
        )
        
        task = client.get_task("test")
        
        assert task is None

    @responses.activate
    def test_client_network_error_cancel_task(self, client, mock_server_url):
        """Test network error handling in cancel_task."""
        responses.add(
            responses.POST,
            f"{mock_server_url}/api/v1/client/tasks/test/cancel",
            body=requests.ConnectionError("Connection refused")
        )
        
        result = client.cancel_task("test")
        
        assert result is False

    @responses.activate
    def test_delete_service_network_error(self, client, mock_server_url):
        """Test network error handling in delete_service."""
        responses.add(
            responses.DELETE,
            f"{mock_server_url}/api/v1/services/svc-1",
            body=requests.ConnectionError("Connection refused")
        )
        success, error_msg = client.delete_service("svc-1")
        assert success is False
        assert "Connection refused" in error_msg

    @responses.activate
    def test_delete_user_network_error(self, client, mock_server_url):
        """Test network error handling in delete_user."""
        responses.add(
            responses.DELETE,
            f"{mock_server_url}/api/v1/admin/users/user-001",
            body=requests.Timeout("Request timed out")
        )
        success, error_msg = client.delete_user("user-001")
        assert success is False
        assert "Request timed out" in error_msg

    @responses.activate
    def test_revoke_service_permission_network_error(self, client, mock_server_url):
        """Test network error handling in revoke_service_permission."""
        responses.add(
            responses.DELETE,
            f"{mock_server_url}/api/v1/admin/services/svc-1/users/user-001",
            body=requests.ConnectionError("Connection refused")
        )
        success, error_msg = client.revoke_service_permission("svc-1", "user-001")
        assert success is False
        assert "Connection refused" in error_msg


class TestClientGetStats:
    """Test client get_stats functionality."""

    @responses.activate
    def test_client_get_stats(self, client, mock_server_url):
        """Test getting task statistics."""
        tasks_data = [
            {"id": "t1", "service_id": "agent", "status": "pending", "session_id": "s1", "created_at": "2024-01-15T10:00:00Z"},
            {"id": "t2", "service_id": "agent", "status": "running", "session_id": "s1", "created_at": "2024-01-15T10:00:00Z"},
            {"id": "t3", "service_id": "agent", "status": "completed", "session_id": "s1", "created_at": "2024-01-15T10:00:00Z"},
            {"id": "t4", "service_id": "agent", "status": "failed", "session_id": "s1", "created_at": "2024-01-15T10:00:00Z"},
            {"id": "t5", "service_id": "agent", "status": "cancelled", "session_id": "s1", "created_at": "2024-01-15T10:00:00Z"},
            {"id": "t6", "service_id": "agent", "status": "cancelling", "session_id": "s1", "created_at": "2024-01-15T10:00:00Z"},
        ]
        responses.add(
            responses.GET,
            f"{mock_server_url}/api/v1/client/tasks",
            json=tasks_data,
            status=200
        )
        
        stats = client.get_stats()
        
        assert stats["total"] == 6
        assert stats["pending"] == 1
        assert stats["running"] == 1
        assert stats["completed"] == 1
        assert stats["failed"] == 1
        assert stats["cancelled"] == 1
        assert stats["cancelling"] == 1

    @responses.activate
    def test_client_get_stats_empty(self, client, mock_server_url):
        """Test getting stats with no tasks."""
        responses.add(
            responses.GET,
            f"{mock_server_url}/api/v1/client/tasks",
            json=[],
            status=200
        )
        
        stats = client.get_stats()
        
        assert stats["total"] == 0
        assert all(v == 0 for k, v in stats.items() if k != "total")


# ==================== Admin API Tests ====================

class TestUserModelValidation:
    """Test User model validation."""

    def test_user_model_validation(self):
        """Test that User model validates correctly."""
        data = {
            "id": "user-001",
            "name": "Test User",
            "api_key": "ak_test_123",
            "role": "admin",
            "created_at": "2024-01-15T10:00:00Z",
        }
        user = User.model_validate(data)
        assert user.id == "user-001"
        assert user.name == "Test User"
        assert user.api_key == "ak_test_123"
        assert user.role == "admin"
        assert isinstance(user.created_at, datetime)

    def test_user_model_missing_required(self):
        """Test that missing required fields raise validation error."""
        incomplete_data = {
            "id": "user-001",
            "name": "Test User",
        }
        with pytest.raises(ValidationError):
            User.model_validate(incomplete_data)


class TestClientListUsers:
    """Test client list_users functionality."""

    @responses.activate
    def test_client_list_users(self, client, mock_server_url):
        """Test listing users."""
        users_data = [
            {"id": "user-001", "name": "Admin", "api_key": "ak_1", "role": "admin", "created_at": "2024-01-15T10:00:00Z"},
            {"id": "user-002", "name": "Client", "api_key": "ak_2", "role": "client", "created_at": "2024-01-15T10:00:00Z"},
        ]
        responses.add(
            responses.GET,
            f"{mock_server_url}/api/v1/admin/users",
            json=users_data,
            status=200,
        )
        users = client.list_users()
        assert len(users) == 2
        assert users[0].id == "user-001"
        assert users[1].role == "client"

    @responses.activate
    def test_client_list_users_wrapped(self, client, mock_server_url):
        """Test listing users with wrapped response."""
        responses.add(
            responses.GET,
            f"{mock_server_url}/api/v1/admin/users",
            json={"users": []},
            status=200,
        )
        users = client.list_users()
        assert users == []

    @responses.activate
    def test_client_list_users_network_error(self, client, mock_server_url):
        """Test network error returns empty list."""
        responses.add(
            responses.GET,
            f"{mock_server_url}/api/v1/admin/users",
            body=requests.ConnectionError("refused"),
        )
        users = client.list_users()
        assert users == []


class TestClientDeleteUser:
    """Test client delete_user functionality."""

    @responses.activate
    def test_client_delete_user_success(self, client, mock_server_url):
        """Test successful user deletion."""
        responses.add(
            responses.DELETE,
            f"{mock_server_url}/api/v1/admin/users/user-001",
            status=200,
        )
        success, error_msg = client.delete_user("user-001")
        assert success is True
        assert error_msg is None

    @responses.activate
    def test_client_delete_user_failure(self, client, mock_server_url):
        """Test failed user deletion."""
        responses.add(
            responses.DELETE,
            f"{mock_server_url}/api/v1/admin/users/user-001",
            json={"error": "User not found"},
            status=404,
        )
        success, error_msg = client.delete_user("user-001")
        assert success is False
        assert error_msg == "User not found"


class TestClientUpdateUserRole:
    """Test client update_user_role functionality."""

    @responses.activate
    def test_client_update_user_role_success(self, client, mock_server_url):
        """Test successful role update."""
        user_data = {
            "id": "user-001",
            "name": "Test",
            "api_key": "ak_1",
            "role": "admin",
            "created_at": "2024-01-15T10:00:00Z",
        }
        responses.add(
            responses.PUT,
            f"{mock_server_url}/api/v1/admin/users/user-001/role",
            json=user_data,
            status=200,
        )
        updated = client.update_user_role("user-001", "admin")
        assert updated is not None
        assert updated.role == "admin"

    @responses.activate
    def test_client_update_user_role_failure(self, client, mock_server_url):
        """Test failed role update returns None."""
        responses.add(
            responses.PUT,
            f"{mock_server_url}/api/v1/admin/users/user-001/role",
            status=403,
        )
        updated = client.update_user_role("user-001", "admin")
        assert updated is None


class TestClientListUserPermissions:
    """Test client list_user_permissions functionality."""

    @responses.activate
    def test_client_list_user_permissions(self, client, mock_server_url):
        """Test listing user permissions."""
        perms = [{"service_id": "svc-1", "service_name": "Agent"}]
        responses.add(
            responses.GET,
            f"{mock_server_url}/api/v1/admin/users/user-001/permissions",
            json=perms,
            status=200,
        )
        result = client.list_user_permissions("user-001")
        assert len(result) == 1
        assert result[0]["service_id"] == "svc-1"

    @responses.activate
    def test_client_list_user_permissions_wrapped(self, client, mock_server_url):
        """Test wrapped permissions response."""
        responses.add(
            responses.GET,
            f"{mock_server_url}/api/v1/admin/users/user-001/permissions",
            json={"permissions": []},
            status=200,
        )
        result = client.list_user_permissions("user-001")
        assert result == []


class TestClientRevokeServicePermission:
    """Test client revoke_service_permission functionality."""

    @responses.activate
    def test_client_revoke_success(self, client, mock_server_url):
        """Test successful permission revocation."""
        responses.add(
            responses.DELETE,
            f"{mock_server_url}/api/v1/admin/services/svc-1/users/user-001",
            status=200,
        )
        success, error_msg = client.revoke_service_permission("svc-1", "user-001")
        assert success is True
        assert error_msg is None

    @responses.activate
    def test_client_revoke_failure(self, client, mock_server_url):
        """Test failed permission revocation."""
        responses.add(
            responses.DELETE,
            f"{mock_server_url}/api/v1/admin/services/svc-1/users/user-001",
            json={"error": "Permission not found"},
            status=404,
        )
        success, error_msg = client.revoke_service_permission("svc-1", "user-001")
        assert success is False
        assert error_msg == "Permission not found"


class TestClientServices:
    """Test client service management functionality."""

    @responses.activate
    def test_client_list_services(self, client, mock_server_url):
        """Test listing services."""
        services_data = [
            {"id": "svc-1", "name": "agent-1", "description": "desc", "agent_status": "online", "registration_status": "active", "is_public": True},
        ]
        responses.add(
            responses.GET,
            f"{mock_server_url}/api/v1/services",
            json=services_data,
            status=200,
        )
        services = client.list_services()
        assert len(services) == 1
        assert services[0]["id"] == "svc-1"

    @responses.activate
    def test_client_create_service(self, client, mock_server_url):
        """Test creating a service."""
        result_data = {
            "id": "svc-new",
            "name": "new-agent",
            "description": "desc",
            "usage": "usage",
            "is_public": False,
            "registration_token": "token-abc",
        }
        responses.add(
            responses.POST,
            f"{mock_server_url}/api/v1/services",
            json=result_data,
            status=200,
        )
        result = client.create_service("new-agent", "desc", "usage", False)
        assert result["id"] == "svc-new"
        assert result["registration_token"] == "token-abc"

    @responses.activate
    def test_client_delete_service(self, client, mock_server_url):
        """Test deleting a service."""
        from aaas_dashboard.client import DeleteServiceResult
        responses.add(
            responses.DELETE,
            f"{mock_server_url}/api/v1/services/svc-1",
            json={"deleted": True},
            status=200,
        )
        success, result = client.delete_service("svc-1")
        assert success is True
        assert isinstance(result, DeleteServiceResult)
        assert result.deleted is True

    @responses.activate
    def test_client_delete_service_failure_with_error(self, client, mock_server_url):
        """Test failed service deletion returns error message."""
        responses.add(
            responses.DELETE,
            f"{mock_server_url}/api/v1/services/svc-1",
            json={"error": "还有 3 个任务关联此服务"},
            status=400,
        )
        success, error_msg = client.delete_service("svc-1")
        assert success is False
        assert error_msg == "还有 3 个任务关联此服务"

    @responses.activate
    def test_client_delete_service_failure_non_json(self, client, mock_server_url):
        """Test failed service deletion with non-JSON response falls back to text."""
        responses.add(
            responses.DELETE,
            f"{mock_server_url}/api/v1/services/svc-1",
            body="Bad Request",
            status=400,
        )
        success, error_msg = client.delete_service("svc-1")
        assert success is False
        assert error_msg == "Bad Request"

    @responses.activate
    def test_client_delete_service_failure_json_with_message(self, client, mock_server_url):
        """Test failed service deletion with JSON containing message key reads it."""
        responses.add(
            responses.DELETE,
            f"{mock_server_url}/api/v1/services/svc-1",
            json={"message": "Something went wrong"},
            status=400,
        )
        success, error_msg = client.delete_service("svc-1")
        assert success is False
        assert error_msg == "Something went wrong"

    @responses.activate
    def test_client_delete_service_failure_json_without_known_keys(self, client, mock_server_url):
        """Test failed service deletion with JSON missing known keys falls back to reason."""
        responses.add(
            responses.DELETE,
            f"{mock_server_url}/api/v1/services/svc-1",
            json={"foo": "bar"},
            status=400,
        )
        success, error_msg = client.delete_service("svc-1")
        assert success is False
        assert error_msg == "Bad Request"

    @responses.activate
    def test_client_delete_service_force_true(self, client, mock_server_url):
        """Test force deleting a service returns DeleteServiceResponse JSON."""
        from aaas_dashboard.client import DeleteServiceResult
        responses.add(
            responses.DELETE,
            f"{mock_server_url}/api/v1/services/svc-1?force=true",
            json={"deleted": True, "tasks_cancelled": 2, "tasks_retained": 3},
            status=200,
        )
        success, result = client.delete_service("svc-1", force=True)
        assert success is True
        assert isinstance(result, DeleteServiceResult)
        assert result.deleted is True
        assert result.tasks_cancelled == 2
        assert result.tasks_retained == 3

    @responses.activate
    def test_client_grant_service_permission(self, client, mock_server_url):
        """Test granting service permission."""
        responses.add(
            responses.POST,
            f"{mock_server_url}/api/v1/client/services/svc-1/grant",
            json={"success": True},
            status=200,
        )
        assert client.grant_service_permission("svc-1", "user-001") is True

    @responses.activate
    def test_client_get_service_usage_success(self, client, mock_server_url):
        """Test getting service usage successfully."""
        responses.add(
            responses.GET,
            f"{mock_server_url}/api/v1/client/services/svc-1/usage",
            json={"id": "svc-1", "name": "agent-1", "usage": "This is the usage description"},
            status=200,
        )
        result = client.get_service_usage("svc-1")
        assert result is not None
        assert result["usage"] == "This is the usage description"

    @responses.activate
    def test_client_get_service_usage_not_found(self, client, mock_server_url):
        """Test getting service usage returns None on 404."""
        responses.add(
            responses.GET,
            f"{mock_server_url}/api/v1/client/services/nonexistent/usage",
            status=404,
        )
        result = client.get_service_usage("nonexistent")
        assert result is None

    @responses.activate
    def test_client_update_service_success(self, client, mock_server_url):
        """Test updating a service with all fields."""
        responses.add(
            responses.PUT,
            f"{mock_server_url}/api/v1/services/svc-1",
            json={
                "id": "svc-1",
                "name": "updated-name",
                "description": "updated-desc",
                "usage": "updated-usage",
                "is_public": False,
                "agent_status": "online",
                "registration_status": "active",
            },
            status=200,
        )
        result = client.update_service(
            "svc-1",
            name="updated-name",
            description="updated-desc",
            usage="updated-usage",
            is_public=False,
        )
        assert result is not None
        assert result["name"] == "updated-name"
        assert result["description"] == "updated-desc"
        assert result["usage"] == "updated-usage"
        assert result["is_public"] is False
        # Verify only changed fields were sent
        request_body = responses.calls[0].request.body
        payload = json.loads(request_body)
        assert payload == {
            "name": "updated-name",
            "description": "updated-desc",
            "usage": "updated-usage",
            "is_public": False,
        }

    @responses.activate
    def test_client_update_service_partial(self, client, mock_server_url):
        """Test updating a service with only some fields."""
        responses.add(
            responses.PUT,
            f"{mock_server_url}/api/v1/services/svc-1",
            json={"id": "svc-1", "name": "only-name-changed"},
            status=200,
        )
        result = client.update_service("svc-1", name="only-name-changed")
        assert result is not None
        assert result["name"] == "only-name-changed"
        payload = json.loads(responses.calls[0].request.body)
        assert payload == {"name": "only-name-changed"}

    @responses.activate
    def test_client_update_service_empty_payload_fallback(self, client, mock_server_url):
        """Test that empty payload falls back to GET current service info."""
        responses.add(
            responses.GET,
            f"{mock_server_url}/api/v1/services/svc-1",
            json={"id": "svc-1", "name": "current-name"},
            status=200,
        )
        result = client.update_service("svc-1")
        assert result is not None
        assert result["name"] == "current-name"
        # Verify GET was called, not PUT
        assert responses.calls[0].request.method == "GET"
        assert len(responses.calls) == 1

    @responses.activate
    def test_client_update_service_not_found(self, client, mock_server_url):
        """Test updating a non-existent service returns None."""
        responses.add(
            responses.PUT,
            f"{mock_server_url}/api/v1/services/nonexistent",
            status=404,
        )
        result = client.update_service("nonexistent", name="new-name")
        assert result is None

    @responses.activate
    def test_client_update_service_network_error(self, client, mock_server_url):
        """Test network error in update_service."""
        responses.add(
            responses.PUT,
            f"{mock_server_url}/api/v1/services/svc-1",
            body=requests.ConnectionError("Connection refused"),
        )
        result = client.update_service("svc-1", name="new-name")
        assert result is None

    @responses.activate
    def test_client_update_service_server_error(self, client, mock_server_url):
        """Test server returns 500 on update."""
        responses.add(
            responses.PUT,
            f"{mock_server_url}/api/v1/services/svc-1",
            json={"message": "Internal server error"},
            status=500,
        )
        result = client.update_service("svc-1", name="new-name")
        assert result is None

    @responses.activate
    def test_client_update_service_empty_payload_get_fails(self, client, mock_server_url):
        """Test empty payload fallback GET returns 404."""
        responses.add(
            responses.GET,
            f"{mock_server_url}/api/v1/services/svc-1",
            status=404,
        )
        result = client.update_service("svc-1")
        assert result is None

    @responses.activate
    def test_client_update_service_empty_payload_network_error(self, client, mock_server_url):
        """Test empty payload fallback GET encounters network error."""
        responses.add(
            responses.GET,
            f"{mock_server_url}/api/v1/services/svc-1",
            body=requests.ConnectionError("Connection refused"),
        )
        result = client.update_service("svc-1")
        assert result is None


class TestMockClient:
    """Test MockOaaSClient functionality."""

    def test_mock_client_generate_tasks(self):
        """Test that mock client generates tasks."""
        client = MockOaaSClient()
        
        tasks = client.list_tasks()
        
        assert len(tasks) == 20
        assert all(isinstance(t, Task) for t in tasks)

    def test_mock_client_health_check(self):
        """Test mock client health check."""
        client = MockOaaSClient()
        
        health = client.health_check()
        
        assert health["status"] == "healthy"
        assert "version" in health

    def test_mock_client_get_task(self):
        """Test mock client get task."""
        client = MockOaaSClient()
        
        task = client.get_task("task-0001")
        
        assert task is not None
        assert task.id == "task-0001"

    def test_mock_client_get_task_not_found(self):
        """Test mock client get non-existent task."""
        client = MockOaaSClient()
        
        task = client.get_task("nonexistent")
        
        assert task is None

    def test_mock_client_cancel_task(self):
        """Test mock client cancel task."""
        client = MockOaaSClient()
        
        # Find a running task to cancel
        running_tasks = client.list_tasks(status=TaskStatus.RUNNING)
        if running_tasks:
            task_id = running_tasks[0].id
            result = client.cancel_task(task_id)
            
            assert result is True
            
            # Verify task is now cancelled
            task = client.get_task(task_id)
            assert task.status == TaskStatus.CANCELLED

    def test_mock_client_cancel_task_not_running(self):
        """Test mock client cancel non-running task."""
        client = MockOaaSClient()
        
        # Find a pending task
        pending_tasks = client.list_tasks(status=TaskStatus.PENDING)
        if pending_tasks:
            task_id = pending_tasks[0].id
            result = client.cancel_task(task_id)
            
            assert result is False

    def test_mock_client_list_tasks_with_filter(self):
        """Test mock client list with status filter."""
        client = MockOaaSClient()
        
        all_tasks = client.list_tasks()
        running_tasks = client.list_tasks(status=TaskStatus.RUNNING)
        
        assert len(running_tasks) < len(all_tasks)
        assert all(t.status == TaskStatus.RUNNING for t in running_tasks)

    def test_mock_client_list_tasks_with_pagination(self):
        """Test mock client list with pagination."""
        client = MockOaaSClient()
        
        first_page = client.list_tasks(limit=5, offset=0)
        second_page = client.list_tasks(limit=5, offset=5)
        
        assert len(first_page) == 5
        assert len(second_page) == 5
        # IDs should be different
        first_ids = {t.id for t in first_page}
        second_ids = {t.id for t in second_page}
        assert not first_ids.intersection(second_ids)

    def test_mock_client_get_stats(self):
        """Test mock client get stats."""
        client = MockOaaSClient()
        
        stats = client.get_stats()
        
        assert stats["total"] == 20
        assert sum(stats[s.value] for s in TaskStatus) == 20

    # Mock admin API tests

    def test_mock_client_list_users(self):
        """Test mock client list users."""
        client = MockOaaSClient()
        users = client.list_users()
        assert len(users) == 3
        assert all(isinstance(u, User) for u in users)

    def test_mock_client_delete_user(self):
        """Test mock client delete user."""
        client = MockOaaSClient()
        success, error_msg = client.delete_user("user-003")
        assert success is True
        assert error_msg is None
        assert len(client.list_users()) == 2
        success, error_msg = client.delete_user("nonexistent")
        assert success is False
        assert error_msg == "User not found"

    def test_mock_client_update_user_role(self):
        """Test mock client update user role."""
        client = MockOaaSClient()
        updated = client.update_user_role("user-002", "admin")
        assert updated is not None
        assert updated.role == "admin"
        assert client.list_users()[1].role == "admin"

    def test_mock_client_update_user_role_not_found(self):
        """Test mock client update role for non-existent user."""
        client = MockOaaSClient()
        updated = client.update_user_role("nonexistent", "admin")
        assert updated is None

    def test_mock_client_list_user_permissions(self):
        """Test mock client list user permissions."""
        client = MockOaaSClient()
        perms = client.list_user_permissions("user-002")
        assert len(perms) == 1
        assert perms[0]["service_id"] == "svc-002"

    def test_mock_client_list_user_permissions_empty(self):
        """Test mock client list permissions for user with none."""
        client = MockOaaSClient()
        perms = client.list_user_permissions("user-003")
        assert perms == []

    def test_mock_client_revoke_service_permission(self):
        """Test mock client revoke permission."""
        client = MockOaaSClient()
        success, error_msg = client.revoke_service_permission("svc-002", "user-002")
        assert success is True
        assert error_msg is None
        assert client.list_user_permissions("user-002") == []
        success, error_msg = client.revoke_service_permission("svc-002", "user-002")
        assert success is False
        assert error_msg == "Permission not found"

    def test_mock_client_list_services(self):
        """Test mock client list services."""
        client = MockOaaSClient()
        services = client.list_services()
        assert len(services) == 3
        assert services[0]["id"] == "svc-001"

    def test_mock_client_create_service(self):
        """Test mock client create service."""
        client = MockOaaSClient()
        result = client.create_service("new-svc", "desc", "usage", False)
        assert "id" in result
        assert result["name"] == "new-svc"
        assert "registration_token" in result
        assert len(client.list_services()) == 4

    def test_mock_client_delete_service(self):
        """Test mock client delete service."""
        from aaas_dashboard.client import DeleteServiceResult
        client = MockOaaSClient()
        # Create a new service with no associated tasks and delete it
        result = client.create_service("orphan-agent", "desc", "usage", False)
        service_id = result["id"]
        assert len(client.list_services()) == 4
        success, result = client.delete_service(service_id)
        assert success is True
        assert isinstance(result, DeleteServiceResult)
        assert result.deleted is True
        assert len(client.list_services()) == 3
        success, error_msg = client.delete_service("nonexistent")
        assert success is False
        assert error_msg == "Service not found"

    def test_mock_client_delete_service_with_associated_tasks(self):
        """Test mock client delete service with associated tasks returns error."""
        client = MockOaaSClient()
        # Find a service that has associated tasks by matching task.service_id to service.id
        service_ids_with_tasks = {t.service_id for t in client._tasks}
        service = next(
            (s for s in client._services if s["id"] in service_ids_with_tasks),
            None,
        )
        assert service is not None, "Need a service with associated tasks"
        success, error_msg = client.delete_service(service["id"])
        assert success is False
        assert "无法删除" in error_msg
        assert "任务关联此服务" in error_msg

    def test_mock_client_force_delete_service(self):
        """Test mock client force delete service with associated tasks."""
        from aaas_dashboard.client import DeleteServiceResult
        client = MockOaaSClient()
        # Find a service that has associated tasks
        service_ids_with_tasks = {t.service_id for t in client._tasks}
        service = next(
            (s for s in client._services if s["id"] in service_ids_with_tasks),
            None,
        )
        assert service is not None, "Need a service with associated tasks"

        # Count tasks before force delete
        pre_active = [t for t in client._tasks if t.service_id == service["id"] and t.status in (TaskStatus.PENDING, TaskStatus.RUNNING, TaskStatus.CANCELLING)]
        pre_retained = [t for t in client._tasks if t.service_id == service["id"] and t.status in (TaskStatus.COMPLETED, TaskStatus.FAILED, TaskStatus.CANCELLED)]

        # Grant a permission so we can verify it's removed
        client.grant_service_permission(service["id"], "user-002")
        assert any(p["service_id"] == service["id"] for p in client.list_user_permissions("user-002"))

        success, result = client.delete_service(service["id"], force=True)
        assert success is True
        assert isinstance(result, DeleteServiceResult)
        assert result.deleted is True
        assert result.tasks_cancelled == len(pre_active)
        assert result.tasks_retained == len(pre_retained)

        # Verify active tasks were cancelled
        for task in pre_active:
            assert task.status == TaskStatus.CANCELLED
            assert task.error_message == "Service was forcefully deleted by admin"
            assert task.completed_at is not None

        # Verify service removed
        assert not any(s["id"] == service["id"] for s in client.list_services())
        # Verify permissions removed
        assert not any(p["service_id"] == service["id"] for p in client.list_user_permissions("user-002"))

    def test_mock_client_grant_service_permission(self):
        """Test mock client grant permission."""
        client = MockOaaSClient()
        assert client.grant_service_permission("svc-003", "user-002") is True
        perms = client.list_user_permissions("user-002")
        assert any(p["service_id"] == "svc-003" for p in perms)
        # Duplicate grant returns False
        assert client.grant_service_permission("svc-003", "user-002") is False

    def test_mock_client_get_service_usage(self):
        """Test mock client get service usage."""
        client = MockOaaSClient()
        # Test existing service
        result = client.get_service_usage("svc-001")
        assert result is not None
        assert result["usage"] == "This service provides code analysis capabilities. You can submit source code files for review, ask for refactoring suggestions, or request bug detection."

        # Test non-existent service
        result = client.get_service_usage("nonexistent")
        assert result is None

    def test_mock_client_update_service(self):
        """Test mock client update service."""
        client = MockOaaSClient()
        result = client.update_service(
            "svc-001",
            name="updated-name",
            description="updated-desc",
            usage="updated-usage",
            is_public=False,
        )
        assert result is not None
        assert result["name"] == "updated-name"
        assert result["description"] == "updated-desc"
        assert result["usage"] == "updated-usage"
        assert result["is_public"] is False
        # Verify in-memory service was updated
        svc = next(s for s in client.list_services() if s["id"] == "svc-001")
        assert svc["name"] == "updated-name"
        assert svc["is_public"] is False

    def test_mock_client_update_service_partial(self):
        """Test mock client partial update."""
        client = MockOaaSClient()
        original = next(s for s in client.list_services() if s["id"] == "svc-001")
        original_name = original["name"]
        assert original_name != "only-name-changed"
        result = client.update_service("svc-001", name="only-name-changed")
        assert result is not None
        assert result["name"] == "only-name-changed"
        # Other fields should remain unchanged
        svc = next(s for s in client.list_services() if s["id"] == "svc-001")
        assert svc["description"] == original["description"]
        assert svc["usage"] == original["usage"]

    def test_mock_client_update_service_empty_payload(self):
        """Test mock client update with no fields returns current service."""
        client = MockOaaSClient()
        original = next(s for s in client.list_services() if s["id"] == "svc-001")
        original_name = original["name"]
        result = client.update_service("svc-001")
        assert result is not None
        assert result["id"] == "svc-001"
        assert result["name"] == original_name
        # Service should remain unchanged
        svc = next(s for s in client.list_services() if s["id"] == "svc-001")
        assert svc["name"] == original_name

    def test_mock_client_update_service_not_found(self):
        """Test mock client update non-existent service returns None."""
        client = MockOaaSClient()
        result = client.update_service("nonexistent", name="new-name")
        assert result is None
