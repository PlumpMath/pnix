//! V04: Debugger Support for Pnix Runtimes
//!
//! Provides debugging infrastructure for SSA and FRP execution.
//!
//! # Overview
//!
//! The debugger system consists of:
//! - `DebugEvent`: Events emitted during execution (step, breakpoint, etc.)
//! - `Breakpoint`: Breakpoint definition (location, condition)
//! - `DebugState`: Current debugger state (running, paused, etc.)
//! - `Debugger` trait: Interface for debuggable runtimes
//!
//! # Status
//!
//! **Experimental**: This is a POC implementation.

use serde::{Deserialize, Serialize};

/// Debugger state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DebugState {
  /// Debugger is not attached
  Detached,
  /// Execution is running normally
  Running,
  /// Execution is paused at a breakpoint or step
  Paused,
  /// Execution has completed
  Finished,
  /// Execution encountered an error
  Error,
}

impl Default for DebugState {
  fn default() -> Self {
    Self::Detached
  }
}

impl std::fmt::Display for DebugState {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Detached => write!(f, "detached"),
      Self::Running => write!(f, "running"),
      Self::Paused => write!(f, "paused"),
      Self::Finished => write!(f, "finished"),
      Self::Error => write!(f, "error"),
    }
  }
}

/// Debug event type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DebugEventKind {
  /// Execution started
  Started,
  /// Step executed
  Step {
    /// Current instruction/expression index
    index: usize,
    /// Description of the step
    description: String,
  },
  /// Breakpoint hit
  BreakpointHit {
    /// Breakpoint ID
    breakpoint_id: usize,
    /// Location description
    location: String,
  },
  /// Variable changed
  VariableChanged {
    /// Variable name
    name: String,
    /// Old value (if available)
    old_value: Option<String>,
    /// New value
    new_value: String,
  },
  /// Signal updated (FRP)
  SignalUpdated {
    /// Signal name
    name: String,
    /// New value
    value: String,
  },
  /// Execution paused
  Paused {
    /// Reason for pause
    reason: String,
  },
  /// Execution resumed
  Resumed,
  /// Execution finished
  Finished {
    /// Result value (if available)
    result: Option<String>,
  },
  /// Error occurred
  Error {
    /// Error message
    message: String,
  },
}

/// Debug event with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugEvent {
  /// Event sequence number
  pub seq: u64,
  /// Timestamp in milliseconds
  pub timestamp_ms: i64,
  /// Event kind
  pub kind: DebugEventKind,
}

impl DebugEvent {
  /// Create a new debug event
  pub fn new(seq: u64, timestamp_ms: i64, kind: DebugEventKind) -> Self {
    Self {
      seq,
      timestamp_ms,
      kind,
    }
  }
}

/// Breakpoint definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
  /// Unique breakpoint ID
  pub id: usize,
  /// Location (expression index, signal name, etc.)
  pub location: BreakpointLocation,
  /// Condition (optional)
  pub condition: Option<String>,
  /// Is this breakpoint enabled?
  pub enabled: bool,
  /// Hit count
  pub hit_count: u64,
}

impl Breakpoint {
  /// Create a new breakpoint
  pub fn new(id: usize, location: BreakpointLocation) -> Self {
    Self {
      id,
      location,
      condition: None,
      enabled: true,
      hit_count: 0,
    }
  }

  /// Set condition
  pub fn with_condition(mut self, condition: impl Into<String>) -> Self {
    self.condition = Some(condition.into());
    self
  }

  /// Enable/disable breakpoint
  pub fn set_enabled(&mut self, enabled: bool) {
    self.enabled = enabled;
  }
}

/// Breakpoint location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BreakpointLocation {
  /// SSA instruction index
  SsaIndex(usize),
  /// FRP signal name
  SignalName(String),
  /// Source line number
  SourceLine(usize),
  /// Named location (node name, function name, etc.)
  Named(String),
}

impl std::fmt::Display for BreakpointLocation {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::SsaIndex(idx) => write!(f, "ssa:{}", idx),
      Self::SignalName(name) => write!(f, "signal:{}", name),
      Self::SourceLine(line) => write!(f, "line:{}", line),
      Self::Named(name) => write!(f, "{}", name),
    }
  }
}

/// Debug command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DebugCommand {
  /// Attach debugger
  Attach,
  /// Detach debugger
  Detach,
  /// Continue execution
  Continue,
  /// Step to next instruction
  StepOver,
  /// Step into function/expression
  StepInto,
  /// Step out of current function/expression
  StepOut,
  /// Pause execution
  Pause,
  /// Add breakpoint
  AddBreakpoint(BreakpointLocation),
  /// Remove breakpoint
  RemoveBreakpoint(usize),
  /// Enable/disable breakpoint
  ToggleBreakpoint(usize, bool),
  /// Evaluate expression
  Evaluate(String),
  /// Get current stack/context
  GetContext,
  /// Get variable value
  GetVariable(String),
}

impl DebugCommand {
  fn name(&self) -> &'static str {
    match self {
      DebugCommand::Attach => "attach",
      DebugCommand::Detach => "detach",
      DebugCommand::Continue => "continue",
      DebugCommand::StepOver => "step_over",
      DebugCommand::StepInto => "step_into",
      DebugCommand::StepOut => "step_out",
      DebugCommand::Pause => "pause",
      DebugCommand::AddBreakpoint(_) => "add_breakpoint",
      DebugCommand::RemoveBreakpoint(_) => "remove_breakpoint",
      DebugCommand::ToggleBreakpoint(_, _) => "toggle_breakpoint",
      DebugCommand::Evaluate(_) => "evaluate",
      DebugCommand::GetContext => "get_context",
      DebugCommand::GetVariable(_) => "get_variable",
    }
  }
}

/// Validate whether a command is allowed for the current debugger state.
pub fn validate_debug_command(
  state: DebugState,
  command: &DebugCommand,
) -> Result<(), DebuggerError> {
  let allowed = match state {
    DebugState::Detached => matches!(command, DebugCommand::Attach),
    DebugState::Running => matches!(
      command,
      DebugCommand::Pause
        | DebugCommand::Detach
        | DebugCommand::AddBreakpoint(_)
        | DebugCommand::RemoveBreakpoint(_)
        | DebugCommand::ToggleBreakpoint(_, _)
        | DebugCommand::GetContext
        | DebugCommand::GetVariable(_)
    ),
    DebugState::Paused => matches!(
      command,
      DebugCommand::Continue
        | DebugCommand::StepOver
        | DebugCommand::StepInto
        | DebugCommand::StepOut
        | DebugCommand::Pause
        | DebugCommand::Detach
        | DebugCommand::AddBreakpoint(_)
        | DebugCommand::RemoveBreakpoint(_)
        | DebugCommand::ToggleBreakpoint(_, _)
        | DebugCommand::Evaluate(_)
        | DebugCommand::GetContext
        | DebugCommand::GetVariable(_)
    ),
    DebugState::Finished | DebugState::Error => matches!(
      command,
      DebugCommand::Detach | DebugCommand::GetContext | DebugCommand::GetVariable(_)
    ),
  };

  if allowed {
    Ok(())
  } else {
    Err(DebuggerError::InvalidCommand(format!(
      "{} is not allowed while debugger is {}",
      command.name(),
      state
    )))
  }
}

/// Debug context (current state snapshot)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DebugContext {
  /// Current state
  pub state: DebugState,
  /// Current location description
  pub location: Option<String>,
  /// Variables in scope
  pub variables: Vec<(String, String)>,
  /// Active breakpoints
  pub breakpoints: Vec<Breakpoint>,
  /// Call stack (if available)
  pub stack: Vec<String>,
}

/// Debugger command execution error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebuggerError {
  /// Debugger is not available for this runtime
  NotSupported,
  /// Command was rejected or invalid in the current state
  InvalidCommand(String),
  /// Runtime failed while executing the command
  ExecutionFailed(String),
}

impl std::fmt::Display for DebuggerError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      DebuggerError::NotSupported => write!(f, "debugger not supported"),
      DebuggerError::InvalidCommand(message) => write!(f, "invalid debug command: {}", message),
      DebuggerError::ExecutionFailed(message) => {
        write!(f, "debugger execution failed: {}", message)
      }
    }
  }
}

impl std::error::Error for DebuggerError {}

/// Debugger trait for runtimes
pub trait Debugger: Send + Sync {
  /// Get current debugger state
  fn state(&self) -> DebugState;

  /// Get current context
  fn context(&self) -> DebugContext;

  /// Execute a debug command
  ///
  /// Implementations should call `validate_debug_command` before applying stateful commands.
  fn execute(&mut self, command: DebugCommand) -> Result<DebugEvent, DebuggerError>;

  /// Check if debugger is attached
  fn is_attached(&self) -> bool {
    self.state() != DebugState::Detached
  }

  /// Add an event listener
  fn on_event(&mut self, callback: Box<dyn Fn(&DebugEvent) + Send + Sync>);
}

/// No-op debugger implementation (for runtimes without debugging)
#[derive(Default)]
pub struct NoOpDebugger;

impl Debugger for NoOpDebugger {
  fn state(&self) -> DebugState {
    DebugState::Detached
  }

  fn context(&self) -> DebugContext {
    DebugContext::default()
  }

  fn execute(&mut self, _command: DebugCommand) -> Result<DebugEvent, DebuggerError> {
    Err(DebuggerError::NotSupported)
  }

  fn on_event(&mut self, _callback: Box<dyn Fn(&DebugEvent) + Send + Sync>) {
    // No-op
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_debug_state_display() {
    assert_eq!(DebugState::Detached.to_string(), "detached");
    assert_eq!(DebugState::Running.to_string(), "running");
    assert_eq!(DebugState::Paused.to_string(), "paused");
  }

  #[test]
  fn test_breakpoint_creation() {
    let bp = Breakpoint::new(1, BreakpointLocation::SsaIndex(42)).with_condition("x > 10");

    assert_eq!(bp.id, 1);
    assert!(bp.enabled);
    assert_eq!(bp.condition, Some("x > 10".to_string()));
  }

  #[test]
  fn test_breakpoint_location_display() {
    assert_eq!(BreakpointLocation::SsaIndex(42).to_string(), "ssa:42");
    assert_eq!(
      BreakpointLocation::SignalName("time".to_string()).to_string(),
      "signal:time"
    );
    assert_eq!(BreakpointLocation::SourceLine(10).to_string(), "line:10");
  }

  #[test]
  fn test_noop_debugger() {
    let mut debugger = NoOpDebugger;

    assert_eq!(debugger.state(), DebugState::Detached);
    assert!(!debugger.is_attached());

    let result = debugger.execute(DebugCommand::Attach);
    assert!(matches!(result, Err(DebuggerError::NotSupported)));
  }

  #[test]
  fn test_debug_event_creation() {
    let event = DebugEvent::new(
      1,
      1234567890,
      DebugEventKind::Step {
        index: 0,
        description: "let x = 1".to_string(),
      },
    );

    assert_eq!(event.seq, 1);
    assert_eq!(event.timestamp_ms, 1234567890);
  }

  #[test]
  fn test_debug_context_default() {
    let ctx = DebugContext::default();
    assert_eq!(ctx.state, DebugState::Detached);
    assert!(ctx.variables.is_empty());
    assert!(ctx.breakpoints.is_empty());
  }

  #[test]
  fn test_validate_debug_command_rules() {
    assert!(validate_debug_command(DebugState::Detached, &DebugCommand::Attach).is_ok());
    assert!(validate_debug_command(DebugState::Detached, &DebugCommand::StepOver).is_err());
    assert!(validate_debug_command(DebugState::Running, &DebugCommand::Pause).is_ok());
    assert!(validate_debug_command(DebugState::Running, &DebugCommand::StepInto).is_err());
    assert!(validate_debug_command(DebugState::Paused, &DebugCommand::StepInto).is_ok());
    assert!(validate_debug_command(DebugState::Finished, &DebugCommand::Continue).is_err());
  }
}
