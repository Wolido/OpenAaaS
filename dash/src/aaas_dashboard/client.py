"""API client for OpenAaaS server."""

from datetime import datetime
from pathlib import Path
from typing import Any, Optional, Union
from enum import Enum
import uuid
from urllib.parse import urljoin, urlparse

import requests
from pydantic import BaseModel, Field, ValidationError


class TaskStatus(str, Enum):
    """Task status enumeration."""
    PENDING = "pending"
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"
    CANCELLED = "cancelled"
    CANCELLING = "cancelling"


class Task(BaseModel):
    """Task model matching OpenAaaS Server TaskResponse."""

    id: str = Field(..., description="Task ID")
    user_id: Optional[str] = Field(None, description="User ID that created the task")
    user_name: Optional[str] = Field(None, description="User name that created the task")
    service_id: str = Field(..., description="Service ID (agent)")
    status: TaskStatus = Field(..., description="Task status")
    input: Optional[dict] = Field(None, description="Task input data (task_prompt, output_prompt)")
    output: Optional[dict] = Field(None, description="Task output data")
    error_message: Optional[str] = Field(None, description="Error message if failed")
    session_id: str = Field(..., description="Session ID")
    retry_count: int = Field(0, description="Retry count")
    created_at: datetime = Field(..., description="Task creation time")
    assigned_at: Optional[datetime] = Field(None, description="Task assigned time")
    started_at: Optional[datetime] = Field(None, description="Task started time")
    completed_at: Optional[datetime] = Field(None, description="Task completion time")

    # 辅助属性（从 input 中提取）
    @property
    def name(self) -> str:
        """Get task name from input."""
        if self.input and "task_prompt" in self.input:
            # 取前50字符作为名称
            prompt = self.input["task_prompt"]
            return prompt[:50] + "..." if len(prompt) > 50 else prompt
        return f"Task {self.id[:8]}"

    @property
    def progress(self) -> Optional[float]:
        """Calculate progress based on status."""
        if self.status == TaskStatus.COMPLETED:
            return 100.0
        elif self.status == TaskStatus.FAILED:
            return 0.0
        elif self.status == TaskStatus.PENDING:
            return 0.0
        elif self.status == TaskStatus.RUNNING:
            return 50.0  # 运行中默认50%，实际可以从 output 中获取
        return None

    class Config:
        """Pydantic config."""
        json_encoders = {
            datetime: lambda v: v.isoformat(),
        }


class User(BaseModel):
    """User model for OpenAaaS admin API."""

    id: str
    name: str
    api_key: str
    role: str
    created_at: datetime


class TaskFileInfo(BaseModel):
    """Task file metadata."""

    id: str
    task_id: str
    filename: str
    mime_type: Optional[str] = None
    size_bytes: int
    created_by: str
    created_at: datetime


class DeleteServiceResult(BaseModel):
    """Result model for delete service operation."""

    deleted: bool
    tasks_cancelled: int = 0
    tasks_retained: int = 0


class OaaSClient:
    """Client for interacting with OpenAaaS server API."""

    def __init__(self, server_url: str, api_key: Optional[str] = None):
        """Initialize the client.

        Args:
            server_url: Base URL of the OpenAaaS server
            api_key: Optional API key for authentication
        """
        self.server_url = server_url.rstrip("/")
        self.api_key = api_key
        self.last_error: Optional[str] = None
        self.session = requests.Session()

        if api_key:
            self.session.headers["Authorization"] = f"Bearer {api_key}"

    def _get(self, endpoint: str, **kwargs) -> dict[str, Any]:
        """Make a GET request to the API.

        Args:
            endpoint: API endpoint (without base URL)
            **kwargs: Additional arguments for requests

        Returns:
            JSON response as dictionary

        Raises:
            requests.RequestException: If the request fails
        """
        url = f"{self.server_url}{endpoint}"
        response = self.session.get(url, timeout=30, **kwargs)
        response.raise_for_status()
        return response.json()

    def _delete(self, endpoint: str, **kwargs) -> tuple[bool, Optional[str]]:
        """Make a DELETE request to the API.

        Args:
            endpoint: API endpoint (without base URL)
            **kwargs: Additional arguments for requests

        Returns:
            (True, None) if the request succeeds, (False, error_msg) otherwise
        """
        url = f"{self.server_url}{endpoint}"
        response = self.session.delete(url, timeout=30, **kwargs)
        if 200 <= response.status_code < 300:
            return True, None
        try:
            data = response.json()
            error_msg = data.get("message") or data.get("detail") or data.get("error")
        except (ValueError, AttributeError):
            error_msg = response.text
        return False, error_msg or response.reason or "Unknown error"

    def _put(self, endpoint: str, json_data: Optional[dict] = None, **kwargs) -> dict[str, Any]:
        """Make a PUT request to the API.

        Args:
            endpoint: API endpoint (without base URL)
            json_data: JSON payload
            **kwargs: Additional arguments for requests

        Returns:
            JSON response as dictionary
        """
        url = f"{self.server_url}{endpoint}"
        response = self.session.put(url, json=json_data, timeout=30, **kwargs)
        response.raise_for_status()
        return response.json()

    def _post(self, endpoint: str, json_data: Optional[dict] = None, **kwargs) -> dict[str, Any]:
        """Make a POST request to the API.

        Args:
            endpoint: API endpoint (without base URL)
            json_data: JSON payload
            **kwargs: Additional arguments for requests

        Returns:
            JSON response as dictionary
        """
        url = f"{self.server_url}{endpoint}"
        response = self.session.post(url, json=json_data, timeout=30, **kwargs)
        response.raise_for_status()
        return response.json()

    def health_check(self) -> dict[str, Any]:
        """Check server health.

        Returns:
            Health check response
        """
        try:
            return self._get("/health")
        except requests.RequestException as e:
            return {"status": "unhealthy", "error": str(e)}

    @staticmethod
    def _extract_list_payload(data: Any, keys: tuple[str, ...]) -> list[Any]:
        """Extract a list from common API response wrappers."""
        payload = data
        if isinstance(payload, dict):
            for key in keys:
                if key in payload:
                    payload = payload[key]
                    break
            else:
                return []

        # Some server versions return one extra list layer, e.g. {"tasks": [[...]]}.
        while (
            isinstance(payload, list)
            and len(payload) == 1
            and isinstance(payload[0], list)
        ):
            payload = payload[0]

        return payload if isinstance(payload, list) else []

    @classmethod
    def _extract_task_payload(cls, data: Any) -> Any:
        """Extract one task object from common create-task response shapes."""
        if isinstance(data, dict):
            for key in ("task", "tasks", "data", "item", "items"):
                if key in data:
                    return cls._extract_task_payload(data[key])
            return data

        while isinstance(data, list) and len(data) == 1:
            data = data[0]

        return data

    def list_tasks(
        self,
        status: Optional[TaskStatus] = None,
        limit: int = 100,
        offset: int = 0,
    ) -> list[Task]:
        """List tasks from the server.

        Args:
            status: Filter by status
            limit: Maximum number of tasks to return
            offset: Offset for pagination

        Returns:
            List of tasks
        """
        params = {"limit": limit, "offset": offset}
        if status:
            params["status"] = status.value

        try:
            data = self._get("/api/v1/client/tasks", params=params)
            tasks_data = self._extract_list_payload(data, ("tasks", "data", "items"))
            return [Task.model_validate(t) for t in tasks_data]
        except requests.RequestException as e:
            print(f"[ERROR] Network error in list_tasks: {e}")
            return []
        except ValidationError as e:
            # 打印验证错误以便调试
            print(f"[ERROR] Task validation failed: {e}")
            return []
        except Exception as e:
            # 捕获其他所有错误
            print(f"[ERROR] Unexpected error in list_tasks: {e}")
            return []

    def list_admin_tasks(
        self,
        status: Optional[TaskStatus] = None,
        user_id: Optional[str] = None,
        service_id: Optional[str] = None,
        limit: int = 100,
        offset: int = 0,
    ) -> list[Task]:
        """List tasks across users using the admin API."""
        params = {"limit": limit, "offset": offset}
        if status:
            params["status"] = status.value
        if user_id:
            params["user_id"] = user_id
        if service_id:
            params["service_id"] = service_id

        try:
            data = self._get("/api/v1/admin/tasks", params=params)
            tasks_data = self._extract_list_payload(data, ("tasks", "data", "items"))
            return [Task.model_validate(t) for t in tasks_data]
        except requests.RequestException as e:
            print(f"[ERROR] Network error in list_admin_tasks: {e}")
            return []
        except ValidationError as e:
            print(f"[ERROR] Admin task validation failed: {e}")
            return []
        except Exception as e:
            print(f"[ERROR] Unexpected error in list_admin_tasks: {e}")
            return []

    def get_task(self, task_id: str) -> Optional[Task]:
        """Get a specific task by ID.

        Args:
            task_id: Task ID

        Returns:
            Task if found, None otherwise
        """
        try:
            data = self._get(f"/api/v1/client/tasks/{task_id}")
            return Task.model_validate(data)
        except requests.RequestException:
            return None

    def cancel_task(self, task_id: str) -> bool:
        """Cancel a running task.

        Args:
            task_id: Task ID to cancel

        Returns:
            True if cancelled successfully
        """
        try:
            url = f"{self.server_url}/api/v1/client/tasks/{task_id}/cancel"
            response = self.session.post(url, timeout=30)
            return 200 <= response.status_code < 300
        except requests.RequestException:
            return False

    def _post_multipart_with_redirects(
        self,
        url: str,
        opened_files: Optional[list[Any]] = None,
        timeout: int = 60,
        max_redirects: int = 3,
        **request_kwargs,
    ) -> tuple[requests.Response, str, int]:
        """POST multipart with manual redirect following.

        Uses allow_redirects=False and follows 3xx redirects manually,
        preserving POST method and rewinding file handles.
        Authorization header is stripped if redirect crosses domains.
        """
        original_url = url
        redirect_count = 0
        response = None

        while True:
            # Rewind file handles before each POST
            if opened_files:
                for fh in opened_files:
                    if hasattr(fh, "seek"):
                        fh.seek(0)

            # Strip Authorization header on cross-domain redirect
            req_headers = dict(request_kwargs.get("headers", {})) if "headers" in request_kwargs else {}
            if urlparse(url).netloc != urlparse(original_url).netloc:
                req_headers["Authorization"] = None

            response = self.session.post(
                url,
                timeout=timeout,
                allow_redirects=False,
                headers=req_headers if req_headers else None,
                **{k: v for k, v in request_kwargs.items() if k != "headers"},
            )

            if 300 <= response.status_code < 400 and redirect_count < max_redirects:
                location = response.headers.get("Location", "")
                if location:
                    url = urljoin(url, location)
                    redirect_count += 1
                    continue
            break

        return response, url, redirect_count

    def create_task(
        self,
        service_id: str,
        task_prompt: str,
        output_prompt: str,
        session_id: Optional[str] = None,
        files: Optional[list[Any]] = None,
    ) -> Optional[Task]:
        """Create a task via multipart/form-data.

        Args:
            service_id: Target service ID
            task_prompt: User task prompt
            output_prompt: Output formatting prompt
            session_id: Optional session ID for conversation reuse
            files: Optional Streamlit uploaded files or file-like objects

        Returns:
            Created task if successful, None otherwise
        """
        url = f"{self.server_url}/api/v1/client/tasks"
        multipart_fields: list[tuple[str, tuple[None, str]]] = [
            ("service_id", (None, service_id)),
            ("task_prompt", (None, task_prompt)),
            ("output_prompt", (None, output_prompt)),
        ]
        if session_id:
            multipart_fields.append(("session_id", (None, session_id)))

        multipart_files: list[tuple[str, tuple[str, Any, str]]] = []
        # 额外保存一份纯 bytes 文件数据，供 multipart 解析失败时回退重试
        fallback_file_parts: list[tuple[str, bytes, str]] = []
        opened_files: list[Any] = []

        self.last_error = None
        try:
            for file_obj in files or []:
                if file_obj is None:
                    continue

                content_type = getattr(file_obj, "type", None) or "application/octet-stream"
                name = getattr(file_obj, "name", None) or "upload.bin"

                if hasattr(file_obj, "getvalue"):
                    payload = file_obj.getvalue()
                    multipart_files.append(("files", (name, payload, content_type)))
                    fallback_file_parts.append((name, payload, content_type))
                else:
                    fh = open(Path(file_obj), "rb")
                    opened_files.append(fh)
                    filename = Path(file_obj).name
                    data = fh.read()
                    fh.seek(0)
                    multipart_files.append(("files", (filename, fh, content_type)))
                    fallback_file_parts.append((filename, data, content_type))

            response, url, redirect_count = self._post_multipart_with_redirects(
                url,
                files=multipart_fields + multipart_files,
                opened_files=opened_files,
            )

            if 300 <= response.status_code < 400:
                location = response.headers.get("Location", "")
                self.last_error = (
                    f"Server URL redirected to {location or '<unknown>'} "
                    f"after {redirect_count} redirects. "
                    "Use the final HTTPS Server URL directly."
                )
                print(f"[ERROR] Failed to create task: {self.last_error}")
                return None
            response.raise_for_status()
            return Task.model_validate(self._extract_task_payload(response.json()))
        except (requests.RequestException, ValidationError) as e:
            # 部分环境下 requests 生成的 multipart 可能被服务端解析失败，这里回退为手工 multipart。
            if isinstance(e, requests.RequestException):
                resp = getattr(e, "response", None)
                body_text = getattr(resp, "text", "") if resp is not None else ""
                status_code = getattr(resp, "status_code", None)
                if (
                    status_code == 400
                    and "multipart/form-data" in (body_text or "")
                    and "Error parsing" in (body_text or "")
                ):
                    try:
                        boundary = f"----openaaas-{uuid.uuid4().hex}"
                        lines: list[bytes] = []

                        def add_field(key: str, value: str) -> None:
                            lines.append(f"--{boundary}\r\n".encode("utf-8"))
                            lines.append(
                                f'Content-Disposition: form-data; name="{key}"\r\n\r\n'.encode("utf-8")
                            )
                            lines.append(value.encode("utf-8"))
                            lines.append(b"\r\n")

                        add_field("service_id", service_id)
                        add_field("task_prompt", task_prompt)
                        add_field("output_prompt", output_prompt)
                        if session_id:
                            add_field("session_id", session_id)

                        for filename, payload, content_type in fallback_file_parts:
                            lines.append(f"--{boundary}\r\n".encode("utf-8"))
                            lines.append(
                                (
                                    f'Content-Disposition: form-data; name="files"; '
                                    f'filename="{filename}"\r\n'
                                ).encode("utf-8")
                            )
                            lines.append(f"Content-Type: {content_type}\r\n\r\n".encode("utf-8"))
                            lines.append(payload)
                            lines.append(b"\r\n")

                        lines.append(f"--{boundary}--\r\n".encode("utf-8"))
                        manual_body = b"".join(lines)
                        manual_headers = {
                            "Content-Type": f"multipart/form-data; boundary={boundary}",
                        }
                        manual_resp, url, redirect_count = self._post_multipart_with_redirects(
                            url,
                            data=manual_body,
                            headers=manual_headers,
                        )

                        if 300 <= manual_resp.status_code < 400:
                            location = manual_resp.headers.get("Location", "")
                            self.last_error = (
                                f"Server URL redirected to {location or '<unknown>'} "
                                f"after {redirect_count} redirects. "
                                "Use the final HTTPS Server URL directly."
                            )
                            print(f"[ERROR] Failed to create task: {self.last_error}")
                            return None
                        manual_resp.raise_for_status()
                        self.last_error = None
                        return Task.model_validate(self._extract_task_payload(manual_resp.json()))
                    except (requests.RequestException, ValidationError) as fallback_exc:
                        e = fallback_exc

            if isinstance(e, requests.RequestException):
                status = getattr(getattr(e, "response", None), "status_code", None)
                body = getattr(getattr(e, "response", None), "text", "")
                if status is not None:
                    detail = body[:800] if body else str(e)
                    self.last_error = f"HTTP {status}: {detail}"
                else:
                    self.last_error = str(e)
            else:
                self.last_error = str(e)
            print(f"[ERROR] Failed to create task: {self.last_error}")
            return None
        finally:
            for fh in opened_files:
                fh.close()

    def list_task_files(self, task_id: str) -> list[TaskFileInfo]:
        """List files associated with a task."""
        try:
            data = self._get(f"/api/v1/client/files/list/{task_id}")
            files = data.get("files", []) if isinstance(data, dict) else data
            return [TaskFileInfo.model_validate(item) for item in files]
        except (requests.RequestException, ValidationError) as e:
            print(f"[ERROR] Failed to list task files: {e}")
            return []

    def download_file_text(self, file_id: str) -> Optional[str]:
        """Download a text-based task file."""
        try:
            url = f"{self.server_url}/api/v1/client/files/{file_id}/download"
            response = self.session.get(url, timeout=60)
            response.raise_for_status()
            response.encoding = response.encoding or "utf-8"
            return response.text
        except requests.RequestException as e:
            print(f"[ERROR] Failed to download file {file_id}: {e}")
            return None

    def download_file_bytes(self, file_id: str) -> Optional[bytes]:
        """Download a task file as raw bytes."""
        try:
            url = f"{self.server_url}/api/v1/client/files/{file_id}/download"
            response = self.session.get(url, timeout=60)
            response.raise_for_status()
            return response.content
        except requests.RequestException as e:
            print(f"[ERROR] Failed to download file bytes {file_id}: {e}")
            return None

    def get_stats(self) -> dict[str, Any]:
        """Get task statistics by counting tasks from list."""
        try:
            tasks = self.list_tasks(limit=10000)

            return {
                "total": len(tasks),
                "pending": sum(1 for t in tasks if t.status == TaskStatus.PENDING),
                "running": sum(1 for t in tasks if t.status == TaskStatus.RUNNING),
                "completed": sum(1 for t in tasks if t.status == TaskStatus.COMPLETED),
                "failed": sum(1 for t in tasks if t.status == TaskStatus.FAILED),
                "cancelled": sum(1 for t in tasks if t.status == TaskStatus.CANCELLED),
                "cancelling": sum(1 for t in tasks if t.status == TaskStatus.CANCELLING),
            }
        except Exception:
            return {
                "total": 0, "pending": 0, "running": 0,
                "completed": 0, "failed": 0, "cancelled": 0, "cancelling": 0,
            }

    # Admin API methods

    def list_users(self) -> list[User]:
        """List all users (admin only)."""
        try:
            data = self._get("/api/v1/admin/users")
            if isinstance(data, list):
                users_data = data
            else:
                users_data = data.get("users", [])
            return [User.model_validate(u) for u in users_data]
        except requests.RequestException as e:
            print(f"[ERROR] Network error in list_users: {e}")
            return []
        except ValidationError as e:
            print(f"[ERROR] User validation failed: {e}")
            return []
        except Exception as e:
            print(f"[ERROR] Unexpected error in list_users: {e}")
            return []

    def delete_user(self, user_id: str) -> tuple[bool, Optional[str]]:
        """Delete a user (admin only)."""
        try:
            return self._delete(f"/api/v1/admin/users/{user_id}")
        except requests.RequestException as e:
            print(f"[ERROR] Network error in delete_user: {e}")
            return False, str(e)

    def update_user_role(self, user_id: str, role: str) -> Optional[User]:
        """Update a user's role (admin only)."""
        try:
            data = self._put(f"/api/v1/admin/users/{user_id}/role", json_data={"role": role})
            return User.model_validate(data)
        except requests.RequestException as e:
            print(f"[ERROR] Network error in update_user_role: {e}")
            return None
        except ValidationError as e:
            print(f"[ERROR] User validation failed in update_user_role: {e}")
            return None

    def list_user_permissions(self, user_id: str) -> list[dict]:
        """List a user's service permissions (admin only)."""
        try:
            data = self._get(f"/api/v1/admin/users/{user_id}/permissions")
            if isinstance(data, list):
                return data
            return data.get("permissions", [])
        except requests.RequestException as e:
            print(f"[ERROR] Network error in list_user_permissions: {e}")
            return []

    def revoke_service_permission(self, service_id: str, user_id: str) -> tuple[bool, Optional[str]]:
        """Revoke a user's permission to a service (admin only)."""
        try:
            return self._delete(f"/api/v1/admin/services/{service_id}/users/{user_id}")
        except requests.RequestException as e:
            print(f"[ERROR] Network error in revoke_service_permission: {e}")
            return False, str(e)

    def list_services(self) -> list[dict]:
        """List all services (admin perspective)."""
        try:
            data = self._get("/api/v1/services")
            if isinstance(data, list):
                return data
            return data.get("services", [])
        except requests.RequestException as e:
            print(f"[ERROR] Network error in list_services: {e}")
            return []

    def list_client_services(self) -> list[dict]:
        """List services from the normal client endpoint."""
        try:
            data = self._get("/api/v1/client/services")
            if isinstance(data, list):
                return data
            return data.get("services", [])
        except requests.RequestException as e:
            print(f"[ERROR] Network error in list_client_services: {e}")
            return []

    def create_service(
        self,
        name: str,
        description: str,
        usage: str,
        is_public: bool = False,
    ) -> Optional[dict]:
        """Create a new service (admin only)."""
        payload = {
            "name": name,
            "description": description,
            "usage": usage,
            "is_public": is_public,
        }
        try:
            return self._post("/api/v1/services", json_data=payload)
        except requests.RequestException as e:
            print(f"[ERROR] Network error in create_service: {e}")
            return None

    def get_service_usage(self, service_id: str) -> Optional[dict]:
        """Get the usage description for a specific service."""
        try:
            return self._get(f"/api/v1/client/services/{service_id}/usage")
        except requests.RequestException as e:
            print(f"[ERROR] Network error in get_service_usage: {e}")
            return None

    def delete_service(self, service_id: str, force: bool = False) -> tuple[bool, Optional[Union[str, DeleteServiceResult]]]:
        """Delete a service (admin only).

        Args:
            service_id: Service ID to delete
            force: If True, forcefully delete the service even if it has associated tasks.
                   Active tasks will be cancelled and the response will include
                   tasks_cancelled and tasks_retained counts.
        """
        try:
            url = f"{self.server_url}/api/v1/services/{service_id}"
            params = {}
            if force:
                params["force"] = "true"
            response = self.session.delete(url, params=params, timeout=30)
            if 200 <= response.status_code < 300:
                data = response.json()
                return True, DeleteServiceResult(**data)
            try:
                data = response.json()
                error_msg = data.get("message") or data.get("detail") or data.get("error")
            except (ValueError, AttributeError):
                error_msg = response.text
            return False, error_msg or response.reason or "Unknown error"
        except requests.RequestException as e:
            print(f"[ERROR] Network error in delete_service: {e}")
            return False, str(e)

    def grant_service_permission(self, service_id: str, user_id: str) -> bool:
        """Grant a user permission to a restricted service (admin only)."""
        try:
            self._post(f"/api/v1/client/services/{service_id}/grant", json_data={"user_id": user_id})
            return True
        except requests.RequestException as e:
            print(f"[ERROR] Network error in grant_service_permission: {e}")
            return False

    def update_service(
        self,
        service_id: str,
        name: Optional[str] = None,
        description: Optional[str] = None,
        usage: Optional[str] = None,
        is_public: Optional[bool] = None,
    ) -> Optional[dict]:
        """Update a service (admin only).

        Args:
            service_id: Service ID to update
            name: New service name
            description: New description
            usage: New usage description
            is_public: New public status

        Returns:
            Updated service dict if successful, None otherwise
        """
        payload: dict[str, Any] = {}
        if name is not None:
            payload["name"] = name
        if description is not None:
            payload["description"] = description
        if usage is not None:
            payload["usage"] = usage
        if is_public is not None:
            payload["is_public"] = is_public

        if not payload:
            # No fields to update, fetch current service info
            try:
                return self._get(f"/api/v1/services/{service_id}")
            except requests.RequestException as e:
                print(f"[ERROR] Network error in update_service: {e}")
                return None

        try:
            return self._put(f"/api/v1/services/{service_id}", json_data=payload)
        except requests.RequestException as e:
            print(f"[ERROR] Network error in update_service: {e}")
            return None


class MockOaaSClient(OaaSClient):
    """Mock client for testing without a real server."""

    def __init__(self, server_url: str = "http://mock", api_key: Optional[str] = None):
        """Initialize mock client."""
        self.server_url = server_url
        self.api_key = api_key
        self.session = requests.Session()
        self._tasks: list[Task] = []
        self._users: list[User] = []
        self._services: list[dict] = []
        self._permissions: dict[str, list[str]] = {}  # user_id -> list of service_ids
        self._generate_mock_users()
        self._generate_mock_services()
        self._generate_mock_tasks()

    def _generate_mock_tasks(self) -> None:
        """Generate mock tasks for demonstration."""
        from datetime import timedelta
        import random

        now = datetime.now()
        statuses = [TaskStatus.PENDING, TaskStatus.RUNNING, TaskStatus.COMPLETED, TaskStatus.FAILED, TaskStatus.CANCELLED, TaskStatus.CANCELLING]
        service_ids = [s["id"] for s in self._services] or [
            "code-agent", "data-agent", "analysis-agent", "review-agent"
        ]

        for i in range(20):
            status = random.choice(statuses)
            created_at = now - timedelta(hours=random.randint(0, 48))
            updated_at = created_at + timedelta(minutes=random.randint(1, 60))
            completed_at = None
            user = random.choice(self._users) if self._users else None

            if status in [TaskStatus.COMPLETED, TaskStatus.FAILED]:
                completed_at = updated_at + timedelta(minutes=random.randint(1, 30))

            task = Task(
                id=f"task-{i+1:04d}",
                user_id=user.id if user else None,
                user_name=user.name if user else None,
                service_id=random.choice(service_ids),
                status=status,
                created_at=created_at,
                assigned_at=created_at if status != TaskStatus.PENDING else None,
                started_at=updated_at if status in [TaskStatus.RUNNING, TaskStatus.COMPLETED, TaskStatus.FAILED] else None,
                completed_at=completed_at,
                input={"task_prompt": f"Input data for task {i+1}"},
                output={"result": f"Output result for task {i+1}"} if status == TaskStatus.COMPLETED else None,
                error_message="Connection timeout" if status == TaskStatus.FAILED else None,
                session_id=f"session-{i}",
                retry_count=0,
            )
            self._tasks.append(task)

    def _generate_mock_users(self) -> None:
        """Generate mock users."""
        now = datetime.now()
        self._users = [
            User(
                id="user-001",
                name="Admin User",
                api_key="ak_admin_001",
                role="admin",
                created_at=now,
            ),
            User(
                id="user-002",
                name="Regular User",
                api_key="ak_user_002",
                role="client",
                created_at=now,
            ),
            User(
                id="user-003",
                name="Another Client",
                api_key="ak_user_003",
                role="client",
                created_at=now,
            ),
        ]

    def _generate_mock_services(self) -> None:
        """Generate mock services."""
        self._services = [
            {
                "id": "svc-001",
                "name": "code-agent",
                "description": "Code analysis agent",
                "usage": "This service provides code analysis capabilities. You can submit source code files for review, ask for refactoring suggestions, or request bug detection.",
                "agent_status": "online",
                "registration_status": "active",
                "is_public": True,
            },
            {
                "id": "svc-002",
                "name": "data-agent",
                "description": "Data processing agent",
                "usage": "This service handles data processing tasks. Submit structured data files for transformation, aggregation, or statistical analysis.",
                "agent_status": "online",
                "registration_status": "active",
                "is_public": False,
            },
            {
                "id": "svc-003",
                "name": "review-agent",
                "description": "Code review agent",
                "usage": "This service performs automated code reviews. Submit pull requests or code snippets to receive feedback on style, security, and performance.",
                "agent_status": "offline",
                "registration_status": "pending",
                "is_public": False,
            },
        ]
        self._permissions = {
            "user-002": ["svc-002"],
        }

    def health_check(self) -> dict[str, Any]:
        """Mock health check."""
        return {"status": "healthy", "version": "0.1.0-mock"}

    def list_tasks(
        self,
        status: Optional[TaskStatus] = None,
        limit: int = 100,
        offset: int = 0,
    ) -> list[Task]:
        """List mock tasks."""
        tasks = self._tasks
        if status:
            tasks = [t for t in tasks if t.status == status]
        return tasks[offset:offset + limit]

    def get_task(self, task_id: str) -> Optional[Task]:
        """Get mock task."""
        for task in self._tasks:
            if task.id == task_id:
                return task
        return None

    def cancel_task(self, task_id: str) -> bool:
        """Mock cancel task."""
        for task in self._tasks:
            if task.id == task_id and task.status == TaskStatus.RUNNING:
                task.status = TaskStatus.CANCELLED
                return True
        return False

    def create_task(
        self,
        service_id: str,
        task_prompt: str,
        output_prompt: str,
        session_id: Optional[str] = None,
        files: Optional[list[Any]] = None,
    ) -> Optional[Task]:
        """Create a mock task."""
        from uuid import uuid4

        task = Task(
            id=f"task-{uuid4().hex[:8]}",
            service_id=service_id,
            status=TaskStatus.PENDING,
            created_at=datetime.now(),
            assigned_at=None,
            started_at=None,
            completed_at=None,
            input={
                "task_prompt": task_prompt,
                "output_prompt": output_prompt,
                "input_files": [getattr(f, "name", "upload.bin") for f in (files or [])],
            },
            output=None,
            error_message=None,
            session_id=session_id or f"session-{uuid4().hex[:8]}",
            retry_count=0,
        )
        self._tasks.insert(0, task)
        return task

    def list_task_files(self, task_id: str) -> list[TaskFileInfo]:
        """List mock files for a task."""
        task = self.get_task(task_id)
        if not task or not task.output or "files" not in task.output:
            return []

        files = []
        for idx, filename in enumerate(task.output.get("files", [])):
            files.append(
                TaskFileInfo(
                    id=f"file-{task_id}-{idx}",
                    task_id=task_id,
                    filename=filename,
                    mime_type="text/markdown" if filename.endswith(".md") else "text/plain",
                    size_bytes=0,
                    created_by="agent",
                    created_at=datetime.now(),
                )
            )
        return files

    def download_file_text(self, file_id: str) -> Optional[str]:
        """Download a mock file."""
        return "Mock file content"

    def download_file_bytes(self, file_id: str) -> Optional[bytes]:
        """Download a mock file as bytes."""
        return b"Mock file content"

    def get_stats(self) -> dict[str, Any]:
        """Get mock stats."""
        stats = {
            "total": len(self._tasks),
            "pending": sum(1 for t in self._tasks if t.status == TaskStatus.PENDING),
            "running": sum(1 for t in self._tasks if t.status == TaskStatus.RUNNING),
            "completed": sum(1 for t in self._tasks if t.status == TaskStatus.COMPLETED),
            "failed": sum(1 for t in self._tasks if t.status == TaskStatus.FAILED),
            "cancelled": sum(1 for t in self._tasks if t.status == TaskStatus.CANCELLED),
            "cancelling": sum(1 for t in self._tasks if t.status == TaskStatus.CANCELLING),
        }
        return stats

    # Mock admin API methods

    def list_users(self) -> list[User]:
        """List mock users."""
        return list(self._users)

    def list_admin_tasks(
        self,
        status: Optional[TaskStatus] = None,
        user_id: Optional[str] = None,
        service_id: Optional[str] = None,
        limit: int = 100,
        offset: int = 0,
    ) -> list[Task]:
        """List mock tasks with admin-style filters."""
        tasks = list(self._tasks)
        if status:
            tasks = [task for task in tasks if task.status == status]
        if user_id:
            tasks = [task for task in tasks if task.user_id == user_id]
        if service_id:
            tasks = [task for task in tasks if task.service_id == service_id]
        return tasks[offset: offset + limit]

    def delete_user(self, user_id: str) -> tuple[bool, Optional[str]]:
        """Delete a mock user."""
        original_len = len(self._users)
        self._users = [u for u in self._users if u.id != user_id]
        if len(self._users) < original_len:
            return True, None
        return False, "User not found"

    def update_user_role(self, user_id: str, role: str) -> Optional[User]:
        """Update a mock user's role."""
        for user in self._users:
            if user.id == user_id:
                # Create updated user with new role
                updated = user.model_copy(update={"role": role})
                idx = self._users.index(user)
                self._users[idx] = updated
                return updated
        return None

    def list_user_permissions(self, user_id: str) -> list[dict]:
        """List mock user permissions."""
        service_ids = self._permissions.get(user_id, [])
        result = []
        for sid in service_ids:
            for svc in self._services:
                if svc["id"] == sid:
                    result.append({
                        "service_id": sid,
                        "service_name": svc["name"],
                    })
        return result

    def revoke_service_permission(self, service_id: str, user_id: str) -> tuple[bool, Optional[str]]:
        """Revoke a mock permission."""
        if user_id in self._permissions:
            if service_id in self._permissions[user_id]:
                self._permissions[user_id].remove(service_id)
                return True, None
        return False, "Permission not found"

    def list_services(self) -> list[dict]:
        """List mock services."""
        return [dict(s) for s in self._services]

    def list_client_services(self) -> list[dict]:
        """List mock services for normal users."""
        return [dict(s) for s in self._services]

    def create_service(
        self,
        name: str,
        description: str,
        usage: str,
        is_public: bool = False,
    ) -> Optional[dict]:
        """Create a mock service."""
        import uuid
        service_id = f"svc-{uuid.uuid4().hex[:8]}"
        service = {
            "id": service_id,
            "name": name,
            "description": description,
            "usage": usage,
            "is_public": is_public,
            "agent_status": "offline",
            "registration_status": "pending",
            "registration_token": f"token-{uuid.uuid4().hex}",
        }
        self._services.append(service)
        return service

    def get_service_usage(self, service_id: str) -> Optional[dict]:
        """Get usage description for a mock service."""
        service = next((s for s in self._services if s["id"] == service_id), None)
        if service is None:
            return None
        return {
            "id": service["id"],
            "name": service["name"],
            "usage": service.get("usage", ""),
        }

    def delete_service(self, service_id: str, force: bool = False) -> tuple[bool, Optional[Union[str, DeleteServiceResult]]]:
        """Delete a mock service."""
        service = next((s for s in self._services if s["id"] == service_id), None)
        if service is None:
            return False, "Service not found"

        associated_tasks = [t for t in self._tasks if t.service_id == service["id"]]

        if not force and associated_tasks:
            return False, f"无法删除：还有 {len(associated_tasks)} 个任务关联此服务"

        if force:
            # Count and cancel active tasks
            active_tasks = [t for t in associated_tasks if t.status in (TaskStatus.PENDING, TaskStatus.RUNNING, TaskStatus.CANCELLING)]
            retained_tasks = [t for t in associated_tasks if t.status in (TaskStatus.COMPLETED, TaskStatus.FAILED, TaskStatus.CANCELLED)]

            for task in active_tasks:
                task.status = TaskStatus.CANCELLED
                task.error_message = "Service was forcefully deleted by admin"
                task.completed_at = datetime.now()

            # Remove permissions for this service
            for user_id in list(self._permissions.keys()):
                if service_id in self._permissions[user_id]:
                    self._permissions[user_id].remove(service_id)

            self._services = [s for s in self._services if s["id"] != service_id]
            return True, DeleteServiceResult(
                deleted=True,
                tasks_cancelled=len(active_tasks),
                tasks_retained=len(retained_tasks),
            )

        self._services = [s for s in self._services if s["id"] != service_id]
        return True, DeleteServiceResult(deleted=True)

    def grant_service_permission(self, service_id: str, user_id: str) -> bool:
        """Grant a mock permission."""
        if user_id not in self._permissions:
            self._permissions[user_id] = []
        if service_id not in self._permissions[user_id]:
            self._permissions[user_id].append(service_id)
            return True
        return False

    def update_service(
        self,
        service_id: str,
        name: Optional[str] = None,
        description: Optional[str] = None,
        usage: Optional[str] = None,
        is_public: Optional[bool] = None,
    ) -> Optional[dict]:
        """Update a mock service."""
        service = next((s for s in self._services if s["id"] == service_id), None)
        if service is None:
            return None

        if name is not None:
            service["name"] = name
        if description is not None:
            service["description"] = description
        if usage is not None:
            service["usage"] = usage
        if is_public is not None:
            service["is_public"] = is_public

        return dict(service)
