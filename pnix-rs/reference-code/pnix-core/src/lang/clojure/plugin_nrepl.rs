use serde::{Deserialize, Serialize};

pub const PLUGIN_NREPL_PROFILE_ID: &str = "plugin-nrepl-subset-v1";
pub const PLUGIN_NREPL_CLONE_SEMANTICS: &str = "fresh_session";

pub const PLUGIN_NREPL_REASON_SESSION_CLOSED: &str = "NREPL_SESSION_CLOSED";
pub const PLUGIN_NREPL_REASON_EVAL_FAILED: &str = "PLUGIN_NREPL_EVAL_FAILED";
pub const PLUGIN_NREPL_REASON_LOAD_FILE_FAILED: &str = "PLUGIN_NREPL_LOAD_FILE_FAILED";
pub const PLUGIN_NREPL_REASON_SWITCH_NS_FAILED: &str = "PLUGIN_NREPL_SWITCH_NS_FAILED";
pub const PLUGIN_NREPL_REASON_INTERRUPT_IDLE: &str = "PLUGIN_NREPL_INTERRUPT_IDLE";
pub const PLUGIN_NREPL_REASON_CLONE_BLOCKED: &str = "PLUGIN_NREPL_CLONE_BLOCKED";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PluginNreplProfile {
  SubsetV1,
}

impl PluginNreplProfile {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::SubsetV1 => PLUGIN_NREPL_PROFILE_ID,
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PluginNreplOp {
  Clone,
  Close,
  Describe,
  Eval,
  Health,
  Interrupt,
  LoadFile,
  SwitchNs,
}

impl PluginNreplOp {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Clone => "clone",
      Self::Close => "close",
      Self::Describe => "describe",
      Self::Eval => "eval",
      Self::Health => "health",
      Self::Interrupt => "interrupt",
      Self::LoadFile => "load-file",
      Self::SwitchNs => "switch-ns",
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginNreplTerminalStatus {
  Completed,
  Failed,
  Cancelled,
  TimedOut,
}

impl PluginNreplTerminalStatus {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Completed => "completed",
      Self::Failed => "failed",
      Self::Cancelled => "cancelled",
      Self::TimedOut => "timed_out",
    }
  }
}

const SUPPORTED_OPS: [PluginNreplOp; 8] = [
  PluginNreplOp::Clone,
  PluginNreplOp::Close,
  PluginNreplOp::Describe,
  PluginNreplOp::Eval,
  PluginNreplOp::Health,
  PluginNreplOp::Interrupt,
  PluginNreplOp::LoadFile,
  PluginNreplOp::SwitchNs,
];

const TERMINAL_STATUSES: [PluginNreplTerminalStatus; 4] = [
  PluginNreplTerminalStatus::Completed,
  PluginNreplTerminalStatus::Failed,
  PluginNreplTerminalStatus::Cancelled,
  PluginNreplTerminalStatus::TimedOut,
];

const UNSUPPORTED_OPS: [&str; 5] = ["classpath", "completions", "lookup", "macroexpand", "stdin"];

const REASON_CODES: [&str; 7] = [
  PLUGIN_NREPL_REASON_SESSION_CLOSED,
  PLUGIN_NREPL_REASON_EVAL_FAILED,
  PLUGIN_NREPL_REASON_LOAD_FILE_FAILED,
  PLUGIN_NREPL_REASON_SWITCH_NS_FAILED,
  PLUGIN_NREPL_REASON_INTERRUPT_IDLE,
  PLUGIN_NREPL_REASON_CLONE_BLOCKED,
  "TASK_CANCELLED",
];

pub fn plugin_nrepl_supported_ops() -> &'static [PluginNreplOp] {
  &SUPPORTED_OPS
}

pub fn plugin_nrepl_terminal_statuses() -> &'static [PluginNreplTerminalStatus] {
  &TERMINAL_STATUSES
}

pub fn plugin_nrepl_unsupported_ops() -> &'static [&'static str] {
  &UNSUPPORTED_OPS
}

pub fn plugin_nrepl_reason_codes() -> &'static [&'static str] {
  &REASON_CODES
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn plugin_nrepl_contract_lists_minimum_ops() {
    let ops: Vec<&str> = plugin_nrepl_supported_ops()
      .iter()
      .map(|op| op.as_str())
      .collect();
    assert_eq!(
      ops,
      vec![
        "clone",
        "close",
        "describe",
        "eval",
        "health",
        "interrupt",
        "load-file",
        "switch-ns",
      ]
    );
  }
}
