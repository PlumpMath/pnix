//! pnix-query-runtime standalone kernel smoke — Rust integration level.
//!
//! `scripts/run-px-phase7-gate.sh` 에 이미 binary level smoke 6 종이 있지만,
//! cargo rebuild 비용 때문에 병렬 실행 시 느리다. 이 Rust test 는 kernel
//! library 를 in-process 호출로 돌려서:
//!   1. 각 dispatch path (accept / held / held-with-context) 가 정확한
//!      KernelResponse 필드를 채우는지 검증
//!   2. held → 다음 query 를 held_state 로 연결하는 multi-turn dispatch 를
//!      검증 (binary smoke 는 `run_stdin_once` 라 불가)
//!
//! 이 test 는 `crates/pnix-query-runtime/data/` 를 기본 data dir 로 쓰므로
//! doghouse / puck 의존 없이 standalone 동작을 검증한다.

use pnix_query_runtime::{response_document, KernelPaths, PnixReplKernel};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

fn make_query(utterance: &str, scope: &str) -> String {
  format!(
    r#"{{ kind = "ontology-query"; utterance = "{}"; scope = "{}"; }}"#,
    utterance, scope
  )
}

fn make_seeded_query(utterance: &str, scope: &str, seeded_term: &str) -> String {
  format!(
    r#"{{ kind = "ontology-query"; utterance = "{}"; scope = "{}"; seeded-term = "{}"; }}"#,
    utterance, scope, seeded_term
  )
}

fn make_handoff_query(utterance: &str, scope: &str) -> String {
  format!(
    r#"{{ kind = "ontology-query"; utterance = "{}"; scope = "{}"; classifier-mode = "handoff"; }}"#,
    utterance, scope
  )
}

fn new_kernel() -> PnixReplKernel {
  PnixReplKernel::new(KernelPaths::default())
}

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(1);

fn write_temp_query_file(source: &str) -> PathBuf {
  let id = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
  let path = std::env::temp_dir().join(format!(
    "pnix-query-runtime-m1-10-{}-{}.px",
    std::process::id(),
    id
  ));
  fs::write(&path, source).expect("write temp query file");
  path
}

#[test]
fn accept_path_emits_pipeline_trace_and_facts() {
  let mut kernel = new_kernel();
  let response = kernel
    .evaluate_px_source(&make_query("힘이 뭐야?", "brief"))
    .expect("kernel eval accept path");

  assert!(!response.route.is_empty(), "route must be populated");
  assert!(
    !response.transcript.is_empty(),
    "transcript must have lines"
  );
  assert!(
    !response.envelope.records.is_empty(),
    "accept path must emit facts to envelope"
  );
  // 6-axis evaluation 축 notes 중 하나는 반드시 envelope 에 찍힌다.
  let notes_joined = response.envelope.notes.join("\n");
  assert!(
    notes_joined.contains("ontology-evaluation-axes:"),
    "envelope notes must include ontology-evaluation-axes line. notes: {notes_joined}"
  );
  assert!(
    notes_joined.contains("coherence="),
    "envelope notes must include coherence axis"
  );
  assert!(
    notes_joined.contains("score="),
    "envelope notes must include score axis"
  );
}

#[test]
fn source_query_runs_through_pnix_eval_expression_path() {
  let mut kernel = new_kernel();
  let source = r#"
      let
        queryKind = "ontology-query";
        requestedScope = "brief";
      in {
        kind = queryKind;
        utterance = "힘이 뭐야?";
        scope = requestedScope;
      }
    "#;
  let response = kernel
    .evaluate_px_source(source)
    .expect("kernel eval via pnix-eval expression path");

  assert!(
    response.summary.contains("힘"),
    "pnix-eval-backed source query should still resolve concept response"
  );
}

#[test]
fn file_query_runs_through_pnix_eval_file_path() {
  let mut kernel = new_kernel();
  let path = write_temp_query_file(
    r#"
          let
            queryKind = "ontology-query";
          in {
            kind = queryKind;
            utterance = "힘이 뭐야?";
            scope = "brief";
          }
        "#,
  );
  let response = kernel
    .evaluate_px_file(&path)
    .expect("kernel eval via pnix-eval file path");

  assert!(
    response.summary.contains("힘"),
    "pnix-eval-backed file query should still resolve concept response"
  );

  let _ = fs::remove_file(path);
}

#[test]
fn response_document_projection_matches_cli_stdout() {
  let mut kernel = new_kernel();
  let query = make_query("힘이 뭐야?", "brief");
  let response = kernel
    .evaluate_px_source(&query)
    .expect("kernel eval response-document path");

  let projection = response_document::response_document_projection(&response);
  assert!(
    projection.fragment_found,
    "response-document fragment missing"
  );
  assert_eq!(projection.native_text, response.response_document_org);
  assert_eq!(
    projection.response_document_px,
    response.response_document_px
  );
  assert_eq!(projection.fragment_kind, Some("response-document"));
  assert_eq!(projection.fragment_visibility, Some("dev"));
  assert_eq!(
    projection.fragment_content_org,
    Some(response.response_document_org.as_str())
  );
  assert_eq!(
    projection.fragment_content_px,
    Some(response.response_document_px.as_str())
  );
  let expected_html =
    response_document::response_document_html(response.response_document_org.as_str());
  assert_eq!(
    projection.fragment_content_html,
    Some(expected_html.as_str()),
    "response-document fragment should carry helper-backed HTML projection"
  );
  let expected_speech =
    response_document::response_document_speech_text(response.response_document_org.as_str());
  assert_eq!(
    projection.fragment_content_speech,
    Some(expected_speech.as_str()),
    "response-document fragment should carry helper-backed speech projection"
  );

  let mut command = Command::new(env!("CARGO_BIN_EXE_pnix-query-repl"));
  command.arg("--stdin");
  command.stdin(Stdio::piped());
  command.stdout(Stdio::piped());
  let mut child = command.spawn().expect("spawn pnix-query-repl");
  {
    let mut stdin = child.stdin.take().expect("pipe pnix-query-repl stdin");
    stdin
      .write_all(query.as_bytes())
      .expect("write query to pnix-query-repl stdin");
  }
  let output = child
    .wait_with_output()
    .expect("wait for pnix-query-repl output");
  assert!(
    output.status.success(),
    "pnix-query-repl should exit successfully: {:?}",
    output
  );
  let stdout = String::from_utf8(output.stdout).expect("pnix-query-repl stdout should be utf-8");
  assert_eq!(
    stdout.trim_end(),
    projection.native_text.trim_end(),
    "pnix-query-repl stdout should match response-document native text"
  );
}

#[test]
fn response_document_speech_text_normalizes_marked_lines() {
  let speech =
    response_document::response_document_speech_text("* 제목\n** 강조\n- 항목\n~주석~\nplain");

  assert_eq!(speech, "제목 강조 항목 주석 plain");
}

#[test]
fn held_unknown_term_emits_follow_up_hint() {
  let mut kernel = new_kernel();
  let response = kernel
    .evaluate_px_source(&make_query("XYZ모르는개념ABC 뭐야?", "brief"))
    .expect("kernel eval held path");

  assert!(
    response.follow_up_hint.is_some(),
    "held path must emit follow_up_hint"
  );
  let notes_joined = response.envelope.notes.join("\n");
  assert!(
    notes_joined.contains("held-reason:unknown-term"),
    "held-unknown-term path must emit held-reason:unknown-term note. notes: {notes_joined}"
  );
}

#[test]
fn held_requires_context_emits_held_term_note() {
  let mut kernel = new_kernel();
  let response = kernel
    .evaluate_px_source(&make_query("2 더하기 3", "brief"))
    .expect("kernel eval held-with-context path");

  assert!(
    response.follow_up_hint.is_some(),
    "held-with-context path must emit follow_up_hint"
  );
  let notes_joined = response.envelope.notes.join("\n");
  assert!(
    notes_joined.contains("held-reason:requires-context"),
    "held-with-context path must emit held-reason:requires-context. notes: {notes_joined}"
  );
  assert!(
    notes_joined.contains("held-term:"),
    "held-with-context path must emit held-term note. notes: {notes_joined}"
  );
}

#[test]
fn handoff_classifier_mode_routes_recipe_request() {
  let mut kernel = new_kernel();
  let response = kernel
    .evaluate_px_source(&make_handoff_query("메모 열어줘", "brief"))
    .expect("kernel eval handoff path");

  assert_eq!(response.route, "recipe-os-handoff");
  let notes_joined = response.envelope.notes.join("\n");
  assert!(
    notes_joined.contains("tool-handoff:code:"),
    "handoff path must emit tool-handoff shell code. notes: {notes_joined}"
  );
  assert!(response
    .envelope
    .records
    .iter()
    .any(|record| match &record.value {
      pnix_core::ontology::SemanticRecordValue::ContextualFact(fact) => {
        fact.pred == "handoff-template" && fact.obj == "pnix.recipe.app.launch"
      }
      _ => false,
    }));
}

#[test]
fn detailed_scope_adds_connected_knowledge() {
  let mut kernel = new_kernel();
  let brief = kernel
    .evaluate_px_source(&make_query("힘이 뭐야?", "brief"))
    .expect("kernel eval brief");
  let detailed = kernel
    .evaluate_px_source(&make_query("힘이 뭐야?", "detailed"))
    .expect("kernel eval detailed");

  let brief_body = brief.transcript.join("\n");
  let detailed_body = detailed.transcript.join("\n");

  assert!(
    detailed_body.contains("연결 지식으로"),
    "detailed scope must emit 연결 지식 sentence. body: {detailed_body}"
  );
  assert!(
    !brief_body.contains("연결 지식으로"),
    "brief scope must NOT emit 연결 지식 sentence. body: {brief_body}"
  );
}

#[test]
fn definition_suffix_query_resolves_definition_route() {
  let mut kernel = new_kernel();
  let response = kernel
    .evaluate_px_source(&make_query("함수란 무엇인가?", "standard"))
    .expect("kernel eval definition suffix query");

  assert_eq!(
    response.route, "concept-definition-lookup",
    "definition suffix query must resolve to concept-definition-lookup"
  );
  assert!(
    response.summary.contains("함수"),
    "definition suffix query must resolve 함수 concept. summary: {}",
    response.summary
  );
  let notes_joined = response.envelope.notes.join("\n");
  assert!(
    !notes_joined.contains("held-reason:unknown-term"),
    "definition suffix query must not fall back to unknown-term held. notes: {notes_joined}"
  );
}

#[test]
fn multi_turn_held_reopen_carries_term() {
  // binary smoke 는 `run_stdin_once` 라 kernel state 가 query 간 유지
  // 되지 않는다. in-process test 는 held_state 가 두 번째 query 에 영향
  // 주는지 검증할 수 있다. held 후 짧은 후속 utterance 를 던지면 같은
  // kernel instance 가 held context 를 재사용한다.
  let mut kernel = new_kernel();
  let first = kernel
    .evaluate_px_source(&make_query("2 더하기 3", "brief"))
    .expect("held query 1");
  assert!(first.follow_up_hint.is_some(), "first query must be held");

  // 두 번째 query — follow-up 성격의 짧은 후속. held_state 가 같은 kernel
  // 인스턴스에 남아 있으면 reopen 가능하다.
  let second = kernel
    .evaluate_px_source(&make_query("더 알려줘", "brief"))
    .expect("follow-up query");
  // held_state 가 유지되면 second 도 관련 응답을 내거나 재-held 를 낸다.
  // 최소한 empty response 가 아니어야 한다.
  assert!(
    !second.transcript.is_empty() || second.follow_up_hint.is_some(),
    "follow-up query must produce transcript or follow-up hint, not empty"
  );
}

#[test]
fn seeded_continuation_query_resolves_continuation_route() {
  let mut kernel = new_kernel();
  let response = kernel
    .evaluate_px_source(&make_seeded_query("예를 들어줘", "standard", "힘"))
    .expect("seeded continuation query");

  assert_eq!(
    response.route, "concept-continuation-example",
    "seeded continuation query must resolve to continuation route"
  );
  let notes_joined = response.envelope.notes.join("\n");
  assert!(
    notes_joined.contains("continuation:kind:example"),
    "continuation query must emit continuation kind note. notes: {notes_joined}"
  );
  assert!(
    notes_joined.contains("continuation:from-term:힘"),
    "continuation query must emit continuation source term note. notes: {notes_joined}"
  );
}

#[test]
fn multi_turn_concept_then_continuation_uses_last_term_mainline() {
  let mut kernel = new_kernel();
  let first = kernel
    .evaluate_px_source(&make_query("힘이 뭐야?", "standard"))
    .expect("concept query before continuation");
  assert_eq!(first.route, "concept-definition-lookup");

  let second = kernel
    .evaluate_px_source(&make_query("예를 들어줘", "standard"))
    .expect("continuation query after concept");
  assert_eq!(
    second.route, "concept-continuation-example",
    "kernel must reuse last term for continuation mainline"
  );
  assert!(
    second.summary.contains("힘"),
    "continuation summary must mention reused term. summary: {}",
    second.summary
  );
}

#[test]
fn malformed_query_kind_returns_error() {
  let mut kernel = new_kernel();
  let bad = r#"{ kind = "wrong-kind"; utterance = "x"; scope = "brief"; }"#;
  let result = kernel.evaluate_px_source(bad);
  assert!(
    result.is_err(),
    "malformed query kind must return Err, got: {result:?}"
  );
  let err = result.unwrap_err().to_string();
  assert!(
    err.contains("unsupported query kind"),
    "error must mention 'unsupported query kind'. got: {err}"
  );
}

#[test]
fn missing_kind_returns_error() {
  let mut kernel = new_kernel();
  let bad = r#"{ utterance = "x"; scope = "brief"; }"#;
  let result = kernel.evaluate_px_source(bad);
  assert!(
    result.is_err(),
    "missing kind must return Err, got: {result:?}"
  );
  let err = result.unwrap_err().to_string();
  assert!(
    err.contains("missing 'kind'"),
    "error must mention missing 'kind'. got: {err}"
  );
}
