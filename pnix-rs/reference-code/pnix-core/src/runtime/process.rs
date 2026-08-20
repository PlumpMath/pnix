//! Runtime process abstraction (data only).
//!
//! Pure structure definitions for processes and IPC.

use crate::effects::{Capability, EffectZone};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fmt;

/// 프로세스 ID: 프로세스 식별자
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcessId(pub u64);

/// 프로세스 상태: 프로세스 상태 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessState {
  /// 생성됨
  Created,
  /// 실행 중
  Running,
  /// 일시 중지됨
  Suspended,
  /// 종료됨
  Terminated,
}

/// 프로세스: 프로세스 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Process {
  /// 프로세스 ID
  pub id: ProcessId,
  /// Effect Zone
  pub effect_zone: EffectZone,
  /// Capability 목록
  pub capabilities: Vec<Capability>,
  /// 프로세스 상태
  pub state: ProcessState,
  /// 부모 프로세스 ID (선택적)
  pub parent: Option<ProcessId>,
}

impl Process {
  pub fn new(
    id: ProcessId,
    effect_zone: EffectZone,
    capabilities: Vec<Capability>,
    parent: Option<ProcessId>,
  ) -> Self {
    Self {
      id,
      effect_zone,
      capabilities,
      state: ProcessState::Created,
      parent,
    }
  }

  pub fn is_alive(&self) -> bool {
    matches!(
      self.state,
      ProcessState::Created | ProcessState::Running | ProcessState::Suspended
    )
  }

  pub fn start(&mut self) -> Result<(), ProcessError> {
    match self.state {
      ProcessState::Created | ProcessState::Suspended => {
        self.state = ProcessState::Running;
        Ok(())
      }
      _ => Err(ProcessError::InvalidState {
        id: self.id,
        state: self.state,
        expected: "created or suspended",
      }),
    }
  }

  pub fn suspend(&mut self) -> Result<(), ProcessError> {
    match self.state {
      ProcessState::Running => {
        self.state = ProcessState::Suspended;
        Ok(())
      }
      _ => Err(ProcessError::InvalidState {
        id: self.id,
        state: self.state,
        expected: "running",
      }),
    }
  }

  pub fn terminate(&mut self) -> Result<(), ProcessError> {
    if self.state == ProcessState::Terminated {
      return Err(ProcessError::InvalidState {
        id: self.id,
        state: self.state,
        expected: "non-terminated",
      });
    }
    self.state = ProcessState::Terminated;
    Ok(())
  }
}

/// 프로세스 메시지: 프로세스 간 통신 메시지 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMessage {
  /// 발신 프로세스 ID
  pub from: ProcessId,
  /// 수신 프로세스 ID
  pub to: ProcessId,
  /// 메시지 페이로드
  pub payload: String,
}

/// 프로세스 에러: 프로세스 작업 중 발생하는 에러 타입
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessError {
  /// 프로세스를 찾을 수 없음
  NotFound(ProcessId),
  /// 잘못된 상태
  InvalidState {
    /// 프로세스 ID
    id: ProcessId,
    /// 현재 상태
    state: ProcessState,
    /// 예상 상태
    expected: &'static str,
  },
}

impl fmt::Display for ProcessError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ProcessError::NotFound(id) => write!(f, "process {:?} not found", id),
      ProcessError::InvalidState {
        id,
        state,
        expected,
      } => write!(
        f,
        "process {:?} invalid state {:?} (expected {})",
        id, state, expected
      ),
    }
  }
}

impl std::error::Error for ProcessError {}

/// 프로세스 매니저: 프로세스 관리자 구조
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ProcessManager {
  /// 다음 프로세스 ID
  next_id: u64,
  /// 프로세스 맵 (프로세스 ID → 프로세스)
  processes: HashMap<ProcessId, Process>,
  /// 인박스 맵 (프로세스 ID → 메시지 큐)
  inbox: HashMap<ProcessId, VecDeque<ProcessMessage>>,
}

impl ProcessManager {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn spawn(&mut self, effect_zone: EffectZone, capabilities: Vec<Capability>) -> ProcessId {
    let id = ProcessId(self.next_id);
    self.next_id = self.next_id.saturating_add(1);
    let process = Process::new(id, effect_zone, capabilities, None);
    self.processes.insert(id, process);
    self.inbox.entry(id).or_default();
    id
  }

  pub fn spawn_child(
    &mut self,
    parent: ProcessId,
    effect_zone: EffectZone,
    capabilities: Vec<Capability>,
  ) -> Result<ProcessId, ProcessError> {
    if !self.processes.contains_key(&parent) {
      return Err(ProcessError::NotFound(parent));
    }
    let id = ProcessId(self.next_id);
    self.next_id = self.next_id.saturating_add(1);
    let process = Process::new(id, effect_zone, capabilities, Some(parent));
    self.processes.insert(id, process);
    self.inbox.entry(id).or_default();
    Ok(id)
  }

  pub fn get(&self, id: ProcessId) -> Option<&Process> {
    self.processes.get(&id)
  }

  pub fn get_mut(&mut self, id: ProcessId) -> Option<&mut Process> {
    self.processes.get_mut(&id)
  }

  pub fn terminate(&mut self, id: ProcessId) -> Result<(), ProcessError> {
    let process = self
      .processes
      .get_mut(&id)
      .ok_or(ProcessError::NotFound(id))?;
    process.terminate()
  }

  pub fn send(&mut self, message: ProcessMessage) -> Result<(), ProcessError> {
    if !self.processes.contains_key(&message.to) {
      return Err(ProcessError::NotFound(message.to));
    }
    self.inbox.entry(message.to).or_default().push_back(message);
    Ok(())
  }

  pub fn drain_messages(&mut self, id: ProcessId) -> Result<Vec<ProcessMessage>, ProcessError> {
    let queue = self.inbox.get_mut(&id).ok_or(ProcessError::NotFound(id))?;
    Ok(queue.drain(..).collect())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::effects::{Capability, EffectZone};

  #[test]
  fn process_lifecycle_transitions() {
    let mut process = Process::new(ProcessId(1), EffectZone::Pure, vec![Capability::Read], None);
    assert_eq!(process.state, ProcessState::Created);
    process.start().expect("start");
    assert_eq!(process.state, ProcessState::Running);
    process.suspend().expect("suspend");
    assert_eq!(process.state, ProcessState::Suspended);
    process.start().expect("resume");
    process.terminate().expect("terminate");
    assert_eq!(process.state, ProcessState::Terminated);
    assert!(process.terminate().is_err());
  }

  #[test]
  fn process_manager_spawns_and_sends_messages() {
    let mut manager = ProcessManager::new();
    let a = manager.spawn(EffectZone::Pure, vec![Capability::Read]);
    let b = manager.spawn(EffectZone::Symbolic, vec![Capability::Write]);
    let message = ProcessMessage {
      from: a,
      to: b,
      payload: "ping".to_string(),
    };
    manager.send(message).expect("send");
    let messages = manager.drain_messages(b).expect("drain");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].payload, "ping");
  }

  #[test]
  fn process_manager_spawn_child_requires_parent() {
    let mut manager = ProcessManager::new();
    let parent = manager.spawn(EffectZone::Pure, vec![]);
    let child = manager
      .spawn_child(parent, EffectZone::Pure, vec![Capability::Read])
      .expect("child");
    assert_eq!(manager.get(child).unwrap().parent, Some(parent));
    assert!(manager
      .spawn_child(ProcessId(999), EffectZone::Pure, vec![])
      .is_err());
  }
}
