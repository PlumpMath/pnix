use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/puncheetah-code-session-contracts.px")
}

fn dry_run_fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/puncheetah-code-patch-preview-dry-run.px")
}

fn human_summary_fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/puncheetah-code-human-summary-render.px")
}

fn converged_human_summary_fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/puncheetah-code-converged-human-summary.px")
}

fn front_door_e2e_fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/puncheetah-code-front-door-e2e.px")
}

fn as_attrs(v: &Value) -> &BTreeMap<String, Value> {
  match v {
    Value::AttrSet(m) => m,
    other => panic!("expected attrset, got {:?}", other),
  }
}

fn as_list(v: &Value) -> &Vec<Value> {
  match v {
    Value::List(items) => items,
    other => panic!("expected list, got {:?}", other),
  }
}

fn as_str(v: &Value) -> &str {
  match v {
    Value::String(s) => s,
    Value::StringContext { text, .. } => text,
    other => panic!("expected string, got {:?}", other),
  }
}

fn as_bool(v: &Value) -> bool {
  match v {
    Value::Bool(b) => *b,
    other => panic!("expected bool, got {:?}", other),
  }
}

fn as_i64(v: &Value) -> i64 {
  match v {
    Value::Int(i) => *i,
    other => panic!("expected int, got {:?}", other),
  }
}

fn get<'a>(v: &'a Value, key: &str) -> &'a Value {
  let attrs = as_attrs(v);
  attrs.get(key).unwrap_or_else(|| {
    panic!(
      "missing key `{}`; available: {:?}",
      key,
      attrs.keys().collect::<Vec<_>>()
    )
  })
}

fn has_key(v: &Value, key: &str) -> bool {
  as_attrs(v).contains_key(key)
}

fn is_null(v: &Value) -> bool {
  matches!(v, Value::Null)
}

fn is_attrset(v: &Value) -> bool {
  matches!(v, Value::AttrSet(_))
}

fn eval_fixture_with_large_stack(path: &Path) -> Value {
  let path = path.to_path_buf();
  std::thread::Builder::new()
    .name("puncheetah-code-heavy-fixture-eval".to_string())
    .stack_size(64 * 1024 * 1024)
    .spawn(move || eval_file(&path).expect("heavy puncheetah code fixture"))
    .expect("spawn eval thread")
    .join()
    .expect("eval thread panicked")
}

#[test]
fn session_contracts_evaluate_with_pnix_eval_not_nix() {
  let run = eval_file(&fixture_path()).expect("puncheetah code session contracts fixture");
  assert_eq!(
    as_str(get(&run, "proof")),
    "puncheetah-code-session-contracts"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(get(meta, "index"), "owner")),
    "puncheetah.contract.project-index-scope-cache.v0"
  );
  assert_eq!(
    as_str(get(get(meta, "group"), "owner")),
    "puncheetah.contract.client-command-group.v0"
  );
  assert_eq!(
    as_str(get(get(meta, "lifter"), "owner")),
    "puncheetah.contract.project-patch-request-lifter.v0"
  );
}

#[test]
fn compact_project_index_builds_scope_cache_without_source_or_primary_file() {
  let run = eval_file(&fixture_path()).unwrap();
  let scope = get(&run, "scope-cache");

  assert_eq!(
    as_str(get(scope, "outcome")),
    "project-index-scope-cache-built"
  );
  assert!(as_bool(get(scope, "verified")));
  assert_eq!(as_i64(get(scope, "total_index_file_count")), 42);
  assert_eq!(as_i64(get(scope, "source_unit_count")), 1);
  assert!(as_bool(get(scope, "files_elided")));
  assert!(!as_bool(get(scope, "full_file_list_sent")));

  let source_units = as_list(get(scope, "source_units"));
  assert_eq!(source_units.len(), 1);
  assert_eq!(as_str(get(&source_units[0], "path")), "src/unit.ext");
  assert!(as_bool(get(&source_units[0], "language_context_elided")));
  assert!(!has_key(&source_units[0], "open_language_id"));

  let manifest = get(scope, "project_manifest");
  assert!(as_bool(get(manifest, "shortcut_fields_elided")));
  assert!(as_bool(get(manifest, "source_payload_elided")));
  assert!(as_bool(get(manifest, "project_identity_not_bound")));
  assert!(as_bool(get(manifest, "primary_target_not_bound")));
  assert_eq!(
    as_str(get(manifest, "test_command")),
    "project test command"
  );
  assert!(!has_key(manifest, "project_id"));
  assert!(!has_key(manifest, "primary_file"));
  assert!(!has_key(manifest, "language"));
  assert!(!has_key(manifest, "source_text"));

  let editor = get(scope, "active_editor");
  assert!(as_bool(get(editor, "shortcut_fields_elided")));
  assert!(as_bool(get(editor, "source_payload_elided")));
  assert!(as_bool(get(editor, "primary_target_not_bound")));
  assert_eq!(as_str(get(editor, "active_document_version")), "doc-v1");
  assert!(as_bool(get(editor, "selection_present")));
  assert!(!has_key(editor, "active_uri"));
  assert!(!has_key(editor, "file"));
  assert!(!has_key(editor, "source_text"));

  let receipt = get(scope, "receipt");
  assert!(as_bool(get(receipt, "identity_shortcut_rejected")));
  assert!(as_bool(get(receipt, "single_file_shortcut_rejected")));
  assert!(as_bool(get(receipt, "no_language_or_fixture_branch")));
  assert!(!as_bool(get(receipt, "file_contents_read")));
}

#[test]
fn client_command_group_exposes_only_translated_commands() {
  let run = eval_file(&fixture_path()).unwrap();
  let group = get(&run, "client-command-group");

  assert_eq!(
    as_str(get(group, "schema")),
    "puncheetah.client-command-group.v0"
  );
  assert_eq!(as_i64(get(group, "command_count")), 2);
  let commands = as_list(get(group, "commands"));
  assert_eq!(commands.len(), 2);

  let security = get(group, "security");
  assert!(!as_bool(get(
    security,
    "client_receives_internal_linear_plan"
  )));
  assert!(as_bool(get(
    security,
    "client_receives_translated_commands_only"
  )));
  assert!(as_bool(get(security, "public_command_ast_required")));
  assert!(!as_bool(get(security, "raw_eval_allowed")));
  assert!(!as_bool(get(security, "file_write_allowed")));

  let execution = get(group, "execution_contract");
  assert_eq!(
    as_str(get(execution, "command_ast_schema")),
    "puncheetah.client-command-ast.v0"
  );
  assert!(as_bool(get(execution, "commands_are_ordered")));
  assert!(as_bool(get(execution, "commands_are_client_executable")));
  assert!(as_bool(get(
    execution,
    "server_internal_linear_plan_elided"
  )));
  assert!(as_bool(get(execution, "client_returns_observations_only")));

  let first_ast = get(&commands[0], "command_ast");
  assert_eq!(
    as_str(get(first_ast, "schema")),
    "puncheetah.client-command-ast.v0"
  );
  assert_eq!(as_str(get(first_ast, "op")), "source-unit.scope-evidence");
  assert_eq!(
    as_str(get(get(first_ast, "execution_contract"), "effect_class")),
    "client-readonly-observation"
  );
  assert!(!as_bool(get(
    get(first_ast, "safety"),
    "internal_plan_visible"
  )));

  let second_ast = get(&commands[1], "command_ast");
  assert_eq!(as_str(get(second_ast, "op")), "project.test.run");
  assert_eq!(
    as_str(get(get(second_ast, "execution_contract"), "effect_class")),
    "client-host-observation"
  );
}

#[test]
fn project_patch_request_routes_only_after_scope_cache_and_holds_without_index() {
  let run = eval_file(&fixture_path()).unwrap();

  let request = get(&run, "patch-request");
  assert_eq!(
    as_str(get(request, "kind")),
    "puncheetah.coding-project-patch-request.v0"
  );
  assert!(!as_bool(get(request, "is_held")));
  assert_eq!(
    as_str(get(request, "next_gate")),
    "project-scope-frontend-selection"
  );

  let patch = get(request, "patch_request");
  assert!(!as_bool(get(patch, "single_file_shortcut_selected")));
  assert!(!as_bool(get(patch, "target_file_selected")));
  assert!(!as_bool(get(patch, "identity_shortcut_selected")));
  assert!(!as_bool(get(patch, "language_selected")));
  assert!(!as_bool(get(patch, "fixture_selected")));
  assert!(!as_bool(get(patch, "content_snapshot_embedded")));
  assert!(!as_bool(get(patch, "prompt_literal_branch")));

  let held = get(&run, "held-patch-request");
  assert!(as_bool(get(held, "is_held")));
  assert_eq!(as_str(get(held, "next_gate")), "project-index-required");
}

#[test]
fn patch_preview_scaffold_holds_until_explicit_preview_artifact() {
  let run = eval_file(&fixture_path()).unwrap();

  let preview = get(&run, "preview-scaffold");
  assert_eq!(
    as_str(get(preview, "outcome")),
    "held-awaiting-preview-artifact"
  );
  assert!(as_bool(get(preview, "is_held")));
  assert_eq!(
    as_str(get(preview, "next_gate")),
    "coding-expression-plan-reopen"
  );
  assert!(as_bool(get(
    get(preview, "receipt"),
    "no_fabricated_patch_content"
  )));

  let transform_turn = get(&run, "code-transform-turn");
  assert!(is_attrset(get(&transform_turn, "patch_preview_scaffold")));
  assert_eq!(
    as_str(get(transform_turn, "next_gate")),
    "coding-expression-plan-reopen"
  );

  let workflow_turn = get(&run, "test-workflow-turn");
  assert!(is_null(get(&workflow_turn, "patch_preview_scaffold")));
  assert_eq!(
    as_str(get(workflow_turn, "next_gate")),
    "client-command-scheduling"
  );
  assert!(is_attrset(get(&workflow_turn, "client_command_group")));

  let code_gen_turn = get(&run, "code-gen-turn");
  assert_eq!(
    as_str(get(code_gen_turn, "operation_kind")),
    "code-generation"
  );
  assert!(is_attrset(get(code_gen_turn, "patch_preview_scaffold")));
  assert_eq!(
    as_str(get(code_gen_turn, "next_gate")),
    "coding-expression-plan-reopen"
  );
  assert_eq!(
    as_str(get(get(code_gen_turn, "public_answer"), "outcome")),
    "answer-ready"
  );
}

#[test]
fn interpretation_frame_dispatches_test_and_build_workflows() {
  let run = eval_fixture_with_large_stack(&dry_run_fixture_path());

  let test_frame = get(&run, "test_frame");
  let chosen = get(test_frame, "chosen");
  assert_eq!(as_str(get(chosen, "operation_kind")), "test");
  assert_eq!(
    as_str(get(chosen, "canonical_meaning")),
    "project.workflow.test"
  );

  let build_frame = get(&run, "build_frame");
  let build_chosen = get(build_frame, "chosen");
  assert_eq!(as_str(get(build_chosen, "operation_kind")), "build");
  assert_eq!(
    as_str(get(build_chosen, "canonical_meaning")),
    "project.workflow.build"
  );

  let test_dispatch = get(&run, "test_dispatch");
  assert_eq!(as_str(get(test_dispatch, "operation_kind")), "test");
  assert_eq!(as_str(get(test_dispatch, "outcome")), "dispatched");
  assert!(as_bool(get(
    get(test_dispatch, "result"),
    "workflow_operation"
  )));

  let build_dispatch = get(&run, "build_dispatch");
  assert_eq!(as_str(get(build_dispatch, "operation_kind")), "build");
  assert!(as_bool(get(
    get(build_dispatch, "result"),
    "workflow_operation"
  )));
}

#[test]
fn patch_preview_dry_run_fixture_chains_scaffold_to_planning_gate() {
  let run = eval_fixture_with_large_stack(&dry_run_fixture_path());

  assert_eq!(
    as_str(get(&run, "proof")),
    "puncheetah-code-patch-preview-dry-run"
  );

  let preview = get(&run, "preview_scaffold");
  assert_eq!(
    as_str(get(preview, "outcome")),
    "held-awaiting-preview-artifact"
  );
  assert!(as_bool(get(
    get(preview, "receipt"),
    "no_fabricated_patch_content"
  )));

  let turn = get(&run, "code_transform_turn");
  assert!(is_attrset(get(&turn, "patch_preview_scaffold")));
  assert_eq!(
    as_str(get(turn, "next_gate")),
    "coding-expression-plan-reopen"
  );

  let planning = get(&run, "dry_run_planning");
  assert_eq!(
    as_str(get(planning, "outcome")),
    "coding-project-patch-planning-or-preview-built"
  );
  assert!(!as_bool(get(planning, "is_held")));
  assert!(as_bool(get(planning, "patch_preview_built")));
  assert!(is_attrset(get(planning, "patch_preview")));

  let review = get(&run, "preview_review");
  assert_eq!(
    as_str(get(review, "outcome")),
    "coding-project-patch-preview-reviewed"
  );
  assert_eq!(as_str(get(review, "review_status")), "reviewable");
  assert!(as_bool(get(review, "approval_required")));
  assert_eq!(
    as_str(get(review, "next_gate")),
    "coding-project-apply-approval-gate"
  );

  let approval = get(&run, "apply_approval");
  assert_eq!(
    as_str(get(approval, "outcome")),
    "coding-project-apply-approval-gate-approved"
  );
  assert!(!as_bool(get(approval, "is_held")));
  assert!(as_bool(get(approval, "approval_token_verified")));
  assert!(as_bool(get(approval, "project_patch_apply_approved")));
  assert_eq!(
    as_str(get(approval, "next_gate")),
    "coding-project-applyable-ir"
  );

  let applyable = get(&run, "applyable_ir");
  assert_eq!(
    as_str(get(applyable, "outcome")),
    "coding-project-applyable-ir-built"
  );
  assert!(!as_bool(get(applyable, "is_held")));
  assert!(as_bool(get(applyable, "applyable_project_patch_ir_built")));
  assert_eq!(
    as_str(get(applyable, "schema")),
    "puncheetah.code.applyable-project-patch-ir.v0"
  );
  let file_edits = as_list(get(applyable, "file_edits"));
  assert!(!file_edits.is_empty());
  assert_eq!(as_str(get(&file_edits[0], "path")), "src/client.ext");
}

#[test]
fn human_summary_renderer_projects_coding_dispatch_text() {
  let run = eval_file(&human_summary_fixture_path()).expect("human summary render fixture");

  assert_eq!(
    as_str(get(&run, "proof")),
    "puncheetah-code-human-summary-render"
  );

  let code_gen = as_str(get(&run, "code_generation_text"));
  assert!(code_gen.contains("behavior atom 3"));
  assert!(code_gen.contains("utterance-decomposed"));

  let test_text = as_str(get(&run, "test_workflow_text"));
  assert!(test_text.contains("테스트 실행"));
  assert!(test_text.contains("client command"));

  let build_text = as_str(get(&run, "build_workflow_text"));
  assert!(build_text.contains("빌드 실행"));
  assert!(build_text.contains("client command"));
}

#[test]
fn front_door_e2e_chains_frame_dispatch_turn_to_converged_human_summary() {
  let run = eval_fixture_with_large_stack(&front_door_e2e_fixture_path());

  assert_eq!(as_str(get(&run, "proof")), "puncheetah-code-front-door-e2e");

  let test_dispatch = get(&run, "test_dispatch");
  assert_eq!(as_str(get(test_dispatch, "operation_kind")), "test");
  assert_eq!(as_str(get(test_dispatch, "outcome")), "dispatched");

  let test_turn = get(&run, "test_turn");
  assert_eq!(as_str(get(test_turn, "outcome")), "code-turn-composed");
  assert_eq!(
    as_str(get(test_turn, "next_gate")),
    "client-command-scheduling"
  );
  assert!(is_attrset(get(test_turn, "client_command_group")));
  assert_eq!(
    as_str(get(get(test_turn, "public_answer"), "outcome")),
    "workflow-scheduled"
  );

  let test_front_door = get(&run, "test_front_door");
  assert_eq!(as_str(get(test_front_door, "requestKind")), "human-summary");
  assert_eq!(
    as_str(get(test_front_door, "route")),
    "stdlib/lib/nl/human-summary-renderer.px"
  );
  let test_result = get(test_front_door, "result");
  assert_eq!(
    as_str(get(test_result, "outcome")),
    "human-summary-rendered"
  );
  let test_text = as_str(get(test_result, "text"));
  assert!(test_text.contains("테스트 실행"));
  assert!(test_text.contains("client command"));
}

#[test]
fn converged_engine_human_summary_renders_coding_dispatch_text() {
  let run = eval_fixture_with_large_stack(&converged_human_summary_fixture_path());

  assert_eq!(
    as_str(get(&run, "proof")),
    "puncheetah-code-converged-human-summary"
  );

  let test_summary = get(&run, "test_workflow_summary");
  assert_eq!(as_str(get(test_summary, "requestKind")), "human-summary");
  assert_eq!(
    as_str(get(test_summary, "route")),
    "stdlib/lib/nl/human-summary-renderer.px"
  );
  let test_result = get(test_summary, "result");
  assert_eq!(
    as_str(get(test_result, "outcome")),
    "human-summary-rendered"
  );
  let test_text = as_str(get(test_result, "text"));
  assert!(test_text.contains("테스트 실행"));
  assert!(test_text.contains("client command"));

  let code_summary = get(&run, "code_generation_summary");
  let code_result = get(code_summary, "result");
  let code_text = as_str(get(code_result, "text"));
  assert!(code_text.contains("behavior atom 2"));
  assert!(code_text.contains("utterance-decomposed"));
}
