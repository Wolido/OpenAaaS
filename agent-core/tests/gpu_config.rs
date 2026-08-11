//! GPU 配置测试（TDD 红阶段）
//!
//! 这些测试验证 GPU 配置字段的预期行为。
//! 当前会编译失败，因为以下类型/字段尚未实现：
//! - `GpuVendor` 枚举
//! - `GpuConfig` 结构体
//! - `ExecutorConfig.gpu` 字段
//! - `ExecutorConfig.gpu_devices` 字段
//!
//! 这是预期的红阶段行为，待 coder 实现后应变绿。

use agent_core::config::{Config, ExecutorConfig, GpuConfig, GpuVendor};

/// GPU 配置 TOML roundtrip：nvidia + devices=all
#[test]
fn test_gpu_config_toml_roundtrip_nvidia_all() {
    // Arrange
    let toml_str = r#"
[executor]
gpu.vendor = "nvidia"
gpu.devices = "all"
"#;

    // Act
    let config: Config = toml::from_str(toml_str).unwrap();

    // Assert
    assert!(
        config.executor.gpu.is_some(),
        "GPU config should be parsed from TOML"
    );
    let gpu = config.executor.gpu.as_ref().unwrap();
    assert_eq!(gpu.vendor, GpuVendor::Nvidia, "Vendor should be Nvidia");
    assert_eq!(
        gpu.devices.as_deref(),
        Some("all"),
        "Devices should be 'all'"
    );
}

/// GPU 配置 TOML roundtrip：nvidia + 特定设备索引
#[test]
fn test_gpu_config_toml_roundtrip_nvidia_specific_devices() {
    // Arrange
    let toml_str = r#"
[executor]
gpu.vendor = "nvidia"
gpu.devices = "0,1"
"#;

    // Act
    let config: Config = toml::from_str(toml_str).unwrap();

    // Assert
    let gpu = config
        .executor
        .gpu
        .as_ref()
        .expect("GPU config should be present");
    assert_eq!(gpu.vendor, GpuVendor::Nvidia);
    assert_eq!(gpu.devices.as_deref(), Some("0,1"));
}

/// 旧配置（无 gpu 字段）应正常加载，gpu 默认为 None
#[test]
fn test_old_config_without_gpu_field_loads_successfully() {
    // Arrange: 模拟旧配置文件，不含 gpu 字段
    let toml_str = r#"
[executor]
image = "custom-executor:v1"
capacity = 4
"#;

    // Act
    let config: Config = toml::from_str(toml_str).unwrap();

    // Assert
    assert!(
        config.executor.gpu.is_none(),
        "Old config without gpu field should load with gpu = None"
    );
}

/// ExecutorConfig 默认值：gpu 应为 None
#[test]
fn test_executor_config_default_gpu_is_none() {
    // Arrange & Act
    let executor = ExecutorConfig::default();

    // Assert
    assert!(
        executor.gpu.is_none(),
        "Default ExecutorConfig should have gpu = None"
    );
}

/// to_runtime_toml：gpu 启用时应输出实际值
#[test]
fn test_runtime_toml_gpu_enabled_outputs_value() {
    // Arrange
    let config = Config {
        executor: ExecutorConfig {
            gpu: Some(GpuConfig {
                vendor: GpuVendor::Nvidia,
                devices: Some("all".to_string()),
            }),
            ..Default::default()
        },
        ..Default::default()
    };

    // Act
    let toml_str = config.to_runtime_toml();

    // Assert: 应输出非注释的 gpu.vendor 行
    assert!(
        toml_str.contains(r#"gpu.vendor = "nvidia""#),
        "Enabled GPU config should output non-commented vendor line, actual:\n{}",
        toml_str
    );
    assert!(
        toml_str.contains(r#"gpu.devices = "all""#),
        "Enabled GPU config should output devices line, actual:\n{}",
        toml_str
    );
}

/// 非法 vendor 值应导致反序列化失败
#[test]
fn test_gpu_invalid_vendor_fails_deserialization() {
    // Arrange: "apple" 不是合法的 GpuVendor
    let toml_str = r#"
[executor]
gpu.vendor = "apple"
"#;

    // Act & Assert: 应反序列化失败
    let result: Result<Config, _> = toml::from_str(toml_str);
    assert!(
        result.is_err(),
        "Invalid vendor 'apple' should fail deserialization"
    );
}

/// GpuVendor 枚举应支持 nvidia/amd/intel 三个变体
#[test]
fn test_gpu_vendor_enum_variants() {
    // Arrange & Act
    let nvidia = GpuVendor::Nvidia;
    let amd = GpuVendor::Amd;
    let intel = GpuVendor::Intel;

    // Assert: 三个变体应存在且可比较
    assert_ne!(nvidia, amd);
    assert_ne!(nvidia, intel);
    assert_ne!(amd, intel);
}
