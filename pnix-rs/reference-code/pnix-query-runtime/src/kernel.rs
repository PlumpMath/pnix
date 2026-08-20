use crate::px::{parse_px_file, parse_px_file_with_pnix_eval_fallback, PxValue};
use crate::response_document::response_document_html;
use anyhow::{anyhow, Context, Result};
use pnix_core::judgement_protocol::{JudgementEvent, PromotionEvent};
use pnix_core::judgment::{self, DecisionEvents, JudgementIntent, OutputScope, QueryRouteSpec};
use pnix_core::lang::{analyze_korean_text, KoreanParticleKind, KoreanSentenceMood};
use pnix_core::ontology::{
  ContextId, ContextualFact, KnowledgeRecord, KnowledgeRecordId, LayerId, MeaningId, MeaningStatus,
  SemanticEpisode, SemanticEpisodeId, SemanticIngestEnvelope, SemanticRecord, SemanticRecordId,
  SemanticRecordKind, SemanticRecordValue,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const OUTPUT_FRAGMENT_PRODUCER_PNIX: &str = "pnix-ontology-engine";
pub const OUTPUT_FRAGMENT_CONTRACT_V1: &str = "pnix.output-fragment.v1";

static EPISODE_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelOutputFragment {
  pub producer: String,
  pub producer_contract: String,
  pub producer_route: String,
  pub producer_episode_id: String,
  pub kind: String,
  pub visibility: String,
  pub content_org: String,
  pub content_px: Option<String>,
  pub content_html: Option<String>,
  pub content_speech: Option<String>,
  pub content_text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct KernelPaths {
  pub data_dir: PathBuf,
  pub concepts_dir: PathBuf,
  pub korean_morphology_path: PathBuf,
  pub query_classifiers_path: PathBuf,
  pub query_routes_path: PathBuf,
  pub query_route_defaults_path: PathBuf,
  pub followup_generation_path: PathBuf,
  pub ontology_invert_path: PathBuf,
  pub synonyms_path: PathBuf,
  pub dialogue_templates_path: PathBuf,
  pub kernel_base_facts_path: PathBuf,
}

impl KernelPaths {
  pub fn from_data_dir(data_dir: impl Into<PathBuf>) -> Self {
    let data_dir = data_dir.into();
    Self {
      concepts_dir: data_dir.join("concepts"),
      korean_morphology_path: data_dir.join("korean-morphology.px"),
      query_classifiers_path: data_dir.join("query-classifiers.px"),
      query_routes_path: data_dir.join("query-routes.px"),
      query_route_defaults_path: data_dir.join("query-route-defaults.px"),
      followup_generation_path: data_dir.join("followup-generation.px"),
      ontology_invert_path: data_dir.join("ontology-invert.px"),
      synonyms_path: data_dir.join("concepts/synonyms.px"),
      dialogue_templates_path: data_dir.join("dialogue-templates.px"),
      kernel_base_facts_path: data_dir.join("kernel-base-facts.px"),
      data_dir,
    }
  }
}

impl Default for KernelPaths {
  fn default() -> Self {
    let data_dir = std::env::var("PNIX_QUERY_RUNTIME_DATA_DIR")
      .map(PathBuf::from)
      .unwrap_or_else(|_| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
          .join("data")
          .canonicalize()
          .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data"))
      });
    Self::from_data_dir(data_dir)
  }
}

#[derive(Debug, Clone, Default)]
struct HeldState {
  reason: String,
  term: Option<String>,
}

#[derive(Debug, Clone)]
pub struct KernelResponse {
  pub episode_id: String,
  pub route: String,
  pub summary: String,
  pub transcript: Vec<String>,
  pub follow_up_hint: Option<String>,
  pub follow_up_choices: Vec<String>,
  pub truth_regime: Option<String>,
  pub envelope: SemanticIngestEnvelope,
  pub judgement_events: Vec<JudgementEvent>,
  pub promotion_events: Vec<PromotionEvent>,
  pub response_document_org: String,
  pub response_document_px: String,
  pub output_fragments: Vec<KernelOutputFragment>,
}

#[derive(Debug, Clone, Default)]
pub struct PnixReplKernel {
  paths: KernelPaths,
  held_state: Option<HeldState>,
  last_term: Option<String>,
}

impl PnixReplKernel {
  pub fn new(paths: KernelPaths) -> Self {
    Self {
      paths,
      held_state: None,
      last_term: None,
    }
  }

  // C6 read-model bridge: pnix-query-runtime may evaluate query
  // documents to load/request typed runtime resources. Do not treat
  // this as canonical mirror primitive proof; substrate decisions route
  // through pnixc-meta mirror lens dispatch.
  pub fn evaluate_px_source(&mut self, source: &str) -> Result<KernelResponse> {
    let doc = pnix_eval::eval_pnix_expr(source).context("evaluate pnix query document")?;
    self.evaluate_eval_document(&doc)
  }

  pub fn evaluate_px_document(&mut self, doc: &PxValue) -> Result<KernelResponse> {
    let request = PnixQueryRequest::from_px(doc)?;
    let resources = RuntimeResources::load(&self.paths)?;
    self.handle_request(&resources, request)
  }

  pub fn evaluate_px_file(&mut self, path: &Path) -> Result<KernelResponse> {
    let doc = pnix_eval::eval_pnix_file(path)
      .with_context(|| format!("evaluate pnix query document {}", path.display()))?;
    self.evaluate_eval_document(&doc)
  }

  fn evaluate_eval_document(&mut self, doc: &pnix_eval::Value) -> Result<KernelResponse> {
    let request = PnixQueryRequest::from_eval_value(doc)?;
    let resources = RuntimeResources::load(&self.paths)?;
    self.handle_request(&resources, request)
  }

  fn handle_request(
    &mut self,
    resources: &RuntimeResources,
    request: PnixQueryRequest,
  ) -> Result<KernelResponse> {
    let reopened = self.held_state.take();
    let reopen_seed_term = reopened
      .as_ref()
      .map(|state| reopen_seed_term(resources, &request.utterance, state))
      .transpose()?
      .flatten();
    let effective_utterance = reopened
      .as_ref()
      .map(|state| {
        reopen_effective_utterance(
          resources,
          &request.utterance,
          state,
          reopen_seed_term.as_deref(),
        )
      })
      .transpose()?
      .unwrap_or_else(|| request.utterance.clone());
    let classifier_seed_term = reopen_seed_term
      .clone()
      .or_else(|| request.seeded_term.clone())
      .or_else(|| self.last_term.clone());
    let dispatch = classify_query(
      resources,
      &effective_utterance,
      classifier_seed_term.as_deref(),
      request.classifier_mode,
    )?;
    let response = match dispatch {
      QueryDispatch::Definition { term } => {
        self.last_term = Some(term.clone());
        let response = answer_definition(resources, &request, &term, reopened.as_ref())?;
        self.held_state = None;
        response
      }
      QueryDispatch::Handoff(query) => {
        let response = answer_handoff(resources, &request, &query, reopened.as_ref())?;
        self.held_state = None;
        response
      }
      QueryDispatch::Continuation { term, kind } => {
        self.last_term = Some(term.clone());
        let response = answer_continuation(resources, &request, &term, kind, reopened.as_ref())?;
        self.held_state = None;
        response
      }
      QueryDispatch::SentenceAnalysis => {
        let response = answer_sentence_analysis(resources, &request, reopened.as_ref())?;
        self.held_state = None;
        response
      }
      QueryDispatch::CrossConcept { term_a, term_b } => {
        let response =
          answer_cross_concept(resources, &request, &term_a, &term_b, reopened.as_ref())?;
        self.held_state = None;
        response
      }
      QueryDispatch::DomainListing { domain } => {
        let response = answer_domain_listing(resources, &request, &domain, reopened.as_ref())?;
        self.held_state = None;
        response
      }
      QueryDispatch::Property {
        term,
        predicate,
        label_ko,
      } => {
        self.last_term = Some(term.clone());
        let response = answer_property(
          resources,
          &request,
          &term,
          &predicate,
          &label_ko,
          reopened.as_ref(),
        )?;
        self.held_state = None;
        response
      }
      QueryDispatch::Why {
        term,
        trigger_type,
        truth_regime,
      } => {
        self.last_term = Some(term.clone());
        let response = answer_why(
          resources,
          &request,
          &term,
          &trigger_type,
          &truth_regime,
          reopened.as_ref(),
        )?;
        self.held_state = None;
        response
      }
      QueryDispatch::Held { term, reason } => {
        let response = build_held_response(
          resources,
          &request,
          term.clone(),
          &reason,
          reopened.as_ref(),
        )?;
        self.held_state = Some(HeldState { reason, term });
        response
      }
    };
    Ok(response)
  }
}

#[derive(Debug, Clone)]
struct PnixQueryRequest {
  utterance: String,
  scope: OutputScope,
  seeded_term: Option<String>,
  classifier_mode: KernelClassifierMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KernelClassifierMode {
  Default,
  Handoff,
}

impl PnixQueryRequest {
  fn from_px(doc: &PxValue) -> Result<Self> {
    let root = doc
      .as_attrset()
      .ok_or_else(|| anyhow!("pnix query document must be an attrset"))?;
    let query_kind = required_px_string(root, "kind")?;
    validate_query_kind(query_kind)?;
    let utterance = root
      .get("utterance")
      .and_then(|v| v.as_str())
      .map(str::trim)
      .filter(|s| !s.is_empty())
      .ok_or_else(|| anyhow!("ontology-query requires non-empty utterance"))?
      .to_string();
    let scope = parse_output_scope(required_px_string(root, "scope")?)?;
    let seeded_term = match root.get("seeded-term") {
      Some(PxValue::String(term)) => {
        let trimmed = term.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
      }
      Some(_) => {
        return Err(anyhow!(
          "'seeded-term' must be string in standalone pnix query document"
        ))
      }
      None => None,
    };
    let classifier_mode = match root.get("classifier-mode") {
      Some(PxValue::String(mode)) => match mode.as_str() {
        "handoff" => KernelClassifierMode::Handoff,
        "default" => KernelClassifierMode::Default,
        _ => {
          return Err(anyhow!(
            "invalid 'classifier-mode' for standalone pnix query document"
          ))
        }
      },
      Some(_) => {
        return Err(anyhow!(
          "'classifier-mode' must be string in standalone pnix query document"
        ))
      }
      None => KernelClassifierMode::Default,
    };
    Ok(Self {
      utterance,
      scope,
      seeded_term,
      classifier_mode,
    })
  }

  fn from_eval_value(doc: &pnix_eval::Value) -> Result<Self> {
    let root = match doc {
      pnix_eval::Value::AttrSet(map) => map,
      _ => return Err(anyhow!("pnix query document must evaluate to an attrset")),
    };
    let query_kind = required_eval_string(root, "kind")?;
    validate_query_kind(query_kind)?;
    let utterance = required_eval_string(root, "utterance")?.trim().to_string();
    if utterance.is_empty() {
      return Err(anyhow!("ontology-query requires non-empty utterance"));
    }
    let scope = parse_output_scope(required_eval_string(root, "scope")?)?;
    let seeded_term = match root.get("seeded-term") {
      Some(pnix_eval::Value::String(term)) => {
        let trimmed = term.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
      }
      Some(_) => {
        return Err(anyhow!(
          "'seeded-term' must be string in standalone pnix query document"
        ))
      }
      None => None,
    };
    let classifier_mode = match root.get("classifier-mode") {
      Some(pnix_eval::Value::String(mode)) => match mode.as_str() {
        "handoff" => KernelClassifierMode::Handoff,
        "default" => KernelClassifierMode::Default,
        _ => {
          return Err(anyhow!(
            "invalid 'classifier-mode' for standalone pnix query document"
          ))
        }
      },
      Some(_) => {
        return Err(anyhow!(
          "'classifier-mode' must be string in standalone pnix query document"
        ))
      }
      None => KernelClassifierMode::Default,
    };
    Ok(Self {
      utterance,
      scope,
      seeded_term,
      classifier_mode,
    })
  }
}

fn required_px_string<'a>(root: &'a BTreeMap<String, PxValue>, field: &str) -> Result<&'a str> {
  match root.get(field) {
    Some(PxValue::String(value)) => Ok(value.as_str()),
    Some(_) => Err(anyhow!(
      "'{}' must be string in standalone pnix query document",
      field
    )),
    None => Err(err_missing_standalone_field(field)),
  }
}

fn required_eval_string<'a>(
  root: &'a BTreeMap<String, pnix_eval::Value>,
  field: &str,
) -> Result<&'a str> {
  match root.get(field) {
    Some(pnix_eval::Value::String(value)) => Ok(value.as_str()),
    Some(_) => Err(anyhow!(
      "'{}' must be string in standalone pnix query document",
      field
    )),
    None => Err(err_missing_standalone_field(field)),
  }
}

fn validate_query_kind(query_kind: &str) -> Result<()> {
  if query_kind != "ontology-query" {
    return Err(anyhow!(
      "unsupported query kind '{query_kind}' — standalone pnix kernel accepts only ontology-query"
    ));
  }
  Ok(())
}

fn parse_output_scope(scope: &str) -> Result<OutputScope> {
  match scope {
    "brief" => Ok(OutputScope::Brief),
    "standard" => Ok(OutputScope::Standard),
    "detailed" => Ok(OutputScope::Detailed),
    _ => Err(anyhow!(
      "invalid 'scope' for standalone pnix query document"
    )),
  }
}

#[derive(Debug, Clone)]
struct ConceptDefinition {
  term_ko: String,
  definition_ko: String,
  formal_symbol: Option<String>,
  context: String,
  domain: String,
  related_concepts: Option<Vec<String>>,
  formula: Option<String>,
  why: Option<String>,
  boundary_conditions: Option<String>,
  source_ref: String,
  scalar_fields: BTreeMap<String, String>,
  list_fields: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
struct PredicateClassifier {
  rule: TextMatchRule,
  predicate: String,
  label_ko: String,
}

#[derive(Debug, Clone)]
struct HandoffClassifier {
  template_id: String,
  tags: Vec<String>,
  execution_owner: String,
  visibility: String,
  match_any: Vec<String>,
  match_terms: Vec<String>,
  match_units: Vec<String>,
  /// .px 가 직접 명시한 route name. 있으면 hardcoded `handoff_route` fn 의
  /// `is_os_owner|recipe-tag` 분기보다 우선시된다. pnix CLAUDE.md §15/§18 정렬:
  /// 새 의미 분기 (어느 utterance → 어느 route) 가 Rust hardcoded match 가 아니라
  /// .px owner data 에서 결정된다.
  handoff_route: Option<String>,
}

#[derive(Debug, Clone)]
struct DomainClassifier {
  keyword: String,
  domain: String,
}

#[derive(Debug, Clone)]
struct ContinuationClassifier {
  kind: String,
  match_any: Vec<String>,
  match_all_pairs: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Default)]
struct QueryClassifierConfig {
  handoff_classifiers: Vec<HandoffClassifier>,
  predicate_classifiers: Vec<PredicateClassifier>,
  domain_classifiers: Vec<DomainClassifier>,
  continuation_classifiers: Vec<ContinuationClassifier>,
  cross_concept_markers: Vec<String>,
  domain_listing_trigger_markers: Vec<String>,
  domain_list_intent_markers: Vec<String>,
  os_execution_owner_markers: Vec<String>,
  query_dispatch_priority: Vec<String>,
  definition_query_rules: Vec<TextMatchRule>,
  dispatch_routes: DispatchRouteConfig,
  held_reason_keys: HeldReasonConfig,
  held_reason_rules: Vec<HeldReasonRule>,
  source_fact_fields: Vec<SourceFactFieldRule>,
  source_list_fields: Vec<SourceFactListRule>,
  source_metadata: SourceFactMetadataConfig,
  concept_what_markers: Vec<String>,
  concept_definition_suffixes: Vec<String>,
  concept_explain_markers: Vec<String>,
  concept_explain_skip_tokens: Vec<String>,
  question_word_stems: Vec<String>,
  term_extraction_suffixes: Vec<String>,
  term_extraction_particle_kinds: Vec<String>,
  term_normalization_trim_chars: Vec<String>,
  term_fallback_policy: String,
}

#[derive(Debug, Clone, Default)]
struct KoreanMorphologyConfig {
  clause_connectors: Vec<ClauseConnector>,
  quotation_markers: Vec<String>,
  continuation_response_templates: BTreeMap<String, String>,
  recipe_command_strip_words: Vec<String>,
  recipe_shell_command_template: String,
  os_recipe_summary_template: String,
  light_handoff_summary_template: String,
}

#[derive(Debug, Clone, Default)]
struct ClauseConnector {
  connector: String,
  relation: String,
  label_ko: String,
}

#[derive(Debug, Clone)]
struct SourceFactFieldRule {
  field: String,
  predicate: String,
  context: Option<String>,
  layer: String,
  status: MeaningStatus,
  confidence: f64,
  object_template: String,
}

#[derive(Debug, Clone)]
struct SourceFactListRule {
  field: String,
  predicate: String,
  context: Option<String>,
  layer: String,
  status: MeaningStatus,
  confidence: f64,
  object_template: String,
}

#[derive(Debug, Clone)]
struct SourceFactMetadataConfig {
  context: String,
  layer: String,
  status: MeaningStatus,
  confidence: f64,
  field_predicate: String,
  value_predicate: String,
  list_field_predicate: String,
  list_item_predicate: String,
  field_object_template: String,
  value_object_template: String,
  list_field_object_template: String,
  list_item_object_template: String,
}

impl Default for SourceFactMetadataConfig {
  fn default() -> Self {
    Self {
      context: "Pnix.KernelSource".to_string(),
      layer: "L1".to_string(),
      status: MeaningStatus::Accepted,
      confidence: 1.0,
      field_predicate: "kernel-source-field".to_string(),
      value_predicate: "kernel-source-value".to_string(),
      list_field_predicate: "kernel-source-list-field".to_string(),
      list_item_predicate: "kernel-source-list-item".to_string(),
      field_object_template: "${field}".to_string(),
      value_object_template: "${field}=${value}".to_string(),
      list_field_object_template: "${field}".to_string(),
      list_item_object_template: "${field}=${value}".to_string(),
    }
  }
}

#[derive(Debug, Clone, Default)]
struct TextMatchRule {
  match_any: Option<Vec<String>>,
  match_all: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default)]
struct DispatchRouteConfig {
  definition: String,
  property: String,
  held: String,
}

#[derive(Debug, Clone, Default)]
struct HeldReasonConfig {
  requires_context: String,
  unknown_term: String,
}

#[derive(Debug, Clone, Default)]
struct HeldReasonRule {
  when: String,
  reason_key: String,
  term_source: String,
}

#[derive(Debug, Clone, Default)]
struct QueryRouteDefaults {
  query_context_rewrite_rules: Vec<PrefixRewriteRule>,
}

#[derive(Debug, Clone, Default)]
struct PrefixRewriteRule {
  from: String,
  to: String,
}

#[derive(Debug, Clone, Default)]
struct FollowupConfig {
  disambiguation_questions: BTreeMap<String, DisambiguationQuestion>,
  reason_question_rules: BTreeMap<String, String>,
  reopen_rules: BTreeMap<String, ReopenRule>,
  concept_choices: BTreeMap<String, Vec<String>>,
  choice_rules: Vec<ChoiceRule>,
  resolved_term_rules: Vec<ResolvedTermRule>,
  held_response_rules: Vec<HeldResponseRule>,
  default_choices: Vec<String>,
  unknown_term_label: String,
}

#[derive(Debug, Clone, Default)]
struct ReopenRule {
  carry_term_policy: String,
  effective_utterance_template: String,
}

#[derive(Debug, Clone, Default)]
struct ChoiceRule {
  when: String,
  choice_source: String,
}

#[derive(Debug, Clone, Default)]
struct ResolvedTermRule {
  when: String,
  term_source: String,
  value: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct HeldResponseRule {
  when: String,
  template: String,
  emit_held_term: bool,
}

#[derive(Debug, Clone, Default)]
struct DisambiguationQuestion {
  question_template: String,
  choices_template: String,
}

#[derive(Debug, Clone)]
struct InvertTrigger {
  pattern: String,
  trigger_type: String,
  truth_regime: String,
  priority: i64,
}

#[derive(Debug, Clone)]
struct InvertCandidateRule {
  trigger_type: String,
  concept_field: Option<String>,
  predicate: String,
  context: String,
  obj_template: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct InvertInterpretationRule {
  trigger_type: String,
  direct_fact_predicates: Vec<String>,
  source_include_predicates: Vec<String>,
  source_include_context_prefixes: Vec<String>,
  direct_interpretation_id: String,
  rich_interpretation_id: String,
}

#[derive(Debug, Clone, Default)]
struct InvertConfig {
  trigger_selection: String,
  route_template: String,
  default_truth_regime: String,
  default_direct_fact_predicates: Vec<String>,
  default_source_include_predicates: Vec<String>,
  default_source_include_context_prefixes: Vec<String>,
  default_direct_interpretation_id: String,
  default_rich_interpretation_id: String,
  triggers: Vec<InvertTrigger>,
  domain_to_regime: Vec<(String, String)>,
  candidate_rules: Vec<InvertCandidateRule>,
  interpretation_rules: Vec<InvertInterpretationRule>,
  resolved_interpretation_rules: BTreeMap<String, InvertInterpretationRule>,
}

#[derive(Debug, Clone, Default)]
struct DialogueTemplates {
  definition_section: TemplateSection,
  why_section: TemplateSection,
  property_section: TemplateSection,
  route_summary_definition: String,
  route_summary_property: String,
  route_summary_why: String,
  route_summary_held: String,
  // batch 74 (2026-04-15): predicate-specific empty response override.
  // doghouse store ConceptPredicateQuery::empty_response 와의 sentence-level
  // parity 용. key = predicate (e.g., "unit-ko"), value = ${term} 치환 템플릿.
  property_empty_by_predicate: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default)]
struct TemplateSection {
  join_with: String,
  suffix: String,
  parts: Vec<ConditionalTemplatePart>,
}

#[derive(Debug, Clone, Default)]
struct ConditionalTemplatePart {
  when: String,
  template: String,
  field_non_empty: Option<String>,
  list_non_empty: Option<String>,
  scope_is: Option<String>,
  values_state: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct KernelRouteRuntimeRule {
  direct_fact_predicates: Option<Vec<String>>,
  direct_interpretation_id: Option<String>,
  rich_interpretation_id: Option<String>,
  interpretation_id: Option<String>,
}

impl KernelRouteRuntimeRule {
  fn definition_direct_fact_predicates<'a>(&'a self, route: &str) -> Result<&'a Vec<String>> {
    self.direct_fact_predicates.as_ref().ok_or_else(|| {
      anyhow!(
        "definition route '{}' runtime rule missing direct fact predicates",
        route
      )
    })
  }

  fn definition_direct_interpretation_id<'a>(&'a self, route: &str) -> Result<&'a str> {
    self.direct_interpretation_id.as_deref().ok_or_else(|| {
      anyhow!(
        "definition route '{}' runtime rule missing direct interpretation id",
        route
      )
    })
  }

  fn definition_rich_interpretation_id<'a>(&'a self, route: &str) -> Result<&'a str> {
    self.rich_interpretation_id.as_deref().ok_or_else(|| {
      anyhow!(
        "definition route '{}' runtime rule missing rich interpretation id",
        route
      )
    })
  }

  fn property_interpretation_id<'a>(&'a self, route: &str) -> Result<&'a str> {
    self.interpretation_id.as_deref().ok_or_else(|| {
      anyhow!(
        "property route '{}' runtime rule missing interpretation id",
        route
      )
    })
  }
}

#[derive(Debug, Clone)]
struct RuntimeResources {
  concepts_by_term: BTreeMap<String, Vec<ConceptDefinition>>,
  synonyms: BTreeMap<String, String>,
  korean_morphology: KoreanMorphologyConfig,
  query_classifiers: QueryClassifierConfig,
  query_routes: BTreeMap<String, QueryRouteSpec>,
  route_runtime_rules: BTreeMap<String, KernelRouteRuntimeRule>,
  followups: FollowupConfig,
  invert: InvertConfig,
  dialogue_templates: DialogueTemplates,
  base_query_fact_rules: Vec<KernelBaseFactRule>,
  concept_source_fact_templates: ConceptSourceFactTemplates,
  note_templates: NoteTemplates,
  query_provenance_templates: QueryProvenanceTemplates,
  semantic_id_templates: SemanticIdTemplates,
  pipeline_trace_note_prefixes: Vec<String>,
  transcript_note_prefix: String,
  output_fragment_templates: OutputFragmentTemplates,
  response_document_schema: ResponseDocumentSchema,
}

impl RuntimeResources {
  fn load(paths: &KernelPaths) -> Result<Self> {
    let concepts_by_term = load_concepts(&paths.concepts_dir)?;
    let korean_morphology = load_korean_morphology(&paths.korean_morphology_path)?;
    let query_route_defaults = load_query_route_defaults(&paths.query_route_defaults_path)?;
    let (query_routes, route_runtime_rules) =
      load_query_routes(&paths.query_routes_path, &query_route_defaults)?;
    let query_classifiers = load_query_classifiers(&paths.query_classifiers_path)?;
    validate_required_route_runtime_rules(&query_classifiers, &route_runtime_rules)?;
    let synonyms = load_synonyms(&paths.synonyms_path)?;
    let followups = load_followups(&paths.followup_generation_path)?;
    let invert = load_invert_config(&paths.ontology_invert_path)?;
    let dialogue_templates = load_dialogue_templates(&paths.dialogue_templates_path)?;
    let (
      base_query_fact_rules,
      concept_source_fact_templates,
      note_templates,
      query_provenance_templates,
      semantic_id_templates,
      pipeline_trace_note_prefixes,
      transcript_note_prefix,
      output_fragment_templates,
      response_document_schema,
    ) = load_kernel_base_facts(&paths.kernel_base_facts_path)?;
    validate_known_concept_field_references(
      &concepts_by_term,
      &query_classifiers,
      &invert,
      &dialogue_templates,
    )?;
    Ok(Self {
      concepts_by_term,
      synonyms,
      korean_morphology,
      query_classifiers,
      query_routes,
      route_runtime_rules,
      followups,
      invert,
      dialogue_templates,
      base_query_fact_rules,
      concept_source_fact_templates,
      note_templates,
      query_provenance_templates,
      semantic_id_templates,
      pipeline_trace_note_prefixes,
      transcript_note_prefix,
      output_fragment_templates,
      response_document_schema,
    })
  }
}

#[derive(Debug, Clone)]
struct KernelBaseFactRule {
  id_template: String,
  when_route: Option<String>,
  repeat_over: Option<KernelBaseFactRepeatOver>,
  context: String,
  subj: String,
  pred: Option<String>,
  pred_template: Option<String>,
  obj_template: Option<String>,
  obj_literal: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KernelBaseFactRepeatOver {
  PropertyValues,
}

#[derive(Debug, Clone)]
struct ConceptSourceFactTemplates {
  scalar_id_template: String,
  list_id_template: String,
  provenance_template: String,
}

#[derive(Debug, Clone)]
struct NoteTemplates {
  transcript_user: String,
  transcript_pnix: String,
  held_reopen_reason: String,
  held_reopen_term: String,
  held_reason: String,
  held_term: String,
  invert_trigger: String,
  truth_regime: String,
  predicate_query: String,
}

#[derive(Debug, Clone)]
struct QueryProvenanceTemplates {
  utterance: String,
  concept_source: String,
}

#[derive(Debug, Clone)]
struct SemanticIdTemplates {
  episode_id_template: String,
  record_id_template: String,
  knowledge_id_template: String,
  knowledge_summary: String,
}

#[derive(Debug, Clone)]
struct OutputFragmentTemplate {
  kind: String,
  visibility: String,
}

#[derive(Debug, Clone)]
struct OutputFragmentTemplates {
  pipeline_trace: OutputFragmentTemplate,
  response_document: OutputFragmentTemplate,
}

#[derive(Debug, Clone)]
struct ResponseDocumentSchema {
  px_header_comment: String,
  px_field_episode_id: String,
  px_field_summary: String,
  px_field_transcript: String,
  px_field_pipeline: String,
  px_field_facts_count: String,
  org_title: String,
  org_pipeline_section_header: String,
  org_facts_count_template: String,
  org_transcript_transforms: Vec<OrgTranscriptTransform>,
}

#[derive(Debug, Clone)]
struct OrgTranscriptTransform {
  input_prefix: String,
  output_prefix: String,
}

#[derive(Debug, Clone)]
enum QueryDispatch {
  Definition {
    term: String,
  },
  Handoff(KernelHandoffQuery),
  Continuation {
    term: String,
    kind: KernelContinuationKind,
  },
  SentenceAnalysis,
  CrossConcept {
    term_a: String,
    term_b: String,
  },
  DomainListing {
    domain: String,
  },
  Property {
    term: String,
    predicate: String,
    label_ko: String,
  },
  Why {
    term: String,
    trigger_type: String,
    truth_regime: String,
  },
  Held {
    term: Option<String>,
    reason: String,
  },
}

#[derive(Debug, Clone)]
struct KernelHandoffQuery {
  template_id: String,
  tags: Vec<String>,
  execution_owner: String,
  visibility: String,
  /// .px-first override. classifier 의 `handoff-route` field 가 있으면 carry.
  /// `answer_handoff` 가 hardcoded `handoff_route` fn 보다 우선 사용한다.
  handoff_route: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum KernelContinuationKind {
  Elaborate,
  Example,
  Related,
}

impl KernelContinuationKind {
  fn as_str(self) -> &'static str {
    match self {
      Self::Elaborate => "elaborate",
      Self::Example => "example",
      Self::Related => "related",
    }
  }

  fn route(self) -> &'static str {
    match self {
      Self::Elaborate => "concept-continuation-elaborate",
      Self::Example => "concept-continuation-example",
      Self::Related => "concept-continuation-related",
    }
  }
}

fn classify_query(
  resources: &RuntimeResources,
  utterance: &str,
  seeded_term: Option<&str>,
  classifier_mode: KernelClassifierMode,
) -> Result<QueryDispatch> {
  let mut extracted_terms = extract_candidate_terms(resources, utterance);
  if let Some(seed) = seeded_term {
    if resources.concepts_by_term.contains_key(seed)
      && !extracted_terms.iter().any(|term| term == seed)
    {
      extracted_terms.insert(0, seed.to_string());
    }
  }
  let term = extracted_terms
    .iter()
    .find(|term| resources.concepts_by_term.contains_key(term.as_str()))
    .cloned();
  let continuation_dispatch = seeded_term
    .filter(|term| resources.concepts_by_term.contains_key(*term))
    .and_then(|term| {
      classify_continuation(resources, utterance).map(|kind| QueryDispatch::Continuation {
        term: term.to_string(),
        kind,
      })
    });

  if classifier_mode == KernelClassifierMode::Handoff {
    if let Some(dispatch) = classify_handoff(resources, utterance) {
      return Ok(QueryDispatch::Handoff(dispatch));
    }
  }

  if let Some(dispatch) = continuation_dispatch {
    return Ok(dispatch);
  }

  let why_dispatch = if let (Some(term), Some(trigger)) =
    (term.clone(), best_invert_trigger(resources, utterance))
  {
    let truth_regime = resolve_truth_regime(resources, &term, trigger);
    Some(QueryDispatch::Why {
      term,
      trigger_type: trigger.trigger_type.clone(),
      truth_regime,
    })
  } else {
    None
  };

  let property_dispatch = if let Some(term) = term.clone() {
    matching_property_classifier(resources, utterance).map(|predicate| QueryDispatch::Property {
      term,
      predicate: predicate.predicate.clone(),
      label_ko: predicate.label_ko.clone(),
    })
  } else {
    None
  };

  let definition_dispatch = term
    .clone()
    .filter(|_| looks_like_definition_query(resources, utterance))
    .map(|term| QueryDispatch::Definition { term });
  let cross_concept_dispatch = classify_cross_concept_query(resources, utterance)
    .map(|(term_a, term_b)| QueryDispatch::CrossConcept { term_a, term_b });
  let domain_listing_dispatch = classify_domain_listing(resources, utterance)
    .map(|domain| QueryDispatch::DomainListing { domain });
  let sentence_analysis_dispatch = looks_like_sentence_analysis_query(resources, utterance)
    .then_some(QueryDispatch::SentenceAnalysis);

  for stage in query_dispatch_priority(resources) {
    match stage {
      "why" => {
        if let Some(dispatch) = why_dispatch.clone() {
          return Ok(dispatch);
        }
      }
      "property" => {
        if let Some(dispatch) = property_dispatch.clone() {
          return Ok(dispatch);
        }
      }
      "definition" => {
        if let Some(dispatch) = cross_concept_dispatch.clone() {
          return Ok(dispatch);
        }
        if let Some(dispatch) = definition_dispatch.clone() {
          return Ok(dispatch);
        }
      }
      other => unreachable!("validated query-dispatch stage must be canonical, got {other}"),
    }
  }
  if let Some(dispatch) = domain_listing_dispatch {
    return Ok(dispatch);
  }
  if let Some(dispatch) = sentence_analysis_dispatch {
    return Ok(dispatch);
  }

  let (held_term, held_reason) = resolve_held_dispatch(
    resources,
    term.as_deref(),
    extracted_terms.first().map(String::as_str),
  )?;
  Ok(QueryDispatch::Held {
    term: held_term,
    reason: held_reason,
  })
}

fn query_dispatch_priority(resources: &RuntimeResources) -> Vec<&str> {
  resources
    .query_classifiers
    .query_dispatch_priority
    .iter()
    .map(String::as_str)
    .collect()
}

fn dispatch_route_definition(resources: &RuntimeResources) -> &str {
  resources
    .query_classifiers
    .dispatch_routes
    .definition
    .as_str()
}

fn dispatch_route_property(resources: &RuntimeResources) -> &str {
  resources
    .query_classifiers
    .dispatch_routes
    .property
    .as_str()
}

fn dispatch_route_why(resources: &RuntimeResources, trigger_type: &str) -> String {
  resources
    .invert
    .route_template
    .replace("${trigger_type}", trigger_type)
}

fn dispatch_route_held(resources: &RuntimeResources) -> &str {
  resources.query_classifiers.dispatch_routes.held.as_str()
}

fn handoff_classifier_matches(rule: &HandoffClassifier, utterance: &str) -> bool {
  if rule
    .match_any
    .iter()
    .any(|value| utterance.contains(value.as_str()))
  {
    return true;
  }
  if !rule.match_terms.is_empty() && !rule.match_units.is_empty() {
    let has_term = rule
      .match_terms
      .iter()
      .any(|value| utterance.contains(value.as_str()));
    let has_unit = rule
      .match_units
      .iter()
      .any(|value| utterance.contains(value.as_str()));
    return has_term && has_unit;
  }
  false
}

fn classify_handoff(resources: &RuntimeResources, utterance: &str) -> Option<KernelHandoffQuery> {
  resources
    .query_classifiers
    .handoff_classifiers
    .iter()
    .find(|rule| handoff_classifier_matches(rule, utterance))
    .map(|rule| KernelHandoffQuery {
      template_id: rule.template_id.clone(),
      tags: rule.tags.clone(),
      execution_owner: rule.execution_owner.clone(),
      visibility: rule.visibility.clone(),
      handoff_route: rule.handoff_route.clone(),
    })
}

fn append_pnix_ontology_query_decision(
  facts: &mut Vec<ContextualFact>,
  notes: &mut Vec<String>,
  provenance: &[String],
  route: &str,
  decision: &judgment::OntologyQueryDecision,
  events: Option<&mut DecisionEvents>,
) {
  if let Some(events) = events {
    events.push_decision(route, decision, provenance);
  }
  let provenance = provenance.to_vec();
  facts.push(make_candidate_fact(
    format!("fact.pnix.ontology.lift.{route}"),
    "Pnix.OntologyQuery",
    "pnix",
    "ontology-lift-id",
    decision.lift.id.clone(),
    provenance.clone(),
  ));
  facts.push(make_candidate_fact(
    format!("fact.pnix.ontology.context.{route}"),
    "Pnix.OntologyQuery",
    "pnix",
    "ontology-query-context",
    decision.lift.to_context.0.clone(),
    provenance.clone(),
  ));
  facts.push(make_candidate_fact(
    format!("fact.pnix.ontology.evaluations.{route}"),
    "Pnix.OntologyQuery",
    "pnix",
    "ontology-evaluation-count",
    decision.evaluations.len().to_string(),
    provenance.clone(),
  ));
  for (index, evaluation) in decision.evaluations.iter().enumerate().take(4) {
    facts.push(make_candidate_fact(
      format!("fact.pnix.ontology.eval.{route}.{index}.interpretation"),
      "Pnix.OntologyQuery",
      "pnix",
      "ontology-evaluation-interpretation",
      evaluation.interpretation.0.clone(),
      provenance.clone(),
    ));
    facts.push(make_candidate_fact(
      format!("fact.pnix.ontology.eval.{route}.{index}.score"),
      "Pnix.OntologyQuery",
      "pnix",
      "ontology-evaluation-score-candidate",
      format!("{:.4}", evaluation.score),
      provenance.clone(),
    ));
  }
  facts.push(make_candidate_fact(
    format!("fact.pnix.ontology.selected.{route}"),
    "Pnix.OntologyQuery",
    "pnix",
    "ontology-selected-interpretation",
    decision
      .selection
      .judgement
      .chosen_interpretation
      .as_ref()
      .map(|id| id.0.clone())
      .unwrap_or_else(|| "none".to_string()),
    provenance.clone(),
  ));
  facts.push(make_candidate_fact(
    format!("fact.pnix.ontology.score.{route}"),
    "Pnix.OntologyQuery",
    "pnix",
    "ontology-evaluation-score",
    format!("{:.4}", decision.selection.evaluation.score),
    provenance.clone(),
  ));
  facts.push(make_candidate_fact(
    format!("fact.pnix.ontology.action.{route}"),
    "Pnix.OntologyQuery",
    "pnix",
    "ontology-judgement-action",
    format!("{:?}", decision.selection.judgement.action),
    provenance.clone(),
  ));
  facts.push(make_candidate_fact(
    format!("fact.pnix.ontology.promotion.{route}"),
    "Pnix.OntologyQuery",
    "pnix",
    "ontology-promotion-status",
    format!("{:?}", decision.promotion.target_status),
    provenance.clone(),
  ));
  notes.push(format!(
    "ontology-lift:route:{route}:context:{}:facts:{}",
    decision.lift.to_context.0,
    decision.lifted_facts.len()
  ));
  notes.push(format!(
    "ontology-evaluate:route:{route}:candidates:{}",
    decision.evaluations.len()
  ));
  let ev = &decision.selection.evaluation;
  notes.push(format!(
    "ontology-evaluation-axes:coherence={:.2}:coverage={:.2}:loss={:.2}:cost={:.2}:replayability={:.2}:safety={:.2}:score={:.4}",
    ev.coherence, ev.coverage, ev.loss_penalty, ev.cost, ev.replayability, ev.safety, ev.score
  ));
  notes.push(format!(
    "ontology-select:route:{route}:interpretation:{}",
    decision
      .selection
      .judgement
      .chosen_interpretation
      .as_ref()
      .map(|id| id.0.as_str())
      .unwrap_or("none")
  ));
  notes.push(format!(
    "ontology-promote:route:{route}:status:{:?}",
    decision.promotion.target_status
  ));
}

fn held_reason_value<'a>(resources: &'a RuntimeResources, reason_key: &'a str) -> &'a str {
  match reason_key {
    "requires-context" => resources
      .query_classifiers
      .held_reason_keys
      .requires_context
      .as_str(),
    "unknown-term" => resources
      .query_classifiers
      .held_reason_keys
      .unknown_term
      .as_str(),
    other => unreachable!("validated held reason-key must be canonical, got {other}"),
  }
}

fn resolve_held_dispatch(
  resources: &RuntimeResources,
  matched_term: Option<&str>,
  first_extracted_term: Option<&str>,
) -> Result<(Option<String>, String)> {
  for rule in &resources.query_classifiers.held_reason_rules {
    let matches = match rule.when.as_str() {
      "known-term" => matched_term.is_some(),
      "unknown-term" => matched_term.is_none(),
      other => unreachable!("validated held-reason when must be canonical, got {other}"),
    };
    if !matches {
      continue;
    }
    let term = match rule.term_source.as_str() {
      "matched-term" => matched_term.map(str::to_string),
      "first-extracted-term" => first_extracted_term.map(str::to_string),
      "none" => None,
      other => unreachable!("validated held-reason term-source must be canonical, got {other}"),
    };
    let reason = held_reason_value(resources, rule.reason_key.as_str()).to_string();
    return Ok((term, reason));
  }
  Err(anyhow!(
    "no held-reason-rules entry matched term state '{}'",
    term_presence_name(matched_term)
  ))
}

fn handoff_route(resources: &RuntimeResources, query: &KernelHandoffQuery) -> &'static str {
  let is_os_owner = resources
    .query_classifiers
    .os_execution_owner_markers
    .iter()
    .any(|marker| query.execution_owner.contains(marker.as_str()));
  if is_os_owner || query.tags.iter().any(|tag| tag == "recipe") {
    "recipe-os-handoff"
  } else {
    "lightweight-korean-dialogue-handoff"
  }
}

fn recipe_handoff_target(resources: &RuntimeResources, utterance: &str) -> String {
  let mut stripped = utterance.to_string();
  for word in &resources.korean_morphology.recipe_command_strip_words {
    stripped = stripped.replace(word.as_str(), "");
  }
  canonicalize_term(resources, stripped.trim())
}

fn recipe_shell_command(template_id: &str, target: &str) -> String {
  let is_macos = cfg!(target_os = "macos");
  match template_id {
    "pnix.recipe.app.launch" => {
      if is_macos {
        format!("open -a '{target}'")
      } else {
        format!("xdg-open '{target}'")
      }
    }
    "pnix.recipe.file.search" => {
      if is_macos {
        format!("mdfind '{target}' | head -10")
      } else {
        format!("find ~ -name '*{target}*' -maxdepth 4 2>/dev/null | head -10")
      }
    }
    "pnix.recipe.clipboard" => {
      if is_macos {
        "pbpaste".to_string()
      } else {
        "xclip -selection clipboard -o".to_string()
      }
    }
    "pnix.recipe.app.list" => {
      if is_macos {
        "ls /Applications/ | sed 's/.app$//' | head -20".to_string()
      } else {
        "ls /usr/share/applications/ | head -20".to_string()
      }
    }
    "pnix.recipe.process.list" => "ps aux | head -15".to_string(),
    "pnix.recipe.notify" => {
      if is_macos {
        format!("osascript -e 'display notification \"{target}\" with title \"doghouse\"'")
      } else {
        format!("notify-send 'doghouse' '{target}'")
      }
    }
    _ => String::new(),
  }
}

fn answer_handoff(
  resources: &RuntimeResources,
  request: &PnixQueryRequest,
  query: &KernelHandoffQuery,
  reopened: Option<&HeldState>,
) -> Result<KernelResponse> {
  // .px-first route resolution: classifier entry 의 `handoff-route` field 가
  // 있으면 그것을 우선 사용. 없으면 hardcoded fallback (recipe-os 또는
  // lightweight-korean-dialogue) 으로. 기존 entries 는 fallback 으로 backward-compat.
  let route_string = query
    .handoff_route
    .clone()
    .unwrap_or_else(|| handoff_route(resources, query).to_string());
  let route = route_string.as_str();
  let provenance = vec![fill_template(
    resources.query_provenance_templates.utterance.as_str(),
    &[("${utterance}", request.utterance.as_str())],
  )];
  let mut facts = base_query_facts(
    resources,
    &request.utterance,
    route,
    request.scope,
    &provenance,
    &[],
    &[],
  );
  facts.push(make_candidate_fact(
    format!("fact.pnix.handoff.template.{}", query.template_id),
    "Pnix.Handoff",
    "pnix",
    "handoff-template",
    query.template_id.clone(),
    provenance.clone(),
  ));
  facts.push(make_candidate_fact(
    format!("fact.pnix.handoff.owner.{}", query.template_id),
    "Pnix.Handoff",
    "pnix",
    "execution-owner",
    query.execution_owner.clone(),
    provenance.clone(),
  ));
  facts.push(make_candidate_fact(
    format!("fact.pnix.handoff.visibility.{}", query.template_id),
    "Pnix.Handoff",
    "pnix",
    "handoff-visibility",
    query.visibility.clone(),
    provenance.clone(),
  ));
  for (index, tag) in query.tags.iter().enumerate() {
    facts.push(make_candidate_fact(
      format!("fact.pnix.handoff.tag.{}.{}", query.template_id, index),
      "Pnix.Handoff",
      "pnix",
      "handoff-tag",
      tag.clone(),
      provenance.clone(),
    ));
  }

  let (summary, response_text, extra_notes) = if route == "recipe-os-handoff" {
    let target = recipe_handoff_target(resources, &request.utterance);
    let shell_cmd = recipe_shell_command(&query.template_id, &target);
    if !target.is_empty() {
      facts.push(make_candidate_fact(
        format!("fact.pnix.handoff.target.{}", query.template_id),
        "Pnix.Handoff",
        "pnix",
        "handoff-target",
        target.clone(),
        provenance.clone(),
      ));
    }
    let summary = fill_template(
      resources
        .korean_morphology
        .os_recipe_summary_template
        .as_str(),
      &[("${template-id}", query.template_id.as_str())],
    );
    let response_text = if shell_cmd.is_empty() {
      summary.clone()
    } else {
      fill_template(
        resources
          .korean_morphology
          .recipe_shell_command_template
          .as_str(),
        &[
          ("${summary}", summary.as_str()),
          ("${shell-cmd}", shell_cmd.as_str()),
        ],
      )
    };
    let mut notes = Vec::new();
    if !shell_cmd.is_empty() {
      notes.push(format!("tool-handoff:code:{shell_cmd}"));
      notes.push("tool-handoff:runtime:shell".to_string());
    }
    let payload = format!(
      "{{\n  kind = \"pnix-recipe-handoff\";\n  template = \"{}\";\n  execution_owner = \"{}\";\n  command = \"{}\";\n  target = \"{}\";\n}}",
      query.template_id,
      query.execution_owner,
      shell_cmd.replace('"', "'"),
      target.replace('"', "'")
    );
    notes.push(format!("handoff:concrete-px-payload:{payload}"));
    (summary, response_text, notes)
  } else {
    let summary = resources
      .korean_morphology
      .light_handoff_summary_template
      .clone();
    (summary.clone(), summary, Vec::new())
  };

  let mut notes = base_notes(resources, &request.utterance, &response_text, reopened);
  notes.push("handoff:source:pnix-kernel".to_string());
  notes.push(format!("handoff:template:{}", query.template_id));
  notes.push(format!("handoff:execution-owner:{}", query.execution_owner));
  notes.push(format!("handoff:visibility:{}", query.visibility));
  notes.extend(extra_notes);
  finalize_response(
    resources,
    &request.utterance,
    route,
    summary,
    notes,
    facts,
    None,
    None,
    DecisionEvents::default(),
  )
}

fn answer_definition(
  resources: &RuntimeResources,
  request: &PnixQueryRequest,
  term: &str,
  reopened: Option<&HeldState>,
) -> Result<KernelResponse> {
  let route = dispatch_route_definition(resources);
  let route_spec = route_spec(resources, route)?;
  let runtime_rule = required_definition_route_runtime_rule(resources, route)?;
  let concepts = concepts_for_term(resources, term)?;
  let concept = concepts
    .first()
    .ok_or_else(|| anyhow!("no concept found for {term}"))?;
  let provenance = vec![
    fill_template(
      resources.query_provenance_templates.utterance.as_str(),
      &[("${utterance}", request.utterance.as_str())],
    ),
    fill_template(
      resources.query_provenance_templates.concept_source.as_str(),
      &[("${source-ref}", concept.source_ref.as_str())],
    ),
  ];
  let formal_name_en = concept
    .scalar_fields
    .get("formal-name-en")
    .map(String::as_str)
    .unwrap_or("");
  let mut facts = base_query_facts_with_extra(
    resources,
    &request.utterance,
    route,
    request.scope,
    &provenance,
    &[
      ("${term}", term),
      ("${domain}", concept.domain.as_str()),
      ("${definition-ko}", concept.definition_ko.as_str()),
      ("${formal-name-en}", formal_name_en),
    ],
    &[],
  );
  let source_facts = concept_source_facts(resources, concept);
  let direct_predicates = runtime_rule.definition_direct_fact_predicates(route)?;
  let direct_predicate_refs = direct_predicates
    .iter()
    .map(String::as_str)
    .collect::<Vec<_>>();
  let direct_refs = fact_ids_for_predicates(&source_facts, &direct_predicate_refs);
  let rich_refs = fact_ids(&source_facts);
  let interpretations = build_interpretations(
    route_interpretation_template(
      runtime_rule.definition_direct_interpretation_id(route)?,
      &[("${term}", term)],
    ),
    direct_refs,
    route_interpretation_template(
      runtime_rule.definition_rich_interpretation_id(route)?,
      &[("${term}", term)],
    ),
    rich_refs,
  );
  let response_text = definition_response_text(
    resources,
    &resources.dialogue_templates,
    concept,
    request.scope,
  )?;
  let mut notes = base_notes(resources, &request.utterance, &response_text, reopened);
  // batch 76 (2026-04-15): concept lane note parity. doghouse store 의
  // `build_concept_query_notes` (builders.rs L311-324) 가 emit 하는 두 note
  // 를 kernel 도 매칭. 향후 unified `.px` migration 에서 같이 닫는다.
  notes.push(format!("concept-lookup:term:{term}"));
  notes.push(format!("concept-lookup:domain:{}", concept.domain));
  let mut events = DecisionEvents::default();
  if let Some(decision) = judgment::ontology_query_decision(
    route,
    &route_spec,
    interpretations,
    &source_facts,
    Some(&JudgementIntent::with_intent_type(
      request.scope,
      "definition",
    )),
  ) {
    append_pnix_ontology_query_decision(
      &mut facts,
      &mut notes,
      &provenance,
      route,
      &decision,
      Some(&mut events),
    );
  }
  facts.extend(source_facts);
  finalize_response(
    resources,
    &request.utterance,
    route,
    resources
      .dialogue_templates
      .route_summary_definition
      .replace("${term}", term),
    notes,
    facts,
    None,
    None,
    events,
  )
}

fn answer_property(
  resources: &RuntimeResources,
  request: &PnixQueryRequest,
  term: &str,
  predicate: &str,
  label_ko: &str,
  reopened: Option<&HeldState>,
) -> Result<KernelResponse> {
  let route = dispatch_route_property(resources);
  let route_spec = route_spec(resources, route)?;
  let runtime_rule = required_property_route_runtime_rule(resources, route)?;
  let concepts = concepts_for_term(resources, term)?;
  let concept = concepts
    .first()
    .ok_or_else(|| anyhow!("no concept found for {term}"))?;
  let source_facts = concept_source_facts(resources, concept);
  let target_refs = fact_ids_for_predicates(&source_facts, &[predicate]);
  let values = source_facts
    .iter()
    .filter(|fact| fact.pred == predicate)
    .map(|fact| fact.obj.clone())
    .collect::<Vec<_>>();
  let response_text = property_response_text(
    &resources.dialogue_templates,
    term,
    label_ko,
    predicate,
    &values,
  )?;
  let provenance = vec![
    fill_template(
      resources.query_provenance_templates.utterance.as_str(),
      &[("${utterance}", request.utterance.as_str())],
    ),
    fill_template(
      resources.query_provenance_templates.concept_source.as_str(),
      &[("${source-ref}", concept.source_ref.as_str())],
    ),
  ];
  let mut facts = base_query_facts(
    resources,
    &request.utterance,
    route,
    request.scope,
    &provenance,
    &[
      ("${term}", term),
      ("${domain}", concept.domain.as_str()),
      ("${predicate}", predicate),
    ],
    &values,
  );
  let mut notes = base_notes(resources, &request.utterance, &response_text, reopened);
  notes.push(fill_template(
    resources.note_templates.predicate_query.as_str(),
    &[("${predicate}", predicate)],
  ));
  for (index, value) in values.iter().enumerate() {
    facts.push(make_candidate_fact(
      format!("fact.pnix.predicate.result.{index}"),
      "Pnix.PredicateQuery",
      "pnix",
      format!("predicate-result-{predicate}").as_str(),
      value.clone(),
      provenance.clone(),
    ));
  }
  let mut events = DecisionEvents::default();
  let interpretations = vec![judgment::interpretation_with_refs(
    route_interpretation_template(
      runtime_rule.property_interpretation_id(route)?,
      &[
        ("${term}", term),
        ("${predicate}", predicate),
        ("${label}", label_ko),
      ],
    ),
    target_refs,
  )];
  if let Some(decision) = judgment::ontology_query_decision(
    route,
    &route_spec,
    interpretations,
    &source_facts,
    Some(&JudgementIntent::with_intent_type(
      request.scope,
      "property",
    )),
  ) {
    append_pnix_ontology_query_decision(
      &mut facts,
      &mut notes,
      &provenance,
      route,
      &decision,
      Some(&mut events),
    );
  }
  facts.extend(source_facts);
  finalize_response(
    resources,
    &request.utterance,
    route,
    // batch 72 (2026-04-15): P5.5 step 4 envelope shaping parity — property lane.
    // `${label}` placeholder 를 predicate classifier label_ko 로 치환.
    // doghouse store 의 predicate envelope 은 summary 에 predicate label
    // (e.g., "단위", "공식") 을 포함한다. kernel 도 같은 contract 를 만족
    // 해야 향후 primary delegation 에서 test 회귀가 없다.
    resources
      .dialogue_templates
      .route_summary_property
      .replace("${term}", term)
      .replace("${label}", label_ko),
    notes,
    facts,
    None,
    None,
    events,
  )
}

fn answer_domain_listing(
  resources: &RuntimeResources,
  request: &PnixQueryRequest,
  domain: &str,
  reopened: Option<&HeldState>,
) -> Result<KernelResponse> {
  let route = "concept-domain-listing";
  let route_spec = route_spec(resources, route)?;
  let runtime_rule = route_runtime_rule(resources, route)
    .ok_or_else(|| err_missing_runtime("route runtime rule for domain listing route", route))?;
  let direct_interpretation_id = runtime_rule
    .direct_interpretation_id
    .as_deref()
    .ok_or_else(|| {
      anyhow!(
        "domain listing route '{}' runtime rule missing direct interpretation id",
        route
      )
    })?;
  let rich_interpretation_id = runtime_rule
    .rich_interpretation_id
    .as_deref()
    .ok_or_else(|| {
      anyhow!(
        "domain listing route '{}' runtime rule missing rich interpretation id",
        route
      )
    })?;
  let terms = terms_for_domain(resources, domain);
  if terms.is_empty() {
    return Err(anyhow!("no concept terms found for domain '{domain}'"));
  }

  let preview_limit = route_spec.default_preview.min(terms.len());
  let preview_terms = terms
    .iter()
    .take(preview_limit)
    .cloned()
    .collect::<Vec<_>>();
  let mut preview_defs = Vec::new();
  let mut ontology_source_facts = Vec::new();
  let mut direct_fact_refs = Vec::new();
  let mut rich_fact_refs = Vec::new();
  for term in &preview_terms {
    let concepts = concepts_for_term(resources, term)?;
    let concept = concepts
      .first()
      .ok_or_else(|| anyhow!("no concept found for {term}"))?;
    let concept_facts = concept_source_facts(resources, concept);
    for fact in &concept_facts {
      if !ontology_source_facts
        .iter()
        .any(|existing: &ContextualFact| existing.id == fact.id)
      {
        ontology_source_facts.push(fact.clone());
      }
      if let Some(id) = fact.id.as_ref() {
        if fact.pred == "domain" {
          direct_fact_refs.push(id.0.clone());
        }
        if matches!(fact.pred.as_str(), "domain" | "category" | "definition-ko") {
          rich_fact_refs.push(id.0.clone());
        }
      }
    }
    if request.scope == OutputScope::Detailed {
      if let Some(definition) = concept_facts
        .iter()
        .find(|fact| fact.pred == "definition-ko")
      {
        preview_defs.push(format!("{term}: {}", definition.obj));
      }
    }
  }
  direct_fact_refs.sort();
  direct_fact_refs.dedup();
  rich_fact_refs.sort();
  rich_fact_refs.dedup();

  let provenance = vec![
    fill_template(
      resources.query_provenance_templates.utterance.as_str(),
      &[("${utterance}", request.utterance.as_str())],
    ),
    format!("domain-listing:{domain}"),
  ];
  let mut response_parts = vec![format!(
    "{domain} 개념 {}개: {}.",
    terms.len(),
    terms.join(", ")
  )];
  if request.scope == OutputScope::Detailed && !preview_defs.is_empty() {
    response_parts.push(format!("대표 개념: {}", preview_defs.join(" ")));
  }
  let response_text = response_parts.join(" ");

  let mut facts = base_query_facts(
    resources,
    &request.utterance,
    route,
    request.scope,
    &provenance,
    &[],
    &[],
  );
  facts.push(make_candidate_fact(
    "fact.pnix.domain.name".to_string(),
    "Pnix.DomainListing",
    "user",
    "domain-query",
    domain.to_string(),
    provenance.clone(),
  ));
  facts.push(make_candidate_fact(
    "fact.pnix.domain.count".to_string(),
    "Pnix.DomainListing",
    "pnix",
    "domain-concept-count",
    terms.len().to_string(),
    provenance.clone(),
  ));
  facts.push(make_candidate_fact(
    "fact.pnix.domain.terms".to_string(),
    "Pnix.DomainListing",
    "pnix",
    "domain-concept-terms",
    terms.join(", "),
    provenance.clone(),
  ));
  for (index, term) in terms.iter().enumerate() {
    facts.push(make_candidate_fact(
      format!("fact.pnix.domain.term.{index}"),
      "Pnix.DomainListing",
      "pnix",
      "domain-concept-term",
      term.clone(),
      provenance.clone(),
    ));
  }
  let mut notes = base_notes(resources, &request.utterance, &response_text, reopened);
  notes.push(format!("domain-listing:domain:{domain}"));
  notes.push(format!("domain-listing:count:{}", terms.len()));
  let mut events = DecisionEvents::default();
  let mut interpretations = vec![judgment::interpretation_with_refs(
    route_interpretation_template(direct_interpretation_id, &[("${domain}", domain)]),
    direct_fact_refs.clone(),
  )];
  if rich_fact_refs.len() > direct_fact_refs.len() {
    interpretations.push(judgment::interpretation_with_refs(
      route_interpretation_template(rich_interpretation_id, &[("${domain}", domain)]),
      rich_fact_refs,
    ));
  }
  if let Some(decision) = judgment::ontology_query_decision(
    route,
    &route_spec,
    interpretations,
    &ontology_source_facts,
    Some(&JudgementIntent::with_intent_type(
      request.scope,
      "domain-listing",
    )),
  ) {
    append_pnix_ontology_query_decision(
      &mut facts,
      &mut notes,
      &provenance,
      route,
      &decision,
      Some(&mut events),
    );
  }
  finalize_response(
    resources,
    &request.utterance,
    route,
    format!(
      "온톨로지에서 {domain} 도메인의 개념 {}개를 조회했다.",
      terms.len()
    ),
    notes,
    facts,
    None,
    None,
    events,
  )
}

fn continuation_template<'a>(resources: &'a RuntimeResources, key: &str) -> Result<&'a str> {
  resources
    .korean_morphology
    .continuation_response_templates
    .get(key)
    .map(String::as_str)
    .ok_or_else(|| err_missing_runtime("continuation-response template", key))
}

fn answer_continuation(
  resources: &RuntimeResources,
  request: &PnixQueryRequest,
  term: &str,
  kind: KernelContinuationKind,
  reopened: Option<&HeldState>,
) -> Result<KernelResponse> {
  let route = kind.route();
  let route_spec = route_spec(resources, route)?;
  let concepts = concepts_for_term(resources, term)?;
  let concept = concepts
    .first()
    .ok_or_else(|| anyhow!("no concept found for {term}"))?;
  let source_facts = concept_source_facts(resources, concept);
  let formula = concept
    .scalar_fields
    .get("formula")
    .map(String::as_str)
    .unwrap_or("");
  let related_terms = concept
    .list_fields
    .get("related-concepts")
    .cloned()
    .unwrap_or_default();
  let preview_terms = related_terms
    .iter()
    .take(route_spec.default_preview.max(1))
    .cloned()
    .collect::<Vec<_>>();
  let mut ontology_source_facts = source_facts.clone();
  let mut related_definitions = Vec::new();
  for related_term in &preview_terms {
    let related_concept = concepts_for_term(resources, related_term)
      .ok()
      .and_then(|concepts| concepts.into_iter().next());
    let Some(related_concept) = related_concept else {
      continue;
    };
    let related_facts = concept_source_facts(resources, &related_concept);
    for fact in &related_facts {
      if !ontology_source_facts
        .iter()
        .any(|existing: &ContextualFact| existing.id == fact.id)
      {
        ontology_source_facts.push(fact.clone());
      }
    }
    if let Some(definition) = related_facts
      .iter()
      .find(|fact| fact.pred == "definition-ko")
    {
      related_definitions.push(format!("{related_term}: {}", definition.obj));
    }
  }

  let response_text = match kind {
    KernelContinuationKind::Elaborate => {
      let mut parts = vec![fill_template(
        continuation_template(resources, "elaborate-header")?,
        &[("${term}", term)],
      )];
      if !concept.definition_ko.is_empty() {
        parts.push(format!("정의: {}", concept.definition_ko));
      }
      if !formula.is_empty() {
        parts.push(format!("공식: {formula}"));
      }
      if let Some(boundary_conditions) = concept.boundary_conditions.as_deref() {
        if !boundary_conditions.is_empty() {
          parts.push(format!("조건: {boundary_conditions}"));
        }
      }
      parts.join(" ")
    }
    KernelContinuationKind::Example => {
      let mut parts = vec![fill_template(
        continuation_template(resources, "example-header")?,
        &[("${term}", term)],
      )];
      if !formula.is_empty() {
        parts.push(fill_template(
          continuation_template(resources, "example-formula")?,
          &[("${formula}", formula)],
        ));
      }
      if !concept.definition_ko.is_empty() {
        parts.push(fill_template(
          continuation_template(resources, "example-definition")?,
          &[("${definition}", concept.definition_ko.as_str())],
        ));
      }
      if !preview_terms.is_empty() {
        parts.push(fill_template(
          continuation_template(resources, "example-related")?,
          &[("${related}", preview_terms.join(", ").as_str())],
        ));
      }
      parts.join(" ")
    }
    KernelContinuationKind::Related => {
      if preview_terms.is_empty() {
        fill_template(
          continuation_template(resources, "related-empty")?,
          &[("${term}", term)],
        )
      } else {
        let mut parts = vec![fill_template(
          continuation_template(resources, "related-header")?,
          &[
            ("${term}", term),
            ("${list}", preview_terms.join(", ").as_str()),
          ],
        )];
        parts.extend(related_definitions);
        parts.join(" ")
      }
    }
  };

  let provenance = vec![
    fill_template(
      resources.query_provenance_templates.utterance.as_str(),
      &[("${utterance}", request.utterance.as_str())],
    ),
    fill_template(
      resources.query_provenance_templates.concept_source.as_str(),
      &[("${source-ref}", concept.source_ref.as_str())],
    ),
    format!("continuation-from:{term}"),
  ];
  let mut facts = base_query_facts(
    resources,
    &request.utterance,
    route,
    request.scope,
    &provenance,
    &[],
    &[],
  );
  facts.push(make_candidate_fact(
    format!("fact.pnix.continuation.kind.{}", kind.as_str()),
    "Pnix.Continuation",
    "pnix",
    "continuation-kind",
    kind.as_str().to_string(),
    provenance.clone(),
  ));
  facts.push(make_candidate_fact(
    format!("fact.pnix.continuation.from.{term}"),
    "Pnix.Continuation",
    "pnix",
    "continuation-from-term",
    term.to_string(),
    provenance.clone(),
  ));

  let mut notes = base_notes(resources, &request.utterance, &response_text, reopened);
  notes.push("continuation:source:pnix-kernel".to_string());
  notes.push(format!("continuation:kind:{}", kind.as_str()));
  notes.push(format!("continuation:from-term:{term}"));
  let mut events = DecisionEvents::default();
  let direct_fact_refs = fact_ids_for_predicates(
    &ontology_source_facts,
    &[
      "definition-ko",
      "formula",
      "related-concept",
      "domain",
      "category",
    ],
  );
  let rich_fact_refs = fact_ids(&ontology_source_facts);
  let mut interpretations = vec![judgment::interpretation_with_refs(
    format!("interp.continuation.{}.direct.{term}", kind.as_str()),
    direct_fact_refs.clone(),
  )];
  if rich_fact_refs.len() > direct_fact_refs.len() {
    interpretations.push(judgment::interpretation_with_refs(
      format!("interp.continuation.{}.rich.{term}", kind.as_str()),
      rich_fact_refs,
    ));
  }
  if let Some(decision) = judgment::ontology_query_decision(
    route,
    &route_spec,
    interpretations,
    &ontology_source_facts,
    Some(&JudgementIntent::with_intent_type(
      request.scope,
      kind.as_str(),
    )),
  ) {
    append_pnix_ontology_query_decision(
      &mut facts,
      &mut notes,
      &provenance,
      route,
      &decision,
      Some(&mut events),
    );
  }
  facts.extend(ontology_source_facts);
  finalize_response(
    resources,
    &request.utterance,
    route,
    format!(
      "pnix가 이전 '{term}' 질의를 이어서 {} 응답을 했다.",
      kind.as_str()
    ),
    notes,
    facts,
    None,
    None,
    events,
  )
}

fn answer_cross_concept(
  resources: &RuntimeResources,
  request: &PnixQueryRequest,
  term_a: &str,
  term_b: &str,
  reopened: Option<&HeldState>,
) -> Result<KernelResponse> {
  let route = "cross-concept-comparison";
  let route_spec = route_spec(resources, route)?;
  let runtime_rule = route_runtime_rule(resources, route)
    .ok_or_else(|| err_missing_runtime("route runtime rule for cross-concept route", route))?;
  let direct_interpretation_id = runtime_rule
    .direct_interpretation_id
    .as_deref()
    .ok_or_else(|| {
      anyhow!(
        "cross-concept route '{}' runtime rule missing direct interpretation id",
        route
      )
    })?;
  let rich_interpretation_id = runtime_rule
    .rich_interpretation_id
    .as_deref()
    .ok_or_else(|| {
      anyhow!(
        "cross-concept route '{}' runtime rule missing rich interpretation id",
        route
      )
    })?;

  let concept_a = resources
    .concepts_by_term
    .get(term_a)
    .and_then(|concepts| concepts.first());
  let concept_b = resources
    .concepts_by_term
    .get(term_b)
    .and_then(|concepts| concepts.first());
  if concept_a.is_none() && concept_b.is_none() {
    return Err(anyhow!(
      "no concept facts found for cross-concept terms '{term_a}' and '{term_b}'"
    ));
  }

  let facts_a = concept_a
    .map(|concept| concept_source_facts(resources, concept))
    .unwrap_or_default();
  let facts_b = concept_b
    .map(|concept| concept_source_facts(resources, concept))
    .unwrap_or_default();

  let def_a = facts_a
    .iter()
    .find(|fact| fact.pred == "definition-ko")
    .map(|fact| fact.obj.as_str())
    .unwrap_or("");
  let def_b = facts_b
    .iter()
    .find(|fact| fact.pred == "definition-ko")
    .map(|fact| fact.obj.as_str())
    .unwrap_or("");
  let domain_a = facts_a
    .iter()
    .find(|fact| fact.pred == "domain")
    .map(|fact| fact.obj.as_str())
    .unwrap_or("");
  let domain_b = facts_b
    .iter()
    .find(|fact| fact.pred == "domain")
    .map(|fact| fact.obj.as_str())
    .unwrap_or("");
  let category_a = facts_a
    .iter()
    .find(|fact| fact.pred == "category")
    .map(|fact| fact.obj.as_str())
    .unwrap_or("");
  let category_b = facts_b
    .iter()
    .find(|fact| fact.pred == "category")
    .map(|fact| fact.obj.as_str())
    .unwrap_or("");
  let inverse_a = facts_a
    .iter()
    .find(|fact| fact.pred == "inverse-of")
    .map(|fact| fact.obj.as_str())
    .unwrap_or("");
  let related_a: Vec<&str> = facts_a
    .iter()
    .filter(|fact| fact.pred == "related-concept")
    .map(|fact| fact.obj.as_str())
    .collect();
  let related_b: Vec<&str> = facts_b
    .iter()
    .filter(|fact| fact.pred == "related-concept")
    .map(|fact| fact.obj.as_str())
    .collect();

  let a_mentions_b = related_a.contains(&term_b) || inverse_a == term_b;
  let b_mentions_a = related_b.contains(&term_a)
    || facts_b
      .iter()
      .any(|fact| fact.pred == "inverse-of" && fact.obj == term_a);
  let same_domain = !domain_a.is_empty() && domain_a == domain_b;
  let same_category = !category_a.is_empty() && category_a == category_b;

  let provenance = vec![
    fill_template(
      resources.query_provenance_templates.utterance.as_str(),
      &[("${utterance}", request.utterance.as_str())],
    ),
    format!("cross-concept:{term_a},{term_b}"),
  ];

  let mut response_parts = vec![format!("{term_a}: {def_a}"), format!("{term_b}: {def_b}")];
  if inverse_a == term_b {
    response_parts.push(format!("{term_a}은(는) {term_b}의 역연산이다."));
  } else if facts_b
    .iter()
    .any(|fact| fact.pred == "inverse-of" && fact.obj == term_a)
  {
    response_parts.push(format!("{term_b}은(는) {term_a}의 역연산이다."));
  }
  if same_category {
    response_parts.push(format!("둘 다 같은 분류({category_a})에 속한다."));
  } else if same_domain {
    response_parts.push(format!(
      "둘 다 같은 도메인({domain_a})이지만 분류가 다르다({category_a}, {category_b})."
    ));
  }
  if a_mentions_b || b_mentions_a {
    response_parts.push("서로 관련 개념으로 연결되어 있다.".to_string());
  }
  let response_text = response_parts.join(" ");

  let mut ontology_source_facts = facts_a.clone();
  for fact in &facts_b {
    if !ontology_source_facts
      .iter()
      .any(|existing: &ContextualFact| existing.id == fact.id)
    {
      ontology_source_facts.push(fact.clone());
    }
  }
  let direct_fact_refs = fact_ids_for_predicates(
    &ontology_source_facts,
    &["definition-ko", "domain", "category"],
  );
  let rich_fact_refs = fact_ids(&ontology_source_facts);

  let mut facts = base_query_facts(
    resources,
    &request.utterance,
    route,
    request.scope,
    &provenance,
    &[],
    &[],
  );
  facts.push(make_candidate_fact(
    "fact.pnix.cross.term-a".to_string(),
    "Pnix.CrossConcept",
    "user",
    "cross-concept-term-a",
    term_a.to_string(),
    provenance.clone(),
  ));
  facts.push(make_candidate_fact(
    "fact.pnix.cross.term-b".to_string(),
    "Pnix.CrossConcept",
    "user",
    "cross-concept-term-b",
    term_b.to_string(),
    provenance.clone(),
  ));
  facts.push(make_candidate_fact(
    "fact.pnix.cross.same-domain".to_string(),
    "Pnix.CrossConcept",
    "pnix",
    "cross-concept-same-domain",
    same_domain.to_string(),
    provenance.clone(),
  ));
  facts.push(make_candidate_fact(
    "fact.pnix.cross.same-category".to_string(),
    "Pnix.CrossConcept",
    "pnix",
    "cross-concept-same-category",
    same_category.to_string(),
    provenance.clone(),
  ));
  facts.push(make_candidate_fact(
    "fact.pnix.cross.mutual-reference".to_string(),
    "Pnix.CrossConcept",
    "pnix",
    "cross-concept-mutual-reference",
    (a_mentions_b || b_mentions_a).to_string(),
    provenance.clone(),
  ));
  if !def_a.is_empty() {
    facts.push(make_candidate_fact(
      "fact.pnix.cross.def-a".to_string(),
      "Pnix.CrossConcept",
      term_a,
      "concept-definition-ko",
      def_a.to_string(),
      provenance.clone(),
    ));
  }
  if !def_b.is_empty() {
    facts.push(make_candidate_fact(
      "fact.pnix.cross.def-b".to_string(),
      "Pnix.CrossConcept",
      term_b,
      "concept-definition-ko",
      def_b.to_string(),
      provenance.clone(),
    ));
  }

  let mut notes = base_notes(resources, &request.utterance, &response_text, reopened);
  notes.push(format!("cross-concept:term-a:{term_a}"));
  notes.push(format!("cross-concept:term-b:{term_b}"));
  let mut events = DecisionEvents::default();
  let mut interpretations = vec![judgment::interpretation_with_refs(
    route_interpretation_template(
      direct_interpretation_id,
      &[("${term_a}", term_a), ("${term_b}", term_b)],
    ),
    direct_fact_refs.clone(),
  )];
  if rich_fact_refs.len() > direct_fact_refs.len() {
    interpretations.push(judgment::interpretation_with_refs(
      route_interpretation_template(
        rich_interpretation_id,
        &[("${term_a}", term_a), ("${term_b}", term_b)],
      ),
      rich_fact_refs,
    ));
  }
  if let Some(decision) = judgment::ontology_query_decision(
    route,
    &route_spec,
    interpretations,
    &ontology_source_facts,
    Some(&JudgementIntent::with_intent_type(
      request.scope,
      "cross-concept",
    )),
  ) {
    append_pnix_ontology_query_decision(
      &mut facts,
      &mut notes,
      &provenance,
      route,
      &decision,
      Some(&mut events),
    );
  }
  finalize_response(
    resources,
    &request.utterance,
    route,
    format!("온톨로지가 '{term_a}'와 '{term_b}'의 관계를 비교하여 응답했다."),
    notes,
    facts,
    None,
    None,
    events,
  )
}

#[derive(Debug, Clone)]
struct SplitClause {
  text: String,
  relation: String,
}

fn split_korean_clauses(resources: &RuntimeResources, utterance: &str) -> Vec<SplitClause> {
  let tokens: Vec<&str> = utterance.split_whitespace().collect();
  let mut clauses = Vec::new();
  let mut current_tokens: Vec<&str> = Vec::new();
  let mut pending_relation = "root".to_string();

  for token in &tokens {
    let mut found = false;
    for marker in &resources.korean_morphology.quotation_markers {
      if token.ends_with(marker.as_str()) && token.len() > marker.len() {
        current_tokens.push(token);
        clauses.push(SplitClause {
          text: current_tokens.join(" "),
          relation: pending_relation.clone(),
        });
        current_tokens.clear();
        pending_relation = "quote".to_string();
        found = true;
        break;
      }
    }
    if found {
      continue;
    }
    for connector in &resources.korean_morphology.clause_connectors {
      if token.ends_with(connector.connector.as_str()) && token.len() > connector.connector.len() {
        current_tokens.push(token);
        clauses.push(SplitClause {
          text: current_tokens.join(" "),
          relation: pending_relation.clone(),
        });
        current_tokens.clear();
        pending_relation = connector.relation.clone();
        found = true;
        break;
      }
    }
    if !found {
      current_tokens.push(token);
    }
  }

  if !current_tokens.is_empty() {
    clauses.push(SplitClause {
      text: current_tokens.join(" "),
      relation: pending_relation,
    });
  }
  clauses
}

fn clause_relation_label_ko(resources: &RuntimeResources, relation: &str) -> String {
  match relation {
    "root" => "시작".to_string(),
    "quote" => "인용".to_string(),
    _ => resources
      .korean_morphology
      .clause_connectors
      .iter()
      .find(|connector| connector.relation == relation)
      .map(|connector| connector.label_ko.clone())
      .unwrap_or_else(|| relation.to_string()),
  }
}

fn dominant_clause_relation(clauses: &[SplitClause]) -> String {
  let unique_relations: BTreeSet<String> = clauses
    .iter()
    .filter_map(|clause| (clause.relation != "root").then_some(clause.relation.clone()))
    .collect();
  match unique_relations.len() {
    0 => "root".to_string(),
    1 => unique_relations
      .into_iter()
      .next()
      .unwrap_or_else(|| "root".to_string()),
    _ => "mixed".to_string(),
  }
}

fn multi_clause_response_text(resources: &RuntimeResources, clauses: &[SplitClause]) -> String {
  let summaries = clauses
    .iter()
    .map(|clause| {
      let analysis = analyze_korean_text(&clause.text);
      let mut summary = format!(
        "[{}] '{}'",
        clause_relation_label_ko(resources, &clause.relation),
        clause.text
      );
      if let Some(final_token) = analysis
        .final_token
        .as_deref()
        .filter(|token| !token.is_empty())
      {
        summary.push_str(&format!(" 동사 표현: {final_token}"));
      }
      if !analysis.particles.is_empty() {
        let roles = analysis
          .particles
          .iter()
          .map(|particle| {
            format!(
              "{}={}",
              particle.stem,
              sentence_role_label_ko(particle.kind)
            )
          })
          .collect::<Vec<_>>()
          .join(", ");
        summary.push_str(&format!(" 조사 기반 역할: {roles}"));
      }
      summary
    })
    .collect::<Vec<_>>();
  format!("{}절 복문 분석: {}", clauses.len(), summaries.join(" → "))
}

fn answer_multi_clause_analysis(
  resources: &RuntimeResources,
  request: &PnixQueryRequest,
  clauses: &[SplitClause],
  reopened: Option<&HeldState>,
) -> Result<KernelResponse> {
  let route = "multi-clause-analysis";
  let route_spec = route_spec(resources, route)?;
  let runtime_rule = route_runtime_rule(resources, route)
    .ok_or_else(|| err_missing_runtime("route runtime rule for multi-clause route", route))?;
  let direct_interpretation_id = runtime_rule
    .direct_interpretation_id
    .as_deref()
    .ok_or_else(|| {
      anyhow!(
        "multi-clause route '{}' runtime rule missing direct interpretation id",
        route
      )
    })?;
  let rich_interpretation_id = runtime_rule
    .rich_interpretation_id
    .as_deref()
    .ok_or_else(|| {
      anyhow!(
        "multi-clause route '{}' runtime rule missing rich interpretation id",
        route
      )
    })?;

  let dominant_relation = dominant_clause_relation(clauses);
  let clause_count = clauses.len().to_string();
  let provenance = vec![
    fill_template(
      resources.query_provenance_templates.utterance.as_str(),
      &[("${utterance}", request.utterance.as_str())],
    ),
    format!("multi-clause:{clause_count}"),
  ];
  let mut facts = base_query_facts(
    resources,
    &request.utterance,
    route,
    request.scope,
    &provenance,
    &[],
    &[],
  );
  for (index, clause) in clauses.iter().enumerate() {
    let clause_subj = format!("clause-{index}");
    facts.push(make_candidate_fact(
      format!("fact.pnix.multiclause.{index}.relation"),
      "Doghouse.MultiClause",
      clause_subj.as_str(),
      "clause-relation",
      clause.relation.clone(),
      provenance.clone(),
    ));
    facts.push(make_candidate_fact(
      format!("fact.pnix.multiclause.{index}.text"),
      "Doghouse.MultiClause",
      clause_subj.as_str(),
      "clause-text",
      clause.text.clone(),
      provenance.clone(),
    ));

    let analysis = analyze_korean_text(&clause.text);
    if let Some(final_token) = analysis
      .final_token
      .as_deref()
      .filter(|token| !token.is_empty())
    {
      facts.push(make_candidate_fact(
        format!("fact.pnix.multiclause.{index}.verb"),
        "Doghouse.MultiClause",
        clause_subj.as_str(),
        "sentence-verb",
        final_token.to_string(),
        provenance.clone(),
      ));
    }
    for particle in &analysis.particles {
      facts.push(make_candidate_fact(
        format!(
          "fact.pnix.multiclause.{index}.role.{}.{}",
          particle.stem,
          particle.kind.as_str()
        ),
        "Doghouse.MultiClause",
        particle.stem.as_str(),
        "sentence-role",
        particle.kind.as_str().to_string(),
        provenance.clone(),
      ));
    }
  }

  let response_text = multi_clause_response_text(resources, clauses);
  let mut notes = base_notes(resources, &request.utterance, &response_text, reopened);
  notes.push("multi-clause-analysis:source:pnix-kernel".to_string());
  notes.push(format!("clause-count:{}", clauses.len()));

  let mut events = DecisionEvents::default();
  let direct_interpretation = route_interpretation_template(
    direct_interpretation_id,
    &[
      ("${relation}", dominant_relation.as_str()),
      ("${clause_count}", clause_count.as_str()),
    ],
  );
  let rich_interpretation = route_interpretation_template(
    rich_interpretation_id,
    &[
      ("${relation}", dominant_relation.as_str()),
      ("${clause_count}", clause_count.as_str()),
    ],
  );
  let fact_refs = fact_ids(&facts);
  let mut interpretations = vec![judgment::interpretation_with_refs(
    direct_interpretation,
    fact_refs.clone(),
  )];
  if request.scope == OutputScope::Detailed {
    interpretations.push(judgment::interpretation_with_refs(
      rich_interpretation,
      fact_refs.clone(),
    ));
  }
  if let Some(decision) = judgment::ontology_query_decision(
    route,
    &route_spec,
    interpretations,
    &facts,
    Some(&JudgementIntent::with_intent_type(
      request.scope,
      "multi-clause-analysis",
    )),
  ) {
    append_pnix_ontology_query_decision(
      &mut facts,
      &mut notes,
      &provenance,
      route,
      &decision,
      Some(&mut events),
    );
  }
  finalize_response(
    resources,
    &request.utterance,
    route,
    format!("온톨로지가 복문을 분석하여 응답했다 ({}절).", clauses.len()),
    notes,
    facts,
    None,
    None,
    events,
  )
}

fn answer_sentence_analysis(
  resources: &RuntimeResources,
  request: &PnixQueryRequest,
  reopened: Option<&HeldState>,
) -> Result<KernelResponse> {
  let clauses = split_korean_clauses(resources, &request.utterance);
  if clauses.len() >= 2 {
    return answer_multi_clause_analysis(resources, request, &clauses, reopened);
  }
  let route = "sentence-semantic-analysis";
  let route_spec = route_spec(resources, route)?;
  let runtime_rule = route_runtime_rule(resources, route)
    .ok_or_else(|| err_missing_runtime("route runtime rule for sentence-analysis route", route))?;
  let direct_interpretation_id = runtime_rule
    .direct_interpretation_id
    .as_deref()
    .ok_or_else(|| {
      anyhow!(
        "sentence-analysis route '{}' runtime rule missing direct interpretation id",
        route
      )
    })?;
  let rich_interpretation_id = runtime_rule
    .rich_interpretation_id
    .as_deref()
    .ok_or_else(|| {
      anyhow!(
        "sentence-analysis route '{}' runtime rule missing rich interpretation id",
        route
      )
    })?;

  let analysis = analyze_korean_text(&request.utterance);
  let mood = analysis.sentence_mood.as_str();
  let provenance = vec![
    fill_template(
      resources.query_provenance_templates.utterance.as_str(),
      &[("${utterance}", request.utterance.as_str())],
    ),
    format!("sentence-analysis:{mood}"),
  ];

  let mut facts = base_query_facts(
    resources,
    &request.utterance,
    route,
    request.scope,
    &provenance,
    &[],
    &[],
  );
  if let Some(final_token) = analysis
    .final_token
    .clone()
    .filter(|token| !token.is_empty())
  {
    facts.push(make_candidate_fact(
      "fact.pnix.sentence.verb".to_string(),
      "Pnix.SentenceAnalysis",
      "pnix",
      "sentence-verb",
      final_token,
      provenance.clone(),
    ));
  }
  if analysis.sentence_mood != KoreanSentenceMood::Unknown {
    facts.push(make_candidate_fact(
      "fact.pnix.sentence.mood".to_string(),
      "Pnix.SentenceAnalysis",
      "pnix",
      "sentence-mood",
      mood.to_string(),
      provenance.clone(),
    ));
  }
  for particle in &analysis.particles {
    facts.push(make_candidate_fact(
      format!(
        "fact.pnix.sentence.role.{}.{}",
        particle.stem,
        particle.kind.as_str()
      ),
      "Pnix.SentenceAnalysis",
      particle.stem.as_str(),
      "sentence-role",
      particle.kind.as_str().to_string(),
      provenance.clone(),
    ));
  }

  let response_text = sentence_analysis_response_text(&analysis);
  let mut notes = base_notes(resources, &request.utterance, &response_text, reopened);
  notes.push("sentence-analysis:source:pnix-kernel".to_string());
  let mut events = DecisionEvents::default();
  let direct_interpretation =
    route_interpretation_template(direct_interpretation_id, &[("${mood}", mood)]);
  let rich_interpretation =
    route_interpretation_template(rich_interpretation_id, &[("${mood}", mood)]);
  let fact_refs = fact_ids(&facts);
  let mut interpretations = vec![judgment::interpretation_with_refs(
    direct_interpretation,
    fact_refs.clone(),
  )];
  if request.scope == OutputScope::Detailed {
    interpretations.push(judgment::interpretation_with_refs(
      rich_interpretation,
      fact_refs.clone(),
    ));
  }
  if let Some(decision) = judgment::ontology_query_decision(
    route,
    &route_spec,
    interpretations,
    &facts,
    Some(&JudgementIntent::with_intent_type(
      request.scope,
      "sentence-analysis",
    )),
  ) {
    append_pnix_ontology_query_decision(
      &mut facts,
      &mut notes,
      &provenance,
      route,
      &decision,
      Some(&mut events),
    );
  }
  finalize_response(
    resources,
    &request.utterance,
    route,
    format!("온톨로지가 문장을 분석했다 ({mood})."),
    notes,
    facts,
    None,
    None,
    events,
  )
}

fn answer_why(
  resources: &RuntimeResources,
  request: &PnixQueryRequest,
  term: &str,
  trigger_type: &str,
  truth_regime: &str,
  reopened: Option<&HeldState>,
) -> Result<KernelResponse> {
  let route = dispatch_route_why(resources, trigger_type);
  let route_spec = route_spec(resources, route.as_str())?;
  let concepts = concepts_for_term(resources, term)?;
  if trigger_type == "contradiction-detect" {
    let interpretation_rule = invert_interpretation_rule(resources, trigger_type)?;
    let mut seen_definitions = BTreeSet::new();
    let conflicting_concepts: Vec<_> = concepts
      .iter()
      .filter(|concept| {
        let definition = concept.definition_ko.trim();
        !definition.is_empty() && seen_definitions.insert(definition.to_string())
      })
      .collect();
    if conflicting_concepts.len() < 2 {
      return Err(anyhow!("no contradictory definitions found for {term}"));
    }
    let mut provenance = vec![fill_template(
      resources.query_provenance_templates.utterance.as_str(),
      &[("${utterance}", request.utterance.as_str())],
    )];
    provenance.extend(conflicting_concepts.iter().map(|concept| {
      fill_template(
        resources.query_provenance_templates.concept_source.as_str(),
        &[("${source-ref}", concept.source_ref.as_str())],
      )
    }));
    let mut facts = base_query_facts(
      resources,
      &request.utterance,
      route.as_str(),
      request.scope,
      &provenance,
      &[],
      &[],
    );
    let branch_count = conflicting_concepts.len();
    for (index, concept) in conflicting_concepts.iter().enumerate() {
      facts.push(make_candidate_fact(
        format!("fact.pnix.invert.contradiction.definition.{term}.{index}"),
        "ontology-invert.contradiction",
        term,
        "definition-ko",
        concept.definition_ko.clone(),
        vec![
          fill_template(
            resources.query_provenance_templates.utterance.as_str(),
            &[("${utterance}", request.utterance.as_str())],
          ),
          fill_template(
            resources.query_provenance_templates.concept_source.as_str(),
            &[("${source-ref}", concept.source_ref.as_str())],
          ),
        ],
      ));
    }
    let branch_desc = format!(
      "'{term}'에 대해 {}개의 서로 다른 정의가 존재한다: {}",
      branch_count,
      conflicting_concepts
        .iter()
        .take(3)
        .map(|concept| format!(
          "'{}'",
          concept.definition_ko.chars().take(40).collect::<String>()
        ))
        .collect::<Vec<_>>()
        .join(" vs ")
    );
    facts.push(make_candidate_fact(
      format!("fact.pnix.invert.branch-point.{term}"),
      "ontology-invert.contradiction",
      term,
      "branch-point",
      branch_desc.clone(),
      provenance.clone(),
    ));
    let mut notes = base_notes(resources, &request.utterance, &branch_desc, reopened);
    notes.push(fill_template(
      resources.note_templates.truth_regime.as_str(),
      &[("${regime}", truth_regime)],
    ));
    notes.push(fill_template(
      resources.note_templates.invert_trigger.as_str(),
      &[("${trigger-type}", trigger_type)],
    ));
    notes.push(format!("branch-count:{branch_count}"));
    let mut events = DecisionEvents::default();
    let direct_fact_refs = fact_ids_for_predicates(&facts, &["branch-point"]);
    let rich_fact_refs = fact_ids(&facts);
    let interpretations = build_interpretations(
      invert_direct_interpretation_id(
        interpretation_rule,
        &[("${trigger_type}", trigger_type), ("${term}", term)],
      ),
      direct_fact_refs,
      invert_rich_interpretation_id(
        interpretation_rule,
        &[("${trigger_type}", trigger_type), ("${term}", term)],
      ),
      rich_fact_refs,
    );
    if let Some(decision) = judgment::ontology_query_decision(
      route.as_str(),
      &route_spec,
      interpretations,
      &facts,
      Some(&JudgementIntent::with_intent_type(request.scope, "why")),
    ) {
      append_pnix_ontology_query_decision(
        &mut facts,
        &mut notes,
        &provenance,
        route.as_str(),
        &decision,
        Some(&mut events),
      );
    }
    return finalize_response(
      resources,
      &request.utterance,
      route.as_str(),
      resources
        .dialogue_templates
        .route_summary_why
        .replace("${term}", term),
      notes,
      facts,
      None,
      Some(truth_regime.to_string()),
      events,
    );
  }
  let concept = concepts
    .first()
    .ok_or_else(|| anyhow!("no concept found for {term}"))?;
  let interpretation_rule = invert_interpretation_rule(resources, trigger_type)?;
  let provenance = vec![
    fill_template(
      resources.query_provenance_templates.utterance.as_str(),
      &[("${utterance}", request.utterance.as_str())],
    ),
    fill_template(
      resources.query_provenance_templates.concept_source.as_str(),
      &[("${source-ref}", concept.source_ref.as_str())],
    ),
  ];
  let mut facts = base_query_facts(
    resources,
    &request.utterance,
    route.as_str(),
    request.scope,
    &provenance,
    &[],
    &[],
  );
  let mut source_facts = concept_source_facts(resources, concept);
  let mut candidate_refs = Vec::new();
  for rule in resources
    .invert
    .candidate_rules
    .iter()
    .filter(|rule| rule.trigger_type == trigger_type)
  {
    let obj = invert_candidate_object(rule, concept, term, &provenance);
    let Some(obj) = obj.filter(|value| !value.is_empty()) else {
      continue;
    };
    let fact = make_candidate_fact(
      format!("fact.pnix.invert.{}.{}", rule.predicate, term),
      &rule.context,
      term,
      &rule.predicate,
      obj,
      provenance.clone(),
    );
    candidate_refs.push(fact.id.as_ref().map(|id| id.0.clone()).ok_or_else(|| {
      anyhow!(
        "candidate fact must carry id for trigger '{}'",
        trigger_type
      )
    })?);
    facts.push(fact);
  }
  source_facts.extend(
    facts
      .iter()
      .filter(|fact| invert_source_fact_included(interpretation_rule, fact))
      .cloned(),
  );
  let response_text = why_response_text(&resources.dialogue_templates, concept, truth_regime)?;
  let mut notes = base_notes(resources, &request.utterance, &response_text, reopened);
  notes.push(fill_template(
    resources.note_templates.truth_regime.as_str(),
    &[("${regime}", truth_regime)],
  ));
  notes.push(fill_template(
    resources.note_templates.invert_trigger.as_str(),
    &[("${trigger-type}", trigger_type)],
  ));
  let mut events = DecisionEvents::default();
  let direct_refs = if candidate_refs.is_empty() {
    let direct_predicate_refs = interpretation_rule
      .direct_fact_predicates
      .iter()
      .map(String::as_str)
      .collect::<Vec<_>>();
    fact_ids_for_predicates(&source_facts, &direct_predicate_refs)
  } else {
    candidate_refs
  };
  let rich_refs = fact_ids(&source_facts);
  let interpretations = build_interpretations(
    invert_direct_interpretation_id(
      interpretation_rule,
      &[("${trigger_type}", trigger_type), ("${term}", term)],
    ),
    direct_refs,
    invert_rich_interpretation_id(
      interpretation_rule,
      &[("${trigger_type}", trigger_type), ("${term}", term)],
    ),
    rich_refs,
  );
  if let Some(decision) = judgment::ontology_query_decision(
    route.as_str(),
    &route_spec,
    interpretations,
    &source_facts,
    Some(&JudgementIntent::with_intent_type(request.scope, "why")),
  ) {
    append_pnix_ontology_query_decision(
      &mut facts,
      &mut notes,
      &provenance,
      route.as_str(),
      &decision,
      Some(&mut events),
    );
  }
  finalize_response(
    resources,
    &request.utterance,
    route.as_str(),
    resources
      .dialogue_templates
      .route_summary_why
      .replace("${term}", term),
    notes,
    facts,
    None,
    Some(truth_regime.to_string()),
    events,
  )
}

fn build_held_response(
  resources: &RuntimeResources,
  request: &PnixQueryRequest,
  term: Option<String>,
  reason: &str,
  reopened: Option<&HeldState>,
) -> Result<KernelResponse> {
  let route = dispatch_route_held(resources);
  let provenance = vec![fill_template(
    resources.query_provenance_templates.utterance.as_str(),
    &[("${utterance}", request.utterance.as_str())],
  )];
  let mut facts = base_query_facts(
    resources,
    &request.utterance,
    route,
    request.scope,
    &provenance,
    &[],
    &[],
  );
  let (hint, choices) = follow_up_for(resources, term.as_deref(), reason)?;
  let resolved_term = resolved_followup_term(resources, term.as_deref())?;
  let response_rule = held_response_rule(resources, term.as_deref())?;
  let response_template = response_rule.template.as_str();
  let response_text = fill_template(
    response_template,
    &required_term_template_replacements(
      response_template,
      resolved_term.as_deref(),
      "held-response template requires resolved term",
    )?,
  );
  let mut notes = base_notes(resources, &request.utterance, &response_text, reopened);
  notes.push(fill_template(
    resources.note_templates.held_reason.as_str(),
    &[("${reason}", reason)],
  ));
  let emit_held_term = response_rule.emit_held_term;
  if emit_held_term {
    if let Some(term) = term.as_deref() {
      notes.push(fill_template(
        resources.note_templates.held_term.as_str(),
        &[("${term}", term)],
      ));
      facts.push(make_candidate_fact(
        format!("fact.pnix.held.term.{term}"),
        "Pnix.Held",
        "pnix",
        "held-term",
        term.to_string(),
        provenance.clone(),
      ));
    }
  }
  facts.push(make_candidate_fact(
    "fact.pnix.held.state".to_string(),
    "Pnix.Held",
    "pnix",
    "held-reason",
    reason.to_string(),
    provenance,
  ));
  finalize_response(
    resources,
    &request.utterance,
    route,
    resources.dialogue_templates.route_summary_held.clone(),
    notes,
    facts,
    Some((hint, choices)),
    None,
    DecisionEvents::default(),
  )
}

fn finalize_response(
  resources: &RuntimeResources,
  utterance: &str,
  route: &str,
  summary: String,
  notes: Vec<String>,
  facts: Vec<ContextualFact>,
  follow_up: Option<(String, Vec<String>)>,
  truth_regime: Option<String>,
  events: DecisionEvents,
) -> Result<KernelResponse> {
  let counter = EPISODE_COUNTER.fetch_add(1, Ordering::Relaxed).to_string();
  let episode_id = SemanticEpisodeId::from(fill_template(
    resources.semantic_id_templates.episode_id_template.as_str(),
    &[("${counter}", counter.as_str())],
  ));
  let observation_ref = fill_template(
    resources.query_provenance_templates.utterance.as_str(),
    &[("${utterance}", utterance)],
  );
  let envelope = build_envelope(
    resources,
    &episode_id,
    observation_ref,
    summary.clone(),
    facts,
    notes,
  );
  let (judgement_events, promotion_events) = events.into_protocol_events(&episode_id.0, 0);
  let transcript =
    transcript_from_notes(&envelope.notes, resources.transcript_note_prefix.as_str());
  let (response_document_px, response_document_org) = build_response_documents(
    resources,
    &envelope,
    &resources.pipeline_trace_note_prefixes,
    resources.transcript_note_prefix.as_str(),
  )?;
  let output_fragments = build_output_fragments(
    resources,
    &envelope,
    route,
    &response_document_px,
    &response_document_org,
    &resources.pipeline_trace_note_prefixes,
    resources.transcript_note_prefix.as_str(),
  );
  let (follow_up_hint, follow_up_choices) = follow_up
    .map(|(hint, choices)| (Some(hint), choices))
    .unwrap_or((None, vec![]));
  Ok(KernelResponse {
    episode_id: envelope.episode.id.0.clone(),
    route: route.to_string(),
    summary,
    transcript,
    follow_up_hint,
    follow_up_choices,
    truth_regime,
    envelope,
    judgement_events,
    promotion_events,
    response_document_org,
    response_document_px,
    output_fragments,
  })
}

fn build_envelope(
  resources: &RuntimeResources,
  episode_id: &SemanticEpisodeId,
  observation_ref: String,
  summary: String,
  facts: Vec<ContextualFact>,
  notes: Vec<String>,
) -> SemanticIngestEnvelope {
  let semantic = &resources.semantic_id_templates;
  let records = facts
    .into_iter()
    .enumerate()
    .map(|(index, fact)| {
      let index_str = index.to_string();
      let record_id = fill_template(
        semantic.record_id_template.as_str(),
        &[
          ("${episode-id}", episode_id.0.as_str()),
          ("${index}", index_str.as_str()),
        ],
      );
      SemanticRecord {
        id: SemanticRecordId::from(record_id),
        episode: episode_id.clone(),
        record_kind: SemanticRecordKind::ContextualFact,
        provenance_refs: fact.provenance_refs.clone(),
        artifact_refs: fact.proof_refs.clone(),
        value: SemanticRecordValue::ContextualFact(fact),
      }
    })
    .collect::<Vec<_>>();
  let knowledge_id = fill_template(
    semantic.knowledge_id_template.as_str(),
    &[("${episode-id}", episode_id.0.as_str())],
  );
  SemanticIngestEnvelope {
    observation_refs: vec![observation_ref.clone()],
    records: records.clone(),
    episode: SemanticEpisode {
      id: episode_id.clone(),
      observation_refs: vec![observation_ref],
      record_refs: records.iter().map(|record| record.id.clone()).collect(),
      chosen_interpretation: None,
      judgement_ref: None,
      promotion_ref: None,
      summary: Some(summary),
    },
    knowledge_records: vec![KnowledgeRecord {
      id: KnowledgeRecordId::from(knowledge_id),
      episode: episode_id.clone(),
      target_status: MeaningStatus::Candidate,
      fact_refs: records.iter().map(|record| record.id.0.clone()).collect(),
      source_record_refs: records.iter().map(|record| record.id.clone()).collect(),
      provenance_refs: vec![],
      summary: Some(semantic.knowledge_summary.clone()),
    }],
    notes,
  }
}

fn build_response_documents(
  resources: &RuntimeResources,
  envelope: &SemanticIngestEnvelope,
  trace_prefixes: &[String],
  transcript_prefix: &str,
) -> Result<(String, String)> {
  let schema = &resources.response_document_schema;
  let transcript_lines = transcript_from_notes(&envelope.notes, transcript_prefix);
  let trace_lines = envelope
    .notes
    .iter()
    .filter(|note| note_matches_trace_prefix(note, trace_prefixes))
    .cloned()
    .collect::<Vec<_>>();
  let fact_count = envelope
    .records
    .iter()
    .filter(|record| matches!(record.record_kind, SemanticRecordKind::ContextualFact))
    .count();
  let summary = envelope
    .episode
    .summary
    .as_deref()
    .ok_or_else(|| anyhow!("standalone kernel response document requires episode summary"))?;

  let mut px = String::new();
  px.push_str(schema.px_header_comment.as_str());
  px.push_str("\n{\n");
  px.push_str(&format!(
    "  {} = \"{}\";\n",
    schema.px_field_episode_id, envelope.episode.id.0
  ));
  px.push_str(&format!(
    "  {} = \"{}\";\n",
    schema.px_field_summary,
    summary.replace('"', "'")
  ));
  px.push_str(&format!("  {} = [\n", schema.px_field_transcript));
  for line in &transcript_lines {
    px.push_str(&format!("    \"{}\"\n", line.replace('"', "'")));
  }
  px.push_str("  ];\n");
  px.push_str(&format!("  {} = [\n", schema.px_field_pipeline));
  for note in &trace_lines {
    px.push_str(&format!("    \"{}\"\n", note.replace('"', "'")));
  }
  px.push_str("  ];\n");
  px.push_str(&format!(
    "  {} = {};\n",
    schema.px_field_facts_count, fact_count
  ));
  px.push_str("}\n");

  let mut org = String::new();
  org.push_str(schema.org_title.as_str());
  org.push('\n');
  for line in &transcript_lines {
    let mut emitted = false;
    for transform in &schema.org_transcript_transforms {
      if let Some(rest) = line.strip_prefix(transform.input_prefix.as_str()) {
        org.push_str(&format!("{}{}\n", transform.output_prefix, rest));
        emitted = true;
        break;
      }
    }
    if !emitted {
      org.push_str(&format!("{line}\n"));
    }
  }
  if !trace_lines.is_empty() {
    org.push('\n');
    org.push_str(schema.org_pipeline_section_header.as_str());
    org.push('\n');
    for note in &trace_lines {
      org.push_str(&format!("- ~{}~\n", note));
    }
  }
  if fact_count > 0 {
    let facts_count_str = fact_count.to_string();
    let facts_line = fill_template(
      schema.org_facts_count_template.as_str(),
      &[("${count}", facts_count_str.as_str())],
    );
    org.push('\n');
    org.push_str(facts_line.as_str());
    org.push('\n');
  }
  Ok((px, org))
}

fn build_output_fragments(
  resources: &RuntimeResources,
  envelope: &SemanticIngestEnvelope,
  route: &str,
  response_document_px: &str,
  response_document_org: &str,
  trace_prefixes: &[String],
  transcript_prefix: &str,
) -> Vec<KernelOutputFragment> {
  let fragment_templates = &resources.output_fragment_templates;
  let trace_text = envelope
    .notes
    .iter()
    .filter(|note| note_matches_trace_prefix(note, trace_prefixes))
    .cloned()
    .collect::<Vec<_>>()
    .join("\n");
  let mut fragments = Vec::new();
  if !trace_text.is_empty() {
    fragments.push(KernelOutputFragment {
      producer: OUTPUT_FRAGMENT_PRODUCER_PNIX.to_string(),
      producer_contract: OUTPUT_FRAGMENT_CONTRACT_V1.to_string(),
      producer_route: route.to_string(),
      producer_episode_id: envelope.episode.id.0.clone(),
      kind: fragment_templates.pipeline_trace.kind.clone(),
      visibility: fragment_templates.pipeline_trace.visibility.clone(),
      content_org: trace_text
        .lines()
        .map(|line| format!("- ~{}~", line))
        .collect::<Vec<_>>()
        .join("\n"),
      content_px: None,
      content_html: None,
      content_speech: None,
      content_text: Some(trace_text),
    });
  }
  fragments.push(KernelOutputFragment {
    producer: OUTPUT_FRAGMENT_PRODUCER_PNIX.to_string(),
    producer_contract: OUTPUT_FRAGMENT_CONTRACT_V1.to_string(),
    producer_route: route.to_string(),
    producer_episode_id: envelope.episode.id.0.clone(),
    kind: fragment_templates.response_document.kind.clone(),
    visibility: fragment_templates.response_document.visibility.clone(),
    content_org: response_document_org.to_string(),
    content_px: Some(response_document_px.to_string()),
    content_html: Some(response_document_html(response_document_org)),
    content_speech: Some(crate::response_document::response_document_speech_text(
      response_document_org,
    )),
    content_text: Some(
      transcript_from_notes(&envelope.notes, transcript_prefix)
        .into_iter()
        .collect::<Vec<_>>()
        .join("\n"),
    ),
  });
  fragments
}

fn output_scope_as_str(scope: OutputScope) -> &'static str {
  match scope {
    OutputScope::Brief => "brief",
    OutputScope::Standard => "standard",
    OutputScope::Detailed => "detailed",
  }
}

fn base_query_facts(
  resources: &RuntimeResources,
  utterance: &str,
  route: &str,
  scope: OutputScope,
  provenance: &[String],
  extra_replacements: &[(&str, &str)],
  property_values: &[String],
) -> Vec<ContextualFact> {
  base_query_facts_with_extra(
    resources,
    utterance,
    route,
    scope,
    provenance,
    extra_replacements,
    property_values,
  )
}

fn base_query_facts_with_extra(
  resources: &RuntimeResources,
  utterance: &str,
  route: &str,
  scope: OutputScope,
  provenance: &[String],
  extra_replacements: &[(&str, &str)],
  property_values: &[String],
) -> Vec<ContextualFact> {
  let route_segment = route.replace('/', ".");
  let scope_str = output_scope_as_str(scope);
  let mut replacements: Vec<(&str, &str)> = vec![
    ("${route}", route),
    ("${route-segment}", route_segment.as_str()),
    ("${utterance}", utterance),
    ("${scope}", scope_str),
  ];
  replacements.extend_from_slice(extra_replacements);
  resources
    .base_query_fact_rules
    .iter()
    .filter(|rule| {
      rule
        .when_route
        .as_deref()
        .map(|expected| expected == route)
        .unwrap_or(true)
    })
    .flat_map(|rule| match rule.repeat_over {
      None => vec![render_kernel_base_fact(rule, &replacements, provenance)],
      Some(KernelBaseFactRepeatOver::PropertyValues) => property_values
        .iter()
        .enumerate()
        .map(|(index, value)| {
          let index_string = index.to_string();
          let mut repeated_replacements = replacements.clone();
          repeated_replacements.push(("${index}", index_string.as_str()));
          repeated_replacements.push(("${value}", value.as_str()));
          render_kernel_base_fact(rule, &repeated_replacements, provenance)
        })
        .collect::<Vec<_>>(),
    })
    .collect()
}

fn render_kernel_base_fact(
  rule: &KernelBaseFactRule,
  replacements: &[(&str, &str)],
  provenance: &[String],
) -> ContextualFact {
  let id = fill_template(&rule.id_template, replacements);
  let pred = if let Some(template) = rule.pred_template.as_deref() {
    fill_template(template, replacements)
  } else if let Some(literal) = rule.pred.as_deref() {
    literal.to_string()
  } else {
    unreachable!("validated kernel-base-facts rule must carry 'pred' or 'pred-template'");
  };
  let obj = if let Some(literal) = rule.obj_literal.as_deref() {
    literal.to_string()
  } else if let Some(template) = rule.obj_template.as_deref() {
    fill_template(template, replacements)
  } else {
    unreachable!("validated kernel-base-facts rule must carry 'obj-template' or 'obj-literal'");
  };
  make_candidate_fact(
    id,
    rule.context.as_str(),
    rule.subj.as_str(),
    pred.as_str(),
    obj,
    provenance.to_vec(),
  )
}

fn base_notes(
  resources: &RuntimeResources,
  utterance: &str,
  response_text: &str,
  reopened: Option<&HeldState>,
) -> Vec<String> {
  let templates = &resources.note_templates;
  let mut notes = vec![
    fill_template(
      templates.transcript_user.as_str(),
      &[("${utterance}", utterance)],
    ),
    fill_template(
      templates.transcript_pnix.as_str(),
      &[("${response}", response_text)],
    ),
  ];
  if let Some(reopened) = reopened {
    notes.push(fill_template(
      templates.held_reopen_reason.as_str(),
      &[("${reason}", reopened.reason.as_str())],
    ));
    if let Some(term) = reopened.term.as_deref() {
      notes.push(fill_template(
        templates.held_reopen_term.as_str(),
        &[("${term}", term)],
      ));
    }
  }
  notes
}

fn transcript_from_notes(notes: &[String], transcript_prefix: &str) -> Vec<String> {
  notes
    .iter()
    .filter_map(|note| note.strip_prefix(transcript_prefix).map(str::to_string))
    .collect()
}

fn note_matches_trace_prefix(note: &str, trace_prefixes: &[String]) -> bool {
  trace_prefixes
    .iter()
    .any(|prefix| note.starts_with(prefix.as_str()))
}

fn make_candidate_fact(
  id: String,
  context: &str,
  subj: &str,
  pred: &str,
  obj: String,
  provenance_refs: Vec<String>,
) -> ContextualFact {
  ContextualFact {
    id: Some(MeaningId::from(id)),
    context: ContextId::from(context),
    layer: LayerId::from("L4"),
    subj: subj.to_string(),
    pred: pred.to_string(),
    obj,
    status: MeaningStatus::Candidate,
    confidence: 0.95,
    provenance_refs,
    proof_refs: vec![],
    contradiction_refs: vec![],
    loss: None,
    timestamp: None,
  }
}

fn make_fact_with_semantics(
  id: String,
  context: &str,
  layer: &str,
  status: MeaningStatus,
  confidence: f64,
  subj: &str,
  pred: &str,
  obj: String,
  provenance_refs: Vec<String>,
) -> ContextualFact {
  ContextualFact {
    id: Some(MeaningId::from(id)),
    context: ContextId::from(context),
    layer: LayerId::from(layer),
    subj: subj.to_string(),
    pred: pred.to_string(),
    obj,
    status,
    confidence,
    provenance_refs,
    proof_refs: vec![],
    contradiction_refs: vec![],
    loss: None,
    timestamp: None,
  }
}

fn concept_source_facts(
  resources: &RuntimeResources,
  concept: &ConceptDefinition,
) -> Vec<ContextualFact> {
  let templates = &resources.concept_source_fact_templates;
  let metadata = &resources.query_classifiers.source_metadata;
  let base_provenance = vec![fill_template(
    templates.provenance_template.as_str(),
    &[("${source-ref}", concept.source_ref.as_str())],
  )];
  let mut facts = Vec::new();
  for rule in &resources.query_classifiers.source_fact_fields {
    let Some(value) = concept.scalar_fields.get(rule.field.as_str()) else {
      continue;
    };
    if value.is_empty() {
      continue;
    }
    let context = rule.context.as_deref().unwrap_or(concept.context.as_str());
    let id = fill_template(
      templates.scalar_id_template.as_str(),
      &[
        ("${term}", concept.term_ko.as_str()),
        ("${predicate}", rule.predicate.as_str()),
      ],
    );
    facts.push(make_fact_with_semantics(
      id,
      context,
      rule.layer.as_str(),
      rule.status.clone(),
      rule.confidence,
      &concept.term_ko,
      &rule.predicate,
      fill_template(
        rule.object_template.as_str(),
        &[
          ("${term}", concept.term_ko.as_str()),
          ("${field}", rule.field.as_str()),
          ("${predicate}", rule.predicate.as_str()),
          ("${value}", value.as_str()),
          ("${context}", context),
        ],
      ),
      base_provenance.clone(),
    ));
    let field_id_predicate = format!("{}.{}", metadata.field_predicate, rule.field);
    let field_id = fill_template(
      templates.scalar_id_template.as_str(),
      &[
        ("${term}", concept.term_ko.as_str()),
        ("${predicate}", field_id_predicate.as_str()),
      ],
    );
    facts.push(make_fact_with_semantics(
      field_id,
      metadata.context.as_str(),
      metadata.layer.as_str(),
      metadata.status.clone(),
      metadata.confidence,
      &concept.term_ko,
      metadata.field_predicate.as_str(),
      fill_template(
        metadata.field_object_template.as_str(),
        &[
          ("${term}", concept.term_ko.as_str()),
          ("${field}", rule.field.as_str()),
          ("${source-predicate}", rule.predicate.as_str()),
        ],
      ),
      base_provenance.clone(),
    ));
    let value_id_predicate = format!("{}.{}", metadata.value_predicate, rule.field);
    let value_id = fill_template(
      templates.scalar_id_template.as_str(),
      &[
        ("${term}", concept.term_ko.as_str()),
        ("${predicate}", value_id_predicate.as_str()),
      ],
    );
    facts.push(make_fact_with_semantics(
      value_id,
      metadata.context.as_str(),
      metadata.layer.as_str(),
      metadata.status.clone(),
      metadata.confidence,
      &concept.term_ko,
      metadata.value_predicate.as_str(),
      fill_template(
        metadata.value_object_template.as_str(),
        &[
          ("${term}", concept.term_ko.as_str()),
          ("${field}", rule.field.as_str()),
          ("${value}", value.as_str()),
          ("${source-predicate}", rule.predicate.as_str()),
        ],
      ),
      base_provenance.clone(),
    ));
  }
  for rule in &resources.query_classifiers.source_list_fields {
    let Some(values) = concept.list_fields.get(rule.field.as_str()) else {
      continue;
    };
    if values.is_empty() {
      continue;
    }
    let context = rule.context.as_deref().unwrap_or(concept.context.as_str());
    let list_field_id_predicate = format!("{}.{}", metadata.list_field_predicate, rule.field);
    let field_id = fill_template(
      templates.scalar_id_template.as_str(),
      &[
        ("${term}", concept.term_ko.as_str()),
        ("${predicate}", list_field_id_predicate.as_str()),
      ],
    );
    facts.push(make_fact_with_semantics(
      field_id,
      metadata.context.as_str(),
      metadata.layer.as_str(),
      metadata.status.clone(),
      metadata.confidence,
      &concept.term_ko,
      metadata.list_field_predicate.as_str(),
      fill_template(
        metadata.list_field_object_template.as_str(),
        &[
          ("${term}", concept.term_ko.as_str()),
          ("${field}", rule.field.as_str()),
          ("${source-predicate}", rule.predicate.as_str()),
        ],
      ),
      base_provenance.clone(),
    ));
    for (index, value) in values.iter().enumerate() {
      let index_str = index.to_string();
      let id = fill_template(
        templates.list_id_template.as_str(),
        &[
          ("${term}", concept.term_ko.as_str()),
          ("${predicate}", rule.predicate.as_str()),
          ("${index}", index_str.as_str()),
        ],
      );
      facts.push(make_fact_with_semantics(
        id,
        context,
        rule.layer.as_str(),
        rule.status.clone(),
        rule.confidence,
        &concept.term_ko,
        &rule.predicate,
        fill_template(
          rule.object_template.as_str(),
          &[
            ("${term}", concept.term_ko.as_str()),
            ("${field}", rule.field.as_str()),
            ("${predicate}", rule.predicate.as_str()),
            ("${value}", value.as_str()),
            ("${index}", index_str.as_str()),
            ("${context}", context),
          ],
        ),
        base_provenance.clone(),
      ));
      let list_item_id_predicate = format!("{}.{}", metadata.list_item_predicate, rule.field);
      let list_item_id = fill_template(
        templates.list_id_template.as_str(),
        &[
          ("${term}", concept.term_ko.as_str()),
          ("${predicate}", list_item_id_predicate.as_str()),
          ("${index}", index_str.as_str()),
        ],
      );
      facts.push(make_fact_with_semantics(
        list_item_id,
        metadata.context.as_str(),
        metadata.layer.as_str(),
        metadata.status.clone(),
        metadata.confidence,
        &concept.term_ko,
        metadata.list_item_predicate.as_str(),
        fill_template(
          metadata.list_item_object_template.as_str(),
          &[
            ("${term}", concept.term_ko.as_str()),
            ("${field}", rule.field.as_str()),
            ("${value}", value.as_str()),
            ("${index}", index_str.as_str()),
            ("${source-predicate}", rule.predicate.as_str()),
          ],
        ),
        base_provenance.clone(),
      ));
    }
  }
  facts
}

fn definition_response_text(
  resources: &RuntimeResources,
  templates: &DialogueTemplates,
  concept: &ConceptDefinition,
  scope: OutputScope,
) -> Result<String> {
  let formal_name_en = concept
    .scalar_fields
    .get("formal-name-en")
    .map(String::as_str);
  let unit_ko = concept.scalar_fields.get("unit-ko").map(String::as_str);
  let related = concept
    .related_concepts
    .as_ref()
    .map(|values| values.join(", "))
    .filter(|values| !values.is_empty());
  let connected = concept_connected_terms(resources, concept, scope)
    .join(", ")
    .trim()
    .to_string();
  let connected = (!connected.is_empty()).then_some(connected);
  render_template_section(
    &templates.definition_section,
    &[
      (
        "${term}",
        Some(concept.term_ko.as_str()),
        "definition template requires term",
      ),
      (
        "${definition}",
        Some(concept.definition_ko.as_str()),
        "definition template requires definition",
      ),
      ("${unit}", unit_ko, "definition template requires unit-ko"),
      (
        "${symbol}",
        concept.formal_symbol.as_deref(),
        "definition template requires formal-symbol",
      ),
      (
        "${formula}",
        concept.formula.as_deref(),
        "definition template requires formula",
      ),
      (
        "${formal-en}",
        formal_name_en,
        "definition template requires formal-name-en",
      ),
      (
        "${related}",
        related.as_deref(),
        "definition template requires related-concepts",
      ),
      (
        "${connected}",
        connected.as_deref(),
        "definition template requires connected knowledge",
      ),
    ],
    |part| {
      template_when_matches(part)
        && template_field_non_empty(part, |field| concept_field_non_empty(concept, field))
        && template_list_non_empty(part, |list| concept_list_non_empty(concept, list))
        && template_scope_matches(part, scope)
    },
  )
}

fn concept_connected_terms(
  resources: &RuntimeResources,
  concept: &ConceptDefinition,
  scope: OutputScope,
) -> Vec<String> {
  if scope != OutputScope::Detailed {
    return Vec::new();
  }
  let max_hops = 2;
  let mut queue = VecDeque::from([(concept.term_ko.clone(), 0_u32)]);
  let mut visited = BTreeSet::from([concept.term_ko.clone()]);
  let mut connected = Vec::new();

  while let Some((term, hop)) = queue.pop_front() {
    if hop > max_hops {
      continue;
    }
    let Some(definitions) = resources.concepts_by_term.get(term.as_str()) else {
      continue;
    };
    if hop > 0 {
      connected.push(term.clone());
    }
    if hop == max_hops {
      continue;
    }
    for definition in definitions {
      for next in concept_graph_neighbors(definition) {
        if visited.insert(next.clone()) {
          queue.push_back((next, hop + 1));
        }
      }
    }
  }

  connected.truncate(3);
  connected
}

fn concept_graph_neighbors(concept: &ConceptDefinition) -> Vec<String> {
  let mut next = concept.related_concepts.clone().unwrap_or_default();
  if let Some(inverse) = concept.scalar_fields.get("inverse-of") {
    let inverse = inverse.trim();
    if !inverse.is_empty() {
      next.push(inverse.to_string());
    }
  }
  if let Some(formula) = concept.formula.as_deref() {
    next.extend(extract_formula_components(formula));
  }
  next.retain(|value| !value.trim().is_empty());
  next.sort();
  next.dedup();
  next
}

fn extract_formula_components(formula: &str) -> Vec<String> {
  let fallback: &[(&str, &str)] = &[
    ("F", "힘"),
    ("m", "질량"),
    ("a", "가속도"),
    ("p", "운동량"),
    ("v", "속도"),
    ("Eₖ", "운동에너지"),
    ("Eₚ", "위치에너지"),
    ("K", "운동에너지"),
    ("U", "위치에너지"),
    ("W", "일"),
    ("E", "에너지"),
  ];
  let mut components = Vec::new();
  for (symbol, quantity) in fallback {
    if formula.contains(symbol) {
      components.push((*quantity).to_string());
    }
  }
  components.sort();
  components.dedup();
  components
}

fn why_response_text(
  templates: &DialogueTemplates,
  concept: &ConceptDefinition,
  truth_regime: &str,
) -> Result<String> {
  render_template_section(
    &templates.why_section,
    &[
      (
        "${term}",
        Some(concept.term_ko.as_str()),
        "why template requires term",
      ),
      (
        "${why}",
        concept.why.as_deref(),
        "why template requires why",
      ),
      (
        "${boundary}",
        concept.boundary_conditions.as_deref(),
        "why template requires boundary-conditions",
      ),
      (
        "${regime}",
        Some(truth_regime),
        "why template requires truth-regime",
      ),
    ],
    |part| {
      template_when_matches(part)
        && template_field_non_empty(part, |field| concept_field_non_empty(concept, field))
    },
  )
}

fn property_response_text(
  templates: &DialogueTemplates,
  term: &str,
  label_ko: &str,
  predicate: &str,
  values: &[String],
) -> Result<String> {
  // batch 74 (2026-04-15): per-predicate empty response override.
  // values 가 비고 predicate-specific 템플릿이 있으면 generic empty template
  // 대신 override 를 쓴다. doghouse store 의 sentence-level parity 용.
  if values.is_empty() {
    if let Some(template) = templates.property_empty_by_predicate.get(predicate) {
      return Ok(template.replace("${term}", term));
    }
  }
  let joined_values = values.join(", ");
  render_template_section(
    &templates.property_section,
    &[
      ("${term}", Some(term), "property template requires term"),
      (
        "${label}",
        Some(label_ko),
        "property template requires label",
      ),
      (
        "${values}",
        (!joined_values.is_empty()).then_some(joined_values.as_str()),
        "property template requires non-empty values",
      ),
    ],
    |part| template_when_matches(part) && template_values_state_matches(part, values),
  )
}

fn invert_candidate_object(
  rule: &InvertCandidateRule,
  concept: &ConceptDefinition,
  term: &str,
  provenance: &[String],
) -> Option<String> {
  if let Some(field) = rule.concept_field.as_deref() {
    return concept_field_value(concept, field)
      .filter(|value| !value.is_empty())
      .map(ToString::to_string);
  }
  let template = rule.obj_template.as_deref()?;
  let provenance_joined = provenance.join(" -> ");
  Some(fill_template(
    template,
    &[
      ("${term}", term),
      ("${provenance}", provenance_joined.as_str()),
    ],
  ))
}

fn concept_field_value<'a>(concept: &'a ConceptDefinition, field: &str) -> Option<&'a str> {
  concept.scalar_fields.get(field).map(String::as_str)
}

fn concept_field_non_empty(concept: &ConceptDefinition, field: &str) -> bool {
  concept_field_value(concept, field)
    .map(|value| !value.is_empty())
    .unwrap_or(false)
}

fn concept_list_non_empty(concept: &ConceptDefinition, list: &str) -> bool {
  concept
    .list_fields
    .get(list)
    .map(|values| !values.is_empty())
    .unwrap_or(false)
}

fn render_template_section(
  section: &TemplateSection,
  replacements: &[(&'static str, Option<&str>, &'static str)],
  include: impl Fn(&ConditionalTemplatePart) -> bool,
) -> Result<String> {
  let parts = section
    .parts
    .iter()
    .filter(|part| include(part))
    .map(|part| {
      Ok(fill_template(
        part.template.as_str(),
        &required_optional_template_replacements(part.template.as_str(), replacements)?,
      ))
    })
    .collect::<Result<Vec<_>>>()?
    .into_iter()
    .filter(|rendered| !rendered.is_empty())
    .collect::<Vec<_>>();
  let mut rendered = parts.join(section.join_with.as_str());
  if !rendered.is_empty() {
    rendered.push_str(section.suffix.as_str());
  }
  Ok(rendered)
}

fn fill_template(template: &str, replacements: &[(&str, &str)]) -> String {
  let mut rendered = template.to_string();
  for (needle, value) in replacements {
    rendered = rendered.replace(needle, value);
  }
  rendered
}

fn required_term_template_replacements<'a>(
  template: &str,
  resolved_term: Option<&'a str>,
  context: &str,
) -> Result<Vec<(&'static str, &'a str)>> {
  if template.contains("${term}") {
    return Ok(vec![(
      "${term}",
      resolved_term.ok_or_else(|| anyhow!("{context}: {template}"))?,
    )]);
  }
  Ok(vec![])
}

fn required_optional_template_replacements<'a>(
  template: &str,
  replacements: &[(&'static str, Option<&'a str>, &'static str)],
) -> Result<Vec<(&'static str, &'a str)>> {
  let mut resolved = Vec::new();
  for (needle, value, context) in replacements {
    if template.contains(needle) {
      resolved.push((
        *needle,
        value.ok_or_else(|| anyhow!("{context}: {template}"))?,
      ));
    }
  }
  Ok(resolved)
}

fn followup_template_replacements<'a>(
  template: &str,
  resolved_term: Option<&'a str>,
  suggestions: &'a str,
) -> Result<Vec<(&'static str, &'a str)>> {
  let mut replacements = required_term_template_replacements(
    template,
    resolved_term,
    "follow-up template requires resolved term",
  )?;
  if template.contains("${suggestions}") {
    if suggestions.is_empty() {
      return Err(anyhow!(
        "follow-up template requires non-empty suggestions: {template}"
      ));
    }
    replacements.push(("${suggestions}", suggestions));
  }
  Ok(replacements)
}

fn template_field_non_empty(
  part: &ConditionalTemplatePart,
  is_non_empty: impl Fn(&str) -> bool,
) -> bool {
  part
    .field_non_empty
    .as_deref()
    .is_none_or(|field| is_non_empty(field))
}

fn template_when_matches(part: &ConditionalTemplatePart) -> bool {
  part.when == "always"
}

fn template_list_non_empty(
  part: &ConditionalTemplatePart,
  is_non_empty: impl Fn(&str) -> bool,
) -> bool {
  part
    .list_non_empty
    .as_deref()
    .is_none_or(|list| is_non_empty(list))
}

fn template_scope_matches(part: &ConditionalTemplatePart, scope: OutputScope) -> bool {
  part
    .scope_is
    .as_deref()
    .is_none_or(|scope_name| match scope_name {
      "brief" => scope == OutputScope::Brief,
      "standard" => scope == OutputScope::Standard,
      "detailed" => scope == OutputScope::Detailed,
      other => unreachable!("validated scope-is must be canonical, got {other}"),
    })
}

fn template_values_state_matches(part: &ConditionalTemplatePart, values: &[String]) -> bool {
  part
    .values_state
    .as_deref()
    .is_none_or(|state| match state {
      "empty" => values.is_empty(),
      "present" => !values.is_empty(),
      other => unreachable!("validated values-state must be canonical, got {other}"),
    })
}

fn build_interpretations(
  direct_id: String,
  direct_refs: Vec<String>,
  rich_id: String,
  rich_refs: Vec<String>,
) -> Vec<pnix_core::ontology::Interpretation> {
  let mut interpretations = Vec::new();
  if !direct_refs.is_empty() {
    interpretations.push(judgment::interpretation_with_refs(
      direct_id,
      direct_refs.clone(),
    ));
  }
  if !rich_refs.is_empty() && rich_refs != direct_refs {
    interpretations.push(judgment::interpretation_with_refs(rich_id, rich_refs));
  }
  interpretations
}

fn fact_ids(facts: &[ContextualFact]) -> Vec<String> {
  facts
    .iter()
    .filter_map(|fact| fact.id.as_ref().map(|id| id.0.clone()))
    .collect()
}

fn fact_ids_for_predicates(facts: &[ContextualFact], predicates: &[&str]) -> Vec<String> {
  facts
    .iter()
    .filter(|fact| predicates.contains(&fact.pred.as_str()))
    .filter_map(|fact| fact.id.as_ref().map(|id| id.0.clone()))
    .collect()
}

fn route_runtime_rule<'a>(
  resources: &'a RuntimeResources,
  route: &str,
) -> Option<&'a KernelRouteRuntimeRule> {
  resources.route_runtime_rules.get(route)
}

fn validate_required_route_runtime_rules(
  query_classifiers: &QueryClassifierConfig,
  route_runtime_rules: &BTreeMap<String, KernelRouteRuntimeRule>,
) -> Result<()> {
  let definition_route = query_classifiers.dispatch_routes.definition.as_str();
  if let Some(definition_rule) = route_runtime_rules.get(definition_route) {
    if match definition_rule.direct_fact_predicates.as_ref() {
      Some(predicates) => predicates.is_empty(),
      None => true,
    } {
      return Err(anyhow!(
        "missing 'kernel-direct-fact-predicates' for definition route '{}'",
        definition_route
      ));
    }
    if match definition_rule.direct_interpretation_id.as_ref() {
      Some(id) => id.is_empty(),
      None => true,
    } {
      return Err(anyhow!(
        "missing 'kernel-direct-interpretation-id' for definition route '{}'",
        definition_route
      ));
    }
    if match definition_rule.rich_interpretation_id.as_ref() {
      Some(id) => id.is_empty(),
      None => true,
    } {
      return Err(anyhow!(
        "missing 'kernel-rich-interpretation-id' for definition route '{}'",
        definition_route
      ));
    }
  }

  let property_route = query_classifiers.dispatch_routes.property.as_str();
  if let Some(property_rule) = route_runtime_rules.get(property_route) {
    if match property_rule.interpretation_id.as_ref() {
      Some(id) => id.is_empty(),
      None => true,
    } {
      return Err(anyhow!(
        "missing 'kernel-interpretation-id' for property route '{}'",
        property_route
      ));
    }
  }
  Ok(())
}

fn required_definition_route_runtime_rule<'a>(
  resources: &'a RuntimeResources,
  route: &str,
) -> Result<&'a KernelRouteRuntimeRule> {
  route_runtime_rule(resources, route)
    .ok_or_else(|| err_missing_runtime("route runtime rule for definition route", route))
}

fn required_property_route_runtime_rule<'a>(
  resources: &'a RuntimeResources,
  route: &str,
) -> Result<&'a KernelRouteRuntimeRule> {
  route_runtime_rule(resources, route)
    .ok_or_else(|| err_missing_runtime("route runtime rule for property route", route))
}

fn route_interpretation_template(template: &str, replacements: &[(&str, &str)]) -> String {
  fill_template(template, replacements)
}

fn invert_interpretation_rule<'a>(
  resources: &'a RuntimeResources,
  trigger_type: &str,
) -> Result<&'a InvertInterpretationRule> {
  resources
    .invert
    .resolved_interpretation_rules
    .get(trigger_type)
    .ok_or_else(|| {
      anyhow!(
        "validated invert config must carry resolved interpretation rule for '{}'",
        trigger_type
      )
    })
}

fn invert_source_fact_included(rule: &InvertInterpretationRule, fact: &ContextualFact) -> bool {
  rule
    .source_include_predicates
    .iter()
    .any(|predicate| predicate == fact.pred.as_str())
    || rule
      .source_include_context_prefixes
      .iter()
      .any(|prefix| fact.context.0.starts_with(prefix.as_str()))
}

fn invert_direct_interpretation_id(
  rule: &InvertInterpretationRule,
  replacements: &[(&str, &str)],
) -> String {
  route_interpretation_template(rule.direct_interpretation_id.as_str(), replacements)
}

fn invert_rich_interpretation_id(
  rule: &InvertInterpretationRule,
  replacements: &[(&str, &str)],
) -> String {
  route_interpretation_template(rule.rich_interpretation_id.as_str(), replacements)
}

fn route_spec(resources: &RuntimeResources, route: &str) -> Result<QueryRouteSpec> {
  resources
    .query_routes
    .get(route)
    .cloned()
    .ok_or_else(|| err_missing_runtime("query route spec", route))
}

fn concepts_for_term<'a>(
  resources: &'a RuntimeResources,
  term: &str,
) -> Result<&'a Vec<ConceptDefinition>> {
  resources
    .concepts_by_term
    .get(term)
    .ok_or_else(|| anyhow!("unknown concept term: {term}"))
}

fn terms_for_domain(resources: &RuntimeResources, domain: &str) -> Vec<String> {
  resources
    .concepts_by_term
    .iter()
    .filter(|(_, concepts)| concepts.iter().any(|concept| concept.domain == domain))
    .map(|(term, _)| term.clone())
    .collect()
}

fn follow_up_for(
  resources: &RuntimeResources,
  term: Option<&str>,
  reason: &str,
) -> Result<(String, Vec<String>)> {
  let choices = resolved_followup_choices(resources, term)?;
  let resolved_term = resolved_followup_term(resources, term)?;
  let suggestions = choices.join(", ");
  let predicate = resources
    .followups
    .reason_question_rules
    .get(reason)
    .cloned()
    .ok_or_else(|| err_missing_runtime("follow-up reason route", reason))?;
  let question = resources
    .followups
    .disambiguation_questions
    .get(predicate.as_str())
    .cloned()
    .ok_or_else(|| err_missing_runtime("disambiguation question", &predicate))?;
  let question_text = fill_template(
    question.question_template.as_str(),
    &followup_template_replacements(
      question.question_template.as_str(),
      resolved_term.as_deref(),
      suggestions.as_str(),
    )?,
  );
  let choices_text = fill_template(
    question.choices_template.as_str(),
    &followup_template_replacements(
      question.choices_template.as_str(),
      resolved_term.as_deref(),
      suggestions.as_str(),
    )?,
  );
  let hint = match (question_text.is_empty(), choices_text.is_empty()) {
    (true, true) => {
      unreachable!("validated disambiguation question must produce non-empty hint component")
    }
    (false, true) => question_text,
    (true, false) => choices_text,
    (false, false) => format!("{} {}", question_text.trim_end(), choices_text.trim_start()),
  };
  Ok((hint, choices))
}

fn reopen_seed_term(
  resources: &RuntimeResources,
  utterance: &str,
  reopened: &HeldState,
) -> Result<Option<String>> {
  let rule = reopen_rule(resources, reopened.reason.as_str())?;
  let Some(term) = reopened.term.as_deref() else {
    return Ok(None);
  };
  Ok(match rule.carry_term_policy.as_str() {
    "always" => Some(term.to_string()),
    "never" => None,
    "when-missing" => {
      let current_terms = extract_candidate_terms(resources, utterance);
      (!current_terms.iter().any(|current| current == term)).then(|| term.to_string())
    }
    other => {
      unreachable!("validated carry-term-policy must be canonical, got {other}")
    }
  })
}

fn reopen_effective_utterance(
  resources: &RuntimeResources,
  utterance: &str,
  reopened: &HeldState,
  carry_term: Option<&str>,
) -> Result<String> {
  let rule = reopen_rule(resources, reopened.reason.as_str())?;
  Ok(fill_template(
    rule.effective_utterance_template.as_str(),
    &required_optional_template_replacements(
      rule.effective_utterance_template.as_str(),
      &[
        (
          "${term}",
          carry_term,
          "reopen effective-utterance template requires carried term",
        ),
        (
          "${utterance}",
          Some(utterance),
          "reopen effective-utterance template requires utterance",
        ),
      ],
    )?,
  ))
}

fn rule_matches_term_presence(when: &str, term: Option<&str>) -> bool {
  match when {
    "term-present" => term.is_some(),
    "term-missing" => term.is_none(),
    "always" => true,
    other => unreachable!("validated follow-up when must be canonical, got {other}"),
  }
}

fn term_presence_name(term: Option<&str>) -> &'static str {
  if term.is_some() {
    "term-present"
  } else {
    "term-missing"
  }
}

fn choice_rule_matches(resources: &RuntimeResources, when: &str, term: Option<&str>) -> bool {
  match when {
    "term-present-with-concept-choice" => {
      term.is_some_and(|term| resources.followups.concept_choices.contains_key(term))
    }
    "term-present-without-concept-choice" => {
      term.is_some_and(|term| !resources.followups.concept_choices.contains_key(term))
    }
    _ => rule_matches_term_presence(when, term),
  }
}

fn choice_rule_state_name(resources: &RuntimeResources, term: Option<&str>) -> &'static str {
  match term {
    Some(term) if resources.followups.concept_choices.contains_key(term) => {
      "term-present-with-concept-choice"
    }
    Some(_) => "term-present-without-concept-choice",
    None => "term-missing",
  }
}

fn resolved_followup_term(
  resources: &RuntimeResources,
  term: Option<&str>,
) -> Result<Option<String>> {
  for rule in &resources.followups.resolved_term_rules {
    if !rule_matches_term_presence(rule.when.as_str(), term) {
      continue;
    }
    return match rule.term_source.as_str() {
      "term" => Ok(term.map(str::to_string)),
      "literal" => Ok(Some(
        rule
          .value
          .as_ref()
          .expect("validated literal resolved-term-rules must have value")
          .clone(),
      )),
      "label" => Ok(Some(if let Some(value) = rule.value.as_ref() {
        value.clone()
      } else {
        resources.followups.unknown_term_label.clone()
      })),
      "none" => Ok(None),
      other => unreachable!("validated follow-up term-source must be canonical, got {other}"),
    };
  }
  Err(anyhow!(
    "no follow-up resolved-term rule matched term state '{}'",
    term_presence_name(term)
  ))
}

fn resolved_followup_choices(
  resources: &RuntimeResources,
  term: Option<&str>,
) -> Result<Vec<String>> {
  for rule in &resources.followups.choice_rules {
    if !choice_rule_matches(resources, rule.when.as_str(), term) {
      continue;
    }
    return match rule.choice_source.as_str() {
      "concept" => {
        let term =
          term.ok_or_else(|| anyhow!("follow-up choice-source 'concept' requires present term"))?;
        resources
          .followups
          .concept_choices
          .get(term)
          .cloned()
          .ok_or_else(|| err_missing_runtime("concept choices", term))
      }
      "default" => Ok(resources.followups.default_choices.clone()),
      "none" => Ok(vec![]),
      other => unreachable!("validated follow-up choice-source must be canonical, got {other}"),
    };
  }
  Err(anyhow!(
    "no follow-up choice rule matched term state '{}'",
    choice_rule_state_name(resources, term)
  ))
}

fn reopen_rule<'a>(resources: &'a RuntimeResources, reason: &str) -> Result<&'a ReopenRule> {
  resources
    .followups
    .reopen_rules
    .get(reason)
    .ok_or_else(|| err_missing_reopen_rule(reason))
}

fn held_response_rule<'a>(
  resources: &'a RuntimeResources,
  term: Option<&str>,
) -> Result<&'a HeldResponseRule> {
  resources
    .followups
    .held_response_rules
    .iter()
    .find(|rule| rule_matches_term_presence(rule.when.as_str(), term))
    .ok_or_else(|| {
      anyhow!(
        "no held-response rule matched term state '{}'",
        term_presence_name(term)
      )
    })
}

fn matching_property_classifier<'a>(
  resources: &'a RuntimeResources,
  utterance: &str,
) -> Option<&'a PredicateClassifier> {
  resources
    .query_classifiers
    .predicate_classifiers
    .iter()
    .find(|rule| text_match_rule_matches(&rule.rule, utterance))
}

fn classify_domain_listing(resources: &RuntimeResources, utterance: &str) -> Option<String> {
  let has_trigger = resources
    .query_classifiers
    .domain_listing_trigger_markers
    .iter()
    .any(|marker| utterance.contains(marker.as_str()));
  if !has_trigger {
    return None;
  }
  let has_list_intent = resources
    .query_classifiers
    .domain_list_intent_markers
    .iter()
    .any(|marker| utterance.contains(marker.as_str()));
  if !has_list_intent {
    return None;
  }
  resources
    .query_classifiers
    .domain_classifiers
    .iter()
    .find(|rule| utterance.contains(rule.keyword.as_str()))
    .map(|rule| rule.domain.clone())
}

fn looks_like_sentence_analysis_query(resources: &RuntimeResources, utterance: &str) -> bool {
  if resources
    .query_classifiers
    .question_word_stems
    .iter()
    .any(|stem| utterance.contains(stem.as_str()))
  {
    return false;
  }

  let analysis = analyze_korean_text(utterance);
  if analysis.tokens.len() < 2 {
    return false;
  }
  if !matches!(
    analysis.sentence_mood,
    KoreanSentenceMood::Declarative | KoreanSentenceMood::Interrogative
  ) {
    return false;
  }
  analysis.particles.iter().any(|particle| {
    matches!(
      particle.kind,
      KoreanParticleKind::Subject
        | KoreanParticleKind::Topic
        | KoreanParticleKind::Object
        | KoreanParticleKind::Locative
        | KoreanParticleKind::Dative
        | KoreanParticleKind::Instrumental
    )
  }) || analysis.sentence_mood != KoreanSentenceMood::Unknown
}

fn is_question_word_stem(resources: &RuntimeResources, stem: &str) -> bool {
  resources
    .query_classifiers
    .question_word_stems
    .iter()
    .any(|candidate| candidate == stem)
}

fn classify_cross_concept_query(
  resources: &RuntimeResources,
  utterance: &str,
) -> Option<(String, String)> {
  let has_cross_marker = resources
    .query_classifiers
    .cross_concept_markers
    .iter()
    .any(|marker| utterance.contains(marker.as_str()));
  if !has_cross_marker {
    return None;
  }

  let analysis = analyze_korean_text(utterance);
  let conjunctive = analysis
    .particles
    .iter()
    .find(|particle| particle.kind == KoreanParticleKind::Conjunctive)?;
  let term_a = canonicalize_term(resources, &conjunctive.stem);
  if term_a.is_empty() || is_question_word_stem(resources, term_a.as_str()) {
    return None;
  }

  let term_b = analysis
    .particles
    .iter()
    .find(|particle| particle.kind == KoreanParticleKind::Genitive)
    .map(|particle| canonicalize_term(resources, &particle.stem))
    .filter(|term| !term.is_empty())
    .or_else(|| {
      analysis
        .particles
        .iter()
        .find(|particle| {
          matches!(
            particle.kind,
            KoreanParticleKind::Subject | KoreanParticleKind::Topic
          ) && particle.stem != conjunctive.stem
        })
        .map(|particle| canonicalize_term(resources, &particle.stem))
        .filter(|term| {
          !term.is_empty() && term != &term_a && !is_question_word_stem(resources, term.as_str())
        })
    })?;

  let knows_a = resources.concepts_by_term.contains_key(term_a.as_str());
  let knows_b = resources.concepts_by_term.contains_key(term_b.as_str());
  (knows_a || knows_b).then_some((term_a, term_b))
}

fn looks_like_definition_query(resources: &RuntimeResources, utterance: &str) -> bool {
  resources
    .query_classifiers
    .definition_query_rules
    .iter()
    .any(|rule| text_match_rule_matches(rule, utterance))
}

fn best_invert_trigger<'a>(
  resources: &'a RuntimeResources,
  utterance: &str,
) -> Option<&'a InvertTrigger> {
  let matches = resources
    .invert
    .triggers
    .iter()
    .filter(|trigger| utterance.contains(trigger.pattern.as_str()))
    .collect::<Vec<_>>();
  if matches.is_empty() {
    return None;
  }
  match resources.invert.trigger_selection.as_str() {
    "list-order" => matches.into_iter().next(),
    "priority-then-list-order" => matches.into_iter().max_by_key(|trigger| trigger.priority),
    "priority-then-pattern-length" => matches
      .into_iter()
      .max_by_key(|trigger| (trigger.priority, trigger.pattern.chars().count())),
    other => unreachable!("validated trigger-selection must be canonical, got {other}"),
  }
}

fn query_context_for_route(
  defaults: &QueryRouteDefaults,
  raw_query_context: &str,
  route: &str,
  path: &Path,
) -> Result<String> {
  if let Some(rule) = defaults
    .query_context_rewrite_rules
    .iter()
    .find(|rule| raw_query_context.starts_with(rule.from.as_str()))
  {
    return Ok(format!(
      "{}{}",
      rule.to,
      &raw_query_context[rule.from.len()..]
    ));
  }
  if defaults
    .query_context_rewrite_rules
    .iter()
    .any(|rule| raw_query_context.starts_with(rule.to.as_str()))
  {
    // Already canonical: starts with an existing rewrite target prefix.
    return Ok(raw_query_context.to_string());
  }
  Err(anyhow!(
    "'query-context' for route '{}' in {} ('{}') does not match any 'query-context-rewrite-rules' 'from' or 'to' prefix; canonical entries must either be rewritten by an explicit rule or already carry a canonical 'to' prefix",
    route,
    path.display(),
    raw_query_context
  ))
}

fn text_match_rule_matches(rule: &TextMatchRule, utterance: &str) -> bool {
  let any_match = rule.match_any.as_ref().is_none_or(|values| {
    values
      .iter()
      .any(|value| utterance.contains(value.as_str()))
  });
  let all_match = rule.match_all.as_ref().is_none_or(|values| {
    values
      .iter()
      .all(|value| utterance.contains(value.as_str()))
  });
  any_match && all_match
}

fn continuation_classifier_matches(rule: &ContinuationClassifier, utterance: &str) -> bool {
  if rule
    .match_any
    .iter()
    .any(|value| utterance.contains(value.as_str()))
  {
    return true;
  }
  rule
    .match_all_pairs
    .iter()
    .any(|group| group.iter().all(|value| utterance.contains(value.as_str())))
}

fn classify_continuation(
  resources: &RuntimeResources,
  utterance: &str,
) -> Option<KernelContinuationKind> {
  let kind = resources
    .query_classifiers
    .continuation_classifiers
    .iter()
    .find(|rule| continuation_classifier_matches(rule, utterance))?
    .kind
    .as_str();
  match kind {
    "elaborate" => Some(KernelContinuationKind::Elaborate),
    "example" => Some(KernelContinuationKind::Example),
    "related" => Some(KernelContinuationKind::Related),
    _ => None,
  }
}

fn resolve_truth_regime(
  resources: &RuntimeResources,
  term: &str,
  trigger: &InvertTrigger,
) -> String {
  if trigger.truth_regime != "auto" {
    return trigger.truth_regime.clone();
  }
  let domain = resources
    .concepts_by_term
    .get(term)
    .and_then(|concepts| concepts.first())
    .map(|concept| concept.domain.as_str())
    .filter(|domain| !domain.is_empty())
    .unwrap_or("*");
  resources
    .invert
    .domain_to_regime
    .iter()
    .filter(|(prefix, _)| prefix == "*" || domain.starts_with(prefix.as_str()))
    .max_by_key(|(prefix, _)| {
      if prefix == "*" {
        0
      } else {
        prefix.chars().count()
      }
    })
    .map(|(_, regime)| regime.clone())
    .unwrap_or_else(|| {
      unreachable!("validated ontology-invert config must provide wildcard truth-regime fallback")
    })
}

fn extract_candidate_terms(resources: &RuntimeResources, utterance: &str) -> Vec<String> {
  let analysis = analyze_korean_text(utterance);
  let mut terms = Vec::new();
  for particle in &analysis.particles {
    if resources
      .query_classifiers
      .term_extraction_particle_kinds
      .iter()
      .any(|kind| kind.eq_ignore_ascii_case(particle.kind.as_str()))
    {
      let term = canonicalize_term(resources, &particle.stem);
      if !term.is_empty()
        && !resources
          .query_classifiers
          .question_word_stems
          .iter()
          .any(|stem| stem == term.as_str())
        && !terms.contains(&term)
      {
        terms.push(term);
      }
    }
  }
  for suffix in &resources.query_classifiers.term_extraction_suffixes {
    if let Some(pos) = utterance.find(suffix.as_str()) {
      if let Some(word) = utterance[..pos].split_whitespace().next_back() {
        let term = canonicalize_term(resources, word);
        if !term.is_empty() && !terms.contains(&term) {
          terms.push(term);
        }
      }
    }
  }
  for suffix in &resources.query_classifiers.concept_definition_suffixes {
    if let Some(pos) = utterance.find(suffix.as_str()) {
      if let Some(word) = utterance[..pos].split_whitespace().next_back() {
        let term = canonicalize_term(resources, word);
        if !term.is_empty()
          && !resources
            .query_classifiers
            .question_word_stems
            .iter()
            .any(|stem| stem == term.as_str())
          && !terms.contains(&term)
        {
          terms.push(term);
        }
      }
    }
  }
  if terms.is_empty() {
    match resources.query_classifiers.term_fallback_policy.as_str() {
      "known-concept-token-scan" => {
        for token in utterance.split_whitespace() {
          let term = canonicalize_term(resources, token);
          if !term.is_empty()
            && resources.concepts_by_term.contains_key(term.as_str())
            && !terms.contains(&term)
          {
            terms.push(term);
          }
        }
      }
      "disabled" => {}
      other => unreachable!("validated term-fallback-policy must be canonical, got {other}"),
    }
  }
  terms
}

fn sentence_role_label_ko(kind: KoreanParticleKind) -> &'static str {
  match kind {
    KoreanParticleKind::Subject => "주어",
    KoreanParticleKind::Topic => "화제",
    KoreanParticleKind::Object => "목적어",
    KoreanParticleKind::Locative => "장소",
    KoreanParticleKind::Genitive => "소유",
    KoreanParticleKind::Conjunctive => "접속",
    KoreanParticleKind::Instrumental => "수단",
    KoreanParticleKind::Dative => "대상",
  }
}

fn sentence_analysis_response_text(analysis: &pnix_core::lang::KoreanTextAnalysis) -> String {
  let mut parts = vec![format!("문장 분위기: {}", analysis.sentence_mood.as_str())];
  if let Some(final_token) = analysis
    .final_token
    .as_deref()
    .filter(|token| !token.is_empty())
  {
    parts.push(format!("동사 표현: {final_token}"));
  }
  if !analysis.particles.is_empty() {
    let roles = analysis
      .particles
      .iter()
      .map(|particle| {
        format!(
          "{}={}",
          particle.stem,
          sentence_role_label_ko(particle.kind)
        )
      })
      .collect::<Vec<_>>()
      .join(", ");
    parts.push(format!("조사 기반 역할: {roles}"));
  }
  parts.join(". ")
}

fn canonicalize_term(resources: &RuntimeResources, raw: &str) -> String {
  let trim_chars = resources
    .query_classifiers
    .term_normalization_trim_chars
    .iter()
    .flat_map(|value| value.chars())
    .collect::<Vec<_>>();
  let trimmed = raw
    .trim()
    .trim_matches(|c: char| trim_chars.contains(&c))
    .to_string();
  if trimmed.is_empty() {
    return trimmed;
  }
  resources
    .synonyms
    .get(trimmed.as_str())
    .cloned()
    .unwrap_or(trimmed)
}

fn load_concepts(dir: &Path) -> Result<BTreeMap<String, Vec<ConceptDefinition>>> {
  let mut files = std::fs::read_dir(dir)
    .with_context(|| format!("read concept directory {}", dir.display()))?
    .filter_map(|entry| entry.ok())
    .map(|entry| entry.path())
    .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("px"))
    .filter(|path| path.file_name().and_then(|s| s.to_str()) != Some("synonyms.px"))
    .collect::<Vec<_>>();
  files.sort();

  let mut concepts_by_term: BTreeMap<String, Vec<ConceptDefinition>> = BTreeMap::new();
  for path in files {
    let value = parse_px_file(&path)?;
    for concept in concepts_from_px_value(&value, &path)? {
      concepts_by_term
        .entry(concept.term_ko.clone())
        .or_default()
        .push(concept);
    }
  }
  Ok(concepts_by_term)
}

fn concepts_from_px_value(value: &PxValue, path: &Path) -> Result<Vec<ConceptDefinition>> {
  match value {
    PxValue::List(items) => items
      .iter()
      .map(|item| concept_from_attrset(item, path))
      .collect(),
    PxValue::AttrSet(map) => {
      if let Some(list) = map.get("concepts") {
        return concepts_from_px_value(list, path);
      }
      Ok(vec![concept_from_attrset(value, path)?])
    }
    _ => Err(anyhow!(
      "expected concept list or attrset in {}",
      path.display()
    )),
  }
}

fn concept_from_attrset(value: &PxValue, path: &Path) -> Result<ConceptDefinition> {
  let map = value
    .as_attrset()
    .ok_or_else(|| anyhow!("concept entry must be attrset in {}", path.display()))?;
  let mut scalar_fields = BTreeMap::new();
  let mut list_fields = BTreeMap::new();
  for (key, value) in map {
    if let Some(s) = value.as_str() {
      scalar_fields.insert(key.clone(), s.to_string());
      continue;
    }
    if let PxValue::List(_) = value {
      let list = value.as_string_list();
      if !list.is_empty() {
        list_fields.insert(key.clone(), list);
      }
    }
  }
  Ok(ConceptDefinition {
    term_ko: required_attrset_string(map, "term-ko", "concept entry", path)?,
    definition_ko: required_attrset_string(map, "definition-ko", "concept entry", path)?,
    formal_symbol: optional_attrset_string(map, "formal-symbol", "concept entry", path)?
      .filter(|value| !value.is_empty()),
    context: required_attrset_string(map, "context", "concept entry", path)?,
    domain: required_attrset_string(map, "domain", "concept entry", path)?,
    related_concepts: optional_attrset_string_list(map, "related-concepts", "concept entry", path)?
      .filter(|values| !values.is_empty()),
    formula: optional_attrset_string(map, "formula", "concept entry", path)?
      .filter(|value| !value.is_empty()),
    why: optional_attrset_string(map, "why", "concept entry", path)?
      .filter(|value| !value.is_empty()),
    boundary_conditions: optional_attrset_string(
      map,
      "boundary-conditions",
      "concept entry",
      path,
    )?
    .filter(|value| !value.is_empty()),
    source_ref: path.display().to_string(),
    scalar_fields,
    list_fields,
  })
}

fn load_synonyms(path: &Path) -> Result<BTreeMap<String, String>> {
  let value = parse_px_file(path)?;
  let root = value
    .as_attrset()
    .ok_or_else(|| anyhow!("synonyms root must be attrset in {}", path.display()))?;
  let mut result = BTreeMap::new();
  let Some(PxValue::List(items)) = root.get("synonym-groups") else {
    return Err(err_missing(path, "synonym-groups"));
  };
  for item in items {
    let Some(group) = item.as_attrset() else {
      return Err(err_invalid_entry(path, "synonym-groups"));
    };
    let canonical = required_attrset_string(group, "canonical", "synonym-groups entry", path)?;
    for alias in present_attrset_string_list(group, "aliases", "synonym-groups entry", path)? {
      result.insert(alias, canonical.clone());
    }
    result.insert(canonical.clone(), canonical);
  }
  Ok(result)
}

fn load_korean_morphology(path: &Path) -> Result<KoreanMorphologyConfig> {
  let value = parse_px_file(path)?;
  let root = value.as_attrset().ok_or_else(|| {
    anyhow!(
      "korean morphology root must be attrset in {}",
      path.display()
    )
  })?;
  let mut config = KoreanMorphologyConfig::default();

  let Some(PxValue::List(items)) = root.get("clause-connectors") else {
    return Err(err_missing(path, "clause-connectors"));
  };
  for item in items {
    let Some(map) = item.as_attrset() else {
      return Err(err_invalid_entry(path, "clause-connectors"));
    };
    config.clause_connectors.push(ClauseConnector {
      connector: required_attrset_string(map, "connector", "clause-connectors entry", path)?,
      relation: required_attrset_string(map, "relation", "clause-connectors entry", path)?,
      label_ko: required_attrset_string(map, "ko", "clause-connectors entry", path)?,
    });
  }

  config.quotation_markers = required_top_level_string_list(&value, "quotation-markers", path)?;
  let response_templates = root
    .get("continuation-response-templates")
    .and_then(PxValue::as_attrset)
    .ok_or_else(|| err_missing(path, "continuation-response-templates"))?;
  for key in [
    "elaborate-header",
    "example-header",
    "example-formula",
    "example-definition",
    "example-related",
    "related-empty",
    "related-header",
    "summary",
  ] {
    config.continuation_response_templates.insert(
      key.to_string(),
      required_attrset_string(
        response_templates,
        key,
        "continuation-response-templates",
        path,
      )?,
    );
  }
  config.recipe_command_strip_words =
    required_top_level_string_list(&value, "recipe-command-strip-words", path)?;
  let context_merge_templates = root
    .get("context-merge-templates")
    .and_then(PxValue::as_attrset)
    .ok_or_else(|| err_missing(path, "context-merge-templates"))?;
  config.recipe_shell_command_template = required_attrset_string(
    context_merge_templates,
    "recipe-shell-command",
    "context-merge-templates",
    path,
  )?;
  let dispatch_summary_templates = root
    .get("dispatch-summary-templates")
    .and_then(PxValue::as_attrset)
    .ok_or_else(|| err_missing(path, "dispatch-summary-templates"))?;
  config.os_recipe_summary_template = required_attrset_string(
    dispatch_summary_templates,
    "os-recipe",
    "dispatch-summary-templates",
    path,
  )?;
  let light_dispatch_templates = root
    .get("light-dispatch-templates")
    .and_then(PxValue::as_attrset)
    .ok_or_else(|| err_missing(path, "light-dispatch-templates"))?;
  config.light_handoff_summary_template = required_attrset_string(
    light_dispatch_templates,
    "summary-handoff-no-computation",
    "light-dispatch-templates",
    path,
  )?;
  Ok(config)
}

fn load_query_classifiers(path: &Path) -> Result<QueryClassifierConfig> {
  const SOURCE_FACT_OBJECT_PLACEHOLDERS: &[&str] = &[
    "${term}",
    "${field}",
    "${predicate}",
    "${value}",
    "${context}",
  ];
  const SOURCE_LIST_OBJECT_PLACEHOLDERS: &[&str] = &[
    "${term}",
    "${field}",
    "${predicate}",
    "${value}",
    "${index}",
    "${context}",
  ];
  const SOURCE_METADATA_FIELD_OBJECT_PLACEHOLDERS: &[&str] =
    &["${term}", "${field}", "${source-predicate}"];
  const SOURCE_METADATA_VALUE_OBJECT_PLACEHOLDERS: &[&str] =
    &["${term}", "${field}", "${value}", "${source-predicate}"];
  const SOURCE_METADATA_LIST_FIELD_OBJECT_PLACEHOLDERS: &[&str] =
    &["${term}", "${field}", "${source-predicate}"];
  const SOURCE_METADATA_LIST_ITEM_OBJECT_PLACEHOLDERS: &[&str] = &[
    "${term}",
    "${field}",
    "${value}",
    "${index}",
    "${source-predicate}",
  ];
  // 헌법 §20 정합 path: literal-only `parse_px_file` 가 받지 못하는
  // generic constructor / `import + ++` expression 도 받을 수 있도록
  // nix-eval fallback variant 를 사용한다. literal-only file 은 first-pass
  // 그대로 통과 (attrset literal); expression 이면 pnix-eval 로 evaluate
  // 한 결과를 PxValue 로 lower.
  let value = parse_px_file_with_pnix_eval_fallback(path)?;
  if value.as_attrset().is_none() {
    return Err(anyhow!(
      "query-classifiers root must be attrset in {}",
      path.display()
    ));
  }
  let mut config = QueryClassifierConfig::default();
  config.query_dispatch_priority =
    required_top_level_string_list(&value, "query-dispatch-priority", path)?;
  for stage in &config.query_dispatch_priority {
    match stage.as_str() {
      "why" | "property" | "definition" => {}
      _ => {
        return Err(anyhow!(
          "invalid 'query-dispatch-priority' entry '{}' in {}",
          stage,
          path.display()
        ));
      }
    }
  }
  if let Some(raw_routes) = value.get("kernel-dispatch-routes") {
    let Some(routes) = raw_routes.as_attrset() else {
      return Err(err_wrong_type(path, "kernel-dispatch-routes", "attrset"));
    };
    config.dispatch_routes.definition =
      required_attrset_string(routes, "definition", "kernel-dispatch-routes", path)?;
    config.dispatch_routes.property =
      required_attrset_string(routes, "property", "kernel-dispatch-routes", path)?;
    config.dispatch_routes.held =
      required_attrset_string(routes, "held", "kernel-dispatch-routes", path)?;
  } else {
    return Err(err_missing(path, "kernel-dispatch-routes"));
  }
  if let Some(raw_reason_keys) = value.get("held-reason-keys") {
    let Some(reason_keys) = raw_reason_keys.as_attrset() else {
      return Err(err_wrong_type(path, "held-reason-keys", "attrset"));
    };
    config.held_reason_keys.requires_context =
      required_attrset_string(reason_keys, "requires-context", "held-reason-keys", path)?;
    config.held_reason_keys.unknown_term =
      required_attrset_string(reason_keys, "unknown-term", "held-reason-keys", path)?;
  } else {
    return Err(err_missing(path, "held-reason-keys"));
  }
  let raw_rules = value
    .get("held-reason-rules")
    .ok_or_else(|| err_missing(path, "held-reason-rules"))?;
  let PxValue::List(items) = raw_rules else {
    return Err(err_wrong_type(path, "held-reason-rules", "list"));
  };
  for item in items {
    let Some(map) = item.as_attrset() else {
      return Err(err_invalid_entry(path, "held-reason-rules"));
    };
    let when = required_attrset_string(map, "when", "held-reason-rules entry", path)?;
    let reason_key = required_attrset_string(map, "reason-key", "held-reason-rules entry", path)?;
    let term_source = required_attrset_string(map, "term-source", "held-reason-rules entry", path)?;
    if !matches!(when.as_str(), "known-term" | "unknown-term") {
      return Err(anyhow!(
        "invalid 'when' for held-reason-rules entry in {}",
        path.display()
      ));
    }
    if !matches!(reason_key.as_str(), "requires-context" | "unknown-term") {
      return Err(anyhow!(
        "invalid 'reason-key' for held-reason-rules entry in {}",
        path.display()
      ));
    }
    if !matches!(
      term_source.as_str(),
      "matched-term" | "first-extracted-term" | "none"
    ) {
      return Err(anyhow!(
        "invalid 'term-source' for held-reason-rules entry in {}",
        path.display()
      ));
    }
    if config.held_reason_rules.iter().any(|r| r.when == when) {
      return Err(err_duplicate_entry(
        path,
        "held-reason-rules",
        "when",
        &when,
      ));
    }
    config.held_reason_rules.push(HeldReasonRule {
      when,
      reason_key,
      term_source,
    });
  }
  let raw_fields = value
    .get("kernel-source-fact-fields")
    .ok_or_else(|| err_missing(path, "kernel-source-fact-fields"))?;
  let PxValue::List(items) = raw_fields else {
    return Err(err_wrong_type(path, "kernel-source-fact-fields", "list"));
  };
  for item in items {
    let Some(map) = item.as_attrset() else {
      return Err(err_invalid_entry(path, "kernel-source-fact-fields"));
    };
    let field = required_attrset_string(map, "field", "kernel-source-fact-fields entry", path)?;
    let predicate =
      required_attrset_string(map, "predicate", "kernel-source-fact-fields entry", path)?;
    let context = optional_attrset_string(map, "context", "kernel-source-fact-fields entry", path)?;
    let layer = optional_attrset_string(map, "layer", "kernel-source-fact-fields entry", path)?
      .unwrap_or_else(|| "L1".to_string());
    let status =
      optional_attrset_meaning_status(map, "status", "kernel-source-fact-fields entry", path)?
        .unwrap_or(MeaningStatus::Accepted);
    let confidence =
      optional_attrset_f64(map, "confidence", "kernel-source-fact-fields entry", path)?
        .unwrap_or(1.0);
    if !(0.0..=1.0).contains(&confidence) {
      return Err(anyhow!(
        "'confidence' in kernel-source-fact-fields entry must be between 0.0 and 1.0 in {}",
        path.display()
      ));
    }
    let object_template = optional_attrset_string(
      map,
      "object-template",
      "kernel-source-fact-fields entry",
      path,
    )?
    .unwrap_or_else(|| "${value}".to_string());
    validate_placeholder_allowlist(
      &object_template,
      SOURCE_FACT_OBJECT_PLACEHOLDERS,
      "kernel-source-fact-fields 'object-template'",
      path,
    )?;
    if config.source_fact_fields.iter().any(|r| r.field == field) {
      return Err(err_duplicate_entry(
        path,
        "kernel-source-fact-fields",
        "field",
        &field,
      ));
    }
    config.source_fact_fields.push(SourceFactFieldRule {
      field,
      predicate,
      context,
      layer,
      status,
      confidence,
      object_template,
    });
  }
  let raw_fields = value
    .get("kernel-source-list-fields")
    .ok_or_else(|| err_missing(path, "kernel-source-list-fields"))?;
  let PxValue::List(items) = raw_fields else {
    return Err(err_wrong_type(path, "kernel-source-list-fields", "list"));
  };
  for item in items {
    let Some(map) = item.as_attrset() else {
      return Err(err_invalid_entry(path, "kernel-source-list-fields"));
    };
    let field = required_attrset_string(map, "field", "kernel-source-list-fields entry", path)?;
    let predicate =
      required_attrset_string(map, "predicate", "kernel-source-list-fields entry", path)?;
    let context = optional_attrset_string(map, "context", "kernel-source-list-fields entry", path)?;
    let layer = optional_attrset_string(map, "layer", "kernel-source-list-fields entry", path)?
      .unwrap_or_else(|| "L1".to_string());
    let status =
      optional_attrset_meaning_status(map, "status", "kernel-source-list-fields entry", path)?
        .unwrap_or(MeaningStatus::Accepted);
    let confidence =
      optional_attrset_f64(map, "confidence", "kernel-source-list-fields entry", path)?
        .unwrap_or(1.0);
    if !(0.0..=1.0).contains(&confidence) {
      return Err(anyhow!(
        "'confidence' in kernel-source-list-fields entry must be between 0.0 and 1.0 in {}",
        path.display()
      ));
    }
    let object_template = optional_attrset_string(
      map,
      "object-template",
      "kernel-source-list-fields entry",
      path,
    )?
    .unwrap_or_else(|| "${value}".to_string());
    validate_placeholder_allowlist(
      &object_template,
      SOURCE_LIST_OBJECT_PLACEHOLDERS,
      "kernel-source-list-fields 'object-template'",
      path,
    )?;
    if config.source_list_fields.iter().any(|r| r.field == field) {
      return Err(err_duplicate_entry(
        path,
        "kernel-source-list-fields",
        "field",
        &field,
      ));
    }
    config.source_list_fields.push(SourceFactListRule {
      field,
      predicate,
      context,
      layer,
      status,
      confidence,
      object_template,
    });
  }
  if let Some(raw_metadata) = value.get("kernel-source-metadata") {
    let Some(metadata) = raw_metadata.as_attrset() else {
      return Err(err_wrong_type(path, "kernel-source-metadata", "attrset"));
    };
    let field_object_template = optional_attrset_string(
      metadata,
      "field-object-template",
      "kernel-source-metadata",
      path,
    )?
    .unwrap_or_else(|| "${field}".to_string());
    let value_object_template = optional_attrset_string(
      metadata,
      "value-object-template",
      "kernel-source-metadata",
      path,
    )?
    .unwrap_or_else(|| "${field}=${value}".to_string());
    let list_field_object_template = optional_attrset_string(
      metadata,
      "list-field-object-template",
      "kernel-source-metadata",
      path,
    )?
    .unwrap_or_else(|| "${field}".to_string());
    let list_item_object_template = optional_attrset_string(
      metadata,
      "list-item-object-template",
      "kernel-source-metadata",
      path,
    )?
    .unwrap_or_else(|| "${field}=${value}".to_string());
    validate_placeholder_allowlist(
      &field_object_template,
      SOURCE_METADATA_FIELD_OBJECT_PLACEHOLDERS,
      "kernel-source-metadata 'field-object-template'",
      path,
    )?;
    validate_placeholder_allowlist(
      &value_object_template,
      SOURCE_METADATA_VALUE_OBJECT_PLACEHOLDERS,
      "kernel-source-metadata 'value-object-template'",
      path,
    )?;
    validate_placeholder_allowlist(
      &list_field_object_template,
      SOURCE_METADATA_LIST_FIELD_OBJECT_PLACEHOLDERS,
      "kernel-source-metadata 'list-field-object-template'",
      path,
    )?;
    validate_placeholder_allowlist(
      &list_item_object_template,
      SOURCE_METADATA_LIST_ITEM_OBJECT_PLACEHOLDERS,
      "kernel-source-metadata 'list-item-object-template'",
      path,
    )?;
    let confidence =
      optional_attrset_f64(metadata, "confidence", "kernel-source-metadata", path)?.unwrap_or(1.0);
    if !(0.0..=1.0).contains(&confidence) {
      return Err(anyhow!(
        "'confidence' in kernel-source-metadata must be between 0.0 and 1.0 in {}",
        path.display()
      ));
    }
    config.source_metadata = SourceFactMetadataConfig {
      context: optional_attrset_string(metadata, "context", "kernel-source-metadata", path)?
        .unwrap_or_else(|| "Pnix.KernelSource".to_string()),
      layer: optional_attrset_string(metadata, "layer", "kernel-source-metadata", path)?
        .unwrap_or_else(|| "L1".to_string()),
      status: optional_attrset_meaning_status(metadata, "status", "kernel-source-metadata", path)?
        .unwrap_or(MeaningStatus::Accepted),
      confidence,
      field_predicate: required_attrset_string(
        metadata,
        "field-predicate",
        "kernel-source-metadata",
        path,
      )?,
      value_predicate: required_attrset_string(
        metadata,
        "value-predicate",
        "kernel-source-metadata",
        path,
      )?,
      list_field_predicate: required_attrset_string(
        metadata,
        "list-field-predicate",
        "kernel-source-metadata",
        path,
      )?,
      list_item_predicate: required_attrset_string(
        metadata,
        "list-item-predicate",
        "kernel-source-metadata",
        path,
      )?,
      field_object_template,
      value_object_template,
      list_field_object_template,
      list_item_object_template,
    };
  }
  if let Some(raw_rules) = value.get("handoff-classifiers") {
    let PxValue::List(items) = raw_rules else {
      return Err(err_wrong_type(path, "handoff-classifiers", "list"));
    };
    for item in items {
      let Some(map) = item.as_attrset() else {
        return Err(err_invalid_entry(path, "handoff-classifiers"));
      };
      let template_id =
        required_attrset_string(map, "template-id", "handoff-classifiers entry", path)?;
      let tags = optional_attrset_string_list(map, "tags", "handoff-classifiers entry", path)?
        .unwrap_or_default();
      let execution_owner =
        required_attrset_string(map, "execution-owner", "handoff-classifiers entry", path)?;
      let visibility =
        required_attrset_string(map, "visibility", "handoff-classifiers entry", path)?;
      let match_any =
        optional_attrset_string_list(map, "match-any", "handoff-classifiers entry", path)?
          .unwrap_or_default();
      let match_terms =
        optional_attrset_string_list(map, "match-terms", "handoff-classifiers entry", path)?
          .unwrap_or_default();
      let match_units =
        optional_attrset_string_list(map, "match-units", "handoff-classifiers entry", path)?
          .unwrap_or_default();
      if match_any.is_empty() && (match_terms.is_empty() || match_units.is_empty()) {
        return Err(err_invalid_entry(path, "handoff-classifiers"));
      }
      let handoff_route =
        optional_attrset_string(map, "handoff-route", "handoff-classifiers entry", path)?;
      config.handoff_classifiers.push(HandoffClassifier {
        template_id,
        tags,
        execution_owner,
        visibility,
        match_any,
        match_terms,
        match_units,
        handoff_route,
      });
    }
  }
  config.concept_what_markers =
    present_top_level_string_list(&value, "concept-what-markers", path)?;
  config.concept_definition_suffixes =
    present_top_level_string_list(&value, "concept-definition-suffixes", path)?;
  config.concept_explain_markers =
    present_top_level_string_list(&value, "concept-explain-markers", path)?;
  config.concept_explain_skip_tokens =
    present_top_level_string_list(&value, "concept-explain-skip-tokens", path)?;
  config.cross_concept_markers =
    optional_top_level_string_list(&value, "cross-concept-markers", path)?.unwrap_or_default();
  config.domain_listing_trigger_markers =
    optional_top_level_string_list(&value, "domain-listing-trigger-markers", path)?
      .unwrap_or_default();
  config.domain_list_intent_markers =
    optional_top_level_string_list(&value, "domain-list-intent-markers", path)?.unwrap_or_default();
  config.os_execution_owner_markers =
    optional_top_level_string_list(&value, "os-execution-owner-markers", path)?.unwrap_or_default();
  config.question_word_stems = present_top_level_string_list(&value, "question-word-stems", path)?;
  if let Some(raw_rules) = value.get("continuation-classifiers") {
    let PxValue::List(items) = raw_rules else {
      return Err(err_wrong_type(path, "continuation-classifiers", "list"));
    };
    for item in items {
      let Some(map) = item.as_attrset() else {
        return Err(err_invalid_entry(path, "continuation-classifiers"));
      };
      let kind = required_attrset_string(map, "kind", "continuation-classifiers entry", path)?;
      require_allowed_literal(
        kind.as_str(),
        "kind",
        "continuation-classifiers entry",
        path,
        &["elaborate", "example", "related"],
      )?;
      let match_any =
        optional_attrset_string_list(map, "match-any", "continuation-classifiers entry", path)?
          .unwrap_or_default();
      let match_all_pairs = match map.get("match-all-pairs") {
        Some(PxValue::List(groups)) => groups
          .iter()
          .map(|group| {
            let PxValue::List(values) = group else {
              return Err(err_wrong_type(
                path,
                "match-all-pairs group",
                "continuation-classifiers entry list",
              ));
            };
            let values = values
              .iter()
              .map(|value| {
                value.as_str().map(str::to_string).ok_or_else(|| {
                  err_wrong_type(
                    path,
                    "match-all-pairs item",
                    "continuation-classifiers entry string",
                  )
                })
              })
              .collect::<Result<Vec<_>>>()?;
            if values.is_empty() {
              return Err(anyhow!(
                "empty matcher list in 'continuation-classifiers' entry in {}",
                path.display()
              ));
            }
            Ok(values)
          })
          .collect::<Result<Vec<_>>>()?,
        Some(_) => return Err(err_wrong_type(path, "match-all-pairs", "list")),
        None => Vec::new(),
      };
      if match_any.is_empty() && match_all_pairs.is_empty() {
        return Err(err_invalid_entry(path, "continuation-classifiers"));
      }
      config
        .continuation_classifiers
        .push(ContinuationClassifier {
          kind,
          match_any,
          match_all_pairs,
        });
    }
  }
  config.term_extraction_suffixes =
    present_top_level_string_list(&value, "term-extraction-suffixes", path)?;
  config.term_extraction_particle_kinds =
    required_top_level_string_list(&value, "term-extraction-particle-kinds", path)?;
  config.term_normalization_trim_chars =
    required_top_level_string_list(&value, "term-normalization-trim-chars", path)?;
  config.term_fallback_policy = required_top_level_string(&value, "term-fallback-policy", path)?;
  require_allowed_literal(
    config.term_fallback_policy.as_str(),
    "term-fallback-policy",
    "query-classifiers",
    path,
    &["known-concept-token-scan", "disabled"],
  )?;
  let raw_rules = value
    .get("definition-query-rules")
    .ok_or_else(|| err_missing(path, "definition-query-rules"))?;
  let PxValue::List(items) = raw_rules else {
    return Err(err_wrong_type(path, "definition-query-rules", "list"));
  };
  for item in items {
    let Some(map) = item.as_attrset() else {
      return Err(err_invalid_entry(path, "definition-query-rules"));
    };
    let rule = TextMatchRule {
      match_any: optional_attrset_string_list(
        map,
        "match-any",
        "definition-query-rules entry",
        path,
      )?,
      match_all: optional_attrset_string_list(
        map,
        "match-all",
        "definition-query-rules entry",
        path,
      )?,
    };
    if rule.match_any.is_none() && rule.match_all.is_none() {
      return Err(err_invalid_entry(path, "definition-query-rules"));
    }
    if rule.match_any.as_ref().is_some_and(Vec::is_empty)
      || rule.match_all.as_ref().is_some_and(Vec::is_empty)
    {
      return Err(anyhow!(
        "empty matcher list in 'definition-query-rules' entry in {}",
        path.display()
      ));
    }
    config.definition_query_rules.push(rule);
  }
  if config.definition_query_rules.is_empty() {
    return Err(err_missing(path, "definition-query-rules"));
  }
  if config.held_reason_rules.is_empty() {
    return Err(err_missing(path, "held-reason-rules"));
  }
  if config.source_fact_fields.is_empty() {
    return Err(err_missing(path, "kernel-source-fact-fields"));
  }
  if config.source_list_fields.is_empty() {
    return Err(err_missing(path, "kernel-source-list-fields"));
  }
  let raw_rules = value
    .get("predicate-classifiers")
    .ok_or_else(|| err_missing(path, "predicate-classifiers"))?;
  let PxValue::List(items) = raw_rules else {
    return Err(err_wrong_type(path, "predicate-classifiers", "list"));
  };
  for item in items {
    let Some(map) = item.as_attrset() else {
      return Err(err_invalid_entry(path, "predicate-classifiers"));
    };
    let predicate = required_attrset_string(map, "predicate", "predicate-classifiers entry", path)?;
    let label_ko = required_attrset_string(map, "label-ko", "predicate-classifiers entry", path)?;
    let rule = TextMatchRule {
      match_any: optional_attrset_string_list(
        map,
        "match-any",
        "predicate-classifiers entry",
        path,
      )?,
      match_all: optional_attrset_string_list(
        map,
        "match-all",
        "predicate-classifiers entry",
        path,
      )?,
    };
    if rule.match_any.is_none() && rule.match_all.is_none() {
      return Err(err_invalid_entry(path, "predicate-classifiers"));
    }
    if rule.match_any.as_ref().is_some_and(Vec::is_empty)
      || rule.match_all.as_ref().is_some_and(Vec::is_empty)
    {
      return Err(anyhow!(
        "empty matcher list in 'predicate-classifiers' entry in {}",
        path.display()
      ));
    }
    if config
      .predicate_classifiers
      .iter()
      .any(|c| c.predicate == predicate)
    {
      return Err(err_duplicate_entry(
        path,
        "predicate-classifiers",
        "predicate",
        &predicate,
      ));
    }
    config.predicate_classifiers.push(PredicateClassifier {
      rule,
      predicate,
      label_ko,
    });
  }
  if let Some(raw_rules) = value.get("domain-classifiers") {
    let PxValue::List(items) = raw_rules else {
      return Err(err_wrong_type(path, "domain-classifiers", "list"));
    };
    for item in items {
      let Some(map) = item.as_attrset() else {
        return Err(err_invalid_entry(path, "domain-classifiers"));
      };
      let keyword = required_attrset_string(map, "keyword", "domain-classifiers entry", path)?;
      let domain = required_attrset_string(map, "domain", "domain-classifiers entry", path)?;
      if config
        .domain_classifiers
        .iter()
        .any(|classifier| classifier.keyword == keyword)
      {
        return Err(err_duplicate_entry(
          path,
          "domain-classifiers",
          "keyword",
          &keyword,
        ));
      }
      config
        .domain_classifiers
        .push(DomainClassifier { keyword, domain });
    }
  }
  Ok(config)
}

fn load_kernel_base_facts(
  path: &Path,
) -> Result<(
  Vec<KernelBaseFactRule>,
  ConceptSourceFactTemplates,
  NoteTemplates,
  QueryProvenanceTemplates,
  SemanticIdTemplates,
  Vec<String>,
  String,
  OutputFragmentTemplates,
  ResponseDocumentSchema,
)> {
  const SUPPORTED_PLACEHOLDERS: &[&str] = &[
    "${route}",
    "${route-segment}",
    "${utterance}",
    "${scope}",
    "${term}",
    "${predicate}",
    "${domain}",
    "${definition-ko}",
    "${formal-name-en}",
  ];
  const CONCEPT_SCALAR_PLACEHOLDERS: &[&str] = &["${term}", "${predicate}"];
  const CONCEPT_LIST_PLACEHOLDERS: &[&str] = &["${term}", "${predicate}", "${index}"];
  const CONCEPT_PROVENANCE_PLACEHOLDERS: &[&str] = &["${source-ref}"];
  const QUERY_PROVENANCE_UTTERANCE_PLACEHOLDERS: &[&str] = &["${utterance}"];
  const QUERY_PROVENANCE_CONCEPT_SOURCE_PLACEHOLDERS: &[&str] = &["${source-ref}"];
  const SEMANTIC_EPISODE_ID_PLACEHOLDERS: &[&str] = &["${counter}"];
  const SEMANTIC_RECORD_ID_PLACEHOLDERS: &[&str] = &["${episode-id}", "${index}"];
  const SEMANTIC_KNOWLEDGE_ID_PLACEHOLDERS: &[&str] = &["${episode-id}"];
  const SEMANTIC_KNOWLEDGE_SUMMARY_PLACEHOLDERS: &[&str] = &[];
  let value = parse_px_file(path)?;
  let root = value.as_attrset().ok_or_else(|| {
    anyhow!(
      "kernel-base-facts root must be attrset in {}",
      path.display()
    )
  })?;
  let raw_rules = root
    .get("base-query-facts")
    .ok_or_else(|| err_missing(path, "base-query-facts"))?;
  let PxValue::List(items) = raw_rules else {
    return Err(err_wrong_type(path, "base-query-facts", "list"));
  };
  let mut rules = Vec::new();
  let mut seen_id_templates = BTreeSet::new();
  for item in items {
    let Some(map) = item.as_attrset() else {
      return Err(err_invalid_entry(path, "base-query-facts"));
    };
    let id_template = required_attrset_string(map, "id-template", "base-query-facts entry", path)?;
    let when_route = optional_attrset_string(map, "when-route", "base-query-facts entry", path)?;
    let context = required_attrset_string(map, "context", "base-query-facts entry", path)?;
    let subj = required_attrset_string(map, "subj", "base-query-facts entry", path)?;
    let pred = required_attrset_string(map, "pred", "base-query-facts entry", path)?;
    let obj_template =
      optional_attrset_string(map, "obj-template", "base-query-facts entry", path)?
        .filter(|s| !s.is_empty());
    let obj_literal = optional_attrset_string(map, "obj-literal", "base-query-facts entry", path)?
      .filter(|s| !s.is_empty());
    match (obj_template.as_deref(), obj_literal.as_deref()) {
      (None, None) => {
        return Err(anyhow!(
          "base-query-facts entry '{}' must carry 'obj-template' or 'obj-literal' in {}",
          id_template,
          path.display()
        ))
      }
      (Some(_), Some(_)) => {
        return Err(anyhow!(
          "base-query-facts entry '{}' must not carry both 'obj-template' and 'obj-literal' in {}",
          id_template,
          path.display()
        ))
      }
      _ => {}
    }
    // id-template placeholder allowlist
    validate_placeholder_allowlist(
      &id_template,
      SUPPORTED_PLACEHOLDERS,
      "base-query-facts id-template",
      path,
    )?;
    if let Some(template) = obj_template.as_deref() {
      validate_placeholder_allowlist(
        template,
        SUPPORTED_PLACEHOLDERS,
        "base-query-facts obj-template",
        path,
      )?;
    }
    if !seen_id_templates.insert(id_template.clone()) {
      return Err(anyhow!(
        "duplicate base-query-facts id-template '{}' in {}",
        id_template,
        path.display()
      ));
    }
    rules.push(KernelBaseFactRule {
      id_template,
      when_route,
      repeat_over: None,
      context,
      subj,
      pred: Some(pred),
      pred_template: None,
      obj_template,
      obj_literal,
    });
  }
  if rules.is_empty() {
    return Err(anyhow!(
      "'base-query-facts' must have at least one entry in {}",
      path.display()
    ));
  }

  // concept-source-facts sub-section
  let concept_source_raw = root
    .get("concept-source-facts")
    .ok_or_else(|| err_missing(path, "concept-source-facts"))?;
  let concept_source_map = concept_source_raw
    .as_attrset()
    .ok_or_else(|| err_wrong_type(path, "concept-source-facts", "attrset"))?;
  let scalar_id_template = required_attrset_string(
    concept_source_map,
    "scalar-id-template",
    "concept-source-facts",
    path,
  )?;
  let list_id_template = required_attrset_string(
    concept_source_map,
    "list-id-template",
    "concept-source-facts",
    path,
  )?;
  let provenance_template = required_attrset_string(
    concept_source_map,
    "provenance-template",
    "concept-source-facts",
    path,
  )?;
  validate_placeholder_allowlist(
    &scalar_id_template,
    CONCEPT_SCALAR_PLACEHOLDERS,
    "concept-source-facts scalar-id-template",
    path,
  )?;
  validate_placeholder_allowlist(
    &list_id_template,
    CONCEPT_LIST_PLACEHOLDERS,
    "concept-source-facts list-id-template",
    path,
  )?;
  validate_placeholder_allowlist(
    &provenance_template,
    CONCEPT_PROVENANCE_PLACEHOLDERS,
    "concept-source-facts provenance-template",
    path,
  )?;

  // transcript-note-prefix top-level entry. note-templates prefix 강제가
  // 이 값을 쓰므로 note-templates 전에 읽는다.
  let transcript_note_prefix = match root.get("transcript-note-prefix") {
    Some(PxValue::String(s)) if !s.is_empty() => s.clone(),
    Some(PxValue::String(_)) => {
      return Err(err_wrong_type(
        path,
        "transcript-note-prefix",
        "non-empty string",
      ))
    }
    Some(_) => return Err(err_wrong_type(path, "transcript-note-prefix", "string")),
    None => return Err(err_missing(path, "transcript-note-prefix")),
  };

  // note-templates sub-section
  let note_raw = root
    .get("note-templates")
    .ok_or_else(|| err_missing(path, "note-templates"))?;
  let note_map = note_raw
    .as_attrset()
    .ok_or_else(|| err_wrong_type(path, "note-templates", "attrset"))?;

  // Loader 는 note-templates 각 entry 에 (1) placeholder allowlist, (2) output
  // prefix 강제를 건다. transcript 계열은 전역 transcript_note_prefix 를 따르고
  // held-* / truth-regime 은 각각 "held-" / "truth-regime:" 고정 prefix 다.
  let note_entries: &[(&str, &[&str], Option<&str>)] = &[
    (
      "transcript-user",
      &["${utterance}"],
      Some(transcript_note_prefix.as_str()),
    ),
    (
      "transcript-pnix",
      &["${response}"],
      Some(transcript_note_prefix.as_str()),
    ),
    ("held-reopen-reason", &["${reason}"], Some("held-")),
    ("held-reopen-term", &["${term}"], Some("held-")),
    ("held-reason", &["${reason}"], Some("held-")),
    ("held-term", &["${term}"], Some("held-")),
    ("invert-trigger", &["${trigger-type}"], None),
    ("truth-regime", &["${regime}"], Some("truth-regime:")),
    ("predicate-query", &["${predicate}"], None),
  ];
  let mut note_values: BTreeMap<&'static str, String> = BTreeMap::new();
  for (key, allowlist, required_prefix) in note_entries {
    let value = required_attrset_string(note_map, key, "note-templates", path)?;
    validate_placeholder_allowlist(
      &value,
      allowlist,
      &format!("note-templates '{}'", key),
      path,
    )?;
    if let Some(prefix) = required_prefix {
      if !value.starts_with(prefix) {
        return Err(anyhow!(
          "note-templates '{}' in {} must start with '{}' (got '{}') to preserve consumer contract",
          key,
          path.display(),
          prefix,
          value
        ));
      }
    }
    note_values.insert(*key, value);
  }
  let note_templates = NoteTemplates {
    transcript_user: note_values.remove("transcript-user").unwrap(),
    transcript_pnix: note_values.remove("transcript-pnix").unwrap(),
    held_reopen_reason: note_values.remove("held-reopen-reason").unwrap(),
    held_reopen_term: note_values.remove("held-reopen-term").unwrap(),
    held_reason: note_values.remove("held-reason").unwrap(),
    held_term: note_values.remove("held-term").unwrap(),
    invert_trigger: note_values.remove("invert-trigger").unwrap(),
    truth_regime: note_values.remove("truth-regime").unwrap(),
    predicate_query: note_values.remove("predicate-query").unwrap(),
  };

  // query-provenance-templates sub-section
  let provenance_raw = root
    .get("query-provenance-templates")
    .ok_or_else(|| err_missing(path, "query-provenance-templates"))?;
  let provenance_map = provenance_raw
    .as_attrset()
    .ok_or_else(|| err_wrong_type(path, "query-provenance-templates", "attrset"))?;
  let utterance_prov = required_attrset_string(
    provenance_map,
    "utterance",
    "query-provenance-templates",
    path,
  )?;
  let concept_source_prov = required_attrset_string(
    provenance_map,
    "concept-source",
    "query-provenance-templates",
    path,
  )?;
  validate_placeholder_allowlist(
    &utterance_prov,
    QUERY_PROVENANCE_UTTERANCE_PLACEHOLDERS,
    "query-provenance-templates 'utterance'",
    path,
  )?;
  validate_placeholder_allowlist(
    &concept_source_prov,
    QUERY_PROVENANCE_CONCEPT_SOURCE_PLACEHOLDERS,
    "query-provenance-templates 'concept-source'",
    path,
  )?;
  let query_provenance_templates = QueryProvenanceTemplates {
    utterance: utterance_prov,
    concept_source: concept_source_prov,
  };

  // semantic-id-templates sub-section
  let semantic_raw = root
    .get("semantic-id-templates")
    .ok_or_else(|| err_missing(path, "semantic-id-templates"))?;
  let semantic_map = semantic_raw
    .as_attrset()
    .ok_or_else(|| err_wrong_type(path, "semantic-id-templates", "attrset"))?;
  let episode_id_template = required_attrset_string(
    semantic_map,
    "episode-id-template",
    "semantic-id-templates",
    path,
  )?;
  let record_id_template = required_attrset_string(
    semantic_map,
    "record-id-template",
    "semantic-id-templates",
    path,
  )?;
  let knowledge_id_template = required_attrset_string(
    semantic_map,
    "knowledge-id-template",
    "semantic-id-templates",
    path,
  )?;
  let knowledge_summary = required_attrset_string(
    semantic_map,
    "knowledge-summary",
    "semantic-id-templates",
    path,
  )?;
  validate_placeholder_allowlist(
    &episode_id_template,
    SEMANTIC_EPISODE_ID_PLACEHOLDERS,
    "semantic-id-templates 'episode-id-template'",
    path,
  )?;
  validate_placeholder_allowlist(
    &record_id_template,
    SEMANTIC_RECORD_ID_PLACEHOLDERS,
    "semantic-id-templates 'record-id-template'",
    path,
  )?;
  validate_placeholder_allowlist(
    &knowledge_id_template,
    SEMANTIC_KNOWLEDGE_ID_PLACEHOLDERS,
    "semantic-id-templates 'knowledge-id-template'",
    path,
  )?;
  validate_placeholder_allowlist(
    &knowledge_summary,
    SEMANTIC_KNOWLEDGE_SUMMARY_PLACEHOLDERS,
    "semantic-id-templates 'knowledge-summary'",
    path,
  )?;
  let semantic_id_templates = SemanticIdTemplates {
    episode_id_template,
    record_id_template,
    knowledge_id_template,
    knowledge_summary,
  };

  // pipeline-trace-note-prefixes
  let pipeline_raw = root
    .get("pipeline-trace-note-prefixes")
    .ok_or_else(|| err_missing(path, "pipeline-trace-note-prefixes"))?;
  let PxValue::List(pipeline_items) = pipeline_raw else {
    return Err(err_wrong_type(path, "pipeline-trace-note-prefixes", "list"));
  };
  let mut pipeline_trace_note_prefixes: Vec<String> = Vec::new();
  for item in pipeline_items {
    match item {
      PxValue::String(s) => {
        if s.is_empty() {
          return Err(anyhow!(
            "'pipeline-trace-note-prefixes' entries must be non-empty strings in {}",
            path.display()
          ));
        }
        pipeline_trace_note_prefixes.push(s.clone());
      }
      _ => {
        return Err(anyhow!(
          "'pipeline-trace-note-prefixes' entries must be strings in {}",
          path.display()
        ))
      }
    }
  }
  if pipeline_trace_note_prefixes.is_empty() {
    return Err(anyhow!(
      "'pipeline-trace-note-prefixes' must have at least one entry in {}",
      path.display()
    ));
  }

  // output-fragment-templates 서브 섹션
  let output_fragment_raw = root
    .get("output-fragment-templates")
    .ok_or_else(|| err_missing(path, "output-fragment-templates"))?;
  let output_fragment_map = output_fragment_raw
    .as_attrset()
    .ok_or_else(|| err_wrong_type(path, "output-fragment-templates", "attrset"))?;
  let read_fragment_entry =
    |key: &str, map: &BTreeMap<String, PxValue>| -> Result<OutputFragmentTemplate> {
      let entry = map
        .get(key)
        .ok_or_else(|| err_missing(path, &format!("output-fragment-templates.{}", key)))?;
      let entry_map = entry.as_attrset().ok_or_else(|| {
        anyhow!(
          "'output-fragment-templates.{}' must be attrset in {}",
          key,
          path.display()
        )
      })?;
      let kind = required_attrset_string(
        entry_map,
        "kind",
        &format!("output-fragment-templates.{}", key),
        path,
      )?;
      let visibility = required_attrset_string(
        entry_map,
        "visibility",
        &format!("output-fragment-templates.{}", key),
        path,
      )?;
      Ok(OutputFragmentTemplate { kind, visibility })
    };
  let output_fragment_templates = OutputFragmentTemplates {
    pipeline_trace: read_fragment_entry("pipeline-trace", output_fragment_map)?,
    response_document: read_fragment_entry("response-document", output_fragment_map)?,
  };

  // response-document-schema 서브 섹션
  const ORG_FACTS_COUNT_PLACEHOLDERS: &[&str] = &["${count}"];
  let response_schema_raw = root
    .get("response-document-schema")
    .ok_or_else(|| err_missing(path, "response-document-schema"))?;
  let response_schema_map = response_schema_raw
    .as_attrset()
    .ok_or_else(|| err_wrong_type(path, "response-document-schema", "attrset"))?;
  let px_header_comment = required_attrset_string(
    response_schema_map,
    "px-header-comment",
    "response-document-schema",
    path,
  )?;
  let px_field_episode_id = required_attrset_string(
    response_schema_map,
    "px-field-episode-id",
    "response-document-schema",
    path,
  )?;
  let px_field_summary = required_attrset_string(
    response_schema_map,
    "px-field-summary",
    "response-document-schema",
    path,
  )?;
  let px_field_transcript = required_attrset_string(
    response_schema_map,
    "px-field-transcript",
    "response-document-schema",
    path,
  )?;
  let px_field_pipeline = required_attrset_string(
    response_schema_map,
    "px-field-pipeline",
    "response-document-schema",
    path,
  )?;
  let px_field_facts_count = required_attrset_string(
    response_schema_map,
    "px-field-facts-count",
    "response-document-schema",
    path,
  )?;
  let org_title = required_attrset_string(
    response_schema_map,
    "org-title",
    "response-document-schema",
    path,
  )?;
  let org_pipeline_section_header = required_attrset_string(
    response_schema_map,
    "org-pipeline-section-header",
    "response-document-schema",
    path,
  )?;
  let org_facts_count_template = required_attrset_string(
    response_schema_map,
    "org-facts-count-template",
    "response-document-schema",
    path,
  )?;
  validate_placeholder_allowlist(
    &org_facts_count_template,
    ORG_FACTS_COUNT_PLACEHOLDERS,
    "response-document-schema 'org-facts-count-template'",
    path,
  )?;
  // org-transcript-transforms: 리스트 필수, 각 entry 는 attrset 이고
  // input-prefix 는 non-empty 문자열, output-prefix 는 string (빈 허용).
  let org_transforms_raw = response_schema_map
    .get("org-transcript-transforms")
    .ok_or_else(|| {
      anyhow!(
        "missing 'response-document-schema.org-transcript-transforms' in {}",
        path.display()
      )
    })?;
  let PxValue::List(org_transforms_items) = org_transforms_raw else {
    return Err(err_wrong_type(
      path,
      "response-document-schema.org-transcript-transforms",
      "list",
    ));
  };
  let mut org_transcript_transforms: Vec<OrgTranscriptTransform> = Vec::new();
  for item in org_transforms_items {
    let Some(entry_map) = item.as_attrset() else {
      return Err(anyhow!(
        "invalid 'response-document-schema.org-transcript-transforms' entry (must be attrset) in {}",
        path.display()
      ));
    };
    let input_prefix = required_attrset_string(
      entry_map,
      "input-prefix",
      "response-document-schema.org-transcript-transforms entry",
      path,
    )?;
    let output_prefix = match entry_map.get("output-prefix") {
      Some(PxValue::String(s)) => s.clone(),
      Some(_) => {
        return Err(anyhow!(
          "'response-document-schema.org-transcript-transforms entry.output-prefix' must be string in {}",
          path.display()
        ))
      }
      None => {
        return Err(anyhow!(
          "missing 'response-document-schema.org-transcript-transforms entry.output-prefix' in {}",
          path.display()
        ))
      }
    };
    org_transcript_transforms.push(OrgTranscriptTransform {
      input_prefix,
      output_prefix,
    });
  }
  if org_transcript_transforms.is_empty() {
    return Err(anyhow!(
      "'response-document-schema.org-transcript-transforms' must have at least one entry in {}",
      path.display()
    ));
  }
  let response_document_schema = ResponseDocumentSchema {
    px_header_comment,
    px_field_episode_id,
    px_field_summary,
    px_field_transcript,
    px_field_pipeline,
    px_field_facts_count,
    org_title,
    org_pipeline_section_header,
    org_facts_count_template,
    org_transcript_transforms,
  };

  Ok((
    rules,
    ConceptSourceFactTemplates {
      scalar_id_template,
      list_id_template,
      provenance_template,
    },
    note_templates,
    query_provenance_templates,
    semantic_id_templates,
    pipeline_trace_note_prefixes,
    transcript_note_prefix,
    output_fragment_templates,
    response_document_schema,
  ))
}

fn validate_placeholder_allowlist(
  template: &str,
  supported: &[&str],
  context: &str,
  path: &Path,
) -> Result<()> {
  let mut cursor = template;
  while let Some(start) = cursor.find("${") {
    let rest = &cursor[start..];
    let Some(end_rel) = rest.find('}') else {
      return Err(anyhow!(
        "{} has unterminated '${{' in {}",
        context,
        path.display()
      ));
    };
    let needle = &rest[..end_rel + 1];
    if !supported.contains(&needle) {
      let hint = closest_placeholder_hint(needle, supported);
      return Err(anyhow!(
        "{} uses unsupported placeholder '{}' in {} (supported: {:?}){}",
        context,
        needle,
        path.display(),
        supported,
        hint,
      ));
    }
    cursor = &rest[end_rel + 1..];
  }
  Ok(())
}

/// placeholder typo 에 대한 `.px` author 힌트. Levenshtein 거리 기반으로 가장
/// 가까운 allowed placeholder 를 찾고, 거리가 충분히 가까우면 ` (did you mean
/// '${foo}'?)` 문구를 돌려준다. 거리가 너무 멀거나 allowed 가 비면 빈 문자열.
fn closest_placeholder_hint(needle: &str, supported: &[&str]) -> String {
  if supported.is_empty() {
    return String::new();
  }
  let Some((best, distance)) = supported
    .iter()
    .map(|candidate| (*candidate, levenshtein(needle, candidate)))
    .min_by_key(|(_, d)| *d)
  else {
    return String::new();
  };
  // needle 길이의 절반 또는 3 중 큰 값 이하 거리만 힌트로 보여준다.
  // ${alt-count} 와 ${term} 처럼 완전히 다른 이름은 힌트 안 띄움.
  let tolerance = std::cmp::max(3, needle.chars().count() / 2);
  if distance == 0 || distance > tolerance {
    String::new()
  } else {
    format!(" (did you mean '{}'?)", best)
  }
}

/// 간단한 Levenshtein 거리 구현. Rust std 에는 없어서 직접 계산.
/// O(|a| * |b|) 공간, 짧은 placeholder 이름 비교라 충분히 빠르다.
fn levenshtein(a: &str, b: &str) -> usize {
  let a: Vec<char> = a.chars().collect();
  let b: Vec<char> = b.chars().collect();
  let n = a.len();
  let m = b.len();
  if n == 0 {
    return m;
  }
  if m == 0 {
    return n;
  }
  let mut prev: Vec<usize> = (0..=m).collect();
  let mut curr: Vec<usize> = vec![0; m + 1];
  for i in 1..=n {
    curr[0] = i;
    for j in 1..=m {
      let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
      let del = prev[j] + 1;
      let ins = curr[j - 1] + 1;
      let sub = prev[j - 1] + cost;
      curr[j] = std::cmp::min(std::cmp::min(del, ins), sub);
    }
    std::mem::swap(&mut prev, &mut curr);
  }
  prev[m]
}

fn load_query_route_defaults(path: &Path) -> Result<QueryRouteDefaults> {
  let value = parse_px_file(path)?;
  if value.as_attrset().is_none() {
    return Err(anyhow!(
      "query-route-defaults root must be attrset in {}",
      path.display()
    ));
  }
  let mut defaults = QueryRouteDefaults::default();
  let raw_rules = value
    .get("query-context-rewrite-rules")
    .ok_or_else(|| err_missing(path, "query-context-rewrite-rules"))?;
  let PxValue::List(items) = raw_rules else {
    return Err(err_wrong_type(path, "query-context-rewrite-rules", "list"));
  };
  for item in items {
    let Some(map) = item.as_attrset() else {
      return Err(err_invalid_entry(path, "query-context-rewrite-rules"));
    };
    let from = required_attrset_string(map, "from", "query-context-rewrite-rules entry", path)?;
    let to = required_attrset_string(map, "to", "query-context-rewrite-rules entry", path)?;
    if defaults
      .query_context_rewrite_rules
      .iter()
      .any(|rule| rule.from == from)
    {
      return Err(err_duplicate_entry(
        path,
        "query-context-rewrite-rules",
        "from",
        &from,
      ));
    }
    defaults
      .query_context_rewrite_rules
      .push(PrefixRewriteRule { from, to });
  }
  if defaults.query_context_rewrite_rules.is_empty() {
    return Err(anyhow!(
      "missing 'query-context-rewrite-rules' in {}",
      path.display()
    ));
  }
  Ok(defaults)
}

fn load_query_routes(
  path: &Path,
  defaults: &QueryRouteDefaults,
) -> Result<(
  BTreeMap<String, QueryRouteSpec>,
  BTreeMap<String, KernelRouteRuntimeRule>,
)> {
  let value = parse_px_file(path)?;
  let mut result = BTreeMap::new();
  let mut runtime_rules = BTreeMap::new();
  let items = match value {
    PxValue::List(ref items) => items,
    _ => {
      return Err(anyhow!(
        "query-routes root must be list in {}",
        path.display()
      ))
    }
  };
  for item in items {
    let Some(map) = item.as_attrset() else {
      return Err(err_invalid_entry(path, "query-routes"));
    };
    let route = required_attrset_string(map, "route", "query-routes entry", path)?;
    let raw_query_context =
      required_attrset_string(map, "query-context", "query-routes entry", path)?;
    let raw_include_hop_knowledge =
      required_attrset_string(map, "include-hop-knowledge", "query-routes entry", path)?;
    let include_hop_knowledge = match raw_include_hop_knowledge.as_str() {
      "true" => true,
      "false" => false,
      _ => {
        return Err(anyhow!(
          "'include-hop-knowledge' for route '{}' must be 'true' or 'false' in {}",
          route,
          path.display()
        ))
      }
    };
    let default_preview = parse_required_route_usize(map, "default-preview", path, route.as_str())?;
    let spec = QueryRouteSpec {
      query_context: query_context_for_route(defaults, &raw_query_context, route.as_str(), path)?,
      include_hop_knowledge,
      default_preview,
      policy_coverage: parse_required_route_f64(map, "policy-coverage", path, route.as_str())?,
      policy_coherence: parse_required_route_f64(map, "policy-coherence", path, route.as_str())?,
      policy_loss: parse_required_route_f64(map, "policy-loss", path, route.as_str())?,
      policy_cost: parse_required_route_f64(map, "policy-cost", path, route.as_str())?,
      policy_accept_threshold: parse_required_route_f64(
        map,
        "policy-accept-threshold",
        path,
        route.as_str(),
      )?,
    };
    let route_key = route.clone();
    if result.contains_key(&route_key) {
      return Err(err_duplicate_entry(
        path,
        "query-routes",
        "route",
        &route_key,
      ));
    }
    result.insert(route_key.clone(), spec);
    runtime_rules.insert(
      route_key,
      KernelRouteRuntimeRule {
        direct_fact_predicates: match map.get("kernel-direct-fact-predicates") {
          Some(raw @ PxValue::List(_)) => {
            let values = raw.as_string_list();
            if values.is_empty() {
              return Err(anyhow!(
                "empty 'kernel-direct-fact-predicates' for route '{}' in {}",
                route,
                path.display()
              ));
            }
            Some(values)
          }
          Some(_) => {
            return Err(anyhow!(
              "'kernel-direct-fact-predicates' for route '{}' must be list in {}",
              route,
              path.display()
            ))
          }
          None => None,
        },
        direct_interpretation_id: match map.get("kernel-direct-interpretation-id") {
          Some(PxValue::String(s)) => Some(s.clone()),
          Some(_) => {
            return Err(anyhow!(
              "'kernel-direct-interpretation-id' for route '{}' must be string in {}",
              route,
              path.display()
            ))
          }
          None => None,
        },
        rich_interpretation_id: match map.get("kernel-rich-interpretation-id") {
          Some(PxValue::String(s)) => Some(s.clone()),
          Some(_) => {
            return Err(anyhow!(
              "'kernel-rich-interpretation-id' for route '{}' must be string in {}",
              route,
              path.display()
            ))
          }
          None => None,
        },
        interpretation_id: match map.get("kernel-interpretation-id") {
          Some(PxValue::String(s)) => Some(s.clone()),
          Some(_) => {
            return Err(anyhow!(
              "'kernel-interpretation-id' for route '{}' must be string in {}",
              route,
              path.display()
            ))
          }
          None => None,
        },
      },
    );
  }
  Ok((result, runtime_rules))
}

fn load_followups(path: &Path) -> Result<FollowupConfig> {
  let value = parse_px_file(path)?;
  if value.as_attrset().is_none() {
    return Err(anyhow!(
      "followup-generation root must be attrset in {}",
      path.display()
    ));
  }
  let mut config = FollowupConfig {
    default_choices: required_top_level_string_list(&value, "default-choices", path)?,
    ..FollowupConfig::default()
  };
  let raw_rules = value
    .get("disambiguation-questions")
    .ok_or_else(|| err_missing(path, "disambiguation-questions"))?;
  let PxValue::List(items) = raw_rules else {
    return Err(err_wrong_type(path, "disambiguation-questions", "list"));
  };
  for item in items {
    let Some(map) = item.as_attrset() else {
      return Err(err_invalid_entry(path, "disambiguation-questions"));
    };
    let predicate = required_attrset_string(
      map,
      "distinguishing-predicate",
      "disambiguation-questions entry",
      path,
    )?;
    if map.get("question-template").is_none() {
      return Err(anyhow!(
        "invalid 'disambiguation-questions' entry for '{}': missing 'question-template' in {}",
        predicate,
        path.display()
      ));
    }
    if map.get("choices-template").is_none() {
      return Err(anyhow!(
        "invalid 'disambiguation-questions' entry for '{}': missing 'choices-template' in {}",
        predicate,
        path.display()
      ));
    }
    let question_template = required_attrset_string(
      map,
      "question-template",
      "disambiguation-questions entry",
      path,
    )?;
    let choices_template = present_attrset_string(
      map,
      "choices-template",
      "disambiguation-questions entry",
      path,
    )?;
    if question_template.is_empty() && choices_template.is_empty() {
      return Err(anyhow!(
        "invalid 'disambiguation-questions' entry for '{}': both templates are empty in {}",
        predicate,
        path.display()
      ));
    }
    if config.disambiguation_questions.contains_key(&predicate) {
      return Err(err_duplicate_entry(
        path,
        "disambiguation-questions",
        "distinguishing-predicate",
        &predicate,
      ));
    }
    config.disambiguation_questions.insert(
      predicate,
      DisambiguationQuestion {
        question_template,
        choices_template,
      },
    );
  }
  let raw_rules = value
    .get("reason-question-rules")
    .ok_or_else(|| err_missing(path, "reason-question-rules"))?;
  let PxValue::List(items) = raw_rules else {
    return Err(err_wrong_type(path, "reason-question-rules", "list"));
  };
  for item in items {
    let Some(map) = item.as_attrset() else {
      return Err(err_invalid_entry(path, "reason-question-rules"));
    };
    let reason = required_attrset_string(map, "reason", "reason-question-rules entry", path)?;
    let predicate = required_attrset_string(map, "predicate", "reason-question-rules entry", path)?;
    if config.reason_question_rules.contains_key(&reason) {
      return Err(err_duplicate_entry(
        path,
        "reason-question-rules",
        "reason",
        &reason,
      ));
    }
    config.reason_question_rules.insert(reason, predicate);
  }
  let raw_rules = value
    .get("reopen-rules")
    .ok_or_else(|| err_missing(path, "reopen-rules"))?;
  let PxValue::List(items) = raw_rules else {
    return Err(err_wrong_type(path, "reopen-rules", "list"));
  };
  for item in items {
    let Some(map) = item.as_attrset() else {
      return Err(err_invalid_entry(path, "reopen-rules"));
    };
    let reason = required_attrset_string(map, "reason", "reopen-rules entry", path)?;
    let carry_term_policy =
      required_attrset_string(map, "carry-term-policy", "reopen-rules entry", path)?;
    let effective_utterance_template = required_attrset_string(
      map,
      "effective-utterance-template",
      "reopen-rules entry",
      path,
    )?;
    require_allowed_literal(
      carry_term_policy.as_str(),
      "carry-term-policy",
      "reopen-rules entry",
      path,
      &["always", "never", "when-missing"],
    )?;
    if config.reopen_rules.contains_key(&reason) {
      return Err(err_duplicate_entry(path, "reopen-rules", "reason", &reason));
    }
    config.reopen_rules.insert(
      reason,
      ReopenRule {
        carry_term_policy,
        effective_utterance_template,
      },
    );
  }
  let raw_rules = value
    .get("choice-rules")
    .ok_or_else(|| err_missing(path, "choice-rules"))?;
  let PxValue::List(items) = raw_rules else {
    return Err(err_wrong_type(path, "choice-rules", "list"));
  };
  for item in items {
    let Some(map) = item.as_attrset() else {
      return Err(err_invalid_entry(path, "choice-rules"));
    };
    let when = required_attrset_string(map, "when", "choice-rules entry", path)?;
    let choice_source = required_attrset_string(map, "choice-source", "choice-rules entry", path)?;
    require_allowed_literal(
      when.as_str(),
      "when",
      "choice-rules entry",
      path,
      &[
        "term-present",
        "term-missing",
        "always",
        "term-present-with-concept-choice",
        "term-present-without-concept-choice",
      ],
    )?;
    require_allowed_literal(
      choice_source.as_str(),
      "choice-source",
      "choice-rules entry",
      path,
      &["concept", "default", "none"],
    )?;
    config.choice_rules.push(ChoiceRule {
      when,
      choice_source,
    });
  }
  let raw_rules = value
    .get("resolved-term-rules")
    .ok_or_else(|| err_missing(path, "resolved-term-rules"))?;
  let PxValue::List(items) = raw_rules else {
    return Err(err_wrong_type(path, "resolved-term-rules", "list"));
  };
  for item in items {
    let Some(map) = item.as_attrset() else {
      return Err(err_invalid_entry(path, "resolved-term-rules"));
    };
    let when = required_attrset_string(map, "when", "resolved-term-rules entry", path)?;
    let term_source =
      required_attrset_string(map, "term-source", "resolved-term-rules entry", path)?;
    require_allowed_literal(
      when.as_str(),
      "when",
      "resolved-term-rules entry",
      path,
      &["term-present", "term-missing", "always"],
    )?;
    require_allowed_literal(
      term_source.as_str(),
      "term-source",
      "resolved-term-rules entry",
      path,
      &["term", "literal", "label", "none"],
    )?;
    let value = optional_attrset_string(map, "value", "resolved-term-rules entry", path)?
      .filter(|value| !value.is_empty());
    if term_source == "literal" && value.is_none() {
      return Err(anyhow!(
        "missing 'value' for literal resolved-term-rules entry in {}",
        path.display()
      ));
    }
    config.resolved_term_rules.push(ResolvedTermRule {
      when,
      term_source,
      value,
    });
  }
  let raw_rules = value
    .get("held-response-rules")
    .ok_or_else(|| err_missing(path, "held-response-rules"))?;
  let PxValue::List(items) = raw_rules else {
    return Err(err_wrong_type(path, "held-response-rules", "list"));
  };
  for item in items {
    let Some(map) = item.as_attrset() else {
      return Err(err_invalid_entry(path, "held-response-rules"));
    };
    let when = required_attrset_string(map, "when", "held-response-rules entry", path)?;
    let template = required_attrset_string(map, "template", "held-response-rules entry", path)?;
    require_allowed_literal(
      when.as_str(),
      "when",
      "held-response-rules entry",
      path,
      &["term-present", "term-missing", "always"],
    )?;
    let emit_held_term =
      match required_attrset_string(map, "emit-held-term", "held-response-rules entry", path)?
        .as_str()
      {
        "true" => true,
        "false" => false,
        _ => {
          return Err(anyhow!(
            "'emit-held-term' in held-response-rules entry must be 'true' or 'false' in {}",
            path.display()
          ))
        }
      };
    config.held_response_rules.push(HeldResponseRule {
      when,
      template,
      emit_held_term,
    });
  }
  let raw_rules = value
    .get("concept-choices")
    .ok_or_else(|| err_missing(path, "concept-choices"))?;
  let PxValue::List(items) = raw_rules else {
    return Err(err_wrong_type(path, "concept-choices", "list"));
  };
  for item in items {
    let Some(map) = item.as_attrset() else {
      return Err(err_invalid_entry(path, "concept-choices"));
    };
    let term = required_attrset_string(map, "term", "concept-choices entry", path)?;
    let choices = match map.get("choices") {
      Some(PxValue::List(list)) => PxValue::List(list.clone()).as_string_list(),
      Some(_) => {
        return Err(anyhow!(
          "'choices' in concept-choices entry must be list in {}",
          path.display()
        ))
      }
      None => {
        return Err(anyhow!(
          "missing 'choices' in concept-choices entry in {}",
          path.display()
        ))
      }
    };
    if term.is_empty() || choices.is_empty() {
      return Err(err_invalid_entry(path, "concept-choices"));
    }
    if config.concept_choices.contains_key(&term) {
      return Err(err_duplicate_entry(path, "concept-choices", "term", &term));
    }
    config.concept_choices.insert(term, choices);
  }
  config.unknown_term_label = required_top_level_string(&value, "unknown-term-label", path)?;
  if config.disambiguation_questions.is_empty() {
    return Err(err_missing(path, "disambiguation-questions"));
  }
  if config.reason_question_rules.is_empty() {
    return Err(err_missing(path, "reason-question-rules"));
  }
  if config.choice_rules.is_empty() {
    return Err(err_missing(path, "choice-rules"));
  }
  if config.resolved_term_rules.is_empty() {
    return Err(err_missing(path, "resolved-term-rules"));
  }
  if config.held_response_rules.is_empty() {
    return Err(err_missing(path, "held-response-rules"));
  }
  if config.reopen_rules.is_empty() {
    return Err(err_missing(path, "reopen-rules"));
  }
  Ok(config)
}

fn load_dialogue_templates(path: &Path) -> Result<DialogueTemplates> {
  let value = parse_px_file(path)?;
  if value.as_attrset().is_none() {
    return Err(anyhow!(
      "dialogue-templates root must be attrset in {}",
      path.display()
    ));
  }
  let mut templates = DialogueTemplates::default();
  templates.definition_section = load_template_section(
    value.get("kernel-definition-section"),
    "kernel-definition-section",
    path,
  )?;
  templates.why_section =
    load_template_section(value.get("kernel-why-section"), "kernel-why-section", path)?;
  templates.property_section = load_template_section(
    value.get("kernel-property-section"),
    "kernel-property-section",
    path,
  )?;
  let route = match value.get("kernel-route-summary") {
    Some(raw_route) => raw_route
      .as_attrset()
      .ok_or_else(|| err_wrong_type(path, "kernel-route-summary", "attrset"))?,
    None => return Err(err_missing(path, "kernel-route-summary")),
  };
  templates.route_summary_definition =
    required_attrset_string(route, "definition", "kernel-route-summary", path)?;
  templates.route_summary_property =
    required_attrset_string(route, "property", "kernel-route-summary", path)?;
  templates.route_summary_why =
    required_attrset_string(route, "why", "kernel-route-summary", path)?;
  templates.route_summary_held =
    required_attrset_string(route, "held", "kernel-route-summary", path)?;

  // batch 74 (2026-04-15): per-predicate empty response override map.
  // 없으면 empty (loader 강제 X — generic kernel-property-section empty
  // template 이 fallback). 존재하면 attrset 이어야 하고, 각 entry 는
  // non-empty string 이면서 `${term}` placeholder 만 허용.
  if let Some(raw_map) = value.get("kernel-property-empty-by-predicate") {
    let map = raw_map
      .as_attrset()
      .ok_or_else(|| err_wrong_type(path, "kernel-property-empty-by-predicate", "attrset"))?;
    for (predicate_key, raw_template) in map {
      let PxValue::String(template) = raw_template else {
        return Err(anyhow!(
          "kernel-property-empty-by-predicate entry '{}' must be a string in {}",
          predicate_key,
          path.display()
        ));
      };
      if template.is_empty() {
        return Err(anyhow!(
          "kernel-property-empty-by-predicate entry '{}' must be non-empty in {}",
          predicate_key,
          path.display()
        ));
      }
      validate_placeholder_allowlist(
        &template,
        &["${term}"],
        "kernel-property-empty-by-predicate entry",
        path,
      )?;
      templates
        .property_empty_by_predicate
        .insert(predicate_key.clone(), template.clone());
    }
  }

  Ok(templates)
}

fn load_template_section(
  value: Option<&PxValue>,
  section_name: &str,
  path: &Path,
) -> Result<TemplateSection> {
  let mut section = TemplateSection::default();
  let Some(raw_value) = value else {
    return Err(err_missing(path, section_name));
  };
  let Some(map) = raw_value.as_attrset() else {
    return Err(anyhow!(
      "'{}' must be attrset in {}",
      section_name,
      path.display()
    ));
  };
  section.join_with = present_attrset_string(map, "join-with", section_name, path)?;
  section.suffix = present_attrset_string(map, "suffix", section_name, path)?;
  let Some(PxValue::List(items)) = map.get("parts") else {
    return Err(anyhow!(
      "missing '{}.parts' in {}",
      section_name,
      path.display()
    ));
  };
  for item in items {
    let Some(part) = item.as_attrset() else {
      return Err(anyhow!(
        "invalid part in {} of {}",
        section_name,
        path.display()
      ));
    };
    let when = required_attrset_string(part, "when", section_name, path)?;
    let template = required_attrset_string(part, "template", section_name, path)?;
    if template.is_empty() {
      return Err(anyhow!(
        "missing 'template' in {} part of {}",
        section_name,
        path.display()
      ));
    }
    if when.is_empty() {
      return Err(anyhow!(
        "missing 'when' in {} part of {}",
        section_name,
        path.display()
      ));
    }
    if when != "always" {
      return Err(anyhow!(
        "unsupported 'when' value '{}' in {} part of {}",
        when,
        section_name,
        path.display()
      ));
    }
    section.parts.push(ConditionalTemplatePart {
      when,
      template,
      field_non_empty: optional_attrset_string(part, "field-non-empty", section_name, path)?,
      list_non_empty: optional_attrset_string(part, "list-non-empty", section_name, path)?,
      scope_is: {
        let value = optional_attrset_string(part, "scope-is", section_name, path)?;
        if let Some(scope_name) = value.as_deref() {
          require_allowed_literal(
            scope_name,
            "scope-is",
            &format!("{} part", section_name),
            path,
            &["brief", "standard", "detailed"],
          )?;
        }
        value
      },
      values_state: {
        let value = optional_attrset_string(part, "values-state", section_name, path)?;
        if let Some(state) = value.as_deref() {
          require_allowed_literal(
            state,
            "values-state",
            &format!("{} part", section_name),
            path,
            &["empty", "present"],
          )?;
        }
        value
      },
    });
  }
  if section.parts.is_empty() {
    return Err(anyhow!(
      "missing '{}.parts' in {}",
      section_name,
      path.display()
    ));
  }
  Ok(section)
}

fn load_invert_config(path: &Path) -> Result<InvertConfig> {
  let value = parse_px_file(path)?;
  if value.as_attrset().is_none() {
    return Err(anyhow!(
      "ontology-invert root must be attrset in {}",
      path.display()
    ));
  }
  let mut config = InvertConfig::default();
  config.trigger_selection = required_top_level_string(&value, "trigger-selection", path)?;
  require_allowed_literal(
    config.trigger_selection.as_str(),
    "trigger-selection",
    "ontology-invert",
    path,
    &[
      "list-order",
      "priority-then-list-order",
      "priority-then-pattern-length",
    ],
  )?;
  config.route_template = required_top_level_string(&value, "route-template", path)?;
  config.default_truth_regime = required_top_level_string(&value, "default-truth-regime", path)?;
  let raw_rule = value
    .get("default-interpretation-rule")
    .ok_or_else(|| err_missing(path, "default-interpretation-rule"))?;
  {
    let Some(map) = raw_rule.as_attrset() else {
      return Err(err_wrong_type(
        path,
        "default-interpretation-rule",
        "attrset",
      ));
    };
    config.default_direct_fact_predicates = match map.get("direct-fact-predicates") {
      Some(PxValue::List(_)) => map.get("direct-fact-predicates").unwrap().as_string_list(),
      Some(_) => {
        return Err(err_wrong_type(
          path,
          "default-interpretation-rule.direct-fact-predicates",
          "list",
        ))
      }
      None => {
        return Err(err_missing(
          path,
          "default-interpretation-rule.direct-fact-predicates",
        ))
      }
    };
    config.default_source_include_predicates = match map.get("source-include-predicates") {
      Some(PxValue::List(_)) => map
        .get("source-include-predicates")
        .unwrap()
        .as_string_list(),
      Some(_) => {
        return Err(err_wrong_type(
          path,
          "default-interpretation-rule.source-include-predicates",
          "list",
        ))
      }
      None => {
        return Err(err_missing(
          path,
          "default-interpretation-rule.source-include-predicates",
        ))
      }
    };
    config.default_source_include_context_prefixes =
      match map.get("source-include-context-prefixes") {
        Some(PxValue::List(_)) => map
          .get("source-include-context-prefixes")
          .unwrap()
          .as_string_list(),
        Some(_) => {
          return Err(err_wrong_type(
            path,
            "default-interpretation-rule.source-include-context-prefixes",
            "list",
          ))
        }
        None => {
          return Err(err_missing(
            path,
            "default-interpretation-rule.source-include-context-prefixes",
          ))
        }
      };
    config.default_direct_interpretation_id = match map.get("direct-interpretation-id") {
      Some(PxValue::String(s)) => s.clone(),
      Some(_) => {
        return Err(err_wrong_type(
          path,
          "default-interpretation-rule.direct-interpretation-id",
          "string",
        ))
      }
      None => {
        return Err(err_missing(
          path,
          "default-interpretation-rule.direct-interpretation-id",
        ))
      }
    };
    config.default_rich_interpretation_id = match map.get("rich-interpretation-id") {
      Some(PxValue::String(s)) => s.clone(),
      Some(_) => {
        return Err(err_wrong_type(
          path,
          "default-interpretation-rule.rich-interpretation-id",
          "string",
        ))
      }
      None => {
        return Err(err_missing(
          path,
          "default-interpretation-rule.rich-interpretation-id",
        ))
      }
    };
  }
  if config.default_direct_fact_predicates.is_empty() {
    return Err(err_missing(
      path,
      "default-interpretation-rule.direct-fact-predicates",
    ));
  }
  if config.default_source_include_predicates.is_empty() {
    return Err(err_missing(
      path,
      "default-interpretation-rule.source-include-predicates",
    ));
  }
  if config.default_source_include_context_prefixes.is_empty() {
    return Err(err_missing(
      path,
      "default-interpretation-rule.source-include-context-prefixes",
    ));
  }
  if config.default_direct_interpretation_id.is_empty() {
    return Err(err_missing(
      path,
      "default-interpretation-rule.direct-interpretation-id",
    ));
  }
  if config.default_rich_interpretation_id.is_empty() {
    return Err(err_missing(
      path,
      "default-interpretation-rule.rich-interpretation-id",
    ));
  }
  let raw_rules = value
    .get("invert-triggers")
    .ok_or_else(|| err_missing(path, "invert-triggers"))?;
  let PxValue::List(items) = raw_rules else {
    return Err(err_wrong_type(path, "invert-triggers", "list"));
  };
  for item in items {
    let Some(map) = item.as_attrset() else {
      return Err(err_invalid_entry(path, "invert-triggers"));
    };
    let pattern = required_attrset_string(map, "pattern", "invert-triggers entry", path)?;
    let trigger_type = required_attrset_string(map, "type", "invert-triggers entry", path)?;
    let truth_regime = required_attrset_string(map, "truth-regime", "invert-triggers entry", path)?;
    let raw_priority = required_attrset_string(map, "priority", "invert-triggers entry", path)?;
    let priority = raw_priority.parse::<i64>().map_err(|_| {
      anyhow!(
        "invalid 'priority' for invert trigger '{}' in {}",
        pattern,
        path.display()
      )
    })?;
    if config.triggers.iter().any(|t| t.pattern == pattern) {
      return Err(err_duplicate_entry(
        path,
        "invert-triggers",
        "pattern",
        &pattern,
      ));
    }
    config.triggers.push(InvertTrigger {
      pattern,
      trigger_type,
      truth_regime,
      priority,
    });
  }
  let raw_rules = value
    .get("domain-to-regime")
    .ok_or_else(|| err_missing(path, "domain-to-regime"))?;
  let PxValue::List(items) = raw_rules else {
    return Err(err_wrong_type(path, "domain-to-regime", "list"));
  };
  for item in items {
    let Some(map) = item.as_attrset() else {
      return Err(err_invalid_entry(path, "domain-to-regime"));
    };
    let prefix = required_attrset_string(map, "domain-prefix", "domain-to-regime entry", path)?;
    let regime = required_attrset_string(map, "regime", "domain-to-regime entry", path)?;
    if config.domain_to_regime.iter().any(|(p, _)| p == &prefix) {
      return Err(err_duplicate_entry(
        path,
        "domain-to-regime",
        "domain-prefix",
        &prefix,
      ));
    }
    if prefix == "*" {
      if regime != config.default_truth_regime {
        return Err(anyhow!(
          "'domain-to-regime' wildcard regime must match 'default-truth-regime' in {}",
          path.display()
        ));
      }
      config.domain_to_regime.push((prefix, regime));
      continue;
    }
    config.domain_to_regime.push((prefix, regime));
  }
  if !config
    .domain_to_regime
    .iter()
    .any(|(prefix, _)| prefix == "*")
  {
    config
      .domain_to_regime
      .push(("*".to_string(), config.default_truth_regime.clone()));
  }
  let raw_rules = value
    .get("invert-candidate-rules")
    .ok_or_else(|| err_missing(path, "invert-candidate-rules"))?;
  let PxValue::List(items) = raw_rules else {
    return Err(err_wrong_type(path, "invert-candidate-rules", "list"));
  };
  for item in items {
    let Some(map) = item.as_attrset() else {
      return Err(err_invalid_entry(path, "invert-candidate-rules"));
    };
    let trigger_type = required_attrset_string(map, "type", "invert-candidate-rules entry", path)?;
    let concept_field =
      optional_attrset_string(map, "concept-field", "invert-candidate-rules entry", path)?
        .filter(|field| !field.is_empty());
    let predicate =
      required_attrset_string(map, "predicate", "invert-candidate-rules entry", path)?;
    let context = required_attrset_string(map, "context", "invert-candidate-rules entry", path)?;
    let obj_template =
      optional_attrset_string(map, "obj-template", "invert-candidate-rules entry", path)?
        .filter(|template| !template.is_empty());
    if concept_field.is_none() && obj_template.is_none() {
      return Err(anyhow!(
        "missing 'concept-field' or 'obj-template' for invert candidate rule '{}' in {}",
        trigger_type,
        path.display()
      ));
    }
    if let Some(template) = obj_template.as_deref() {
      const SUPPORTED_PLACEHOLDERS: &[&str] = &["${term}", "${provenance}"];
      validate_placeholder_allowlist(
        template,
        SUPPORTED_PLACEHOLDERS,
        &format!("invert candidate rule '{}' obj-template", trigger_type),
        path,
      )?;
    }
    config.candidate_rules.push(InvertCandidateRule {
      trigger_type,
      concept_field,
      predicate,
      context,
      obj_template,
    });
  }
  let raw_rules = value
    .get("interpretation-rules")
    .ok_or_else(|| err_missing(path, "interpretation-rules"))?;
  let PxValue::List(items) = raw_rules else {
    return Err(err_wrong_type(path, "interpretation-rules", "list"));
  };
  for item in items {
    let Some(map) = item.as_attrset() else {
      return Err(err_invalid_entry(path, "interpretation-rules"));
    };
    let trigger_type = required_attrset_string(map, "type", "interpretation-rules entry", path)?;
    if trigger_type.is_empty() {
      return Err(anyhow!(
        "missing 'type' for invert interpretation rule in {}",
        path.display()
      ));
    }
    let direct_fact_predicates = match map.get("direct-fact-predicates") {
      Some(PxValue::List(_)) => map.get("direct-fact-predicates").unwrap().as_string_list(),
      Some(_) => {
        return Err(anyhow!(
          "'direct-fact-predicates' for invert interpretation rule '{}' must be list in {}",
          trigger_type,
          path.display()
        ))
      }
      None => {
        return Err(anyhow!(
          "missing 'direct-fact-predicates' for invert interpretation rule '{}' in {}",
          trigger_type,
          path.display()
        ))
      }
    };
    if direct_fact_predicates.is_empty() {
      return Err(anyhow!(
        "empty 'direct-fact-predicates' for invert interpretation rule '{}' in {}",
        trigger_type,
        path.display()
      ));
    }
    let source_include_predicates = match map.get("source-include-predicates") {
      Some(PxValue::List(_)) => map
        .get("source-include-predicates")
        .unwrap()
        .as_string_list(),
      Some(_) => {
        return Err(anyhow!(
          "'source-include-predicates' for invert interpretation rule '{}' must be list in {}",
          trigger_type,
          path.display()
        ))
      }
      None => {
        return Err(anyhow!(
          "missing 'source-include-predicates' for invert interpretation rule '{}' in {}",
          trigger_type,
          path.display()
        ))
      }
    };
    if source_include_predicates.is_empty() {
      return Err(anyhow!(
        "empty 'source-include-predicates' for invert interpretation rule '{}' in {}",
        trigger_type,
        path.display()
      ));
    }
    let source_include_context_prefixes = match map.get("source-include-context-prefixes") {
      Some(PxValue::List(_)) => map
        .get("source-include-context-prefixes")
        .unwrap()
        .as_string_list(),
      Some(_) => {
        return Err(anyhow!(
        "'source-include-context-prefixes' for invert interpretation rule '{}' must be list in {}",
        trigger_type,
        path.display()
      ))
      }
      None => {
        return Err(anyhow!(
          "missing 'source-include-context-prefixes' for invert interpretation rule '{}' in {}",
          trigger_type,
          path.display()
        ))
      }
    };
    if source_include_context_prefixes.is_empty() {
      return Err(anyhow!(
        "empty 'source-include-context-prefixes' for invert interpretation rule '{}' in {}",
        trigger_type,
        path.display()
      ));
    }
    let direct_interpretation_id = match map.get("direct-interpretation-id") {
      Some(PxValue::String(s)) => s.clone(),
      Some(_) => {
        return Err(anyhow!(
          "'direct-interpretation-id' for invert interpretation rule '{}' must be string in {}",
          trigger_type,
          path.display()
        ))
      }
      None => {
        return Err(anyhow!(
          "missing 'direct-interpretation-id' for invert interpretation rule '{}' in {}",
          trigger_type,
          path.display()
        ))
      }
    };
    let rich_interpretation_id = match map.get("rich-interpretation-id") {
      Some(PxValue::String(s)) => s.clone(),
      Some(_) => {
        return Err(anyhow!(
          "'rich-interpretation-id' for invert interpretation rule '{}' must be string in {}",
          trigger_type,
          path.display()
        ))
      }
      None => {
        return Err(anyhow!(
          "missing 'rich-interpretation-id' for invert interpretation rule '{}' in {}",
          trigger_type,
          path.display()
        ))
      }
    };
    if direct_fact_predicates.is_empty() {
      return Err(anyhow!(
        "missing 'direct-fact-predicates' for invert interpretation rule '{}' in {}",
        trigger_type,
        path.display()
      ));
    }
    if source_include_predicates.is_empty() {
      return Err(anyhow!(
        "missing 'source-include-predicates' for invert interpretation rule '{}' in {}",
        trigger_type,
        path.display()
      ));
    }
    if source_include_context_prefixes.is_empty() {
      return Err(anyhow!(
        "missing 'source-include-context-prefixes' for invert interpretation rule '{}' in {}",
        trigger_type,
        path.display()
      ));
    }
    if direct_interpretation_id.is_empty() {
      return Err(anyhow!(
        "missing 'direct-interpretation-id' for invert interpretation rule '{}' in {}",
        trigger_type,
        path.display()
      ));
    }
    if rich_interpretation_id.is_empty() {
      return Err(anyhow!(
        "missing 'rich-interpretation-id' for invert interpretation rule '{}' in {}",
        trigger_type,
        path.display()
      ));
    }
    config.interpretation_rules.push(InvertInterpretationRule {
      trigger_type,
      direct_fact_predicates,
      source_include_predicates,
      source_include_context_prefixes,
      direct_interpretation_id,
      rich_interpretation_id,
    });
  }
  materialize_invert_interpretation_rules(&mut config, path)?;
  Ok(config)
}

fn materialize_invert_interpretation_rules(config: &mut InvertConfig, path: &Path) -> Result<()> {
  let trigger_types = config
    .triggers
    .iter()
    .map(|trigger| trigger.trigger_type.clone())
    .collect::<BTreeSet<_>>();
  let mut explicit_rules = BTreeMap::new();
  for rule in &config.interpretation_rules {
    if !trigger_types.contains(rule.trigger_type.as_str()) {
      return Err(anyhow!(
        "unknown interpretation rule type '{}' in {}",
        rule.trigger_type,
        path.display()
      ));
    }
    if explicit_rules
      .insert(rule.trigger_type.clone(), rule.clone())
      .is_some()
    {
      return Err(anyhow!(
        "duplicate interpretation rule type '{}' in {}",
        rule.trigger_type,
        path.display()
      ));
    }
  }
  config.resolved_interpretation_rules.clear();
  for trigger in &config.triggers {
    let rule = explicit_rules
      .remove(trigger.trigger_type.as_str())
      .unwrap_or_else(|| InvertInterpretationRule {
        trigger_type: trigger.trigger_type.clone(),
        direct_fact_predicates: config.default_direct_fact_predicates.clone(),
        source_include_predicates: config.default_source_include_predicates.clone(),
        source_include_context_prefixes: config.default_source_include_context_prefixes.clone(),
        direct_interpretation_id: config.default_direct_interpretation_id.clone(),
        rich_interpretation_id: config.default_rich_interpretation_id.clone(),
      });
    config
      .resolved_interpretation_rules
      .insert(trigger.trigger_type.clone(), rule);
  }
  Ok(())
}

fn validate_known_concept_field_references(
  concepts_by_term: &BTreeMap<String, Vec<ConceptDefinition>>,
  query_classifiers: &QueryClassifierConfig,
  invert: &InvertConfig,
  dialogue_templates: &DialogueTemplates,
) -> Result<()> {
  let mut known_scalar_fields = BTreeSet::new();
  let mut known_list_fields = BTreeSet::new();
  for concepts in concepts_by_term.values() {
    for concept in concepts {
      known_scalar_fields.extend(concept.scalar_fields.keys().cloned());
      known_list_fields.extend(concept.list_fields.keys().cloned());
    }
  }
  known_scalar_fields.extend(
    query_classifiers
      .source_fact_fields
      .iter()
      .map(|rule| rule.field.clone()),
  );
  known_list_fields.extend(
    query_classifiers
      .source_list_fields
      .iter()
      .map(|rule| rule.field.clone()),
  );

  for (section_name, section) in [
    (
      "kernel-definition-section",
      &dialogue_templates.definition_section,
    ),
    ("kernel-why-section", &dialogue_templates.why_section),
    (
      "kernel-property-section",
      &dialogue_templates.property_section,
    ),
  ] {
    for part in &section.parts {
      if let Some(field) = part.field_non_empty.as_deref() {
        if !known_scalar_fields.contains(field) {
          return Err(anyhow!(
            "unknown concept scalar field '{}' in {}",
            field,
            section_name
          ));
        }
      }
      if let Some(list) = part.list_non_empty.as_deref() {
        if !known_list_fields.contains(list) {
          return Err(anyhow!(
            "unknown concept list field '{}' in {}",
            list,
            section_name
          ));
        }
      }
    }
  }

  for rule in &invert.candidate_rules {
    if let Some(field) = rule.concept_field.as_deref() {
      if !known_scalar_fields.contains(field) {
        return Err(anyhow!(
          "unknown concept scalar field '{}' in invert-candidate-rules",
          field
        ));
      }
    }
  }

  Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Loader 에러 helper 군 (Option C: line/col suffix 부착)
//
// ## suffix coverage audit (batch 23 기준)
//
// `.px` 파일 경로를 carry 하는 canonical loader helper 는 전부 best-effort 로
// `(at line N, col M)` suffix 를 붙인다. coverage 표 (path-based, Option C 대상):
//
// | helper                          | suffix scanner                         |
// |---------------------------------|----------------------------------------|
// | `err_missing`                   | `locate_section_in_source` (top-level) |
// | `err_wrong_type`                | `locate_section_in_source` (top-level) |
// | `err_invalid_entry`             | `locate_section_in_source` (top-level) |
// | `err_missing_in_context`        | `locate_section_in_source` (top-level) |
// | `err_wrong_type_in_context`     | `locate_section_in_source` (top-level) |
// | `err_duplicate_entry`           | `locate_duplicate_entry_in_source` →   |
// |                                 | fallback `locate_section_in_source`    |
// | `err_missing_for_route`         | `locate_route_entry_in_source`         |
// | `err_with_location`             | `locate_section_in_source` (internal)  |
//
// path 가 없는 runtime-only helper 는 원리적으로 Option C 대상이 **아니다**:
//
// | helper                         | 이유                                     |
// |--------------------------------|------------------------------------------|
// | `err_missing_standalone_field` | path 없음, in-memory document 기반       |
// | `err_missing_runtime`          | path 없음, `RuntimeResources` 기반 lookup |
// | `err_missing_reopen_rule`      | path 없음, `RuntimeResources` 기반 lookup |
//
// ## suffix 가 붙지 않는 best-effort 케이스
//
// suffix 는 항상 **additive** 이다. scanner 가 실패하면 기존 base body 만 emit
// 한다. 회귀 테스트들은 전부 substring match 라서 suffix 가 있어도 없어도 통과
// 한다. 따라서 scanner 의 hit rate 에는 production assertion 이 걸려 있지 않다.
//
// ## scan limitation (batch 22-23)
//
//   1. `locate_section_in_source` 는 top-level `<section> =` line 만 본다. 중첩된
//      `parent.child` sub-path 는 leaf key 이름으로 fallback 스캔한다. 2 단계
//      nesting (parent → child → grandchild) 는 못 본다.
//   2. `locate_duplicate_entry_in_source` 는 section-scoped 로 동작한다 (batch 23).
//      section header 가 없는 root-list 파일 (예: `query-routes.px`) 에서는
//      file-wide fallback.
//   3. `locate_route_entry_in_source` 는 `route = "..."` 첫 매치만 리턴. route 의
//      uniqueness 는 loader 가 별도로 강제한다.
//   4. bracket depth 트래킹은 `[` / `]` 문자 카운트 기반이다. 문자열 안에 있는
//      `[` / `]` 는 구분 못 한다. `.px` config 파일 기준 실용적 범위 내.
//   5. 모든 scanner 는 에러 경로에서만 실행된다 → 정상 loading 성능 영향 없음.
//
// ## ratchet
//
// `kernel_rs_has_no_raw_anyhow_missing_literals_outside_helpers` 가 helper 밖
// single-line `anyhow!("missing "` literal 이 3 개 이하 (runtime-only helper 3 개
// 만) 를 강제한다. 새 inline literal 이 helper 밖에 추가되면 즉시 fail.
// ─────────────────────────────────────────────────────────────────────────────

/// Loader 에러 body 에 Option C 식 `(at line N, col M)` suffix 를 best-effort 로
/// 붙인다. `locate_section_in_source` 가 section 을 찾으면 suffix 를 덧붙이고,
/// 찾지 못하면 base 를 그대로 돌려준다. 모든 loader-level helper 가 이 함수를
/// 경유해서 에러를 만든다.
fn err_with_location(path: &Path, section: &str, base: String) -> anyhow::Error {
  match locate_section_in_source(path, section) {
    Some((line, col)) => anyhow!("{} (at line {}, col {})", base, line, col),
    None => anyhow!("{}", base),
  }
}

/// Loader 에러 메시지 owner. 기존 `anyhow!("missing '{section}' in {path}")` 패턴을
/// 한 곳에서 만든다. 에러 body 는 기본적으로 기존과 동일하지만, parent-section
/// 이 `.px` 파일에 보이면 `(at line N, col M)` suffix 를 best-effort 로 덧붙인다.
/// 회귀 테스트는 모두 substring match 라서 suffix 가 있어도 통과한다.
fn err_missing(path: &Path, section: &str) -> anyhow::Error {
  let base = format!("missing '{}' in {}", section, path.display());
  err_with_location(path, section, base)
}

/// Loader 에러 메시지 owner. 기존 `anyhow!("'{section}' must be {expected} in {path}")`
/// 패턴을 한 곳에서 만든다. 에러 body 는 기본적으로 기존과 동일하지만, `.px`
/// 파일을 다시 읽어 `section = ...` line 을 찾으면 `(at line N, col M)` suffix 를
/// 덧붙인다 (Option C). 회귀 테스트는 모두 substring match 라서 suffix 가 있어도
/// 통과한다.
fn err_wrong_type(path: &Path, section: &str, expected: &str) -> anyhow::Error {
  let base = format!("'{}' must be {} in {}", section, expected, path.display());
  err_with_location(path, section, base)
}

/// `.px` 파일에서 주어진 section key 의 정의 line 을 선형 탐색한다. `err_wrong_type`
/// / `err_missing` 같은 loader 에러 경로에서만 호출되므로 성능은 relevant 하지 않다.
///
/// 매칭 규칙:
///   1. section 이 `parent.child` 형태면 parent block 먼저 찾고, 그 안에서 child
///      를 찾는 건 현재 구현 범위 밖. 대신 leaf key 이름 (`child`) 으로 한 번 더
///      시도한다 (best-effort hint).
///   2. 각 라인을 trim 한 뒤 `<section> =` prefix 가 있는지 확인.
///
/// 찾으면 1-based (line, col) 리턴. 못 찾으면 `None`.
fn locate_section_in_source(path: &Path, section: &str) -> Option<(usize, usize)> {
  let text = std::fs::read_to_string(path).ok()?;
  let candidates: Vec<&str> = if let Some(idx) = section.rfind('.') {
    vec![section, &section[idx + 1..]]
  } else {
    vec![section]
  };
  for candidate in candidates {
    for (idx, line) in text.lines().enumerate() {
      let leading = line.len() - line.trim_start().len();
      let trimmed = line.trim_start();
      if let Some(rest) = trimmed.strip_prefix(candidate) {
        let after = rest.trim_start();
        if after.starts_with('=') {
          return Some((idx + 1, leading + 1));
        }
      }
    }
  }
  None
}

/// `.px` 파일에서 `route = "<route>"` 형태의 entry 정의 line 을 찾는다.
/// `err_missing_for_route` 가 어느 route entry 에서 필드가 누락됐는지 suffix 로
/// 표시하기 위한 전용 scanner. space 여부 (`route = "x"` vs `route="x"`) 를 같이
/// 허용.
fn locate_route_entry_in_source(path: &Path, route: &str) -> Option<(usize, usize)> {
  let text = std::fs::read_to_string(path).ok()?;
  let needle_spaced = format!("route = \"{}\"", route);
  let needle_tight = format!("route=\"{}\"", route);
  for (idx, line) in text.lines().enumerate() {
    let hit = line
      .find(needle_spaced.as_str())
      .or_else(|| line.find(needle_tight.as_str()));
    if let Some(col_idx) = hit {
      return Some((idx + 1, col_idx + 1));
    }
  }
  None
}

/// `.px` 파일에서 `<key_label> = "<key>"` 쌍의 **두 번째** 등장 line 을 찾는다.
/// `err_duplicate_entry` 가 duplicate 가 실제로 일어난 entry 위치를 suffix 로
/// 표시하기 위한 전용 scanner. space 여부 (`when = "x"` vs `when="x"`) 를 같이
/// 허용. 매치가 0 개거나 1 개면 `None` (fallback 처리는 호출자가 한다).
///
/// Section-scoping (batch 23):
///   - `section` 이 파일에 `<section> =` 형태의 header 로 존재하면, 그 header 에서
///     시작해 bracket depth 가 0 으로 돌아올 때까지의 line 범위로 scan 을 제한한다.
///     같은 key_label / key 쌍이 **다른** section 에 존재하더라도 false positive 를
///     일으키지 않는다.
///   - Section header 를 찾지 못하면 file 전체를 scan 한다 (예: `query-routes.px` 는
///     root 자체가 list 이므로 `query-routes =` header 가 없다). 이 경우는 기존
///     file-wide 동작.
///
/// Scan limitation:
///   - bracket depth 는 `[` / `]` 문자 카운트만 본다. 문자열 안에 있는 `[` / `]`
///     는 구분 못 함. `.px` config 파일 기준 실용적으로 충분하지만, 문자열 안에
///     닫는 bracket 이 들어 있는 edge case 에서는 section 범위가 일찍 끝날 수 있다.
///   - 중첩된 section (parent.child) 내부의 duplicate 는 parent section 범위 안에
///     떨어지므로 parent scope 로 잡힌다. child 단계 scope 로 더 좁히려면 2단계
///     header 탐색이 필요하지만 현재 구현 범위 밖이다.
fn locate_duplicate_entry_in_source(
  path: &Path,
  section: &str,
  key_label: &str,
  key: &str,
) -> Option<(usize, usize)> {
  let text = std::fs::read_to_string(path).ok()?;
  let lines: Vec<&str> = text.lines().collect();
  let needle_spaced = format!("{} = \"{}\"", key_label, key);
  let needle_tight = format!("{}=\"{}\"", key_label, key);

  // Section header line 을 먼저 찾는다. 있으면 그 line 부터 bracket depth 가 0 으로
  // 돌아올 때까지의 범위만 scan 한다.
  let section_start = lines.iter().position(|line| {
    let t = line.trim_start();
    if let Some(rest) = t.strip_prefix(section) {
      rest.trim_start().starts_with('=')
    } else {
      false
    }
  });

  let (scan_start, scan_end): (usize, usize) = match section_start {
    Some(start) => {
      let mut depth: i32 = 0;
      let mut seen_open = false;
      let mut end = lines.len();
      for (offset, line) in lines[start..].iter().enumerate() {
        for ch in line.chars() {
          match ch {
            '[' => {
              depth += 1;
              seen_open = true;
            }
            ']' => depth -= 1,
            _ => {}
          }
        }
        if seen_open && depth <= 0 {
          end = start + offset + 1;
          break;
        }
      }
      (start, end)
    }
    None => (0, lines.len()),
  };

  let mut seen = false;
  for idx in scan_start..scan_end {
    let line = lines[idx];
    let hit = line
      .find(needle_spaced.as_str())
      .or_else(|| line.find(needle_tight.as_str()));
    if let Some(col_idx) = hit {
      if seen {
        return Some((idx + 1, col_idx + 1));
      }
      seen = true;
    }
  }
  None
}

/// Loader 에러 메시지 owner.
/// `anyhow!("duplicate '{section}' entry for {key_label} '{key}' in {path}")`
/// 패턴을 한 곳에서 만든다. base body 는 기존과 동일하고, **실제 duplicate 가
/// 일어난 entry 위치** (같은 `<key_label> = "<key>"` 쌍의 두 번째 등장) 를
/// `locate_duplicate_entry_in_source` 로 찾아 `(at line N, col M)` suffix 로
/// 붙인다. duplicate scanner 가 실패하면 parent section 위치로 fallback.
fn err_duplicate_entry(path: &Path, section: &str, key_label: &str, key: &str) -> anyhow::Error {
  let base = format!(
    "duplicate '{}' entry for {} '{}' in {}",
    section,
    key_label,
    key,
    path.display()
  );
  match locate_duplicate_entry_in_source(path, section, key_label, key) {
    Some((line, col)) => anyhow!("{} (at line {}, col {})", base, line, col),
    None => err_with_location(path, section, base),
  }
}

/// Loader 에러 메시지 owner.
/// `anyhow!("invalid '{section}' entry in {path}")` 패턴을 한 곳에서 만든다.
/// base body 는 기존과 동일하고, parent section 위치가 보이면 best-effort suffix.
fn err_invalid_entry(path: &Path, section: &str) -> anyhow::Error {
  let base = format!("invalid '{}' entry in {}", section, path.display());
  err_with_location(path, section, base)
}

/// Loader 에러 메시지 owner (dynamic context).
/// `anyhow!("missing '{key}' in {context} of {path}")` 패턴을 한 곳에서 만든다.
/// attrset element helper 들 (`required_attrset_string` / `present_attrset_string`)
/// 이 이 helper 를 통해 에러를 낸다. base body 는 기존과 동일하고, `context` 에서
/// 유추 가능한 parent section (`context` 의 첫 token) 이 `.px` 파일에 보이면
/// best-effort 로 suffix 를 붙인다.
fn err_missing_in_context(path: &Path, key: &str, context: &str) -> anyhow::Error {
  let base = format!("missing '{}' in {} of {}", key, context, path.display());
  // context 는 보통 `"<section> entry"` 형태 (예: `"held-reason-rules entry"`).
  // 첫 whitespace 이전 token 을 parent section 후보로 사용.
  let parent_section = context.split_whitespace().next().unwrap_or(context);
  err_with_location(path, parent_section, base)
}

/// Loader 에러 메시지 owner (dynamic context).
/// `anyhow!("'{key}' in {context} must be {expected} in {path}")` 패턴을 한 곳에서
/// 만든다. body 는 기존과 동일.
fn err_wrong_type_in_context(
  path: &Path,
  key: &str,
  context: &str,
  expected: &str,
) -> anyhow::Error {
  let base = format!(
    "'{}' in {} must be {} in {}",
    key,
    context,
    expected,
    path.display()
  );
  // context 는 보통 `"<section> entry"` 형태. 첫 token 을 parent section 후보로
  // 사용해 best-effort suffix 를 붙인다.
  let parent_section = context.split_whitespace().next().unwrap_or(context);
  err_with_location(path, parent_section, base)
}

/// Loader 에러 메시지 owner (route-scoped).
/// `anyhow!("missing '{key}' for route '{route}' in {path}")` 패턴을 한 곳에서 만든다.
/// base body 는 기존과 동일하고, `.px` 파일에서 `route = "<route>"` entry 정의 line
/// 을 `locate_route_entry_in_source` 로 찾으면 `(at line N, col M)` suffix 를 붙인다.
fn err_missing_for_route(path: &Path, key: &str, route: &str) -> anyhow::Error {
  let base = format!(
    "missing '{}' for route '{}' in {}",
    key,
    route,
    path.display()
  );
  match locate_route_entry_in_source(path, route) {
    Some((line, col)) => anyhow!("{} (at line {}, col {})", base, line, col),
    None => anyhow!("{}", base),
  }
}

/// Standalone pnix query document 의 top-level 필드 누락 owner.
/// `anyhow!("missing '{field}' in standalone pnix query document")` 패턴.
/// body 는 기존과 동일.
fn err_missing_standalone_field(field: &str) -> anyhow::Error {
  anyhow!("missing '{}' in standalone pnix query document", field)
}

/// Runtime lookup 에러 owner (lookup 대상 / key 기반).
/// `anyhow!("missing {description} for '{key}'")` 패턴을 한 곳에서 만든다.
/// body 는 기존과 동일.
fn err_missing_runtime(description: &str, key: &str) -> anyhow::Error {
  anyhow!("missing {} for '{}'", description, key)
}

/// Runtime lookup 에러 owner (reopen-rules 에서만 사용).
/// `anyhow!("missing 'reopen-rules' entry for reason '{reason}'")` 패턴.
/// body 는 기존과 동일.
fn err_missing_reopen_rule(reason: &str) -> anyhow::Error {
  anyhow!("missing 'reopen-rules' entry for reason '{}'", reason)
}

fn optional_top_level_string(value: &PxValue, key: &str, path: &Path) -> Result<Option<String>> {
  match value.get(key) {
    Some(PxValue::String(s)) => Ok(Some(s.clone())),
    Some(_) => Err(err_wrong_type(path, key, "string")),
    None => Ok(None),
  }
}

fn required_top_level_string(value: &PxValue, key: &str, path: &Path) -> Result<String> {
  let field = optional_top_level_string(value, key, path)?;
  match field {
    Some(s) if !s.is_empty() => Ok(s),
    _ => Err(err_missing(path, key)),
  }
}

fn optional_top_level_string_list(
  value: &PxValue,
  key: &str,
  path: &Path,
) -> Result<Option<Vec<String>>> {
  match value.get(key) {
    Some(PxValue::List(_)) => Ok(Some(value.get(key).unwrap().as_string_list())),
    Some(_) => Err(err_wrong_type(path, key, "list")),
    None => Ok(None),
  }
}

fn required_top_level_string_list(value: &PxValue, key: &str, path: &Path) -> Result<Vec<String>> {
  let field = optional_top_level_string_list(value, key, path)?;
  match field {
    Some(items) if !items.is_empty() => Ok(items),
    _ => Err(err_missing(path, key)),
  }
}

fn present_top_level_string_list(value: &PxValue, key: &str, path: &Path) -> Result<Vec<String>> {
  let field = optional_top_level_string_list(value, key, path)?;
  match field {
    Some(items) => Ok(items),
    None => Err(err_missing(path, key)),
  }
}

fn optional_attrset_string(
  map: &BTreeMap<String, PxValue>,
  key: &str,
  context: &str,
  path: &Path,
) -> Result<Option<String>> {
  match map.get(key) {
    Some(PxValue::String(s)) => Ok(Some(s.clone())),
    Some(_) => Err(err_wrong_type_in_context(path, key, context, "string")),
    None => Ok(None),
  }
}

fn required_attrset_string(
  map: &BTreeMap<String, PxValue>,
  key: &str,
  context: &str,
  path: &Path,
) -> Result<String> {
  let value = optional_attrset_string(map, key, context, path)?;
  match value {
    Some(s) if !s.is_empty() => Ok(s),
    _ => Err(err_missing_in_context(path, key, context)),
  }
}

fn optional_attrset_f64(
  map: &BTreeMap<String, PxValue>,
  key: &str,
  context: &str,
  path: &Path,
) -> Result<Option<f64>> {
  let Some(raw) = optional_attrset_string(map, key, context, path)? else {
    return Ok(None);
  };
  raw
    .parse::<f64>()
    .map(Some)
    .map_err(|_| anyhow!("'{}' in {} must be f64 in {}", key, context, path.display()))
}

fn optional_attrset_meaning_status(
  map: &BTreeMap<String, PxValue>,
  key: &str,
  context: &str,
  path: &Path,
) -> Result<Option<MeaningStatus>> {
  let Some(raw) = optional_attrset_string(map, key, context, path)? else {
    return Ok(None);
  };
  let status = match raw.as_str() {
    "candidate" => MeaningStatus::Candidate,
    "accepted" => MeaningStatus::Accepted,
    "rejected" => MeaningStatus::Rejected,
    "contradicted" => MeaningStatus::Contradicted,
    "held" => MeaningStatus::Held,
    "deprecated" => MeaningStatus::Deprecated,
    "deleted" => MeaningStatus::Deleted,
    _ => {
      return Err(anyhow!(
        "invalid '{}' for {} in {} (got '{}', allowed: [\"candidate\", \"accepted\", \"rejected\", \"contradicted\", \"held\", \"deprecated\", \"deleted\"])",
        key,
        context,
        path.display(),
        raw
      ))
    }
  };
  Ok(Some(status))
}

fn require_allowed_literal(
  value: &str,
  key: &str,
  context: &str,
  path: &Path,
  allowed: &[&str],
) -> Result<()> {
  if allowed.iter().any(|candidate| *candidate == value) {
    Ok(())
  } else {
    Err(anyhow!(
      "invalid '{}' for {} in {} (got '{}', allowed: {:?})",
      key,
      context,
      path.display(),
      value,
      allowed
    ))
  }
}

fn present_attrset_string(
  map: &BTreeMap<String, PxValue>,
  key: &str,
  context: &str,
  path: &Path,
) -> Result<String> {
  let value = optional_attrset_string(map, key, context, path)?;
  match value {
    Some(s) => Ok(s),
    None => Err(err_missing_in_context(path, key, context)),
  }
}

fn present_attrset_string_list(
  map: &BTreeMap<String, PxValue>,
  key: &str,
  context: &str,
  path: &Path,
) -> Result<Vec<String>> {
  let value = optional_attrset_string_list(map, key, context, path)?;
  match value {
    Some(items) => Ok(items),
    None => Err(err_missing_in_context(path, key, context)),
  }
}

fn optional_attrset_string_list(
  map: &BTreeMap<String, PxValue>,
  key: &str,
  context: &str,
  path: &Path,
) -> Result<Option<Vec<String>>> {
  match map.get(key) {
    Some(PxValue::List(_)) => Ok(Some(map.get(key).unwrap().as_string_list())),
    Some(_) => Err(err_wrong_type_in_context(path, key, context, "list")),
    None => Ok(None),
  }
}

fn parse_required_route_f64(
  map: &BTreeMap<String, PxValue>,
  key: &str,
  path: &Path,
  route: &str,
) -> Result<f64> {
  map
    .get(key)
    .and_then(|v| v.as_str())
    .and_then(|s| s.parse::<f64>().ok())
    .ok_or_else(|| err_missing_for_route(path, key, route))
}

fn parse_required_route_usize(
  map: &BTreeMap<String, PxValue>,
  key: &str,
  path: &Path,
  route: &str,
) -> Result<usize> {
  map
    .get(key)
    .and_then(|v| v.as_str())
    .and_then(|s| s.parse::<usize>().ok())
    .filter(|value| *value > 0)
    .ok_or_else(|| err_missing_for_route(path, key, route))
}
