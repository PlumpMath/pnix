//! Task 구조 정의
//!
//! pnix-old의 pnix_io_runtime/src/async_utils.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, async 실행 로직 제외
//! - TaskCancelHandle: 태스크 취소 핸들 구조 정의
//! - TaskGroup: 태스크 그룹 구조 정의
//! - 실제 async 태스크 실행 및 취소는 executor에서 구현

use serde::{Deserialize, Serialize};

/// 태스크 취소 핸들 구조
///
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, async 실행 로직 제외
/// - task_id: 태스크 식별자 (구조 정의만)
/// - 실제 async 태스크 실행 및 취소는 executor에서 구현
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TaskCancelHandle {
  /// 태스크 식별자 (구조 정의만, 실제 태스크는 executor에서 관리)
  pub task_id: String,
}

impl TaskCancelHandle {
  /// 새로운 태스크 취소 핸들 생성
  pub fn new(task_id: String) -> Self {
    Self { task_id }
  }

  /// 태스크 식별자 조회
  pub fn task_id(&self) -> &str {
    &self.task_id
  }
}

/// 태스크 그룹 구조
///
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, async 실행 로직 제외
/// - task_ids: 태스크 식별자 목록 (구조 정의만)
/// - 실제 태스크 그룹 관리 및 취소는 executor에서 구현
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskGroup {
  /// 태스크 식별자 목록 (구조 정의만)
  pub task_ids: Vec<String>,
}

impl TaskGroup {
  /// 새로운 태스크 그룹 생성
  pub fn new() -> Self {
    Self {
      task_ids: Vec::new(),
    }
  }

  /// 태스크 추가 (구조 변경만)
  pub fn add_task(&mut self, task_id: String) {
    self.task_ids.push(task_id);
  }

  /// 태스크 식별자 목록 조회
  pub fn task_ids(&self) -> &[String] {
    &self.task_ids
  }
}

impl Default for TaskGroup {
  fn default() -> Self {
    Self::new()
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - cancel() (실제 태스크 취소 실행)
// - await_task() -> Result<T, OsError> (태스크 완료 대기)
// - is_finished() -> bool (태스크 완료 여부 확인)
// - spawn_cancellable_task(task) -> TaskCancelHandle (태스크 실행)
// - cancel_all() (모든 태스크 취소)
// - await_all() -> Vec<Result<T, OsError>> (모든 태스크 완료 대기)
//
// 이 함수들은 async 실행 및 I/O를 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_task_cancel_handle_creation() {
    let handle = TaskCancelHandle::new("task_1".to_string());
    assert_eq!(handle.task_id(), "task_1");
  }

  #[test]
  fn test_task_group_creation() {
    let mut group = TaskGroup::new();
    group.add_task("task_1".to_string());
    group.add_task("task_2".to_string());
    assert_eq!(group.task_ids().len(), 2);
  }
}
