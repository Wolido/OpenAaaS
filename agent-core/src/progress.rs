//! 执行期实时进度采集
//!
//! 容器把进度写进 workspace 下的 `progress.jsonl`，这里按字节偏移差分读取原文，
//! 供 poll 请求顺路上报。所有读取失败一律忽略，不允许影响任务执行。

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::RwLock;

/// 进度文件所在目录（相对 workspace）
const PROGRESS_DIR: &str = ".open-aaas";
/// 追加写入的进度文本
const PROGRESS_FILE: &str = "progress.jsonl";
/// text 滚动窗口上限（字节）
const TEXT_TAIL_LIMIT: usize = 8192;

/// 单个任务的实时状态
#[derive(Debug)]
pub struct TaskLive {
    pub seq: u64,
    /// progress.jsonl 已读到的字节偏移
    pub offset: u64,
    pub text_tail: String,
    /// 上次发送出去的文本，用于判断是否需要带上 text
    sent_text: Option<String>,
}

/// 对外输出的进度快照
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub task_id: String,
    pub seq: u64,
    pub text: Option<String>,
}

/// 在跑任务的实时注册表
#[derive(Default)]
pub struct LiveRegistry {
    inner: RwLock<HashMap<String, TaskLive>>,
}

impl LiveRegistry {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    /// 任务开始执行时登记
    pub fn register(&self, task_id: &str) {
        let Ok(mut map) = self.inner.write() else {
            return;
        };
        map.insert(
            task_id.to_string(),
            TaskLive {
                seq: 0,
                offset: 0,
                text_tail: String::new(),
                sent_text: None,
            },
        );
    }

    /// 任务结束后注销
    pub fn unregister(&self, task_id: &str) {
        let Ok(mut map) = self.inner.write() else {
            return;
        };
        map.remove(task_id);
    }

    /// 是否有该任务的注册项（尚无进度文本时 snapshots 为空，无法据此判断）
    pub fn is_registered(&self, task_id: &str) -> bool {
        match self.inner.read() {
            Ok(map) => map.contains_key(task_id),
            Err(_) => false,
        }
    }

    /// 采样一次：差分读进度文件
    pub fn tick(&self, task_id: &str, workspace: &Path) {
        let Ok(mut map) = self.inner.write() else {
            return;
        };
        let Some(state) = map.get_mut(task_id) else {
            return;
        };

        let progress_path = workspace.join(PROGRESS_DIR).join(PROGRESS_FILE);

        // 差分读 progress.jsonl
        let delta = read_incremental(&progress_path, &mut state.offset);
        if !delta.is_empty() {
            state.text_tail.push_str(&delta);
            trim_text_tail(&mut state.text_tail, TEXT_TAIL_LIMIT);
        }
    }

    /// 合成单个任务的快照；只在文本相对上次发送有变化时返回，否则返回 None
    pub fn snapshot(&self, task_id: &str) -> Option<Snapshot> {
        let Ok(mut map) = self.inner.write() else {
            return None;
        };
        let state = map.get_mut(task_id)?;

        // 只在文本变化时上报：没有任何进度文本（含首次、从未写入）或文本与上次
        // 上报相同时不出快照。否则每轮 poll 都会因 seq 更大而刷新 server 的
        // received_at，`stale`（容器卡死时唯一的信号）会永远为 false。
        let has_text = !state.text_tail.is_empty();
        let text_changed = state.sent_text.as_deref() != Some(state.text_tail.as_str());
        if !has_text || !text_changed {
            return None;
        }

        state.sent_text = Some(state.text_tail.clone());
        state.seq += 1;
        tracing::trace!("进度快照 {}: seq={}", task_id, state.seq);

        Some(Snapshot {
            task_id: task_id.to_string(),
            seq: state.seq,
            text: Some(state.text_tail.clone()),
        })
    }

    /// 给所有在跑任务各出一份快照
    pub fn snapshots(&self) -> Vec<Snapshot> {
        let task_ids: Vec<String> = match self.inner.read() {
            Ok(map) => map.keys().cloned().collect(),
            Err(_) => return Vec::new(),
        };

        let mut result = Vec::with_capacity(task_ids.len());
        for task_id in task_ids {
            if let Some(snapshot) = self.snapshot(&task_id) {
                result.push(snapshot);
            }
        }
        result.sort_by(|a, b| a.task_id.cmp(&b.task_id));
        result
    }
}

/// 差分读取：记录 offset，文件变短视为被重建并从 0 开始；读失败返回空
fn read_incremental(path: &Path, offset: &mut u64) -> String {
    let Ok(mut file) = fs::File::open(path) else {
        return String::new();
    };
    let Ok(metadata) = file.metadata() else {
        return String::new();
    };
    let len = metadata.len();

    if len < *offset {
        // 文件被重建，重新从头读
        *offset = 0;
    }
    if len == *offset {
        return String::new();
    }
    if file.seek(SeekFrom::Start(*offset)).is_err() {
        return String::new();
    }

    let mut buf = Vec::new();
    if file.read_to_end(&mut buf).is_err() {
        return String::new();
    }

    // 只消费完整的 UTF-8 字符：写入把一个多字节字符劈在两次读取之间时，
    // 尾部不完整的几个字节不解码、offset 也不推进，下一轮从它们重新读入拼接。
    let mut consumed = 0usize;
    let mut text = String::new();
    while consumed < buf.len() {
        match std::str::from_utf8(&buf[consumed..]) {
            Ok(valid) => {
                text.push_str(valid);
                consumed = buf.len();
            }
            Err(e) => {
                let valid_end = consumed + e.valid_up_to();
                // valid_up_to 之前的字节必然合法，这里用 if let 保持在运行路径上不 panic
                if let Ok(valid) = std::str::from_utf8(&buf[consumed..valid_end]) {
                    text.push_str(valid);
                }
                consumed = valid_end;
                match e.error_len() {
                    // 末尾被截断的多字节字符：剩余字节留给下一轮，offset 不推进
                    None => break,
                    // 真非法字节：跳过（不输出 �），避免 offset 永久卡死
                    Some(len) => consumed += len,
                }
            }
        }
    }
    // offset 只推进到已消费的字节数；未消费的尾字节下轮重新读入
    *offset += consumed as u64;
    text
}

/// 按 UTF-8 字符边界裁剪文本尾部
fn trim_text_tail(text: &mut String, limit: usize) {
    if text.len() <= limit {
        return;
    }
    let mut start = text.len() - limit;
    while start < text.len() && !text.is_char_boundary(start) {
        start += 1;
    }
    text.drain(..start);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn write_line(path: &Path, line: &str) {
        write_bytes(path, line.as_bytes());
    }

    fn write_bytes(path: &Path, bytes: &[u8]) {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        file.write_all(bytes).unwrap();
    }

    fn setup(task_id: &str) -> (TempDir, PathBuf, LiveRegistry) {
        let temp = TempDir::new().unwrap();
        let workspace = temp.path().to_path_buf();
        fs::create_dir_all(workspace.join(PROGRESS_DIR)).unwrap();
        let registry = LiveRegistry::new();
        registry.register(task_id);
        (temp, workspace, registry)
    }

    #[test]
    fn test_tick_reads_progress_text() {
        let (temp, workspace, registry) = setup("task-1");
        let progress = workspace.join(PROGRESS_DIR).join(PROGRESS_FILE);

        write_line(
            &progress,
            "2026-11-12T10:00:01Z phase=step1 tool=bash 开始\n",
        );
        write_line(&progress, "2026-11-12T10:00:03Z note=已解析完 12 个 CSV\n");

        registry.tick("task-1", &workspace);
        let snapshot = registry.snapshot("task-1").unwrap();

        assert_eq!(snapshot.seq, 1);
        let text = snapshot.text.unwrap();
        assert!(text.contains("phase=step1"));
        assert!(text.contains("已解析完 12 个 CSV"));

        drop(temp);
    }

    #[test]
    fn test_tick_incremental_read_does_not_duplicate() {
        let (temp, workspace, registry) = setup("task-2");
        let progress = workspace.join(PROGRESS_DIR).join(PROGRESS_FILE);

        write_line(&progress, "10:00:01Z phase=step1 开始\n");
        registry.tick("task-2", &workspace);
        assert_eq!(
            registry
                .snapshot("task-2")
                .unwrap()
                .text
                .unwrap()
                .matches("phase=step1")
                .count(),
            1
        );

        write_line(&progress, "10:00:03Z note=增量\n");
        registry.tick("task-2", &workspace);
        let text = registry.snapshot("task-2").unwrap().text.unwrap();
        assert_eq!(
            text.matches("phase=step1").count(),
            1,
            "不应重复读取旧内容: {}",
            text
        );
        assert!(text.contains("note=增量"));

        drop(temp);
    }

    #[test]
    fn test_tick_resets_offset_when_file_rebuilt() {
        let (temp, workspace, registry) = setup("task-3");
        let progress = workspace.join(PROGRESS_DIR).join(PROGRESS_FILE);

        write_line(&progress, "10:00:01Z phase=step1 很长的一行内容\n");
        registry.tick("task-3", &workspace);
        assert!(registry.snapshot("task-3").unwrap().seq == 1);

        // 文件被重建为更短的内容，offset 必须归零重新读
        fs::write(&progress, "x\n").unwrap();
        registry.tick("task-3", &workspace);

        let mut map = registry.inner.write().unwrap();
        let state = map.get_mut("task-3").unwrap();
        assert_eq!(state.offset, 2, "重建后 offset 应为新文件长度");
        assert!(state.text_tail.contains("很长的一行内容"));
        assert!(state.text_tail.ends_with("x\n"));
        drop(map);
        drop(temp);
    }

    #[test]
    fn test_text_tail_truncated_on_char_boundary() {
        let (temp, workspace, registry) = setup("task-4");
        let progress = workspace.join(PROGRESS_DIR).join(PROGRESS_FILE);

        // 每个汉字 3 字节，构造远超 8KB 的内容
        let line = format!("phase=步骤 {}\n", "汉".repeat(4000));
        write_line(&progress, &line);
        registry.tick("task-4", &workspace);

        let mut map = registry.inner.write().unwrap();
        let state = map.get_mut("task-4").unwrap();
        assert!(state.text_tail.len() <= TEXT_TAIL_LIMIT);
        // 合法 UTF-8 说明没有截出半个汉字
        assert!(std::str::from_utf8(state.text_tail.as_bytes()).is_ok());
        assert!(state.text_tail.ends_with('\n'));
        drop(map);
        drop(temp);
    }

    #[test]
    fn test_offset_matches_actually_read_bytes() {
        let (temp, workspace, registry) = setup("task-8");
        let progress = workspace.join(PROGRESS_DIR).join(PROGRESS_FILE);

        write_line(&progress, "10:00:01Z phase=step1 第一段\n");
        registry.tick("task-8", &workspace);
        let len_after_first = fs::metadata(&progress).unwrap().len();
        assert_eq!(
            registry.inner.read().unwrap()["task-8"].offset,
            len_after_first
        );

        // 追加后再连续 tick 两次：旧内容只应出现一次，offset 不漂移
        write_line(&progress, "10:00:02Z phase=step2 第二段\n");
        registry.tick("task-8", &workspace);
        registry.tick("task-8", &workspace);

        let map = registry.inner.read().unwrap();
        let state = &map["task-8"];
        let actual_len = fs::metadata(&progress).unwrap().len();
        assert_eq!(state.offset, actual_len, "offset 应等于文件实际长度");
        assert!(state.offset >= len_after_first, "offset 应单调不减");
        assert_eq!(
            state.text_tail.matches("第一段").count(),
            1,
            "前一段内容不应重复追加: {}",
            state.text_tail
        );
        assert_eq!(state.text_tail.matches("第二段").count(), 1);
        drop(map);
        drop(temp);
    }

    #[test]
    fn test_snapshots_empty_when_text_unchanged() {
        let (temp, workspace, registry) = setup("task-5");
        let progress = workspace.join(PROGRESS_DIR).join(PROGRESS_FILE);

        write_line(&progress, "10:00:01Z phase=step1\n");
        registry.tick("task-5", &workspace);

        let first = registry.snapshots();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].seq, 1);
        assert!(first[0].text.is_some());

        // 文件没变：不应再出快照，否则 server 的 received_at 会被无意义刷新
        assert!(registry.snapshots().is_empty(), "文本未变化时不应上报");

        drop(temp);
    }

    #[test]
    fn test_snapshot_reappears_after_append() {
        let (temp, workspace, registry) = setup("task-9");
        let progress = workspace.join(PROGRESS_DIR).join(PROGRESS_FILE);

        write_line(&progress, "10:00:01Z phase=step1\n");
        registry.tick("task-9", &workspace);
        let first = registry.snapshots();
        assert_eq!(first.len(), 1);

        write_line(&progress, "10:00:03Z note=追加\n");
        registry.tick("task-9", &workspace);
        let second = registry.snapshots();
        assert_eq!(second.len(), 1, "文本变化后应重新出现");
        assert!(second[0].seq > first[0].seq, "seq 应递增");
        assert!(second[0].text.as_deref().unwrap().contains("note=追加"));

        drop(temp);
    }

    #[test]
    fn test_snapshots_empty_without_progress_file() {
        let (temp, workspace, registry) = setup("task-10");

        assert!(registry.snapshots().is_empty(), "从未写入不应建立条目");
        registry.tick("task-10", &workspace);
        assert!(registry.snapshots().is_empty(), "没有进度文本不应上报");

        drop(temp);
    }

    #[test]
    fn test_partial_utf8_char_not_corrupted() {
        let (temp, workspace, registry) = setup("task-11");
        let progress = workspace.join(PROGRESS_DIR).join(PROGRESS_FILE);
        let han = "汉".as_bytes();
        assert_eq!(han.len(), 3);

        // 只写前 2 个字节：多字节字符被截断
        fs::write(&progress, &han[..2]).unwrap();
        registry.tick("task-11", &workspace);
        let map = registry.inner.read().unwrap();
        let state = &map["task-11"];
        assert!(!state.text_tail.contains('\u{fffd}'), "不应产生替换字符");
        assert!(state.text_tail.is_empty());
        assert_eq!(state.offset, 0, "未消费的尾字节不应推进 offset");
        drop(map);

        // 补上第 3 个字节：拼接后应正确得到「汉」
        write_bytes(&progress, &han[2..]);
        registry.tick("task-11", &workspace);
        let snapshot = registry.snapshot("task-11").unwrap();
        let text = snapshot.text.unwrap();
        assert!(!text.contains('\u{fffd}'));
        assert_eq!(text, "汉");

        drop(temp);
    }

    #[test]
    fn test_unregister_removes_snapshots() {
        let (temp, workspace, registry) = setup("task-6");
        let progress = workspace.join(PROGRESS_DIR).join(PROGRESS_FILE);
        write_line(&progress, "10:00:01Z phase=step1\n");
        registry.tick("task-6", &workspace);
        assert_eq!(registry.snapshots().len(), 1);
        registry.unregister("task-6");
        assert!(registry.snapshots().is_empty());
        drop(temp);
    }
}
