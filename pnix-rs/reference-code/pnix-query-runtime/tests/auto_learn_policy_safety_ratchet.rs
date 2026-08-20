//! ratchet: `auto-learn-policy.px` 의 **자동 학습 safety envelope** 를 lock.
//!
//! intelligence.md P0 (auto-learn closure) 의 런타임 헌법:
//!   - 모든 자동 학습 fact 는 `Candidate` 로 시작 (Accepted 직행 금지)
//!   - Candidate 는 다른 fact 의 검증 근거가 될 수 없다 (순환 방지)
//!   - 독립 소스 ≥2 + 참조 ≥3 이어야 Accepted 자동 승격
//!   - Medicine / Law / Finance context 는 require-human-review 필수
//!   - math / physics / tool 검색 소스 가 비면 학습 경로 자체가 단절됨
//!   - tool-verification 은 Math / Physics / Code.Python 을 반드시 커버
//!   - proactive 학습 trigger 는 합리적 범위 (threshold ≥1, window ≥1 일)
//!
//! `auto_learn.rs::analyze_held_facts` / `promote_if_cross_verified` 의
//! safety invariant 가 이 파일에 박혀 있다. 실수로 `initial-status` 를
//! `"Accepted"` 로 바꾸거나 Medicine 을 `require-human-review` 에서 빼면
//! 자동 학습이 안전 경계를 넘는다. 이 ratchet 이 정적으로 차단한다.
//!
//! `data_px_mirror_drift` 가 byte-identical 이므로 이 검증은 자동으로
//! kernel 외부의 mirrored policy data 에도 동일하게 적용된다.

use pnix_query_runtime::px::{parse_px_file, PxValue};
use std::path::PathBuf;

fn data_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data")
}

fn load() -> PxValue {
  let path = data_dir().join("auto-learn-policy.px");
  parse_px_file(&path).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn scalar(root: &PxValue, key: &str) -> String {
  let attrset = root
    .as_attrset()
    .unwrap_or_else(|| panic!("root must be attrset"));
  let value = attrset
    .get(key)
    .unwrap_or_else(|| panic!("missing key `{key}`"));
  value
    .as_str()
    .unwrap_or_else(|| panic!("key `{key}` must be string, got {:?}", value))
    .to_string()
}

fn string_list(root: &PxValue, key: &str) -> Vec<String> {
  let attrset = root
    .as_attrset()
    .unwrap_or_else(|| panic!("root must be attrset"));
  let value = attrset
    .get(key)
    .unwrap_or_else(|| panic!("missing key `{key}`"));
  match value {
    PxValue::List(items) => items
      .iter()
      .filter_map(|v| v.as_str().map(String::from))
      .collect(),
    other => panic!("key `{key}` must be list of strings, got {:?}", other),
  }
}

fn attrset_list<'a>(root: &'a PxValue, key: &str) -> &'a [PxValue] {
  let attrset = root
    .as_attrset()
    .unwrap_or_else(|| panic!("root must be attrset"));
  let value = attrset
    .get(key)
    .unwrap_or_else(|| panic!("missing key `{key}`"));
  match value {
    PxValue::List(items) => items.as_slice(),
    other => panic!("key `{key}` must be list, got {:?}", other),
  }
}

fn inner_scalar(item: &PxValue, key: &str) -> Option<String> {
  item.as_attrset()?.get(key)?.as_str().map(String::from)
}

fn parse_u32(s: &str, key: &str) -> u32 {
  s.parse::<u32>()
    .unwrap_or_else(|_| panic!("`{key}` must parse as u32, got `{s}`"))
}

#[test]
fn initial_status_must_be_candidate() {
  // 안전 헌법 1: 자동 학습 fact 는 Accepted 로 직행할 수 없다.
  let px = load();
  assert_eq!(
    scalar(&px, "initial-status"),
    "Candidate",
    "`initial-status` 를 `Candidate` 가 아닌 값으로 바꾸면 \
         자동 학습이 human review / cross-verification 경로를 우회한다"
  );
}

#[test]
fn verification_must_reference_accepted_only() {
  // 안전 헌법 2: Candidate 가 다른 Candidate 의 검증 근거가 되면
  // 자기참조 루프가 생긴다.
  let px = load();
  assert_eq!(
    scalar(&px, "verification-only-accepted"),
    "true",
    "`verification-only-accepted=true` 가 풀리면 Candidate → Candidate \
         순환 검증으로 근거 없는 Accepted 승격이 가능해진다"
  );
}

#[test]
fn promotion_thresholds_respect_intelligence_p0_floor() {
  // intelligence.md P0: "독립 소스 2개 + 참조 3회" 가 최소 승격 조건.
  // 더 엄격하게(2→3, 3→5) 가는 것은 허용, 느슨하게(2→1, 3→2) 가면 fail.
  let px = load();
  let independent = parse_u32(
    &scalar(&px, "auto-promote-independent-sources"),
    "auto-promote-independent-sources",
  );
  assert!(
        independent >= 2,
        "auto-promote-independent-sources 는 최소 2 이상 (intelligence.md P0 safety floor), got {independent}"
    );
  let references = parse_u32(
    &scalar(&px, "auto-promote-after-references"),
    "auto-promote-after-references",
  );
  assert!(
        references >= 3,
        "auto-promote-after-references 는 최소 3 이상 (intelligence.md P0 safety floor), got {references}"
    );
}

#[test]
fn high_stakes_contexts_must_require_human_review() {
  // 안전 헌법 3: 의료 / 법률 / 금융 context 는 자동 학습 결과를
  // 사람 승인 없이 Accepted 로 올릴 수 없다.
  let px = load();
  let review = string_list(&px, "require-human-review");
  for required in &["Medicine.*", "Law.*", "Finance.*"] {
    assert!(
      review.iter().any(|c| c == required),
      "require-human-review 에 `{required}` 가 없으면 고위험 도메인이 \
             자동 학습 경로로 Accepted 된다. 현재: {:?}",
      review
    );
  }
}

#[test]
fn search_sources_must_not_be_empty() {
  // 학습 경로가 끊기면 auto-learn 루프 자체가 성립하지 않는다.
  let px = load();
  for key in &["math-sources", "physics-sources", "tool-sources"] {
    let items = attrset_list(&px, key);
    assert!(
      !items.is_empty(),
      "`{key}` 가 비어 있으면 해당 도메인 자동 학습 경로가 단절된다"
    );
  }
}

#[test]
fn tool_verification_covers_math_physics_python() {
  // 학습한 공식 / 코드는 tool-verification 을 거쳐야 실행 검증된다.
  // Math / Physics / Code.Python 은 P0 의 초기 verification 타겟.
  let px = load();
  let items = attrset_list(&px, "tool-verification");
  let patterns: Vec<String> = items
    .iter()
    .filter_map(|item| inner_scalar(item, "context-match"))
    .collect();
  for required in &["Math.*", "Physics.*", "Code.Python.*"] {
    assert!(
      patterns.iter().any(|p| p == required),
      "tool-verification 에 `{required}` 가 없으면 해당 도메인 학습 결과가 \
             실행 검증 없이 Accepted 될 수 있다. 현재 patterns: {:?}",
      patterns
    );
  }
}

#[test]
fn proactive_learning_config_is_coherent() {
  // 선제 학습 trigger 가 합리적 범위 안에 있어야 한다.
  // 0 이나 과도하게 크면 과학습 / 과소학습 trigger.
  let px = load();
  assert_eq!(
    scalar(&px, "proactive-learning"),
    "true",
    "proactive-learning 이 꺼지면 held 가 쌓여도 선제 학습이 안 일어난다"
  );
  let threshold = parse_u32(
    &scalar(&px, "proactive-held-threshold"),
    "proactive-held-threshold",
  );
  assert!(
    (1..=100).contains(&threshold),
    "proactive-held-threshold 는 1..=100 범위여야 한다, got {threshold}"
  );
  let window = parse_u32(
    &scalar(&px, "proactive-window-days"),
    "proactive-window-days",
  );
  assert!(
    (1..=365).contains(&window),
    "proactive-window-days 는 1..=365 범위여야 한다, got {window}"
  );
}

#[test]
fn allowed_and_blocked_contexts_present() {
  // auto-learn 이 어떤 context 에 살릴지 / 막을지 가 .px owner 에 있어야 한다.
  let px = load();
  let allowed = string_list(&px, "allowed-contexts");
  assert!(
    !allowed.is_empty(),
    "allowed-contexts 가 비면 auto-learn 이 아무 도메인도 못 한다"
  );
  let blocked = string_list(&px, "blocked-contexts");
  assert!(
    !blocked.is_empty(),
    "blocked-contexts 가 비면 고위험 영역 차단이 없다"
  );
}
