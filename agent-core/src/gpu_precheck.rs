//! GPU 平台启动预检
//!
//! 配置了 GPU 挂载时在启动阶段检查平台与 Docker 环境：
//! - macOS / Windows：不支持 GPU 挂载，硬错误（fail-fast）
//! - Linux / WSL2：检查 nvidia container runtime，缺失时仅警告不阻断
//! - docker info 不可用：按未知处理，仅警告不误报

use crate::config::GpuConfig;
use tracing::{info, warn};

/// GPU 预检结果
#[derive(Debug, Clone, PartialEq)]
pub enum GpuPrecheckResult {
    /// 未配置 GPU，跳过预检
    NotConfigured,
    /// 预检通过
    Ok,
    /// 存在风险，仅警告（不阻断启动）
    Warning(String),
    /// 平台不支持，硬错误（阻断启动）
    Unsupported(String),
}

/// GPU 平台预检（可注入平台标识与 docker info 输出，便于测试）
///
/// - `platform`：平台标识（取值同 `std::env::consts::OS`，WSL2 为 "linux"）
/// - `docker_info`：`docker info` 的标准输出；`None` 表示命令执行失败
pub fn check_gpu_platform(
    gpu: &Option<GpuConfig>,
    platform: &str,
    docker_info: Option<&str>,
) -> GpuPrecheckResult {
    let Some(gpu) = gpu else {
        return GpuPrecheckResult::NotConfigured;
    };

    match platform {
        "macos" => {
            return GpuPrecheckResult::Unsupported(format!(
                "macOS 不支持 Docker GPU 挂载（vendor: {}），请移除 [executor] 的 gpu 配置或在 Linux / WSL2 上运行",
                gpu.vendor.as_str()
            ));
        }
        "windows" => {
            return GpuPrecheckResult::Unsupported(
                "Windows 原生 Docker 不支持 GPU 挂载（v1），请使用 WSL2 后端运行".to_string(),
            );
        }
        _ => {}
    }

    // Linux / WSL2（WSL2 的平台标识同样为 "linux"）
    let Some(info) = docker_info else {
        return GpuPrecheckResult::Warning(
            "无法获取 docker info 输出，跳过 nvidia runtime 检查；如任务无法使用 GPU，请确认已安装 nvidia-container-toolkit"
                .to_string(),
        );
    };

    if has_nvidia_runtime(info) {
        GpuPrecheckResult::Ok
    } else {
        GpuPrecheckResult::Warning(format!(
            "docker info 中未检测到 nvidia runtime，配置了 GPU（vendor: {}）的任务可能无法启动；请安装 nvidia-container-toolkit",
            gpu.vendor.as_str()
        ))
    }
}

/// 从 docker info 输出中判断是否存在 nvidia runtime
fn has_nvidia_runtime(docker_info: &str) -> bool {
    docker_info
        .lines()
        .find(|line| line.contains("Runtimes"))
        .map(|line| line.contains("nvidia"))
        .unwrap_or_else(|| docker_info.contains("nvidia"))
}

/// 启动时执行 GPU 预检（真实环境：读取当前平台并调用 docker info）
///
/// 未配置 GPU 时直接跳过；Unsupported 返回错误阻断启动，Warning 仅记日志。
pub async fn precheck_gpu_on_startup(gpu: &Option<GpuConfig>) -> anyhow::Result<()> {
    if gpu.is_none() {
        return Ok(());
    }

    let docker_info = match tokio::process::Command::new("docker")
        .arg("info")
        .output()
        .await
    {
        Ok(output) if output.status.success() => {
            Some(String::from_utf8_lossy(&output.stdout).to_string())
        }
        Ok(output) => {
            warn!(
                "docker info 执行失败: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            None
        }
        Err(e) => {
            warn!("执行 docker info 失败: {}", e);
            None
        }
    };

    match check_gpu_platform(gpu, std::env::consts::OS, docker_info.as_deref()) {
        GpuPrecheckResult::NotConfigured => Ok(()),
        GpuPrecheckResult::Ok => {
            info!("GPU 预检通过");
            Ok(())
        }
        GpuPrecheckResult::Warning(msg) => {
            warn!("GPU 预检警告: {}", msg);
            Ok(())
        }
        GpuPrecheckResult::Unsupported(msg) => anyhow::bail!("GPU 预检失败: {}", msg),
    }
}
