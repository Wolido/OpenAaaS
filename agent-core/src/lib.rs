//! OpenAaaS Agent Core
//!
//! 调度层与执行层的核心框架

pub mod client;
pub mod config;
pub mod executor;
pub mod gpu_precheck;
pub mod main_support;
pub mod scheduler;
pub mod state;
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

pub use client::ApiClient;
pub use config::Config;
pub use executor::{Executor, ExecutorError, Task, TaskResult};
pub use scheduler::Scheduler;
pub use state::StateManager;
