//! 런타임 커널 구현
//!
//! 결정론적 실행을 위한 시간 관리, 태스크 스케줄링, 이펙트 수집을 담당

use crate::{
  ClockConfig, ClockMode, ClockTick, EffectEvent, EffectHost, KernelClock, KernelResult, TaskId,
  TaskScheduler,
};
use pnix_runtime_api::{CtConfig, EvalConfig, FrpConfig};

/// 커널 설정: 결정론적 실행을 위한 시간 및 시드 설정
#[derive(Debug, Clone)]
pub struct KernelConfig {
  /// 결정론적 모드 여부
  pub deterministic: bool,
  /// 랜덤 시드 (결정론적 모드에서 사용)
  pub seed: Option<u64>,
  /// 현재 시간 (밀리초)
  pub now_ms: i64,
  /// 클럭 스텝 간격 (밀리초)
  pub clock_step_ms: i64,
}

impl KernelConfig {
  /// 새 커널 설정 생성: 파라미터 검증 포함
  pub fn new(
    deterministic: bool,
    seed: Option<u64>,
    now_ms: i64,
    clock_step_ms: i64,
  ) -> KernelResult<Self> {
    if clock_step_ms < 0 {
      return Err(pnix_runtime_api::RuntimeError::message(
        "clock_step_ms must be >= 0",
      ));
    }

    Ok(Self {
      deterministic,
      seed,
      now_ms,
      clock_step_ms,
    })
  }

  /// 결정론적 기본값: 테스트 및 재현 가능한 실행용
  pub fn deterministic_defaults() -> Self {
    Self {
      deterministic: true,
      seed: None,
      now_ms: 0,
      clock_step_ms: 16,
    }
  }

  /// 클럭 모드 반환: 결정론적 또는 실시간
  pub fn clock_mode(&self) -> ClockMode {
    if self.deterministic {
      ClockMode::Deterministic
    } else {
      ClockMode::Realtime
    }
  }

  /// 클럭 설정 반환: 모드에 맞는 ClockConfig 생성
  pub fn clock_config(&self) -> ClockConfig {
    match self.clock_mode() {
      ClockMode::Deterministic => ClockConfig::deterministic(self.now_ms, self.clock_step_ms),
      ClockMode::Realtime => ClockConfig::realtime(self.now_ms, self.clock_step_ms),
    }
  }

  pub fn from_eval_config(config: &EvalConfig) -> KernelResult<Self> {
    let now_ms = config.now_ms.unwrap_or(0);
    let clock_step_ms = config.clock_step_ms.unwrap_or(16);
    Self::new(config.deterministic, config.seed, now_ms, clock_step_ms)
  }

  /// FrpConfig로부터 커널 설정 생성 (항상 결정론적)
  pub fn from_frp_config(config: &FrpConfig) -> KernelResult<Self> {
    let now_ms = config.now_ms.unwrap_or(0);
    let clock_step_ms = config.clock_step_ms.unwrap_or(16);
    Self::new(true, config.seed, now_ms, clock_step_ms)
  }

  /// CtConfig로부터 커널 설정 생성 (항상 결정론적)
  pub fn from_ct_config(config: &CtConfig) -> KernelResult<Self> {
    let now_ms = config.now_ms.unwrap_or(0);
    let clock_step_ms = config.clock_step_ms.unwrap_or(16);
    Self::new(true, config.seed, now_ms, clock_step_ms)
  }
}

/// 커널 통계: 실행된 태스크 수 및 틱 수 추적
#[derive(Debug, Default, Clone, Copy)]
pub struct KernelStats {
  /// 실행된 태스크 수
  pub tasks_run: u64,
  /// 경과한 틱 수
  pub ticks: u64,
}

/// 런타임 커널: 시간 관리, 태스크 스케줄링, 이펙트 수집을 담당하는 핵심 컴포넌트
pub struct Kernel {
  /// 커널 설정
  config: KernelConfig,
  /// 클럭 (시간 관리)
  clock: KernelClock,
  /// 태스크 스케줄러
  scheduler: TaskScheduler,
  /// 이펙트 호스트 (부수 효과 수집)
  effects: EffectHost,
  /// 실행 통계
  stats: KernelStats,
}

impl Kernel {
  /// 새 커널 생성: 설정에 따라 클럭, 스케줄러, 이펙트 호스트 초기화
  pub fn new(config: KernelConfig) -> KernelResult<Self> {
    let clock = KernelClock::new(config.clock_config())?;
    Ok(Self {
      config,
      clock,
      scheduler: TaskScheduler::new(),
      effects: EffectHost::new(),
      stats: KernelStats::default(),
    })
  }

  /// EvalConfig로부터 커널 생성
  pub fn from_eval_config(config: &EvalConfig) -> KernelResult<Self> {
    Self::new(KernelConfig::from_eval_config(config)?)
  }

  /// 커널 설정 참조 반환
  pub fn config(&self) -> &KernelConfig {
    &self.config
  }

  /// 클럭 참조 반환
  pub fn clock(&self) -> &KernelClock {
    &self.clock
  }

  /// 클럭 가변 참조 반환
  pub fn clock_mut(&mut self) -> &mut KernelClock {
    &mut self.clock
  }

  /// 태스크 스케줄러 참조 반환
  pub fn scheduler(&self) -> &TaskScheduler {
    &self.scheduler
  }

  /// 이펙트 호스트 참조 반환
  pub fn effects(&self) -> &EffectHost {
    &self.effects
  }

  pub fn effects_mut(&mut self) -> &mut EffectHost {
    &mut self.effects
  }

  pub fn stats(&self) -> KernelStats {
    self.stats
  }

  pub fn tick(&mut self) -> ClockTick {
    let tick = self.clock.tick();
    self.stats.ticks = self.stats.ticks.saturating_add(1);
    tick
  }

  pub fn schedule(
    &mut self,
    label: impl Into<String>,
    action: impl FnOnce(&mut Kernel) -> KernelResult<()> + 'static,
  ) -> TaskId {
    self.scheduler.schedule(label, action)
  }

  pub fn schedule_at(
    &mut self,
    label: impl Into<String>,
    due_ms: i64,
    action: impl FnOnce(&mut Kernel) -> KernelResult<()> + 'static,
  ) -> TaskId {
    self.scheduler.schedule_at(label, due_ms, action)
  }

  pub fn schedule_after(
    &mut self,
    label: impl Into<String>,
    delay_ms: i64,
    action: impl FnOnce(&mut Kernel) -> KernelResult<()> + 'static,
  ) -> TaskId {
    let due = self.now_ms().saturating_add(delay_ms);
    self.schedule_at(label, due, action)
  }

  pub fn run_next(&mut self) -> KernelResult<Option<TaskId>> {
    self.scheduler.promote_due(self.now_ms());
    let task = match self.scheduler.pop_next() {
      Some(task) => task,
      None => return Ok(None),
    };

    let task_id = task.id;
    task.run(self)?;
    self.stats.tasks_run = self.stats.tasks_run.saturating_add(1);
    Ok(Some(task_id))
  }

  pub fn run_all(&mut self) -> KernelResult<usize> {
    let mut count = 0;
    while self.run_next()?.is_some() {
      count += 1;
    }
    Ok(count)
  }

  pub fn now_ms(&self) -> i64 {
    self.clock.now_ms()
  }

  pub fn delta_ms(&self) -> i64 {
    self.clock.delta_ms()
  }

  pub fn delta_secs(&self) -> f64 {
    self.clock.delta_secs()
  }

  pub fn emit_effect(&mut self, event: EffectEvent) {
    self.effects.emit(event);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn kernel_config_from_eval_config_defaults() {
    let config = EvalConfig::default();
    let kernel_config = KernelConfig::from_eval_config(&config).unwrap();
    assert!(kernel_config.deterministic);
    assert_eq!(kernel_config.now_ms, 0);
    assert_eq!(kernel_config.clock_step_ms, 16);
  }
}
