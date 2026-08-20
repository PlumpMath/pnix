//! 이펙트 호스트: 부수 효과 이벤트 수집 및 관리

use pnix_fxcore_types::Effect as FxEffect;

/// 이펙트 존 타입 별칭
pub type EffectZone = FxEffect;

/// 이펙트 이벤트: 발생한 부수 효과
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectEvent {
  /// 이펙트 존 (영역)
  pub zone: EffectZone,
  /// 이벤트 이름
  pub name: String,
  /// 이벤트 상세 정보
  pub detail: String,
}

impl EffectEvent {
  pub fn new(zone: EffectZone, name: impl Into<String>, detail: impl Into<String>) -> Self {
    Self {
      zone,
      name: name.into(),
      detail: detail.into(),
    }
  }
}

/// 이펙트 호스트: 이펙트 이벤트 수집 및 관리
#[derive(Debug, Default)]
pub struct EffectHost {
  /// 수집된 이벤트 목록
  events: Vec<EffectEvent>,
}

impl EffectHost {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn emit(&mut self, event: EffectEvent) {
    self.events.push(event);
  }

  pub fn snapshot(&self) -> &[EffectEvent] {
    &self.events
  }

  pub fn drain(&mut self) -> Vec<EffectEvent> {
    std::mem::take(&mut self.events)
  }

  pub fn is_empty(&self) -> bool {
    self.events.is_empty()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn effect_host_records() {
    let mut host = EffectHost::new();
    assert!(host.is_empty());

    host.emit(EffectEvent::new(EffectZone::Pure, "tick", "ok"));
    assert_eq!(host.snapshot().len(), 1);
  }
}
