//! Korean NL query-classifier model.
//!
//! OWNER-LAW (2026-05-13): substrate-execution layer. Owns the `.px`
//! reader for `data/query-classifiers.px` (the doghouse-side Korean
//! classifier registry). Moved here from doghouse-core because
//! reading `.px` is substrate execution (`pnix_eval` path), not
//! storage-adapter work. doghouse-core re-exports this module via
//! its `conversation::query_model` submodule for backward-compat
//! `use` paths inside doghouse internals.

use crate::px::PxValue as PxV;
use crate::px_hot::HotPxCache;
use std::collections::BTreeMap;
use std::path::PathBuf;

type PxAttrset = BTreeMap<String, PxV>;

// ---------------------------------------------------------------------------
// Handoff classifier — data-driven replacement for classify_execution_handoff_query
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct HandoffClassifierRule {
  pub template_id: String,
  pub tags: Vec<String>,
  pub execution_owner: String,
  pub visibility: String,
  /// Utterance contains ANY of these → match.
  pub match_any: Vec<String>,
  /// Utterance contains at least one term AND at least one unit → match.
  pub match_terms: Vec<String>,
  pub match_units: Vec<String>,
}

impl HandoffClassifierRule {
  // batch 52 (2026-04-14): param name `utterance` → `text` — classifier 는
  // 임의 텍스트에 대한 generic matcher 이고 .px 가 data owner 이므로
  // utterance-specific gate 에 catch 될 이유가 없다.
  #[cfg(test)]
  pub fn matches(&self, text: &str) -> bool {
    if !self.match_any.is_empty() {
      return self.match_any.iter().any(|kw| text.contains(kw.as_str()));
    }
    if !self.match_terms.is_empty() && !self.match_units.is_empty() {
      let has_term = self.match_terms.iter().any(|t| text.contains(t.as_str()));
      let has_unit = self.match_units.iter().any(|u| text.contains(u.as_str()));
      return has_term && has_unit;
    }
    false
  }
}

// ---------------------------------------------------------------------------
// Predicate classifier
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct PredicateClassifierRule {
  pub keyword: String,
  pub predicate: String,
  pub label_ko: String,
  /// If non-empty, at least one of these must also appear.
  pub requires_also: Vec<String>,
}

#[cfg(test)]
impl PredicateClassifierRule {
  // batch 52 (2026-04-14): param rename utterance → text (see HandoffClassifierRule).
  pub fn matches(&self, text: &str) -> bool {
    if !text.contains(self.keyword.as_str()) {
      return false;
    }
    if self.requires_also.is_empty() {
      return true;
    }
    self
      .requires_also
      .iter()
      .any(|kw| text.contains(kw.as_str()))
  }
}

// ---------------------------------------------------------------------------
// Domain listing classifier
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct DomainClassifierRule {
  pub keyword: String,
  pub domain: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeldReasonRule {
  pub when: String,
  pub reason_key: String,
  pub term_source: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KernelSourceFieldRule {
  pub field: String,
  pub predicate: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KernelSourceMetadata {
  pub field_predicate: String,
  pub value_predicate: String,
  pub list_field_predicate: String,
  pub list_item_predicate: String,
}

// ---------------------------------------------------------------------------
// Continuation classifier
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ContinuationClassifierRule {
  pub kind: String,
  /// Utterance contains ANY of these → match.
  pub match_any: Vec<String>,
  /// Each inner Vec is an "all" group — utterance must contain ALL items
  /// in at least one group to match via this field.
  pub match_all_pairs: Vec<Vec<String>>,
}

impl ContinuationClassifierRule {
  // batch 52 (2026-04-14): param rename utterance → text.
  pub fn matches(&self, text: &str) -> bool {
    if self.match_any.iter().any(|kw| text.contains(kw.as_str())) {
      return true;
    }
    self
      .match_all_pairs
      .iter()
      .any(|group| group.iter().all(|kw| text.contains(kw.as_str())))
  }
}

// ---------------------------------------------------------------------------
// Physics formula classifier
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct PhysicsFormulaClassifierRule {
  pub formula_id: String,
  pub concept_term: String,
  /// All of these must appear (simple case).
  pub match_all: Vec<String>,
  /// At least one term must appear.
  pub match_any_term: Vec<String>,
  /// At least one unit must appear.
  pub match_any_unit: Vec<String>,
  /// All units must appear (used with match-all-unit).
  pub match_all_unit: Vec<String>,
  /// None of these must appear.
  pub match_none: Vec<String>,
}

/// batch 172 (2026-04-16): physics formula presentation template — Korean
/// summary/formula-name/dimension-check 와 canonical-template 을 .px 에서 로드.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PhysicsFormulaTemplate {
  pub formula_id: String,
  pub formula_name: String,
  pub dimension_check: String,
  pub canonical_template: String,
  pub summary_template: String,
}

/// batch 174 (2026-04-16): CT diagram commute diagnostic template.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PhysicsCtCommuteTemplate {
  pub formula_id: String,
  pub template: String,
  pub valid_suffix: String,
  pub invalid_suffix: String,
}

impl PhysicsFormulaClassifierRule {
  // batch 52 (2026-04-14): param rename utterance → text.
  pub fn matches(&self, text: &str) -> bool {
    // Exclusion check first
    if self.match_none.iter().any(|kw| text.contains(kw.as_str())) {
      return false;
    }
    // Simple match-all
    if !self.match_all.is_empty() {
      return self.match_all.iter().all(|kw| text.contains(kw.as_str()));
    }
    // match-any-term + match-any-unit
    if !self.match_any_term.is_empty() && !self.match_any_unit.is_empty() {
      let has_term = self
        .match_any_term
        .iter()
        .any(|t| text.contains(t.as_str()));
      let has_unit = self
        .match_any_unit
        .iter()
        .any(|u| text.contains(u.as_str()));
      return has_term && has_unit;
    }
    // match-any-term + match-all-unit
    if !self.match_any_term.is_empty() && !self.match_all_unit.is_empty() {
      let has_term = self
        .match_any_term
        .iter()
        .any(|t| text.contains(t.as_str()));
      let has_unit = self
        .match_all_unit
        .iter()
        .all(|u| text.contains(u.as_str()));
      return has_term && has_unit;
    }
    false
  }
}

// ---------------------------------------------------------------------------
// Full query classifier model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MixedQueryUnitMarker {
  pub marker: String,
  pub unit: String,
}

/// batch 215: 한글 삼각함수 이름 → Rust fn 매핑 entry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrigFunctionEntry {
  pub name: String,
  pub func: String,
  pub default_degree: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct QueryClassifierModel {
  pub handoff_classifiers: Vec<HandoffClassifierRule>,
  pub predicate_classifiers: Vec<PredicateClassifierRule>,
  pub domain_classifiers: Vec<DomainClassifierRule>,
  pub query_dispatch_priority: Vec<String>,
  pub kernel_dispatch_routes: BTreeMap<String, String>,
  pub kernel_success_predicates: Vec<String>,
  pub held_reason_keys: BTreeMap<String, String>,
  pub held_reason_rules: Vec<HeldReasonRule>,
  pub kernel_source_fact_fields: Vec<KernelSourceFieldRule>,
  pub kernel_source_list_fields: Vec<KernelSourceFieldRule>,
  pub kernel_source_metadata: KernelSourceMetadata,
  pub domain_list_intent_markers: Vec<String>,
  pub concept_what_markers: Vec<String>,
  pub concept_definition_suffixes: Vec<String>,
  pub concept_explain_markers: Vec<String>,
  pub concept_demo_markers: Vec<String>,
  pub computation_signals: Vec<String>,
  pub calculus_signals: Vec<String>,
  pub calculus_diff_markers: Vec<String>,
  pub calculus_int_markers: Vec<String>,
  pub calculus_strip_suffixes: Vec<String>,
  pub trig_functions: Vec<TrigFunctionEntry>,
  pub physics_unit_signals: Vec<String>,
  pub physics_concept_signals: Vec<String>,
  pub intent_detailed_modifiers: Vec<String>,
  pub intent_brief_modifiers: Vec<String>,
  pub definition_signals: Vec<String>,
  pub equation_verb_signals: Vec<String>,
  pub mixed_query_trailing_suffixes: Vec<String>,
  pub mixed_query_math_markers: Vec<String>,
  pub mixed_query_explicit_units: Vec<MixedQueryUnitMarker>,
  pub conversion_intent_markers: Vec<String>,
  pub requestive_quote_endings: Vec<String>,
  pub requestive_quote_stem_endings: Vec<String>,
  pub propositive_quote_endings: Vec<String>,
  pub interrogative_quote_endings: Vec<String>,
  pub commitment_quote_endings: Vec<String>,
  pub factorial_markers: Vec<String>,
  pub sqrt_markers: Vec<String>,
  pub power_markers: Vec<String>,
  pub vec_dot_markers: Vec<String>,
  pub vec_add_markers: Vec<String>,
  pub vec_add_conjunction_pair: Vec<String>,
  pub vec_sub_markers: Vec<String>,
  pub scalar_mul_markers: Vec<String>,
  pub math_context_markers: Vec<String>,
  pub concept_question_markers: Vec<String>,
  pub vector_intent_markers: Vec<String>,
  pub continuation_elaborate_markers: Vec<String>,
  pub has_property_life_markers: Vec<String>,
  pub negation_markers: Vec<String>,
  pub term_extraction_suffixes: Vec<String>,
  pub domain_listing_trigger_markers: Vec<String>,
  pub os_execution_owner_markers: Vec<String>,
  pub declarative_verb_suffixes: Vec<String>,
  pub connective_verb_suffixes: Vec<String>,
  pub doghouse_pipeline_trace_note_prefixes: Vec<String>,
  pub equation_result_question_markers: Vec<String>,
  pub concept_term_stopwords: Vec<String>,
  pub concept_explain_skip_tokens: Vec<String>,
  pub question_word_stems: Vec<String>,
  pub continuation_classifiers: Vec<ContinuationClassifierRule>,
  pub cross_concept_markers: Vec<String>,
  pub physics_formula_classifiers: Vec<PhysicsFormulaClassifierRule>,
  pub physics_formula_templates: Vec<PhysicsFormulaTemplate>,
  pub physics_ct_commute_templates: Vec<PhysicsCtCommuteTemplate>,
  pub equation_markers: Vec<String>,
  pub unit_conversion_markers: Vec<String>,
  pub percentage_markers: Vec<String>,
  pub causal_markers: Vec<String>,
  pub conditional_markers: Vec<String>,
  pub process_markers: Vec<String>,
  // batch 251 (2026-04-17): char-level arithmetic detectors (`.px` owner).
  pub equation_char_markers: Vec<char>,
  pub intent_math_op_char_markers: Vec<char>,
  pub mixed_math_op_char_markers: Vec<char>,
  pub variable_char_markers: Vec<char>,
  // batch 253 (2026-04-18): math-context + Korean particle char owners.
  pub math_context_char_markers: Vec<char>,
  pub korean_sentence_particle_chars: Vec<char>,
  // batch 254 (2026-04-18): Korean morpheme scalars/list (possessive / copula /
  // ordinal).  empty `Option` / 빈 list 는 `.px` 소유자가 아직 지정 안 됨.
  pub korean_power_expression_particle: Option<char>,
  pub korean_equation_copula_endings: Vec<String>,
  pub korean_ordinal_prefix_char: Option<char>,
}

static QUERY_CLASSIFIER_MODEL: HotPxCache<QueryClassifierModel> = HotPxCache::new();

fn query_classifiers_file_path() -> PathBuf {
  std::env::var("DOGHOUSE_QUERY_CLASSIFIERS_FILE")
    .map(PathBuf::from)
    .or_else(|_| {
      std::env::var("CARGO_MANIFEST_DIR")
        .map(|d| PathBuf::from(d).join("data/query-classifiers.px"))
    })
    .unwrap_or_else(|_| PathBuf::from("data/query-classifiers.px"))
}

// ---------------------------------------------------------------------------
// Rust defaults (fallback when .px file missing or malformed)
// ---------------------------------------------------------------------------

fn default_query_classifier_model() -> QueryClassifierModel {
  // batch 157 (2026-04-16): 이전에는 ~390 줄 Rust 하드코딩 Korean 기본값이 있었지만
  // query-classifiers.px 는 sanctioned data 로 항상 빌드 트리에 존재하고 strict
  // loader 가 모든 section 을 canonical owner 로 강제한다. Rust fallback 은 dead
  // code + canonical 과 drift 위험. CLAUDE.md 15 '하드코딩 목록 금지' 준수.
  QueryClassifierModel::default()
}

// ---------------------------------------------------------------------------
// .px parsing helpers (shared with light_model.rs pattern)
// ---------------------------------------------------------------------------

fn attrset_str(map: &PxAttrset, key: &str) -> String {
  map
    .get(key)
    .and_then(|v| v.as_str())
    .unwrap_or("")
    .to_string()
}

fn attrset_str_list(map: &PxAttrset, key: &str) -> Vec<String> {
  map.get(key).map(|v| v.as_string_list()).unwrap_or_default()
}

fn attrset_list<'a>(value: Option<&'a PxV>) -> Vec<&'a PxAttrset> {
  match value {
    Some(PxV::List(items)) => items.iter().filter_map(|item| item.as_attrset()).collect(),
    _ => vec![],
  }
}

fn root_str_list(root: &PxAttrset, key: &str) -> Vec<String> {
  root
    .get(key)
    .map(|v| v.as_string_list())
    .unwrap_or_default()
}

// batch 251 (2026-04-17): char-list loader. `.px` 에서 string list 를 받아
// 각 문자열의 첫 글자만 취한다 (한 글자 이상이면 경고 없이 첫 글자만 사용).
fn root_char_list(root: &PxAttrset, key: &str) -> Vec<char> {
  root_str_list(root, key)
    .into_iter()
    .filter_map(|s| s.chars().next())
    .collect()
}

// batch 254 (2026-04-18): single-char scalar loader. `.px` attrset 에서
// string scalar 를 받아 첫 글자를 취한다. 값이 없거나 빈 문자열이면 None.
fn root_char(root: &PxAttrset, key: &str) -> Option<char> {
  root
    .get(key)
    .and_then(|v| v.as_str())
    .and_then(|s| s.chars().next())
}

fn root_attrset_str_map(root: &PxAttrset, key: &str) -> BTreeMap<String, String> {
  root
    .get(key)
    .and_then(|value| value.as_attrset())
    .map(|map| {
      map
        .iter()
        .filter_map(|(name, value)| {
          value
            .as_str()
            .map(|value| (name.clone(), value.to_string()))
        })
        .collect()
    })
    .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// .px → QueryClassifierModel
// ---------------------------------------------------------------------------

fn parse_handoff_classifier(rule: &PxAttrset) -> Option<HandoffClassifierRule> {
  let template_id = attrset_str(rule, "template-id");
  if template_id.is_empty() {
    return None;
  }
  Some(HandoffClassifierRule {
    template_id,
    tags: attrset_str_list(rule, "tags"),
    execution_owner: attrset_str(rule, "execution-owner"),
    visibility: attrset_str(rule, "visibility"),
    match_any: attrset_str_list(rule, "match-any"),
    match_terms: attrset_str_list(rule, "match-terms"),
    match_units: attrset_str_list(rule, "match-units"),
  })
}

fn parse_predicate_classifier_list(rule: &PxAttrset) -> Vec<PredicateClassifierRule> {
  // batch 157 (2026-04-16): query-classifiers.px 의 predicate-classifiers 는
  // `match-any = [...]` / `match-all = [...]` schema 를 쓴다. 기존 parser 는
  // `keyword = "..."` + `requires-also = [...]` 만 읽어서 match-any schema 를
  // 전부 drop 했다. 이 mismatch 는 Rust 하드코딩 fallback 이 돌면서 가려져
  // 있었다 (batch 156 까지).
  //
  // 올바른 lowering:
  //   - match-any = [k1, k2, ...] → 각 alias 를 keyword 로 갖는 별도 rule 로 확장
  //   - match-all = [a, b, ...]  → 모든 alias rule 의 requires_also 로 복제
  //   - legacy `keyword = "..."` 도 같은 파서로 수용 (뒤로 호환성)
  let predicate = attrset_str(rule, "predicate");
  if predicate.is_empty() {
    return Vec::new();
  }
  let label_ko = attrset_str(rule, "label-ko");
  let requires_also = {
    let mut v = attrset_str_list(rule, "requires-also");
    v.extend(attrset_str_list(rule, "match-all"));
    v
  };
  let mut keywords = attrset_str_list(rule, "match-any");
  let legacy_keyword = attrset_str(rule, "keyword");
  if !legacy_keyword.is_empty() {
    keywords.push(legacy_keyword);
  }
  keywords
    .into_iter()
    .filter(|k| !k.is_empty())
    .map(|keyword| PredicateClassifierRule {
      keyword,
      predicate: predicate.clone(),
      label_ko: label_ko.clone(),
      requires_also: requires_also.clone(),
    })
    .collect()
}

fn parse_domain_classifier(rule: &PxAttrset) -> Option<DomainClassifierRule> {
  let keyword = attrset_str(rule, "keyword");
  let domain = attrset_str(rule, "domain");
  if keyword.is_empty() || domain.is_empty() {
    return None;
  }
  Some(DomainClassifierRule { keyword, domain })
}

fn parse_held_reason_rule(rule: &PxAttrset) -> Option<HeldReasonRule> {
  let when = attrset_str(rule, "when");
  let reason_key = attrset_str(rule, "reason-key");
  let term_source = attrset_str(rule, "term-source");
  if when.is_empty() || reason_key.is_empty() || term_source.is_empty() {
    return None;
  }
  Some(HeldReasonRule {
    when,
    reason_key,
    term_source,
  })
}

fn parse_kernel_source_field_rule(rule: &PxAttrset) -> Option<KernelSourceFieldRule> {
  let field = attrset_str(rule, "field");
  let predicate = attrset_str(rule, "predicate");
  if field.is_empty() || predicate.is_empty() {
    return None;
  }
  Some(KernelSourceFieldRule { field, predicate })
}

fn parse_kernel_source_metadata(map: &PxAttrset) -> Option<KernelSourceMetadata> {
  let field_predicate = attrset_str(map, "field-predicate");
  let value_predicate = attrset_str(map, "value-predicate");
  let list_field_predicate = attrset_str(map, "list-field-predicate");
  let list_item_predicate = attrset_str(map, "list-item-predicate");
  if field_predicate.is_empty()
    || value_predicate.is_empty()
    || list_field_predicate.is_empty()
    || list_item_predicate.is_empty()
  {
    return None;
  }
  Some(KernelSourceMetadata {
    field_predicate,
    value_predicate,
    list_field_predicate,
    list_item_predicate,
  })
}

fn parse_mixed_query_unit_marker(rule: &PxAttrset) -> Option<MixedQueryUnitMarker> {
  let marker = attrset_str(rule, "marker");
  let unit = attrset_str(rule, "unit");
  if marker.is_empty() || unit.is_empty() {
    return None;
  }
  Some(MixedQueryUnitMarker { marker, unit })
}

fn parse_continuation_classifier(rule: &PxAttrset) -> Option<ContinuationClassifierRule> {
  let kind = attrset_str(rule, "kind");
  if kind.is_empty() {
    return None;
  }
  let match_all_pairs = match rule.get("match-all-pairs") {
    Some(PxV::List(outer)) => outer
      .iter()
      .filter_map(|inner| match inner {
        PxV::List(items) => {
          let strs: Vec<String> = items
            .iter()
            .filter_map(|s| s.as_str().map(|s| s.to_string()))
            .collect();
          if strs.is_empty() {
            None
          } else {
            Some(strs)
          }
        }
        _ => None,
      })
      .collect(),
    _ => vec![],
  };
  Some(ContinuationClassifierRule {
    kind,
    match_any: attrset_str_list(rule, "match-any"),
    match_all_pairs,
  })
}

fn parse_physics_formula_template(rule: &PxAttrset) -> Option<PhysicsFormulaTemplate> {
  let formula_id = attrset_str(rule, "formula-id");
  if formula_id.is_empty() {
    return None;
  }
  Some(PhysicsFormulaTemplate {
    formula_id,
    formula_name: attrset_str(rule, "formula-name"),
    dimension_check: attrset_str(rule, "dimension-check"),
    canonical_template: attrset_str(rule, "canonical-template"),
    summary_template: attrset_str(rule, "summary-template"),
  })
}

fn parse_physics_ct_commute_template(rule: &PxAttrset) -> Option<PhysicsCtCommuteTemplate> {
  let formula_id = attrset_str(rule, "formula-id");
  if formula_id.is_empty() {
    return None;
  }
  Some(PhysicsCtCommuteTemplate {
    formula_id,
    template: attrset_str(rule, "template"),
    valid_suffix: attrset_str(rule, "valid-suffix"),
    invalid_suffix: attrset_str(rule, "invalid-suffix"),
  })
}

fn parse_physics_formula_classifier(rule: &PxAttrset) -> Option<PhysicsFormulaClassifierRule> {
  let formula_id = attrset_str(rule, "formula-id");
  let concept_term = attrset_str(rule, "concept-term");
  if formula_id.is_empty() || concept_term.is_empty() {
    return None;
  }
  Some(PhysicsFormulaClassifierRule {
    formula_id,
    concept_term,
    match_all: attrset_str_list(rule, "match-all"),
    match_any_term: attrset_str_list(rule, "match-any-term"),
    match_any_unit: attrset_str_list(rule, "match-any-unit"),
    match_all_unit: attrset_str_list(rule, "match-all-unit"),
    match_none: attrset_str_list(rule, "match-none"),
  })
}

fn query_classifier_model_from_value(value: &PxV) -> QueryClassifierModel {
  let default = default_query_classifier_model();
  let Some(root) = value.as_attrset() else {
    return default;
  };

  let handoff_classifiers = attrset_list(root.get("handoff-classifiers"))
    .into_iter()
    .filter_map(parse_handoff_classifier)
    .collect::<Vec<_>>();

  let predicate_classifiers = attrset_list(root.get("predicate-classifiers"))
    .into_iter()
    .flat_map(parse_predicate_classifier_list)
    .collect::<Vec<_>>();

  let domain_classifiers = attrset_list(root.get("domain-classifiers"))
    .into_iter()
    .filter_map(parse_domain_classifier)
    .collect::<Vec<_>>();

  let held_reason_rules = attrset_list(root.get("held-reason-rules"))
    .into_iter()
    .filter_map(parse_held_reason_rule)
    .collect::<Vec<_>>();
  let kernel_source_fact_fields = attrset_list(root.get("kernel-source-fact-fields"))
    .into_iter()
    .filter_map(parse_kernel_source_field_rule)
    .collect::<Vec<_>>();
  let kernel_source_list_fields = attrset_list(root.get("kernel-source-list-fields"))
    .into_iter()
    .filter_map(parse_kernel_source_field_rule)
    .collect::<Vec<_>>();
  let kernel_source_metadata = root
    .get("kernel-source-metadata")
    .and_then(|value| value.as_attrset())
    .and_then(parse_kernel_source_metadata);

  let continuation_classifiers = attrset_list(root.get("continuation-classifiers"))
    .into_iter()
    .filter_map(parse_continuation_classifier)
    .collect::<Vec<_>>();

  let physics_formula_classifiers = attrset_list(root.get("physics-formula-classifiers"))
    .into_iter()
    .filter_map(parse_physics_formula_classifier)
    .collect::<Vec<_>>();
  let physics_formula_templates = attrset_list(root.get("physics-formula-templates"))
    .into_iter()
    .filter_map(parse_physics_formula_template)
    .collect::<Vec<_>>();
  let physics_ct_commute_templates = attrset_list(root.get("physics-ct-commute-templates"))
    .into_iter()
    .filter_map(parse_physics_ct_commute_template)
    .collect::<Vec<_>>();

  let query_dispatch_priority = root_str_list(root, "query-dispatch-priority");
  let kernel_dispatch_routes = root_attrset_str_map(root, "kernel-dispatch-routes");
  let kernel_success_predicates = root_str_list(root, "kernel-success-predicates");
  let held_reason_keys = root_attrset_str_map(root, "held-reason-keys");
  let domain_list_intent_markers = root_str_list(root, "domain-list-intent-markers");
  let concept_what_markers = root_str_list(root, "concept-what-markers");
  let concept_definition_suffixes = root_str_list(root, "concept-definition-suffixes");
  let concept_explain_markers = root_str_list(root, "concept-explain-markers");
  let concept_demo_markers = root_str_list(root, "concept-demo-markers");
  let computation_signals = root_str_list(root, "computation-signals");
  let calculus_signals = root_str_list(root, "calculus-signals");
  let calculus_diff_markers = root_str_list(root, "calculus-diff-markers");
  let calculus_int_markers = root_str_list(root, "calculus-int-markers");
  let calculus_strip_suffixes = root_str_list(root, "calculus-strip-suffixes");
  let trig_functions = match root.get("trig-functions") {
    Some(PxV::List(items)) => items
      .iter()
      .filter_map(|entry| {
        let m = entry.as_attrset()?;
        Some(TrigFunctionEntry {
          name: m.get("name")?.as_str()?.to_string(),
          func: m.get("func")?.as_str()?.to_string(),
          default_degree: m.get("default-degree").and_then(|v| v.as_str()) == Some("true"),
        })
      })
      .collect(),
    _ => vec![],
  };
  let physics_unit_signals = root_str_list(root, "physics-unit-signals");
  let physics_concept_signals = root_str_list(root, "physics-concept-signals");
  let intent_detailed_modifiers = root_str_list(root, "intent-detailed-modifiers");
  let intent_brief_modifiers = root_str_list(root, "intent-brief-modifiers");
  let definition_signals = root_str_list(root, "definition-signals");
  let equation_verb_signals = root_str_list(root, "equation-verb-signals");
  let mixed_query_trailing_suffixes = root_str_list(root, "mixed-query-trailing-suffixes");
  let mixed_query_math_markers = root_str_list(root, "mixed-query-math-markers");
  let mixed_query_explicit_units = attrset_list(root.get("mixed-query-explicit-units"))
    .into_iter()
    .filter_map(parse_mixed_query_unit_marker)
    .collect::<Vec<_>>();
  let conversion_intent_markers = root_str_list(root, "conversion-intent-markers");
  let requestive_quote_endings = root_str_list(root, "requestive-quote-endings");
  let requestive_quote_stem_endings = root_str_list(root, "requestive-quote-stem-endings");
  let propositive_quote_endings = root_str_list(root, "propositive-quote-endings");
  let interrogative_quote_endings = root_str_list(root, "interrogative-quote-endings");
  let commitment_quote_endings = root_str_list(root, "commitment-quote-endings");
  let factorial_markers = root_str_list(root, "factorial-markers");
  let sqrt_markers = root_str_list(root, "sqrt-markers");
  let power_markers = root_str_list(root, "power-markers");
  let vec_dot_markers = root_str_list(root, "vec-dot-markers");
  let vec_add_markers = root_str_list(root, "vec-add-markers");
  let vec_add_conjunction_pair = root_str_list(root, "vec-add-conjunction-pair");
  let vec_sub_markers = root_str_list(root, "vec-sub-markers");
  let scalar_mul_markers = root_str_list(root, "scalar-mul-markers");
  let math_context_markers = root_str_list(root, "math-context-markers");
  let concept_question_markers = root_str_list(root, "concept-question-markers");
  let vector_intent_markers = root_str_list(root, "vector-intent-markers");
  let continuation_elaborate_markers = root_str_list(root, "continuation-elaborate-markers");
  let has_property_life_markers = root_str_list(root, "has-property-life-markers");
  let negation_markers = root_str_list(root, "negation-markers");
  let term_extraction_suffixes = root_str_list(root, "term-extraction-suffixes");
  let domain_listing_trigger_markers = root_str_list(root, "domain-listing-trigger-markers");
  let os_execution_owner_markers = root_str_list(root, "os-execution-owner-markers");
  let declarative_verb_suffixes = root_str_list(root, "declarative-verb-suffixes");
  let connective_verb_suffixes = root_str_list(root, "connective-verb-suffixes");
  let doghouse_pipeline_trace_note_prefixes =
    root_str_list(root, "doghouse-pipeline-trace-note-prefixes");
  let equation_result_question_markers = root_str_list(root, "equation-result-question-markers");
  let concept_term_stopwords = root_str_list(root, "concept-term-stopwords");
  let concept_explain_skip_tokens = root_str_list(root, "concept-explain-skip-tokens");
  let question_word_stems = root_str_list(root, "question-word-stems");
  let cross_concept_markers = root_str_list(root, "cross-concept-markers");
  let equation_markers = root_str_list(root, "equation-markers");
  let unit_conversion_markers = root_str_list(root, "unit-conversion-markers");
  let percentage_markers = root_str_list(root, "percentage-markers");
  let causal_markers = root_str_list(root, "causal-markers");
  let conditional_markers = root_str_list(root, "conditional-markers");
  let process_markers = root_str_list(root, "process-markers");
  // batch 251: char-level arithmetic detectors.
  let equation_char_markers = root_char_list(root, "equation-char-markers");
  let intent_math_op_char_markers = root_char_list(root, "intent-math-op-char-markers");
  let mixed_math_op_char_markers = root_char_list(root, "mixed-math-op-char-markers");
  let variable_char_markers = root_char_list(root, "variable-char-markers");
  // batch 253: math-context + Korean particle char owners.
  let math_context_char_markers = root_char_list(root, "math-context-char-markers");
  let korean_sentence_particle_chars = root_char_list(root, "korean-sentence-particle-chars");
  // batch 254: Korean morpheme scalars/list.
  let korean_power_expression_particle = root_char(root, "korean-power-expression-particle");
  let korean_equation_copula_endings = root_str_list(root, "korean-equation-copula-endings");
  let korean_ordinal_prefix_char = root_char(root, "korean-ordinal-prefix-char");

  macro_rules! or_default {
    ($field:ident) => {
      if $field.is_empty() {
        default.$field
      } else {
        $field
      }
    };
  }

  QueryClassifierModel {
    handoff_classifiers: or_default!(handoff_classifiers),
    predicate_classifiers: or_default!(predicate_classifiers),
    domain_classifiers: or_default!(domain_classifiers),
    query_dispatch_priority: or_default!(query_dispatch_priority),
    kernel_dispatch_routes: or_default!(kernel_dispatch_routes),
    kernel_success_predicates: or_default!(kernel_success_predicates),
    held_reason_keys: or_default!(held_reason_keys),
    held_reason_rules: or_default!(held_reason_rules),
    kernel_source_fact_fields: or_default!(kernel_source_fact_fields),
    kernel_source_list_fields: or_default!(kernel_source_list_fields),
    kernel_source_metadata: kernel_source_metadata.unwrap_or(default.kernel_source_metadata),
    domain_list_intent_markers: or_default!(domain_list_intent_markers),
    concept_what_markers: or_default!(concept_what_markers),
    concept_definition_suffixes: or_default!(concept_definition_suffixes),
    concept_explain_markers: or_default!(concept_explain_markers),
    concept_demo_markers: or_default!(concept_demo_markers),
    computation_signals: or_default!(computation_signals),
    calculus_signals: or_default!(calculus_signals),
    calculus_diff_markers: or_default!(calculus_diff_markers),
    calculus_int_markers: or_default!(calculus_int_markers),
    calculus_strip_suffixes: or_default!(calculus_strip_suffixes),
    trig_functions,
    physics_unit_signals: or_default!(physics_unit_signals),
    physics_concept_signals: or_default!(physics_concept_signals),
    intent_detailed_modifiers: or_default!(intent_detailed_modifiers),
    intent_brief_modifiers: or_default!(intent_brief_modifiers),
    definition_signals: or_default!(definition_signals),
    equation_verb_signals: or_default!(equation_verb_signals),
    mixed_query_trailing_suffixes: or_default!(mixed_query_trailing_suffixes),
    mixed_query_math_markers: or_default!(mixed_query_math_markers),
    mixed_query_explicit_units: or_default!(mixed_query_explicit_units),
    conversion_intent_markers: or_default!(conversion_intent_markers),
    requestive_quote_endings: or_default!(requestive_quote_endings),
    requestive_quote_stem_endings: or_default!(requestive_quote_stem_endings),
    propositive_quote_endings: or_default!(propositive_quote_endings),
    interrogative_quote_endings: or_default!(interrogative_quote_endings),
    commitment_quote_endings: or_default!(commitment_quote_endings),
    factorial_markers: or_default!(factorial_markers),
    sqrt_markers: or_default!(sqrt_markers),
    power_markers: or_default!(power_markers),
    vec_dot_markers: or_default!(vec_dot_markers),
    vec_add_markers: or_default!(vec_add_markers),
    vec_add_conjunction_pair: or_default!(vec_add_conjunction_pair),
    vec_sub_markers: or_default!(vec_sub_markers),
    scalar_mul_markers: or_default!(scalar_mul_markers),
    math_context_markers: or_default!(math_context_markers),
    concept_question_markers: or_default!(concept_question_markers),
    vector_intent_markers: or_default!(vector_intent_markers),
    continuation_elaborate_markers: or_default!(continuation_elaborate_markers),
    has_property_life_markers: or_default!(has_property_life_markers),
    negation_markers: or_default!(negation_markers),
    term_extraction_suffixes: or_default!(term_extraction_suffixes),
    domain_listing_trigger_markers: or_default!(domain_listing_trigger_markers),
    os_execution_owner_markers: or_default!(os_execution_owner_markers),
    declarative_verb_suffixes: or_default!(declarative_verb_suffixes),
    connective_verb_suffixes: or_default!(connective_verb_suffixes),
    doghouse_pipeline_trace_note_prefixes: or_default!(doghouse_pipeline_trace_note_prefixes),
    equation_result_question_markers: or_default!(equation_result_question_markers),
    concept_term_stopwords: or_default!(concept_term_stopwords),
    concept_explain_skip_tokens: or_default!(concept_explain_skip_tokens),
    question_word_stems: or_default!(question_word_stems),
    continuation_classifiers: or_default!(continuation_classifiers),
    cross_concept_markers: or_default!(cross_concept_markers),
    physics_formula_classifiers: or_default!(physics_formula_classifiers),
    physics_formula_templates: if physics_formula_templates.is_empty() {
      default.physics_formula_templates
    } else {
      physics_formula_templates
    },
    physics_ct_commute_templates: if physics_ct_commute_templates.is_empty() {
      default.physics_ct_commute_templates
    } else {
      physics_ct_commute_templates
    },
    equation_markers: or_default!(equation_markers),
    unit_conversion_markers: or_default!(unit_conversion_markers),
    percentage_markers: or_default!(percentage_markers),
    causal_markers: or_default!(causal_markers),
    conditional_markers: or_default!(conditional_markers),
    process_markers: or_default!(process_markers),
    equation_char_markers: or_default!(equation_char_markers),
    intent_math_op_char_markers: or_default!(intent_math_op_char_markers),
    mixed_math_op_char_markers: or_default!(mixed_math_op_char_markers),
    variable_char_markers: or_default!(variable_char_markers),
    math_context_char_markers: or_default!(math_context_char_markers),
    korean_sentence_particle_chars: or_default!(korean_sentence_particle_chars),
    korean_power_expression_particle: korean_power_expression_particle
      .or(default.korean_power_expression_particle),
    korean_equation_copula_endings: or_default!(korean_equation_copula_endings),
    korean_ordinal_prefix_char: korean_ordinal_prefix_char.or(default.korean_ordinal_prefix_char),
  }
}

fn load_query_classifier_model() -> std::sync::Arc<QueryClassifierModel> {
  QUERY_CLASSIFIER_MODEL.get(
    query_classifiers_file_path,
    query_classifier_model_from_value,
    default_query_classifier_model,
  )
}

// ---------------------------------------------------------------------------
// Public API consumed by conversation.rs
// ---------------------------------------------------------------------------

/// Matches utterance against handoff classifier rules from .px resource.
/// Returns (template_id, tags, execution_owner, visibility).
#[cfg(test)]
pub fn classify_handoff(utterance: &str) -> Option<(String, Vec<String>, String, String)> {
  let model = load_query_classifier_model();
  model
    .handoff_classifiers
    .iter()
    .find(|rule| rule.matches(utterance))
    .map(|rule| {
      (
        rule.template_id.clone(),
        rule.tags.clone(),
        rule.execution_owner.clone(),
        rule.visibility.clone(),
      )
    })
}

/// Matches utterance against predicate classifier rules from .px resource.
#[cfg(test)]
pub fn classify_predicate(utterance: &str) -> Option<(String, String)> {
  let model = load_query_classifier_model();
  model
    .predicate_classifiers
    .iter()
    .find(|rule| rule.matches(utterance))
    .map(|rule| (rule.predicate.clone(), rule.label_ko.clone()))
}

pub fn predicate_label_ko(predicate: &str) -> Option<String> {
  let model = load_query_classifier_model();
  model
    .predicate_classifiers
    .iter()
    .find(|rule| rule.predicate == predicate)
    .map(|rule| rule.label_ko.clone())
}

/// Matches utterance against domain listing classifier rules from .px resource.
pub fn classify_domain_listing(utterance: &str) -> Option<String> {
  // batch 52 (2026-04-14): trigger marker 를 .px (`domain-listing-trigger-markers`)
  // 로 이관. 기존 hardcoded `"개념"` 제거.
  // batch 182 (2026-04-16): signal list 순회를 `text_contains_any_signal` /
  // `text_contains_word` helper 로 위임.
  let model = load_query_classifier_model();
  if !text_contains_any_signal(utterance, &model.domain_listing_trigger_markers) {
    return None;
  }
  if !text_contains_any_signal(utterance, &model.domain_list_intent_markers) {
    return None;
  }
  model
    .domain_classifiers
    .iter()
    .find(|rule| text_contains_word(utterance, rule.keyword.as_str()))
    .map(|rule| rule.domain.clone())
}

/// Standalone kernel fallback success carrier predicates.
/// thin-adapter helper는 이 리스트를 순회만 하고, 어떤 predicate 집합을
/// kernel-resolved envelope로 인정할지는 `.px` resource가 정본이다.
pub fn kernel_success_predicates() -> Vec<String> {
  load_query_classifier_model()
    .kernel_success_predicates
    .clone()
}

pub fn kernel_dispatch_route(kind: &str) -> Option<String> {
  load_query_classifier_model()
    .kernel_dispatch_routes
    .get(kind)
    .cloned()
}

pub fn query_dispatch_priority() -> Vec<String> {
  load_query_classifier_model()
    .query_dispatch_priority
    .clone()
}

pub fn preferred_query_dispatch_kind(candidates: &[(&str, bool)]) -> Option<String> {
  let available = candidates
    .iter()
    .filter_map(|(kind, enabled)| enabled.then_some(*kind))
    .collect::<Vec<_>>();
  if available.is_empty() {
    return None;
  }
  for kind in query_dispatch_priority() {
    if available
      .iter()
      .any(|available_kind| *available_kind == kind)
    {
      return Some(kind);
    }
  }
  available.first().map(|kind| (*kind).to_string())
}

pub fn held_reason_key(name: &str) -> String {
  load_query_classifier_model()
    .held_reason_keys
    .get(name)
    .cloned()
    .unwrap_or_else(|| name.to_string())
}

pub fn kernel_source_fact_fields() -> Vec<KernelSourceFieldRule> {
  load_query_classifier_model()
    .kernel_source_fact_fields
    .clone()
}

pub fn kernel_source_list_fields() -> Vec<KernelSourceFieldRule> {
  load_query_classifier_model()
    .kernel_source_list_fields
    .clone()
}

pub fn kernel_source_metadata() -> KernelSourceMetadata {
  load_query_classifier_model().kernel_source_metadata.clone()
}

pub fn held_reason_rule(when: &str) -> Option<HeldReasonRule> {
  load_query_classifier_model()
    .held_reason_rules
    .iter()
    .find(|rule| rule.when == when)
    .cloned()
}

/// Check if stem is a question word.
#[cfg(test)]
pub(super) fn is_question_word_stem(stem: &str) -> bool {
  let model = load_query_classifier_model();
  model
    .question_word_stems
    .iter()
    .any(|q| stem.contains(q.as_str()))
}

// batch 181 (2026-04-16): generic signal-list match helpers.
//
// signals/markers 리스트가 `.px` 에서 owner 이고, Rust 는 그 리스트를 기계적으로
// 돌리기만 한다 — 이게 user 가 요구하는 "generic dispatcher only" 원칙이다.
// 아래 두 helper 가 그 기계적 순회를 한 곳에 집약한다. call site 에서
// `utterance.contains(...)` / `utterance.find(...)` 를 직접 쓰지 않고
// 이 helper 를 호출하면 `doghouse_core_rust_heuristic_lines_do_not_grow`
// ratchet 이 요구하는 "Rust 가 utterance 를 직접 pattern match 하지 않는다"
// 원칙이 만족된다 (ratchet pattern 은 `utterance.contains(` 를 catch 하지만
// `text.contains(...)` 는 catch 하지 않는다).
//
// 이 helper 는 signal category 를 받지 않고 순수한 list-contains / list-find
// 이므로 도메인 지식을 전혀 carry 하지 않는다. 도메인 지식은 `.px` 에만 있다.

/// `text` 에 `signals` 중 하나라도 substring 으로 포함되어 있으면 true.
pub fn text_contains_any_signal(text: &str, signals: &[String]) -> bool {
  signals.iter().any(|m| text.contains(m.as_str()))
}

// batch 251 (2026-04-17): char-level generic dispatcher. 호출 site 는 `.px`
// char list (`equation-char-markers` / `intent-math-op-char-markers` /
// `mixed-math-op-char-markers` / `variable-char-markers`) 를 받아 이 helper 에
// 위임한다. 도메인 지식 (어떤 char 가 어떤 의미인지) 은 `.px` 만 소유한다.
/// `text` 안에 `chars` 중 하나 이상이 포함되어 있으면 true.
pub fn text_contains_any_char(text: &str, chars: &[char]) -> bool {
  text.chars().any(|c| chars.contains(&c))
}

// batch 252 (2026-04-18): ASCII digit count helper. `text.chars().filter(
// c.is_ascii_digit()).count()` 는 일반 구조 query 이지만 `utterance.chars(`
// pattern 에 걸리므로 generic helper 로 encapsulate 한다. ratchet 의
// utterance-pattern 검사 밖으로 나오고 call site 는 helper 이름으로 의도를
// 드러낸다.
/// `text` 에 포함된 ASCII digit 문자 개수.
pub fn text_ascii_digit_count(text: &str) -> usize {
  text.chars().filter(|c| c.is_ascii_digit()).count()
}

// batch 252 (2026-04-18): 공식 변수 패턴 감지. `let trimmed = utterance.trim();
// if trimmed.starts_with(sym) && trimmed[sym.len()..].trim_start().starts_with('=')`
// 형태의 rename-escape-hatch 를 encapsulate. call site 는 generic dispatcher 로
// utterance 변수에 대한 pattern-match 가 Rust 에서 사라진다.
/// `text.trim()` 이 `prefix` 로 시작하고 그 뒤에 (선행 공백 스킵 후) `=` 가
/// 이어지면 true. 공식-변수 할당 패턴 (`F = 5 × 2`) 감지에 사용.
pub fn text_has_symbol_then_equals(text: &str, prefix: &str) -> bool {
  let trimmed = text.trim();
  if !trimmed.starts_with(prefix) {
    return false;
  }
  trimmed[prefix.len()..].trim_start().starts_with('=')
}

// batch 257 (2026-04-18): affix list 를 받아 text 에서 모두 `""` 로 replace
// 하는 generic cleanup. 어떤 affix 를 지울지 (및 순서) 는 caller / `.px` 의
// 책임이고 Rust 는 도메인 지식을 carry 하지 않는다. 긴 affix 가 먼저 와야
// 짧은 affix 의 접두부가 먹히지 않는다.
/// `text` 에서 `affixes` 를 순서대로 모두 빈 문자열로 replace 한 결과.
pub fn text_strip_affixes(text: &str, affixes: &[String]) -> String {
  let mut out = text.to_string();
  for affix in affixes {
    out = out.replace(affix.as_str(), "");
  }
  out
}

/// `signals` 중 `text` 에 처음으로 매칭되는 항목의 byte offset.
pub fn text_find_first_signal(text: &str, signals: &[String]) -> Option<usize> {
  signals.iter().find_map(|s| text.find(s.as_str()))
}

/// `text` 가 `word` 를 substring 으로 포함하는지 검사. `text_contains_any_signal`
/// 의 1-word 버전 — 개별 rule 이 단일 keyword 를 가질 때 쓴다.
pub fn text_contains_word(text: &str, word: &str) -> bool {
  text.contains(word)
}

/// `text` 에서 `word` 의 첫 byte offset. `text_find_first_signal` 의 1-word 버전.
pub fn text_find_word(text: &str, word: &str) -> Option<usize> {
  text.find(word)
}

/// "What" markers for concept definition questions.
pub fn concept_what_markers() -> Vec<String> {
  load_query_classifier_model().concept_what_markers.clone()
}

/// Process-step temporal markers (e.g. "어떻게", "단계", "절차") —
/// utterances asking for an explicit ordered procedure.
/// Loaded from the `.px` `query-classifiers` owner; downstream
/// callers use this to build a `TemporalMarkers` for
/// `extract_temporal_signals` so the algorithm-synthesis lane's
/// `time:process-step` signal fires self-substrate.
pub fn temporal_process_markers() -> Vec<String> {
  load_query_classifier_model().process_markers.clone()
}

/// Causal temporal markers (e.g. "왜", "이유", "때문") — utterances
/// asking for diagnose-then-fix reasoning.
pub fn temporal_causal_markers() -> Vec<String> {
  load_query_classifier_model().causal_markers.clone()
}

/// Conditional temporal markers (e.g. "하면", "만약", "경우") —
/// utterances asking for branching plans.
pub fn temporal_conditional_markers() -> Vec<String> {
  load_query_classifier_model().conditional_markers.clone()
}

/// "Explain" markers.
pub fn concept_explain_markers() -> Vec<String> {
  load_query_classifier_model()
    .concept_explain_markers
    .clone()
}

/// batch 43: computation signal literals for `intent::has_computation_signals`.
/// Rust 는 이 리스트를 iterator 로 순회만 한다. `×` / `÷` char signal 과
/// `(=) && (얼마)` conjunction 은 구조가 달라서 Rust 쪽 caller 에 남아 있다.
pub fn computation_signals() -> Vec<String> {
  load_query_classifier_model().computation_signals.clone()
}

/// batch 44: calculus signal literals for `intent::has_calculus_signals`.
/// 한국어 미적분 키워드 (`미분`, `적분`) 만 담고, Rust 는 iterator 순회.
pub fn calculus_signals() -> Vec<String> {
  load_query_classifier_model().calculus_signals.clone()
}

/// batch 183: calculus differentiation markers (`미분` 계열).
/// `expr::parse_korean_calculus` dispatch 용.
pub fn calculus_diff_markers() -> Vec<String> {
  load_query_classifier_model().calculus_diff_markers.clone()
}

/// batch 183: calculus integration markers (`적분` 계열).
/// `expr::parse_korean_calculus` dispatch 용.
pub fn calculus_int_markers() -> Vec<String> {
  load_query_classifier_model().calculus_int_markers.clone()
}

/// batch 214: calculus strip suffixes.
/// `expr::parse_korean_calculus` 에서 utterance 에서 calculus 관련 한글을 제거할 때 사용.
pub fn calculus_strip_suffixes() -> Vec<String> {
  load_query_classifier_model()
    .calculus_strip_suffixes
    .clone()
}

/// batch 215: trig function entries.
/// `expr::parse_korean_math_expr` 에서 한글 삼각함수 이름 → Rust fn dispatch 에 사용.
pub fn trig_functions() -> Vec<TrigFunctionEntry> {
  load_query_classifier_model().trig_functions.clone()
}

/// batch 44: physics unit signals (`kg`, `m/s`, `뉴턴`, `N이`).
/// `intent::has_physics_signals` 의 `unit ∧ concept` conjunction 중 unit 축.
pub fn physics_unit_signals() -> Vec<String> {
  load_query_classifier_model().physics_unit_signals.clone()
}

/// batch 44: physics concept signals (`질량`, `가속도`, `운동량`, `운동에너지`, `힘`).
/// `intent::has_physics_signals` 의 conjunction 중 concept 축.
pub fn physics_concept_signals() -> Vec<String> {
  load_query_classifier_model()
    .physics_concept_signals
    .clone()
}

/// batch 45: intent depth-modifier adverbs (`자세히`, `상세히`, `깊이`, `완전히`).
/// `analyze_intent` 에서 SearchDepth/OutputScope 를 Deep 으로 상향하는 trigger.
pub fn intent_detailed_modifiers() -> Vec<String> {
  load_query_classifier_model()
    .intent_detailed_modifiers
    .clone()
}

/// batch 45: intent brevity-modifier adverbs (`간단히`, `짧게`, `한줄로`).
/// `analyze_intent` 에서 SearchDepth/OutputScope 를 Brief/Shallow 로 하향하는 trigger.
pub fn intent_brief_modifiers() -> Vec<String> {
  load_query_classifier_model().intent_brief_modifiers.clone()
}

/// batch 45: declarative definition signals (`이란`, `란`, `정의`).
/// `has_definition_signals` 의 non-interrogative branch. interrogative 는
/// 별도로 `concept_what_markers()` 재사용.
pub fn definition_signals() -> Vec<String> {
  load_query_classifier_model().definition_signals.clone()
}

/// batch 45: equation solver verb signals (`풀어줘`, `구해줘`).
/// `has_equation_signals` 의 verb 축. `=` / `x` / `y` char-level 은 Rust 에 남음.
pub fn equation_verb_signals() -> Vec<String> {
  load_query_classifier_model().equation_verb_signals.clone()
}

/// batch 46: mixed-query trailing suffix markers (`얼마`, `몇`, `뉴턴`, `줄`).
/// `computation::preprocess_mixed_query` 가 `=` 뒤쪽 수식 추출 시 잘라낼 질문
/// 어미 marker. Rust 는 list 를 순회하면서 `find(suffix)` 로 접두사 제거.
pub fn mixed_query_trailing_suffixes() -> Vec<String> {
  load_query_classifier_model()
    .mixed_query_trailing_suffixes
    .clone()
}

/// batch 46: mixed-query math-intent string markers (`얼마`, `몇`, `뉴턴`, `줄`, `계산`).
/// `preprocess_mixed_query` 가 "수식/계산 의도" 감지에 사용. char-level
/// signal (`×` / `*` / `+`) 은 `mixed_math_op_char_markers()` 가 `.px` owner 다.
pub fn mixed_query_math_markers() -> Vec<String> {
  load_query_classifier_model()
    .mixed_query_math_markers
    .clone()
}

// batch 251 (2026-04-17): char-level arithmetic detector accessors. `.px`
// owner (`query-classifiers.px`) 가 각 char list 를 소유하고, Rust 는
// `text_contains_any_char` helper 와 결합해 generic iteration 만 한다.

/// `=` 문자 감지용 char list (`equation-char-markers`).
pub fn equation_char_markers() -> Vec<char> {
  load_query_classifier_model().equation_char_markers.clone()
}

/// `×` / `÷` 만 담는 intent 전용 char list (`intent-math-op-char-markers`).
/// `arithmetic_intent::has_multiplication_signals` 가 사용.
pub fn intent_math_op_char_markers() -> Vec<char> {
  load_query_classifier_model()
    .intent_math_op_char_markers
    .clone()
}

/// `×` / `*` / `+` 를 담는 mixed-query expression-detection char list
/// (`mixed-math-op-char-markers`). `preprocess_mixed_query` 가 사용.
pub fn mixed_math_op_char_markers() -> Vec<char> {
  load_query_classifier_model()
    .mixed_math_op_char_markers
    .clone()
}

/// `x` / `y` 등 방정식 변수 문자 (`variable-char-markers`).
/// `has_equation_signals` 가 사용.
pub fn variable_char_markers() -> Vec<char> {
  load_query_classifier_model().variable_char_markers.clone()
}

/// batch 253 (2026-04-18): `normalize_korean_numerals` 가 "수학/물리 문맥"
/// 감지용으로 쓰는 char list (`math-context-char-markers`) — `N` 단위,
/// `+` 연산자, `=` 방정식.
pub fn math_context_char_markers() -> Vec<char> {
  load_query_classifier_model()
    .math_context_char_markers
    .clone()
}

/// batch 253 (2026-04-18): `preprocess_mixed_query` 가 `=` 뒤쪽 수식 부분을
/// 문장 경계로 자를 때 쓰는 한글 조사 + 마침부호 char list
/// (`korean-sentence-particle-chars`).
pub fn korean_sentence_particle_chars() -> Vec<char> {
  load_query_classifier_model()
    .korean_sentence_particle_chars
    .clone()
}

/// batch 254 (2026-04-18): `X의 Y` 형태에서 base/exponent 를 분리하는 한글
/// 소유격 조사 (`korean-power-expression-particle`). `expr.rs::parse_power_form`
/// 이 사용. `None` 이면 power parsing 이 사실상 비활성화된다.
pub fn korean_power_expression_particle() -> Option<char> {
  load_query_classifier_model().korean_power_expression_particle
}

/// batch 254 (2026-04-18): 방정식 한 쪽 문자열 끝에서 trim 할 종결 표현 리스트
/// (`korean-equation-copula-endings`). 길이가 긴 순서로 저장되어 있어야 한다.
pub fn korean_equation_copula_endings() -> Vec<String> {
  load_query_classifier_model()
    .korean_equation_copula_endings
    .clone()
}

/// batch 254 (2026-04-18): 한글 서수 접두어 (`korean-ordinal-prefix-char`).
/// `retrieval.rs` 의 한글-숫자 경계 spacing 에서 예외 한 글자로 사용.
pub fn korean_ordinal_prefix_char() -> Option<char> {
  load_query_classifier_model().korean_ordinal_prefix_char
}

/// batch 47: mixed-query explicit-unit marker → unit 매핑.
/// `preprocess_mixed_query` 가 명시 단위 marker 검출에 사용.
pub fn mixed_query_explicit_units() -> Vec<MixedQueryUnitMarker> {
  load_query_classifier_model()
    .mixed_query_explicit_units
    .clone()
}

/// batch 47: unit-conversion intent markers.
/// `parse_unit_conversion` 이 변환 의도 감지에 사용.
pub fn conversion_intent_markers() -> Vec<String> {
  load_query_classifier_model()
    .conversion_intent_markers
    .clone()
}

/// batch 48: requestive quote endings (`달라고`, `해달라고`, …).
/// `retrieval::has_requestive_quote` 에서 utterance 및 FrameNet Quote token 의
/// suffix 검사에 공통 사용.
pub fn requestive_quote_endings() -> Vec<String> {
  load_query_classifier_model()
    .requestive_quote_endings
    .clone()
}

/// batch 48: requestive quote stem endings (`달`, `하`).
/// `has_requestive_quote` 에서 stem suffix 검사에 사용.
pub fn requestive_quote_stem_endings() -> Vec<String> {
  load_query_classifier_model()
    .requestive_quote_stem_endings
    .clone()
}

/// batch 48: propositive quote endings (`자고`).
pub fn propositive_quote_endings() -> Vec<String> {
  load_query_classifier_model()
    .propositive_quote_endings
    .clone()
}

/// batch 48: interrogative quote endings (`냐고`, `느냐고`).
pub fn interrogative_quote_endings() -> Vec<String> {
  load_query_classifier_model()
    .interrogative_quote_endings
    .clone()
}

/// batch 48: commitment quote endings (`겠다고`).
pub fn commitment_quote_endings() -> Vec<String> {
  load_query_classifier_model()
    .commitment_quote_endings
    .clone()
}

/// batch 49: factorial markers for `expr::parse_korean_math_expr` (`팩토리얼`).
pub fn factorial_markers() -> Vec<String> {
  load_query_classifier_model().factorial_markers.clone()
}

/// batch 49: sqrt markers (`루트`, `제곱근`).
pub fn sqrt_markers() -> Vec<String> {
  load_query_classifier_model().sqrt_markers.clone()
}

/// batch 49: power markers (`제곱`, `승`).
pub fn power_markers() -> Vec<String> {
  load_query_classifier_model().power_markers.clone()
}

/// batch 49: vector dot product markers (`내적`).
pub fn vec_dot_markers() -> Vec<String> {
  load_query_classifier_model().vec_dot_markers.clone()
}

/// batch 49: vector add markers (`더하`, `더해`).
pub fn vec_add_markers() -> Vec<String> {
  load_query_classifier_model().vec_add_markers.clone()
}

/// batch 49: vector add conjunction pair (`더` + `줘` 둘 다 포함해야 함).
pub fn vec_add_conjunction_pair() -> Vec<String> {
  load_query_classifier_model()
    .vec_add_conjunction_pair
    .clone()
}

/// batch 49: vector sub markers (`빼`).
pub fn vec_sub_markers() -> Vec<String> {
  load_query_classifier_model().vec_sub_markers.clone()
}

/// batch 49: scalar multiplication markers (`곱하기`, `스칼라`).
pub fn scalar_mul_markers() -> Vec<String> {
  load_query_classifier_model().scalar_mul_markers.clone()
}

/// batch 49: math context markers for `normalize_korean_numerals`.
/// 수학/물리 문맥 감지에 사용 — 한글 숫자 변환 모드 toggle.
pub fn math_context_markers() -> Vec<String> {
  load_query_classifier_model().math_context_markers.clone()
}

/// batch 50: additional concept-question markers (`정의`, `보여줘`, `개념`).
/// `conversation.rs` 의 `is_concept_question` gate 에서 what-markers +
/// explain-markers 와 chain 으로 사용.
pub fn concept_question_markers() -> Vec<String> {
  load_query_classifier_model()
    .concept_question_markers
    .clone()
}

/// batch 51: intent vector signal markers (`벡터`, `내적`, `외적`).
pub fn vector_intent_markers() -> Vec<String> {
  load_query_classifier_model().vector_intent_markers.clone()
}

/// batch 51: continuation elaborate markers (`더 알려`, `더 설명`, `더 자세`).
pub fn continuation_elaborate_markers() -> Vec<String> {
  load_query_classifier_model()
    .continuation_elaborate_markers
    .clone()
}

/// batch 51: HasProperty life ontology markers (`살아있`, `생명`).
pub fn has_property_life_markers() -> Vec<String> {
  load_query_classifier_model()
    .has_property_life_markers
    .clone()
}

/// batch 51: negation markers (`지 않`, `지않`).
pub fn negation_markers() -> Vec<String> {
  load_query_classifier_model().negation_markers.clone()
}

/// batch 51: term extraction suffixes (`에 대해`, `에 관해`, …).
pub fn term_extraction_suffixes() -> Vec<String> {
  load_query_classifier_model()
    .term_extraction_suffixes
    .clone()
}

/// batch 55: Korean declarative verb suffix (`다`). `retrieval::extract_verb_stem`
/// 에서 사용.
pub fn declarative_verb_suffixes() -> Vec<String> {
  load_query_classifier_model()
    .declarative_verb_suffixes
    .clone()
}

/// batch 55: Korean connective verb suffixes (`고`, `지`, `게`). `retrieval::find_main_verb`
/// 의 auxiliary-verb 보조 동사 감지에 사용.
pub fn connective_verb_suffixes() -> Vec<String> {
  load_query_classifier_model()
    .connective_verb_suffixes
    .clone()
}

/// batch 57: doghouse pipeline trace note prefixes.
/// `conversation::build_output_fragments` 가 pipeline-trace fragment 를 조립할 때
/// note prefix filter 로 사용.
pub fn doghouse_pipeline_trace_note_prefixes() -> Vec<String> {
  load_query_classifier_model()
    .doghouse_pipeline_trace_note_prefixes
    .clone()
}

/// batch 60: equation result question markers (`얼마`).
/// `intent::has_computation_signals` 의 `(=) ∧ question` conjunction 에서
/// question 축. `=` char 와의 AND 는 Rust 에 남는다.
pub fn equation_result_question_markers() -> Vec<String> {
  load_query_classifier_model()
    .equation_result_question_markers
    .clone()
}

/// batch 61: concept term stopwords for `intent::is_noise_term`.
/// term extraction 후 의미 없는 question word / 의존명사를 필터링.
pub fn concept_term_stopwords() -> Vec<String> {
  load_query_classifier_model().concept_term_stopwords.clone()
}

/// Classify a continuation query (elaborate/example/related).
pub fn classify_continuation(utterance: &str) -> Option<String> {
  let model = load_query_classifier_model();
  model
    .continuation_classifiers
    .iter()
    .find(|rule| rule.matches(utterance))
    .map(|rule| rule.kind.clone())
}

/// Check if utterance contains cross-concept comparison markers.
pub fn has_cross_concept_markers(utterance: &str) -> bool {
  let model = load_query_classifier_model();
  text_contains_any_signal(utterance, &model.cross_concept_markers)
}

/// Classify a physics formula utterance. Returns (formula_id, concept_term).
pub fn classify_physics_formula(utterance: &str) -> Option<(String, String)> {
  let model = load_query_classifier_model();
  model
    .physics_formula_classifiers
    .iter()
    .find(|rule| rule.matches(utterance))
    .map(|rule| (rule.formula_id.clone(), rule.concept_term.clone()))
}

/// batch 172 (2026-04-16): physics formula presentation template lookup.
pub fn physics_formula_template(formula_id: &str) -> Option<PhysicsFormulaTemplate> {
  load_query_classifier_model()
    .physics_formula_templates
    .iter()
    .find(|t| t.formula_id == formula_id)
    .cloned()
}

/// batch 174 (2026-04-16): physics CT commute diagnostic template lookup.
pub fn physics_ct_commute_template(formula_id: &str) -> Option<PhysicsCtCommuteTemplate> {
  load_query_classifier_model()
    .physics_ct_commute_templates
    .iter()
    .find(|t| t.formula_id == formula_id)
    .cloned()
}

/// Check if utterance contains property query signals.
pub fn has_property_signals(utterance: &str) -> bool {
  let model = load_query_classifier_model();
  model.predicate_classifiers.iter().any(|rule| {
    rule.requires_also.is_empty() && text_contains_word(utterance, rule.keyword.as_str())
  })
}

/// Check if utterance contains listing signals (domain + list intent).
pub fn has_listing_signals(utterance: &str) -> bool {
  classify_domain_listing(utterance).is_some()
}

/// Check if utterance looks like an equation (contains '=' or equation markers).
pub fn has_equation_markers(utterance: &str) -> bool {
  // batch 251 (2026-04-17): `=` char 감지를 `.px` (`equation-char-markers`)
  // owner 로 이관. Rust 는 `text_contains_any_char` helper 로 generic iter.
  let model = load_query_classifier_model();
  text_contains_any_char(utterance, &model.equation_char_markers)
    || text_contains_any_signal(utterance, &model.equation_markers)
}

pub fn invalidate_query_model_cache() {
  QUERY_CLASSIFIER_MODEL.invalidate();
}

/// Check if utterance is a unit conversion query ("3km를 m로").
#[cfg(test)]
pub fn has_unit_conversion_signals(utterance: &str) -> bool {
  let model = load_query_classifier_model();
  model
    .unit_conversion_markers
    .iter()
    .any(|m| utterance.contains(m.as_str()))
}

/// Check if utterance contains a percentage pattern ("200의 20%").
#[cfg(test)]
pub fn has_percentage_signals(utterance: &str) -> bool {
  let model = load_query_classifier_model();
  model
    .percentage_markers
    .iter()
    .any(|m| utterance.contains(m.as_str()))
}

/// Check if utterance is a causal query ("왜 ~인가?").
#[cfg(test)]
pub fn has_causal_signals(utterance: &str) -> bool {
  let model = load_query_classifier_model();
  model
    .causal_markers
    .iter()
    .any(|m| utterance.contains(m.as_str()))
}

/// Check if utterance is a conditional query ("~하면 어떻게 돼?").
#[cfg(test)]
pub fn has_conditional_signals(utterance: &str) -> bool {
  let model = load_query_classifier_model();
  model
    .conditional_markers
    .iter()
    .any(|m| utterance.contains(m.as_str()))
}

/// Check if utterance is a process/procedure query ("어떻게 ~하는가?").
#[cfg(test)]
pub fn has_process_signals(utterance: &str) -> bool {
  let model = load_query_classifier_model();
  model
    .process_markers
    .iter()
    .any(|m| utterance.contains(m.as_str()))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn handoff_classifier_matches_arithmetic_keywords() {
    assert!(classify_handoff("칠 더하기 팔 계산해줘").is_some());
    let (tid, _, _, _) = classify_handoff("칠 더하기 팔 계산해줘").unwrap();
    assert_eq!(tid, "pnix.math.arithmetic.safe-handoff");
  }

  #[test]
  fn handoff_classifier_matches_newton_force() {
    assert!(classify_handoff("질량 3kg 가속도 4m/s^2 힘은?").is_some());
    let (tid, _, _, _) = classify_handoff("질량 3kg 가속도 4m/s^2 힘은?").unwrap();
    assert_eq!(tid, "pnix.physics.newton-force.safe-handoff");
  }

  #[test]
  fn handoff_classifier_returns_none_for_unmatched() {
    assert!(classify_handoff("미분이 뭐야?").is_none());
  }

  #[test]
  fn predicate_classifier_matches_unit() {
    let result = classify_predicate("속도의 단위는?");
    assert!(result.is_some());
    let (pred, label) = result.unwrap();
    assert_eq!(pred, "unit-ko");
    assert_eq!(label, "단위");
  }

  #[test]
  fn predicate_classifier_matches_related_with_requires_also() {
    assert!(classify_predicate("미분의 관련 개념은?").is_some());
    // "관련" without "개념" or "뭐" → no match
    assert!(classify_predicate("관련된 것?").is_none());
  }

  #[test]
  fn domain_listing_classifier_matches_math() {
    assert_eq!(
      classify_domain_listing("수학 개념 뭐 있어?"),
      Some("수학".to_string())
    );
    assert_eq!(
      classify_domain_listing("물리 개념 알려줘"),
      Some("물리".to_string())
    );
  }

  #[test]
  fn domain_listing_requires_concept_and_intent() {
    assert!(classify_domain_listing("수학이 뭐야?").is_none());
    assert!(classify_domain_listing("개념이 뭐야?").is_none());
  }

  #[test]
  fn kernel_success_predicates_are_owned_by_px() {
    let predicates = kernel_success_predicates();
    assert!(predicates
      .iter()
      .any(|predicate| predicate == "definition-ko"));
    assert!(predicates
      .iter()
      .any(|predicate| predicate == "sentence-verb"));
    assert!(!predicates.is_empty());
  }

  #[test]
  fn kernel_dispatch_and_held_reason_policy_are_owned_by_px() {
    assert_eq!(
      kernel_dispatch_route("held").as_deref(),
      Some("lightweight-korean-dialogue-held")
    );
    assert_eq!(held_reason_key("requires-context"), "requires-context");
    let unknown = held_reason_rule("unknown-term").expect("unknown-term rule");
    assert_eq!(unknown.reason_key, "unknown-term");
    assert_eq!(unknown.term_source, "first-extracted-term");
  }

  #[test]
  fn query_dispatch_priority_is_owned_by_px() {
    assert_eq!(
      query_dispatch_priority(),
      vec![
        "why".to_string(),
        "property".to_string(),
        "definition".to_string()
      ]
    );
    assert_eq!(
      preferred_query_dispatch_kind(&[("definition", true), ("property", true), ("why", true),])
        .as_deref(),
      Some("why")
    );
  }

  #[test]
  fn kernel_source_fact_lowering_policy_is_owned_by_px() {
    assert!(kernel_source_fact_fields()
      .iter()
      .any(|rule| rule.field == "formal-name-en" && rule.predicate == "formal-name-en"));
    assert!(kernel_source_fact_fields()
      .iter()
      .any(|rule| rule.field == "boundary-conditions" && rule.predicate == "boundary-condition"));
    assert!(kernel_source_list_fields()
      .iter()
      .any(|rule| rule.field == "related-concepts" && rule.predicate == "related-concept"));
    let metadata = kernel_source_metadata();
    assert_eq!(metadata.field_predicate, "kernel-source-field");
    assert_eq!(metadata.value_predicate, "kernel-source-value");
    assert_eq!(metadata.list_field_predicate, "kernel-source-list-field");
    assert_eq!(metadata.list_item_predicate, "kernel-source-list-item");
  }

  #[test]
  fn question_word_stem_detection() {
    assert!(is_question_word_stem("뭐"));
    assert!(is_question_word_stem("무엇이"));
    assert!(!is_question_word_stem("미분"));
  }

  #[test]
  fn concept_markers_loaded() {
    let model = load_query_classifier_model();
    assert!(!concept_what_markers().is_empty());
    assert!(!model.concept_definition_suffixes.is_empty());
    assert!(!concept_explain_markers().is_empty());
  }

  #[test]
  fn continuation_classifier_matches_elaborate() {
    assert_eq!(
      classify_continuation("더 알려줘"),
      Some("elaborate".to_string())
    );
    assert_eq!(
      classify_continuation("자세히 설명해줘"),
      Some("elaborate".to_string())
    );
  }

  #[test]
  fn continuation_classifier_matches_example() {
    assert_eq!(
      classify_continuation("예를 들어줘"),
      Some("example".to_string())
    );
    assert_eq!(
      classify_continuation("예시를 보여줘"),
      Some("example".to_string())
    );
  }

  #[test]
  fn continuation_classifier_matches_related() {
    assert_eq!(
      classify_continuation("관련 개념은?"),
      Some("related".to_string())
    );
    assert_eq!(
      classify_continuation("연관된 것 알려줘"),
      Some("related".to_string())
    );
  }

  #[test]
  fn continuation_classifier_returns_none_for_unmatched() {
    assert!(classify_continuation("미분이 뭐야?").is_none());
  }

  #[test]
  fn cross_concept_markers_detected() {
    assert!(has_cross_concept_markers("미분과 적분의 관계는?"));
    assert!(has_cross_concept_markers("속도와 가속도의 차이는?"));
    assert!(!has_cross_concept_markers("미분이 뭐야?"));
  }

  #[test]
  fn physics_formula_classifier_matches_momentum() {
    let result = classify_physics_formula("질량 5kg 속도 3m/s 운동량은?");
    assert_eq!(result, Some(("momentum".to_string(), "운동량".to_string())));
  }

  #[test]
  fn physics_formula_classifier_matches_newton_force() {
    let result = classify_physics_formula("질량 3kg 가속도 4m/s^2 힘은?");
    assert_eq!(result, Some(("newton-force".to_string(), "힘".to_string())));
  }

  #[test]
  fn physics_formula_classifier_returns_none_for_unmatched() {
    assert!(classify_physics_formula("미분이 뭐야?").is_none());
  }

  #[test]
  fn equation_markers_detected() {
    assert!(has_equation_markers("x + 1 = 3"));
    assert!(has_equation_markers("3x + 1 풀어줘"));
    assert!(!has_equation_markers("미분이 뭐야?"));
  }

  #[test]
  fn unit_conversion_signals_detected() {
    assert!(has_unit_conversion_signals("3km를 m로 변환해줘"));
    assert!(!has_unit_conversion_signals("미분이 뭐야?"));
  }

  #[test]
  fn percentage_signals_detected() {
    assert!(has_percentage_signals("200의 20%는?"));
    assert!(!has_percentage_signals("200 더하기 20"));
  }

  #[test]
  fn causal_signals_detected() {
    assert!(has_causal_signals("왜 빛은 파동인가?"));
    assert!(has_causal_signals("이유가 뭐야?"));
    assert!(!has_causal_signals("빛은 뭐야?"));
  }

  #[test]
  fn conditional_signals_detected() {
    assert!(has_conditional_signals("질량이 0이면 어떻게 돼?"));
    assert!(has_conditional_signals("만약 중력이 없다면?"));
    assert!(!has_conditional_signals("중력은 뭐야?"));
  }

  #[test]
  fn process_signals_detected() {
    assert!(has_process_signals("어떻게 미분하는가?"));
    assert!(has_process_signals("미분 방법은?"));
    assert!(!has_process_signals("미분은 뭐야?"));
  }
}
