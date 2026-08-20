//! Executor Graph: FxCore 그래프 실행기

mod apply;
mod backend_catalog;
mod backend_supervisor;
mod build_cache;
mod builtins;
mod canon;
mod capability_check;
mod cli;
mod emit_fs;
mod error;
mod eval_patch;
mod frp_patch;
mod kernel;
mod model;
mod module_loader;
mod output;
mod patch;
mod plan;
pub mod project;
mod replay;
mod replay_classify;
mod rpc;
mod seto_bootstrap;
mod state_mutation_contract;
mod stdlib_sync;

pub use backend_supervisor::BackendSupervisor;
pub use cli::run_cli;
pub use error::{ExecutionError, ExecutionResult};
pub use kernel::{kernel_config_from_ct, kernel_config_from_eval, kernel_config_from_frp};
#[cfg(feature = "proptest")]
#[doc(hidden)]
pub use model::{FxCoreMeta, FxCoreModule, FxNode};
#[cfg(feature = "proptest")]
#[doc(hidden)]
pub use patch::{apply_patch, FxCorePatch, PatchOp};
pub use plan::build_plan;
pub use replay::{ReplayConfig, ReplayMode};
#[doc(hidden)]
pub use rpc::client::{run_with_retry, RpcError, RpcRetryPolicy};
pub use seto_bootstrap::bootstrap_seto;

#[cfg(test)]
#[test]
fn test_repl_state_bindings_persistence() {
  crate::cli::repl::verify_repl_state_bindings_persistence();
}

#[cfg(test)]
#[test]
fn test_repl_state_reset() {
  crate::cli::repl::verify_repl_state_reset();
}
