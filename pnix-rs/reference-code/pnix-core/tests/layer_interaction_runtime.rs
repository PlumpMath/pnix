//! 레이어 상호작용 런타임 테스트: 런타임에서의 레이어 상호작용 테스트
//!
//! 런타임에서 프로세스의 capability와 effect zone 정책이 올바르게 동작하는지 검증합니다.

use pnix_core::effects::{Capability, CapabilitySet, EffectZone, ZoneCapabilities};
use pnix_core::runtime::{Process, ProcessId};

#[test]
fn effect_policy_allows_runtime_process_capabilities() {
  let process = Process::new(
    ProcessId(1),
    EffectZone::Interop,
    vec![Capability::Read, Capability::SpawnProcess],
    None,
  );
  let caps: CapabilitySet = process.capabilities.iter().copied().collect();
  let policy = ZoneCapabilities::from_seto_default();

  assert!(policy.allows(process.effect_zone, &caps));
}

#[test]
fn effect_policy_rejects_pure_zone_side_effects() {
  let process = Process::new(
    ProcessId(2),
    EffectZone::Pure,
    vec![Capability::Write],
    None,
  );
  let caps: CapabilitySet = process.capabilities.iter().copied().collect();
  let policy = ZoneCapabilities::from_seto_default();

  assert!(!policy.allows(process.effect_zone, &caps));
}
