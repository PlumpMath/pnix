//! Fixture-existence and structural-coverage check for the
//! Phase-7 `.px` judgement-protocol spec. Lifted out of
//! `src/judgement_protocol.rs` (inline `#[cfg(test)]`) on
//! 2026-05-06 because:
//!
//! - `forbid_runtime_symbols_in_src` (sibling test) text-scans
//!   `src/**/*.rs` for `std::fs` and other runtime symbols.
//!   The fixture-load assertion uses `std::fs::read_to_string`
//!   to read `fixtures/judgement_protocol/protocol-spec.px`,
//!   which the text-scan flagged as a forbidden runtime symbol
//!   even though the call lives inside `#[cfg(test)]` and never
//!   compiles into production builds.
//! - CLAUDE.md note 8 codifies the same rule for
//!   `pnix-query-runtime/src` ("inline `#[cfg(test)]` is not
//!   allowed"). Moving the fixture-load test out follows that
//!   pattern uniformly.
//!
//! The constitution requirement remains: the Rust mirror in
//! `crates/pnix-core/src/judgement_protocol.rs` must stay
//! semantically aligned with the `.px` spec fixture. This test
//! is the minimum check that the spec exists and contains the
//! required protocol-message section names.

use std::fs;
use std::path::Path;

#[test]
fn px_protocol_spec_fixture_exists() {
  let manifest_dir = env!("CARGO_MANIFEST_DIR");
  // pnix-core/Cargo.toml → pnix root
  let root = Path::new(manifest_dir)
    .parent() // crates/
    .and_then(|p| p.parent()); // pnix/
  let fixture_path = root
    .map(|r| r.join("fixtures/judgement_protocol/protocol-spec.px"))
    .expect("cannot resolve pnix root");
  assert!(
    fixture_path.exists(),
    "Phase 7 .px protocol spec fixture not found at {}",
    fixture_path.display()
  );
  let content = fs::read_to_string(&fixture_path).unwrap();
  pnix_core::lang::pnix::parse_expr(&content).expect("Phase 7 .px protocol spec should parse");
  // 기본 구조 검증: 필수 protocol message 이름이 모두 있는지
  for expected in [
    "evidence-envelope",
    "judgement-request",
    "judgement-event",
    "promotion-event",
    "peer-evidence",
    "evidence-sources",
    "protocol-invariants",
  ] {
    assert!(
      content.contains(expected),
      ".px spec missing required section: {expected}"
    );
  }
  // Rust mirror의 타입 이름과 .px spec의 section 이름이 대응하는지 확인
  // (의미적 일치의 최소 검증)
  assert!(
    content.contains("append-only"),
    ".px spec must declare append-only invariant"
  );
  assert!(
    content.contains("Candidate"),
    ".px spec must mention Candidate status"
  );
}
