//! 커널 클럭: 시간 관리 및 틱 생성

use std::time::Instant;

use crate::KernelResult;

/// 클럭 모드: 결정론적 또는 실시간
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockMode {
  /// 결정론적 모드: 재현 가능한 시간
  Deterministic,
  /// 실시간 모드: 실제 시간 사용
  Realtime,
}

/// 클럭 설정: 모드 및 시간 파라미터
#[derive(Debug, Clone, Copy)]
pub struct ClockConfig {
  /// 클럭 모드
  pub mode: ClockMode,
  /// 현재 시간 (밀리초)
  pub now_ms: i64,
  /// 클럭 스텝 간격 (밀리초)
  pub clock_step_ms: i64,
}

impl ClockConfig {
  pub fn deterministic(now_ms: i64, clock_step_ms: i64) -> Self {
    Self {
      mode: ClockMode::Deterministic,
      now_ms,
      clock_step_ms,
    }
  }

  pub fn realtime(now_ms: i64, clock_step_ms: i64) -> Self {
    Self {
      mode: ClockMode::Realtime,
      now_ms,
      clock_step_ms,
    }
  }
}

/// 클럭 틱: 한 틱의 시간 정보
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockTick {
  /// 틱 인덱스
  pub tick_index: u64,
  /// 현재 시간 (밀리초)
  pub now_ms: i64,
  /// 델타 시간 (밀리초)
  pub delta_ms: i64,
}

/// 커널 클럭: 시간 관리 및 틱 생성
#[derive(Debug, Clone)]
pub struct KernelClock {
  /// 클럭 모드
  mode: ClockMode,
  /// 기준 시간 (밀리초)
  base_ms: i64,
  /// 스텝 간격 (밀리초)
  step_ms: i64,
  /// 틱 카운트
  tick_count: u64,
  /// 현재 시간 (밀리초)
  current_ms: i64,
  /// 델타 시간 (밀리초)
  delta_ms: i64,
  /// 실시간 모드 시작 시점 (실시간 모드일 때만)
  realtime_start: Option<Instant>,
}

impl KernelClock {
  pub fn new(config: ClockConfig) -> KernelResult<Self> {
    if config.clock_step_ms < 0 {
      return Err(pnix_runtime_api::RuntimeError::message(
        "clock_step_ms must be >= 0",
      ));
    }

    let realtime_start = match config.mode {
      ClockMode::Deterministic => None,
      ClockMode::Realtime => Some(Instant::now()),
    };

    Ok(Self {
      mode: config.mode,
      base_ms: config.now_ms,
      step_ms: config.clock_step_ms,
      tick_count: 0,
      current_ms: config.now_ms,
      delta_ms: config.clock_step_ms,
      realtime_start,
    })
  }

  pub fn mode(&self) -> ClockMode {
    self.mode
  }

  pub fn now_ms(&self) -> i64 {
    self.current_ms
  }

  pub fn delta_ms(&self) -> i64 {
    self.delta_ms
  }

  pub fn delta_secs(&self) -> f64 {
    self.delta_ms as f64 / 1000.0
  }

  pub fn tick_index(&self) -> u64 {
    self.tick_count
  }

  pub fn tick(&mut self) -> ClockTick {
    match self.mode {
      ClockMode::Deterministic => {
        self.tick_count += 1;
        self.current_ms = self.base_ms + (self.tick_count as i64 * self.step_ms);
        self.delta_ms = self.step_ms;
      }
      ClockMode::Realtime => {
        let elapsed_ms = self
          .realtime_start
          .map(|start| start.elapsed().as_millis() as i64)
          .unwrap_or(0);
        let now = self.base_ms + elapsed_ms;
        let delta = if self.step_ms > 0 {
          self.step_ms
        } else {
          now.saturating_sub(self.current_ms)
        };
        self.current_ms = now;
        self.delta_ms = delta;
        self.tick_count = self.tick_count.saturating_add(1);
      }
    }

    ClockTick {
      tick_index: self.tick_count,
      now_ms: self.current_ms,
      delta_ms: self.delta_ms,
    }
  }

  pub fn advance(&mut self, steps: u64) -> KernelResult<ClockTick> {
    if self.mode != ClockMode::Deterministic {
      return Err(pnix_runtime_api::RuntimeError::unimplemented(
        "clock.advance (realtime)",
      ));
    }

    self.tick_count = self.tick_count.saturating_add(steps);
    self.current_ms = self.base_ms + (self.tick_count as i64 * self.step_ms);
    self.delta_ms = self.step_ms;

    Ok(ClockTick {
      tick_index: self.tick_count,
      now_ms: self.current_ms,
      delta_ms: self.delta_ms,
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn clock_deterministic_ticks() {
    let mut clock = KernelClock::new(ClockConfig::deterministic(0, 10)).unwrap();
    assert_eq!(clock.now_ms(), 0);

    let tick1 = clock.tick();
    assert_eq!(tick1.tick_index, 1);
    assert_eq!(tick1.now_ms, 10);
    assert_eq!(tick1.delta_ms, 10);

    let tick2 = clock.tick();
    assert_eq!(tick2.tick_index, 2);
    assert_eq!(tick2.now_ms, 20);
    assert_eq!(tick2.delta_ms, 10);
  }
}
