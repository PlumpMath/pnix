//! RPC 결정론성 테스트: RPC 호출의 결정론적 동작 테스트
//!
//! RPC 호출이 결정론적으로 동작하는지 검증합니다.

#[test]
fn backoff_defaults_to_fixed_schedule_when_seed_zero() {
  let policy = pnix_executor_graph::RpcRetryPolicy::new(3, 100);
  let delays: Vec<u64> = (0..3)
    .map(|attempt| policy.backoff_for_attempt(attempt).as_millis() as u64)
    .collect();

  assert_eq!(delays, vec![100, 200, 400]);
}

#[test]
fn backoff_seeded_schedule_is_deterministic() {
  let policy = pnix_executor_graph::RpcRetryPolicy::new(3, 100).with_seed(42);
  let delays: Vec<u64> = (0..3)
    .map(|attempt| policy.backoff_for_attempt(attempt).as_millis() as u64)
    .collect();

  assert_eq!(delays, vec![113, 210, 498]);
}
