//! Runtime kernel bridge helpers (non-breaking integration point).

use pnix_runtime_api::{CtConfig, EvalConfig, FrpConfig};
use pnix_runtime_kernel::KernelConfig;

pub fn kernel_config_from_eval(
  config: &EvalConfig,
) -> pnix_runtime_kernel::KernelResult<KernelConfig> {
  KernelConfig::from_eval_config(config)
}

pub fn kernel_config_from_frp(
  config: &FrpConfig,
) -> pnix_runtime_kernel::KernelResult<KernelConfig> {
  KernelConfig::from_frp_config(config)
}

pub fn kernel_config_from_ct(config: &CtConfig) -> pnix_runtime_kernel::KernelResult<KernelConfig> {
  KernelConfig::from_ct_config(config)
}
