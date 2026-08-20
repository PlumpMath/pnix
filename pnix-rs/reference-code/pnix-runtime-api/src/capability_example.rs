//! RuntimeCapabilities 사용 예제
//!
//! W06 완료 조건: "최소 1개 조합에서 명시적 거부 + 메시지가 구현되고 테스트가 있다"

use crate::{CapabilityCheckResult, CapabilityChecker, RuntimeCapabilities, RuntimeCapability};

/// Legacy eval 엔진의 capabilities 예제
pub struct LegacyEvalCapabilities;

impl CapabilityChecker for LegacyEvalCapabilities {
  fn capabilities(&self) -> RuntimeCapabilities {
    RuntimeCapabilities::new()
      .with_capability(RuntimeCapability::Pure)
      .with_capability(RuntimeCapability::Math)
      .with_capability(RuntimeCapability::Arithmetic)
      .with_capability(RuntimeCapability::Comparison)
      .with_capability(RuntimeCapability::Logic)
      .with_capability(RuntimeCapability::SSAEval)
      .with_engine("legacy-eval")
      .with_option("deterministic")
  }
}

/// FRP 엔진의 capabilities 예제
pub struct FrpCapabilities;

impl CapabilityChecker for FrpCapabilities {
  fn capabilities(&self) -> RuntimeCapabilities {
    RuntimeCapabilities::new()
      .with_capability(RuntimeCapability::Pure)
      .with_capability(RuntimeCapability::Math)
      .with_capability(RuntimeCapability::Arithmetic)
      .with_capability(RuntimeCapability::FRPTick)
      .with_engine("frp")
      .with_option("deterministic")
      .with_option("hot-swap")
  }
}

/// Network capability가 필요한 모듈을 체크하는 예제
pub fn check_network_capability(checker: &dyn CapabilityChecker) -> CapabilityCheckResult {
  checker.check_capabilities(&[RuntimeCapability::Network], &[], &[])
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_legacy_eval_capabilities() {
    let checker = LegacyEvalCapabilities;
    let caps = checker.capabilities();

    assert!(caps.supports(&RuntimeCapability::Pure));
    assert!(caps.supports(&RuntimeCapability::Math));
    assert!(!caps.supports(&RuntimeCapability::Network));
    assert!(caps.supports_engine("legacy-eval"));
  }

  #[test]
  fn test_network_capability_rejection() {
    let checker = LegacyEvalCapabilities;
    let result = check_network_capability(&checker);

    // Legacy eval은 Network를 지원하지 않으므로 거부되어야 함
    assert!(!result.success);
    assert_eq!(result.missing_capabilities.len(), 1);
    assert_eq!(result.missing_capabilities[0], RuntimeCapability::Network);
    assert!(result.message.contains("missing capabilities"));
    assert!(result.message.contains("network"));
  }

  #[test]
  fn test_supported_capability_passes() {
    let checker = LegacyEvalCapabilities;
    let result = checker.check_capabilities(
      &[RuntimeCapability::Pure, RuntimeCapability::Math],
      &[],
      &[],
    );

    // 지원하는 capability는 통과해야 함
    assert!(result.success);
    assert!(result.missing_capabilities.is_empty());
  }

  #[test]
  fn test_frp_capabilities() {
    let checker = FrpCapabilities;
    let caps = checker.capabilities();

    assert!(caps.supports(&RuntimeCapability::FRPTick));
    assert!(caps.supports_option("hot-swap"));
  }
}
