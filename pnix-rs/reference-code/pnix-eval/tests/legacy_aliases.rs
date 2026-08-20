//! batch 261 (2026-04-18): M1-8 legacy API aliases 테스트.
//!
//! pnix-runtime-legacy 호환 이름 (`eval_pnix_expr` / `eval_pnix_file` /
//! `eval_and_format` + `OutputFormat`) 이 기대대로 동작하는지 검증.
//! 이 테스트가 green 이어야 M1-11 (pnixc --eval swap) 이 mechanical 로
//! 진행 가능하다.

use pnix_eval::{eval_and_format, eval_pnix_expr, eval_pnix_file, OutputFormat, Value};

#[test]
fn eval_pnix_expr_behaves_like_eval_expr() {
  let v = eval_pnix_expr("1 + 2").unwrap();
  assert!(matches!(v, Value::Int(3)));
}

#[test]
fn eval_pnix_file_reads_source_and_evaluates() {
  let tmp = std::env::temp_dir().join(format!("pnix-eval-legacy-alias-{}.px", std::process::id()));
  std::fs::write(&tmp, "40 + 2").unwrap();
  let v = eval_pnix_file(&tmp).unwrap();
  let _ = std::fs::remove_file(&tmp);
  assert!(matches!(v, Value::Int(42)));
}

#[test]
fn eval_and_format_json_matches_eval_to_json() {
  let source = "{ a = 1 + 2; b = \"hello\"; }";
  let got = eval_and_format(source, false, OutputFormat::Json).unwrap();
  // pnix-eval 은 attrset key 정렬 순서에 의존하지 않도록 포함 검사로.
  assert!(got.contains("\"a\":3"));
  assert!(got.contains("\"b\":\"hello\""));
}

#[test]
fn eval_and_format_unsupported_returns_error() {
  let result = eval_and_format("1 + 2", false, OutputFormat::Unsupported);
  assert!(result.is_err(), "OutputFormat::Unsupported 은 에러여야 함");
  let err = format!("{}", result.unwrap_err());
  assert!(
    err.contains("Unsupported"),
    "에러 메시지에 Unsupported 포함"
  );
}

#[test]
fn eval_and_format_file_mode_reads_path() {
  let tmp = std::env::temp_dir().join(format!(
    "pnix-eval-legacy-alias-file-{}.px",
    std::process::id()
  ));
  std::fs::write(&tmp, "{ x = 7; }").unwrap();
  let path_str = tmp.to_str().unwrap();
  let got = eval_and_format(path_str, true, OutputFormat::Json).unwrap();
  let _ = std::fs::remove_file(&tmp);
  assert!(got.contains("\"x\":7"));
}
