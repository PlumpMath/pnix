//! Task Loop 구조 정의
//!
//! pnix-old의 pnix_task_loop/src/lib.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! - TaskId: 태스크 ID 타입 정의
//! - BlockReason: 블로킹 이유 enum 정의
//! - Task: 태스크 트레이트 정의 (구조만)
//! - EventLoop: 이벤트 루프 구조 정의 (실행 로직 제외)
//! - 실제 태스크 실행, 스케줄링, 타이머 처리 로직은 executor에서 구현

use serde::{Deserialize, Serialize};

/// 태스크 ID 타입
pub type TaskId = u64;

/// 태스크가 블로킹된 이유
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockReason {
  /// I/O 완료 대기
  Io,
  /// 타이머 발화 대기 (타임스탬프는 밀리초로 표현, 실제 체크는 executor에서)
  Timer { timestamp_ms: u64 },
  /// 다른 태스크 완료 대기
  Task(TaskId),
  /// 채널 작업 대기
  Channel,
}

/// 태스크 트레이트
///
/// **주의**: 메서드 구현은 executor에서 수행합니다.
/// pnix-core에는 trait 정의만 포함합니다.
pub trait Task: Send {
  /// 태스크의 한 단계 실행 (executor에서 구현)
  /// true 반환 시 완료, false 반환 시 재스케줄 필요
  fn execute(&mut self) -> Result<bool, String>;

  /// 태스크 ID 가져오기 (executor에서 구현)
  fn id(&self) -> TaskId;
}

/// 이벤트 루프 구조 (순수 데이터)
///
/// **주의**: 실제 실행 로직은 executor에서 구현합니다.
/// 이 구조는 태스크 관리 상태만 정의합니다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskLoopState {
  /// 준비된 태스크 ID 목록
  pub ready_task_ids: Vec<TaskId>,
  /// 블로킹된 태스크 ID → 블로킹 이유 매핑
  pub blocked_tasks: Vec<(TaskId, BlockReason)>,
  /// 타이머 스케줄 (타임스탬프 → 태스크 ID 목록)
  pub timer_schedule: Vec<(u64, Vec<TaskId>)>,
  /// 중지 요청 여부
  pub should_stop: bool,
}

impl TaskLoopState {
  /// 새로운 태스크 루프 상태 생성
  pub fn new() -> Self {
    Self {
      ready_task_ids: Vec::new(),
      blocked_tasks: Vec::new(),
      timer_schedule: Vec::new(),
      should_stop: false,
    }
  }

  /// 준비된 태스크 추가 (구조 변경만)
  pub fn add_ready_task(&mut self, task_id: TaskId) {
    self.ready_task_ids.push(task_id);
  }

  /// 블로킹된 태스크 추가 (구조 변경만)
  pub fn add_blocked_task(&mut self, task_id: TaskId, reason: BlockReason) {
    self.blocked_tasks.push((task_id, reason));
  }

  /// 타이머 스케줄 추가 (구조 변경만)
  pub fn schedule_timer(&mut self, timestamp_ms: u64, task_id: TaskId) {
    // 기존 스케줄에 같은 타임스탬프가 있으면 추가, 없으면 새로 생성
    if let Some((_, task_ids)) = self
      .timer_schedule
      .iter_mut()
      .find(|(ts, _)| *ts == timestamp_ms)
    {
      task_ids.push(task_id);
    } else {
      self.timer_schedule.push((timestamp_ms, vec![task_id]));
    }
  }
}

impl Default for TaskLoopState {
  fn default() -> Self {
    Self::new()
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 구조체와 메서드들은 executor/runtime 계층에서 구현하세요:
// - EventLoop 구조체 및 메서드들 (run, spawn, fire_ready_timers, execute_ready_tasks 등)
// - Task trait 구현체들
// - 실제 태스크 실행, 스케줄링, 타이머 처리 로직
//
// 이 함수들은 태스크 실행, 상태 관리, 또는 값 계산을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_block_reason() {
    let reason = BlockReason::Timer { timestamp_ms: 1000 };
    assert!(matches!(reason, BlockReason::Timer { .. }));
  }

  #[test]
  fn test_task_loop_state() {
    let mut state = TaskLoopState::new();
    state.add_ready_task(1);
    state.add_blocked_task(2, BlockReason::Io);
    assert_eq!(state.ready_task_ids.len(), 1);
    assert_eq!(state.blocked_tasks.len(), 1);
  }

  #[test]
  fn test_timer_schedule() {
    let mut state = TaskLoopState::new();
    state.schedule_timer(1000, 1);
    state.schedule_timer(1000, 2);
    assert_eq!(state.timer_schedule.len(), 1);
    assert_eq!(state.timer_schedule[0].1.len(), 2);
  }
}
