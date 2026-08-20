//! PNIX 파싱 에러 테스트: PNIX 표현식 파싱 에러 메시지 검증
//!
//! PNIX 표현식 파싱 시 발생하는 에러 메시지가 위치 정보와 예상/발견 토큰을
//! 포함하는지 확인합니다.

use pnix_core::lang::pnix::parse_expr;
use serde::Deserialize;

const CASES: &str = include_str!("../../../fixtures/pnix_parse_errors/cases.json");

#[derive(Debug, Deserialize)]
struct Case {
  name: String,
  source: String,
  expected: String,
}

fn parse_error_header(message: &str) -> (&str, usize, usize) {
  let (code, rest) = message
    .strip_prefix('[')
    .and_then(|text| text.split_once("] Parse error at line "))
    .expect("parse error header format should be '[E####] Parse error at line L:C: ...'");
  let (line_col, _detail) = rest
    .split_once(": ")
    .expect("parse error header should include ': ' after line/column");
  let (line, column) = line_col
    .split_once(':')
    .expect("parse error header should include line:column");
  let line = line.parse::<usize>().expect("line should be numeric");
  let column = column.parse::<usize>().expect("column should be numeric");
  (code, line, column)
}

#[test]
fn pnix_parse_errors_include_location_and_expected_found() {
  let cases: Vec<Case> = serde_json::from_str(CASES).expect("valid cases fixture");
  for case in cases {
    let err = parse_expr(&case.source).unwrap_err();
    let message = err.to_string();
    let (code, line, column) = parse_error_header(&message);

    assert!(
      code.len() == 5
        && code.as_bytes()[0] == b'E'
        && code.as_bytes()[1..].iter().all(|ch| ch.is_ascii_digit()),
      "case {}: invalid error code format in message: {}",
      case.name,
      message
    );
    assert!(
      line > 0,
      "case {}: line must be >= 1: {}",
      case.name,
      message
    );
    assert!(
      column > 0,
      "case {}: column must be >= 1: {}",
      case.name,
      message
    );
    assert!(
      message.contains("Parse error at line"),
      "case {}: {}",
      case.name,
      message
    );
    assert!(
      message.contains(&case.expected),
      "case {}: {}",
      case.name,
      message
    );
    assert!(
      !message.ends_with(": "),
      "case {}: parse error summary should not be empty: {}",
      case.name,
      message
    );
    assert!(
      !message.contains('\n') && !message.contains('\r'),
      "case {}: parse error should be single-line: {}",
      case.name,
      message
    );
    assert!(
      !message.contains('\u{1b}'),
      "case {}: parse error should not include ANSI escapes: {}",
      case.name,
      message
    );
    assert!(
      message.chars().all(|ch| !ch.is_control()),
      "case {}: parse error should not contain control characters: {}",
      case.name,
      message
    );
  }
}

#[test]
fn pnix_parse_error_empty_input_uses_one_based_position() {
  let err = parse_expr("").expect_err("empty input should fail");
  let message = err.to_string();
  let (_code, line, column) = parse_error_header(&message);
  assert_eq!(line, 1, "empty input line should be 1: {}", message);
  assert_eq!(column, 1, "empty input column should be 1: {}", message);
}
