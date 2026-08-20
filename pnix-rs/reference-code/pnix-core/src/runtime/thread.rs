//! Thread/Fiber abstraction (data only).
//!
//! Scheduling policies and structures are pure metadata here.

use super::process::ProcessId;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// 스레드 ID: 스레드 식별자
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ThreadId(pub u64);

/// 파이버 ID: 파이버 식별자
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FiberId(pub u64);

/// 스레드 상태: 스레드 상태 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreadState {
  /// 생성됨
  Created,
  /// 실행 중
  Running,
  /// 대기 중
  Parked,
  /// 종료됨
  Terminated,
}

/// 파이버 상태: 파이버 상태 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FiberState {
  /// 준비됨
  Ready,
  /// 실행 중
  Running,
  /// 일시 중지됨
  Suspended,
  /// 완료됨
  Completed,
}

/// 스레드: 스레드 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
  /// 스레드 ID
  pub id: ThreadId,
  /// 프로세스 ID
  pub process_id: ProcessId,
  /// 스레드 이름
  pub name: String,
  /// 스레드 상태
  pub state: ThreadState,
}

impl Thread {
  pub fn new(id: ThreadId, process_id: ProcessId, name: impl Into<String>) -> Self {
    Self {
      id,
      process_id,
      name: name.into(),
      state: ThreadState::Created,
    }
  }
}

/// 파이버: 파이버 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fiber {
  /// 파이버 ID
  pub id: FiberId,
  /// 소속 스레드 ID
  pub thread_id: ThreadId,
  /// 파이버 상태
  pub state: FiberState,
}

impl Fiber {
  pub fn new(id: FiberId, thread_id: ThreadId) -> Self {
    Self {
      id,
      thread_id,
      state: FiberState::Ready,
    }
  }
}

/// 스케줄러 정책: 스레드/파이버 스케줄링 정책 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchedulerPolicy {
  /// FIFO (First In First Out)
  Fifo,
  /// Round Robin
  RoundRobin,
}

impl Default for SchedulerPolicy {
  fn default() -> Self {
    Self::Fifo
  }
}

/// 스레드 스케줄러: 스레드/파이버 스케줄러 구조
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ThreadScheduler {
  /// 다음 스레드 ID
  next_thread: u64,
  /// 다음 파이버 ID
  next_fiber: u64,
  /// 스레드 맵 (스레드 ID → 스레드)
  threads: HashMap<ThreadId, Thread>,
  /// 파이버 맵 (파이버 ID → 파이버)
  fibers: HashMap<FiberId, Fiber>,
  /// 실행 큐 (파이버 ID 목록)
  run_queue: VecDeque<FiberId>,
  /// 스케줄링 정책
  pub policy: SchedulerPolicy,
}

impl ThreadScheduler {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn spawn_thread(&mut self, process_id: ProcessId, name: impl Into<String>) -> ThreadId {
    let id = ThreadId(self.next_thread);
    self.next_thread = self.next_thread.saturating_add(1);
    let thread = Thread::new(id, process_id, name);
    self.threads.insert(id, thread);
    id
  }

  pub fn spawn_fiber(&mut self, thread_id: ThreadId) -> FiberId {
    let id = FiberId(self.next_fiber);
    self.next_fiber = self.next_fiber.saturating_add(1);
    let fiber = Fiber::new(id, thread_id);
    self.fibers.insert(id, fiber);
    self.run_queue.push_back(id);
    id
  }

  pub fn mark_ready(&mut self, fiber_id: FiberId) {
    if let Some(fiber) = self.fibers.get_mut(&fiber_id) {
      fiber.state = FiberState::Ready;
      self.run_queue.push_back(fiber_id);
    }
  }

  pub fn next_fiber(&mut self) -> Option<FiberId> {
    self.run_queue.pop_front()
  }

  pub fn thread(&self, id: ThreadId) -> Option<&Thread> {
    self.threads.get(&id)
  }

  pub fn fiber(&self, id: FiberId) -> Option<&Fiber> {
    self.fibers.get(&id)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn scheduler_spawns_threads_and_fibers() {
    let mut scheduler = ThreadScheduler::new();
    let thread = scheduler.spawn_thread(ProcessId(7), "main");
    let fiber = scheduler.spawn_fiber(thread);

    let thread_ref = scheduler.thread(thread).expect("thread");
    assert_eq!(thread_ref.name, "main");
    assert_eq!(thread_ref.state, ThreadState::Created);

    let fiber_ref = scheduler.fiber(fiber).expect("fiber");
    assert_eq!(fiber_ref.thread_id, thread);
    assert_eq!(fiber_ref.state, FiberState::Ready);
  }

  #[test]
  fn scheduler_run_queue_order_is_fifo() {
    let mut scheduler = ThreadScheduler::new();
    let thread = scheduler.spawn_thread(ProcessId(1), "worker");
    let f1 = scheduler.spawn_fiber(thread);
    let f2 = scheduler.spawn_fiber(thread);

    assert_eq!(scheduler.next_fiber(), Some(f1));
    assert_eq!(scheduler.next_fiber(), Some(f2));
    assert_eq!(scheduler.next_fiber(), None);
  }

  #[test]
  fn scheduler_mark_ready_enqueues_again() {
    let mut scheduler = ThreadScheduler::new();
    let thread = scheduler.spawn_thread(ProcessId(1), "worker");
    let fiber = scheduler.spawn_fiber(thread);
    assert_eq!(scheduler.next_fiber(), Some(fiber));
    scheduler.mark_ready(fiber);
    assert_eq!(scheduler.next_fiber(), Some(fiber));
  }
}
