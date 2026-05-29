"""Admin management page for OpenAaaS Dashboard."""

import streamlit as st

from aaas_dashboard.components import (
    get_or_create_client,
    init_session_state,
    render_sidebar,
    sync_sidebar_state,
    sync_sidebar_state_for_page,
)
from aaas_dashboard.config import get_config

st.set_page_config(
    page_title="Admin Management - OpenAaaS",
    page_icon="🔧",
    layout="wide",
    initial_sidebar_state="expanded",
)

init_session_state()

if st.session_state.config is None:
    st.session_state.config = get_config()
    sync_sidebar_state(st.session_state.config)

config = st.session_state.config
sync_sidebar_state_for_page(config, "admin")
new_config, _ = render_sidebar(config)

if new_config.server_url != config.server_url or new_config.api_key != config.api_key:
    st.session_state.config = new_config
    st.session_state.client = None
    config = new_config

client = get_or_create_client(config)

st.title("🔧 Admin Management")

# Determine if current user is admin
current_user_id = None

if st.session_state.use_mock:
    current_user_id = "user-001"
    try:
        users = client.list_users()
        services = client.list_services()
        all_tasks = client.list_tasks(limit=10000)
    except Exception:
        st.error("Admin access required.")
        st.stop()
else:
    try:
        users = client.list_users()
        services = client.list_services()
        all_tasks = client.list_tasks(limit=10000)
    except Exception:
        st.error("Admin access required.")
        st.stop()
    # Try to find current user by matching API key
    for u in users:
        if u.api_key == config.api_key:
            current_user_id = u.id
            break

tab_services, tab_users, tab_permissions = st.tabs([
    "🛠️ Service Management",
    "👤 User Management",
    "🔐 Permission Management",
])

# ==================== Tab 1: Service Management ====================
with tab_services:
    st.subheader("All Services")

    if not services:
        st.info("No services found.")
    else:
        for svc in services:
            with st.container():
                svc_id = svc.get('id')
                service_name = svc.get("name", "-") or "-"
                if not svc_id:
                    continue
                svc_tasks = [t for t in all_tasks if t.service_id == svc_id]
                pending_count = sum(1 for t in svc_tasks if t.status == "pending")
                running_count = sum(1 for t in svc_tasks if t.status == "running")
                cancelling_count = sum(1 for t in svc_tasks if t.status == "cancelling")
                active_count = pending_count + running_count + cancelling_count
                retained_count = sum(1 for t in svc_tasks if t.status in ("completed", "failed", "cancelled"))

                col1, col2, col3, col4, col5, col6 = st.columns([2, 2, 3, 1.5, 1, 1.5])
                with col1:
                    st.markdown(f"**{service_name}**")
                    st.caption(f"ID: `{svc_id or '-'}`")
                with col2:
                    st.write(svc.get('description', '-') or '-')
                with col3:
                    st.write(f"Agent: `{svc.get('agent_status', '-')}`")
                    st.write(f"Registration: `{svc.get('registration_status', '-')}`")
                with col4:
                    is_public = svc.get('is_public', False)
                    st.write("🌍 Public" if is_public else "🔒 Restricted")
                with col6:
                    if st.button("✏️ Edit", key=f"edit_svc_{svc_id}"):
                        st.session_state[f"edit_svc_open_{svc_id}"] = True
                        st.session_state[f"delete_svc_confirming_{svc_id}"] = False

                    if st.button("🗑️ Delete", key=f"delete_svc_{svc_id}"):
                        st.session_state[f"delete_svc_confirming_{svc_id}"] = True
                        st.session_state[f"edit_svc_open_{svc_id}"] = False

                if st.session_state.get(f"delete_svc_confirming_{svc_id}", False):
                    st.warning("Delete will remove this service only if the server allows it. It will not force-cancel associated tasks.")
                    st.write(f"**Service to delete:** `{service_name}`")
                    st.caption(f"Service ID: `{svc_id}`")
                    st.write(f"Associated tasks: **{len(svc_tasks)}**")
                    st.write(f"Unfinished tasks: **{active_count}** (Pending {pending_count}, Running {running_count}, Cancelling {cancelling_count})")
                    st.caption("Type DELETE to confirm normal deletion.")

                    delete_confirm = st.text_input(
                        "Confirm normal delete",
                        key=f"delete_svc_confirm_text_{svc_id}",
                        placeholder="DELETE",
                    )
                    delete_cols = st.columns([1.2, 1, 3.8])
                    with delete_cols[0]:
                        if st.button(
                            "Confirm Delete",
                            key=f"confirm_delete_svc_{svc_id}",
                            type="primary",
                            use_container_width=True,
                            disabled=delete_confirm != "DELETE",
                        ):
                            success, error_msg = client.delete_service(svc_id)
                            if success:
                                st.success("Deleted!")
                                st.rerun()
                            else:
                                st.error(error_msg or "Failed to delete service.")
                    with delete_cols[1]:
                        if st.button("Cancel", key=f"cancel_delete_svc_{svc_id}", use_container_width=True):
                            st.session_state[f"delete_svc_confirming_{svc_id}"] = False
                            st.rerun()

                # Edit form
                if st.session_state.get(f"edit_svc_open_{svc_id}", False):
                    with st.container(border=True):
                        st.markdown(f"**✏️ Edit Service: `{service_name}`**")
                        original_name = svc.get("name") or ""
                        original_desc = svc.get("description") or ""
                        original_usage = svc.get("usage") or ""
                        original_public = svc.get("is_public") or False

                        edit_name = st.text_input("Name", value=original_name, key=f"edit_name_{svc_id}")
                        edit_desc = st.text_area("Description", value=original_desc, key=f"edit_desc_{svc_id}")
                        edit_usage = st.text_area("Usage", value=original_usage, key=f"edit_usage_{svc_id}")
                        edit_public = st.checkbox("Public", value=original_public, key=f"edit_public_{svc_id}")

                        edit_cols = st.columns([1.2, 1, 3.8])
                        with edit_cols[0]:
                            if st.button("💾 Save", key=f"save_edit_{svc_id}", type="primary", use_container_width=True):
                                updated = client.update_service(
                                    service_id=svc_id,
                                    name=edit_name if edit_name != original_name else None,
                                    description=edit_desc if edit_desc != original_desc else None,
                                    usage=edit_usage if edit_usage != original_usage else None,
                                    is_public=edit_public if edit_public != original_public else None,
                                )
                                if updated:
                                    st.success("Service updated!")
                                    st.session_state[f"edit_svc_open_{svc_id}"] = False
                                    st.rerun()
                                else:
                                    st.error("Failed to update service.")
                        with edit_cols[1]:
                            if st.button("Cancel", key=f"cancel_edit_{svc_id}", use_container_width=True):
                                st.session_state[f"edit_svc_open_{svc_id}"] = False
                                st.rerun()

                # Force delete expander on a new full-width row
                with st.expander("⚠️ Force Delete", expanded=False):
                    st.warning("Force Delete will permanently delete this service and cancel its unfinished tasks.")
                    st.write(f"**Service to delete:** `{service_name}`")
                    st.caption(f"Service ID: `{svc_id}`")

                    metrics = st.columns(5)
                    metrics[0].metric("Associated tasks", len(svc_tasks))
                    metrics[1].metric("Pending", pending_count)
                    metrics[2].metric("Running", running_count)
                    metrics[3].metric("Cancelling", cancelling_count)
                    metrics[4].metric("Historical retained", retained_count)

                    st.error("This action cannot be undone. Only this service will be deleted.")
                    st.caption(
                        "To confirm, type the exact service name below. "
                        "Unfinished tasks are pending, running, or cancelling tasks."
                    )
                    confirm_value = st.text_input(
                        "Confirm service name",
                        key=f"force_delete_confirm_name_{svc_id}",
                        placeholder=f"Type {service_name} here",
                    )
                    can_force_delete = confirm_value.strip() == service_name

                    c1, c2, _ = st.columns([1.3, 1, 3.7])
                    with c1:
                        if st.button(
                            "🚨 Confirm Force Delete",
                            key=f"confirm_force_delete_{svc_id}",
                            type="primary",
                            use_container_width=True,
                            disabled=not can_force_delete,
                        ):
                            from aaas_dashboard.client import DeleteServiceResult
                            success, result = client.delete_service(svc_id, force=True)
                            if success and isinstance(result, DeleteServiceResult):
                                st.success(f"Service deleted. {result.tasks_cancelled} tasks cancelled, {result.tasks_retained} tasks retained.")
                                st.rerun()
                            elif success:
                                st.success("Service deleted.")
                                st.rerun()
                            else:
                                st.error(result or "Failed to force delete service.")
                    with c2:
                        if st.button("Cancel", key=f"cancel_force_delete_{svc_id}", use_container_width=True):
                            st.session_state[f"force_delete_confirm_name_{svc_id}"] = ""
                            st.rerun()
                st.divider()

    st.subheader("Create Service")

    created_service = st.session_state.get("created_service_result")
    if created_service:
        token = created_service.get("registration_token", "")
        st.success(f"Service `{created_service.get('name', '-')}` created successfully.")
        st.caption(f"Service ID: `{created_service.get('id', '-')}`")
        if token:
            st.info("Registration token generated. Give this token to the agent-core deployer. It is used once during first registration.")
            st.markdown("**Agent registration token:**")
            st.code(token, language="text")
        else:
            st.warning("Service was created, but no registration token was returned by the server.")
        if st.button("Dismiss created service token", key="dismiss_created_service_token"):
            st.session_state.pop("created_service_result", None)
            st.rerun()

    with st.form("create_service_form"):
        svc_name = st.text_input("Name")
        svc_desc = st.text_area("Description")
        svc_usage = st.text_area("Usage")
        svc_public = st.checkbox("Public", value=False)
        submitted = st.form_submit_button("➕ Create Service")
        if submitted:
            if not svc_name:
                st.error("Name is required.")
            else:
                result = client.create_service(
                    name=svc_name,
                    description=svc_desc,
                    usage=svc_usage,
                    is_public=svc_public,
                )
                if result:
                    st.session_state["created_service_result"] = result
                    st.rerun()
                else:
                    st.error("Failed to create service.")

# ==================== Tab 2: User Management ====================
with tab_users:
    st.subheader("All Users")

    if current_user_id is None:
        st.info("Could not identify current user. Protections are enabled for all users.")

    if not users:
        st.info("No users found.")
    else:
        for user in users:
            with st.container():
                col1, col2, col3, col4, col5 = st.columns([2, 2, 1.5, 1.5, 1.5])
                with col1:
                    st.markdown(f"**{user.name}**")
                    st.caption(f"ID: `{user.id}`")
                with col2:
                    st.caption(f"API Key: `{user.api_key[:10]}...`")
                with col3:
                    st.write(f"Role: `{user.role}`")
                with col4:
                    st.write(user.created_at.strftime("%Y-%m-%d %H:%M"))
                with col5:
                    is_self = current_user_id is None or user.id == current_user_id
                    if is_self:
                        st.caption("You")
                    else:
                        if st.button("🗑️ Delete", key=f"delete_user_{user.id}"):
                            success, error_msg = client.delete_user(user.id)
                            if success:
                                st.success("Deleted!")
                                st.rerun()
                            else:
                                st.error(error_msg or "Failed to delete user.")

                # Role changer
                if not is_self:
                    new_role = st.selectbox(
                        "Change role",
                        options=["client", "admin"],
                        index=0 if user.role == "client" else 1,
                        key=f"role_{user.id}",
                        label_visibility="collapsed",
                    )
                    if new_role != user.role:
                        if st.button("💾 Update Role", key=f"update_role_{user.id}"):
                            updated = client.update_user_role(user.id, new_role)
                            if updated:
                                st.success(f"Role updated to {new_role}!")
                                st.rerun()
                            else:
                                st.error("Failed to update role.")
                st.divider()

# ==================== Tab 3: Permission Management ====================
with tab_permissions:
    st.subheader("Grant Permission")

    restricted_services = [s for s in services if not s.get("is_public", False)]

    col_u, col_s = st.columns(2)
    with col_u:
        selected_user = st.selectbox(
            "Select User",
            options=users,
            format_func=lambda u: f"{u.name} ({u.id})",
            key="perm_user",
        )
    with col_s:
        selected_service = st.selectbox(
            "Select Restricted Service",
            options=restricted_services,
            format_func=lambda s: f"{s.get('name', '-')} ({s.get('id', '-')})",
            key="perm_service",
        )

    if selected_user and selected_service:
        if st.button("🔑 Grant Permission"):
            if client.grant_service_permission(selected_service["id"], selected_user.id):
                st.success("Permission granted!")
                st.rerun()
            else:
                st.error("Failed to grant permission.")

    st.divider()
    st.subheader("User Permissions")

    if selected_user:
        perms = client.list_user_permissions(selected_user.id)
        if not perms:
            st.info(f"No service permissions for {selected_user.name}.")
        else:
            for p in perms:
                with st.container():
                    c1, c2 = st.columns([4, 1])
                    with c1:
                        svc_name = p.get("service_name", p.get("service_id", "-"))
                        st.write(f"**{svc_name}**")
                        st.caption(f"Service ID: `{p.get('service_id', '-')}`")
                    with c2:
                        if st.button("🗑️ Revoke", key=f"revoke_{selected_user.id}_{p.get('service_id')}"):
                            success, error_msg = client.revoke_service_permission(p.get("service_id"), selected_user.id)
                            if success:
                                st.success("Revoked!")
                                st.rerun()
                            else:
                                st.error(error_msg or "Failed to revoke permission.")
                    st.divider()
