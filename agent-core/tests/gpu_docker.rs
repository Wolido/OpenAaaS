//! GPU Docker 参数翻译测试（TDD 红阶段）
//!
//! 这些测试验证 GPU 配置到 Docker 运行参数的翻译逻辑。
//! 当前会编译失败，因为以下尚未实现：
//! - `GpuVendor` 枚举
//! - `GpuConfig` 结构体
//! - `ExecutorConfig.gpu` 字段
//! - `agent_core::executor::docker::gpu_run_args()` 函数
//!
//! 这是预期的红阶段行为，待 coder 实现后应变绿。

use agent_core::config::{GpuConfig, GpuVendor};
use agent_core::executor::docker::gpu_run_args;

/// gpu 为 None 时，不应生成任何 GPU 参数
#[test]
fn test_gpu_run_args_none_returns_empty() {
    // Arrange
    let gpu = None;

    // Act
    let args = gpu_run_args(&gpu);

    // Assert
    assert!(
        args.is_empty(),
        "No GPU args should be generated when gpu is None, got: {:?}",
        args
    );
}

/// nvidia + devices=all → ["--gpus", "all"]
#[test]
fn test_gpu_run_args_nvidia_all() {
    // Arrange
    let gpu = Some(GpuConfig {
        vendor: GpuVendor::Nvidia,
        devices: Some("all".to_string()),
    });

    // Act
    let args = gpu_run_args(&gpu);

    // Assert
    assert_eq!(
        args,
        vec!["--gpus", "all"],
        "Nvidia + all should produce ['--gpus', 'all']"
    );
}

/// nvidia + devices="0,1" → ["--gpus", "device=0,1"]
#[test]
fn test_gpu_run_args_nvidia_specific_devices() {
    // Arrange
    let gpu = Some(GpuConfig {
        vendor: GpuVendor::Nvidia,
        devices: Some("0,1".to_string()),
    });

    // Act
    let args = gpu_run_args(&gpu);

    // Assert
    assert_eq!(
        args,
        vec!["--gpus", "device=0,1"],
        "Nvidia + '0,1' should produce ['--gpus', 'device=0,1']"
    );
}

/// GPU 参数值中不应包含 shell 引号（防止注入）
#[test]
fn test_gpu_run_args_no_shell_quotes() {
    // Arrange
    let gpu = Some(GpuConfig {
        vendor: GpuVendor::Nvidia,
        devices: Some("0,1".to_string()),
    });

    // Act
    let args = gpu_run_args(&gpu);

    // Assert
    for arg in &args {
        assert!(
            !arg.contains('\'') && !arg.contains('"'),
            "GPU args must not contain shell quotes, got: {:?}",
            args
        );
    }
}

/// AMD 和 Intel 在 v1 中未实现，应返回空
#[test]
fn test_gpu_run_args_amd_intel_returns_empty_v1() {
    // Arrange
    let amd_gpu = Some(GpuConfig {
        vendor: GpuVendor::Amd,
        devices: Some("all".to_string()),
    });
    let intel_gpu = Some(GpuConfig {
        vendor: GpuVendor::Intel,
        devices: Some("all".to_string()),
    });

    // Act
    let amd_args = gpu_run_args(&amd_gpu);
    let intel_args = gpu_run_args(&intel_gpu);

    // Assert: v1 仅支持 nvidia
    assert!(
        amd_args.is_empty(),
        "AMD GPU args should be empty in v1, got: {:?}",
        amd_args
    );
    assert!(
        intel_args.is_empty(),
        "Intel GPU args should be empty in v1, got: {:?}",
        intel_args
    );
}

/// devices 为 None 时应默认为 "all"
#[test]
fn test_gpu_run_args_devices_none_defaults_to_all() {
    // Arrange
    let gpu = Some(GpuConfig {
        vendor: GpuVendor::Nvidia,
        devices: None, // None 应默认为 "all"
    });

    // Act
    let args = gpu_run_args(&gpu);

    // Assert
    assert_eq!(
        args,
        vec!["--gpus", "all"],
        "Nvidia + None devices should default to 'all'"
    );
}

/// 多设备索引格式正确
#[test]
fn test_gpu_run_args_multiple_device_indices() {
    // Arrange
    let gpu = Some(GpuConfig {
        vendor: GpuVendor::Nvidia,
        devices: Some("0,2,3".to_string()),
    });

    // Act
    let args = gpu_run_args(&gpu);

    // Assert
    assert_eq!(
        args,
        vec!["--gpus", "device=0,2,3"],
        "Multiple device indices should be formatted correctly"
    );
}
