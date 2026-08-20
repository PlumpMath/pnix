//! 태스크 스케줄러: 태스크 큐 관리 및 실행

use std::collections::VecDeque;

use crate::{Kernel, KernelResult};

/// 태스크 ID: 태스크 고유 식별자
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TaskId(u64);

impl TaskId {
  /// u64 값으로 변환
  pub fn as_u64(self) -> u64 {
    self.0
  }
}

/// 커널 태스크 함수 트레잇: 태스크 실행 로직
pub(crate) trait KernelTaskFn {
  /// 태스크 실행
  fn run(self: Box<Self>, kernel: &mut Kernel) -> KernelResult<()>;
}

impl<F> KernelTaskFn for F
where
  F: FnOnce(&mut Kernel) -> KernelResult<()> + 'static,
{
  fn run(self: Box<Self>, kernel: &mut Kernel) -> KernelResult<()> {
    (self)(kernel)
  }
}

/// 커널 태스크: 실행할 작업
pub struct KernelTask {
  /// 태스크 ID
  pub id: TaskId,
  /// 태스크 레이블 (디버깅용)
  pub label: String,
  /// 실행할 액션
  action: Box<dyn KernelTaskFn>,
}

impl std::fmt::Debug for KernelTask {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("KernelTask")
      .field("id", &self.id)
      .field("label", &self.label)
      .finish()
  }
}

impl KernelTask {
  pub(crate) fn new(id: TaskId, label: impl Into<String>, action: Box<dyn KernelTaskFn>) -> Self {
    Self {
      id,
      label: label.into(),
      action,
    }
  }

  pub fn run(self, kernel: &mut Kernel) -> KernelResult<()> {
    self.action.run(kernel)
  }
}

/// 태스크 스케줄러: 태스크 큐 및 지연 태스크 관리
#[derive(Debug, Default)]
pub struct TaskScheduler {
  /// 즉시 실행할 태스크 큐
  queue: VecDeque<KernelTask>,
  /// 지연 실행 태스크 목록
  delayed: Vec<DelayedTask>,
  /// 다음 태스크 ID
  next_id: u64,
}

impl TaskScheduler {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn schedule(
    &mut self,
    label: impl Into<String>,
    action: impl FnOnce(&mut Kernel) -> KernelResult<()> + 'static,
  ) -> TaskId {
    let id = TaskId(self.next_id);
    self.next_id = self.next_id.saturating_add(1);
    let task = KernelTask::new(id, label, Box::new(action));
    self.queue.push_back(task);
    id
  }

  pub fn schedule_at(
    &mut self,
    label: impl Into<String>,
    due_ms: i64,
    action: impl FnOnce(&mut Kernel) -> KernelResult<()> + 'static,
  ) -> TaskId {
    let id = TaskId(self.next_id);
    self.next_id = self.next_id.saturating_add(1);
    let task = KernelTask::new(id, label, Box::new(action));
    self.delayed.push(DelayedTask { due_ms, task });
    id
  }

  pub fn promote_due(&mut self, now_ms: i64) -> usize {
    if self.delayed.is_empty() {
      return 0;
    }
    let mut remaining = Vec::with_capacity(self.delayed.len());
    let mut count = 0;
    for delayed in self.delayed.drain(..) {
      if delayed.due_ms <= now_ms {
        self.queue.push_back(delayed.task);
        count += 1;
      } else {
        remaining.push(delayed);
      }
    }
    self.delayed = remaining;
    count
  }

  pub fn pop_next(&mut self) -> Option<KernelTask> {
    self.queue.pop_front()
  }

  pub fn len(&self) -> usize {
    self.queue.len()
  }

  pub fn is_empty(&self) -> bool {
    self.queue.is_empty()
  }

  pub fn has_delayed(&self) -> bool {
    !self.delayed.is_empty()
  }

  pub fn next_due_ms(&self) -> Option<i64> {
    self.delayed.iter().map(|d| d.due_ms).min()
  }

  pub fn next_id(&self) -> TaskId {
    TaskId(self.next_id)
  }
}

#[cfg(test)]
mod delayed_tests {
  use crate::KernelConfig;

  #[test]
  fn scheduler_promotes_delayed_tasks() {
    let mut kernel = crate::Kernel::new(KernelConfig::deterministic_defaults()).unwrap();
    let task_id = kernel.schedule_at("later", 10, |_| Ok(()));
    assert!(kernel.scheduler().has_delayed());
    assert!(kernel.scheduler().is_empty());
    kernel.tick();
    kernel.run_next().unwrap();
    assert_eq!(task_id.as_u64(), 0);
  }
}

#[derive(Debug)]
struct DelayedTask {
  due_ms: i64,
  task: KernelTask,
}

#[cfg(test)]
mod tests {
  use std::sync::{Arc, Mutex};

  use crate::kernel::Kernel;
  use crate::KernelConfig;

  #[test]
  fn scheduler_fifo_order() {
    let mut kernel = Kernel::new(KernelConfig::deterministic_defaults()).unwrap();
    let order: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    let order_a = Arc::clone(&order);
    kernel.schedule("first", move |kernel| {
      kernel.emit_effect(crate::EffectEvent::new(
        crate::EffectZone::Pure,
        "first",
        "ok",
      ));
      order_a.lock().unwrap().push("first".to_string());
      Ok(())
    });

    let order_b = Arc::clone(&order);
    kernel.schedule("second", move |kernel| {
      kernel.emit_effect(crate::EffectEvent::new(
        crate::EffectZone::Pure,
        "second",
        "ok",
      ));
      order_b.lock().unwrap().push("second".to_string());
      Ok(())
    });

    kernel.run_all().unwrap();
    let result = order.lock().unwrap().clone();
    assert_eq!(result, vec!["first".to_string(), "second".to_string()]);
  }
}
