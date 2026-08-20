use pnix_query_runtime::{
  px::{parse_px, parse_px_file_with_pnix_eval_fallback},
  KernelPaths, PnixReplKernel,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

fn cleanup_stale_test_dirs() {
  let tmp = std::env::temp_dir();
  let my_pid = format!("-{}-", std::process::id());
  if let Ok(entries) = fs::read_dir(&tmp) {
    for entry in entries.flatten() {
      let name = entry.file_name();
      let name = name.to_string_lossy();
      if name.starts_with("pnix-query-runtime-tests-") && !name.contains(&my_pid) {
        let _ = fs::remove_dir_all(entry.path());
      }
    }
  }
}

fn temp_px_path(name: &str) -> PathBuf {
  static CLEANUP: std::sync::Once = std::sync::Once::new();
  CLEANUP.call_once(cleanup_stale_test_dirs);
  static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(1);
  let temp_id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
  let dir = std::env::temp_dir().join(format!(
    "pnix-query-runtime-tests-{}-{temp_id}",
    std::process::id()
  ));
  fs::create_dir_all(&dir).expect("create temp dir");
  dir.join(name)
}

fn write_px(path: &Path, body: &str) {
  fs::write(path, body).expect("write px");
}

fn data_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("data")
    .canonicalize()
    .expect("canonicalize data dir")
}

#[test]
fn standalone_query_document_kind_is_required() {
  let mut kernel = PnixReplKernel::new(KernelPaths::from_data_dir(data_dir()));
  let err = kernel
    .evaluate_px_source(r#"{ utterance = "힘은 뭐야"; }"#)
    .expect_err("missing kind should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'kind' in standalone pnix query document"),
    "{err:#}"
  );
}

#[test]
fn standalone_query_document_scope_requires_known_values() {
  let mut kernel = PnixReplKernel::new(KernelPaths::from_data_dir(data_dir()));
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "bogus"; utterance = "힘은 뭐야"; }"#)
    .expect_err("invalid scope should fail");

  assert!(
    err
      .to_string()
      .contains("invalid 'scope' for standalone pnix query document"),
    "{err:#}"
  );
}

#[test]
fn standalone_query_document_scope_is_required() {
  let mut kernel = PnixReplKernel::new(KernelPaths::from_data_dir(data_dir()));
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; utterance = "힘은 뭐야"; }"#)
    .expect_err("missing scope should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'scope' in standalone pnix query document"),
    "{err:#}"
  );
}

#[test]
fn standalone_query_document_scope_must_be_string() {
  let mut kernel = PnixReplKernel::new(KernelPaths::from_data_dir(data_dir()));
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = ["standard"]; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("non-string scope should fail");

  assert!(
    err
      .to_string()
      .contains("'scope' must be string in standalone pnix query document"),
    "{err:#}"
  );
}

#[test]
fn px_parser_rejects_duplicate_attrset_keys() {
  let err = parse_px(r#"{ route = "a"; route = "b"; }"#).expect_err("duplicate key should fail");

  assert!(
    err
      .to_string()
      .contains("duplicate attrset key 'route' in .px"),
    "{err:#}"
  );
}

#[test]
fn query_classifiers_dispatch_routes_reject_duplicate_attrset_keys() {
  let query_classifiers = temp_px_path("query-classifiers-duplicate-dispatch-route.px");
  let query_routes = temp_px_path("query-routes-minimal-duplicate-dispatch-route.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        definition = "concept-definition-lookup-2";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      concept-what-markers = ["뭐"];
      concept-definition-suffixes = ["이란"];
      concept-explain-markers = ["설명"];
      concept-explain-skip-tokens = ["설명"];
      definition-query-rules = [
        { match-any = ["뭐"]; }
      ];
      question-word-stems = ["왜"];
      term-extraction-particle-kinds = ["topic"];
      term-extraction-suffixes = ["은" "는"];
      term-normalization-trim-chars = ["?" " "];
      term-fallback-policy = "known-concept-token-scan";
      predicate-classifiers = [
        { match-any = ["단위"]; predicate = "unit-ko"; label-ko = "단위"; }
      ];
    }
    "#,
  );

  let err = PnixReplKernel::new(KernelPaths {
    data_dir: data_dir(),
    korean_morphology_path: data_dir().join("korean-morphology.px"),
    query_classifiers_path: query_classifiers.clone(),
    query_routes_path: query_routes.clone(),
    query_route_defaults_path: data_dir().join("query-route-defaults.px"),
    concepts_dir: data_dir().join("concepts"),
    followup_generation_path: data_dir().join("followup-generation.px"),
    ontology_invert_path: data_dir().join("ontology-invert.px"),
    synonyms_path: data_dir().join("concepts/synonyms.px"),
    dialogue_templates_path: data_dir().join("dialogue-templates.px"),
    kernel_base_facts_path: data_dir().join("kernel-base-facts.px"),
  })
  .evaluate_px_source(
    r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
  )
  .expect_err("duplicate attrset key in query-classifiers should fail");

  assert!(
    format!("{err:#}").contains("duplicate attrset key 'definition' in .px"),
    "{err:#}"
  );
}

fn write_minimal_why_kernel_fixtures(query_classifiers: &Path, query_routes: &Path) {
  write_px(
    query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐" "무엇" "뭔"]; }
        { match-any = ["이란" "란"]; }
        { match-any = ["설명" "알려" "에 대해" "에 관해" "에 대하여" "에 관하여"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["왜"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  write_px(
    query_routes,
    r#"
    [
      {
        route = "ontology-invert-causal-inverse";
        query-context = "Pnix.Query.why.causal.inverse";
        include-hop-knowledge = "false";
        default-preview = "3";
        policy-coverage = "0.0";
        policy-coherence = "0.0";
        policy-loss = "0.0";
        policy-cost = "0.0";
        policy-accept-threshold = "0.0";
      }
    ]
    "#,
  );
}

fn write_query_classifier_fixture(
  query_classifiers: &Path,
  held_reason_rules: &str,
  source_fact_fields: &str,
  source_list_fields: &str,
  definition_query_rules: &str,
  predicate_classifiers: &str,
) {
  write_px(
    query_classifiers,
    &format!(
      r#"
    {{
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {{
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      }};
      held-reason-keys = {{
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      }};
      held-reason-rules = {held_reason_rules};
      kernel-source-fact-fields = {source_fact_fields};
      kernel-source-list-fields = {source_list_fields};
      definition-query-rules = {definition_query_rules};
      predicate-classifiers = {predicate_classifiers};
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }}
    "#
    ),
  );
}

fn write_followup_fixture(
  followups: &Path,
  reopen_rules: &str,
  choice_rules: &str,
  resolved_term_rules: &str,
  held_response_rules: &str,
  _default_reopen_rule: &str,
) {
  write_px(
    followups,
    &format!(
      r#"
    {{
      disambiguation-questions = [
        {{ distinguishing-predicate = "experimental-context"; question-template = "CTX ${{term}}"; choices-template = ""; }}
      ];
      reason-question-rules = [
        {{ reason = "requires-context"; predicate = "experimental-context"; }}
        {{ reason = "unknown-term"; predicate = "experimental-context"; }}
      ];
      reopen-rules = {reopen_rules};
      choice-rules = {choice_rules};
      resolved-term-rules = {resolved_term_rules};
      held-response-rules = {held_response_rules};
      default-choices = ["선택A"];
      unknown-term-label = "질문";
      concept-choices = [];
    }}
    "#
    ),
  );
}

#[test]
fn query_dispatch_priority_is_owned_by_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["definition" "property" "why"];
      kernel-dispatch-routes = {
        definition = "custom-definition-route";
        property = "custom-property-route";
        held = "custom-held-route";
      };
      held-reason-keys = {
        requires-context = "ctx-needed";
        unknown-term = "unknown-concept";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["설명"]; }
      ];
      predicate-classifiers = [
        { match-any = ["공식"]; predicate = "formula"; label-ko = "공식"; }
      ];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  write_px(
    &query_routes,
    r#"
    [
      {
        route = "custom-definition-route";
        query-context = "Pnix.Query.CustomDefinition";
        include-hop-knowledge = "true";
        default-preview = "5";
        policy-coverage = "0.0";
        policy-coherence = "0.0";
        policy-loss = "0.0";
        policy-cost = "0.0";
        policy-accept-threshold = "0.0";
        kernel-direct-fact-predicates = ["definition-ko"];
        kernel-direct-interpretation-id = "custom.definition.direct.${term}";
        kernel-rich-interpretation-id = "custom.definition.rich.${term}";
      }
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  let mut kernel = PnixReplKernel::new(paths);

  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 설명 공식"; }"#,
    )
    .expect("evaluate");

  assert_eq!(response.route, "custom-definition-route");
  assert!(response.response_document_org.contains("힘:"));
  assert!(!response.response_document_org.contains("힘의 공식:"));
  assert!(response
    .response_document_org
    .contains("custom.definition.direct.힘"));
  assert!(response.response_document_px.contains(&format!(
    "  summary = \"{}\";",
    response.summary.replace('"', "'")
  )));
}

#[test]
fn query_dispatch_priority_rejects_unknown_stage_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["definition" "bogus"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["설명"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 설명"; }"#,
    )
    .expect_err("invalid dispatch stage should fail");

  assert!(
    err
      .to_string()
      .contains("invalid 'query-dispatch-priority' entry 'bogus'"),
    "{err:#}"
  );
}

#[test]
fn query_classifiers_root_must_be_attrset() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    [
      "bogus"
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 설명"; }"#,
    )
    .expect_err("query-classifiers root type should fail");

  assert!(
    err
      .to_string()
      .contains("query-classifiers root must be attrset"),
    "{err:#}"
  );
}

#[test]
fn query_dispatch_priority_must_be_list() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = "definition";
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["설명"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 설명"; }"#,
    )
    .expect_err("query-dispatch-priority wrong type should fail");

  assert!(
    err
      .to_string()
      .contains("'query-dispatch-priority' must be list"),
    "{err:#}"
  );
}

#[test]
fn query_classifiers_dispatch_routes_must_be_attrset() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["definition" "property" "why"];
      kernel-dispatch-routes = "bogus";
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["설명"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 설명"; }"#,
    )
    .expect_err("kernel-dispatch-routes type should fail");

  assert!(
    err
      .to_string()
      .contains("'kernel-dispatch-routes' must be attrset"),
    "{err:#}"
  );
}

#[test]
fn query_classifiers_dispatch_route_fields_must_be_string() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["definition" "property" "why"];
      kernel-dispatch-routes = {
        definition = ["concept-definition-lookup"];
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["설명"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 설명"; }"#,
    )
    .expect_err("kernel-dispatch-routes nested type should fail");

  assert!(
    err
      .to_string()
      .contains("'definition' in kernel-dispatch-routes must be string"),
    "{err:#}"
  );
}

#[test]
fn query_classifiers_held_reason_key_fields_must_be_string() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["definition" "property" "why"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = ["requires-context"];
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["설명"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 설명"; }"#,
    )
    .expect_err("held-reason-keys nested type should fail");

  assert!(
    err
      .to_string()
      .contains("'requires-context' in held-reason-keys must be string"),
    "{err:#}"
  );
}

#[test]
fn query_classifiers_term_fallback_policy_must_be_string() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["definition" "property" "why"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["설명"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?"];
      term-fallback-policy = ["known-concept-token-scan"];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 설명"; }"#,
    )
    .expect_err("term-fallback-policy wrong type should fail");

  assert!(
    err
      .to_string()
      .contains("'term-fallback-policy' must be string"),
    "{err:#}"
  );
}

#[test]
fn query_classifiers_term_fallback_policy_requires_known_values() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["definition" "property" "why"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["설명"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?"];
      term-fallback-policy = "bogus";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 설명"; }"#,
    )
    .expect_err("invalid term-fallback-policy should fail");

  assert!(
    err
      .to_string()
      .contains("invalid 'term-fallback-policy' for query-classifiers"),
    "{err:#}"
  );
}

#[test]
fn query_classifiers_concept_what_markers_are_required() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["definition" "property" "why"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["설명"]; }
      ];
      predicate-classifiers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 설명"; }"#,
    )
    .expect_err("missing concept-what-markers should fail");

  assert!(
    err.to_string().contains("missing 'concept-what-markers'"),
    "{err:#}"
  );
}

#[test]
fn query_classifiers_question_word_stems_are_required() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["definition" "property" "why"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["설명"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 설명"; }"#,
    )
    .expect_err("missing question-word-stems should fail");

  assert!(
    err.to_string().contains("missing 'question-word-stems'"),
    "{err:#}"
  );
}

#[test]
fn query_classifiers_term_extraction_suffixes_are_required() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["definition" "property" "why"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["설명"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 설명"; }"#,
    )
    .expect_err("missing term-extraction-suffixes should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'term-extraction-suffixes'"),
    "{err:#}"
  );
}

#[test]
fn query_classifiers_held_reason_rules_must_be_list() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["definition" "property" "why"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = "bogus";
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["설명"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 설명"; }"#,
    )
    .expect_err("held-reason-rules type should fail");

  assert!(
    err.to_string().contains("'held-reason-rules' must be list"),
    "{err:#}"
  );
}

#[test]
fn followup_disambiguation_questions_are_owned_by_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "ctx-needed";
        unknown-term = "unknown-concept";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐" "무엇" "뭔"]; }
        { match-any = ["이란" "란"]; }
        { match-any = ["설명" "알려" "에 대해" "에 관해" "에 대하여" "에 관하여"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = ["뭐"];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "unknown-term"; question-template = "ROUTE ${term}"; choices-template = " :: ${suggestions}"; }
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = " :: ${suggestions}"; }
      ];
      reason-question-rules = [
        { reason = "unknown-concept"; predicate = "unknown-term"; }
        { reason = "ctx-needed"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        {
          reason = "ctx-needed";
          carry-term-policy = "when-missing";
          effective-utterance-template = "${term}의 ${utterance}";
        }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      concept-choices = [];
      default-choices = ["선택A" "선택B"];
      held-response-rules = [
        { when = "term-present"; template = "HELD ${term}"; emit-held-term = "true"; }
        { when = "term-missing"; template = "HELD NONE"; emit-held-term = "false"; }
      ];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);

  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "푸바는 뭐야"; }"#,
    )
    .expect("evaluate");

  assert_eq!(response.route, "lightweight-korean-dialogue-held");
  assert!(
    response
      .envelope
      .notes
      .iter()
      .any(|note| note == "held-reason:unknown-concept"),
    "{:?}",
    response.envelope.notes
  );
}

#[test]
fn followup_requires_context_route_uses_px_reason_key() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "ctx-needed";
        unknown-term = "unknown-concept";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐" "무엇" "뭔"]; }
        { match-any = ["이란" "란"]; }
        { match-any = ["설명" "알려" "에 대해" "에 관해" "에 대하여" "에 관하여"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = ["뭐"];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "unknown-term"; question-template = "ROUTE ${term}"; choices-template = " :: ${suggestions}"; }
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = " :: ${suggestions}"; }
      ];
      reason-question-rules = [
        { reason = "unknown-concept"; predicate = "unknown-term"; }
        { reason = "ctx-needed"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      concept-choices = [];
      default-choices = ["선택A" "선택B"];
      held-response-rules = [
        { when = "term-present"; template = "HELD ${term}"; emit-held-term = "true"; }
        { when = "term-missing"; template = "HELD NONE"; emit-held-term = "false"; }
      ];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);

  let response = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect("evaluate");

  assert_eq!(
    response.follow_up_hint.as_deref(),
    Some("CTX 힘 :: 선택A, 선택B")
  );
  assert!(
    response
      .envelope
      .notes
      .iter()
      .any(|note| note == "held-reason:ctx-needed" || note == "held-reason:requires-context"),
    "{:?}",
    response.envelope.notes
  );
}

#[test]
fn followup_reopen_rules_are_owned_by_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["property" "definition" "why"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "ctx-needed";
        unknown-term = "unknown-concept";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐" "무엇" "뭔"]; }
        { match-any = ["이란" "란"]; }
        { match-any = ["설명" "알려" "에 대해" "에 관해" "에 대하여" "에 관하여"]; }
      ];
      predicate-classifiers = [
        { match-any = ["공식"]; predicate = "formula"; label-ko = "공식"; }
      ];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = " :: ${suggestions}"; }
      ];
      reason-question-rules = [
        { reason = "ctx-needed"; predicate = "experimental-context"; }
        { reason = "unknown-concept"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        {
          reason = "ctx-needed";
          carry-term-policy = "when-missing";
          effective-utterance-template = "${term} ${utterance}";
        }
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      concept-choices = [];
      default-choices = ["뉴턴 역학에서?"];
      held-response-rules = [
        { when = "term-present"; template = "HELD ${term}"; emit-held-term = "true"; }
        { when = "term-missing"; template = "HELD NONE"; emit-held-term = "false"; }
      ];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);

  let first = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect("first evaluate");
  assert_eq!(first.route, "lightweight-korean-dialogue-held");

  let second = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "공식은 뭐야"; }"#,
    )
    .expect("second evaluate");
  assert_eq!(second.route, "concept-predicate-lookup");
  assert!(second.response_document_org.contains("힘의 공식"));
}

#[test]
fn followup_reopen_templates_require_carried_term_when_using_term_placeholder() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["property" "definition" "why"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "ctx-needed";
        unknown-term = "unknown-concept";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐" "무엇" "뭔"]; }
        { match-any = ["이란" "란"]; }
        { match-any = ["설명" "알려" "에 대해" "에 관해" "에 대하여" "에 관하여"]; }
      ];
      predicate-classifiers = [
        { match-any = ["공식"]; predicate = "formula"; label-ko = "공식"; }
      ];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = " :: ${suggestions}"; }
      ];
      reason-question-rules = [
        { reason = "ctx-needed"; predicate = "experimental-context"; }
        { reason = "unknown-concept"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        {
          reason = "ctx-needed";
          carry-term-policy = "never";
          effective-utterance-template = "${term}의 ${utterance}";
        }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      concept-choices = [];
      default-choices = ["뉴턴 역학에서?"];
      held-response-rules = [
        { when = "term-present"; template = "HELD ${term}"; emit-held-term = "true"; }
        { when = "term-missing"; template = "HELD NONE"; emit-held-term = "false"; }
      ];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);

  let first = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect("first evaluate");
  assert_eq!(first.route, "lightweight-korean-dialogue-held");

  let error = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "공식은 뭐야"; }"#,
    )
    .expect_err("reopen template requiring missing carried term should fail");

  assert!(
    error
      .to_string()
      .contains("reopen effective-utterance template requires carried term"),
    "{error:#}"
  );
}

#[test]
fn followup_generation_root_must_be_attrset() {
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    [
      "bogus"
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);

  let error = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("followup-generation root type should fail");

  assert!(
    error
      .to_string()
      .contains("followup-generation root must be attrset"),
    "{error:#}"
  );
}

#[test]
fn followup_default_choices_must_be_list() {
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      held-response-rules = [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
      ];
      concept-choices = [];
      default-choices = "선택A";
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);

  let error = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("default-choices wrong type should fail");

  assert!(
    error.to_string().contains("'default-choices' must be list"),
    "{error:#}"
  );
}

#[test]
fn followup_reason_question_rules_must_be_list() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"[
      { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
      { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
    ]"#,
    r#"[
      { field = "definition-ko"; predicate = "definition-ko"; }
      { field = "formal-name-en"; predicate = "formal-name-en"; }
      { field = "formal-symbol"; predicate = "formal-symbol"; }
      { field = "domain"; predicate = "domain"; }
      { field = "unit-ko"; predicate = "unit-ko"; }
      { field = "formula"; predicate = "formula"; }
      { field = "inverse-of"; predicate = "inverse-of"; }
      { field = "category"; predicate = "category"; }
      { field = "why"; predicate = "why"; }
      { field = "boundary-conditions"; predicate = "boundary-condition"; }
    ]"#,
    r#"[
      { field = "related-concepts"; predicate = "related-concept"; }
    ]"#,
    r#"[
      { match-any = ["뭐"]; }
    ]"#,
    "[]",
  );
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
        { distinguishing-predicate = "unknown-term"; question-template = "UNKNOWN ${term}"; choices-template = ""; }
      ];
      reason-question-rules = "bogus";
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      default-choices = ["선택A"];
      held-response-rules = [
        { when = "term-present"; template = "HELD ${term}"; emit-held-term = "true"; }
        { when = "term-missing"; template = "HELD NONE"; emit-held-term = "false"; }
      ];
      unknown-term-label = "질문";
      concept-choices = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);

  let error = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("reason-question-rules type should fail");

  assert!(
    error
      .to_string()
      .contains("'reason-question-rules' must be list"),
    "{error:#}"
  );
}

#[test]
fn followup_unknown_term_label_must_be_string() {
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      held-response-rules = [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
      ];
      concept-choices = [];
      default-choices = ["선택A"];
      unknown-term-label = ["질문"];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);

  let error = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("unknown-term-label wrong type should fail");

  assert!(
    error
      .to_string()
      .contains("'unknown-term-label' must be string"),
    "{error:#}"
  );
}

#[test]
fn dialogue_templates_root_must_be_attrset() {
  let dialogue_templates = temp_px_path("dialogue-templates.px");
  write_px(
    &dialogue_templates,
    r#"
    [
      "bogus"
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.dialogue_templates_path = dialogue_templates;
  let mut kernel = PnixReplKernel::new(paths);

  let error = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("dialogue-templates root type should fail");

  assert!(
    error
      .to_string()
      .contains("dialogue-templates root must be attrset"),
    "{error:#}"
  );
}

#[test]
fn dialogue_route_summary_must_be_attrset() {
  let dialogue_templates = temp_px_path("dialogue-templates.px");
  write_px(
    &dialogue_templates,
    r#"
    {
      kernel-definition-section = {
        join-with = ". ";
        suffix = ".";
        parts = [
          { when = "always"; template = "BASE ${term}"; }
        ];
      };
      kernel-why-section = {
        join-with = " ";
        suffix = "";
        parts = [
          { when = "always"; template = "WHY ${regime}"; }
        ];
      };
      kernel-property-section = {
        join-with = "";
        suffix = "";
        parts = [
          { when = "always"; values-state = "present"; template = "PROP ${term} ${values}"; }
          { when = "always"; values-state = "empty"; template = "EMPTY ${term}"; }
        ];
      };
      kernel-route-summary = "bogus";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.dialogue_templates_path = dialogue_templates;
  let mut kernel = PnixReplKernel::new(paths);

  let error = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("kernel-route-summary type should fail");

  assert!(
    error
      .to_string()
      .contains("'kernel-route-summary' must be attrset"),
    "{error:#}"
  );
}

#[test]
fn dialogue_template_sections_control_response_parts() {
  let dialogue_templates = temp_px_path("dialogue-templates.px");
  write_px(
    &dialogue_templates,
    r#"
    {
      kernel-definition-section = {
        join-with = " | ";
        suffix = "!";
        parts = [
          { when = "always"; template = "BASE ${term}"; }
          { when = "always"; field-non-empty = "formula"; template = "FORM ${formula}"; }
        ];
      };
      kernel-why-section = {
        join-with = " / ";
        suffix = "";
        parts = [
          { when = "always"; template = "WHY ${regime}"; }
        ];
      };
      kernel-property-section = {
        join-with = "";
        suffix = "";
        parts = [
          { when = "always"; values-state = "present"; template = "PROP ${term} ${values}"; }
          { when = "always"; values-state = "empty"; template = "EMPTY ${term}"; }
        ];
      };
      kernel-route-summary = {
        definition = "def";
        property = "prop";
        why = "why";
        held = "held";
      };
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.dialogue_templates_path = dialogue_templates;
  let mut kernel = PnixReplKernel::new(paths);

  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect("evaluate");

  assert!(
    response
      .response_document_org
      .contains("BASE 힘 | FORM F = ma"),
    "{}",
    response.response_document_org
  );
}

#[test]
fn dialogue_templates_require_present_optional_values_when_placeholder_is_used() {
  let dialogue_templates = temp_px_path("dialogue-templates.px");
  let concepts_dir = temp_px_path("concepts");
  fs::create_dir_all(&concepts_dir).expect("create concepts dir");
  write_px(
    &concepts_dir.join("minimal-force.px"),
    r#"
    [
      {
        term-ko = "힘";
        definition-ko = "정의";
        context = "Physics.Mechanics";
        domain = "물리";
      }
    ]
    "#,
  );
  write_px(
    &dialogue_templates,
    r#"
    {
      kernel-definition-section = {
        join-with = ". ";
        suffix = ".";
        parts = [
          { when = "always"; template = "BASE ${term}"; }
          { when = "always"; template = "FORM ${formula}"; }
        ];
      };
      kernel-why-section = {
        join-with = " ";
        suffix = "";
        parts = [
          { when = "always"; template = "WHY ${regime}"; }
        ];
      };
      kernel-property-section = {
        join-with = "";
        suffix = "";
        parts = [
          { when = "always"; values-state = "present"; template = "PROP ${term} ${values}"; }
          { when = "always"; values-state = "empty"; template = "EMPTY ${term}"; }
        ];
      };
      kernel-route-summary = {
        definition = "def";
        property = "prop";
        why = "why";
        held = "held";
      };
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.concepts_dir = concepts_dir;
  paths.dialogue_templates_path = dialogue_templates;
  let mut kernel = PnixReplKernel::new(paths);

  let error = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("optional definition placeholder should require present value");

  assert!(
    error
      .to_string()
      .contains("definition template requires formula"),
    "{error:#}"
  );
}

#[test]
fn dialogue_template_parts_require_explicit_when() {
  let dialogue_templates = temp_px_path("dialogue-templates.px");
  write_px(
    &dialogue_templates,
    r#"
    {
      kernel-definition-section = {
        join-with = ". ";
        suffix = ".";
        parts = [
          { template = "BASE ${term}"; }
        ];
      };
      kernel-why-section = {
        join-with = " ";
        suffix = "";
        parts = [
          { when = "always"; template = "WHY ${regime}"; }
        ];
      };
      kernel-property-section = {
        join-with = "";
        suffix = "";
        parts = [
          { when = "always"; values-state = "present"; template = "PROP ${term} ${values}"; }
          { when = "always"; values-state = "empty"; template = "EMPTY ${term}"; }
        ];
      };
      kernel-route-summary = {
        definition = "def";
        property = "prop";
        why = "why";
        held = "held";
      };
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.dialogue_templates_path = dialogue_templates;
  let mut kernel = PnixReplKernel::new(paths);

  let error = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("missing when should fail");

  assert!(error.to_string().contains("missing 'when'"), "{error:#}");
}

#[test]
fn dialogue_template_section_join_with_must_be_string() {
  let dialogue_templates = temp_px_path("dialogue-templates.px");
  write_px(
    &dialogue_templates,
    r#"
    {
      kernel-definition-section = {
        join-with = [". "];
        suffix = ".";
        parts = [
          { when = "always"; template = "BASE ${term}"; }
        ];
      };
      kernel-why-section = {
        join-with = " ";
        suffix = "";
        parts = [
          { when = "always"; template = "WHY ${regime}"; }
        ];
      };
      kernel-property-section = {
        join-with = "";
        suffix = "";
        parts = [
          { when = "always"; values-state = "present"; template = "PROP ${term} ${values}"; }
          { when = "always"; values-state = "empty"; template = "EMPTY ${term}"; }
        ];
      };
      kernel-route-summary = {
        definition = "def";
        property = "prop";
        why = "why";
        held = "held";
      };
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.dialogue_templates_path = dialogue_templates;
  let mut kernel = PnixReplKernel::new(paths);

  let error = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("join-with wrong type should fail");

  assert!(
    error
      .to_string()
      .contains("'join-with' in kernel-definition-section must be string"),
    "{error:#}"
  );
}

#[test]
fn dialogue_template_section_join_with_is_required() {
  let dialogue_templates = temp_px_path("dialogue-templates.px");
  write_px(
    &dialogue_templates,
    r#"
    {
      kernel-definition-section = {
        suffix = ".";
        parts = [
          { when = "always"; template = "BASE ${term}"; }
        ];
      };
      kernel-why-section = {
        join-with = " ";
        suffix = "";
        parts = [
          { when = "always"; template = "WHY ${regime}"; }
        ];
      };
      kernel-property-section = {
        join-with = "";
        suffix = "";
        parts = [
          { when = "always"; values-state = "present"; template = "PROP ${term} ${values}"; }
          { when = "always"; values-state = "empty"; template = "EMPTY ${term}"; }
        ];
      };
      kernel-route-summary = {
        definition = "def";
        property = "prop";
        why = "why";
        held = "held";
      };
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.dialogue_templates_path = dialogue_templates;
  let mut kernel = PnixReplKernel::new(paths);

  let error = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("missing join-with should fail");

  assert!(
    error
      .to_string()
      .contains("missing 'join-with' in kernel-definition-section"),
    "{error:#}"
  );
}

#[test]
fn dialogue_template_section_suffix_is_required() {
  let dialogue_templates = temp_px_path("dialogue-templates.px");
  write_px(
    &dialogue_templates,
    r#"
    {
      kernel-definition-section = {
        join-with = ". ";
        parts = [
          { when = "always"; template = "BASE ${term}"; }
        ];
      };
      kernel-why-section = {
        join-with = " ";
        suffix = "";
        parts = [
          { when = "always"; template = "WHY ${regime}"; }
        ];
      };
      kernel-property-section = {
        join-with = "";
        suffix = "";
        parts = [
          { when = "always"; values-state = "present"; template = "PROP ${term} ${values}"; }
          { when = "always"; values-state = "empty"; template = "EMPTY ${term}"; }
        ];
      };
      kernel-route-summary = {
        definition = "def";
        property = "prop";
        why = "why";
        held = "held";
      };
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.dialogue_templates_path = dialogue_templates;
  let mut kernel = PnixReplKernel::new(paths);

  let error = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("missing suffix should fail");

  assert!(
    error
      .to_string()
      .contains("missing 'suffix' in kernel-definition-section"),
    "{error:#}"
  );
}

#[test]
fn dialogue_template_sections_must_exist() {
  let dialogue_templates = temp_px_path("dialogue-templates.px");
  write_px(
    &dialogue_templates,
    r#"
    {
      kernel-definition-section = {
        join-with = ". ";
        suffix = ".";
        parts = [
          { when = "always"; template = "BASE ${term}"; }
        ];
      };
      kernel-property-section = {
        join-with = "";
        suffix = "";
        parts = [
          { when = "always"; values-state = "present"; template = "PROP ${term} ${values}"; }
          { when = "always"; values-state = "empty"; template = "EMPTY ${term}"; }
        ];
      };
      kernel-route-summary = {
        definition = "def";
        property = "prop";
        why = "why";
        held = "held";
      };
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.dialogue_templates_path = dialogue_templates;
  let mut kernel = PnixReplKernel::new(paths);

  let error = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("missing section should fail");

  assert!(
    error.to_string().contains("missing 'kernel-why-section'"),
    "{error:#}"
  );
}

#[test]
fn dialogue_template_parts_must_be_attrsets() {
  let dialogue_templates = temp_px_path("dialogue-templates.px");
  write_px(
    &dialogue_templates,
    r#"
    {
      kernel-definition-section = {
        join-with = ". ";
        suffix = ".";
        parts = [
          "not-an-attrset"
        ];
      };
      kernel-why-section = {
        join-with = " ";
        suffix = "";
        parts = [
          { when = "always"; template = "WHY ${regime}"; }
        ];
      };
      kernel-property-section = {
        join-with = "";
        suffix = "";
        parts = [
          { when = "always"; values-state = "present"; template = "PROP ${term} ${values}"; }
          { when = "always"; values-state = "empty"; template = "EMPTY ${term}"; }
        ];
      };
      kernel-route-summary = {
        definition = "def";
        property = "prop";
        why = "why";
        held = "held";
      };
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.dialogue_templates_path = dialogue_templates;
  let mut kernel = PnixReplKernel::new(paths);

  let error = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("non-attrset part should fail");

  assert!(
    error
      .to_string()
      .contains("invalid part in kernel-definition-section"),
    "{error:#}"
  );
}

#[test]
fn dialogue_template_parts_require_template() {
  let dialogue_templates = temp_px_path("dialogue-templates.px");
  write_px(
    &dialogue_templates,
    r#"
    {
      kernel-definition-section = {
        join-with = ". ";
        suffix = ".";
        parts = [
          { when = "always"; }
        ];
      };
      kernel-why-section = {
        join-with = " ";
        suffix = "";
        parts = [
          { when = "always"; template = "WHY ${regime}"; }
        ];
      };
      kernel-property-section = {
        join-with = "";
        suffix = "";
        parts = [
          { when = "always"; values-state = "present"; template = "PROP ${term} ${values}"; }
          { when = "always"; values-state = "empty"; template = "EMPTY ${term}"; }
        ];
      };
      kernel-route-summary = {
        definition = "def";
        property = "prop";
        why = "why";
        held = "held";
      };
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.dialogue_templates_path = dialogue_templates;
  let mut kernel = PnixReplKernel::new(paths);

  let error = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("missing template should fail");

  assert!(
    error
      .to_string()
      .contains("missing 'template' in kernel-definition-section"),
    "{error:#}"
  );
}

#[test]
fn dialogue_route_summary_fields_are_required() {
  let dialogue_templates = temp_px_path("dialogue-templates.px");
  write_px(
    &dialogue_templates,
    r#"
    {
      kernel-definition-section = {
        join-with = ". ";
        suffix = ".";
        parts = [
          { when = "always"; template = "BASE ${term}"; }
        ];
      };
      kernel-why-section = {
        join-with = " ";
        suffix = "";
        parts = [
          { when = "always"; template = "WHY ${regime}"; }
        ];
      };
      kernel-property-section = {
        join-with = "";
        suffix = "";
        parts = [
          { when = "always"; values-state = "present"; template = "PROP ${term} ${values}"; }
          { when = "always"; values-state = "empty"; template = "EMPTY ${term}"; }
        ];
      };
      kernel-route-summary = {
        definition = "def";
        property = "prop";
        why = "why";
      };
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.dialogue_templates_path = dialogue_templates;
  let mut kernel = PnixReplKernel::new(paths);

  let error = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("missing route summary should fail");

  assert!(
    error
      .to_string()
      .contains("missing 'held' in kernel-route-summary"),
    "{error:#}"
  );
}

#[test]
fn dialogue_template_scope_is_requires_known_values() {
  let dialogue_templates = temp_px_path("dialogue-templates.px");
  write_px(
    &dialogue_templates,
    r#"
    {
      kernel-definition-section = {
        join-with = ". ";
        suffix = ".";
        parts = [
          { when = "always"; template = "BASE ${term}"; }
          { when = "always"; scope-is = "bogus"; template = "DETAIL ${term}"; }
        ];
      };
      kernel-why-section = {
        join-with = " ";
        suffix = "";
        parts = [
          { when = "always"; template = "WHY ${regime}"; }
        ];
      };
      kernel-property-section = {
        join-with = "";
        suffix = "";
        parts = [
          { when = "always"; values-state = "present"; template = "PROP ${term} ${values}"; }
          { when = "always"; values-state = "empty"; template = "EMPTY ${term}"; }
        ];
      };
      kernel-route-summary = {
        definition = "def";
        property = "prop";
        why = "why";
        held = "held";
      };
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.dialogue_templates_path = dialogue_templates;
  let mut kernel = PnixReplKernel::new(paths);

  let error = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("invalid scope-is should fail");

  assert!(
    error
      .to_string()
      .contains("invalid 'scope-is' for kernel-definition-section part"),
    "{error:#}"
  );
}

#[test]
fn dialogue_template_values_state_requires_known_values() {
  let dialogue_templates = temp_px_path("dialogue-templates.px");
  write_px(
    &dialogue_templates,
    r#"
    {
      kernel-definition-section = {
        join-with = ". ";
        suffix = ".";
        parts = [
          { when = "always"; template = "BASE ${term}"; }
        ];
      };
      kernel-why-section = {
        join-with = " ";
        suffix = "";
        parts = [
          { when = "always"; template = "WHY ${regime}"; }
        ];
      };
      kernel-property-section = {
        join-with = "";
        suffix = "";
        parts = [
          { when = "always"; values-state = "bogus"; template = "PROP ${term} ${values}"; }
          { when = "always"; values-state = "empty"; template = "EMPTY ${term}"; }
        ];
      };
      kernel-route-summary = {
        definition = "def";
        property = "prop";
        why = "why";
        held = "held";
      };
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.dialogue_templates_path = dialogue_templates;
  let mut kernel = PnixReplKernel::new(paths);

  let error = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("invalid values-state should fail");

  assert!(
    error
      .to_string()
      .contains("invalid 'values-state' for kernel-property-section part"),
    "{error:#}"
  );
}

#[test]
fn dialogue_route_summary_fields_must_be_strings() {
  let dialogue_templates = temp_px_path("dialogue-templates.px");
  write_px(
    &dialogue_templates,
    r#"
    {
      kernel-definition-section = {
        join-with = ". ";
        suffix = ".";
        parts = [
          { when = "always"; template = "BASE ${term}"; }
        ];
      };
      kernel-why-section = {
        join-with = " ";
        suffix = "";
        parts = [
          { when = "always"; template = "WHY ${regime}"; }
        ];
      };
      kernel-property-section = {
        join-with = "";
        suffix = "";
        parts = [
          { when = "always"; values-state = "present"; template = "PROP ${term} ${values}"; }
          { when = "always"; values-state = "empty"; template = "EMPTY ${term}"; }
        ];
      };
      kernel-route-summary = {
        definition = ["def"];
        property = "prop";
        why = "why";
        held = "held";
      };
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.dialogue_templates_path = dialogue_templates;
  let mut kernel = PnixReplKernel::new(paths);

  let error = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("route summary wrong type should fail");

  assert!(
    error
      .to_string()
      .contains("'definition' in kernel-route-summary must be string"),
    "{error:#}"
  );
}

#[test]
fn dialogue_template_field_references_must_match_known_concept_fields() {
  let dialogue_templates = temp_px_path("dialogue-templates.px");
  write_px(
    &dialogue_templates,
    r#"
    {
      kernel-definition-section = {
        join-with = ". ";
        suffix = ".";
        parts = [
          { when = "always"; field-non-empty = "bogus-field"; template = "BASE ${term}"; }
        ];
      };
      kernel-why-section = {
        join-with = " ";
        suffix = "";
        parts = [
          { when = "always"; template = "WHY ${regime}"; }
        ];
      };
      kernel-property-section = {
        join-with = "";
        suffix = "";
        parts = [
          { when = "always"; values-state = "present"; template = "PROP ${term} ${values}"; }
          { when = "always"; values-state = "empty"; template = "EMPTY ${term}"; }
        ];
      };
      kernel-route-summary = {
        definition = "def";
        property = "prop";
        why = "why";
        held = "held";
      };
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.dialogue_templates_path = dialogue_templates;
  let mut kernel = PnixReplKernel::new(paths);

  let error = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("unknown dialogue field should fail");

  assert!(
    error
      .to_string()
      .contains("unknown concept scalar field 'bogus-field' in kernel-definition-section"),
    "{error:#}"
  );
}

#[test]
fn property_classifier_matching_is_owned_by_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["property" "definition" "why"];
      kernel-dispatch-routes = {
        definition = "custom-definition-route";
        property = "custom-property-route";
        held = "custom-held-route";
      };
      held-reason-keys = {
        requires-context = "ctx-needed";
        unknown-term = "unknown-concept";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐"]; }
      ];
      predicate-classifiers = [
        { match-any = ["식"]; predicate = "formula"; label-ko = "식"; }
      ];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  write_px(
    &query_routes,
    r#"
    [
      {
        route = "custom-property-route";
        query-context = "Pnix.Query.CustomProperty";
        include-hop-knowledge = "false";
        default-preview = "3";
        policy-coverage = "0.0";
        policy-coherence = "0.0";
        policy-loss = "0.0";
        policy-cost = "0.0";
        policy-accept-threshold = "0.0";
        kernel-interpretation-id = "custom.property.${predicate}.${term}";
      }
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  let mut kernel = PnixReplKernel::new(paths);

  let response = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 식"; }"#)
    .expect("evaluate");

  assert_eq!(response.route, "custom-property-route");
  assert!(response.response_document_org.contains("힘의 식: F = ma"));
  assert!(response
    .response_document_org
    .contains("custom.property.formula.힘"));
}

#[test]
fn concept_source_fact_rules_are_owned_by_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["property" "definition" "why"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "custom-property-route";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "formula"; predicate = "custom-formula"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐"]; }
      ];
      predicate-classifiers = [
        { match-any = ["식"]; predicate = "custom-formula"; label-ko = "식"; }
      ];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  write_px(
    &query_routes,
    r#"
    [
      {
        route = "custom-property-route";
        query-context = "Pnix.Query.CustomProperty";
        include-hop-knowledge = "false";
        default-preview = "3";
        policy-coverage = "0.0";
        policy-coherence = "0.0";
        policy-loss = "0.0";
        policy-cost = "0.0";
        policy-accept-threshold = "0.0";
        kernel-interpretation-id = "custom.source.${predicate}.${term}";
      }
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  let mut kernel = PnixReplKernel::new(paths);

  let response = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 식"; }"#)
    .expect("evaluate");

  assert_eq!(response.route, "custom-property-route");
  assert!(response.response_document_org.contains("힘의 식: F = ma"));
  assert!(response
    .response_document_org
    .contains("custom.source.custom-formula.힘"));
  let has_custom_fact = response.envelope.records.iter().any(|record| {
    let pnix_core::ontology::SemanticRecordValue::ContextualFact(fact) = &record.value else {
      return false;
    };
    fact.pred == "custom-formula"
  });
  assert!(has_custom_fact);
}

#[test]
fn invert_candidate_shaping_is_owned_by_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = ["뭐"];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["왜"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  write_px(
    &query_routes,
    r#"
    [
      {
        route = "custom-why-causal-inverse";
        query-context = "Pnix.Query.custom.why.causal.inverse";
        include-hop-knowledge = "false";
        default-preview = "3";
        policy-coverage = "0.0";
        policy-coherence = "0.0";
        policy-loss = "0.0";
        policy-cost = "0.0";
        policy-accept-threshold = "0.0";
      }
    ]
    "#,
  );
  let invert = temp_px_path("ontology-invert.px");
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "custom-why-${trigger_type}";
      default-truth-regime = "empirical-physical";
      default-interpretation-rule = {
        direct-fact-predicates = ["custom-why"];
        source-include-predicates = ["custom-why"];
        source-include-context-prefixes = ["ontology-invert.custom"];
        direct-interpretation-id = "custom.invert.default.direct.${trigger_type}.${term}";
        rich-interpretation-id = "custom.invert.default.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [
        { domain-prefix = "물리"; regime = "empirical-physical"; }
      ];
      invert-candidate-rules = [
        {
          type = "causal-inverse";
          concept-field = "why";
          predicate = "custom-why";
          context = "ontology-invert.custom";
        }
      ];
      interpretation-rules = [
        {
          type = "causal-inverse";
          direct-fact-predicates = ["custom-why"];
          source-include-predicates = ["custom-why"];
          source-include-context-prefixes = ["ontology-invert.custom"];
          direct-interpretation-id = "custom.invert.direct.${trigger_type}.${term}";
          rich-interpretation-id = "custom.invert.rich.${trigger_type}.${term}";
        }
      ];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);

  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect("evaluate");

  assert_eq!(response.route, "custom-why-causal-inverse");
  assert!(response
    .response_document_org
    .contains("custom.invert.direct.causal-inverse.힘"));
  assert!(!response
    .response_document_org
    .contains("interp.invert.direct.causal-inverse.힘"));
  let has_custom_fact = response.envelope.records.iter().any(|record| {
    let pnix_core::ontology::SemanticRecordValue::ContextualFact(fact) = &record.value else {
      return false;
    };
    fact.pred == "custom-why" && fact.context.0 == "ontology-invert.custom"
  });
  assert!(has_custom_fact);
}

#[test]
fn invert_candidate_rule_field_references_must_match_known_concept_fields() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  let invert = temp_px_path("ontology-invert.px");
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "custom-why-${trigger_type}";
      default-truth-regime = "empirical-physical";
      default-interpretation-rule = {
        direct-fact-predicates = ["custom-why"];
        source-include-predicates = ["custom-why"];
        source-include-context-prefixes = ["ontology-invert.custom"];
        direct-interpretation-id = "custom.invert.default.direct.${trigger_type}.${term}";
        rich-interpretation-id = "custom.invert.default.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [
        { domain-prefix = "물리"; regime = "empirical-physical"; }
      ];
      invert-candidate-rules = [
        {
          type = "causal-inverse";
          concept-field = "bogus-field";
          predicate = "custom-why";
          context = "ontology-invert.custom";
        }
      ];
      interpretation-rules = [
        {
          type = "causal-inverse";
          direct-fact-predicates = ["custom-why"];
          source-include-predicates = ["custom-why"];
          source-include-context-prefixes = ["ontology-invert.custom"];
          direct-interpretation-id = "custom.invert.direct.${trigger_type}.${term}";
          rich-interpretation-id = "custom.invert.rich.${trigger_type}.${term}";
        }
      ];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);

  let error = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("unknown invert concept field should fail");

  assert!(
    error
      .to_string()
      .contains("unknown concept scalar field 'bogus-field' in invert-candidate-rules"),
    "{error:#}"
  );
}

#[test]
fn invert_default_rule_and_truth_regime_are_owned_by_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = ["뭐"];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["왜"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  write_px(
    &query_routes,
    r#"
    [
      {
        route = "custom-why-causal-inverse";
        query-context = "Pnix.Query.custom.why.causal.inverse";
        include-hop-knowledge = "false";
        default-preview = "3";
        policy-coverage = "0.0";
        policy-coherence = "0.0";
        policy-loss = "0.0";
        policy-cost = "0.0";
        policy-accept-threshold = "0.0";
      }
    ]
    "#,
  );
  let invert = temp_px_path("ontology-invert.px");
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "custom-why-${trigger_type}";
      default-truth-regime = "speculative-theoretical";
      default-interpretation-rule = {
        direct-fact-predicates = ["custom-default-why"];
        source-include-predicates = ["custom-default-why"];
        source-include-context-prefixes = ["custom.default"];
        direct-interpretation-id = "custom.default.direct.${trigger_type}.${term}";
        rich-interpretation-id = "custom.default.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [
        {
          type = "causal-inverse";
          concept-field = "why";
          predicate = "custom-default-why";
          context = "custom.default";
        }
      ];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);

  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect("evaluate");

  assert_eq!(response.route, "custom-why-causal-inverse");
  assert!(response
    .response_document_org
    .contains("custom.default.direct.causal-inverse.힘"));
  assert!(
    response
      .envelope
      .notes
      .iter()
      .any(|note| note == "truth-regime:speculative-theoretical"),
    "{:?}",
    response.envelope.notes
  );
  let has_custom_fact = response.envelope.records.iter().any(|record| {
    let pnix_core::ontology::SemanticRecordValue::ContextualFact(fact) = &record.value else {
      return false;
    };
    fact.pred == "custom-default-why" && fact.context.0 == "custom.default"
  });
  assert!(has_custom_fact);
}

#[test]
fn ontology_invert_root_must_be_attrset() {
  let invert = temp_px_path("ontology-invert.px");
  write_px(
    &invert,
    r#"
    [
      "bogus"
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("ontology-invert root type should fail");

  assert!(
    err
      .to_string()
      .contains("ontology-invert root must be attrset"),
    "{err:#}"
  );
}

#[test]
fn invert_trigger_selection_must_be_string() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);

  let invert = temp_px_path("ontology-invert.px");
  write_px(
    &invert,
    r#"
    {
      trigger-selection = ["priority-then-pattern-length"];
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "speculative-theoretical";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["why"];
        source-include-context-prefixes = ["ontology-invert"];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "interpretive"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("trigger-selection wrong type should fail");

  assert!(
    err
      .to_string()
      .contains("'trigger-selection' must be string"),
    "{err:#}"
  );
}

#[test]
fn invert_trigger_selection_requires_known_values() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);

  let invert = temp_px_path("ontology-invert.px");
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "bogus";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "speculative-theoretical";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["why"];
        source-include-context-prefixes = ["ontology-invert"];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "interpretive"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("invalid trigger-selection should fail");

  assert!(
    err
      .to_string()
      .contains("invalid 'trigger-selection' for ontology-invert"),
    "{err:#}"
  );
}

#[test]
fn invert_route_template_must_be_string() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);

  let invert = temp_px_path("ontology-invert.px");
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = ["ontology-invert-${trigger_type}"];
      default-truth-regime = "speculative-theoretical";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["why"];
        source-include-context-prefixes = ["ontology-invert"];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "interpretive"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("route-template wrong type should fail");

  assert!(
    err.to_string().contains("'route-template' must be string"),
    "{err:#}"
  );
}

#[test]
fn invert_default_truth_regime_must_be_string() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);

  let invert = temp_px_path("ontology-invert.px");
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = ["speculative-theoretical"];
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["why"];
        source-include-context-prefixes = ["ontology-invert"];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "interpretive"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("default-truth-regime wrong type should fail");

  assert!(
    err
      .to_string()
      .contains("'default-truth-regime' must be string"),
    "{err:#}"
  );
}

#[test]
fn ontology_invert_triggers_must_be_list() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);

  let invert = temp_px_path("ontology-invert.px");
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "speculative-theoretical";
      default-interpretation-rule = {
        direct-fact-predicates = ["custom-default-why"];
        source-include-predicates = ["custom-default-why"];
        source-include-context-prefixes = ["custom.default"];
        direct-interpretation-id = "custom.default.direct.${trigger_type}.${term}";
        rich-interpretation-id = "custom.default.rich.${trigger_type}.${term}";
      };
      invert-triggers = "bogus";
      domain-to-regime = [];
      invert-candidate-rules = [];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("invert-triggers type should fail");

  assert!(
    err.to_string().contains("'invert-triggers' must be list"),
    "{err:#}"
  );
}

#[test]
fn term_extraction_particle_trim_and_fallback_are_owned_by_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["property" "definition" "why"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "custom-property-route";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      predicate-classifiers = [
        { match-any = ["식"]; predicate = "formula"; label-ko = "식"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐" "무엇" "뭔"]; }
        { match-any = ["이란" "란"]; }
        { match-any = ["설명" "알려" "에 대해" "에 관해" "에 대하여" "에 관하여"]; }
      ];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["genitive"];
      term-normalization-trim-chars = ["@"];
      term-fallback-policy = "disabled";
    }
    "#,
  );
  write_px(
    &query_routes,
    r#"
    [
      {
        route = "custom-property-route";
        query-context = "Pnix.Query.CustomProperty";
        include-hop-knowledge = "false";
        default-preview = "3";
        policy-coverage = "0.0";
        policy-coherence = "0.0";
        policy-loss = "0.0";
        policy-cost = "0.0";
        policy-accept-threshold = "0.0";
        kernel-interpretation-id = "custom.term.${predicate}.${term}";
      }
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  let mut kernel = PnixReplKernel::new(paths);

  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "@힘의 식"; }"#,
    )
    .expect("evaluate");

  assert_eq!(response.route, "custom-property-route");
  assert!(response.response_document_org.contains("힘의 식: F = ma"));
  assert!(response
    .response_document_org
    .contains("custom.term.formula.힘"));
}

#[test]
fn followup_reopen_rules_require_canonical_fields() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "ctx-needed";
        unknown-term = "unknown-concept";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      predicate-classifiers = [];
      definition-query-rules = [
        { match-any = ["뭐" "무엇" "뭔"]; }
        { match-any = ["이란" "란"]; }
      ];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
        { distinguishing-predicate = "unknown-term"; question-template = "UNKNOWN ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "ctx-needed"; predicate = "experimental-context"; }
        { reason = "unknown-concept"; predicate = "unknown-term"; }
      ];
      reopen-rules = [
        {
          reason = "ctx-needed";
          effective-utterance-template = "${term} ${utterance}";
        }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      default-choices = ["선택A"];
      held-response-rules = [
        { when = "term-present"; template = "HELD ${term}"; emit-held-term = "true"; }
        { when = "term-missing"; template = "HELD NONE"; emit-held-term = "false"; }
      ];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);

  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("missing carry-term-policy should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'carry-term-policy' in reopen-rules entry"),
    "{err:#}"
  );
}

#[test]
fn followup_reopen_rules_are_required() {
  let followups = temp_px_path("followup-generation.px");
  write_followup_fixture(
    &followups,
    "[]",
    r#"
      [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ]
    "#,
    r#"
      [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ]
    "#,
    r#"
      [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
        { when = "term-missing"; template = "WAIT NONE"; emit-held-term = "false"; }
      ]
    "#,
    "{}",
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let err = PnixReplKernel::new(paths)
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("empty reopen-rules should fail");

  assert!(
    err.to_string().contains("missing 'reopen-rules'"),
    "{err:#}"
  );
}

#[test]
fn followup_reason_question_rules_require_matching_reopen_rules() {
  let followups = temp_px_path("followup-generation.px");
  write_followup_fixture(
    &followups,
    r#"
      [
        {
          reason = "unknown-term";
          carry-term-policy = "never";
          effective-utterance-template = "${utterance}";
        }
      ]
    "#,
    r#"
      [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ]
    "#,
    r#"
      [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ]
    "#,
    r#"
      [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
        { when = "term-missing"; template = "WAIT NONE"; emit-held-term = "false"; }
      ]
    "#,
    "{}",
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let first = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect("first held query should succeed");

  assert_eq!(first.route, "lightweight-korean-dialogue-held");

  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "공식은 뭐야"; }"#,
    )
    .expect_err("missing reopen-rules reason should fail on reopen");

  assert!(
    err
      .to_string()
      .contains("missing 'reopen-rules' entry for reason 'requires-context'"),
    "{err:#}"
  );
}

#[test]
fn held_reason_keys_require_matching_followup_reason_rules() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "ctx-needed";
        unknown-term = "unknown-concept";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["설명"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );

  let followups = temp_px_path("followup-generation.px");
  write_followup_fixture(
    &followups,
    r#"
      [
        {
          reason = "requires-context";
          carry-term-policy = "never";
          effective-utterance-template = "${utterance}";
        }
        {
          reason = "unknown-term";
          carry-term-policy = "never";
          effective-utterance-template = "${utterance}";
        }
      ]
    "#,
    r#"
      [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ]
    "#,
    r#"
      [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ]
    "#,
    r#"
      [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
        { when = "term-missing"; template = "WAIT NONE"; emit-held-term = "false"; }
      ]
    "#,
    "{}",
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("missing held reason route should fail");

  assert!(
    err
      .to_string()
      .contains("missing follow-up reason route for 'ctx-needed'"),
    "{err:#}"
  );
}

#[test]
fn followup_reopen_rules_require_known_carry_term_policy_values() {
  let followups = temp_px_path("followup-generation.px");
  write_followup_fixture(
    &followups,
    r#"
      [
        {
          reason = "requires-context";
          carry-term-policy = "bogus";
          effective-utterance-template = "${utterance}";
        }
      ]
    "#,
    r#"
      [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ]
    "#,
    r#"
      [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ]
    "#,
    r#"
      [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
        { when = "term-missing"; template = "WAIT NONE"; emit-held-term = "false"; }
      ]
    "#,
    r#"
      {
        carry-term-policy = "never";
        effective-utterance-template = "${utterance}";
      }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("invalid carry-term-policy should fail");

  assert!(
    err
      .to_string()
      .contains("invalid 'carry-term-policy' for reopen-rules entry"),
    "{err:#}"
  );
}

#[test]
fn followup_reopen_rule_templates_must_be_strings() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let followups = temp_px_path("followup-generation.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"[
      { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
      { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
    ]"#,
    r#"[
      { field = "definition-ko"; predicate = "definition-ko"; }
    ]"#,
    r#"[
      { field = "related-concepts"; predicate = "related-concept"; }
    ]"#,
    r#"[
      { match-any = ["설명"]; }
    ]"#,
    r#"[]"#,
  );
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        {
          reason = "requires-context";
          carry-term-policy = "when-missing";
          effective-utterance-template = ["${term} ${utterance}"];
        }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      default-choices = ["선택A"];
      held-response-rules = [
        { when = "term-present"; template = "HELD ${term}"; emit-held-term = "true"; }
        { when = "term-missing"; template = "HELD NONE"; emit-held-term = "false"; }
      ];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);

  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("wrong effective-utterance-template type should fail");

  assert!(
    err
      .to_string()
      .contains("'effective-utterance-template' in reopen-rules entry must be string"),
    "{err:#}"
  );
}

#[test]
fn followup_choice_rules_require_known_when_values() {
  let followups = temp_px_path("followup-generation.px");
  write_followup_fixture(
    &followups,
    "[]",
    r#"
      [
        { when = "bogus"; choice-source = "default"; }
      ]
    "#,
    r#"
      [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ]
    "#,
    r#"
      [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
        { when = "term-missing"; template = "WAIT NONE"; emit-held-term = "false"; }
      ]
    "#,
    r#"
      {
        carry-term-policy = "never";
        effective-utterance-template = "${utterance}";
      }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("invalid choice-rules when should fail");

  assert!(
    err
      .to_string()
      .contains("invalid 'when' for choice-rules entry"),
    "{err:#}"
  );
}

#[test]
fn followup_choice_rules_require_known_choice_source_values() {
  let followups = temp_px_path("followup-generation.px");
  write_followup_fixture(
    &followups,
    "[]",
    r#"
      [
        { when = "term-present"; choice-source = "concept-or-default"; }
      ]
    "#,
    r#"
      [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ]
    "#,
    r#"
      [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
        { when = "term-missing"; template = "WAIT NONE"; emit-held-term = "false"; }
      ]
    "#,
    r#"
      {
        carry-term-policy = "never";
        effective-utterance-template = "${utterance}";
      }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("invalid choice-source should fail");

  assert!(
    err
      .to_string()
      .contains("invalid 'choice-source' for choice-rules entry"),
    "{err:#}"
  );
}

#[test]
fn followup_resolved_term_rules_require_known_when_values() {
  let followups = temp_px_path("followup-generation.px");
  write_followup_fixture(
    &followups,
    "[]",
    r#"
      [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ]
    "#,
    r#"
      [
        { when = "bogus"; term-source = "term"; }
      ]
    "#,
    r#"
      [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
        { when = "term-missing"; template = "WAIT NONE"; emit-held-term = "false"; }
      ]
    "#,
    r#"
      {
        carry-term-policy = "never";
        effective-utterance-template = "${utterance}";
      }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("invalid resolved-term when should fail");

  assert!(
    err
      .to_string()
      .contains("invalid 'when' for resolved-term-rules entry"),
    "{err:#}"
  );
}

#[test]
fn followup_resolved_term_rules_require_known_term_source_values() {
  let followups = temp_px_path("followup-generation.px");
  write_followup_fixture(
    &followups,
    "[]",
    r#"
      [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ]
    "#,
    r#"
      [
        { when = "term-present"; term-source = "bogus"; }
      ]
    "#,
    r#"
      [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
        { when = "term-missing"; template = "WAIT NONE"; emit-held-term = "false"; }
      ]
    "#,
    r#"
      {
        carry-term-policy = "never";
        effective-utterance-template = "${utterance}";
      }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("invalid term-source should fail");

  assert!(
    err
      .to_string()
      .contains("invalid 'term-source' for resolved-term-rules entry"),
    "{err:#}"
  );
}

#[test]
fn followup_literal_resolved_term_rules_require_value() {
  let followups = temp_px_path("followup-generation.px");
  write_followup_fixture(
    &followups,
    "[]",
    r#"
      [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ]
    "#,
    r#"
      [
        { when = "term-missing"; term-source = "literal"; }
      ]
    "#,
    r#"
      [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
        { when = "term-missing"; template = "WAIT NONE"; emit-held-term = "false"; }
      ]
    "#,
    r#"
      {
        carry-term-policy = "never";
        effective-utterance-template = "${utterance}";
      }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("literal resolved-term-rules without value should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'value' for literal resolved-term-rules entry"),
    "{err:#}"
  );
}

#[test]
fn followup_held_response_rules_require_known_when_values() {
  let followups = temp_px_path("followup-generation.px");
  write_followup_fixture(
    &followups,
    "[]",
    r#"
      [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ]
    "#,
    r#"
      [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ]
    "#,
    r#"
      [
        { when = "bogus"; template = "WAIT ${term}"; emit-held-term = "false"; }
      ]
    "#,
    r#"
      {
        carry-term-policy = "never";
        effective-utterance-template = "${utterance}";
      }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("invalid held-response when should fail");

  assert!(
    err
      .to_string()
      .contains("invalid 'when' for held-response-rules entry"),
    "{err:#}"
  );
}

#[test]
fn followup_held_response_rules_are_required() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "ctx-needed";
        unknown-term = "unknown-concept";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      predicate-classifiers = [];
      definition-query-rules = [
        { match-any = ["뭐" "무엇" "뭔"]; }
        { match-any = ["이란" "란"]; }
      ];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
        { distinguishing-predicate = "unknown-term"; question-template = "UNKNOWN ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "ctx-needed"; predicate = "experimental-context"; }
        { reason = "unknown-concept"; predicate = "unknown-term"; }
      ];
      reopen-rules = [
        {
          reason = "ctx-needed";
          carry-term-policy = "when-missing";
          effective-utterance-template = "${term} ${utterance}";
        }
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      concept-choices = [];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);

  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("missing held-response-rules should fail");

  assert!(
    err.to_string().contains("missing 'held-response-rules'"),
    "{err:#}"
  );
}

#[test]
fn followup_default_choices_are_required() {
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      held-response-rules = [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
      ];
      concept-choices = [];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;

  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("missing default-choices should fail");

  assert!(
    err.to_string().contains("missing 'default-choices'"),
    "{err:#}"
  );
}

#[test]
fn followup_disambiguation_questions_are_required() {
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      reason-question-rules = [
        { reason = "requires-context"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      held-response-rules = [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
      ];
      concept-choices = [];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("missing disambiguation-questions should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'disambiguation-questions'"),
    "{err:#}"
  );
}

#[test]
fn followup_reason_question_rules_are_required() {
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      held-response-rules = [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
      ];
      concept-choices = [];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("missing reason-question-rules should fail");

  assert!(
    err.to_string().contains("missing 'reason-question-rules'"),
    "{err:#}"
  );
}

#[test]
fn followup_choice_rules_are_required() {
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      held-response-rules = [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
      ];
      concept-choices = [];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("missing choice-rules should fail");

  assert!(
    err.to_string().contains("missing 'choice-rules'"),
    "{err:#}"
  );
}

#[test]
fn followup_resolved_term_rules_are_required() {
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      held-response-rules = [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
      ];
      concept-choices = [];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("missing resolved-term-rules should fail");

  assert!(
    err.to_string().contains("missing 'resolved-term-rules'"),
    "{err:#}"
  );
}

#[test]
fn followup_unknown_term_label_is_required() {
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      held-response-rules = [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
      ];
      concept-choices = [];
      default-choices = ["선택A"];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;

  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("missing unknown-term-label should fail");

  assert!(
    err.to_string().contains("missing 'unknown-term-label'"),
    "{err:#}"
  );
}

#[test]
fn followup_choice_rules_must_match_term_state() {
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
      ];
      held-response-rules = [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
      ];
      concept-choices = [];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("missing matching choice rule should fail");

  assert!(
    err.to_string().contains(
      "no follow-up choice rule matched term state 'term-present-without-concept-choice'"
    ),
    "{err:#}"
  );
}

#[test]
fn followup_resolved_term_rules_must_match_term_state() {
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      held-response-rules = [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
      ];
      concept-choices = [];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("missing matching resolved-term rule should fail");

  assert!(
    err
      .to_string()
      .contains("no follow-up resolved-term rule matched term state 'term-present'"),
    "{err:#}"
  );
}

#[test]
fn held_response_rules_must_match_term_state() {
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
      ];
      held-response-rules = [
        { when = "term-missing"; template = "WAIT NONE"; emit-held-term = "false"; }
      ];
      concept-choices = [];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("missing matching held-response rule should fail");

  assert!(
    err
      .to_string()
      .contains("no held-response rule matched term state 'term-present'"),
    "{err:#}"
  );
}

#[test]
fn followup_reopen_seed_policy_is_owned_by_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["property" "definition" "why"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "ctx-needed";
        unknown-term = "unknown-concept";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐" "무엇" "뭔"]; }
        { match-any = ["이란" "란"]; }
        { match-any = ["설명" "알려" "에 대해" "에 관해" "에 대하여" "에 관하여"]; }
      ];
      predicate-classifiers = [
        { match-any = ["공식"]; predicate = "formula"; label-ko = "공식"; }
      ];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = " :: ${suggestions}"; }
      ];
      reason-question-rules = [
        { reason = "ctx-needed"; predicate = "experimental-context"; }
        { reason = "unknown-concept"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        {
          reason = "ctx-needed";
          carry-term-policy = "never";
          effective-utterance-template = "${utterance}";
        }
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      default-choices = ["뉴턴 역학에서?"];
      held-response-rules = [
        { when = "term-present"; template = "HELD ${term}"; emit-held-term = "true"; }
        { when = "term-missing"; template = "HELD NONE"; emit-held-term = "false"; }
      ];
      concept-choices = [];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);

  let first = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect("first evaluate");
  assert_eq!(first.route, "lightweight-korean-dialogue-held");

  let second = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "공식은 뭐야"; }"#,
    )
    .expect("second evaluate");

  assert_eq!(second.route, "lightweight-korean-dialogue-held");
  assert!(
    !second.response_document_org.contains("힘의 공식"),
    "{}",
    second.response_document_org
  );
}

#[test]
fn followup_reason_question_rules_route_is_owned_by_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["property" "definition" "why"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "ctx-needed";
        unknown-term = "unknown-concept";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐" "무엇" "뭔"]; }
        { match-any = ["이란" "란"]; }
        { match-any = ["설명" "알려" "에 대해" "에 관해" "에 대하여" "에 관하여"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "fallback-custom"; question-template = "FALLBACK ${term}"; choices-template = " :: ${suggestions}"; }
      ];
      reason-question-rules = [
        { reason = "ctx-needed"; predicate = "fallback-custom"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      default-choices = ["선택A" "선택B"];
      held-response-rules = [
        { when = "term-present"; template = "HELD ${term}"; emit-held-term = "true"; }
        { when = "term-missing"; template = "HELD NONE"; emit-held-term = "false"; }
      ];
      concept-choices = [];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);

  let response = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect("evaluate");

  assert_eq!(response.route, "lightweight-korean-dialogue-held");
  assert_eq!(
    response.follow_up_hint.as_deref(),
    Some("FALLBACK 힘 :: 선택A, 선택B")
  );
}

#[test]
fn followup_missing_reason_route_fails_without_fallback() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["property" "definition" "why"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "ctx-needed";
        unknown-term = "unknown-concept";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐" "무엇" "뭔"]; }
        { match-any = ["이란" "란"]; }
        { match-any = ["설명" "알려" "에 대해" "에 관해" "에 대하여" "에 관하여"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "fallback-custom"; question-template = "FALLBACK ${term}"; choices-template = " :: ${suggestions}"; }
      ];
      reason-question-rules = [
        { reason = "unknown-concept"; predicate = "fallback-custom"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      default-choices = ["선택A" "선택B"];
      held-response-rules = [
        { when = "term-present"; template = "HELD ${term}"; emit-held-term = "true"; }
        { when = "term-missing"; template = "HELD NONE"; emit-held-term = "false"; }
      ];
      concept-choices = [];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);

  let error = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("missing reason route should fail");

  assert!(
    error
      .to_string()
      .contains("missing follow-up reason route for 'ctx-needed'"),
    "{error:#}"
  );
}

#[test]
fn invert_default_interpretation_rule_must_be_attrset() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);

  let invert = temp_px_path("ontology-invert.px");
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "speculative-theoretical";
      default-interpretation-rule = "bogus";
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);

  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("wrong default-interpretation-rule type should fail");

  assert!(
    err
      .to_string()
      .contains("'default-interpretation-rule' must be attrset"),
    "{err:#}"
  );
}

#[test]
fn invert_default_interpretation_rule_fields_require_canonical_types() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);

  let invert = temp_px_path("ontology-invert.px");
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "speculative-theoretical";
      default-interpretation-rule = {
        direct-fact-predicates = "why";
        source-include-predicates = ["why"];
        source-include-context-prefixes = ["ontology-invert"];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);

  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("wrong default interpretation nested type should fail");

  assert!(
    err
      .to_string()
      .contains("'default-interpretation-rule.direct-fact-predicates' must be list"),
    "{err:#}"
  );
}

#[test]
fn invert_default_interpretation_rule_requires_canonical_fields() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);

  let invert = temp_px_path("ontology-invert.px");
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "speculative-theoretical";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["why"];
        source-include-context-prefixes = ["ontology-invert"];
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);

  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("missing default interpretation field should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'default-interpretation-rule.direct-interpretation-id'"),
    "{err:#}"
  );
}

#[test]
fn invert_default_interpretation_rule_is_required() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);

  let invert = temp_px_path("ontology-invert.px");
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "speculative-theoretical";
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);

  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("missing default-interpretation-rule should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'default-interpretation-rule'"),
    "{err:#}"
  );
}

#[test]
fn invert_trigger_priority_is_owned_by_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐" "무엇" "뭔"]; }
        { match-any = ["이란" "란"]; }
        { match-any = ["설명" "알려" "에 대해" "에 관해" "에 대하여" "에 관하여"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["왜"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  write_px(
    &query_routes,
    r#"
    [
      {
        route = "custom-why-causal-inverse";
        query-context = "Pnix.Query.custom.why.causal.inverse";
        include-hop-knowledge = "false";
        default-preview = "3";
        policy-coverage = "0.0";
        policy-coherence = "0.0";
        policy-loss = "0.0";
        policy-cost = "0.0";
        policy-accept-threshold = "0.0";
      }
      {
        route = "custom-why-counterfactual";
        query-context = "Pnix.Query.custom.why.counterfactual";
        include-hop-knowledge = "false";
        default-preview = "3";
        policy-coverage = "0.0";
        policy-coherence = "0.0";
        policy-loss = "0.0";
        policy-cost = "0.0";
        policy-accept-threshold = "0.0";
      }
    ]
    "#,
  );
  let invert = temp_px_path("ontology-invert.px");
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "custom-why-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["custom-counterfactual"];
        source-include-predicates = ["custom-counterfactual"];
        source-include-context-prefixes = ["ontology-invert.custom"];
        direct-interpretation-id = "custom.default.direct.${trigger_type}.${term}";
        rich-interpretation-id = "custom.default.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
        { pattern = "없다면"; type = "counterfactual"; truth-regime = "auto"; priority = "10"; }
      ];
      domain-to-regime = [
        { domain-prefix = "물리"; regime = "empirical-physical"; }
      ];
      invert-candidate-rules = [
        {
          type = "causal-inverse";
          concept-field = "why";
          predicate = "custom-why";
          context = "ontology-invert.custom";
        }
        {
          type = "counterfactual";
          predicate = "custom-counterfactual";
          context = "ontology-invert.custom";
          obj-template = "CF ${term}";
        }
      ];
      interpretation-rules = [
        {
          type = "counterfactual";
          direct-fact-predicates = ["custom-counterfactual"];
          source-include-predicates = ["custom-counterfactual"];
          source-include-context-prefixes = ["ontology-invert.custom"];
          direct-interpretation-id = "custom.counterfactual.direct.${term}";
          rich-interpretation-id = "custom.counterfactual.rich.${term}";
        }
      ];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);

  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘이 없다면"; }"#,
    )
    .expect("evaluate");

  assert_eq!(response.route, "custom-why-counterfactual");
  assert!(
    response
      .response_document_org
      .contains("custom.counterfactual.direct.힘"),
    "{}",
    response.response_document_org
  );
}

#[test]
fn invert_trigger_truth_regime_is_required_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("missing truth-regime should fail");
  assert!(
    err
      .to_string()
      .contains("missing 'truth-regime' in invert-triggers entry"),
    "{err:#}"
  );
}

#[test]
fn invert_trigger_priority_is_required_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("missing priority should fail");
  assert!(
    err
      .to_string()
      .contains("missing 'priority' in invert-triggers entry"),
    "{err:#}"
  );
}

#[test]
fn invert_trigger_pattern_and_type_are_required_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("missing trigger pattern should fail");
  assert!(
    err
      .to_string()
      .contains("missing 'pattern' in invert-triggers entry"),
    "{err:#}"
  );
}

#[test]
fn invert_triggers_reject_non_attrset_entries_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = ["bogus"];
      domain-to-regime = [];
      invert-candidate-rules = [];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("non-attrset invert-triggers should fail");
  assert!(
    err.to_string().contains("invalid 'invert-triggers' entry"),
    "{err:#}"
  );
}

#[test]
fn invert_triggers_are_required() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      domain-to-regime = [];
      invert-candidate-rules = [];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("missing invert-triggers should fail");
  assert!(
    err.to_string().contains("missing 'invert-triggers'"),
    "{err:#}"
  );
}

#[test]
fn invert_domain_to_regime_entries_require_canonical_fields() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [
        { domain-prefix = "물리"; }
      ];
      invert-candidate-rules = [];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("invalid domain-to-regime entry should fail");
  assert!(
    err
      .to_string()
      .contains("missing 'regime' in domain-to-regime entry"),
    "{err:#}"
  );
}

#[test]
fn invert_domain_to_regime_wildcard_must_match_default_truth_regime() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [
        { domain-prefix = "*"; regime = "formal-closed"; }
      ];
      invert-candidate-rules = [];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("wildcard mismatch should fail");
  assert!(
    err
      .to_string()
      .contains("'domain-to-regime' wildcard regime must match 'default-truth-regime'"),
    "{err:#}"
  );
}

#[test]
fn invert_domain_to_regime_reject_non_attrset_entries_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = ["bogus"];
      invert-candidate-rules = [];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("non-attrset domain-to-regime should fail");
  assert!(
    err.to_string().contains("invalid 'domain-to-regime' entry"),
    "{err:#}"
  );
}

#[test]
fn invert_domain_to_regime_is_required() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      invert-candidate-rules = [];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("missing domain-to-regime should fail");
  assert!(
    err.to_string().contains("missing 'domain-to-regime'"),
    "{err:#}"
  );
}

#[test]
fn invert_candidate_rules_require_canonical_fields_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [
        {
          type = "causal-inverse";
          predicate = "causal-chain";
          context = "ontology-invert.causal";
        }
      ];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("malformed invert candidate rule should fail");
  assert!(
    err.to_string().contains(
      "missing 'concept-field' or 'obj-template' for invert candidate rule 'causal-inverse'"
    ),
    "{err:#}"
  );
}

#[test]
fn invert_candidate_rules_reject_non_attrset_entries_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = ["bogus"];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("non-attrset invert-candidate-rules should fail");
  assert!(
    err
      .to_string()
      .contains("invalid 'invert-candidate-rules' entry"),
    "{err:#}"
  );
}

#[test]
fn invert_candidate_rules_are_required() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("missing invert-candidate-rules should fail");
  assert!(
    err.to_string().contains("missing 'invert-candidate-rules'"),
    "{err:#}"
  );
}

#[test]
fn invert_candidate_rules_require_type_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [
        {
          predicate = "causal-chain";
          context = "ontology-invert.causal";
          concept-field = "why";
        }
      ];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("missing candidate type should fail");
  assert!(
    err
      .to_string()
      .contains("missing 'type' in invert-candidate-rules entry"),
    "{err:#}"
  );
}

#[test]
fn invert_interpretation_rules_reject_non_attrset_entries_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [
        {
          type = "causal-inverse";
          concept-field = "why";
          predicate = "causal-chain";
          context = "ontology-invert.causal";
        }
      ];
      interpretation-rules = ["bogus"];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("non-attrset interpretation-rules should fail");
  assert!(
    err
      .to_string()
      .contains("invalid 'interpretation-rules' entry"),
    "{err:#}"
  );
}

#[test]
fn invert_interpretation_rules_require_canonical_fields_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [
        {
          type = "causal-inverse";
          concept-field = "why";
          predicate = "causal-chain";
          context = "ontology-invert.causal";
        }
      ];
      interpretation-rules = [
        {
          type = "causal-inverse";
          source-include-predicates = ["causal-chain"];
          source-include-context-prefixes = ["ontology-invert.causal"];
          direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
          rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
        }
      ];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("malformed invert interpretation rule should fail");
  assert!(
    err
      .to_string()
      .contains("missing 'direct-fact-predicates' for invert interpretation rule 'causal-inverse'"),
    "{err:#}"
  );
}

#[test]
fn invert_interpretation_rules_require_canonical_field_types_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [
        {
          type = "causal-inverse";
          concept-field = "why";
          predicate = "causal-chain";
          context = "ontology-invert.causal";
        }
      ];
      interpretation-rules = [
        {
          type = "causal-inverse";
          direct-fact-predicates = ["causal-chain"];
          source-include-predicates = ["causal-chain"];
          source-include-context-prefixes = ["ontology-invert.causal"];
          direct-interpretation-id = ["interp.invert.direct.${trigger_type}.${term}"];
          rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
        }
      ];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("wrong interpretation rule nested type should fail");
  assert!(
    err.to_string().contains(
      "'direct-interpretation-id' for invert interpretation rule 'causal-inverse' must be string"
    ),
    "{err:#}"
  );
}

#[test]
fn invert_interpretation_rules_direct_fact_predicates_must_be_non_empty_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [
        {
          type = "causal-inverse";
          concept-field = "why";
          predicate = "causal-chain";
          context = "ontology-invert.causal";
        }
      ];
      interpretation-rules = [
        {
          type = "causal-inverse";
          direct-fact-predicates = [];
          source-include-predicates = ["causal-chain"];
          source-include-context-prefixes = ["ontology-invert.causal"];
          direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
          rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
        }
      ];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("empty direct-fact-predicates should fail");
  assert!(
    err
      .to_string()
      .contains("empty 'direct-fact-predicates' for invert interpretation rule 'causal-inverse'"),
    "{err:#}"
  );
}

#[test]
fn invert_interpretation_rules_source_include_predicates_must_be_non_empty_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [
        {
          type = "causal-inverse";
          concept-field = "why";
          predicate = "causal-chain";
          context = "ontology-invert.causal";
        }
      ];
      interpretation-rules = [
        {
          type = "causal-inverse";
          direct-fact-predicates = ["causal-chain"];
          source-include-predicates = [];
          source-include-context-prefixes = ["ontology-invert.causal"];
          direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
          rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
        }
      ];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("empty source-include-predicates should fail");
  assert!(
    err.to_string().contains(
      "empty 'source-include-predicates' for invert interpretation rule 'causal-inverse'"
    ),
    "{err:#}"
  );
}

#[test]
fn invert_interpretation_rules_source_include_context_prefixes_must_be_non_empty_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [
        {
          type = "causal-inverse";
          concept-field = "why";
          predicate = "causal-chain";
          context = "ontology-invert.causal";
        }
      ];
      interpretation-rules = [
        {
          type = "causal-inverse";
          direct-fact-predicates = ["causal-chain"];
          source-include-predicates = ["causal-chain"];
          source-include-context-prefixes = [];
          direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
          rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
        }
      ];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("empty source-include-context-prefixes should fail");
  assert!(
    err.to_string().contains(
      "empty 'source-include-context-prefixes' for invert interpretation rule 'causal-inverse'"
    ),
    "{err:#}"
  );
}

#[test]
fn invert_interpretation_rules_require_all_canonical_fields_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [];
      interpretation-rules = [
        {
          type = "causal-inverse";
          direct-fact-predicates = ["why"];
          source-include-predicates = ["causal-chain"];
          source-include-context-prefixes = ["ontology-invert."];
          direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        }
      ];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);

  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("missing invert interpretation field should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'rich-interpretation-id' for invert interpretation rule 'causal-inverse'"),
    "{err:#}"
  );
}

#[test]
fn invert_interpretation_rules_require_type_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [
        {
          type = "causal-inverse";
          concept-field = "why";
          predicate = "causal-chain";
          context = "ontology-invert.causal";
        }
      ];
      interpretation-rules = [
        {
          direct-fact-predicates = ["causal-chain"];
          source-include-predicates = ["causal-chain"];
          source-include-context-prefixes = ["ontology-invert.causal"];
          direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
          rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
        }
      ];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("missing interpretation type should fail");
  assert!(
    err
      .to_string()
      .contains("missing 'type' in interpretation-rules entry"),
    "{err:#}"
  );
}

#[test]
fn invert_interpretation_rules_are_required() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("missing interpretation-rules should fail");
  assert!(
    err.to_string().contains("missing 'interpretation-rules'"),
    "{err:#}"
  );
}

#[test]
fn invert_interpretation_rules_reject_unknown_trigger_types() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [
        {
          type = "causal-inverse";
          concept-field = "why";
          predicate = "causal-chain";
          context = "ontology-invert.causal";
        }
      ];
      interpretation-rules = [
        {
          type = "bogus";
          direct-fact-predicates = ["causal-chain"];
          source-include-predicates = ["causal-chain"];
          source-include-context-prefixes = ["ontology-invert.causal"];
          direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
          rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
        }
      ];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("unknown interpretation rule type should fail");

  assert!(
    err
      .to_string()
      .contains("unknown interpretation rule type 'bogus'"),
    "{err:#}"
  );
}

#[test]
fn query_route_defaults_and_context_rewrite_are_owned_by_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let query_route_defaults = temp_px_path("query-route-defaults.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["property" "definition" "why"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "custom-property-route";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      predicate-classifiers = [
        { match-any = ["식"]; predicate = "formula"; label-ko = "식"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐" "무엇" "뭔"]; }
        { match-any = ["이란" "란"]; }
        { match-any = ["설명" "알려" "에 대해" "에 관해" "에 대하여" "에 관하여"]; }
      ];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  write_px(
    &query_routes,
    r#"
    [
      {
        route = "custom-property-route";
        query-context = "Doghouse.Custom.custom-property-route";
        include-hop-knowledge = "false";
        default-preview = "4";
        policy-coverage = "0.0";
        policy-coherence = "0.0";
        policy-loss = "0.0";
        policy-cost = "0.0";
        policy-accept-threshold = "0.0";
        kernel-interpretation-id = "custom.defaults.${predicate}.${term}";
      }
    ]
    "#,
  );
  write_px(
    &query_route_defaults,
    r#"
    {
      query-context-rewrite-rules = [
        { from = "Doghouse."; to = "Pnix."; }
      ];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.query_route_defaults_path = query_route_defaults;
  let mut kernel = PnixReplKernel::new(paths);

  let response = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 식"; }"#)
    .expect("evaluate");

  assert_eq!(response.route, "custom-property-route");
  assert!(response
    .response_document_org
    .contains("custom.defaults.formula.힘"));
  let has_rewritten_context = response.envelope.records.iter().any(|record| {
    let pnix_core::ontology::SemanticRecordValue::ContextualFact(fact) = &record.value else {
      return false;
    };
    fact.pred == "ontology-query-context" && fact.obj == "Pnix.Custom.custom-property-route"
  });
  assert!(has_rewritten_context, "{:?}", response.envelope.records);
}

#[test]
fn query_route_defaults_root_must_be_attrset() {
  let query_route_defaults = temp_px_path("query-route-defaults.px");
  write_px(
    &query_route_defaults,
    r#"
    [
      "bogus"
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_route_defaults_path = query_route_defaults;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("query-route-defaults root type should fail");

  assert!(
    err
      .to_string()
      .contains("query-route-defaults root must be attrset"),
    "{err:#}"
  );
}

#[test]
fn query_route_defaults_rewrite_rules_must_be_list() {
  let query_route_defaults = temp_px_path("query-route-defaults.px");
  write_px(
    &query_route_defaults,
    r#"
    {
      query-context-rewrite-rules = "bogus";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_route_defaults_path = query_route_defaults;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("rewrite rule type should fail");

  assert!(
    err
      .to_string()
      .contains("'query-context-rewrite-rules' must be list"),
    "{err:#}"
  );
}

#[test]
fn query_route_defaults_rewrite_rules_are_required() {
  let query_route_defaults = temp_px_path("query-route-defaults.px");
  write_px(
    &query_route_defaults,
    r#"
    {
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_route_defaults_path = query_route_defaults;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("missing query-context-rewrite-rules should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'query-context-rewrite-rules'"),
    "{err:#}"
  );
}

#[test]
fn query_classifiers_held_reason_rules_are_required() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("missing held-reason-rules should fail");

  assert!(
    err.to_string().contains("missing 'held-reason-rules'"),
    "{err:#}"
  );
}

#[test]
fn query_classifiers_source_fact_rules_are_required() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("missing kernel-source-fact-fields should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'kernel-source-fact-fields'"),
    "{err:#}"
  );
}

#[test]
fn query_classifiers_source_list_rules_are_required() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("missing kernel-source-list-fields should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'kernel-source-list-fields'"),
    "{err:#}"
  );
}

#[test]
fn query_classifiers_definition_query_rules_are_required() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("missing definition-query-rules should fail");

  assert!(
    err.to_string().contains("missing 'definition-query-rules'"),
    "{err:#}"
  );
}

#[test]
fn held_reason_rules_require_canonical_fields_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"
      [
        { when = "known-term"; reason-key = "requires-context"; }
      ]
    "#,
    r#"
      [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ]
    "#,
    r#"
      [
        { field = "related-concepts"; predicate = "related-concept"; }
      ]
    "#,
    r#"
      [
        { match-any = ["뭐"]; }
      ]
    "#,
    r#"[]"#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("invalid held-reason-rules should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'term-source' in held-reason-rules entry"),
    "{err:#}"
  );
}

#[test]
fn held_reason_rules_reject_non_attrset_entries_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"[ "bogus" ]"#,
    r#"
      [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ]
    "#,
    r#"
      [
        { field = "related-concepts"; predicate = "related-concept"; }
      ]
    "#,
    r#"
      [
        { match-any = ["뭐"]; }
      ]
    "#,
    r#"[]"#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("non-attrset held-reason-rules should fail");

  assert!(
    err
      .to_string()
      .contains("invalid 'held-reason-rules' entry"),
    "{err:#}"
  );
}

#[test]
fn source_lowering_rules_require_canonical_fields_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"
      [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ]
    "#,
    r#"
      [
        { field = "definition-ko"; }
      ]
    "#,
    r#"
      [
        { field = "related-concepts"; predicate = "related-concept"; }
      ]
    "#,
    r#"
      [
        { match-any = ["뭐"]; }
      ]
    "#,
    r#"[]"#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("invalid source fact fields should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'predicate' in kernel-source-fact-fields entry"),
    "{err:#}"
  );
}

#[test]
fn source_lowering_rules_reject_non_attrset_entries_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"
      [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ]
    "#,
    r#"[ "bogus" ]"#,
    r#"
      [
        { field = "related-concepts"; predicate = "related-concept"; }
      ]
    "#,
    r#"
      [
        { match-any = ["뭐"]; }
      ]
    "#,
    r#"[]"#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("non-attrset source fact rule should fail");

  assert!(
    err
      .to_string()
      .contains("invalid 'kernel-source-fact-fields' entry"),
    "{err:#}"
  );
}

#[test]
fn source_list_rules_require_canonical_fields_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"
      [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ]
    "#,
    r#"
      [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ]
    "#,
    r#"
      [
        { field = "related-concepts"; }
      ]
    "#,
    r#"
      [
        { match-any = ["뭐"]; }
      ]
    "#,
    r#"[]"#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("invalid source list fields should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'predicate' in kernel-source-list-fields entry"),
    "{err:#}"
  );
}

#[test]
fn source_list_rules_reject_non_attrset_entries_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"
      [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ]
    "#,
    r#"
      [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ]
    "#,
    r#"[ "bogus" ]"#,
    r#"
      [
        { match-any = ["뭐"]; }
      ]
    "#,
    r#"[]"#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("non-attrset source list rule should fail");

  assert!(
    err
      .to_string()
      .contains("invalid 'kernel-source-list-fields' entry"),
    "{err:#}"
  );
}

#[test]
fn definition_query_rules_require_canonical_matchers_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"
      [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ]
    "#,
    r#"
      [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ]
    "#,
    r#"
      [
        { field = "related-concepts"; predicate = "related-concept"; }
      ]
    "#,
    r#"
      [
        { }
      ]
    "#,
    r#"[]"#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("invalid definition-query-rules should fail");

  assert!(
    err
      .to_string()
      .contains("invalid 'definition-query-rules' entry"),
    "{err:#}"
  );
}

#[test]
fn definition_query_rules_matchers_must_be_lists_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"
      [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ]
    "#,
    r#"
      [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ]
    "#,
    r#"
      [
        { field = "related-concepts"; predicate = "related-concept"; }
      ]
    "#,
    r#"
      [
        { match-any = "뭐"; }
      ]
    "#,
    r#"[]"#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("wrong type in definition-query-rules matcher should fail");

  assert!(
    err
      .to_string()
      .contains("'match-any' in definition-query-rules entry must be list"),
    "{err:#}"
  );
}

#[test]
fn definition_query_rules_matchers_must_be_non_empty_lists_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"
      [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ]
    "#,
    r#"
      [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ]
    "#,
    r#"
      [
        { field = "related-concepts"; predicate = "related-concept"; }
      ]
    "#,
    r#"
      [
        { match-any = []; }
      ]
    "#,
    r#"[]"#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("empty definition-query-rules matcher should fail");

  assert!(
    err
      .to_string()
      .contains("empty matcher list in 'definition-query-rules' entry"),
    "{err:#}"
  );
}

#[test]
fn definition_query_rules_reject_non_attrset_entries_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"
      [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ]
    "#,
    r#"
      [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ]
    "#,
    r#"
      [
        { field = "related-concepts"; predicate = "related-concept"; }
      ]
    "#,
    r#"[ "bogus" ]"#,
    r#"[]"#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("non-attrset definition-query-rules should fail");

  assert!(
    err
      .to_string()
      .contains("invalid 'definition-query-rules' entry"),
    "{err:#}"
  );
}

#[test]
fn predicate_classifiers_require_canonical_fields_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"
      [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ]
    "#,
    r#"
      [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ]
    "#,
    r#"
      [
        { field = "related-concepts"; predicate = "related-concept"; }
      ]
    "#,
    r#"
      [
        { match-any = ["뭐"]; }
      ]
    "#,
    r#"
      [
        { predicate = "formula"; label-ko = "공식"; }
      ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("invalid predicate-classifiers should fail");

  assert!(
    err
      .to_string()
      .contains("invalid 'predicate-classifiers' entry"),
    "{err:#}"
  );
}

#[test]
fn predicate_classifier_fields_require_canonical_types_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"
      [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ]
    "#,
    r#"
      [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ]
    "#,
    r#"
      [
        { field = "related-concepts"; predicate = "related-concept"; }
      ]
    "#,
    r#"
      [
        { match-any = ["뭐"]; }
      ]
    "#,
    r#"
      [
        { match-any = ["공식"]; predicate = "formula"; label-ko = ["공식"]; }
      ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("wrong type in predicate-classifiers should fail");

  assert!(
    err
      .to_string()
      .contains("'label-ko' in predicate-classifiers entry must be string"),
    "{err:#}"
  );
}

#[test]
fn predicate_classifiers_matchers_must_be_non_empty_lists_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"
      [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ]
    "#,
    r#"
      [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ]
    "#,
    r#"
      [
        { field = "related-concepts"; predicate = "related-concept"; }
      ]
    "#,
    r#"
      [
        { match-any = ["뭐"]; }
      ]
    "#,
    r#"
      [
        { match-any = []; predicate = "formula"; label-ko = "공식"; }
      ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("empty predicate-classifiers matcher should fail");

  assert!(
    err
      .to_string()
      .contains("empty matcher list in 'predicate-classifiers' entry"),
    "{err:#}"
  );
}

#[test]
fn predicate_classifiers_reject_non_attrset_entries_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"
      [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ]
    "#,
    r#"
      [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ]
    "#,
    r#"
      [
        { field = "related-concepts"; predicate = "related-concept"; }
      ]
    "#,
    r#"
      [
        { match-any = ["뭐"]; }
      ]
    "#,
    r#"[ "bogus" ]"#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("non-attrset predicate-classifiers should fail");

  assert!(
    err
      .to_string()
      .contains("invalid 'predicate-classifiers' entry"),
    "{err:#}"
  );
}

#[test]
fn query_classifiers_predicate_classifiers_are_required() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐"]; }
      ];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("missing predicate-classifiers should fail");

  assert!(
    err.to_string().contains("missing 'predicate-classifiers'"),
    "{err:#}"
  );
}

#[test]
fn query_route_policy_fields_are_required_in_query_routes_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["property" "definition" "why"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "custom-property-route";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      predicate-classifiers = [
        { match-any = ["식"]; predicate = "formula"; label-ko = "식"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐" "무엇" "뭔"]; }
        { match-any = ["이란" "란"]; }
        { match-any = ["설명" "알려" "에 대해" "에 관해" "에 대하여" "에 관하여"]; }
      ];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  write_px(
    &query_routes,
    r#"
    [
      {
        route = "custom-property-route";
        query-context = "Pnix.Query.CustomProperty";
        include-hop-knowledge = "false";
        default-preview = "4";
        kernel-interpretation-id = "custom.defaults.${predicate}.${term}";
      }
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  let mut kernel = PnixReplKernel::new(paths);

  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 식"; }"#)
    .expect_err("missing route policy field should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'policy-coverage' for route 'custom-property-route'"),
    "{err:#}"
  );
}

#[test]
fn query_route_specs_must_exist_in_query_routes_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let query_route_defaults = temp_px_path("query-route-defaults.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["property" "definition" "why"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "custom-property-route";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      predicate-classifiers = [
        { match-any = ["식"]; predicate = "formula"; label-ko = "식"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐" "무엇" "뭔"]; }
        { match-any = ["이란" "란"]; }
        { match-any = ["설명" "알려" "에 대해" "에 관해" "에 대하여" "에 관하여"]; }
      ];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  write_px(&query_routes, "[]\n");
  write_px(
    &query_route_defaults,
    r#"
    {
      query-context-rewrite-rules = [
        { from = "Doghouse."; to = "Pnix."; }
      ];
      default-preview = "4";
      policy-defaults = {
        coverage = "0.0";
        coherence = "0.0";
        loss = "0.0";
        cost = "0.0";
        accept-threshold = "0.0";
      };
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.query_route_defaults_path = query_route_defaults;
  let mut kernel = PnixReplKernel::new(paths);

  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 식"; }"#)
    .expect_err("missing route spec should fail");

  assert!(
    err
      .to_string()
      .contains("missing query route spec for 'custom-property-route'"),
    "{err:#}"
  );
}

#[test]
fn query_route_context_is_required_in_query_routes_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let query_route_defaults = temp_px_path("query-route-defaults.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["property" "definition" "why"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "custom-property-route";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      predicate-classifiers = [
        { match-any = ["식"]; predicate = "formula"; label-ko = "식"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐" "무엇" "뭔"]; }
        { match-any = ["이란" "란"]; }
        { match-any = ["설명" "알려" "에 대해" "에 관해" "에 대하여" "에 관하여"]; }
      ];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  write_px(
    &query_routes,
    r#"
    [
      {
        route = "custom-property-route";
        include-hop-knowledge = "false";
        kernel-interpretation-id = "custom.defaults.${predicate}.${term}";
      }
    ]
    "#,
  );
  write_px(
    &query_route_defaults,
    r#"
    {
      query-context-rewrite-rules = [
        { from = "Doghouse."; to = "Pnix."; }
      ];
      default-preview = "4";
      policy-defaults = {
        coverage = "0.0";
        coherence = "0.0";
        loss = "0.0";
        cost = "0.0";
        accept-threshold = "0.0";
      };
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.query_route_defaults_path = query_route_defaults;
  let mut kernel = PnixReplKernel::new(paths);

  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 식"; }"#)
    .expect_err("missing query-context should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'query-context' in query-routes entry"),
    "{err:#}"
  );
}

#[test]
fn query_route_include_hop_knowledge_is_required_in_query_routes_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let query_route_defaults = temp_px_path("query-route-defaults.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["property" "definition" "why"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "custom-property-route";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      predicate-classifiers = [
        { match-any = ["식"]; predicate = "formula"; label-ko = "식"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐" "무엇" "뭔"]; }
        { match-any = ["이란" "란"]; }
        { match-any = ["설명" "알려" "에 대해" "에 관해" "에 대하여" "에 관하여"]; }
      ];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  write_px(
    &query_routes,
    r#"
    [
      {
        route = "custom-property-route";
        query-context = "Doghouse.Custom.custom-property-route";
        kernel-interpretation-id = "custom.defaults.${predicate}.${term}";
      }
    ]
    "#,
  );
  write_px(
    &query_route_defaults,
    r#"
    {
      query-context-rewrite-rules = [
        { from = "Doghouse."; to = "Pnix."; }
      ];
      default-preview = "4";
      policy-defaults = {
        coverage = "0.0";
        coherence = "0.0";
        loss = "0.0";
        cost = "0.0";
        accept-threshold = "0.0";
      };
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.query_route_defaults_path = query_route_defaults;
  let mut kernel = PnixReplKernel::new(paths);

  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 식"; }"#)
    .expect_err("missing include-hop-knowledge should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'include-hop-knowledge' in query-routes entry"),
    "{err:#}"
  );
}

#[test]
fn query_route_include_hop_knowledge_must_be_boolean_string() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"
    [
      { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
      { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
    ]
    "#,
    r#"
    [
      { field = "definition-ko"; predicate = "definition-ko"; }
    ]
    "#,
    r#"
    [
      { field = "related-concepts"; predicate = "related-concept"; }
    ]
    "#,
    r#"
    [
      { match-any = ["설명"]; }
    ]
    "#,
    "[]",
  );
  write_px(
    &query_routes,
    r#"
    [
      {
        route = "concept-definition-lookup";
        query-context = "Pnix.Query.ConceptDefinition";
        include-hop-knowledge = "bogus";
        default-preview = "3";
        policy-coverage = "0.6";
        policy-coherence = "0.4";
        policy-loss = "0.1";
        policy-cost = "0.2";
        policy-accept-threshold = "0.5";
        kernel-direct-fact-predicates = ["definition-ko"];
        kernel-direct-interpretation-id = "definition.direct.${term}";
        kernel-rich-interpretation-id = "definition.rich.${term}";
      }
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 설명"; }"#,
    )
    .expect_err("invalid include-hop-knowledge literal should fail");

  assert!(
    err.to_string().contains(
      "'include-hop-knowledge' for route 'concept-definition-lookup' must be 'true' or 'false'"
    ),
    "{err:#}"
  );
}

#[test]
fn query_route_definition_runtime_fields_are_required_in_query_routes_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["definition" "property" "why"];
      kernel-dispatch-routes = {
        definition = "custom-definition-route";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "formula"; predicate = "formula"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["설명"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  write_px(
    &query_routes,
    r#"
    [
      {
        route = "custom-definition-route";
        query-context = "Pnix.Query.CustomDefinition";
        include-hop-knowledge = "true";
        default-preview = "5";
        policy-coverage = "0.0";
        policy-coherence = "0.0";
        policy-loss = "0.0";
        policy-cost = "0.0";
        policy-accept-threshold = "0.0";
      }
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  let mut kernel = PnixReplKernel::new(paths);

  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 설명"; }"#,
    )
    .expect_err("missing definition runtime fields should fail");

  assert!(
    err.to_string().contains(
      "missing 'kernel-direct-fact-predicates' for definition route 'custom-definition-route'"
    ),
    "{err:#}"
  );
}

#[test]
fn query_route_definition_runtime_fields_require_canonical_types_in_query_routes_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["definition" "property" "why"];
      kernel-dispatch-routes = {
        definition = "custom-definition-route";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "formula"; predicate = "formula"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["설명"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  write_px(
    &query_routes,
    r#"
    [
      {
        route = "custom-definition-route";
        query-context = "Pnix.Query.CustomDefinition";
        include-hop-knowledge = "true";
        default-preview = "5";
        policy-coverage = "0.0";
        policy-coherence = "0.0";
        policy-loss = "0.0";
        policy-cost = "0.0";
        policy-accept-threshold = "0.0";
        kernel-direct-fact-predicates = "definition-ko";
        kernel-direct-interpretation-id = "custom.definition.direct.${term}";
        kernel-rich-interpretation-id = "custom.definition.rich.${term}";
      }
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  let mut kernel = PnixReplKernel::new(paths);

  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 설명"; }"#,
    )
    .expect_err("wrong definition runtime field type should fail");

  assert!(
    err
      .to_string()
      .contains("'kernel-direct-fact-predicates' for route 'custom-definition-route' must be list"),
    "{err:#}"
  );
}

#[test]
fn query_route_definition_runtime_lists_must_be_non_empty_in_query_routes_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["definition" "property" "why"];
      kernel-dispatch-routes = {
        definition = "custom-definition-route";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "formula"; predicate = "formula"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["설명"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  write_px(
    &query_routes,
    r#"
    [
      {
        route = "custom-definition-route";
        query-context = "Pnix.Query.CustomDefinition";
        include-hop-knowledge = "true";
        default-preview = "5";
        policy-coverage = "0.0";
        policy-coherence = "0.0";
        policy-loss = "0.0";
        policy-cost = "0.0";
        policy-accept-threshold = "0.0";
        kernel-direct-fact-predicates = [];
        kernel-direct-interpretation-id = "custom.definition.direct.${term}";
        kernel-rich-interpretation-id = "custom.definition.rich.${term}";
      }
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  let mut kernel = PnixReplKernel::new(paths);

  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 설명"; }"#,
    )
    .expect_err("empty definition runtime fact predicates should fail");

  assert!(
    err
      .to_string()
      .contains("empty 'kernel-direct-fact-predicates' for route 'custom-definition-route'"),
    "{err:#}"
  );
}

#[test]
fn query_route_property_runtime_fields_are_required_in_query_routes_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["property" "definition" "why"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "custom-property-route";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      predicate-classifiers = [
        { match-any = ["식"]; predicate = "formula"; label-ko = "식"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐" "무엇" "뭔"]; }
        { match-any = ["이란" "란"]; }
        { match-any = ["설명" "알려" "에 대해" "에 관해" "에 대하여" "에 관하여"]; }
      ];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  write_px(
    &query_routes,
    r#"
    [
      {
        route = "custom-property-route";
        query-context = "Pnix.Query.CustomProperty";
        include-hop-knowledge = "false";
        default-preview = "3";
        policy-coverage = "0.0";
        policy-coherence = "0.0";
        policy-loss = "0.0";
        policy-cost = "0.0";
        policy-accept-threshold = "0.0";
      }
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  let mut kernel = PnixReplKernel::new(paths);

  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 식"; }"#)
    .expect_err("missing property runtime fields should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'kernel-interpretation-id' for property route 'custom-property-route'"),
    "{err:#}"
  );
}

#[test]
fn query_route_property_runtime_fields_require_canonical_types_in_query_routes_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["property" "definition" "why"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "custom-property-route";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["설명"]; }
      ];
      predicate-classifiers = [
        { match-any = ["공식"]; predicate = "formula"; label-ko = "공식"; }
      ];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  write_px(
    &query_routes,
    r#"
    [
      {
        route = "custom-property-route";
        query-context = "Pnix.Query.CustomProperty";
        include-hop-knowledge = "false";
        default-preview = "5";
        policy-coverage = "0.0";
        policy-coherence = "0.0";
        policy-loss = "0.0";
        policy-cost = "0.0";
        policy-accept-threshold = "0.0";
        kernel-interpretation-id = ["custom.property.${predicate}.${term}"];
      }
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  let mut kernel = PnixReplKernel::new(paths);

  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 공식"; }"#,
    )
    .expect_err("wrong property runtime field type should fail");

  assert!(
    err
      .to_string()
      .contains("'kernel-interpretation-id' for route 'custom-property-route' must be string"),
    "{err:#}"
  );
}

#[test]
fn query_route_defaults_rewrite_entries_require_canonical_fields() {
  let query_route_defaults = temp_px_path("query-route-defaults.px");
  write_px(
    &query_route_defaults,
    r#"
    {
      query-context-rewrite-rules = [
        { to = "Pnix."; }
      ];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_route_defaults_path = query_route_defaults;
  let mut kernel = PnixReplKernel::new(paths);

  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 설명"; }"#,
    )
    .expect_err("invalid query-context-rewrite-rules should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'from' in query-context-rewrite-rules entry"),
    "{err:#}"
  );
}

#[test]
fn query_routes_entries_must_be_attrsets() {
  let query_routes = temp_px_path("query-routes.px");
  write_px(
    &query_routes,
    r#"
    [
      "bogus"
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_routes_path = query_routes;
  let mut kernel = PnixReplKernel::new(paths);

  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 설명"; }"#,
    )
    .expect_err("non-attrset query-routes entry should fail");

  assert!(
    err.to_string().contains("invalid 'query-routes' entry"),
    "{err:#}"
  );
}

#[test]
fn query_routes_root_must_be_list() {
  let query_routes = temp_px_path("query-routes.px");
  write_px(
    &query_routes,
    r#"
    {
      route = "bogus"
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_routes_path = query_routes;
  let mut kernel = PnixReplKernel::new(paths);

  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 설명"; }"#,
    )
    .expect_err("query-routes root type should fail");

  assert!(
    err.to_string().contains("query-routes root must be list"),
    "{err:#}"
  );
}

#[test]
fn query_route_name_is_required_in_query_routes_px() {
  let query_routes = temp_px_path("query-routes.px");
  write_px(
    &query_routes,
    r#"
    [
      {
        query-context = "Pnix.Query.Custom";
        include-hop-knowledge = "false";
        default-preview = "3";
        policy-coverage = "0.0";
        policy-coherence = "0.0";
        policy-loss = "0.0";
        policy-cost = "0.0";
        policy-accept-threshold = "0.0";
      }
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_routes_path = query_routes;
  let mut kernel = PnixReplKernel::new(paths);

  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 설명"; }"#,
    )
    .expect_err("missing route name should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'route' in query-routes entry"),
    "{err:#}"
  );
}

#[test]
fn held_reason_rules_are_owned_by_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "ctx-needed";
        unknown-term = "unknown-concept";
      };
      held-reason-rules = [
        {
          when = "known-term";
          reason-key = "requires-context";
          term-source = "none";
        }
        {
          when = "unknown-term";
          reason-key = "unknown-term";
          term-source = "first-extracted-term";
        }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      predicate-classifiers = [];
      definition-query-rules = [
        { match-any = ["뭐" "무엇" "뭔"]; }
        { match-any = ["이란" "란"]; }
        { match-any = ["설명" "알려" "에 대해" "에 관해" "에 대하여" "에 관하여"]; }
      ];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
        { distinguishing-predicate = "unknown-term"; question-template = "UNKNOWN ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "ctx-needed"; predicate = "experimental-context"; }
        { reason = "unknown-concept"; predicate = "unknown-term"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "미정"; }
      ];
      default-choices = ["선택A"];
      held-response-rules = [
        { when = "term-present"; template = "HELD ${term}"; emit-held-term = "true"; }
        { when = "term-missing"; template = "HELD NONE"; emit-held-term = "false"; }
      ];
      concept-choices = [];
      unknown-term-label = "미정";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);

  let response = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect("evaluate");

  assert_eq!(response.route, "lightweight-korean-dialogue-held");
  assert_eq!(response.follow_up_hint.as_deref(), Some("CTX 미정"));
  assert!(
    response
      .envelope
      .notes
      .iter()
      .any(|note| note == "held-reason:ctx-needed"),
    "{:?}",
    response.envelope.notes
  );
  assert!(
    response
      .envelope
      .notes
      .iter()
      .all(|note| note != "concept-held:term:힘"),
    "{:?}",
    response.envelope.notes
  );
}

#[test]
fn held_reason_rules_require_known_when_values_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"
      [
        { when = "bogus"; reason-key = "requires-context"; term-source = "matched-term"; }
      ]
    "#,
    r#"
      [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ]
    "#,
    r#"
      [
        { field = "related-concepts"; predicate = "related-concept"; }
      ]
    "#,
    r#"
      [
        { match-any = ["뭐"]; }
      ]
    "#,
    r#"[]"#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("invalid held-reason when should fail");

  assert!(
    err
      .to_string()
      .contains("invalid 'when' for held-reason-rules entry"),
    "{err:#}"
  );
}

#[test]
fn held_reason_rules_require_known_term_source_values_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"
      [
        { when = "known-term"; reason-key = "requires-context"; term-source = "bogus"; }
      ]
    "#,
    r#"
      [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ]
    "#,
    r#"
      [
        { field = "related-concepts"; predicate = "related-concept"; }
      ]
    "#,
    r#"
      [
        { match-any = ["뭐"]; }
      ]
    "#,
    r#"[]"#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("invalid held-reason term-source should fail");

  assert!(
    err
      .to_string()
      .contains("invalid 'term-source' for held-reason-rules entry"),
    "{err:#}"
  );
}

#[test]
fn held_reason_rules_require_known_reason_key_values_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"
      [
        { when = "known-term"; reason-key = "bogus"; term-source = "matched-term"; }
      ]
    "#,
    r#"
      [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ]
    "#,
    r#"
      [
        { field = "related-concepts"; predicate = "related-concept"; }
      ]
    "#,
    r#"
      [
        { match-any = ["뭐"]; }
      ]
    "#,
    r#"[]"#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("invalid held-reason reason-key should fail");

  assert!(
    err
      .to_string()
      .contains("invalid 'reason-key' for held-reason-rules entry"),
    "{err:#}"
  );
}

#[test]
fn held_reason_rules_must_cover_term_states_in_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"
      [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
      ]
    "#,
    r#"
      [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ]
    "#,
    r#"
      [
        { field = "related-concepts"; predicate = "related-concept"; }
      ]
    "#,
    r#"
      [
        { match-any = ["뭐"]; }
      ]
    "#,
    r#"[]"#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "미지개념"; }"#,
    )
    .expect_err("uncovered held term state should fail");

  assert!(
    err
      .to_string()
      .contains("no held-reason-rules entry matched term state 'term-missing'"),
    "{err:#}"
  );
}

#[test]
fn followup_resolved_term_rules_are_owned_by_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "none"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      predicate-classifiers = [];
      definition-query-rules = [
        { match-any = ["뭐" "무엇" "뭔"]; }
        { match-any = ["이란" "란"]; }
        { match-any = ["설명" "알려" "에 대해" "에 관해" "에 대하여" "에 관하여"]; }
      ];
      concept-what-markers = ["뭐"];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "unknown-term"; question-template = "UNKNOWN ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "unknown-term"; predicate = "unknown-term"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "literal"; value = "미상개념"; }
      ];
      held-response-rules = [
        { when = "term-missing"; template = "HELD ${term}"; emit-held-term = "false"; }
      ];
      concept-choices = [];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);

  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "푸바는 뭐야"; }"#,
    )
    .expect("evaluate");

  assert_eq!(response.follow_up_hint.as_deref(), Some("UNKNOWN 미상개념"));
  assert!(response.response_document_org.contains("HELD 미상개념"));
}

#[test]
fn followup_templates_require_resolved_term_when_using_term_placeholder() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let followups = temp_px_path("followup-generation.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"[
      { when = "unknown-term"; reason-key = "unknown-term"; term-source = "none"; }
    ]"#,
    r#"[
      { field = "definition-ko"; predicate = "definition-ko"; }
    ]"#,
    r#"[
      { field = "related-concepts"; predicate = "related-concept"; }
    ]"#,
    r#"[
      { match-any = ["뭐"]; }
    ]"#,
    r#"[]"#,
  );
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "unknown-term"; question-template = "UNKNOWN ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "unknown-term"; predicate = "unknown-term"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-missing"; term-source = "none"; }
      ];
      held-response-rules = [
        { when = "term-missing"; template = "HELD NONE"; emit-held-term = "false"; }
      ];
      concept-choices = [];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);

  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "푸바는 뭐야"; }"#,
    )
    .expect_err("follow-up template requiring missing term should fail");

  assert!(
    err
      .to_string()
      .contains("follow-up template requires resolved term"),
    "{err:#}"
  );
}

#[test]
fn held_response_templates_require_resolved_term_when_using_term_placeholder() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let followups = temp_px_path("followup-generation.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"[
      { when = "unknown-term"; reason-key = "unknown-term"; term-source = "none"; }
    ]"#,
    r#"[
      { field = "definition-ko"; predicate = "definition-ko"; }
    ]"#,
    r#"[
      { field = "related-concepts"; predicate = "related-concept"; }
    ]"#,
    r#"[
      { match-any = ["뭐"]; }
    ]"#,
    r#"[]"#,
  );
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "unknown-term"; question-template = "UNKNOWN"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "unknown-term"; predicate = "unknown-term"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-missing"; term-source = "none"; }
      ];
      held-response-rules = [
        { when = "term-missing"; template = "HELD ${term}"; emit-held-term = "false"; }
      ];
      concept-choices = [];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);

  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "푸바는 뭐야"; }"#,
    )
    .expect_err("held-response template requiring missing term should fail");

  assert!(
    err
      .to_string()
      .contains("held-response template requires resolved term"),
    "{err:#}"
  );
}

#[test]
fn followup_templates_require_non_empty_suggestions_when_using_placeholder() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let followups = temp_px_path("followup-generation.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"[
      { when = "unknown-term"; reason-key = "unknown-term"; term-source = "none"; }
    ]"#,
    r#"[
      { field = "definition-ko"; predicate = "definition-ko"; }
    ]"#,
    r#"[
      { field = "related-concepts"; predicate = "related-concept"; }
    ]"#,
    r#"[
      { match-any = ["뭐"]; }
    ]"#,
    r#"[]"#,
  );
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "unknown-term"; question-template = "UNKNOWN ${term}"; choices-template = " :: ${suggestions}"; }
      ];
      reason-question-rules = [
        { reason = "unknown-term"; predicate = "unknown-term"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-missing"; choice-source = "none"; }
      ];
      resolved-term-rules = [
        { when = "term-missing"; term-source = "literal"; value = "미상개념"; }
      ];
      held-response-rules = [
        { when = "term-missing"; template = "HELD NONE"; emit-held-term = "false"; }
      ];
      concept-choices = [];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);

  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "푸바는 뭐야"; }"#,
    )
    .expect_err("follow-up template requiring empty suggestions should fail");

  assert!(
    err
      .to_string()
      .contains("follow-up template requires non-empty suggestions"),
    "{err:#}"
  );
}

#[test]
fn held_response_emit_held_term_must_be_string() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let followups = temp_px_path("followup-generation.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"[
      { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
      { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
    ]"#,
    r#"[
      { field = "definition-ko"; predicate = "definition-ko"; }
    ]"#,
    r#"[
      { field = "related-concepts"; predicate = "related-concept"; }
    ]"#,
    r#"[
      { match-any = ["뭐"]; }
    ]"#,
    r#"[]"#,
  );
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "unknown-term"; question-template = "UNKNOWN ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "unknown-term"; predicate = "unknown-term"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "literal"; value = "미상개념"; }
      ];
      held-response-rules = [
        { when = "term-missing"; template = "HELD ${term}"; emit-held-term = ["false"]; }
      ];
      concept-choices = [];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "푸바는 뭐야"; }"#,
    )
    .expect_err("wrong emit-held-term type should fail");

  assert!(
    err
      .to_string()
      .contains("'emit-held-term' in held-response-rules entry must be string"),
    "{err:#}"
  );
}

#[test]
fn held_response_emit_held_term_must_be_boolean_string() {
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "unknown-term"; question-template = "UNKNOWN ${term}"; choices-template = ""; }
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = " :: ${suggestions}"; }
      ];
      reason-question-rules = [
        { reason = "unknown-term"; predicate = "unknown-term"; }
        { reason = "requires-context"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      held-response-rules = [
        { when = "term-present"; template = "HELD ${term}"; emit-held-term = "bogus"; }
      ];
      concept-choices = [];
      default-choices = ["더 구체적으로 알려줘"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "모르는것"; }"#,
    )
    .expect_err("invalid emit-held-term literal should fail");

  assert!(
    err
      .to_string()
      .contains("'emit-held-term' in held-response-rules entry must be 'true' or 'false'"),
    "{err:#}"
  );
}

#[test]
fn held_response_rules_are_owned_by_px() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "formal-name-en"; predicate = "formal-name-en"; }
        { field = "formal-symbol"; predicate = "formal-symbol"; }
        { field = "domain"; predicate = "domain"; }
        { field = "unit-ko"; predicate = "unit-ko"; }
        { field = "formula"; predicate = "formula"; }
        { field = "inverse-of"; predicate = "inverse-of"; }
        { field = "category"; predicate = "category"; }
        { field = "why"; predicate = "why"; }
        { field = "boundary-conditions"; predicate = "boundary-condition"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      predicate-classifiers = [];
      definition-query-rules = [
        { match-any = ["뭐" "무엇" "뭔"]; }
        { match-any = ["이란" "란"]; }
        { match-any = ["설명" "알려" "에 대해" "에 관해" "에 대하여" "에 관하여"]; }
      ];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "literal"; value = "미상"; }
      ];
      held-response-rules = [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
        { when = "term-missing"; template = "WAIT NONE"; emit-held-term = "false"; }
      ];
      concept-choices = [];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);

  let response = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect("evaluate");

  assert_eq!(response.follow_up_hint.as_deref(), Some("CTX 힘"));
  assert!(response.response_document_org.contains("WAIT 힘"));
  assert!(
    response
      .envelope
      .notes
      .iter()
      .all(|note| note != "held-term:힘"),
    "{:?}",
    response.envelope.notes
  );
}

#[test]
fn followup_disambiguation_question_entries_require_canonical_fields() {
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { question-template = "CTX ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      held-response-rules = [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
        { when = "term-missing"; template = "WAIT NONE"; emit-held-term = "false"; }
      ];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("invalid disambiguation question should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'distinguishing-predicate' in disambiguation-questions entry"),
    "{err:#}"
  );
}

#[test]
fn followup_disambiguation_question_entries_reject_non_attrsets() {
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = ["bogus"];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      held-response-rules = [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
        { when = "term-missing"; template = "WAIT NONE"; emit-held-term = "false"; }
      ];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("non-attrset disambiguation-questions should fail");

  assert!(
    err
      .to_string()
      .contains("invalid 'disambiguation-questions' entry"),
    "{err:#}"
  );
}

#[test]
fn followup_reason_question_rule_entries_require_canonical_fields() {
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "requires-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      held-response-rules = [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
        { when = "term-missing"; template = "WAIT NONE"; emit-held-term = "false"; }
      ];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("invalid reason-question-rules should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'predicate' in reason-question-rules entry"),
    "{err:#}"
  );
}

#[test]
fn followup_reopen_rules_reject_non_attrsets() {
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "experimental-context"; }
      ];
      reopen-rules = ["bogus"];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      held-response-rules = [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
        { when = "term-missing"; template = "WAIT NONE"; emit-held-term = "false"; }
      ];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("non-attrset reopen-rules should fail");

  assert!(
    err.to_string().contains("invalid 'reopen-rules' entry"),
    "{err:#}"
  );
}

#[test]
fn followup_choice_entries_require_canonical_fields() {
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present"; }
      ];
      resolved-term-rules = [
        { when = "term-missing"; value = "질문"; }
      ];
      held-response-rules = [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
      ];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("invalid choice-rules should fail before runtime");

  assert!(
    err
      .to_string()
      .contains("missing 'choice-source' in choice-rules entry"),
    "{err:#}"
  );
}

#[test]
fn followup_concept_choices_require_canonical_fields() {
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      held-response-rules = [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
        { when = "term-missing"; template = "WAIT NONE"; emit-held-term = "false"; }
      ];
      concept-choices = [
        { term = "힘"; }
      ];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("invalid concept-choices should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'choices' in concept-choices entry"),
    "{err:#}"
  );
}

#[test]
fn followup_concept_choices_are_required() {
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      held-response-rules = [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
        { when = "term-missing"; template = "WAIT NONE"; emit-held-term = "false"; }
      ];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("missing concept-choices should fail");

  assert!(
    err.to_string().contains("missing 'concept-choices'"),
    "{err:#}"
  );
}

#[test]
fn followup_concept_choices_choices_must_be_list() {
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      held-response-rules = [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
        { when = "term-missing"; template = "WAIT NONE"; emit-held-term = "false"; }
      ];
      concept-choices = [
        { term = "힘"; choices = "뉴턴 역학에서?"; }
      ];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("wrong concept-choices.choices type should fail");

  assert!(
    err
      .to_string()
      .contains("'choices' in concept-choices entry must be list"),
    "{err:#}"
  );
}

#[test]
fn followup_concept_choices_reject_non_attrsets() {
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      held-response-rules = [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
        { when = "term-missing"; template = "WAIT NONE"; emit-held-term = "false"; }
      ];
      concept-choices = ["bogus"];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("non-attrset concept-choices should fail");

  assert!(
    err.to_string().contains("invalid 'concept-choices' entry"),
    "{err:#}"
  );
}

#[test]
fn followup_concept_choice_source_requires_existing_concept_choices() {
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present"; choice-source = "concept"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      held-response-rules = [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
        { when = "term-missing"; template = "WAIT NONE"; emit-held-term = "false"; }
      ];
      concept-choices = [];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("missing concept choices should fail for concept choice-source");

  assert!(
    err.to_string().contains("missing concept choices for '힘'"),
    "{err:#}"
  );
}

#[test]
fn followup_resolved_term_entries_require_canonical_fields() {
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-missing"; value = "질문"; }
      ];
      held-response-rules = [
        { when = "term-present"; template = "WAIT ${term}"; emit-held-term = "false"; }
        { when = "term-missing"; template = "WAIT NONE"; emit-held-term = "false"; }
      ];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("invalid resolved-term-rules should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'term-source' in resolved-term-rules entry"),
    "{err:#}"
  );
}

#[test]
fn followup_held_response_entries_require_canonical_fields() {
  let followups = temp_px_path("followup-generation.px");
  write_px(
    &followups,
    r#"
    {
      disambiguation-questions = [
        { distinguishing-predicate = "experimental-context"; question-template = "CTX ${term}"; choices-template = ""; }
      ];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "experimental-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
        { reason = "unknown-term"; carry-term-policy = "never"; effective-utterance-template = "${utterance}"; }
      ];
      choice-rules = [
        { when = "term-present-with-concept-choice"; choice-source = "concept"; }
        { when = "term-present-without-concept-choice"; choice-source = "default"; }
        { when = "term-missing"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "term-present"; term-source = "term"; }
        { when = "term-missing"; term-source = "label"; value = "질문"; }
      ];
      held-response-rules = [
        { when = "term-present"; }
      ];
      default-choices = ["선택A"];
      unknown-term-label = "질문";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followups;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘"; }"#)
    .expect_err("invalid held-response-rules should fail");

  assert!(
    err
      .to_string()
      .contains("missing 'template' in held-response-rules entry"),
    "{err:#}"
  );
}

#[test]
fn concept_optional_scalar_fields_must_be_strings() {
  let concepts_dir = temp_px_path("concepts");
  fs::create_dir_all(&concepts_dir).expect("create concepts dir");
  write_px(
    &concepts_dir.join("bad-concept.px"),
    r#"
    [
      {
        term-ko = "힘";
        definition-ko = "정의";
        formal-symbol = ["F"];
        context = "Physics.Mechanics";
        domain = "물리";
        related-concepts = ["질량"];
      }
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.concepts_dir = concepts_dir;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("wrong concept scalar type should fail");

  assert!(
    err
      .to_string()
      .contains("'formal-symbol' in concept entry must be string"),
    "{err:#}"
  );
}

#[test]
fn concept_related_concepts_must_be_list() {
  let concepts_dir = temp_px_path("concepts");
  fs::create_dir_all(&concepts_dir).expect("create concepts dir");
  write_px(
    &concepts_dir.join("bad-concept.px"),
    r#"
    [
      {
        term-ko = "힘";
        definition-ko = "정의";
        context = "Physics.Mechanics";
        domain = "물리";
        related-concepts = "질량";
      }
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.concepts_dir = concepts_dir;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("wrong related-concepts type should fail");

  assert!(
    err
      .to_string()
      .contains("'related-concepts' in concept entry must be list"),
    "{err:#}"
  );
}

#[test]
fn synonyms_root_must_be_attrset() {
  let synonyms = temp_px_path("synonyms.px");
  write_px(&synonyms, r#"[ "힘" ]"#);

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.synonyms_path = synonyms;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("wrong synonyms root type should fail");

  assert!(
    err.to_string().contains("synonyms root must be attrset"),
    "{err:#}"
  );
}

#[test]
fn synonym_group_aliases_must_be_list() {
  let synonyms = temp_px_path("synonyms.px");
  write_px(
    &synonyms,
    r#"
    {
      synonym-groups = [
        { canonical = "힘"; aliases = "포스"; }
      ];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.synonyms_path = synonyms;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("wrong aliases type should fail");

  assert!(
    err
      .to_string()
      .contains("'aliases' in synonym-groups entry must be list"),
    "{err:#}"
  );
}

#[test]
fn invert_candidate_rule_obj_template_rejects_unsupported_placeholder() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [
        {
          type = "causal-inverse";
          predicate = "causal-chain";
          context = "ontology-invert.causal";
          obj-template = "'${term}' 인과 후보 ${alt-count}개";
        }
      ];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("unsupported obj-template placeholder should fail");

  assert!(
    err
      .to_string()
      .contains("unsupported placeholder '${alt-count}'"),
    "{err:#}"
  );
}

#[test]
fn invert_domain_to_regime_wildcard_entry_is_tolerated_if_matches_default() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [
        { domain-prefix = "*"; regime = "interpretive"; }
      ];
      invert-candidate-rules = [];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect("wildcard-only domain-to-regime should evaluate");
  assert!(
    response
      .envelope
      .notes
      .iter()
      .any(|note| note == "truth-regime:interpretive"),
    "{:?}",
    response.envelope.notes
  );
}

#[test]
fn query_routes_reject_query_context_not_matching_any_rewrite_rule() {
  let query_routes = temp_px_path("query-routes.px");
  let query_route_defaults = temp_px_path("query-route-defaults.px");

  write_px(
    &query_route_defaults,
    r#"
    {
      query-context-rewrite-rules = [
        { from = "Doghouse."; to = "Pnix."; }
      ];
    }
    "#,
  );

  write_px(
    &query_routes,
    r#"
    [
      {
        route = "concept-definition-lookup";
        query-context = "Legacy.Namespace.Definition";
        include-hop-knowledge = "false";
        default-preview = "1";
        policy-coverage = "0.5";
        policy-coherence = "0.5";
        policy-loss = "0.5";
        policy-cost = "0.5";
        policy-accept-threshold = "0.5";
        kernel-direct-fact-predicates = ["definition-ko"];
        kernel-direct-interpretation-id = "interp.definition.direct.${term}";
        kernel-rich-interpretation-id = "interp.definition.rich.${term}";
      }
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_routes_path = query_routes;
  paths.query_route_defaults_path = query_route_defaults;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("query-context not matching any rewrite rule should fail");

  let message = err.to_string();
  assert!(
    message.contains("does not match any 'query-context-rewrite-rules'"),
    "{message}"
  );
  assert!(message.contains("Legacy.Namespace.Definition"), "{message}");
}

#[test]
fn query_routes_accept_query_context_already_canonical_when_starts_with_to_prefix() {
  let query_routes = temp_px_path("query-routes.px");
  let query_route_defaults = temp_px_path("query-route-defaults.px");

  write_px(
    &query_route_defaults,
    r#"
    {
      query-context-rewrite-rules = [
        { from = "Doghouse."; to = "Pnix."; }
      ];
    }
    "#,
  );

  // "Pnix.Query.Definition" already starts with the canonical "Pnix." target
  // prefix so it must passthrough without rewriting.
  write_px(
    &query_routes,
    r#"
    [
      {
        route = "concept-definition-lookup";
        query-context = "Pnix.Query.Definition";
        include-hop-knowledge = "false";
        default-preview = "1";
        policy-coverage = "0.5";
        policy-coherence = "0.5";
        policy-loss = "0.5";
        policy-cost = "0.5";
        policy-accept-threshold = "0.5";
        kernel-direct-fact-predicates = ["definition-ko"];
        kernel-direct-interpretation-id = "interp.definition.direct.${term}";
        kernel-rich-interpretation-id = "interp.definition.rich.${term}";
      }
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_routes_path = query_routes;
  paths.query_route_defaults_path = query_route_defaults;
  let _kernel = PnixReplKernel::new(paths);
  // Loading alone proves the strict passthrough is accepted. If it had been
  // rejected, PnixReplKernel::new would have panicked during fixture load.
}

#[test]
fn kernel_base_facts_must_exist_and_have_at_least_one_entry() {
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [];
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("empty base-query-facts should fail");
  assert!(
    err
      .to_string()
      .contains("'base-query-facts' must have at least one entry"),
    "{err:#}"
  );
}

#[test]
fn kernel_base_facts_reject_unsupported_placeholder_in_obj_template() {
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${bogus}";
        }
      ];
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("unsupported placeholder should fail");
  assert!(
    err
      .to_string()
      .contains("unsupported placeholder '${bogus}'"),
    "{err:#}"
  );
}

#[test]
fn kernel_base_facts_reject_both_obj_template_and_obj_literal() {
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
          obj-literal = "duplicate";
        }
      ];
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("both obj-template and obj-literal should fail");
  assert!(err.to_string().contains("must not carry both"), "{err:#}");
}

#[test]
fn kernel_base_facts_runtime_shaping_is_owned_by_px() {
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "custom.fact.route.${route-segment}";
          context = "Custom.Scope";
          subj = "custom-subj";
          pred = "custom-route";
          obj-template = "${route}";
        }
        {
          id-template = "custom.fact.lang";
          context = "Custom.Scope";
          subj = "custom-subj";
          pred = "custom-lang";
          obj-literal = "custom-ko";
        }
        {
          when-route = "concept-definition-lookup";
          id-template = "custom.fact.concept.term";
          context = "Custom.Concept";
          subj = "custom-subj";
          pred = "custom-concept-term";
          obj-template = "concept:${term}";
        }
        {
          when-route = "concept-definition-lookup";
          id-template = "custom.fact.concept.formal";
          context = "Custom.Concept";
          subj = "custom-subj";
          pred = "custom-concept-formal-en";
          obj-template = "${formal-name-en}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      note-templates = {
        transcript-user = "transcript:user: ${utterance}";
        transcript-pnix = "transcript:pnix: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
      query-provenance-templates = {
        utterance = "utterance:${utterance}";
        concept-source = "concept-source:${source-ref}";
      };
      semantic-id-templates = {
        episode-id-template = "episode.pnix.standalone.${counter}";
        record-id-template = "record.fact.${episode-id}.${index}";
        knowledge-id-template = "knowledge.pnix.${episode-id}";
        knowledge-summary = "pnix standalone query kernel staging record";
      };
      pipeline-trace-note-prefixes = ["ontology-" "truth-regime:" "held-"];
      transcript-note-prefix = "transcript:";
      output-fragment-templates = {
        pipeline-trace = { kind = "pipeline-trace"; visibility = "dev"; };
        response-document = { kind = "response-document"; visibility = "dev"; };
      };
      response-document-schema = {
        px-header-comment = "# pnix ontology response document (auto-generated)";
        px-field-episode-id = "episode-id";
        px-field-summary = "summary";
        px-field-transcript = "transcript";
        px-field-pipeline = "pipeline";
        px-field-facts-count = "facts-count";
        org-title = "* Ontology Response";
        org-pipeline-section-header = "** Pipeline";
        org-facts-count-template = "- Facts: ${count}";
        org-transcript-transforms = [
          { input-prefix = "user: "; output-prefix = "** Q: "; }
          { input-prefix = "pnix: "; output-prefix = ""; }
        ];
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect("should evaluate");

  use pnix_core::ontology::SemanticRecordValue;
  let base_facts: Vec<_> = response
    .envelope
    .records
    .iter()
    .filter_map(|r| match &r.value {
      SemanticRecordValue::ContextualFact(f) => Some(f.clone()),
      _ => None,
    })
    .filter(|f| f.context.0 == "Custom.Scope")
    .collect();
  assert!(base_facts.len() >= 2, "got: {base_facts:#?}");
  let has_route_fact = base_facts
    .iter()
    .any(|f| f.subj == "custom-subj" && f.pred == "custom-route" && !f.obj.is_empty());
  let has_lang_fact = base_facts
    .iter()
    .any(|f| f.subj == "custom-subj" && f.pred == "custom-lang" && f.obj == "custom-ko");
  assert!(has_route_fact, "{base_facts:#?}");
  assert!(has_lang_fact, "{base_facts:#?}");
  let custom_concept_fact = response
    .envelope
    .records
    .iter()
    .find_map(|r| match &r.value {
      SemanticRecordValue::ContextualFact(f)
        if f.context.0 == "Custom.Concept" && f.pred == "custom-concept-term" =>
      {
        Some(f.clone())
      }
      _ => None,
    })
    .expect("route-scoped concept base fact missing");
  assert_eq!(
    custom_concept_fact.obj, "concept:힘",
    "{custom_concept_fact:#?}"
  );
  let custom_formal_fact = response
    .envelope
    .records
    .iter()
    .find_map(|r| match &r.value {
      SemanticRecordValue::ContextualFact(f)
        if f.context.0 == "Custom.Concept" && f.pred == "custom-concept-formal-en" =>
      {
        Some(f.clone())
      }
      _ => None,
    })
    .expect("route-scoped formal-name-en base fact missing");
  assert_eq!(custom_formal_fact.obj, "force", "{custom_formal_fact:#?}");
}

#[test]
fn kernel_base_facts_scope_placeholder_is_filled_from_request_scope() {
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "custom.fact.route.${route-segment}";
          context = "Custom.Scope";
          subj = "custom-subj";
          pred = "custom-route";
          obj-template = "${route}";
        }
        {
          id-template = "custom.fact.scope";
          context = "Custom.Scope";
          subj = "custom-subj";
          pred = "custom-scope";
          obj-template = "${scope}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      note-templates = {
        transcript-user = "transcript:user: ${utterance}";
        transcript-pnix = "transcript:pnix: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
      query-provenance-templates = {
        utterance = "utterance:${utterance}";
        concept-source = "concept-source:${source-ref}";
      };
      semantic-id-templates = {
        episode-id-template = "episode.pnix.standalone.${counter}";
        record-id-template = "record.fact.${episode-id}.${index}";
        knowledge-id-template = "knowledge.pnix.${episode-id}";
        knowledge-summary = "pnix standalone query kernel staging record";
      };
      pipeline-trace-note-prefixes = ["ontology-" "truth-regime:" "held-"];
      transcript-note-prefix = "transcript:";
      output-fragment-templates = {
        pipeline-trace = { kind = "pipeline-trace"; visibility = "dev"; };
        response-document = { kind = "response-document"; visibility = "dev"; };
      };
      response-document-schema = {
        px-header-comment = "# pnix ontology response document (auto-generated)";
        px-field-episode-id = "episode-id";
        px-field-summary = "summary";
        px-field-transcript = "transcript";
        px-field-pipeline = "pipeline";
        px-field-facts-count = "facts-count";
        org-title = "* Ontology Response";
        org-pipeline-section-header = "** Pipeline";
        org-facts-count-template = "- Facts: ${count}";
        org-transcript-transforms = [
          { input-prefix = "user: "; output-prefix = "** Q: "; }
          { input-prefix = "pnix: "; output-prefix = ""; }
        ];
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "detailed"; utterance = "힘은 뭐야"; }"#,
    )
    .expect("should evaluate");

  use pnix_core::ontology::SemanticRecordValue;
  let scope_fact = response
    .envelope
    .records
    .iter()
    .find_map(|r| match &r.value {
      SemanticRecordValue::ContextualFact(f)
        if f.context.0 == "Custom.Scope" && f.pred == "custom-scope" =>
      {
        Some(f.clone())
      }
      _ => None,
    })
    .expect("scope base fact missing");
  assert_eq!(scope_fact.obj, "detailed", "{scope_fact:#?}");
}

#[test]
fn kernel_base_facts_scope_placeholder_rejects_non_scope_placeholder_when_allowlist_hit() {
  // Sanity check: the shared validate_placeholder_allowlist helper still
  // applies to invert-candidate-rules obj-template, producing the same error
  // format the previous regression test used.
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [
        {
          type = "causal-inverse";
          predicate = "causal-chain";
          context = "ontology-invert.causal";
          obj-template = "'${term}' 근거 ${scope}";
        }
      ];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  // ${scope} is not in the invert obj-template allowlist — only ${term} and
  // ${provenance} are. The shared helper must still reject this.
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("invert rule ${scope} placeholder should be rejected");
  assert!(
    err
      .to_string()
      .contains("unsupported placeholder '${scope}'"),
    "{err:#}"
  );
}

#[test]
fn dialogue_template_scope_is_requires_lowercase_canonical() {
  // scope-is canonical is now lowercase ("brief" / "standard" / "detailed").
  // The pre-unification "Detailed" (Capital) form must be rejected.
  let dialogue_templates = temp_px_path("dialogue-templates.px");
  write_px(
    &dialogue_templates,
    r#"
    {
      kernel-definition-section = {
        join-with = ". ";
        suffix = ".";
        parts = [
          { when = "always"; template = "BASE ${term}"; }
          { when = "always"; scope-is = "Detailed"; template = "DETAIL ${term}"; }
        ];
      };
      kernel-why-section = {
        join-with = " ";
        suffix = "";
        parts = [
          { when = "always"; template = "WHY ${regime}"; }
        ];
      };
      kernel-property-section = {
        join-with = "";
        suffix = "";
        parts = [
          { when = "always"; values-state = "present"; template = "PROP ${term} ${values}"; }
          { when = "always"; values-state = "empty"; template = "EMPTY ${term}"; }
        ];
      };
      kernel-route-summary = {
        definition = "def";
        property = "prop";
        why = "why";
        held = "held";
      };
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.dialogue_templates_path = dialogue_templates;
  let mut kernel = PnixReplKernel::new(paths);

  let error = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("Capital scope-is should fail after lowercase unification");

  let message = error.to_string();
  assert!(
    message.contains("invalid 'scope-is' for kernel-definition-section part"),
    "{message}"
  );
  assert!(message.contains("brief"), "{message}");
  assert!(message.contains("detailed"), "{message}");
}

#[test]
fn dialogue_template_scope_is_accepts_lowercase_canonical_end_to_end() {
  let dialogue_templates = temp_px_path("dialogue-templates.px");
  write_px(
    &dialogue_templates,
    r#"
    {
      kernel-definition-section = {
        join-with = ". ";
        suffix = ".";
        parts = [
          { when = "always"; template = "BASE ${term}"; }
          { when = "always"; scope-is = "detailed"; template = "DETAIL ${term}"; }
        ];
      };
      kernel-why-section = {
        join-with = " ";
        suffix = "";
        parts = [
          { when = "always"; template = "WHY ${regime}"; }
        ];
      };
      kernel-property-section = {
        join-with = "";
        suffix = "";
        parts = [
          { when = "always"; values-state = "present"; template = "PROP ${term} ${values}"; }
          { when = "always"; values-state = "empty"; template = "EMPTY ${term}"; }
        ];
      };
      kernel-route-summary = {
        definition = "def";
        property = "prop";
        why = "why";
        held = "held";
      };
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.dialogue_templates_path = dialogue_templates;
  let mut kernel = PnixReplKernel::new(paths);
  // detailed scope request: the DETAIL part must fire (since scope-is matches).
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "detailed"; utterance = "힘은 뭐야"; }"#,
    )
    .expect("detailed scope with lowercase scope-is should evaluate");
  let transcript: Vec<&String> = response
    .envelope
    .notes
    .iter()
    .filter(|n| n.starts_with("transcript:pnix:"))
    .collect();
  assert!(
    transcript.iter().any(|n| n.contains("DETAIL")),
    "DETAIL part missing in {transcript:?}"
  );
}

#[test]
fn kernel_base_facts_concept_source_facts_section_is_required() {
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("missing concept-source-facts section should fail");
  assert!(
    err.to_string().contains("missing 'concept-source-facts'"),
    "{err:#}"
  );
}

#[test]
fn kernel_base_facts_concept_source_facts_reject_unsupported_placeholders() {
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}.${utterance}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("unsupported placeholder in concept-source-facts should fail");
  assert!(
    err
      .to_string()
      .contains("unsupported placeholder '${utterance}'"),
    "{err:#}"
  );
  assert!(
    err
      .to_string()
      .contains("concept-source-facts scalar-id-template"),
    "{err:#}"
  );
}

#[test]
fn kernel_base_facts_concept_source_facts_runtime_shape_is_owned_by_px() {
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "custom.concept.${term}.${predicate}";
        list-id-template = "custom.concept.${term}.${predicate}.${index}";
        provenance-template = "custom-provenance:${source-ref}";
      };
      note-templates = {
        transcript-user = "transcript:user: ${utterance}";
        transcript-pnix = "transcript:pnix: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
      query-provenance-templates = {
        utterance = "utterance:${utterance}";
        concept-source = "concept-source:${source-ref}";
      };
      semantic-id-templates = {
        episode-id-template = "episode.pnix.standalone.${counter}";
        record-id-template = "record.fact.${episode-id}.${index}";
        knowledge-id-template = "knowledge.pnix.${episode-id}";
        knowledge-summary = "pnix standalone query kernel staging record";
      };
      pipeline-trace-note-prefixes = ["ontology-" "truth-regime:" "held-"];
      transcript-note-prefix = "transcript:";
      output-fragment-templates = {
        pipeline-trace = { kind = "pipeline-trace"; visibility = "dev"; };
        response-document = { kind = "response-document"; visibility = "dev"; };
      };
      response-document-schema = {
        px-header-comment = "# pnix ontology response document (auto-generated)";
        px-field-episode-id = "episode-id";
        px-field-summary = "summary";
        px-field-transcript = "transcript";
        px-field-pipeline = "pipeline";
        px-field-facts-count = "facts-count";
        org-title = "* Ontology Response";
        org-pipeline-section-header = "** Pipeline";
        org-facts-count-template = "- Facts: ${count}";
        org-transcript-transforms = [
          { input-prefix = "user: "; output-prefix = "** Q: "; }
          { input-prefix = "pnix: "; output-prefix = ""; }
        ];
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect("should evaluate");

  use pnix_core::ontology::SemanticRecordValue;
  let concept_facts: Vec<_> = response
    .envelope
    .records
    .iter()
    .filter_map(|r| match &r.value {
      SemanticRecordValue::ContextualFact(f) => Some(f.clone()),
      _ => None,
    })
    .filter(|f| {
      f.id
        .as_ref()
        .map(|id| id.0.starts_with("custom.concept.힘."))
        .unwrap_or(false)
    })
    .collect();
  assert!(
    !concept_facts.is_empty(),
    "no custom-prefixed concept facts in {:#?}",
    response.envelope.records
  );
  let has_custom_provenance = concept_facts.iter().any(|f| {
    f.provenance_refs
      .iter()
      .any(|p| p.starts_with("custom-provenance:"))
  });
  assert!(has_custom_provenance, "{concept_facts:#?}");
}

#[test]
fn kernel_base_facts_note_templates_section_is_required() {
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      transcript-note-prefix = "transcript:";
      output-fragment-templates = {
        pipeline-trace = { kind = "pipeline-trace"; visibility = "dev"; };
        response-document = { kind = "response-document"; visibility = "dev"; };
      };
      response-document-schema = {
        px-header-comment = "# pnix ontology response document (auto-generated)";
        px-field-episode-id = "episode-id";
        px-field-summary = "summary";
        px-field-transcript = "transcript";
        px-field-pipeline = "pipeline";
        px-field-facts-count = "facts-count";
        org-title = "* Ontology Response";
        org-pipeline-section-header = "** Pipeline";
        org-facts-count-template = "- Facts: ${count}";
        org-transcript-transforms = [
          { input-prefix = "user: "; output-prefix = "** Q: "; }
          { input-prefix = "pnix: "; output-prefix = ""; }
        ];
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("missing note-templates section should fail");
  assert!(
    err.to_string().contains("missing 'note-templates'"),
    "{err:#}"
  );
}

#[test]
fn kernel_base_facts_note_templates_enforce_consumer_prefix_contract() {
  // truth-regime consumer contract: build_response_documents pipeline-trace
  // filter uses note.starts_with("truth-regime:"). If the .px template does
  // not start with that prefix, loader must reject so the consumer doesn't
  // silently drop the note.
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      note-templates = {
        transcript-user = "transcript:user: ${utterance}";
        transcript-pnix = "transcript:pnix: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "BAD-PREFIX:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
      transcript-note-prefix = "transcript:";
      output-fragment-templates = {
        pipeline-trace = { kind = "pipeline-trace"; visibility = "dev"; };
        response-document = { kind = "response-document"; visibility = "dev"; };
      };
      response-document-schema = {
        px-header-comment = "# pnix ontology response document (auto-generated)";
        px-field-episode-id = "episode-id";
        px-field-summary = "summary";
        px-field-transcript = "transcript";
        px-field-pipeline = "pipeline";
        px-field-facts-count = "facts-count";
        org-title = "* Ontology Response";
        org-pipeline-section-header = "** Pipeline";
        org-facts-count-template = "- Facts: ${count}";
        org-transcript-transforms = [
          { input-prefix = "user: "; output-prefix = "** Q: "; }
          { input-prefix = "pnix: "; output-prefix = ""; }
        ];
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("truth-regime template must start with 'truth-regime:'");
  let message = err.to_string();
  assert!(
    message.contains("note-templates 'truth-regime'"),
    "{message}"
  );
  assert!(
    message.contains("must start with 'truth-regime:'"),
    "{message}"
  );
}

#[test]
fn kernel_base_facts_note_templates_reject_unsupported_placeholder() {
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      note-templates = {
        transcript-user = "transcript:user: ${utterance} ${bogus}";
        transcript-pnix = "transcript:pnix: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
      transcript-note-prefix = "transcript:";
      output-fragment-templates = {
        pipeline-trace = { kind = "pipeline-trace"; visibility = "dev"; };
        response-document = { kind = "response-document"; visibility = "dev"; };
      };
      response-document-schema = {
        px-header-comment = "# pnix ontology response document (auto-generated)";
        px-field-episode-id = "episode-id";
        px-field-summary = "summary";
        px-field-transcript = "transcript";
        px-field-pipeline = "pipeline";
        px-field-facts-count = "facts-count";
        org-title = "* Ontology Response";
        org-pipeline-section-header = "** Pipeline";
        org-facts-count-template = "- Facts: ${count}";
        org-transcript-transforms = [
          { input-prefix = "user: "; output-prefix = "** Q: "; }
          { input-prefix = "pnix: "; output-prefix = ""; }
        ];
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("bogus placeholder should fail");
  assert!(
    err
      .to_string()
      .contains("unsupported placeholder '${bogus}'"),
    "{err:#}"
  );
}

#[test]
fn kernel_base_facts_note_templates_runtime_shape_is_owned_by_px() {
  // custom note template bodies should flow through to the runtime notes,
  // so long as the consumer prefix contract is satisfied.
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      note-templates = {
        transcript-user = "transcript:user-custom | ${utterance}";
        transcript-pnix = "transcript:pnix-custom | ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query-custom:${predicate}";
      };
      query-provenance-templates = {
        utterance = "utterance:${utterance}";
        concept-source = "concept-source:${source-ref}";
      };
      semantic-id-templates = {
        episode-id-template = "episode.pnix.standalone.${counter}";
        record-id-template = "record.fact.${episode-id}.${index}";
        knowledge-id-template = "knowledge.pnix.${episode-id}";
        knowledge-summary = "pnix standalone query kernel staging record";
      };
      pipeline-trace-note-prefixes = ["ontology-" "truth-regime:" "held-"];
      transcript-note-prefix = "transcript:";
      output-fragment-templates = {
        pipeline-trace = { kind = "pipeline-trace"; visibility = "dev"; };
        response-document = { kind = "response-document"; visibility = "dev"; };
      };
      response-document-schema = {
        px-header-comment = "# pnix ontology response document (auto-generated)";
        px-field-episode-id = "episode-id";
        px-field-summary = "summary";
        px-field-transcript = "transcript";
        px-field-pipeline = "pipeline";
        px-field-facts-count = "facts-count";
        org-title = "* Ontology Response";
        org-pipeline-section-header = "** Pipeline";
        org-facts-count-template = "- Facts: ${count}";
        org-transcript-transforms = [
          { input-prefix = "user: "; output-prefix = "** Q: "; }
          { input-prefix = "pnix: "; output-prefix = ""; }
        ];
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect("should evaluate");
  assert!(
    response
      .envelope
      .notes
      .iter()
      .any(|n| n.starts_with("transcript:user-custom | 힘은 뭐야")),
    "notes: {:#?}",
    response.envelope.notes
  );
  assert!(
    response
      .envelope
      .notes
      .iter()
      .any(|n| n.starts_with("transcript:pnix-custom | ")),
    "notes: {:#?}",
    response.envelope.notes
  );
}

#[test]
fn kernel_base_facts_query_provenance_templates_section_is_required() {
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      note-templates = {
        transcript-user = "transcript:user: ${utterance}";
        transcript-pnix = "transcript:pnix: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
      transcript-note-prefix = "transcript:";
      output-fragment-templates = {
        pipeline-trace = { kind = "pipeline-trace"; visibility = "dev"; };
        response-document = { kind = "response-document"; visibility = "dev"; };
      };
      response-document-schema = {
        px-header-comment = "# pnix ontology response document (auto-generated)";
        px-field-episode-id = "episode-id";
        px-field-summary = "summary";
        px-field-transcript = "transcript";
        px-field-pipeline = "pipeline";
        px-field-facts-count = "facts-count";
        org-title = "* Ontology Response";
        org-pipeline-section-header = "** Pipeline";
        org-facts-count-template = "- Facts: ${count}";
        org-transcript-transforms = [
          { input-prefix = "user: "; output-prefix = "** Q: "; }
          { input-prefix = "pnix: "; output-prefix = ""; }
        ];
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("missing query-provenance-templates should fail");
  assert!(
    err
      .to_string()
      .contains("missing 'query-provenance-templates'"),
    "{err:#}"
  );
}

#[test]
fn kernel_base_facts_query_provenance_templates_reject_unsupported_placeholder() {
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      note-templates = {
        transcript-user = "transcript:user: ${utterance}";
        transcript-pnix = "transcript:pnix: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
      query-provenance-templates = {
        utterance = "utterance:${route}";
        concept-source = "concept-source:${source-ref}";
      };
      transcript-note-prefix = "transcript:";
      output-fragment-templates = {
        pipeline-trace = { kind = "pipeline-trace"; visibility = "dev"; };
        response-document = { kind = "response-document"; visibility = "dev"; };
      };
      response-document-schema = {
        px-header-comment = "# pnix ontology response document (auto-generated)";
        px-field-episode-id = "episode-id";
        px-field-summary = "summary";
        px-field-transcript = "transcript";
        px-field-pipeline = "pipeline";
        px-field-facts-count = "facts-count";
        org-title = "* Ontology Response";
        org-pipeline-section-header = "** Pipeline";
        org-facts-count-template = "- Facts: ${count}";
        org-transcript-transforms = [
          { input-prefix = "user: "; output-prefix = "** Q: "; }
          { input-prefix = "pnix: "; output-prefix = ""; }
        ];
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("non-allowed placeholder should fail");
  assert!(
    err
      .to_string()
      .contains("unsupported placeholder '${route}'"),
    "{err:#}"
  );
  assert!(
    err
      .to_string()
      .contains("query-provenance-templates 'utterance'"),
    "{err:#}"
  );
}

#[test]
fn kernel_base_facts_query_provenance_templates_runtime_shape_is_owned_by_px() {
  // custom provenance bodies must flow through to the base-fact
  // provenance_refs, proving the runtime actually uses the .px template.
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      note-templates = {
        transcript-user = "transcript:user: ${utterance}";
        transcript-pnix = "transcript:pnix: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
      query-provenance-templates = {
        utterance = "custom-utterance|${utterance}";
        concept-source = "custom-source|${source-ref}";
      };
      semantic-id-templates = {
        episode-id-template = "episode.pnix.standalone.${counter}";
        record-id-template = "record.fact.${episode-id}.${index}";
        knowledge-id-template = "knowledge.pnix.${episode-id}";
        knowledge-summary = "pnix standalone query kernel staging record";
      };
      pipeline-trace-note-prefixes = ["ontology-" "truth-regime:" "held-"];
      transcript-note-prefix = "transcript:";
      output-fragment-templates = {
        pipeline-trace = { kind = "pipeline-trace"; visibility = "dev"; };
        response-document = { kind = "response-document"; visibility = "dev"; };
      };
      response-document-schema = {
        px-header-comment = "# pnix ontology response document (auto-generated)";
        px-field-episode-id = "episode-id";
        px-field-summary = "summary";
        px-field-transcript = "transcript";
        px-field-pipeline = "pipeline";
        px-field-facts-count = "facts-count";
        org-title = "* Ontology Response";
        org-pipeline-section-header = "** Pipeline";
        org-facts-count-template = "- Facts: ${count}";
        org-transcript-transforms = [
          { input-prefix = "user: "; output-prefix = "** Q: "; }
          { input-prefix = "pnix: "; output-prefix = ""; }
        ];
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect("should evaluate");
  use pnix_core::ontology::SemanticRecordValue;
  let all_provenance: Vec<String> = response
    .envelope
    .records
    .iter()
    .filter_map(|r| match &r.value {
      SemanticRecordValue::ContextualFact(f) => Some(f.provenance_refs.clone()),
      _ => None,
    })
    .flatten()
    .collect();
  assert!(
    all_provenance
      .iter()
      .any(|p| p.starts_with("custom-utterance|힘은 뭐야")),
    "missing custom-utterance provenance: {all_provenance:#?}"
  );
  assert!(
    all_provenance
      .iter()
      .any(|p| p.starts_with("custom-source|")),
    "missing custom-source provenance: {all_provenance:#?}"
  );
  // envelope observation_refs should also carry the custom utterance.
  assert!(
    response
      .envelope
      .observation_refs
      .iter()
      .any(|o| o.starts_with("custom-utterance|")),
    "observation_refs: {:#?}",
    response.envelope.observation_refs
  );
}

#[test]
fn kernel_base_facts_semantic_id_templates_section_is_required() {
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      note-templates = {
        transcript-user = "transcript:user: ${utterance}";
        transcript-pnix = "transcript:pnix: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
      query-provenance-templates = {
        utterance = "utterance:${utterance}";
        concept-source = "concept-source:${source-ref}";
      };
      pipeline-trace-note-prefixes = ["ontology-" "truth-regime:" "held-"];
      transcript-note-prefix = "transcript:";
      output-fragment-templates = {
        pipeline-trace = { kind = "pipeline-trace"; visibility = "dev"; };
        response-document = { kind = "response-document"; visibility = "dev"; };
      };
      response-document-schema = {
        px-header-comment = "# pnix ontology response document (auto-generated)";
        px-field-episode-id = "episode-id";
        px-field-summary = "summary";
        px-field-transcript = "transcript";
        px-field-pipeline = "pipeline";
        px-field-facts-count = "facts-count";
        org-title = "* Ontology Response";
        org-pipeline-section-header = "** Pipeline";
        org-facts-count-template = "- Facts: ${count}";
        org-transcript-transforms = [
          { input-prefix = "user: "; output-prefix = "** Q: "; }
          { input-prefix = "pnix: "; output-prefix = ""; }
        ];
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("missing semantic-id-templates should fail");
  assert!(
    err.to_string().contains("missing 'semantic-id-templates'"),
    "{err:#}"
  );
}

#[test]
fn kernel_base_facts_semantic_id_templates_reject_unsupported_placeholder() {
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      note-templates = {
        transcript-user = "transcript:user: ${utterance}";
        transcript-pnix = "transcript:pnix: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
      query-provenance-templates = {
        utterance = "utterance:${utterance}";
        concept-source = "concept-source:${source-ref}";
      };
      semantic-id-templates = {
        episode-id-template = "episode.pnix.${utterance}";
        record-id-template = "record.fact.${episode-id}.${index}";
        knowledge-id-template = "knowledge.pnix.${episode-id}";
        knowledge-summary = "pnix standalone query kernel staging record";
      };
      pipeline-trace-note-prefixes = ["ontology-" "truth-regime:" "held-"];
      transcript-note-prefix = "transcript:";
      output-fragment-templates = {
        pipeline-trace = { kind = "pipeline-trace"; visibility = "dev"; };
        response-document = { kind = "response-document"; visibility = "dev"; };
      };
      response-document-schema = {
        px-header-comment = "# pnix ontology response document (auto-generated)";
        px-field-episode-id = "episode-id";
        px-field-summary = "summary";
        px-field-transcript = "transcript";
        px-field-pipeline = "pipeline";
        px-field-facts-count = "facts-count";
        org-title = "* Ontology Response";
        org-pipeline-section-header = "** Pipeline";
        org-facts-count-template = "- Facts: ${count}";
        org-transcript-transforms = [
          { input-prefix = "user: "; output-prefix = "** Q: "; }
          { input-prefix = "pnix: "; output-prefix = ""; }
        ];
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("non-allowed placeholder should fail");
  assert!(
    err
      .to_string()
      .contains("unsupported placeholder '${utterance}'"),
    "{err:#}"
  );
  assert!(
    err
      .to_string()
      .contains("semantic-id-templates 'episode-id-template'"),
    "{err:#}"
  );
}

#[test]
fn kernel_base_facts_semantic_id_templates_runtime_shape_is_owned_by_px() {
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      note-templates = {
        transcript-user = "transcript:user: ${utterance}";
        transcript-pnix = "transcript:pnix: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
      query-provenance-templates = {
        utterance = "utterance:${utterance}";
        concept-source = "concept-source:${source-ref}";
      };
      semantic-id-templates = {
        episode-id-template = "custom-episode.${counter}";
        record-id-template = "custom-record.${episode-id}.${index}";
        knowledge-id-template = "custom-knowledge.${episode-id}";
        knowledge-summary = "custom-staging-summary";
      };
      pipeline-trace-note-prefixes = ["ontology-" "truth-regime:" "held-"];
      transcript-note-prefix = "transcript:";
      output-fragment-templates = {
        pipeline-trace = { kind = "pipeline-trace"; visibility = "dev"; };
        response-document = { kind = "response-document"; visibility = "dev"; };
      };
      response-document-schema = {
        px-header-comment = "# pnix ontology response document (auto-generated)";
        px-field-episode-id = "episode-id";
        px-field-summary = "summary";
        px-field-transcript = "transcript";
        px-field-pipeline = "pipeline";
        px-field-facts-count = "facts-count";
        org-title = "* Ontology Response";
        org-pipeline-section-header = "** Pipeline";
        org-facts-count-template = "- Facts: ${count}";
        org-transcript-transforms = [
          { input-prefix = "user: "; output-prefix = "** Q: "; }
          { input-prefix = "pnix: "; output-prefix = ""; }
        ];
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect("should evaluate");
  assert!(
    response
      .envelope
      .episode
      .id
      .0
      .starts_with("custom-episode."),
    "episode id: {}",
    response.envelope.episode.id.0
  );
  assert!(
    response
      .envelope
      .records
      .iter()
      .any(|r| r.id.0.starts_with("custom-record.")),
    "record ids: {:#?}",
    response
      .envelope
      .records
      .iter()
      .map(|r| r.id.0.as_str())
      .collect::<Vec<_>>()
  );
  assert!(
    response
      .envelope
      .knowledge_records
      .iter()
      .any(|k| k.id.0.starts_with("custom-knowledge.")
        && k.summary.as_deref() == Some("custom-staging-summary")),
    "knowledge records: {:#?}",
    response.envelope.knowledge_records
  );
}

#[test]
fn kernel_base_facts_pipeline_trace_prefixes_must_not_be_empty() {
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      note-templates = {
        transcript-user = "transcript:user: ${utterance}";
        transcript-pnix = "transcript:pnix: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
      query-provenance-templates = {
        utterance = "utterance:${utterance}";
        concept-source = "concept-source:${source-ref}";
      };
      semantic-id-templates = {
        episode-id-template = "episode.pnix.standalone.${counter}";
        record-id-template = "record.fact.${episode-id}.${index}";
        knowledge-id-template = "knowledge.pnix.${episode-id}";
        knowledge-summary = "pnix standalone query kernel staging record";
      };
      pipeline-trace-note-prefixes = [];
      transcript-note-prefix = "transcript:";
      output-fragment-templates = {
        pipeline-trace = { kind = "pipeline-trace"; visibility = "dev"; };
        response-document = { kind = "response-document"; visibility = "dev"; };
      };
      response-document-schema = {
        px-header-comment = "# pnix ontology response document (auto-generated)";
        px-field-episode-id = "episode-id";
        px-field-summary = "summary";
        px-field-transcript = "transcript";
        px-field-pipeline = "pipeline";
        px-field-facts-count = "facts-count";
        org-title = "* Ontology Response";
        org-pipeline-section-header = "** Pipeline";
        org-facts-count-template = "- Facts: ${count}";
        org-transcript-transforms = [
          { input-prefix = "user: "; output-prefix = "** Q: "; }
          { input-prefix = "pnix: "; output-prefix = ""; }
        ];
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("empty pipeline-trace-note-prefixes should fail");
  assert!(
    err
      .to_string()
      .contains("'pipeline-trace-note-prefixes' must have at least one entry"),
    "{err:#}"
  );
}

#[test]
fn kernel_base_facts_pipeline_trace_prefixes_runtime_shape_is_owned_by_px() {
  // Make the pipeline-trace prefix list custom: only "ontology-". The held
  // lane emits "held-reason:..." notes, and those must NOT appear in the
  // pipeline-trace output fragment when the .px restricts the prefix list.
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      note-templates = {
        transcript-user = "transcript:user: ${utterance}";
        transcript-pnix = "transcript:pnix: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
      query-provenance-templates = {
        utterance = "utterance:${utterance}";
        concept-source = "concept-source:${source-ref}";
      };
      semantic-id-templates = {
        episode-id-template = "episode.pnix.standalone.${counter}";
        record-id-template = "record.fact.${episode-id}.${index}";
        knowledge-id-template = "knowledge.pnix.${episode-id}";
        knowledge-summary = "pnix standalone query kernel staging record";
      };
      pipeline-trace-note-prefixes = ["ontology-"];
      transcript-note-prefix = "transcript:";
      output-fragment-templates = {
        pipeline-trace = { kind = "pipeline-trace"; visibility = "dev"; };
        response-document = { kind = "response-document"; visibility = "dev"; };
      };
      response-document-schema = {
        px-header-comment = "# pnix ontology response document (auto-generated)";
        px-field-episode-id = "episode-id";
        px-field-summary = "summary";
        px-field-transcript = "transcript";
        px-field-pipeline = "pipeline";
        px-field-facts-count = "facts-count";
        org-title = "* Ontology Response";
        org-pipeline-section-header = "** Pipeline";
        org-facts-count-template = "- Facts: ${count}";
        org-transcript-transforms = [
          { input-prefix = "user: "; output-prefix = "** Q: "; }
          { input-prefix = "pnix: "; output-prefix = ""; }
        ];
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  // Unknown term → held lane. Generates "held-reason:..." notes in envelope.
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "zzznonexistent 뭐야"; }"#,
    )
    .expect("should evaluate");

  // envelope.notes still carries the held-reason note (emission side).
  assert!(
    response
      .envelope
      .notes
      .iter()
      .any(|n| n.starts_with("held-reason:")),
    "envelope should carry held-reason note: {:#?}",
    response.envelope.notes
  );
  // pipeline-trace output fragment must NOT include held- lines, because
  // the .px prefix list was restricted to "ontology-" only.
  for fragment in &response.output_fragments {
    if fragment.kind == "pipeline-trace" {
      assert!(
        !fragment.content_org.contains("held-"),
        "pipeline-trace fragment should not contain 'held-' when the prefix list is restricted: {:#?}",
        fragment
      );
    }
  }
}

#[test]
fn kernel_base_facts_transcript_note_prefix_is_required() {
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("missing transcript-note-prefix should fail");
  assert!(
    err.to_string().contains("missing 'transcript-note-prefix'"),
    "{err:#}"
  );
}

#[test]
fn kernel_base_facts_transcript_note_prefix_enforces_note_templates_contract() {
  // transcript-note-prefix = "talk:" 인데 note-templates.transcript-user 가
  // 여전히 "transcript:" 로 시작하면 loader 가 consumer contract 불일치로 거부.
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      transcript-note-prefix = "talk:";
      note-templates = {
        transcript-user = "transcript:user: ${utterance}";
        transcript-pnix = "transcript:pnix: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("transcript-* must start with transcript-note-prefix");
  let message = err.to_string();
  assert!(
    message.contains("note-templates 'transcript-user'"),
    "{message}"
  );
  assert!(message.contains("must start with 'talk:'"), "{message}");
}

#[test]
fn kernel_base_facts_transcript_note_prefix_runtime_shape_is_owned_by_px() {
  // custom transcript prefix "talk:" end-to-end: envelope 의 notes 와
  // transcript 에 이 prefix 가 실제로 반영되는지 확인.
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      transcript-note-prefix = "talk:";
      note-templates = {
        transcript-user = "talk:user: ${utterance}";
        transcript-pnix = "talk:pnix: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
      query-provenance-templates = {
        utterance = "utterance:${utterance}";
        concept-source = "concept-source:${source-ref}";
      };
      semantic-id-templates = {
        episode-id-template = "episode.pnix.standalone.${counter}";
        record-id-template = "record.fact.${episode-id}.${index}";
        knowledge-id-template = "knowledge.pnix.${episode-id}";
        knowledge-summary = "pnix standalone query kernel staging record";
      };
      pipeline-trace-note-prefixes = ["ontology-" "truth-regime:" "held-"];
      output-fragment-templates = {
        pipeline-trace = { kind = "pipeline-trace"; visibility = "dev"; };
        response-document = { kind = "response-document"; visibility = "dev"; };
      };
      response-document-schema = {
        px-header-comment = "# pnix ontology response document (auto-generated)";
        px-field-episode-id = "episode-id";
        px-field-summary = "summary";
        px-field-transcript = "transcript";
        px-field-pipeline = "pipeline";
        px-field-facts-count = "facts-count";
        org-title = "* Ontology Response";
        org-pipeline-section-header = "** Pipeline";
        org-facts-count-template = "- Facts: ${count}";
        org-transcript-transforms = [
          { input-prefix = "user: "; output-prefix = "** Q: "; }
          { input-prefix = "pnix: "; output-prefix = ""; }
        ];
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect("should evaluate");

  // envelope.notes 에 새 prefix 가 실려 있음
  assert!(
    response
      .envelope
      .notes
      .iter()
      .any(|n| n.starts_with("talk:user: 힘은 뭐야")),
    "notes: {:#?}",
    response.envelope.notes
  );
  // transcript_from_notes 가 새 prefix 를 strip_prefix 해서 본문만 남김
  assert!(
    response
      .transcript
      .iter()
      .any(|t| t.starts_with("user: 힘은 뭐야")),
    "transcript: {:#?}",
    response.transcript
  );
}

#[test]
fn kernel_base_facts_output_fragment_templates_section_is_required() {
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      transcript-note-prefix = "transcript:";
      note-templates = {
        transcript-user = "transcript:user: ${utterance}";
        transcript-pnix = "transcript:pnix: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
      query-provenance-templates = {
        utterance = "utterance:${utterance}";
        concept-source = "concept-source:${source-ref}";
      };
      semantic-id-templates = {
        episode-id-template = "episode.pnix.standalone.${counter}";
        record-id-template = "record.fact.${episode-id}.${index}";
        knowledge-id-template = "knowledge.pnix.${episode-id}";
        knowledge-summary = "pnix standalone query kernel staging record";
      };
      pipeline-trace-note-prefixes = ["ontology-" "truth-regime:" "held-"];
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("missing output-fragment-templates should fail");
  assert!(
    err
      .to_string()
      .contains("missing 'output-fragment-templates'"),
    "{err:#}"
  );
}

#[test]
fn kernel_base_facts_output_fragment_templates_require_pipeline_trace_and_response_document() {
  // pipeline-trace 섹션은 있지만 response-document 섹션이 없다.
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      transcript-note-prefix = "transcript:";
      note-templates = {
        transcript-user = "transcript:user: ${utterance}";
        transcript-pnix = "transcript:pnix: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
      query-provenance-templates = {
        utterance = "utterance:${utterance}";
        concept-source = "concept-source:${source-ref}";
      };
      semantic-id-templates = {
        episode-id-template = "episode.pnix.standalone.${counter}";
        record-id-template = "record.fact.${episode-id}.${index}";
        knowledge-id-template = "knowledge.pnix.${episode-id}";
        knowledge-summary = "pnix standalone query kernel staging record";
      };
      pipeline-trace-note-prefixes = ["ontology-" "truth-regime:" "held-"];
      output-fragment-templates = {
        pipeline-trace = { kind = "pipeline-trace"; visibility = "dev"; };
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("missing response-document entry should fail");
  assert!(
    err
      .to_string()
      .contains("missing 'output-fragment-templates.response-document'"),
    "{err:#}"
  );
}

#[test]
fn kernel_base_facts_output_fragment_templates_runtime_shape_is_owned_by_px() {
  // custom kind/visibility 값이 runtime 이 만드는 KernelOutputFragment 에 실제로 반영되는지 확인.
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      transcript-note-prefix = "transcript:";
      note-templates = {
        transcript-user = "transcript:user: ${utterance}";
        transcript-pnix = "transcript:pnix: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
      query-provenance-templates = {
        utterance = "utterance:${utterance}";
        concept-source = "concept-source:${source-ref}";
      };
      semantic-id-templates = {
        episode-id-template = "episode.pnix.standalone.${counter}";
        record-id-template = "record.fact.${episode-id}.${index}";
        knowledge-id-template = "knowledge.pnix.${episode-id}";
        knowledge-summary = "pnix standalone query kernel staging record";
      };
      pipeline-trace-note-prefixes = ["ontology-" "truth-regime:" "held-"];
      output-fragment-templates = {
        pipeline-trace = { kind = "custom-trace"; visibility = "public"; };
        response-document = { kind = "custom-response"; visibility = "all"; };
      };
      response-document-schema = {
        px-header-comment = "# pnix ontology response document (auto-generated)";
        px-field-episode-id = "episode-id";
        px-field-summary = "summary";
        px-field-transcript = "transcript";
        px-field-pipeline = "pipeline";
        px-field-facts-count = "facts-count";
        org-title = "* Ontology Response";
        org-pipeline-section-header = "** Pipeline";
        org-facts-count-template = "- Facts: ${count}";
        org-transcript-transforms = [
          { input-prefix = "user: "; output-prefix = "** Q: "; }
          { input-prefix = "pnix: "; output-prefix = ""; }
        ];
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  // held 가 아닌 known-term 을 피하려고 알 수 없는 단어. held lane 도 OK.
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "zzznonexistent 뭐야"; }"#,
    )
    .expect("should evaluate");
  let has_custom_response = response
    .output_fragments
    .iter()
    .any(|f| f.kind == "custom-response" && f.visibility == "all");
  assert!(
    has_custom_response,
    "response-document fragment 가 custom kind/visibility 를 쓰지 않음: {:#?}",
    response.output_fragments
  );
}

#[test]
fn kernel_base_facts_response_document_schema_section_is_required() {
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      transcript-note-prefix = "transcript:";
      note-templates = {
        transcript-user = "transcript:user: ${utterance}";
        transcript-pnix = "transcript:pnix: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
      query-provenance-templates = {
        utterance = "utterance:${utterance}";
        concept-source = "concept-source:${source-ref}";
      };
      semantic-id-templates = {
        episode-id-template = "episode.pnix.standalone.${counter}";
        record-id-template = "record.fact.${episode-id}.${index}";
        knowledge-id-template = "knowledge.pnix.${episode-id}";
        knowledge-summary = "pnix standalone query kernel staging record";
      };
      pipeline-trace-note-prefixes = ["ontology-" "truth-regime:" "held-"];
      output-fragment-templates = {
        pipeline-trace = { kind = "pipeline-trace"; visibility = "dev"; };
        response-document = { kind = "response-document"; visibility = "dev"; };
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("missing response-document-schema should fail");
  assert!(
    err
      .to_string()
      .contains("missing 'response-document-schema'"),
    "{err:#}"
  );
}

#[test]
fn kernel_base_facts_response_document_schema_org_facts_count_template_allowlist() {
  // org-facts-count-template allowlist 는 ${count} 만. ${utterance} 등은 거부.
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      transcript-note-prefix = "transcript:";
      note-templates = {
        transcript-user = "transcript:user: ${utterance}";
        transcript-pnix = "transcript:pnix: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
      query-provenance-templates = {
        utterance = "utterance:${utterance}";
        concept-source = "concept-source:${source-ref}";
      };
      semantic-id-templates = {
        episode-id-template = "episode.pnix.standalone.${counter}";
        record-id-template = "record.fact.${episode-id}.${index}";
        knowledge-id-template = "knowledge.pnix.${episode-id}";
        knowledge-summary = "pnix standalone query kernel staging record";
      };
      pipeline-trace-note-prefixes = ["ontology-" "truth-regime:" "held-"];
      output-fragment-templates = {
        pipeline-trace = { kind = "pipeline-trace"; visibility = "dev"; };
        response-document = { kind = "response-document"; visibility = "dev"; };
      };
      response-document-schema = {
        px-header-comment = "# pnix ontology response document (auto-generated)";
        px-field-episode-id = "episode-id";
        px-field-summary = "summary";
        px-field-transcript = "transcript";
        px-field-pipeline = "pipeline";
        px-field-facts-count = "facts-count";
        org-title = "* Ontology Response";
        org-pipeline-section-header = "** Pipeline";
        org-facts-count-template = "- Facts: ${count} ${utterance}";
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("non-allowed placeholder in org-facts-count-template should fail");
  assert!(
    err
      .to_string()
      .contains("unsupported placeholder '${utterance}'"),
    "{err:#}"
  );
  assert!(
    err
      .to_string()
      .contains("response-document-schema 'org-facts-count-template'"),
    "{err:#}"
  );
}

#[test]
fn kernel_base_facts_response_document_schema_runtime_shape_is_owned_by_px() {
  // custom PX 필드 이름 / org 마크업이 runtime content_px / content_org 에
  // 실제로 반영되는지 end-to-end 확인.
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      transcript-note-prefix = "transcript:";
      note-templates = {
        transcript-user = "transcript:user: ${utterance}";
        transcript-pnix = "transcript:pnix: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
      query-provenance-templates = {
        utterance = "utterance:${utterance}";
        concept-source = "concept-source:${source-ref}";
      };
      semantic-id-templates = {
        episode-id-template = "episode.pnix.standalone.${counter}";
        record-id-template = "record.fact.${episode-id}.${index}";
        knowledge-id-template = "knowledge.pnix.${episode-id}";
        knowledge-summary = "pnix standalone query kernel staging record";
      };
      pipeline-trace-note-prefixes = ["ontology-" "truth-regime:" "held-"];
      output-fragment-templates = {
        pipeline-trace = { kind = "pipeline-trace"; visibility = "dev"; };
        response-document = { kind = "response-document"; visibility = "dev"; };
      };
      response-document-schema = {
        px-header-comment = "# custom-header";
        px-field-episode-id = "ep";
        px-field-summary = "sum";
        px-field-transcript = "tr";
        px-field-pipeline = "pl";
        px-field-facts-count = "fc";
        org-title = "* Custom Title";
        org-pipeline-section-header = "** CustomPipeline";
        org-facts-count-template = "- CustomFacts: ${count}";
        org-transcript-transforms = [
          { input-prefix = "user: "; output-prefix = "** Q: "; }
          { input-prefix = "pnix: "; output-prefix = ""; }
        ];
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect("should evaluate");

  // response-document fragment 찾기
  let response_doc = response
    .output_fragments
    .iter()
    .find(|f| f.kind == "response-document")
    .expect("response-document fragment missing");
  let px = response_doc.content_px.as_deref().unwrap_or("");
  let org = response_doc.content_org.as_str();

  assert!(
    px.starts_with("# custom-header\n{\n"),
    "custom px header 가 반영되지 않음: {px}"
  );
  assert!(
    px.contains("ep = \""),
    "custom px field episode 반영 안 됨: {px}"
  );
  assert!(
    px.contains("sum = \""),
    "custom px field summary 반영 안 됨: {px}"
  );
  assert!(
    px.contains("tr = ["),
    "custom px field transcript 반영 안 됨: {px}"
  );
  assert!(
    px.contains("pl = ["),
    "custom px field pipeline 반영 안 됨: {px}"
  );
  assert!(
    px.contains("fc = "),
    "custom px field facts-count 반영 안 됨: {px}"
  );
  assert!(
    org.starts_with("* Custom Title\n"),
    "custom org title 반영 안 됨: {org}"
  );
}

#[test]
fn kernel_base_facts_response_document_schema_org_transcript_transforms_is_required() {
  // org-transcript-transforms 섹션 자체가 비어 있으면 load 거부.
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      transcript-note-prefix = "transcript:";
      note-templates = {
        transcript-user = "transcript:user: ${utterance}";
        transcript-pnix = "transcript:pnix: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
      query-provenance-templates = {
        utterance = "utterance:${utterance}";
        concept-source = "concept-source:${source-ref}";
      };
      semantic-id-templates = {
        episode-id-template = "episode.pnix.standalone.${counter}";
        record-id-template = "record.fact.${episode-id}.${index}";
        knowledge-id-template = "knowledge.pnix.${episode-id}";
        knowledge-summary = "pnix standalone query kernel staging record";
      };
      pipeline-trace-note-prefixes = ["ontology-" "truth-regime:" "held-"];
      output-fragment-templates = {
        pipeline-trace = { kind = "pipeline-trace"; visibility = "dev"; };
        response-document = { kind = "response-document"; visibility = "dev"; };
      };
      response-document-schema = {
        px-header-comment = "# pnix ontology response document (auto-generated)";
        px-field-episode-id = "episode-id";
        px-field-summary = "summary";
        px-field-transcript = "transcript";
        px-field-pipeline = "pipeline";
        px-field-facts-count = "facts-count";
        org-title = "* Ontology Response";
        org-pipeline-section-header = "** Pipeline";
        org-facts-count-template = "- Facts: ${count}";
        org-transcript-transforms = [];
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("empty org-transcript-transforms should fail");
  assert!(
    err.to_string().contains(
      "'response-document-schema.org-transcript-transforms' must have at least one entry"
    ),
    "{err:#}"
  );
}

#[test]
fn kernel_base_facts_response_document_schema_org_transcript_transforms_require_input_prefix() {
  // entry 에 input-prefix 가 비어 있으면 load 거부.
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      transcript-note-prefix = "transcript:";
      note-templates = {
        transcript-user = "transcript:user: ${utterance}";
        transcript-pnix = "transcript:pnix: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
      query-provenance-templates = {
        utterance = "utterance:${utterance}";
        concept-source = "concept-source:${source-ref}";
      };
      semantic-id-templates = {
        episode-id-template = "episode.pnix.standalone.${counter}";
        record-id-template = "record.fact.${episode-id}.${index}";
        knowledge-id-template = "knowledge.pnix.${episode-id}";
        knowledge-summary = "pnix standalone query kernel staging record";
      };
      pipeline-trace-note-prefixes = ["ontology-" "truth-regime:" "held-"];
      output-fragment-templates = {
        pipeline-trace = { kind = "pipeline-trace"; visibility = "dev"; };
        response-document = { kind = "response-document"; visibility = "dev"; };
      };
      response-document-schema = {
        px-header-comment = "# pnix ontology response document (auto-generated)";
        px-field-episode-id = "episode-id";
        px-field-summary = "summary";
        px-field-transcript = "transcript";
        px-field-pipeline = "pipeline";
        px-field-facts-count = "facts-count";
        org-title = "* Ontology Response";
        org-pipeline-section-header = "** Pipeline";
        org-facts-count-template = "- Facts: ${count}";
        org-transcript-transforms = [
          { input-prefix = ""; output-prefix = "** Q: "; }
        ];
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("empty input-prefix should fail");
  assert!(err.to_string().contains("input-prefix"), "{err:#}");
}

#[test]
fn kernel_base_facts_response_document_schema_org_transcript_transforms_runtime_shape_is_owned_by_px(
) {
  // custom transform 으로 user/pnix 대신 me/bot 역할을 .px 에서 정의하면
  // .org 에 그 값이 실제로 반영되는지 end-to-end 확인.
  // note-templates.transcript-user 도 transcript:me: ... 로 맞춰 준다.
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      transcript-note-prefix = "transcript:";
      note-templates = {
        transcript-user = "transcript:me: ${utterance}";
        transcript-pnix = "transcript:bot: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
      query-provenance-templates = {
        utterance = "utterance:${utterance}";
        concept-source = "concept-source:${source-ref}";
      };
      semantic-id-templates = {
        episode-id-template = "episode.pnix.standalone.${counter}";
        record-id-template = "record.fact.${episode-id}.${index}";
        knowledge-id-template = "knowledge.pnix.${episode-id}";
        knowledge-summary = "pnix standalone query kernel staging record";
      };
      pipeline-trace-note-prefixes = ["ontology-" "truth-regime:" "held-"];
      output-fragment-templates = {
        pipeline-trace = { kind = "pipeline-trace"; visibility = "dev"; };
        response-document = { kind = "response-document"; visibility = "dev"; };
      };
      response-document-schema = {
        px-header-comment = "# pnix ontology response document (auto-generated)";
        px-field-episode-id = "episode-id";
        px-field-summary = "summary";
        px-field-transcript = "transcript";
        px-field-pipeline = "pipeline";
        px-field-facts-count = "facts-count";
        org-title = "* Ontology Response";
        org-pipeline-section-header = "** Pipeline";
        org-facts-count-template = "- Facts: ${count}";
        org-transcript-transforms = [
          { input-prefix = "me: "; output-prefix = "** Question: "; }
          { input-prefix = "bot: "; output-prefix = ">> "; }
        ];
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect("should evaluate");

  let response_doc = response
    .output_fragments
    .iter()
    .find(|f| f.kind == "response-document")
    .expect("response-document fragment missing");
  let org = response_doc.content_org.as_str();
  // "me: 힘은 뭐야" → "** Question: 힘은 뭐야"
  assert!(
    org.contains("** Question: 힘은 뭐야"),
    "user → Question 변환이 반영되지 않음: {org}"
  );
  // bot: ... → >> ...
  assert!(org.contains(">> "), "pnix → >> 변환이 반영되지 않음: {org}");
}

#[test]
fn followup_disambiguation_questions_reject_duplicate_distinguishing_predicate() {
  let followup = temp_px_path("followup-generation.px");
  write_px(
    &followup,
    r#"
    {
      default-choices = ["파동" "입자"];
      unknown-term-label = "그 용어";
      disambiguation-questions = [
        {
          distinguishing-predicate = "experiment-context";
          question-template = "Q1: ${term}";
          choices-template = "C1";
        }
        {
          distinguishing-predicate = "experiment-context";
          question-template = "Q2: ${term}";
          choices-template = "C2";
        }
      ];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "experiment-context"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "always"; effective-utterance-template = "${term}"; }
      ];
      choice-rules = [
        { when = "always"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "always"; term-source = "term"; }
      ];
      held-response-rules = [
        { when = "always"; template = "held ${term}"; emit-held-term = "false"; }
      ];
      concept-choices = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followup;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("duplicate disambiguation-questions 엔트리는 load 에서 거부되어야 함");
  assert!(
    err
      .to_string()
      .contains("duplicate 'disambiguation-questions' entry for distinguishing-predicate 'experiment-context'"),
    "{err:#}"
  );
}

#[test]
fn followup_reason_question_rules_reject_duplicate_reason() {
  let followup = temp_px_path("followup-generation.px");
  write_px(
    &followup,
    r#"
    {
      default-choices = ["a" "b"];
      unknown-term-label = "X";
      disambiguation-questions = [
        { distinguishing-predicate = "p1"; question-template = "Q"; choices-template = "C"; }
      ];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "p1"; }
        { reason = "requires-context"; predicate = "p2"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "always"; effective-utterance-template = "${term}"; }
      ];
      choice-rules = [
        { when = "always"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "always"; term-source = "term"; }
      ];
      held-response-rules = [
        { when = "always"; template = "h"; emit-held-term = "false"; }
      ];
      concept-choices = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followup;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("duplicate reason-question-rules 엔트리는 거부되어야 함");
  assert!(
    err
      .to_string()
      .contains("duplicate 'reason-question-rules' entry for reason 'requires-context'"),
    "{err:#}"
  );
}

#[test]
fn followup_reopen_rules_reject_duplicate_reason() {
  let followup = temp_px_path("followup-generation.px");
  write_px(
    &followup,
    r#"
    {
      default-choices = ["a" "b"];
      unknown-term-label = "X";
      disambiguation-questions = [
        { distinguishing-predicate = "p1"; question-template = "Q"; choices-template = "C"; }
      ];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "p1"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "always"; effective-utterance-template = "${term}"; }
        { reason = "requires-context"; carry-term-policy = "never"; effective-utterance-template = "${term}"; }
      ];
      choice-rules = [
        { when = "always"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "always"; term-source = "term"; }
      ];
      held-response-rules = [
        { when = "always"; template = "h"; emit-held-term = "false"; }
      ];
      concept-choices = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followup;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("duplicate reopen-rules 엔트리는 거부되어야 함");
  assert!(
    err
      .to_string()
      .contains("duplicate 'reopen-rules' entry for reason 'requires-context'"),
    "{err:#}"
  );
}

#[test]
fn followup_concept_choices_reject_duplicate_term() {
  let followup = temp_px_path("followup-generation.px");
  write_px(
    &followup,
    r#"
    {
      default-choices = ["a" "b"];
      unknown-term-label = "X";
      disambiguation-questions = [
        { distinguishing-predicate = "p1"; question-template = "Q"; choices-template = "C"; }
      ];
      reason-question-rules = [
        { reason = "requires-context"; predicate = "p1"; }
      ];
      reopen-rules = [
        { reason = "requires-context"; carry-term-policy = "always"; effective-utterance-template = "${term}"; }
      ];
      choice-rules = [
        { when = "always"; choice-source = "default"; }
      ];
      resolved-term-rules = [
        { when = "always"; term-source = "term"; }
      ];
      held-response-rules = [
        { when = "always"; template = "h"; emit-held-term = "false"; }
      ];
      concept-choices = [
        { term = "빛"; choices = ["파동" "입자"]; }
        { term = "빛"; choices = ["확률파"]; }
      ];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.followup_generation_path = followup;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("duplicate concept-choices term 은 거부되어야 함");
  assert!(
    err
      .to_string()
      .contains("duplicate 'concept-choices' entry for term '빛'"),
    "{err:#}"
  );
}

#[test]
fn ontology_invert_triggers_reject_duplicate_pattern() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
        { pattern = "왜"; type = "evidence-inverse"; truth-regime = "auto"; priority = "3"; }
      ];
      domain-to-regime = [];
      invert-candidate-rules = [];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("duplicate invert-triggers pattern 은 거부되어야 함");
  assert!(
    err
      .to_string()
      .contains("duplicate 'invert-triggers' entry for pattern '왜'"),
    "{err:#}"
  );
}

#[test]
fn ontology_invert_domain_to_regime_reject_duplicate_prefix() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  let query_routes = temp_px_path("query-routes.px");
  let invert = temp_px_path("ontology-invert.px");
  write_minimal_why_kernel_fixtures(&query_classifiers, &query_routes);
  write_px(
    &invert,
    r#"
    {
      trigger-selection = "priority-then-pattern-length";
      route-template = "ontology-invert-${trigger_type}";
      default-truth-regime = "interpretive";
      default-interpretation-rule = {
        direct-fact-predicates = ["why"];
        source-include-predicates = ["causal-chain"];
        source-include-context-prefixes = ["ontology-invert."];
        direct-interpretation-id = "interp.invert.direct.${trigger_type}.${term}";
        rich-interpretation-id = "interp.invert.rich.${trigger_type}.${term}";
      };
      invert-triggers = [
        { pattern = "왜"; type = "causal-inverse"; truth-regime = "auto"; priority = "0"; }
      ];
      domain-to-regime = [
        { domain-prefix = "물리"; regime = "empirical-physical"; }
        { domain-prefix = "물리"; regime = "formal-closed"; }
      ];
      invert-candidate-rules = [];
      interpretation-rules = [];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  paths.ontology_invert_path = invert;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야"; }"#,
    )
    .expect_err("duplicate domain-to-regime domain-prefix 는 거부되어야 함");
  assert!(
    err
      .to_string()
      .contains("duplicate 'domain-to-regime' entry for domain-prefix '물리'"),
    "{err:#}"
  );
}

#[test]
fn query_classifiers_held_reason_rules_reject_duplicate_when() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "known-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["왜"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("duplicate held-reason-rules when 은 거부되어야 함");
  assert!(
    err
      .to_string()
      .contains("duplicate 'held-reason-rules' entry for when 'known-term'"),
    "{err:#}"
  );
}

#[test]
fn query_classifiers_kernel_source_fact_fields_reject_duplicate_field() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
        { field = "definition-ko"; predicate = "definition-ko-dup"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["왜"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("duplicate kernel-source-fact-fields 는 거부되어야 함");
  assert!(
    err
      .to_string()
      .contains("duplicate 'kernel-source-fact-fields' entry for field 'definition-ko'"),
    "{err:#}"
  );
}

#[test]
fn query_classifiers_kernel_source_list_fields_reject_duplicate_field() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
        { field = "related-concepts"; predicate = "related-concept-dup"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["왜"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("duplicate kernel-source-list-fields 는 거부되어야 함");
  assert!(
    err
      .to_string()
      .contains("duplicate 'kernel-source-list-fields' entry for field 'related-concepts'"),
    "{err:#}"
  );
}

#[test]
fn query_classifiers_predicate_classifiers_reject_duplicate_predicate() {
  let query_classifiers = temp_px_path("query-classifiers.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      definition-query-rules = [
        { match-any = ["뭐"]; }
      ];
      predicate-classifiers = [
        { predicate = "mass"; label-ko = "질량"; match-any = ["질량"]; }
        { predicate = "mass"; label-ko = "질량"; match-any = ["무게"]; }
      ];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["왜"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("duplicate predicate-classifiers 는 거부되어야 함");
  assert!(
    err
      .to_string()
      .contains("duplicate 'predicate-classifiers' entry for predicate 'mass'"),
    "{err:#}"
  );
}

#[test]
fn query_route_defaults_query_context_rewrite_rules_reject_duplicate_from() {
  let query_route_defaults = temp_px_path("query-route-defaults.px");
  write_px(
    &query_route_defaults,
    r#"
    {
      query-context-rewrite-rules = [
        { from = "Doghouse."; to = "Pnix."; }
        { from = "Doghouse."; to = "Other."; }
      ];
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_route_defaults_path = query_route_defaults;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("duplicate query-context-rewrite-rules from 은 거부되어야 함");
  assert!(
    err
      .to_string()
      .contains("duplicate 'query-context-rewrite-rules' entry for from 'Doghouse.'"),
    "{err:#}"
  );
}

#[test]
fn query_routes_reject_duplicate_route() {
  let query_routes = temp_px_path("query-routes.px");
  let query_route_defaults = temp_px_path("query-route-defaults.px");
  write_px(
    &query_route_defaults,
    r#"
    {
      query-context-rewrite-rules = [
        { from = "Doghouse."; to = "Pnix."; }
      ];
    }
    "#,
  );
  write_px(
    &query_routes,
    r#"
    [
      {
        route = "concept-definition-lookup";
        query-context = "Pnix.Query.Def";
        include-hop-knowledge = "false";
        default-preview = "1";
        policy-coverage = "0.5";
        policy-coherence = "0.5";
        policy-loss = "0.5";
        policy-cost = "0.5";
        policy-accept-threshold = "0.5";
        kernel-direct-fact-predicates = ["definition-ko"];
        kernel-direct-interpretation-id = "interp.definition.direct.${term}";
        kernel-rich-interpretation-id = "interp.definition.rich.${term}";
      }
      {
        route = "concept-definition-lookup";
        query-context = "Pnix.Query.Def.Dup";
        include-hop-knowledge = "false";
        default-preview = "1";
        policy-coverage = "0.5";
        policy-coherence = "0.5";
        policy-loss = "0.5";
        policy-cost = "0.5";
        policy-accept-threshold = "0.5";
        kernel-direct-fact-predicates = ["definition-ko"];
        kernel-direct-interpretation-id = "interp.definition.direct.${term}";
        kernel-rich-interpretation-id = "interp.definition.rich.${term}";
      }
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_routes_path = query_routes;
  paths.query_route_defaults_path = query_route_defaults;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("duplicate query-routes route 는 거부되어야 함");
  assert!(
    err
      .to_string()
      .contains("duplicate 'query-routes' entry for route 'concept-definition-lookup'"),
    "{err:#}"
  );
}

#[test]
fn placeholder_error_suggests_close_match_when_typo_is_within_tolerance() {
  // `kernel-base-facts.px` 의 `org-facts-count-template` 의 allowlist 는
  // `${count}` 뿐이다. `${counter}` 는 Levenshtein 거리 2 라서 힌트가 떠야 한다.
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      transcript-note-prefix = "transcript:";
      note-templates = {
        transcript-user = "transcript:user: ${utterance}";
        transcript-pnix = "transcript:pnix: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
      query-provenance-templates = {
        utterance = "utterance:${utterance}";
        concept-source = "concept-source:${source-ref}";
      };
      semantic-id-templates = {
        episode-id-template = "episode.pnix.standalone.${counter}";
        record-id-template = "record.fact.${episode-id}.${index}";
        knowledge-id-template = "knowledge.pnix.${episode-id}";
        knowledge-summary = "pnix standalone query kernel staging record";
      };
      pipeline-trace-note-prefixes = ["ontology-" "truth-regime:" "held-"];
      output-fragment-templates = {
        pipeline-trace = { kind = "pipeline-trace"; visibility = "dev"; };
        response-document = { kind = "response-document"; visibility = "dev"; };
      };
      response-document-schema = {
        px-header-comment = "# pnix ontology response document (auto-generated)";
        px-field-episode-id = "episode-id";
        px-field-summary = "summary";
        px-field-transcript = "transcript";
        px-field-pipeline = "pipeline";
        px-field-facts-count = "facts-count";
        org-title = "* Ontology Response";
        org-pipeline-section-header = "** Pipeline";
        org-facts-count-template = "- Facts: ${counter}";
        org-transcript-transforms = [
          { input-prefix = "user: "; output-prefix = "** Q: "; }
          { input-prefix = "pnix: "; output-prefix = ""; }
        ];
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("typo 는 거부되어야 함");
  let message = err.to_string();
  assert!(
    message.contains("unsupported placeholder '${counter}'"),
    "{message}"
  );
  assert!(
    message.contains("did you mean '${count}'"),
    "message should include hint toward '${{count}}': {message}"
  );
}

#[test]
fn placeholder_error_omits_hint_when_typo_is_too_far() {
  // `${completely-unrelated}` 는 allowed `${count}` 에서 Levenshtein 거리가
  // 너무 크다. 힌트 없이 에러 메시지만 나와야 한다.
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
    {
      base-query-facts = [
        {
          id-template = "fact.pnix.route.${route-segment}";
          context = "Pnix.Query";
          subj = "pnix";
          pred = "service-route";
          obj-template = "${route}";
        }
      ];
      concept-source-facts = {
        scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
        list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
        provenance-template = "concept-source:${source-ref}";
      };
      transcript-note-prefix = "transcript:";
      note-templates = {
        transcript-user = "transcript:user: ${utterance}";
        transcript-pnix = "transcript:pnix: ${response}";
        held-reopen-reason = "held-reopen:reason:${reason}";
        held-reopen-term = "held-reopen:term:${term}";
        held-reason = "held-reason:${reason}";
        held-term = "held-term:${term}";
        invert-trigger = "invert-trigger:${trigger-type}";
        truth-regime = "truth-regime:${regime}";
        predicate-query = "predicate-query:${predicate}";
      };
      query-provenance-templates = {
        utterance = "utterance:${utterance}";
        concept-source = "concept-source:${source-ref}";
      };
      semantic-id-templates = {
        episode-id-template = "episode.pnix.standalone.${counter}";
        record-id-template = "record.fact.${episode-id}.${index}";
        knowledge-id-template = "knowledge.pnix.${episode-id}";
        knowledge-summary = "pnix standalone query kernel staging record";
      };
      pipeline-trace-note-prefixes = ["ontology-" "truth-regime:" "held-"];
      output-fragment-templates = {
        pipeline-trace = { kind = "pipeline-trace"; visibility = "dev"; };
        response-document = { kind = "response-document"; visibility = "dev"; };
      };
      response-document-schema = {
        px-header-comment = "# pnix ontology response document (auto-generated)";
        px-field-episode-id = "episode-id";
        px-field-summary = "summary";
        px-field-transcript = "transcript";
        px-field-pipeline = "pipeline";
        px-field-facts-count = "facts-count";
        org-title = "* Ontology Response";
        org-pipeline-section-header = "** Pipeline";
        org-facts-count-template = "- Facts: ${completely-unrelated}";
        org-transcript-transforms = [
          { input-prefix = "user: "; output-prefix = "** Q: "; }
          { input-prefix = "pnix: "; output-prefix = ""; }
        ];
      };
    }
    "##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("너무 먼 typo 도 거부되어야 함");
  let message = err.to_string();
  assert!(
    message.contains("unsupported placeholder '${completely-unrelated}'"),
    "{message}"
  );
  assert!(
    !message.contains("did you mean"),
    "힌트가 떠서는 안 됨 (tolerance 초과): {message}"
  );
}

#[test]
fn all_sanctioned_px_files_parse_cleanly() {
  // parser layer 가 duplicate attrset key 를 거부한 이후, production
  // `.px` 파일 중 실제로 duplicate 를 가진 파일이 없는지 audit.
  // 실패 시 실패 파일 경로와 에러 메시지를 한 번에 보여준다. 신규 `.px`
  // 파일 추가 시 이 test 가 자동으로 duplicate key 와 기본 parse 오류를 검증한다.
  fn walk(dir: &Path, results: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
      Ok(e) => e,
      Err(_) => return,
    };
    for entry in entries.flatten() {
      let path = entry.path();
      if path.is_dir() {
        walk(&path, results);
      } else if path.extension().and_then(|e| e.to_str()) == Some("px") {
        results.push(path);
      }
    }
  }

  let data_dir = data_dir();
  let mut px_files = Vec::new();
  walk(&data_dir, &mut px_files);
  px_files.sort();

  assert!(
    !px_files.is_empty(),
    "sanctioned data directory 아래에 .px 파일이 없음: {}",
    data_dir.display()
  );

  let mut failures: Vec<(PathBuf, String)> = Vec::new();
  for path in &px_files {
    // 헌법 §20 정합 path: literal-only `parse_px_file` 가 받지 못하는
    // generic constructor / `import + ++` expression 은 nix-eval fallback
    // 으로 통과해야 한다. 두 path 모두 fail 하는 경우만 sanctioned-px parse
    // failure 로 본다 (duplicate key 등은 nix-eval 도 reject 라 그대로 잡힌다).
    if let Err(err) = parse_px_file_with_pnix_eval_fallback(path) {
      failures.push((path.clone(), format!("{err}")));
    }
  }

  assert!(
    failures.is_empty(),
    "{}개의 .px 파일이 parser strictness 를 통과하지 못함 (literal + nix-eval fallback 모두 fail):\n{}",
    failures.len(),
    failures
      .iter()
      .map(|(p, e)| format!("  {} : {}", p.display(), e))
      .collect::<Vec<_>>()
      .join("\n")
  );
}

#[test]
fn px_parser_duplicate_key_error_includes_line_and_col() {
  // duplicate attrset key 에러 메시지에 발생 위치 (line/col) 가 포함되어야
  // `.px` author 가 어디를 고쳐야 하는지 즉시 볼 수 있다.
  let input = "{\n  foo = \"a\";\n  bar = \"b\";\n  foo = \"c\";\n}";
  let err = parse_px(input).expect_err("duplicate foo 는 거부되어야 함");
  let message = err.to_string();
  assert!(message.contains("duplicate attrset key 'foo'"), "{message}");
  // 두 번째 foo 는 4번째 줄에 있다. "at line 4" 포함 확인.
  assert!(message.contains("at line 4"), "{message}");
}

#[test]
fn px_parser_unterminated_string_error_includes_line_and_col() {
  // 문자열이 닫히지 않는 경우 에러에 문자열 시작 위치가 포함되어야 한다.
  // input 마지막에 닫는 `"` 가 없어서 EOF 도달 전 unterminated.
  let input = "{\n  foo = \"bar baz";
  let err = parse_px(input).expect_err("unterminated string 은 거부되어야 함");
  let message = err.to_string();
  assert!(message.contains("unterminated .px string"), "{message}");
  // 시작 문자열은 2번째 줄에 있다.
  assert!(message.contains("at line 2"), "{message}");
}

#[test]
fn px_parser_unterminated_attrset_error_includes_line_and_col() {
  let input = "{\n  foo = \"a\";\n";
  let err = parse_px(input).expect_err("unterminated attrset 은 거부되어야 함");
  let message = err.to_string();
  assert!(message.contains("unterminated .px attrset"), "{message}");
  // attrset 시작 `{` 은 1번째 줄에 있다.
  assert!(message.contains("at line 1"), "{message}");
}

#[test]
fn loader_wrong_type_error_attaches_line_col_when_section_found() {
  // Option C: loader 가 wrong-type 에러를 낼 때, 파일을 다시 읽어서 section key
  // 의 line/col 을 scan 하고 suffix 로 붙인다. base-query-facts 를 일부러 list
  // 가 아닌 attrset 으로 바꿔 trigger.
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    // 4 번째 line 에 `base-query-facts = ` 가 오도록 leading 공백 2 개 포함.
    "# header\n\n  {\n  base-query-facts = { wrong = \"attrset-not-list\"; };\n}\n",
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("attrset base-query-facts 는 list 가 아니라 거부되어야 함");
  let message = err.to_string();
  // 기존 substring (helper 를 건드리기 전 회귀 assertion 들) 은 여전히 present.
  assert!(
    message.contains("'base-query-facts' must be list"),
    "{message}"
  );
  // 새 Option C suffix: line/col 힌트.
  assert!(
    message.contains("at line 4"),
    "Option C suffix 누락: {message}"
  );
  assert!(message.contains("col 3"), "Option C col 누락: {message}");
}

/// kernel.rs 소스에서 `fn err_<name>(` 로 시작하고 first argument 가 `path: &Path`
/// 인 함수 이름을 전부 뽑는다. batch 25 에서 manual inventory drift 를 막기 위해
/// 도입한 헬퍼. manual list 와 cross-check 하는 용도로만 쓰인다.
fn discover_path_based_err_helpers(src: &str) -> Vec<String> {
  let mut names: Vec<String> = Vec::new();
  let mut pos: usize = 0;
  while let Some(idx) = src[pos..].find("fn err_") {
    let abs = pos + idx;
    // skip "fn "
    let after_fn = abs + 3;
    let tail = &src[after_fn..];
    // Extract name up to '('.
    let Some(paren_open) = tail.find('(') else {
      break;
    };
    let name = &tail[..paren_open];
    // Grab argument list between '(' and matching ')'. Arg list is shallow so
    // first ')' is sufficient for our helpers.
    let rest = &tail[paren_open + 1..];
    let Some(paren_close) = rest.find(')') else {
      break;
    };
    let args = &rest[..paren_close];
    if args.contains("path: &Path") {
      names.push(name.to_string());
    }
    pos = after_fn + paren_open + 1 + paren_close;
  }
  names.sort();
  names.dedup();
  names
}

#[test]
fn kernel_rs_loader_helper_suffix_coverage_audit_matches_source() {
  // Batch 24/25: suffix coverage audit drift ratchet.
  //
  // `kernel.rs` block comment 의 audit 표는 "어느 helper 가 Option C (line/col
  // suffix) 를 지원하는가" 를 문서화한다. 이 테스트는 audit 가 실제 helper 구현과
  // 일치하는지 drift 를 막는다.
  //
  // 세 invariant (batch 25 에서 한 개 추가):
  //   1. manual `PATH_BASED_HELPERS` 목록이 kernel.rs 의 실제 `fn err_*(path: &Path)`
  //      집합과 완전히 일치해야 한다 (새 helper 가 추가됐는데 목록에 안 올라오는
  //      drift 를 자동으로 잡는다).
  //   2. 각 helper fn body 안에 `SUFFIX_EMITTERS` 중 최소 하나의 호출이 있어야
  //      한다 (helper 가 suffix 경로를 우회해 raw anyhow! 를 쓰면 감지된다).
  //   3. 같은 helper 목록이 block comment audit 표에도 전부 명시되어 있어야
  //      한다 (문서가 구현과 drift 하면 감지된다).
  const PATH_BASED_HELPERS: &[&str] = &[
    "err_duplicate_entry",
    "err_invalid_entry",
    "err_missing",
    "err_missing_for_route",
    "err_missing_in_context",
    "err_with_location",
    "err_wrong_type",
    "err_wrong_type_in_context",
  ];
  const SUFFIX_EMITTERS: &[&str] = &[
    "err_with_location",
    "locate_section_in_source",
    "locate_route_entry_in_source",
    "locate_duplicate_entry_in_source",
  ];

  let src_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/kernel.rs");
  let src = fs::read_to_string(&src_path).expect("read kernel.rs source");

  // invariant 1 (batch 25): manual list 가 source truth 와 일치.
  let discovered = discover_path_based_err_helpers(&src);
  let mut expected: Vec<String> = PATH_BASED_HELPERS.iter().map(|s| s.to_string()).collect();
  expected.sort();
  assert_eq!(
    discovered, expected,
    "PATH_BASED_HELPERS manual inventory 가 kernel.rs 의 실제 `fn err_*(path: &Path)` \
     집합과 drift 했다. discovered={discovered:?}, manual={expected:?}"
  );

  // invariant 2: 각 helper body 안에 suffix emitter 가 있는가.
  for helper in PATH_BASED_HELPERS {
    let needle = format!("fn {}(", helper);
    let start = src
      .find(needle.as_str())
      .unwrap_or_else(|| panic!("helper `{helper}` 가 kernel.rs 에 없다"));
    // fn body 는 첫 `{` 부터 brace depth 가 0 으로 돌아올 때까지.
    let body_start = src[start..]
      .find('{')
      .map(|o| start + o)
      .unwrap_or_else(|| panic!("helper `{helper}` body 시작 `{{` 못 찾음"));
    let mut depth: i32 = 0;
    let mut body_end = body_start;
    for (offset, ch) in src[body_start..].char_indices() {
      match ch {
        '{' => depth += 1,
        '}' => {
          depth -= 1;
          if depth == 0 {
            body_end = body_start + offset + 1;
            break;
          }
        }
        _ => {}
      }
    }
    let body = &src[body_start..body_end];
    let covers = SUFFIX_EMITTERS
      .iter()
      .any(|emitter| body.contains(&format!("{}(", emitter)));
    assert!(
      covers,
      "helper `{helper}` 가 suffix emitter ({SUFFIX_EMITTERS:?}) 중 어느 것도 \
       호출하지 않는다. Option C 우회 의심 — audit 표와 실제 구현이 drift 했거나, \
       새 helper 가 suffix 경로를 빼먹었다."
    );
  }

  // invariant 3: audit 표 block comment 에 각 helper 이름이 나와야 한다.
  // audit block 은 `// ## suffix coverage audit` 로 시작하는 comment block.
  let audit_start = src
    .find("## suffix coverage audit")
    .expect("audit block 헤더 (`## suffix coverage audit`) 를 kernel.rs 에서 못 찾음");
  // audit block 은 audit_start 이후 `fn err_with_location` 정의까지 이어진다.
  // 간단히 그 시작 위치까지로 본다 (char boundary 안전).
  let audit_block_end = src[audit_start..]
    .find("fn err_with_location")
    .map(|o| audit_start + o)
    .unwrap_or(src.len());
  let audit_block = &src[audit_start..audit_block_end];
  for helper in PATH_BASED_HELPERS {
    assert!(
      audit_block.contains(helper),
      "helper `{helper}` 가 suffix coverage audit 표에 없다. block comment 가 \
       drift 됐다 — 표를 업데이트하거나 helper 목록을 수정해야 한다."
    );
  }
}

#[test]
fn discover_path_based_err_helpers_matches_current_kernel() {
  // batch 25: auto-discovery helper 자체의 sanity check. 알려진 path-based
  // helper 8 개는 전부 나오고, runtime-only helper 3 개는 안 나와야 한다.
  let src_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/kernel.rs");
  let src = fs::read_to_string(&src_path).expect("read kernel.rs source");
  let discovered = discover_path_based_err_helpers(&src);

  // path-based (포함되어야 함)
  for expected in [
    "err_with_location",
    "err_missing",
    "err_wrong_type",
    "err_invalid_entry",
    "err_missing_in_context",
    "err_wrong_type_in_context",
    "err_duplicate_entry",
    "err_missing_for_route",
  ] {
    assert!(
      discovered.iter().any(|n| n == expected),
      "discover_path_based_err_helpers 가 `{expected}` 를 못 찾음: {discovered:?}"
    );
  }

  // runtime-only (포함되면 안 됨 — path argument 가 없으므로)
  for unexpected in [
    "err_missing_standalone_field",
    "err_missing_runtime",
    "err_missing_reopen_rule",
  ] {
    assert!(
      !discovered.iter().any(|n| n == unexpected),
      "discover_path_based_err_helpers 가 runtime-only helper `{unexpected}` 를 \
       포함했다: {discovered:?}"
    );
  }
}

#[test]
fn kernel_rs_has_no_raw_anyhow_missing_literals_outside_helpers() {
  // Ratchet: 앞으로 새 inline `anyhow!("missing '...'"` literal 이 helper 밖
  // 으로 추가되면 여기서 바로 fail 하게 만든다. 현재 기준 helper 정의 안에 있는
  // `anyhow!("missing ` literal 갯수를 상한으로 삼는다.
  //
  // 여기서 세는 건 "missing" 로 시작하는 에러 body 생성 literal 만. 다른 단어
  // (duplicate / invalid / unknown / unterminated 등) 는 본 ratchet 범위 밖.
  //
  // 모든 comment line (`//` 또는 `///` 로 시작) 은 제외한다 — helper 정의의
  // 설명문이나 block comment 안에 해당 pattern 을 인용할 수 있기 때문이다.
  let src_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/kernel.rs");
  let src = fs::read_to_string(&src_path).expect("read kernel.rs source");
  let count = src
    .lines()
    .filter(|line| !line.trim_start().starts_with("//"))
    .filter(|line| line.contains("anyhow!(\"missing "))
    .count();
  // 2026-04-13 기준 허용 상한 (helper 정의 body 안쪽):
  //   err_missing_standalone_field → 1 line
  //   err_missing_runtime          → 1 line
  //   err_missing_reopen_rule      → 1 line
  // total = 3. batch 20 시점에는 5 였지만 batch 21 에서 `err_missing` 과
  // `err_missing_in_context` 가 `err_with_location` 경유로 바뀌면서 내부 anyhow 의
  // 첫 token 이 `"{}"` 가 되어 이 scan 에 더 이상 걸리지 않는다.
  // `err_missing_for_route` 의 helper body 는 원래부터 multi-line 이라 single-line
  // 기준으로는 카운트되지 않는다. helper 밖에 새 single-line literal 이 추가되면
  // 이 ratchet 이 즉시 fail.
  assert!(
    count <= 3,
    "kernel.rs 에 single-line `anyhow!(\"missing \"` literal 이 {count} 개 — \
     helper 밖으로 새 inline 에러가 추가됐을 가능성이 있다. 새 에러는 \
     `err_missing_*` helper 를 경유해야 한다."
  );
}

#[test]
fn loader_missing_error_attaches_line_col_when_parent_section_found() {
  // Option C 확장 (batch 21): `err_missing` 도 이제 best-effort 로 section
  // location 을 찾아 suffix 를 붙인다. kernel-base-facts 의 `concept-source-facts`
  // 가 누락된 상황을 만들고, parent `concept-source-facts =` line 이 있어서는 안
  // 되는 상황이므로 대신 `base-query-facts` 를 미리 두어서 missing 할 다른 section
  // 을 비워 트리거. 구체적으로는 `semantic-id-templates` 자리를 통째로 비워 missing
  // 을 유발. semantic-id-templates 자체는 파일에 없으므로 suffix 는 못 붙지만,
  // 회귀 자체는 "기존 substring 이 여전히 있는가" 만 확인한다.
  let kernel_base_facts = temp_px_path("kernel-base-facts.px");
  write_px(
    &kernel_base_facts,
    r##"
{
  base-query-facts = [
    {
      id-template = "fact.pnix.route.${route-segment}";
      context = "Pnix.Query";
      subj = "pnix";
      pred = "service-route";
      obj-template = "${route}";
    }
  ];
  concept-source-facts = {
    scalar-id-template = "fact.pnix.concept.${term}.${predicate}";
    list-id-template = "fact.pnix.concept.${term}.${predicate}.${index}";
    provenance-template = "concept-source:${source-ref}";
  };
}
"##,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.kernel_base_facts_path = kernel_base_facts;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("missing note-templates 는 거부되어야 함");
  let message = err.to_string();
  // 기존 substring 은 변함없이 present (회귀 compat).
  assert!(
    message.contains("missing 'transcript-note-prefix'")
      || message.contains("missing 'note-templates'"),
    "기본 missing body 누락: {message}"
  );
}

#[test]
fn loader_missing_for_route_attaches_route_entry_line_col() {
  // Option C 확장 (batch 22): `err_missing_for_route` 가 이제 `.px` 파일에서
  // `route = "<route>"` entry 의 정의 line 을 찾아 suffix 를 붙인다. 어느 route
  // entry 에서 필드가 누락됐는지 바로 알 수 있다.
  //
  // fixture: custom property route 에서 `policy-coverage` 등 필수 필드를 전부
  // 빼서 `missing 'policy-coverage' for route '...'` 에러를 유발한다. entry 는
  // 일부러 3 번째 line 에 배치한다.
  let query_classifiers = temp_px_path("query-classifiers-missing-route-field.px");
  let query_routes = temp_px_path("query-routes-missing-route-field.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
    "#,
    r#"
        { field = "definition-ko"; predicate = "definition-ko"; }
    "#,
    r#"
        { field = "related-concepts"; predicate = "related-concept"; }
    "#,
    r#"
        { match-any = ["뭐"]; }
    "#,
    r#"
        { match-any = ["식"]; predicate = "formula"; label-ko = "식"; }
    "#,
  );
  // line 3 에 `route = "custom-property-route"` 가 오도록 header 2 line 포함.
  write_px(
    &query_routes,
    // line 1: blank
    // line 2: `[`
    // line 3: `  { route = "custom-property-route"; ... }`  -- 여기가 hit 지점
    "\n[\n  { route = \"custom-property-route\"; query-context = \"Pnix.Query.Custom\"; include-hop-knowledge = \"false\"; default-preview = \"4\"; kernel-interpretation-id = \"custom.${predicate}.${term}\"; }\n]\n",
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  let mut kernel = PnixReplKernel::new(paths);

  let err = kernel
    .evaluate_px_source(r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘 식"; }"#)
    .expect_err("missing route policy field 는 거부되어야 함");
  let message = err.to_string();
  // 기존 base body (회귀 compat).
  assert!(
    message.contains("missing 'policy-coverage' for route 'custom-property-route'"),
    "base body 누락: {message}"
  );
  // 새 Option C suffix: `route = "custom-property-route"` 가 line 3 에 있다.
  assert!(
    message.contains("at line 3"),
    "route entry line/col suffix 누락: {message}"
  );
}

#[test]
fn loader_duplicate_entry_points_at_second_occurrence_line() {
  // Option C 확장 (batch 22): `err_duplicate_entry` 가 이제 parent section 이
  // 아니라 실제 duplicate 가 일어난 `<key_label> = "<key>"` 쌍의 두 번째 등장
  // line 을 가리킨다.
  //
  // fixture: query-routes.px 에 같은 route 를 두 번 넣어 duplicate 를 trigger.
  // 두 번째 entry 의 `route = "concept-definition-lookup"` line 위치가 suffix
  // 에 나타나야 한다.
  let query_routes = temp_px_path("query-routes-dup-suffix.px");
  let query_route_defaults = temp_px_path("query-route-defaults-dup-suffix.px");
  write_px(
    &query_route_defaults,
    r#"
    {
      query-context-rewrite-rules = [
        { from = "Doghouse."; to = "Pnix."; }
      ];
    }
    "#,
  );
  // 의도적으로 라인 배치를 통제한다. 두 번째 `route = "concept-definition-lookup"`
  // 은 line 16 에 오도록 header / 첫 entry 를 다음과 같이 짠다:
  //   line 1: (blank)
  //   line 2: [
  //   line 3: {
  //   line 4..14: 첫 entry body
  //   line 15: }
  //   line 16: { route = "concept-definition-lookup"; ...
  //   line 17: ...
  //   line 18: }
  //   line 19: ]
  write_px(
    &query_routes,
    "
[
  {
    route = \"concept-definition-lookup\";
    query-context = \"Pnix.Query.Def\";
    include-hop-knowledge = \"false\";
    default-preview = \"1\";
    policy-coverage = \"0.5\";
    policy-coherence = \"0.5\";
    policy-loss = \"0.5\";
    policy-cost = \"0.5\";
    policy-accept-threshold = \"0.5\";
    kernel-direct-fact-predicates = [\"definition-ko\"];
    kernel-direct-interpretation-id = \"interp.d.direct.${term}\";
    kernel-rich-interpretation-id = \"interp.d.rich.${term}\";
  }
  { route = \"concept-definition-lookup\"; query-context = \"Pnix.Query.Def.Dup\"; include-hop-knowledge = \"false\"; default-preview = \"1\"; policy-coverage = \"0.5\"; policy-coherence = \"0.5\"; policy-loss = \"0.5\"; policy-cost = \"0.5\"; policy-accept-threshold = \"0.5\"; kernel-direct-fact-predicates = [\"definition-ko\"]; kernel-direct-interpretation-id = \"interp.d.direct.${term}\"; kernel-rich-interpretation-id = \"interp.d.rich.${term}\"; }
]
",
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_routes_path = query_routes;
  paths.query_route_defaults_path = query_route_defaults;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("duplicate query-routes route 는 거부되어야 함");
  let message = err.to_string();
  assert!(
    message.contains("duplicate 'query-routes' entry for route 'concept-definition-lookup'"),
    "base body 누락: {message}"
  );
  // 두 번째 `route = "concept-definition-lookup"` 은 위 fixture 기준 line 17 에
  // 있다 (line 1 은 첫 newline, line 2 는 `[`, line 3 는 `{`, line 4-15 는 첫
  // entry, line 16 은 닫는 `}`, line 17 은 두 번째 entry 의 single-line body).
  assert!(
    message.contains("at line 17"),
    "duplicate entry line/col suffix 가 second occurrence 를 가리키지 않음: {message}"
  );
}

#[test]
fn loader_duplicate_scanner_is_section_scoped_and_ignores_comment_mentions() {
  // Batch 23: `locate_duplicate_entry_in_source` 가 section-scope 로 동작하는지
  // 검증. file 상단 주석에 `when = "known-term"` 이 literal 로 등장하고, 같은
  // pattern 이 held-reason-rules section 안에 duplicate 로 존재한다. file-wide
  // scanner 라면 comment line 을 첫 매치로 잡고 duplicate 의 첫 entry 를 두 번째
  // 매치로 잡아서 wrong line 을 가리킨다. section-scoped scanner 는 held-reason-rules
  // 내부 두 entry 만 보고 올바른 second occurrence 를 가리킨다.
  let query_classifiers = temp_px_path("query-classifiers-sectscope-dup.px");
  let query_routes = temp_px_path("query-routes-sectscope-dup.px");
  // 중요: 첫 줄 주석에 `when = "known-term"` 리터럴을 넣어 file-wide scanner 를
  // 의도적으로 혼란시킨다. held-reason-rules 내부 duplicate 는 훨씬 뒷 라인에
  // 위치한다.
  write_px(
    &query_classifiers,
    r#"# 테스트 fixture: when = "known-term" 리터럴이 주석과 section 양쪽에 등장한다
{
  query-dispatch-priority = ["why" "property" "definition"];
  kernel-dispatch-routes = {
    definition = "concept-definition-lookup";
    property = "concept-predicate-lookup";
    held = "lightweight-korean-dialogue-held";
  };
  held-reason-keys = {
    requires-context = "requires-context";
    unknown-term = "unknown-term";
  };
  held-reason-rules = [
    { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
    { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
  ];
  kernel-source-fact-fields = [
    { field = "definition-ko"; predicate = "definition-ko"; }
  ];
  kernel-source-list-fields = [
    { field = "related-concepts"; predicate = "related-concept"; }
  ];
  definition-query-rules = [
    { match-any = ["뭐"]; }
  ];
  predicate-classifiers = [
    { match-any = ["식"]; predicate = "formula"; label-ko = "식"; }
  ];
  concept-what-markers = [];
  concept-definition-suffixes = [];
  concept-explain-markers = [];
  concept-explain-skip-tokens = [];
  question-word-stems = ["뭐"];
  term-extraction-suffixes = [];
  term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
  term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
  term-fallback-policy = "known-concept-token-scan";
}
"#,
  );
  write_px(
    &query_routes,
    r#"
    [
      {
        route = "concept-definition-lookup";
        query-context = "Pnix.Query.Def";
        include-hop-knowledge = "false";
        default-preview = "1";
        policy-coverage = "0.5";
        policy-coherence = "0.5";
        policy-loss = "0.5";
        policy-cost = "0.5";
        policy-accept-threshold = "0.5";
        kernel-direct-fact-predicates = ["definition-ko"];
        kernel-direct-interpretation-id = "interp.d.direct.${term}";
        kernel-rich-interpretation-id = "interp.d.rich.${term}";
      }
      {
        route = "concept-predicate-lookup";
        query-context = "Pnix.Query.Property";
        include-hop-knowledge = "false";
        default-preview = "1";
        policy-coverage = "0.5";
        policy-coherence = "0.5";
        policy-loss = "0.5";
        policy-cost = "0.5";
        policy-accept-threshold = "0.5";
        kernel-interpretation-id = "interp.p.${predicate}.${term}";
      }
      {
        route = "lightweight-korean-dialogue-held";
        query-context = "Pnix.Query.Held";
        include-hop-knowledge = "false";
        default-preview = "1";
        policy-coverage = "0.5";
        policy-coherence = "0.5";
        policy-loss = "0.5";
        policy-cost = "0.5";
        policy-accept-threshold = "0.5";
      }
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  paths.query_routes_path = query_routes;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("duplicate held-reason-rules 는 거부되어야 함");
  let message = err.to_string();
  // base body.
  assert!(
    message.contains("duplicate 'held-reason-rules' entry for when 'known-term'"),
    "base body 누락: {message}"
  );
  // Section-scoped scanner 가 정확히 **두 번째 entry** 를 가리켜야 한다. 위
  // fixture 에서 실제 1-indexed line 배치:
  //   line  1: 주석 (`when = "known-term"` 리터럴)
  //   line  2: `{`
  //   line  3: `  query-dispatch-priority = ...`
  //   ...
  //   line 13: `  held-reason-rules = [`
  //   line 14: 첫 `{ when = "known-term"; ... }`
  //   line 15: 두 번째 `{ when = "known-term"; ... }`  ← 정답
  //   line 16: `  ];`
  //
  // file-wide scanner 라면 line 1 (comment 매치) + line 14 (첫 entry) 를 찾아서
  // line 14 를 "두 번째 매치" 로 잘못 리턴한다. section-scoped scanner 는
  // held-reason-rules header (line 13) 이후만 scan 하므로 line 14 / line 15 를
  // 찾아 line 15 를 정답으로 리턴한다.
  assert!(
    message.contains("at line 15"),
    "section-scoped scanner 가 올바른 second occurrence 를 가리키지 않음: {message}"
  );
  // 추가 guard: 주석 line 1 이나 첫 entry line 14 를 가리키면 section-scoping 이
  // 동작하지 않은 것이다.
  assert!(
    !message.contains("at line 1,") && !message.contains("at line 14,"),
    "false positive 위치 (comment 또는 first entry) 를 suffix 로 emit: {message}"
  );
}

#[test]
fn loader_invalid_entry_error_preserves_base_body_after_helper_refactor() {
  // Option C 확장 (batch 21): `err_invalid_entry` 가 `err_with_location` 경유로
  // 바뀌었을 때, parent section 이 파일에 없더라도 base body 가 그대로 유지되는지
  // 확인. 여기서는 query-routes root 가 list (top-level 에 `=` 가 없음) 이므로
  // locate_section_in_source("query-routes", ...) 는 None 을 돌려준다. 따라서
  // helper 는 suffix 없이 base body 만 emit 해야 한다.
  let query_routes = temp_px_path("query-routes-non-attrset.px");
  let query_route_defaults = temp_px_path("query-route-defaults-for-invalid.px");
  write_px(
    &query_route_defaults,
    r#"
    {
      query-context-rewrite-rules = [
        { from = "Doghouse."; to = "Pnix."; }
      ];
    }
    "#,
  );
  // query-routes root 는 list 지만, 그 list 안에 attrset 이 아닌 element 를 넣어
  // invalid entry 를 trigger.
  write_px(
    &query_routes,
    r#"
    [
      "this-is-not-an-attrset"
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_routes_path = query_routes;
  paths.query_route_defaults_path = query_route_defaults;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect_err("non-attrset query-routes entry 는 거부되어야 함");
  let message = err.to_string();
  // batch 20 에서 body 를 `invalid 'query-routes' entry` 로 이미 통일. batch 21
  // helper refactor 이후에도 이 base body 는 유지되어야 한다.
  assert!(
    message.contains("invalid 'query-routes' entry"),
    "invalid entry base body 누락: {message}"
  );
}

/// batch 72 (2026-04-15): P5.5 step 4 envelope shaping parity — property lane.
///
/// `kernel-route-summary.property` 템플릿이 `${label}` placeholder 를
/// predicate classifier 의 `label-ko` 로 치환한다는 것을 증명한다. doghouse
/// store 의 predicate envelope 은 summary 에 predicate label (e.g., "단위")
/// 을 포함하는데, kernel 도 같은 contract 를 만족해야 향후 primary delegation
/// 에서 `response.summary.contains("단위")` 같은 test 가 통과한다.
#[test]
fn batch72_kernel_property_summary_contains_predicate_label() {
  let mut kernel = PnixReplKernel::new(KernelPaths::from_data_dir(data_dir()));
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘의 단위는 뭐야?"; }"#,
    )
    .expect("evaluate property query");

  // summary 는 term 과 predicate label 을 모두 포함해야 한다.
  assert!(
    response.summary.contains("힘"),
    "summary missing term '힘': {}",
    response.summary
  );
  assert!(
    response.summary.contains("단위"),
    "summary missing predicate label '단위' — shaping parity 미닫힘: {}",
    response.summary
  );
}

/// batch 73 (2026-04-15): P5.5 step 4.2 envelope shaping parity — property
/// lane structured facts.
///
/// doghouse store 의 `build_predicate_query_facts` 가 emit 하는 4 종류의
/// structured fact (`predicate-query-term`, `predicate-query-kind`,
/// `concept-domain`, `predicate-result-{pred}`) 를 kernel `answer_property`
/// 도 envelope 에 담는지 검증. 이 4 fact 가 모두 있어야 predicate lane 의
/// fact parity 가 닫혔다고 말할 수 있고, 그 후에 primary flip 실험이 가능.
#[test]
fn batch73_kernel_property_facts_include_store_contract_predicates() {
  use pnix_core::ontology::SemanticRecordValue;

  let mut kernel = PnixReplKernel::new(KernelPaths::from_data_dir(data_dir()));
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘의 단위는 뭐야?"; }"#,
    )
    .expect("evaluate property query");

  let facts: Vec<_> = response
    .envelope
    .records
    .iter()
    .filter_map(|r| match &r.value {
      SemanticRecordValue::ContextualFact(fact) => Some(fact),
      _ => None,
    })
    .collect();

  // 1) predicate-query-term = "힘"
  assert!(
    facts
      .iter()
      .any(|f| f.pred == "predicate-query-term" && f.obj == "힘"),
    "missing predicate-query-term=힘: {:?}",
    facts
      .iter()
      .map(|f| (f.pred.as_str(), f.obj.as_str()))
      .collect::<Vec<_>>()
  );

  // 2) predicate-query-kind = "unit-ko"
  assert!(
    facts
      .iter()
      .any(|f| f.pred == "predicate-query-kind" && f.obj == "unit-ko"),
    "missing predicate-query-kind=unit-ko: {:?}",
    facts
      .iter()
      .map(|f| (f.pred.as_str(), f.obj.as_str()))
      .collect::<Vec<_>>()
  );

  // 3) concept-domain 은 비어있지 않은 값으로 존재해야 한다 (정확한
  //    domain string 은 concept data 가 owner 이므로 non-empty 만 assert).
  assert!(
    facts
      .iter()
      .any(|f| f.pred == "concept-domain" && !f.obj.is_empty()),
    "missing non-empty concept-domain"
  );

  // 4) predicate-result-unit-ko 이 최소 하나 존재 (값은 concept data 기준).
  assert!(
    facts
      .iter()
      .any(|f| f.pred == "predicate-result-unit-ko" && !f.obj.is_empty()),
    "missing non-empty predicate-result-unit-ko fact"
  );
}

/// batch 75 (2026-04-15): P5.5 step 4.4 concept lane shaping parity — summary 시작.
///
/// `kernel-route-summary.definition` 템플릿이 `${term}` 과 함께 "온톨로지"
/// 문자열을 포함한다는 것을 증명한다. doghouse store 의 concept envelope
/// summary 는 `"doghouse가 저장된 온톨로지에서 '{term}'(...)의 엄밀한 한글
/// 정의를 조회하여 응답했다."` 형식이고, store test 들이
/// `response.summary.contains("온톨로지")` 로 assert 한다. kernel 이 같은
/// contract 를 만족해야 향후 concept dispatch primary flip 에서 회귀가 없다.
#[test]
fn batch75_kernel_definition_summary_contains_ontology_label() {
  let mut kernel = PnixReplKernel::new(KernelPaths::from_data_dir(data_dir()));
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "덧셈은 뭐야?"; }"#,
    )
    .expect("evaluate definition query");

  assert!(
    response.summary.contains("덧셈"),
    "summary missing term '덧셈': {}",
    response.summary
  );
  assert!(
    response.summary.contains("온톨로지"),
    "summary missing '온톨로지' string — concept lane shaping parity 미닫힘: {}",
    response.summary
  );
}

/// batch 76 (2026-04-15): P5.5 step 4.4.2 concept lane shaping parity —
/// structured facts + interpretation id prefix + notes.
///
/// doghouse store 의 `build_concept_query_facts` 가 emit 하는 4 개 structured
/// fact (`concept-query-term`, `concept-domain`, `concept-definition-ko`,
/// `concept-formal-name-en`) + `build_concept_query_notes` 의 2 개 note
/// (`concept-lookup:term:X`, `concept-lookup:domain:Y`) + ontology decision
/// fact 의 `ontology-selected-interpretation` obj 가 `"interp.concept."`
/// prefix 를 갖는다는 것을 한 번에 검증한다. predicate lane 의 batch 73 과
/// 같은 패턴.
#[test]
fn batch76_kernel_definition_facts_include_store_contract_predicates() {
  use pnix_core::ontology::SemanticRecordValue;

  let mut kernel = PnixReplKernel::new(KernelPaths::from_data_dir(data_dir()));
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "덧셈은 뭐야?"; }"#,
    )
    .expect("evaluate definition query");

  let facts: Vec<_> = response
    .envelope
    .records
    .iter()
    .filter_map(|r| match &r.value {
      SemanticRecordValue::ContextualFact(fact) => Some(fact),
      _ => None,
    })
    .collect();

  // 1) concept-query-term == "덧셈"
  assert!(
    facts
      .iter()
      .any(|f| f.pred == "concept-query-term" && f.obj == "덧셈"),
    "missing concept-query-term=덧셈"
  );

  // 2) concept-domain non-empty
  assert!(
    facts
      .iter()
      .any(|f| f.pred == "concept-domain" && !f.obj.is_empty()),
    "missing non-empty concept-domain"
  );

  // 3) concept-definition-ko contains "이항연산" (덧셈 의 definition 본문)
  assert!(
    facts
      .iter()
      .any(|f| f.pred == "concept-definition-ko" && f.obj.contains("이항연산")),
    "missing concept-definition-ko containing '이항연산'"
  );

  // 4) concept-formal-name-en == "addition"
  assert!(
    facts
      .iter()
      .any(|f| f.pred == "concept-formal-name-en" && f.obj == "addition"),
    "missing concept-formal-name-en=addition"
  );

  // 5) ontology-selected-interpretation starts_with "interp.concept."
  assert!(
    facts.iter().any(|f| f.pred == "ontology-selected-interpretation"
      && f.obj.starts_with("interp.concept.")),
    "missing ontology-selected-interpretation with interp.concept.* prefix"
  );

  // 6) notes 에 concept-lookup:term / concept-lookup:domain 포함
  assert!(
    response
      .envelope
      .notes
      .iter()
      .any(|n| n == "concept-lookup:term:덧셈"),
    "missing note 'concept-lookup:term:덧셈'"
  );
  assert!(
    response
      .envelope
      .notes
      .iter()
      .any(|n| n.starts_with("concept-lookup:domain:") && !n.ends_with(":")),
    "missing non-empty 'concept-lookup:domain:*' note"
  );
}

/// batch 77 (2026-04-15): P5.5 step 4.4.3 concept lane transcript parity.
///
/// kernel definition response 가 transcript note 로 materialize 될 때,
/// definition body 와 formal-name-en 이 모두 포함되는지 확인한다.
#[test]
fn batch77_kernel_definition_transcript_contains_definition_and_formal_name() {
  let mut kernel = PnixReplKernel::new(KernelPaths::from_data_dir(data_dir()));
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "덧셈은 뭐야?"; }"#,
    )
    .expect("evaluate definition query");

  let transcript: Vec<&String> = response
    .envelope
    .notes
    .iter()
    .filter(|n| n.starts_with("transcript:pnix:"))
    .collect();

  assert!(
    transcript.iter().any(|n| n.contains("이항연산")),
    "missing definition body in transcript notes: {transcript:?}"
  );
  assert!(
    transcript.iter().any(|n| n.contains("addition")),
    "missing formal-name-en in transcript notes: {transcript:?}"
  );
}

/// batch 78 (2026-04-15): P5.5 step 4.4.4 concept standard transcript parity.
///
/// standard scope kernel definition transcript 가 store standard response 와 같은
/// 핵심 enriched surface 를 담는지 확인한다: 단위, 공식, 관련 개념.
#[test]
fn batch78_kernel_definition_transcript_contains_unit_formula_and_related() {
  let mut kernel = PnixReplKernel::new(KernelPaths::from_data_dir(data_dir()));
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "운동에너지가 뭐야?"; }"#,
    )
    .expect("evaluate enriched definition query");

  let transcript: Vec<&String> = response
    .envelope
    .notes
    .iter()
    .filter(|n| n.starts_with("transcript:pnix:"))
    .collect();

  assert!(
    transcript.iter().any(|n| n.contains("줄(J)")),
    "missing unit in transcript notes: {transcript:?}"
  );
  assert!(
    transcript.iter().any(|n| n.contains("Eₖ = ½mv²")),
    "missing formula in transcript notes: {transcript:?}"
  );
  assert!(
    transcript.iter().any(|n| n.contains("관련 개념")),
    "missing related concept phrase in transcript notes: {transcript:?}"
  );
}

/// batch 80 (2026-04-15): P5.5 step 4.4.5 concept detailed transcript parity.
///
/// kernel detailed definition transcript 가 store rich path 와 같은
/// "연결 지식으로 ..." surface 를 materialize 하는지 확인한다.
#[test]
fn batch80_kernel_definition_detailed_transcript_contains_connected_knowledge() {
  let mut kernel = PnixReplKernel::new(KernelPaths::from_data_dir(data_dir()));
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "detailed"; utterance = "에너지에 대해 자세히 설명해줘"; }"#,
    )
    .expect("evaluate detailed definition query");

  let transcript: Vec<&String> = response
    .envelope
    .notes
    .iter()
    .filter(|n| n.starts_with("transcript:pnix:"))
    .collect();

  assert!(
    transcript.iter().any(|n| n.contains("연결 지식")),
    "missing connected knowledge phrase in transcript notes: {transcript:?}"
  );
  assert!(
    transcript.iter().any(|n| {
      n.contains("운동에너지") || n.contains("위치에너지") || n.contains("일")
    }),
    "missing connected concept terms in transcript notes: {transcript:?}"
  );
}

/// batch 82 (2026-04-15): P5.5 step 4.5.3 concept brief lane parity.
///
/// object particle("를")를 가진 brief definition query 도 kernel term extraction 이
/// concept term 을 잡아야 하고, brief scope transcript 는 detailed-only
/// "연결 지식" surface 없이 definition 을 반환해야 한다.
#[test]
fn batch82_kernel_definition_brief_query_with_object_particle_resolves_term() {
  let mut kernel = PnixReplKernel::new(KernelPaths::from_data_dir(data_dir()));
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "brief"; utterance = "에너지를 간단히 설명해줘"; }"#,
    )
    .expect("evaluate brief definition query");

  let transcript: Vec<&String> = response
    .envelope
    .notes
    .iter()
    .filter(|n| n.starts_with("transcript:pnix:"))
    .collect();

  assert!(
    transcript.iter().any(|n| n.contains("에너지")),
    "missing concept term in transcript notes: {transcript:?}"
  );
  assert!(
    transcript.iter().any(|n| n.contains("물리량")),
    "missing definition body in transcript notes: {transcript:?}"
  );
  assert!(
    !transcript.iter().any(|n| n.contains("연결 지식")),
    "brief scope should not emit detailed-only connected knowledge: {transcript:?}"
  );
}

/// batch 85 (2026-04-15): P5.5 step 4.7 domain listing lane shaping parity.
/// kernel 이 `concept-domain-listing` route 를 직접 answer 해서 store contract 의
/// 핵심 carrier (`domain-query`, `domain-concept-count`, `domain-concept-term`,
/// `domain-listing:*` notes, `interp.domain.list.*`) 를 맞추는지 검증한다.
#[test]
fn batch85_kernel_domain_listing_contains_store_contract_fields() {
  use pnix_core::ontology::SemanticRecordValue;

  let mut kernel = PnixReplKernel::new(KernelPaths::from_data_dir(data_dir()));
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "수학 개념 뭐 있어?"; }"#,
    )
    .expect("evaluate domain listing query");

  assert_eq!(response.route, "concept-domain-listing");
  assert!(response.summary.contains("수학"));

  let facts: Vec<_> = response
    .envelope
    .records
    .iter()
    .filter_map(|record| match &record.value {
      SemanticRecordValue::ContextualFact(fact) => Some(fact),
      _ => None,
    })
    .collect();

  assert!(
    facts
      .iter()
      .any(|fact| fact.pred == "domain-query" && fact.obj == "수학"),
    "missing domain-query=수학"
  );
  assert!(
    facts
      .iter()
      .any(|fact| fact.pred == "domain-concept-count" && fact.obj.parse::<usize>().ok().is_some()),
    "missing numeric domain-concept-count"
  );
  assert!(
    facts
      .iter()
      .any(|fact| fact.pred == "domain-concept-term" && fact.obj == "덧셈"),
    "missing domain-concept-term=덧셈"
  );
  assert!(
    facts.iter().any(|fact| {
      fact.pred == "ontology-selected-interpretation" && fact.obj.starts_with("interp.domain.list.")
    }),
    "missing ontology-selected-interpretation with interp.domain.list.* prefix"
  );
  assert!(
    response
      .envelope
      .notes
      .iter()
      .any(|note| note == "domain-listing:domain:수학"),
    "missing domain-listing:domain:수학"
  );
  assert!(
    response
      .envelope
      .notes
      .iter()
      .any(|note| note.starts_with("domain-listing:count:")),
    "missing domain-listing:count:*"
  );
}

/// batch 86 (2026-04-15): P5.5 step 4.8 cross-concept lane shaping parity.
/// kernel 이 `cross-concept-comparison` route 를 직접 answer 해서 store contract 의
/// 핵심 carrier (`cross-concept-term-a`, `cross-concept-same-category`,
/// `cross-concept:term-*` notes, `interp.cross.*`) 와 transcript surface
/// (`역연산`, `사칙연산`) 를 맞추는지 검증한다.
#[test]
fn batch86_kernel_cross_concept_contains_store_contract_fields() {
  use pnix_core::ontology::SemanticRecordValue;

  let mut kernel = PnixReplKernel::new(KernelPaths::from_data_dir(data_dir()));
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "덧셈과 뺄셈의 관계는 뭐야?"; }"#,
    )
    .expect("evaluate cross-concept query");

  assert_eq!(response.route, "cross-concept-comparison");
  assert!(response.summary.contains("덧셈"));
  assert!(response.summary.contains("뺄셈"));
  assert!(
    response
      .envelope
      .notes
      .iter()
      .any(|note| note.starts_with("transcript:pnix:") && note.contains("역연산")),
    "missing inverse relation transcript note"
  );
  assert!(
    response
      .envelope
      .notes
      .iter()
      .any(|note| note.starts_with("transcript:pnix:") && note.contains("사칙연산")),
    "missing same-category transcript note"
  );

  let facts: Vec<_> = response
    .envelope
    .records
    .iter()
    .filter_map(|record| match &record.value {
      SemanticRecordValue::ContextualFact(fact) => Some(fact),
      _ => None,
    })
    .collect();

  assert!(
    facts
      .iter()
      .any(|fact| fact.pred == "cross-concept-term-a" && fact.obj == "덧셈"),
    "missing cross-concept-term-a=덧셈"
  );
  assert!(
    facts
      .iter()
      .any(|fact| fact.pred == "cross-concept-term-b" && fact.obj == "뺄셈"),
    "missing cross-concept-term-b=뺄셈"
  );
  assert!(
    facts
      .iter()
      .any(|fact| fact.pred == "cross-concept-same-category" && fact.obj == "true"),
    "missing cross-concept-same-category=true"
  );
  assert!(
    facts.iter().any(|fact| {
      fact.pred == "ontology-selected-interpretation" && fact.obj.starts_with("interp.cross.")
    }),
    "missing ontology-selected-interpretation with interp.cross.* prefix"
  );
  assert!(
    response
      .envelope
      .notes
      .iter()
      .any(|note| note == "cross-concept:term-a:덧셈"),
    "missing cross-concept:term-a:덧셈"
  );
  assert!(
    response
      .envelope
      .notes
      .iter()
      .any(|note| note == "cross-concept:term-b:뺄셈"),
    "missing cross-concept:term-b:뺄셈"
  );
}

/// batch 87 (2026-04-15): P5.5 step 4.9 single-clause sentence-analysis lane parity.
#[test]
fn batch87_kernel_sentence_analysis_contains_store_contract_fields() {
  use pnix_core::ontology::SemanticRecordValue;

  let mut kernel = PnixReplKernel::new(KernelPaths::from_data_dir(data_dir()));
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "철수가 사과를 줬다."; }"#,
    )
    .expect("evaluate sentence analysis query");

  assert_eq!(response.route, "sentence-semantic-analysis");
  let facts: Vec<_> = response
    .envelope
    .records
    .iter()
    .filter_map(|record| match &record.value {
      SemanticRecordValue::ContextualFact(fact) => Some(fact),
      _ => None,
    })
    .collect();

  assert!(
    facts
      .iter()
      .any(|fact| fact.pred == "sentence-verb" && fact.obj.contains("줬")),
    "missing sentence-verb carrier"
  );
  assert!(
    facts
      .iter()
      .any(|fact| fact.pred == "sentence-role" && fact.subj == "철수" && fact.obj == "subject"),
    "missing sentence-role subject carrier"
  );
  assert!(
    facts
      .iter()
      .any(|fact| fact.pred == "sentence-role" && fact.subj == "사과" && fact.obj == "object"),
    "missing sentence-role object carrier"
  );
  assert!(
    facts
      .iter()
      .any(|fact| fact.pred == "sentence-mood" && fact.obj == "declarative"),
    "missing sentence-mood=declarative"
  );
  assert!(
    facts.iter().any(|fact| {
      fact.pred == "ontology-selected-interpretation" && fact.obj.starts_with("interp.sentence.")
    }),
    "missing ontology-selected-interpretation with interp.sentence.* prefix"
  );
  assert!(
    response
      .envelope
      .notes
      .iter()
      .any(|note| note == "sentence-analysis:source:pnix-kernel"),
    "missing sentence-analysis source note"
  );
}

/// batch 88 (2026-04-15): P5.5 step 4.10 why / invert lane production route parity.
#[test]
fn batch88_kernel_why_query_contains_invert_contract_fields() {
  use pnix_core::ontology::SemanticRecordValue;

  let mut kernel = PnixReplKernel::new(KernelPaths::from_data_dir(data_dir()));
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "왜 힘은 F=ma야?"; }"#,
    )
    .expect("evaluate why query");

  assert_eq!(response.route, "ontology-invert-causal-inverse");
  let facts: Vec<_> = response
    .envelope
    .records
    .iter()
    .filter_map(|record| match &record.value {
      SemanticRecordValue::ContextualFact(fact) => Some(fact),
      _ => None,
    })
    .collect();

  assert!(
    facts.iter().any(|fact| {
      fact.pred == "ontology-selected-interpretation" && fact.obj.starts_with("interp.invert.")
    }),
    "missing ontology-selected-interpretation with interp.invert.* prefix"
  );
  assert!(
    response
      .envelope
      .notes
      .iter()
      .any(|note| note == "invert-trigger:causal-inverse"),
    "missing invert-trigger note"
  );
  assert!(
    response
      .envelope
      .notes
      .iter()
      .any(|note| note.starts_with("truth-regime:")),
    "missing truth-regime note"
  );
}

#[test]
fn batch89_kernel_contradiction_query_contains_branch_point_contract_fields() {
  use pnix_core::ontology::SemanticRecordValue;

  let concepts_dir = temp_px_path("concepts");
  fs::create_dir_all(&concepts_dir).expect("create concepts dir");
  write_px(
    &concepts_dir.join("contradiction-light.px"),
    r#"
    [
      {
        term-ko = "빛";
        definition-ko = "전자기파의 한 형태이다.";
        context = "Physics.Optics";
        domain = "물리";
        source-ref = "fixture:wave";
      }
      {
        term-ko = "빛";
        definition-ko = "광자로 취급되는 입자적 현상이다.";
        context = "Physics.Quantum";
        domain = "물리";
        source-ref = "fixture:particle";
      }
    ]
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.concepts_dir = concepts_dir;
  let mut kernel = PnixReplKernel::new(paths);
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "빛은 모순이야?"; }"#,
    )
    .expect("evaluate contradiction query");

  assert_eq!(response.route, "ontology-invert-contradiction-detect");
  let facts: Vec<_> = response
    .envelope
    .records
    .iter()
    .filter_map(|record| match &record.value {
      SemanticRecordValue::ContextualFact(fact) => Some(fact),
      _ => None,
    })
    .collect();

  assert!(
    facts.iter().any(|fact| fact.pred == "branch-point"),
    "missing branch-point carrier"
  );
  assert!(
    facts
      .iter()
      .filter(|fact| fact.pred == "definition-ko")
      .count()
      >= 2,
    "missing conflicting definition carriers"
  );
  assert!(
    facts.iter().any(|fact| {
      fact.pred == "ontology-selected-interpretation" && fact.obj.starts_with("interp.invert.")
    }),
    "missing ontology-selected-interpretation with interp.invert.* prefix"
  );
  assert!(
    response
      .envelope
      .notes
      .iter()
      .any(|note| note == "invert-trigger:contradiction-detect"),
    "missing contradiction invert-trigger note"
  );
  assert!(
    response
      .envelope
      .notes
      .iter()
      .any(|note| note == "branch-count:2"),
    "missing branch-count note"
  );
}

#[test]
fn batch90_kernel_multiclause_sentence_analysis_contains_store_contract_fields() {
  use pnix_core::ontology::SemanticRecordValue;

  let mut kernel = PnixReplKernel::new(KernelPaths::from_data_dir(data_dir()));
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "밥을 먹고 학교에 갔다."; }"#,
    )
    .expect("evaluate multi-clause sentence analysis");

  assert_eq!(response.route, "multi-clause-analysis");
  let facts: Vec<_> = response
    .envelope
    .records
    .iter()
    .filter_map(|record| match &record.value {
      SemanticRecordValue::ContextualFact(fact) => Some(fact),
      _ => None,
    })
    .collect();

  assert!(
    facts
      .iter()
      .filter(|fact| fact.pred == "clause-text")
      .count()
      >= 2,
    "missing multi-clause clause-text carriers"
  );
  assert!(
    facts
      .iter()
      .any(|fact| fact.pred == "clause-relation" && fact.obj == "sequential"),
    "missing sequential clause-relation"
  );
  assert!(
    facts.iter().any(|fact| {
      fact.pred == "ontology-selected-interpretation" && fact.obj.starts_with("interp.multiclause.")
    }),
    "missing ontology-selected-interpretation with interp.multiclause.* prefix"
  );
  assert!(
    response
      .envelope
      .notes
      .iter()
      .any(|note| note == "multi-clause-analysis:source:pnix-kernel"),
    "missing multi-clause source note"
  );
  assert!(
    response
      .envelope
      .notes
      .iter()
      .any(|note| note == "clause-count:2"),
    "missing clause-count note"
  );
}

#[test]
fn batch91_kernel_source_metadata_facts_are_emitted_in_standalone_runtime() {
  use pnix_core::ontology::SemanticRecordValue;
  use std::collections::BTreeSet;

  let mut kernel = PnixReplKernel::new(KernelPaths::from_data_dir(data_dir()));
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "힘은 뭐야"; }"#,
    )
    .expect("evaluate definition query");

  let facts: Vec<_> = response
    .envelope
    .records
    .iter()
    .filter_map(|record| match &record.value {
      SemanticRecordValue::ContextualFact(fact) => Some(fact),
      _ => None,
    })
    .collect();

  assert!(
    facts.iter().any(|fact| {
      fact.context.0 == "Pnix.KernelSource"
        && fact.pred == "kernel-source-field"
        && fact.obj == "definition-ko"
        && fact.layer.0 == "L1"
        && fact.status == pnix_core::ontology::MeaningStatus::Accepted
        && (fact.confidence - 1.0).abs() < f64::EPSILON
    }),
    "missing Pnix.KernelSource field carrier"
  );
  assert!(
    facts.iter().any(|fact| {
      fact.context.0 == "Pnix.KernelSource"
        && fact.pred == "kernel-source-value"
        && fact.obj.starts_with("definition-ko=")
    }),
    "missing Pnix.KernelSource value carrier"
  );
  assert!(
    facts.iter().any(|fact| {
      fact.context.0 == "Pnix.KernelSource"
        && fact.pred == "kernel-source-list-field"
        && fact.obj == "related-concepts"
    }),
    "missing Pnix.KernelSource list field carrier"
  );
  assert!(
    facts.iter().any(|fact| {
      fact.context.0 == "Pnix.KernelSource"
        && fact.pred == "kernel-source-list-item"
        && fact.obj.starts_with("related-concepts=")
    }),
    "missing Pnix.KernelSource list item carrier"
  );
  let metadata_ids = facts
    .iter()
    .filter(|fact| fact.context.0 == "Pnix.KernelSource")
    .filter_map(|fact| fact.id.as_ref().map(|id| id.0.clone()))
    .collect::<Vec<_>>();
  let unique_metadata_ids = metadata_ids.iter().cloned().collect::<BTreeSet<_>>();
  assert_eq!(
    metadata_ids.len(),
    unique_metadata_ids.len(),
    "Pnix.KernelSource metadata fact ids must be unique: {metadata_ids:?}"
  );
}

#[test]
fn query_classifiers_kernel_source_metadata_owns_standalone_payload_shape() {
  use pnix_core::ontology::SemanticRecordValue;

  let query_classifiers = temp_px_path("query-classifiers-runtime-metadata.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      kernel-source-metadata = {
        context = "Standalone.CustomSource";
        layer = "L3";
        status = "candidate";
        confidence = "0.42";
        field-predicate = "source-field-name";
        value-predicate = "source-field-value";
        list-field-predicate = "source-list-name";
        list-item-predicate = "source-list-item";
        field-object-template = "field:${term}:${field}:${source-predicate}";
        value-object-template = "value:${term}:${field}:${source-predicate}:${value}";
        list-field-object-template = "list:${term}:${field}:${source-predicate}";
        list-item-object-template = "item:${term}:${index}:${field}:${source-predicate}:${value}";
      };
      definition-query-rules = [
        { match-any = ["뭐"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "덧셈은 뭐야"; }"#,
    )
    .expect("evaluate definition query");

  let facts: Vec<_> = response
    .envelope
    .records
    .iter()
    .filter_map(|record| match &record.value {
      SemanticRecordValue::ContextualFact(fact) => Some(fact),
      _ => None,
    })
    .collect();

  assert!(
    facts.iter().any(|fact| {
      fact.context.0 == "Standalone.CustomSource"
        && fact.pred == "source-field-name"
        && fact.obj == "field:덧셈:definition-ko:definition-ko"
        && fact.layer.0 == "L3"
        && fact.status == pnix_core::ontology::MeaningStatus::Candidate
        && (fact.confidence - 0.42).abs() < 1e-9
    }),
    "missing custom source field carrier"
  );
  assert!(
    facts.iter().any(|fact| {
      fact.context.0 == "Standalone.CustomSource"
        && fact.pred == "source-field-value"
        && fact.obj
          == "value:덧셈:definition-ko:definition-ko:두 수를 합하여 하나의 수를 구하는 이항연산이다. a + b = c에서 a, b는 피가수이고 c는 합이다. 교환법칙과 결합법칙이 성립한다."
    }),
    "missing custom source value carrier"
  );
  assert!(
    facts.iter().any(|fact| {
      fact.context.0 == "Standalone.CustomSource"
        && fact.pred == "source-list-name"
        && fact.obj == "list:덧셈:related-concepts:related-concept"
    }),
    "missing custom source list field carrier"
  );
  assert!(
    facts.iter().any(|fact| {
      fact.context.0 == "Standalone.CustomSource"
        && fact.pred == "source-list-item"
        && fact.obj == "item:덧셈:0:related-concepts:related-concept:뺄셈"
    }),
    "missing custom source list item carrier"
  );
}

#[test]
fn query_classifiers_kernel_source_metadata_templates_reject_unsupported_placeholders() {
  let query_classifiers = temp_px_path("query-classifiers-runtime-metadata-invalid.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      kernel-source-metadata = {
        field-predicate = "source-field-name";
        value-predicate = "source-field-value";
        list-field-predicate = "source-list-name";
        list-item-predicate = "source-list-item";
        value-object-template = "value:${bogus}";
      };
      definition-query-rules = [
        { match-any = ["뭐"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "덧셈은 뭐야"; }"#,
    )
    .expect_err("unsupported metadata placeholder should fail");

  assert!(
    err.to_string().contains(
      "kernel-source-metadata 'value-object-template' uses unsupported placeholder '${bogus}'"
    ),
    "{err:#}"
  );
}

#[test]
fn query_classifiers_kernel_source_metadata_rejects_invalid_status() {
  let query_classifiers = temp_px_path("query-classifiers-runtime-metadata-invalid-status.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      kernel-source-metadata = {
        status = "bogus";
        field-predicate = "source-field-name";
        value-predicate = "source-field-value";
        list-field-predicate = "source-list-name";
        list-item-predicate = "source-list-item";
      };
      definition-query-rules = [
        { match-any = ["뭐"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "덧셈은 뭐야"; }"#,
    )
    .expect_err("invalid metadata status should fail");

  assert!(
    err
      .to_string()
      .contains("invalid 'status' for kernel-source-metadata"),
    "{err:#}"
  );
}

#[test]
fn query_classifiers_kernel_source_metadata_rejects_out_of_range_confidence() {
  let query_classifiers = temp_px_path("query-classifiers-runtime-metadata-invalid-confidence.px");
  write_px(
    &query_classifiers,
    r#"
    {
      query-dispatch-priority = ["why" "property" "definition"];
      kernel-dispatch-routes = {
        definition = "concept-definition-lookup";
        property = "concept-predicate-lookup";
        held = "lightweight-korean-dialogue-held";
      };
      held-reason-keys = {
        requires-context = "requires-context";
        unknown-term = "unknown-term";
      };
      held-reason-rules = [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ];
      kernel-source-fact-fields = [
        { field = "definition-ko"; predicate = "definition-ko"; }
      ];
      kernel-source-list-fields = [
        { field = "related-concepts"; predicate = "related-concept"; }
      ];
      kernel-source-metadata = {
        confidence = "1.5";
        field-predicate = "source-field-name";
        value-predicate = "source-field-value";
        list-field-predicate = "source-list-name";
        list-item-predicate = "source-list-item";
      };
      definition-query-rules = [
        { match-any = ["뭐"]; }
      ];
      predicate-classifiers = [];
      concept-what-markers = [];
      concept-definition-suffixes = [];
      concept-explain-markers = [];
      concept-explain-skip-tokens = [];
      question-word-stems = ["뭐"];
      term-extraction-suffixes = [];
      term-extraction-particle-kinds = ["subject" "topic" "genitive" "conjunctive"];
      term-normalization-trim-chars = ["?" "!" "," "." "\"" "'"];
      term-fallback-policy = "known-concept-token-scan";
    }
    "#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "덧셈은 뭐야"; }"#,
    )
    .expect_err("out of range metadata confidence should fail");

  assert!(
    err
      .to_string()
      .contains("'confidence' in kernel-source-metadata must be between 0.0 and 1.0"),
    "{err:#}"
  );
}

#[test]
fn query_classifiers_source_fact_rules_own_standalone_fact_semantics() {
  use pnix_core::ontology::{MeaningStatus, SemanticRecordValue};

  let query_classifiers = temp_px_path("query-classifiers-source-fact-semantics.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"
      [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ]
    "#,
    r#"
      [
        {
          field = "definition-ko";
          predicate = "definition-ko";
          context = "Standalone.CustomFact";
          layer = "L2";
          status = "candidate";
          confidence = "0.37";
          object-template = "scalar:${term}:${field}:${predicate}:${context}:${value}";
        }
      ]
    "#,
    r#"
      [
        {
          field = "related-concepts";
          predicate = "related-concept";
          context = "Standalone.CustomList";
          layer = "L3";
          status = "held";
          confidence = "0.61";
          object-template = "list:${term}:${index}:${field}:${predicate}:${context}:${value}";
        }
      ]
    "#,
    r#"
      [
        { match-any = ["뭐"]; }
      ]
    "#,
    r#"[]"#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let response = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "덧셈은 뭐야"; }"#,
    )
    .expect("evaluate definition query");

  let facts: Vec<_> = response
    .envelope
    .records
    .iter()
    .filter_map(|record| match &record.value {
      SemanticRecordValue::ContextualFact(fact) => Some(fact),
      _ => None,
    })
    .collect();

  assert!(
    facts.iter().any(|fact| {
      fact.context.0 == "Standalone.CustomFact"
        && fact.pred == "definition-ko"
        && fact.layer.0 == "L2"
        && fact.status == MeaningStatus::Candidate
        && (fact.confidence - 0.37).abs() < 1e-9
        && fact
          .obj
          .starts_with("scalar:덧셈:definition-ko:definition-ko:Standalone.CustomFact:")
    }),
    "missing custom scalar source fact carrier"
  );
  assert!(
    facts.iter().any(|fact| {
      fact.context.0 == "Standalone.CustomList"
        && fact.pred == "related-concept"
        && fact.layer.0 == "L3"
        && fact.status == MeaningStatus::Held
        && (fact.confidence - 0.61).abs() < 1e-9
        && fact.obj == "list:덧셈:0:related-concepts:related-concept:Standalone.CustomList:뺄셈"
    }),
    "missing custom list source fact carrier"
  );
}

#[test]
fn query_classifiers_source_fact_rules_reject_unsupported_object_placeholder() {
  let query_classifiers = temp_px_path("query-classifiers-source-fact-invalid-placeholder.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"
      [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ]
    "#,
    r#"
      [
        { field = "definition-ko"; predicate = "definition-ko"; object-template = "bad:${bogus}"; }
      ]
    "#,
    r#"
      [
        { field = "related-concepts"; predicate = "related-concept"; }
      ]
    "#,
    r#"
      [
        { match-any = ["뭐"]; }
      ]
    "#,
    r#"[]"#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "덧셈은 뭐야"; }"#,
    )
    .expect_err("unsupported source fact placeholder should fail");

  assert!(
    err.to_string().contains(
      "kernel-source-fact-fields 'object-template' uses unsupported placeholder '${bogus}'"
    ),
    "{err:#}"
  );
}

#[test]
fn query_classifiers_source_fact_rules_reject_invalid_status() {
  let query_classifiers = temp_px_path("query-classifiers-source-fact-invalid-status.px");
  write_query_classifier_fixture(
    &query_classifiers,
    r#"
      [
        { when = "known-term"; reason-key = "requires-context"; term-source = "matched-term"; }
        { when = "unknown-term"; reason-key = "unknown-term"; term-source = "first-extracted-term"; }
      ]
    "#,
    r#"
      [
        { field = "definition-ko"; predicate = "definition-ko"; status = "bogus"; }
      ]
    "#,
    r#"
      [
        { field = "related-concepts"; predicate = "related-concept"; }
      ]
    "#,
    r#"
      [
        { match-any = ["뭐"]; }
      ]
    "#,
    r#"[]"#,
  );

  let mut paths = KernelPaths::from_data_dir(data_dir());
  paths.query_classifiers_path = query_classifiers;
  let mut kernel = PnixReplKernel::new(paths);
  let err = kernel
    .evaluate_px_source(
      r#"{ kind = "ontology-query"; scope = "standard"; utterance = "덧셈은 뭐야"; }"#,
    )
    .expect_err("invalid source fact status should fail");

  assert!(
    err
      .to_string()
      .contains("invalid 'status' for kernel-source-fact-fields entry"),
    "{err:#}"
  );
}
