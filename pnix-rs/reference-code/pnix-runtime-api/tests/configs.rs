//! Runtime API 설정 테스트: 런타임 API 설정 빌더 테스트
//!
//! EvalConfig, FrpConfig, CtConfig 등의 설정 빌더 기능을 테스트합니다.

use pnix_runtime_api::{CtConfig, EvalConfig, FrpConfig};

#[test]
fn test_eval_config_builders() {
  let config = EvalConfig::default()
    .nondeterministic()
    .with_seed(7)
    .with_now_ms(123)
    .with_clock_step_ms(10)
    .with_float_pattern_tolerance(0.25);

  assert!(!config.deterministic);
  assert_eq!(config.seed, Some(7));
  assert_eq!(config.now_ms, Some(123));
  assert_eq!(config.clock_step_ms, Some(10));
  assert_eq!(config.float_pattern_tolerance, Some(0.25));
}

#[test]
fn test_frp_config_builders() {
  let config = FrpConfig::default()
    .with_tick_index(5)
    .with_seed(11)
    .with_now_ms(456)
    .with_clock_step_ms(20);

  assert_eq!(config.tick_index, 5);
  assert_eq!(config.seed, Some(11));
  assert_eq!(config.now_ms, Some(456));
  assert_eq!(config.clock_step_ms, Some(20));
}

#[test]
fn test_ct_config_builders() {
  let config = CtConfig::default()
    .permissive()
    .with_seed(3)
    .with_now_ms(789)
    .with_clock_step_ms(30);

  assert!(!config.strict);
  assert_eq!(config.seed, Some(3));
  assert_eq!(config.now_ms, Some(789));
  assert_eq!(config.clock_step_ms, Some(30));
}
