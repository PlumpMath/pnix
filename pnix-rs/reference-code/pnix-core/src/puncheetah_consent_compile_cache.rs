//! I5.3: Puncheetah consent-to-grant **pure compile** cache (replay-safe, bounded).
//!
//! Implements the deterministic half of
//! `docs/puck/puck-consent-compiler-cache-replay-v0.1.md` under puncheetah naming.
//! Bind/acquire (environment-dependent grant issuance) stays outside this module.

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};

/// Schema id for cache owner receipts / envelopes.
pub const PUNCHEETAH_CONSENT_COMPILE_CACHE_SCHEMA: &str = "puncheetah.consent-compile-cache.v0";

/// Schema id for pure-compile result envelopes stored in the cache.
pub const PUNCHEETAH_CONSENT_COMPILE_RESULT_SCHEMA: &str = "puncheetah.consent-compile-result.v0";

/// Default bounded capacity for the primary compile-result cache.
pub const DEFAULT_PUNCHEETAH_CONSENT_COMPILE_CACHE_CAPACITY: usize = 256;

/// Pure compile request inputs (deterministic; no bind/acquire fields).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PuncheetahConsentCompilePureRequest {
  pub request_id: String,
  pub idempotency_key: String,
  pub template_hash: String,
  pub policy_hash: String,
  pub runtime_policy_hash: String,
  pub backend_spec_hash: String,
  pub request_scope_hash: String,
  pub compiler_version: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub template_id: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub session_id: Option<String>,
}

/// Narrowed grant skeleton emitted by pure compile (scope never expands).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PuncheetahGrantSpecSkeleton {
  pub template_hash: String,
  pub policy_hash: String,
  pub request_scope_hash: String,
  pub granted_scope_hash: String,
  pub compile_fingerprint: String,
}

/// Out-of-scope diff carrier for replay audits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PuncheetahOutOfScopeDiff {
  pub requested_scope_hash: String,
  pub candidate_template_hash: String,
  pub reason: String,
}

/// Replay decision labels aligned with puck consent replay schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PuncheetahConsentCompileDecision {
  CacheHitTier1,
  CacheHitTier2,
  CacheHitTier3,
  CacheMiss,
  JoinReuse,
  RetryNewGrant,
  OutOfScope,
  PolicyDenied,
  DivergentRetryForbidden,
}

/// Grant reuse mode for replay results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PuncheetahConsentReuseMode {
  JoinSharedGrant,
  RetryNewGrant,
  NewGrant,
  SkeletonOnly,
}

/// Join / retry / divergent classification for idempotency replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PuncheetahConsentJoinRetryMode {
  Join,
  Retry,
  DivergentRetry,
}

/// Join/retry rule evaluation envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PuncheetahConsentJoinRetryRule {
  pub schema: String,
  pub idempotency_key: String,
  pub compile_fingerprint: String,
  pub mode: PuncheetahConsentJoinRetryMode,
  pub allowed: bool,
  pub reason: String,
}

/// Cached pure-compile result envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PuncheetahConsentCompileResultEnvelope {
  pub schema: String,
  pub request_id: String,
  pub idempotency_key: String,
  pub compile_fingerprint: String,
  pub template_hash: String,
  pub policy_hash: String,
  pub backend_spec_hash: String,
  pub request_scope_hash: String,
  pub granted_scope_hash: String,
  pub compiler_version: String,
  pub decision: PuncheetahConsentCompileDecision,
  pub reuse_mode: PuncheetahConsentReuseMode,
  pub grant_spec_skeleton: PuncheetahGrantSpecSkeleton,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub out_of_scope_diff: Option<PuncheetahOutOfScopeDiff>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub grant_id: Option<String>,
}

/// Primary cache key: `compile_fingerprint + idempotency_key`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PuncheetahConsentCompileCacheKey {
  pub compile_fingerprint: String,
  pub idempotency_key: String,
}

impl PuncheetahConsentCompileCacheKey {
  pub fn new(compile_fingerprint: impl Into<String>, idempotency_key: impl Into<String>) -> Self {
    Self {
      compile_fingerprint: compile_fingerprint.into(),
      idempotency_key: idempotency_key.into(),
    }
  }
}

/// Tier-2 session skeleton cache key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SessionSkeletonKey {
  session_id: String,
  compile_fingerprint: String,
}

/// Tier-3 template skeleton cache key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TemplateSkeletonKey {
  template_id: String,
  compile_fingerprint: String,
}

/// In-flight join tracking (tier-1 idempotency lane).
#[derive(Debug, Clone, PartialEq, Eq)]
struct InflightBinding {
  compile_fingerprint: String,
  grant_id: Option<String>,
}

/// Cache counters for observability (replay-safe tallies only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PuncheetahConsentCompileCacheStats {
  pub primary_hits: u64,
  pub primary_misses: u64,
  pub tier2_hits: u64,
  pub tier3_hits: u64,
  pub evictions: u64,
  pub divergent_rejections: u64,
  pub inserts: u64,
}

/// Bounded in-memory pure-compile cache.
#[derive(Debug, Clone)]
pub struct PuncheetahConsentCompileCache {
  capacity: usize,
  primary: HashMap<PuncheetahConsentCompileCacheKey, PuncheetahConsentCompileResultEnvelope>,
  lru_order: VecDeque<PuncheetahConsentCompileCacheKey>,
  session_skeletons: HashMap<SessionSkeletonKey, PuncheetahGrantSpecSkeleton>,
  template_skeletons: HashMap<TemplateSkeletonKey, PuncheetahGrantSpecSkeleton>,
  inflight: HashMap<String, InflightBinding>,
  idempotency_fingerprints: HashMap<String, String>,
  stats: PuncheetahConsentCompileCacheStats,
}

impl PuncheetahConsentCompileCache {
  pub fn with_capacity(capacity: usize) -> Self {
    Self {
      capacity: capacity.max(1),
      primary: HashMap::new(),
      lru_order: VecDeque::new(),
      session_skeletons: HashMap::new(),
      template_skeletons: HashMap::new(),
      inflight: HashMap::new(),
      idempotency_fingerprints: HashMap::new(),
      stats: PuncheetahConsentCompileCacheStats::default(),
    }
  }

  pub fn capacity(&self) -> usize {
    self.capacity
  }

  pub fn stats(&self) -> PuncheetahConsentCompileCacheStats {
    self.stats
  }

  pub fn len(&self) -> usize {
    self.primary.len()
  }

  pub fn is_empty(&self) -> bool {
    self.primary.is_empty()
  }

  pub fn get(
    &self,
    key: &PuncheetahConsentCompileCacheKey,
  ) -> Option<&PuncheetahConsentCompileResultEnvelope> {
    self.primary.get(key)
  }

  /// Record an in-flight grant binding for join reuse (tier-1).
  pub fn track_inflight(
    &mut self,
    idempotency_key: impl Into<String>,
    compile_fingerprint: impl Into<String>,
    grant_id: Option<String>,
  ) {
    let idempotency_key = idempotency_key.into();
    let compile_fingerprint = compile_fingerprint.into();
    self.inflight.insert(
      idempotency_key.clone(),
      InflightBinding {
        compile_fingerprint: compile_fingerprint.clone(),
        grant_id,
      },
    );
    self
      .idempotency_fingerprints
      .insert(idempotency_key, compile_fingerprint);
  }

  /// Evaluate join / retry / divergent retry rule for an idempotency key.
  pub fn evaluate_join_retry_rule(
    &self,
    idempotency_key: &str,
    compile_fingerprint: &str,
    mode: PuncheetahConsentJoinRetryMode,
  ) -> PuncheetahConsentJoinRetryRule {
    let stored = self.idempotency_fingerprints.get(idempotency_key);
    let (allowed, reason) = match (stored, mode) {
      (None, PuncheetahConsentJoinRetryMode::Join) => (
        false,
        "no in-flight binding for idempotency key".to_string(),
      ),
      (None, PuncheetahConsentJoinRetryMode::Retry) => (
        true,
        "first attempt or prior attempt not recorded; retry may issue new grant".to_string(),
      ),
      (None, PuncheetahConsentJoinRetryMode::DivergentRetry) => (
        false,
        "no prior fingerprint recorded for divergent comparison".to_string(),
      ),
      (Some(prev), PuncheetahConsentJoinRetryMode::DivergentRetry)
        if prev != compile_fingerprint =>
      {
        (
          false,
          "idempotency key fingerprint mismatch; require new idempotency key".to_string(),
        )
      }
      (Some(prev), PuncheetahConsentJoinRetryMode::DivergentRetry) => {
        (true, format!("fingerprints match: {prev}"))
      }
      (Some(prev), PuncheetahConsentJoinRetryMode::Join) if prev != compile_fingerprint => (
        false,
        "join forbidden: compile_fingerprint mismatch for idempotency key".to_string(),
      ),
      (Some(_), PuncheetahConsentJoinRetryMode::Join) => {
        let inflight = self.inflight.get(idempotency_key);
        match inflight {
          Some(binding) if binding.grant_id.is_some() => {
            (true, "in-flight grant available for join".to_string())
          }
          Some(_) => (
            false,
            "in-flight binding exists but grant_id not yet bound".to_string(),
          ),
          None => (
            false,
            "fingerprint matches but no in-flight binding".to_string(),
          ),
        }
      }
      (Some(prev), PuncheetahConsentJoinRetryMode::Retry) if prev != compile_fingerprint => (
        false,
        "retry forbidden: compile_fingerprint mismatch for idempotency key".to_string(),
      ),
      (Some(_), PuncheetahConsentJoinRetryMode::Retry) => (
        true,
        "retry may issue new grant; skeleton may be reused".to_string(),
      ),
    };

    PuncheetahConsentJoinRetryRule {
      schema: "puncheetah.consent-join-retry-rule.v0".to_string(),
      idempotency_key: idempotency_key.to_string(),
      compile_fingerprint: compile_fingerprint.to_string(),
      mode,
      allowed,
      reason,
    }
  }

  /// Deterministic pure compile (no bind/acquire side effects).
  pub fn pure_compile(
    request: &PuncheetahConsentCompilePureRequest,
  ) -> PuncheetahConsentCompileResultEnvelope {
    let compile_fingerprint = compute_compile_fingerprint(request);
    let granted_scope_hash = granted_scope_hash_for_request(request);
    let skeleton = PuncheetahGrantSpecSkeleton {
      template_hash: request.template_hash.clone(),
      policy_hash: request.policy_hash.clone(),
      request_scope_hash: request.request_scope_hash.clone(),
      granted_scope_hash: granted_scope_hash.clone(),
      compile_fingerprint: compile_fingerprint.clone(),
    };

    PuncheetahConsentCompileResultEnvelope {
      schema: PUNCHEETAH_CONSENT_COMPILE_RESULT_SCHEMA.to_string(),
      request_id: request.request_id.clone(),
      idempotency_key: request.idempotency_key.clone(),
      compile_fingerprint,
      template_hash: request.template_hash.clone(),
      policy_hash: request.policy_hash.clone(),
      backend_spec_hash: request.backend_spec_hash.clone(),
      request_scope_hash: request.request_scope_hash.clone(),
      granted_scope_hash,
      compiler_version: request.compiler_version.clone(),
      decision: PuncheetahConsentCompileDecision::CacheMiss,
      reuse_mode: PuncheetahConsentReuseMode::NewGrant,
      grant_spec_skeleton: skeleton,
      out_of_scope_diff: None,
      grant_id: None,
    }
  }

  /// Lookup tiers then compile on miss; never expands scope.
  pub fn compile_with_cache(
    &mut self,
    request: &PuncheetahConsentCompilePureRequest,
  ) -> PuncheetahConsentCompileResultEnvelope {
    let compile_fingerprint = compute_compile_fingerprint(request);
    if let Some(prev) = self.idempotency_fingerprints.get(&request.idempotency_key) {
      if prev != &compile_fingerprint {
        self.stats.divergent_rejections += 1;
        return divergent_forbidden_envelope(request, compile_fingerprint);
      }
    } else {
      self
        .idempotency_fingerprints
        .insert(request.idempotency_key.clone(), compile_fingerprint.clone());
    }

    let primary_key = PuncheetahConsentCompileCacheKey::new(
      compile_fingerprint.clone(),
      request.idempotency_key.clone(),
    );

    if let Some(hit) = self.primary.get(&primary_key).cloned() {
      self.stats.primary_hits += 1;
      self.touch_lru(&primary_key);
      return with_decision(hit, PuncheetahConsentCompileDecision::CacheHitTier1);
    }

    if let Some(session_id) = request.session_id.as_ref() {
      let session_key = SessionSkeletonKey {
        session_id: session_id.clone(),
        compile_fingerprint: compile_fingerprint.clone(),
      };
      if let Some(skeleton) = self.session_skeletons.get(&session_key).cloned() {
        self.stats.tier2_hits += 1;
        let envelope = PuncheetahConsentCompileResultEnvelope::from_skeleton(
          request,
          skeleton,
          PuncheetahConsentCompileDecision::CacheHitTier2,
          PuncheetahConsentReuseMode::SkeletonOnly,
        );
        self.insert_primary(primary_key, envelope.clone());
        return envelope;
      }
    }

    if let Some(template_id) = request.template_id.as_ref() {
      let template_key = TemplateSkeletonKey {
        template_id: template_id.clone(),
        compile_fingerprint: compile_fingerprint.clone(),
      };
      if let Some(skeleton) = self.template_skeletons.get(&template_key).cloned() {
        self.stats.tier3_hits += 1;
        let envelope = PuncheetahConsentCompileResultEnvelope::from_skeleton(
          request,
          skeleton,
          PuncheetahConsentCompileDecision::CacheHitTier3,
          PuncheetahConsentReuseMode::SkeletonOnly,
        );
        self.insert_primary(primary_key, envelope.clone());
        return envelope;
      }
    }

    self.stats.primary_misses += 1;
    let mut envelope = Self::pure_compile(request);
    self.seed_tier_caches(request, &envelope.grant_spec_skeleton);
    self.insert_primary(primary_key, envelope.clone());
    envelope
  }

  fn seed_tier_caches(
    &mut self,
    request: &PuncheetahConsentCompilePureRequest,
    skeleton: &PuncheetahGrantSpecSkeleton,
  ) {
    if let Some(session_id) = request.session_id.as_ref() {
      self.session_skeletons.insert(
        SessionSkeletonKey {
          session_id: session_id.clone(),
          compile_fingerprint: skeleton.compile_fingerprint.clone(),
        },
        skeleton.clone(),
      );
    }
    if let Some(template_id) = request.template_id.as_ref() {
      self.template_skeletons.insert(
        TemplateSkeletonKey {
          template_id: template_id.clone(),
          compile_fingerprint: skeleton.compile_fingerprint.clone(),
        },
        skeleton.clone(),
      );
    }
  }

  fn insert_primary(
    &mut self,
    key: PuncheetahConsentCompileCacheKey,
    envelope: PuncheetahConsentCompileResultEnvelope,
  ) {
    if self.primary.contains_key(&key) {
      self.touch_lru(&key);
      self.primary.insert(key, envelope);
      return;
    }

    while self.primary.len() >= self.capacity {
      if let Some(oldest) = self.lru_order.pop_front() {
        self.primary.remove(&oldest);
        self.stats.evictions += 1;
      } else {
        break;
      }
    }

    self.lru_order.push_back(key.clone());
    self.primary.insert(key, envelope);
    self.stats.inserts += 1;
  }

  fn touch_lru(&mut self, key: &PuncheetahConsentCompileCacheKey) {
    if let Some(pos) = self.lru_order.iter().position(|k| k == key) {
      self.lru_order.remove(pos);
      self.lru_order.push_back(key.clone());
    }
  }
}

impl Default for PuncheetahConsentCompileCache {
  fn default() -> Self {
    Self::with_capacity(DEFAULT_PUNCHEETAH_CONSENT_COMPILE_CACHE_CAPACITY)
  }
}

impl PuncheetahConsentCompileResultEnvelope {
  fn from_skeleton(
    request: &PuncheetahConsentCompilePureRequest,
    skeleton: PuncheetahGrantSpecSkeleton,
    decision: PuncheetahConsentCompileDecision,
    reuse_mode: PuncheetahConsentReuseMode,
  ) -> Self {
    Self {
      schema: PUNCHEETAH_CONSENT_COMPILE_RESULT_SCHEMA.to_string(),
      request_id: request.request_id.clone(),
      idempotency_key: request.idempotency_key.clone(),
      compile_fingerprint: skeleton.compile_fingerprint.clone(),
      template_hash: request.template_hash.clone(),
      policy_hash: request.policy_hash.clone(),
      backend_spec_hash: request.backend_spec_hash.clone(),
      request_scope_hash: request.request_scope_hash.clone(),
      granted_scope_hash: skeleton.granted_scope_hash.clone(),
      compiler_version: request.compiler_version.clone(),
      decision,
      reuse_mode,
      grant_spec_skeleton: skeleton,
      out_of_scope_diff: None,
      grant_id: None,
    }
  }
}

fn with_decision(
  mut envelope: PuncheetahConsentCompileResultEnvelope,
  decision: PuncheetahConsentCompileDecision,
) -> PuncheetahConsentCompileResultEnvelope {
  envelope.decision = decision;
  envelope
}

fn divergent_forbidden_envelope(
  request: &PuncheetahConsentCompilePureRequest,
  compile_fingerprint: String,
) -> PuncheetahConsentCompileResultEnvelope {
  let skeleton = PuncheetahGrantSpecSkeleton {
    template_hash: request.template_hash.clone(),
    policy_hash: request.policy_hash.clone(),
    request_scope_hash: request.request_scope_hash.clone(),
    granted_scope_hash: granted_scope_hash_for_request(request),
    compile_fingerprint: compile_fingerprint.clone(),
  };
  PuncheetahConsentCompileResultEnvelope {
    schema: PUNCHEETAH_CONSENT_COMPILE_RESULT_SCHEMA.to_string(),
    request_id: request.request_id.clone(),
    idempotency_key: request.idempotency_key.clone(),
    compile_fingerprint,
    template_hash: request.template_hash.clone(),
    policy_hash: request.policy_hash.clone(),
    backend_spec_hash: request.backend_spec_hash.clone(),
    request_scope_hash: request.request_scope_hash.clone(),
    granted_scope_hash: skeleton.granted_scope_hash.clone(),
    compiler_version: request.compiler_version.clone(),
    decision: PuncheetahConsentCompileDecision::DivergentRetryForbidden,
    reuse_mode: PuncheetahConsentReuseMode::NewGrant,
    grant_spec_skeleton: skeleton,
    out_of_scope_diff: None,
    grant_id: None,
  }
}

/// `compile_fingerprint = H(template + policy + runtime_policy + backend + scope + version)`
pub fn compute_compile_fingerprint(request: &PuncheetahConsentCompilePureRequest) -> String {
  let mut hasher = Sha256::new();
  hasher.update(request.template_hash.as_bytes());
  hasher.update(b"|");
  hasher.update(request.policy_hash.as_bytes());
  hasher.update(b"|");
  hasher.update(request.runtime_policy_hash.as_bytes());
  hasher.update(b"|");
  hasher.update(request.backend_spec_hash.as_bytes());
  hasher.update(b"|");
  hasher.update(request.request_scope_hash.as_bytes());
  hasher.update(b"|");
  hasher.update(request.compiler_version.as_bytes());
  format!("sha256:{}", hex_encode(&hasher.finalize()))
}

/// Deterministic granted-scope hash: intersection proxy (never widens request scope).
pub fn granted_scope_hash_for_request(request: &PuncheetahConsentCompilePureRequest) -> String {
  let mut hasher = Sha256::new();
  hasher.update(request.template_hash.as_bytes());
  hasher.update(b"&");
  hasher.update(request.policy_hash.as_bytes());
  hasher.update(b"&");
  hasher.update(request.runtime_policy_hash.as_bytes());
  hasher.update(b"&");
  hasher.update(request.backend_spec_hash.as_bytes());
  hasher.update(b"&");
  hasher.update(request.request_scope_hash.as_bytes());
  format!("sha256:{}", hex_encode(&hasher.finalize()))
}

fn hex_encode(bytes: &[u8]) -> String {
  bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn run_i5_3_self_test() -> bool {
  let request = PuncheetahConsentCompilePureRequest {
    request_id: "i5-3-self-test".to_string(),
    idempotency_key: "ik-i5-3-self-test".to_string(),
    template_hash: "sha256:template-i5-3".to_string(),
    policy_hash: "sha256:policy-i5-3".to_string(),
    runtime_policy_hash: "sha256:runtime-i5-3".to_string(),
    backend_spec_hash: "sha256:backend-i5-3".to_string(),
    request_scope_hash: "sha256:scope-i5-3".to_string(),
    compiler_version: "v0.1-tier5-i5.3".to_string(),
    template_id: None,
    session_id: None,
  };
  let first = PuncheetahConsentCompileCache::pure_compile(&request);
  let mut cache = PuncheetahConsentCompileCache::with_capacity(8);
  let miss = cache.compile_with_cache(&request);
  let hit = cache.compile_with_cache(&request);
  first.schema == PUNCHEETAH_CONSENT_COMPILE_RESULT_SCHEMA
    && miss.decision == PuncheetahConsentCompileDecision::CacheMiss
    && hit.decision == PuncheetahConsentCompileDecision::CacheHitTier1
    && first.compile_fingerprint == hit.compile_fingerprint
}

static I5_3_SELF_TEST: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// Cheap cached self-test for tier5 inventory (`i5_3_installed`).
pub fn i5_3_install_verified() -> bool {
  *I5_3_SELF_TEST.get_or_init(run_i5_3_self_test)
}

#[cfg(test)]
mod tests {
  use super::*;

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
  fn compile_fingerprint_is_deterministic() {
    let request = sample_request();
    let a = compute_compile_fingerprint(&request);
    let b = compute_compile_fingerprint(&request);
    assert_eq!(a, b);
    assert!(a.starts_with("sha256:"));
  }

  #[test]
  fn compile_fingerprint_changes_when_inputs_change() {
    let mut request = sample_request();
    let baseline = compute_compile_fingerprint(&request);
    request.compiler_version = "v0.1.37".to_string();
    assert_ne!(baseline, compute_compile_fingerprint(&request));
  }

  #[test]
  fn pure_compile_is_replay_identical() {
    let request = sample_request();
    let first = PuncheetahConsentCompileCache::pure_compile(&request);
    let second = PuncheetahConsentCompileCache::pure_compile(&request);
    assert_eq!(first, second);
    assert_eq!(first.decision, PuncheetahConsentCompileDecision::CacheMiss);
  }

  #[test]
  fn primary_cache_hit_on_same_fingerprint_and_idempotency() {
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
  fn bounded_cache_evicts_oldest_entry() {
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
  fn divergent_retry_forbidden_for_same_idempotency_different_fingerprint() {
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
  fn join_rule_allows_matching_inflight_binding() {
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
    assert_eq!(rule.mode, PuncheetahConsentJoinRetryMode::Join);
  }

  #[test]
  fn retry_rule_allows_same_fingerprint_new_grant() {
    let request = sample_request();
    let fingerprint = compute_compile_fingerprint(&request);
    let mut cache = PuncheetahConsentCompileCache::with_capacity(8);
    cache
      .idempotency_fingerprints
      .insert(request.idempotency_key.clone(), fingerprint.clone());

    let rule = cache.evaluate_join_retry_rule(
      &request.idempotency_key,
      &fingerprint,
      PuncheetahConsentJoinRetryMode::Retry,
    );
    assert!(rule.allowed);
    assert_eq!(rule.mode, PuncheetahConsentJoinRetryMode::Retry);
  }

  #[test]
  fn session_tier_reuses_skeleton_without_primary_entry() {
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
  fn i5_3_install_self_test_passes() {
    assert!(run_i5_3_self_test());
    assert!(i5_3_install_verified());
  }
}
