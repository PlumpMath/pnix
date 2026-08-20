//! I5.3 integration tests for puncheetah consent pure-compile cache.

use pnix_core::puncheetah_consent_compile_cache::{
  compute_compile_fingerprint, granted_scope_hash_for_request, PuncheetahConsentCompileCache,
  PuncheetahConsentCompileCacheKey, PuncheetahConsentCompileDecision,
  PuncheetahConsentCompilePureRequest, PuncheetahConsentJoinRetryMode,
};

fn sample_request() -> PuncheetahConsentCompilePureRequest {
  PuncheetahConsentCompilePureRequest {
    request_id: "ccpr_test_100".to_string(),
    idempotency_key: "ik_test_100".to_string(),
    template_hash: "sha256:template-aa11".to_string(),
    policy_hash: "sha256:policy-bb22".to_string(),
    runtime_policy_hash: "sha256:runtime-cc33".to_string(),
    backend_spec_hash: "sha256:backend-dd44".to_string(),
    request_scope_hash: "sha256:reqscope-ee55".to_string(),
    compiler_version: "v0.1.36".to_string(),
    template_id: Some("ct_runtime_python_api".to_string()),
    session_id: Some("sess_01".to_string()),
  }
}

#[test]
fn puncheetah_consent_compile_fingerprint_is_deterministic() {
  let request = sample_request();
  let a = compute_compile_fingerprint(&request);
  let b = compute_compile_fingerprint(&request);
  assert_eq!(a, b);
  assert!(a.starts_with("sha256:"));
}

#[test]
fn puncheetah_consent_compile_fingerprint_changes_when_inputs_change() {
  let mut request = sample_request();
  let baseline = compute_compile_fingerprint(&request);
  request.compiler_version = "v0.1.37".to_string();
  assert_ne!(baseline, compute_compile_fingerprint(&request));
}

#[test]
fn puncheetah_consent_pure_compile_is_replay_identical() {
  let request = sample_request();
  let first = PuncheetahConsentCompileCache::pure_compile(&request);
  let second = PuncheetahConsentCompileCache::pure_compile(&request);
  assert_eq!(first, second);
  assert_eq!(first.decision, PuncheetahConsentCompileDecision::CacheMiss);
}

#[test]
fn puncheetah_consent_primary_cache_hit_on_same_fingerprint_and_idempotency() {
  let request = sample_request();
  let fingerprint = compute_compile_fingerprint(&request);
  let mut cache = PuncheetahConsentCompileCache::with_capacity(8);

  let miss = cache.compile_with_cache(&request);
  assert_eq!(miss.decision, PuncheetahConsentCompileDecision::CacheMiss);
  assert_eq!(cache.stats().primary_misses, 1);

  let hit = cache.compile_with_cache(&request);
  assert_eq!(
    hit.decision,
    PuncheetahConsentCompileDecision::CacheHitTier1
  );
  assert_eq!(hit.compile_fingerprint, fingerprint);
  assert_eq!(cache.stats().primary_hits, 1);
  assert_eq!(cache.len(), 1);
}

#[test]
fn puncheetah_consent_bounded_cache_evicts_oldest_entry() {
  let mut cache = PuncheetahConsentCompileCache::with_capacity(2);
  let base = sample_request();

  let mut r1 = base.clone();
  r1.request_id = "r1".to_string();
  r1.idempotency_key = "ik1".to_string();
  r1.request_scope_hash = "sha256:scope-1".to_string();
  cache.compile_with_cache(&r1);

  let mut r2 = base.clone();
  r2.request_id = "r2".to_string();
  r2.idempotency_key = "ik2".to_string();
  r2.request_scope_hash = "sha256:scope-2".to_string();
  cache.compile_with_cache(&r2);

  let mut r3 = base.clone();
  r3.request_id = "r3".to_string();
  r3.idempotency_key = "ik3".to_string();
  r3.request_scope_hash = "sha256:scope-3".to_string();
  cache.compile_with_cache(&r3);

  assert_eq!(cache.len(), 2);
  assert_eq!(cache.stats().evictions, 1);

  let fp1 = compute_compile_fingerprint(&r1);
  let key1 = PuncheetahConsentCompileCacheKey::new(fp1, "ik1");
  assert!(cache.get(&key1).is_none());
}

#[test]
fn puncheetah_consent_divergent_retry_forbidden_for_fingerprint_mismatch() {
  let mut cache = PuncheetahConsentCompileCache::with_capacity(8);
  let mut first = sample_request();
  cache.compile_with_cache(&first);

  first.request_scope_hash = "sha256:reqscope-changed".to_string();
  let divergent = cache.compile_with_cache(&first);
  assert_eq!(
    divergent.decision,
    PuncheetahConsentCompileDecision::DivergentRetryForbidden
  );
  assert_eq!(cache.stats().divergent_rejections, 1);

  let rule = cache.evaluate_join_retry_rule(
    &first.idempotency_key,
    &compute_compile_fingerprint(&first),
    PuncheetahConsentJoinRetryMode::DivergentRetry,
  );
  assert!(!rule.allowed);
}

#[test]
fn puncheetah_consent_join_rule_allows_matching_inflight_binding() {
  let request = sample_request();
  let fingerprint = compute_compile_fingerprint(&request);
  let mut cache = PuncheetahConsentCompileCache::with_capacity(8);
  cache.track_inflight(
    &request.idempotency_key,
    &fingerprint,
    Some("gr_shared".to_string()),
  );

  let rule = cache.evaluate_join_retry_rule(
    &request.idempotency_key,
    &fingerprint,
    PuncheetahConsentJoinRetryMode::Join,
  );
  assert!(rule.allowed);
}

#[test]
fn puncheetah_consent_session_tier_reuses_skeleton_without_primary_entry() {
  let request = sample_request();
  let mut cache = PuncheetahConsentCompileCache::with_capacity(8);
  cache.compile_with_cache(&request);

  let mut retry = request.clone();
  retry.request_id = "ccpr_retry".to_string();
  retry.idempotency_key = "ik_different".to_string();
  let hit = cache.compile_with_cache(&retry);
  assert_eq!(
    hit.decision,
    PuncheetahConsentCompileDecision::CacheHitTier2
  );
  assert_eq!(cache.stats().tier2_hits, 1);
  assert_eq!(cache.stats().primary_hits, 0);
}

#[test]
fn puncheetah_consent_granted_scope_hash_is_deterministic() {
  let request = sample_request();
  assert_eq!(
    granted_scope_hash_for_request(&request),
    granted_scope_hash_for_request(&request)
  );
}
