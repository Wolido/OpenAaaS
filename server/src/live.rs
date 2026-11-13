//! 执行器内实时进度（内存态）
//!
//! Agent 在 poll 请求中随带上报各任务的最新进度快照，Server 只保存在进程内存中，
//! Client 查询任务详情时按需附带返回，让运行中的长任务对用户可见。
//!
//! 该数据不落库：进程重启即丢失，由 Agent 后续快照重新填充。

use chrono::{DateTime, Duration, Utc};
use dashmap::{DashMap, mapref::entry::Entry};
use serde::{Deserialize, Serialize};

/// text 最大字符数
const MAX_TEXT_CHARS: usize = 8192;
/// 超过该时长未收到新快照即视为 stale
const STALE_AFTER_SECS: i64 = 30;
/// 超过该时长的条目由后台任务清理
const RETAIN_SECS: i64 = 600;
/// 单次 poll 最多接受的快照条数
pub const MAX_SNAPSHOTS_PER_POLL: usize = 32;

/// Agent 上报的单任务进度快照
#[derive(Debug, Clone, Deserialize)]
pub struct PollSnapshot {
    /// 所属服务ID，由 handler 用认证信息填充，Agent 无需上报
    #[serde(default, skip_deserializing)]
    pub service_id: String,
    /// 任务ID
    pub task_id: String,
    /// 任务内单调递增的序号
    pub seq: i64,
    /// 进度文本；省略表示沿用上一次文本，存在（含空串）表示替换
    #[serde(default)]
    pub text: Option<String>,
}

/// 内存中的进度条目
#[derive(Debug, Clone)]
pub struct LiveEntry {
    /// 所属服务ID
    pub service_id: String,
    /// 已接受的最大 seq
    pub seq: i64,
    /// 进度文本（None 表示 Agent 从未上报过文本）
    pub text: Option<String>,
    /// text 是否被截断
    pub text_truncated: bool,
    /// Server 收到该条目的时刻，仅用于 stale 判定与过期清理
    pub received_at: DateTime<Utc>,
}

impl LiveEntry {
    /// 生成对外只读视图
    fn to_view(&self, now: DateTime<Utc>) -> LiveView {
        LiveView {
            text: self.text.clone().unwrap_or_default(),
            stale: now.signed_duration_since(self.received_at)
                > Duration::seconds(STALE_AFTER_SECS),
            truncated: self.text_truncated,
        }
    }
}

/// 对外返回的只读进度视图
#[derive(Debug, Clone, Serialize)]
pub struct LiveView {
    /// 进度文本
    pub text: String,
    /// 是否已超过 30 秒未更新
    pub stale: bool,
    /// text 是否被截断
    pub truncated: bool,
}

/// 执行器内实时进度存储
#[derive(Debug, Default)]
pub struct LiveProgress {
    map: DashMap<String, LiveEntry>,
}

impl LiveProgress {
    /// 创建空的进度存储
    pub fn new() -> Self {
        Self::default()
    }

    /// 写入快照。
    ///
    /// seq 不大于已存 seq 的载荷被丢弃并返回 `false`；text 缺省时沿用旧文本。
    /// text 缺省且条目不存在时不建条目而是返回 `false`：从没写过进度的任务
    /// 不该占用内存条目，否则 stale 判定会指向一个从未有过进度的空条目。
    pub fn upsert(&self, snapshot: PollSnapshot) -> bool {
        let now = Utc::now();
        let (text, text_truncated) = match snapshot.text.as_deref() {
            Some(text) => {
                let (text, truncated) = truncate_chars(text, MAX_TEXT_CHARS);
                (Some(text), truncated)
            }
            None => (None, false),
        };
        match self.map.entry(snapshot.task_id) {
            Entry::Occupied(mut occupied) => {
                let entry = occupied.get_mut();
                if snapshot.seq <= entry.seq {
                    return false;
                }

                entry.seq = snapshot.seq;
                // 回写 service_id，避免条目归属漂移后原服务清不掉
                entry.service_id = snapshot.service_id;
                // text 省略 = 沿用上一次文本
                if let Some(text) = text {
                    entry.text = Some(text);
                    entry.text_truncated = text_truncated;
                }
                entry.received_at = now;
                true
            }
            Entry::Vacant(vacant) => {
                // 条目不存在且没有文本：没有可展示的进度，不建空条目
                let Some(text) = text else {
                    return false;
                };
                vacant.insert(LiveEntry {
                    service_id: snapshot.service_id,
                    seq: snapshot.seq,
                    text: Some(text),
                    text_truncated,
                    received_at: now,
                });
                true
            }
        }
    }

    /// 查询任务的进度视图
    pub fn get(&self, task_id: &str) -> Option<LiveView> {
        self.map.get(task_id).map(|entry| entry.to_view(Utc::now()))
    }

    /// 删除任务的进度条目
    pub fn remove(&self, task_id: &str) -> bool {
        self.map.remove(task_id).is_some()
    }

    /// 清除某个服务的全部进度条目，返回清理数量
    pub fn clear_service(&self, service_id: &str) -> usize {
        let removed = std::cell::Cell::new(0usize);
        self.map.retain(|_, entry| {
            if entry.service_id == service_id {
                removed.set(removed.get() + 1);
                false
            } else {
                true
            }
        });
        removed.get()
    }

    /// 清理 received_at 超过 10 分钟的条目，返回清理数量
    pub fn prune(&self) -> usize {
        let now = Utc::now();
        let removed = std::cell::Cell::new(0usize);
        self.map.retain(|_, entry| {
            if now.signed_duration_since(entry.received_at) > Duration::seconds(RETAIN_SECS) {
                removed.set(removed.get() + 1);
                false
            } else {
                true
            }
        });
        removed.get()
    }

    /// 当前条目数量（测试用）
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// 是否为空（测试用）
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// 按字符截断，返回截断后的字符串与是否发生截断
fn truncate_chars(value: &str, max_chars: usize) -> (String, bool) {
    match value.char_indices().nth(max_chars) {
        Some((idx, _)) => (value[..idx].to_string(), true),
        None => (value.to_string(), false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(task_id: &str, seq: i64, text: Option<&str>) -> PollSnapshot {
        PollSnapshot {
            service_id: "svc-1".to_string(),
            task_id: task_id.to_string(),
            seq,
            text: text.map(str::to_string),
        }
    }

    #[test]
    fn test_upsert_then_get() {
        let live = LiveProgress::new();
        assert!(live.upsert(snapshot("task-1", 12, Some("step1"))));

        let view = live.get("task-1").expect("view should exist");
        assert_eq!(view.text, "step1");
        assert!(!view.truncated);
        assert!(!view.stale);
        let json = serde_json::to_string(&view).unwrap();
        assert_eq!(json, r#"{"text":"step1","stale":false,"truncated":false}"#);
        assert!(
            !json.contains("source"),
            "live 视图不应含 source 键：{json}"
        );
        assert_eq!(live.len(), 1);
    }

    #[test]
    fn test_upsert_drops_smaller_or_equal_seq() {
        let live = LiveProgress::new();
        assert!(live.upsert(snapshot("task-1", 12, Some("v12"))));
        assert!(!live.upsert(snapshot("task-1", 12, Some("v12 dup"))));
        assert!(!live.upsert(snapshot("task-1", 11, Some("old"))));

        let view = live.get("task-1").expect("view should exist");
        assert_eq!(view.text, "v12");
    }

    #[test]
    fn test_upsert_text_omitted_keeps_previous() {
        let live = LiveProgress::new();
        live.upsert(snapshot("task-1", 1, Some("line1\nline2")));
        live.upsert(snapshot("task-1", 2, None));

        let view = live.get("task-1").expect("view should exist");
        assert_eq!(view.text, "line1\nline2");
    }

    #[test]
    fn test_upsert_without_text_on_missing_entry_creates_nothing() {
        let live = LiveProgress::new();

        assert!(!live.upsert(snapshot("task-1", 1, None)));
        assert!(live.get("task-1").is_none());
        assert_eq!(live.len(), 0);
    }

    #[test]
    fn test_upsert_without_text_refreshes_existing_entry() {
        let live = LiveProgress::new();
        live.upsert(snapshot("task-1", 1, Some("line1")));

        // 手动把 received_at 拨回 40 秒前
        if let Some(mut entry) = live.map.get_mut("task-1") {
            entry.received_at = Utc::now() - Duration::seconds(40);
        }
        assert!(live.get("task-1").expect("view should exist").stale);

        assert!(live.upsert(snapshot("task-1", 2, None)));

        let view = live.get("task-1").expect("view should exist");
        assert_eq!(view.text, "line1");
        assert!(!view.stale);
    }

    #[test]
    fn test_upsert_text_present_replaces() {
        let live = LiveProgress::new();
        live.upsert(snapshot("task-1", 1, Some("old")));
        live.upsert(snapshot("task-1", 2, Some("")));

        let view = live.get("task-1").expect("view should exist");
        assert_eq!(view.text, "");

        live.upsert(snapshot("task-1", 3, Some("new")));
        let view = live.get("task-1").expect("view should exist");
        assert_eq!(view.text, "new");
    }

    #[test]
    fn test_upsert_truncates_overlong_text() {
        let live = LiveProgress::new();
        let text = "行".repeat(MAX_TEXT_CHARS + 100);
        live.upsert(snapshot("task-1", 1, Some(&text)));

        let view = live.get("task-1").expect("view should exist");
        assert_eq!(view.text.chars().count(), MAX_TEXT_CHARS);
        assert!(view.truncated);
    }

    #[test]
    fn test_view_stale_after_30_seconds() {
        let live = LiveProgress::new();
        live.upsert(snapshot("task-1", 1, Some("text")));

        // 手动把 received_at 拨回 40 秒前
        if let Some(mut entry) = live.map.get_mut("task-1") {
            entry.received_at = Utc::now() - Duration::seconds(40);
        }

        let view = live.get("task-1").expect("view should exist");
        assert!(view.stale);
    }

    #[test]
    fn test_view_not_stale_when_just_received() {
        let live = LiveProgress::new();
        live.upsert(snapshot("task-1", 1, Some("text")));

        if let Some(mut entry) = live.map.get_mut("task-1") {
            entry.received_at = Utc::now();
        }

        let view = live.get("task-1").expect("view should exist");
        assert!(!view.stale);
    }

    #[test]
    fn test_prune_removes_expired_entries() {
        let live = LiveProgress::new();
        live.upsert(snapshot("task-old", 1, Some("old")));
        live.upsert(snapshot("task-new", 1, Some("new")));

        if let Some(mut entry) = live.map.get_mut("task-old") {
            entry.received_at = Utc::now() - Duration::seconds(RETAIN_SECS + 1);
        }

        assert_eq!(live.prune(), 1);
        assert!(live.get("task-old").is_none());
        assert!(live.get("task-new").is_some());
    }

    #[test]
    fn test_upsert_updates_service_id_on_existing_entry() {
        let live = LiveProgress::new();
        let mut first = snapshot("task-1", 1, Some("a"));
        first.service_id = "svc-old".to_string();
        live.upsert(first);

        let mut second = snapshot("task-1", 2, Some("b"));
        second.service_id = "svc-new".to_string();
        live.upsert(second);

        // 归属已回写，旧服务无法再清掉该条目
        assert_eq!(live.clear_service("svc-old"), 0);
        assert!(live.get("task-1").is_some());
        assert_eq!(live.clear_service("svc-new"), 1);
        assert!(live.get("task-1").is_none());
    }

    #[test]
    fn test_clear_service_only_clears_target() {
        let live = LiveProgress::new();
        let mut svc1 = snapshot("task-1", 1, Some("a"));
        svc1.service_id = "svc-1".to_string();
        let mut svc2 = snapshot("task-2", 1, Some("b"));
        svc2.service_id = "svc-2".to_string();
        live.upsert(svc1);
        live.upsert(svc2);

        assert_eq!(live.clear_service("svc-1"), 1);
        assert_eq!(live.len(), 1);
        assert!(live.get("task-1").is_none());
        assert!(live.get("task-2").is_some());
    }
}
