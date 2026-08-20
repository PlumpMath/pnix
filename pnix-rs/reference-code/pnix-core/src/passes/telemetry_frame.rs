//! FRP Telemetry Frame 구조
//!
//! pnix-old의 meaning_core/telemetry/frp_frame.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 없음

/// FRP 캐시 텔레메트리 단일 프레임: FRP 캐시 성능 추적을 위한 단일 프레임 데이터
///
/// 헌법 P0-1 준수: 구조 정의만, 실행 없음
#[derive(Clone, Debug, PartialEq)]
pub struct FrpTelemetryFrame {
  // === Frame identification ===
  /// 순차 프레임 카운터
  pub frame_index: u64,
  /// 벽시계 시간 (시작 이후 초)
  pub time: f64,

  // === Policy configuration ===
  /// 현재 정책 이름 ("default", "conservative", "aggressive")
  pub policy_name: String,
  /// 캐싱을 위한 최소 표현식 크기
  pub min_size: u32,
  /// 프레임당 고려할 최대 후보 수
  pub max_candidates: u32,
  /// 허용되는 최대 메모 항목 수
  pub max_memo_entries: u32,
  /// 자동 튜닝을 위한 목표 히트율
  pub target_hit_rate: f32,

  // === Cache statistics (this frame) ===
  /// 이 프레임의 캐시 히트
  pub hits: u32,
  /// 이 프레임의 캐시 미스
  pub misses: u32,
  /// 히트율 (hits / (hits + misses))
  pub hit_rate: f32,

  // === Memo state (current) ===
  /// 현재 메모 항목 수
  pub memo_len: u32,
  /// 총 제거 수 (누적)
  pub memo_evictions: u32,

  // === Plan statistics (this frame) ===
  /// 캐싱을 위해 선택된 후보 수
  pub candidates_selected: u32,
  /// 전체 표현식이 캐시 가능한지 여부
  pub whole_cacheable: bool,

  // === Tuner decision (this frame) ===
  /// 결정: "hold", "increase_min_size", "decrease_min_size"
  pub decision: String,
  /// 결정 이유
  pub reason: String,
}

impl Default for FrpTelemetryFrame {
  fn default() -> Self {
    Self {
      frame_index: 0,
      time: 0.0,
      policy_name: "default".to_string(),
      min_size: 3,
      max_candidates: 16,
      max_memo_entries: 256,
      target_hit_rate: 0.70,
      hits: 0,
      misses: 0,
      hit_rate: 0.0,
      memo_len: 0,
      memo_evictions: 0,
      candidates_selected: 0,
      whole_cacheable: false,
      decision: "hold".to_string(),
      reason: String::new(),
    }
  }
}

impl FrpTelemetryFrame {
  /// 주어진 인덱스와 시간으로 새 프레임 생성
  pub fn new(frame_index: u64, time: f64) -> Self {
    Self {
      frame_index,
      time,
      ..Default::default()
    }
  }

  // 헌법 준수 (P0-1): 값 계산 함수 제거
  // calculate_hit_rate(), is_below_target(), is_above_ceiling(), memo_fill_pct() 등의 값 계산 함수는
  // 실행/상태 업데이트는 executor/runtime 계층에서 구현하세요.
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_frame_default() {
    let frame = FrpTelemetryFrame::default();
    assert_eq!(frame.frame_index, 0);
    assert_eq!(frame.policy_name, "default");
    assert_eq!(frame.min_size, 3);
  }

  // 헌법 준수 (P0-1): 값 계산 테스트 제거
  // calculate_hit_rate(), memo_fill_pct() 테스트는 executor에서 수행하세요.
}
