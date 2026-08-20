//! 컴파일 에러 테스트: 컴파일 에러 메시지 및 위치 정보 검증
//!
//! 다양한 컴파일 에러 케이스에 대한 에러 메시지와 위치 정보가 올바르게 생성되는지 검증합니다.

use pnix_core::diagnostics::{error_codes, CompileError, SourceLocation};
use serde::Deserialize;

const CASES: &str = include_str!("../../../fixtures/compile_errors/cases.json");

#[derive(Debug, Deserialize)]
struct Location {
  file: Option<String>,
  line: usize,
  column: usize,
}

#[derive(Debug, Deserialize)]
struct Case {
  name: String,
  kind: String,
  expected: String,
  expected_type: Option<String>,
  actual_type: Option<String>,
  variable: Option<String>,
  candidates: Option<Vec<String>>,
  message: Option<String>,
  location: Option<Location>,
}

fn to_location(loc: &Location) -> SourceLocation {
  SourceLocation {
    file: loc.file.clone(),
    line: loc.line,
    column: loc.column,
  }
}

#[test]
fn compile_error_messages_include_expected_found_and_suggestions() {
  let cases: Vec<Case> = serde_json::from_str(CASES).unwrap_or_else(|err| {
    panic!(
      "invalid compile error fixtures in fixtures/compile_errors/cases.json: {}",
      err
    )
  });
  for case in cases {
    let err = match case.kind.as_str() {
      "type" => CompileError::TypeError {
        code: error_codes::TYPE_MISMATCH,
        expected: case.expected_type.expect("expected_type"),
        actual: case.actual_type.expect("actual_type"),
        source: None,
      },
      "type-at" => CompileError::TypeErrorAt {
        code: error_codes::TYPE_MISMATCH,
        expected: case.expected_type.expect("expected_type"),
        actual: case.actual_type.expect("actual_type"),
        location: to_location(case.location.as_ref().expect("location")),
        source: None,
      },
      "undef" => CompileError::undefined_variable(
        case.variable.expect("variable"),
        case.candidates.as_ref().expect("candidates"),
      ),
      "undef-at" => CompileError::undefined_variable_at(
        case.variable.expect("variable"),
        case.candidates.as_ref().expect("candidates"),
        to_location(case.location.as_ref().expect("location")),
      ),
      "compile" => CompileError::Compile {
        code: error_codes::COMPILE_ERROR,
        message: case.message.expect("message"),
        source: None,
      },
      "compile-at" => CompileError::CompileAt {
        code: error_codes::COMPILE_ERROR,
        message: case.message.expect("message"),
        location: to_location(case.location.as_ref().expect("location")),
        source: None,
      },
      other => panic!("unknown case kind: {}", other),
    };

    let message = err.to_string();
    assert!(
      message.contains(&case.expected),
      "case {}: {}",
      case.name,
      message
    );
  }
}
