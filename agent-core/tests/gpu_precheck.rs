//! GPU 平台预检测试（TDD 红阶段）
//!
//! 这些测试验证 GPU 平台预检逻辑。
//! 当前会编译失败，因为以下尚未实现：
//! - `agent_core::gpu_precheck` 模块
//! - `check_gpu_platform()` 函数
//! - `GpuPrecheckResult` 枚举
//! - `GpuVendor` 枚举
//! - `GpuConfig` 结构体
//!
//! 这是预期的红阶段行为，待 coder 实现后应变绿。

use agent_core::config::{GpuConfig, GpuVendor};
use agent_core::gpu_precheck::{GpuPrecheckResult, check_gpu_platform};

/// GPU 未配置时，预检应跳过（NotConfigured）
#[test]
fn test_gpu_not_configured_returns_not_configured() {
    // Arrange
    let gpu = None;
    let platform = "linux";
    let docker_info = Some("Runtimes: nvidia runc");

    // Act
    let result = check_gpu_platform(&gpu, platform, docker_info);

    // Assert
    assert_eq!(
        result,
        GpuPrecheckResult::NotConfigured,
        "When GPU is not configured, precheck should return NotConfigured"
    );
}

/// macOS 上配置了 GPU 应返回 Unsupported（fail-fast）
#[test]
fn test_macos_with_gpu_returns_unsupported() {
    // Arrange
    let gpu = Some(GpuConfig {
        vendor: GpuVendor::Nvidia,
        devices: Some("all".to_string()),
    });
    let platform = "macos";
    let docker_info = None; // macOS 上不需要 docker info

    // Act
    let result = check_gpu_platform(&gpu, platform, docker_info);

    // Assert
    match result {
        GpuPrecheckResult::Unsupported(msg) => {
            assert!(
                msg.to_lowercase().contains("macos") || msg.to_lowercase().contains("mac"),
                "Error message should mention macOS, got: {}",
                msg
            );
        }
        _ => panic!(
            "macOS with GPU configured should return Unsupported, got: {:?}",
            result
        ),
    }
}

/// Linux + nvidia runtime 存在 → Ok
#[test]
fn test_linux_with_nvidia_runtime_returns_ok() {
    // Arrange
    let gpu = Some(GpuConfig {
        vendor: GpuVendor::Nvidia,
        devices: Some("all".to_string()),
    });
    let platform = "linux";
    let docker_info = Some(
        r#"
Client: Docker Engine - Community
 Version:           20.10.7
 Runtimes:          nvidia runc
 Default Runtime:   runc
"#,
    );

    // Act
    let result = check_gpu_platform(&gpu, platform, docker_info);

    // Assert
    assert_eq!(
        result,
        GpuPrecheckResult::Ok,
        "Linux with nvidia runtime should return Ok"
    );
}

/// Linux + nvidia runtime 缺失 → Warning（不失败）
#[test]
fn test_linux_without_nvidia_runtime_returns_warning() {
    // Arrange
    let gpu = Some(GpuConfig {
        vendor: GpuVendor::Nvidia,
        devices: Some("all".to_string()),
    });
    let platform = "linux";
    let docker_info = Some(
        r#"
Client: Docker Engine - Community
 Version:           20.10.7
 Runtimes:          runc
 Default Runtime:   runc
"#,
    );

    // Act
    let result = check_gpu_platform(&gpu, platform, docker_info);

    // Assert
    match result {
        GpuPrecheckResult::Warning(msg) => {
            assert!(
                msg.to_lowercase().contains("nvidia") || msg.to_lowercase().contains("runtime"),
                "Warning message should mention nvidia or runtime, got: {}",
                msg
            );
        }
        _ => panic!(
            "Linux without nvidia runtime should return Warning, got: {:?}",
            result
        ),
    }
}

/// Linux + docker info 命令失败 → Warning（按未知处理）
#[test]
fn test_linux_docker_info_fails_returns_warning() {
    // Arrange
    let gpu = Some(GpuConfig {
        vendor: GpuVendor::Nvidia,
        devices: Some("all".to_string()),
    });
    let platform = "linux";
    let docker_info = None; // docker info 命令失败

    // Act
    let result = check_gpu_platform(&gpu, platform, docker_info);

    // Assert
    match result {
        GpuPrecheckResult::Warning(msg) => {
            assert!(!msg.is_empty(), "Warning message should not be empty");
        }
        _ => panic!(
            "Linux with docker info failure should return Warning, got: {:?}",
            result
        ),
    }
}

/// WSL2 平台应视为 Linux
#[test]
fn test_wsl2_platform_treated_as_linux() {
    // Arrange
    let gpu = Some(GpuConfig {
        vendor: GpuVendor::Nvidia,
        devices: Some("all".to_string()),
    });
    let platform = "linux"; // WSL2 报告为 "linux"
    let docker_info = Some("Runtimes: nvidia runc");

    // Act
    let result = check_gpu_platform(&gpu, platform, docker_info);

    // Assert
    assert_eq!(
        result,
        GpuPrecheckResult::Ok,
        "WSL2 (reported as linux) with nvidia runtime should return Ok"
    );
}

/// Windows 原生 Docker 应返回 Unsupported（v1 不支持）
#[test]
fn test_windows_with_gpu_returns_unsupported() {
    // Arrange
    let gpu = Some(GpuConfig {
        vendor: GpuVendor::Nvidia,
        devices: Some("all".to_string()),
    });
    let platform = "windows";
    let docker_info = None;

    // Act
    let result = check_gpu_platform(&gpu, platform, docker_info);

    // Assert
    match result {
        GpuPrecheckResult::Unsupported(msg) => {
            assert!(
                msg.to_lowercase().contains("windows"),
                "Error message should mention Windows, got: {}",
                msg
            );
        }
        _ => panic!(
            "Windows with GPU configured should return Unsupported in v1, got: {:?}",
            result
        ),
    }
}
