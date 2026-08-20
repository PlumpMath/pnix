//! 진단 메시지 품질 테스트 (U37)
//!
//! 위치 정보와 힌트가 포함된 진단 메시지가 올바르게 생성되는지 검증합니다.

use pnix_core::diagnostics::{error_codes, line_col_at, Diagnostic, Diagnostics, Span};
use pnix_core::lang::pnix::parse_pnix_module;

#[test]
fn test_diagnostic_format_with_source() {
  let source = "let x = undefined_var;";
  let span = Span::new(9, 22); // "undefined_var" 위치

  let diag = Diagnostic {
    message: "unknown variable".to_string(),
    span: Some(span),
    hint: Some("Did you mean 'x'?".to_string()),
  };

  let formatted = diag.format_with_source(source, "test.px");

  // 위치 정보 포함 확인
  assert!(formatted.contains("test.px:1:10"));
  assert!(formatted.contains("unknown variable"));

  // 소스 코드 하이라이트 포함 확인
  assert!(formatted.contains("let x = undefined_var;"));
  assert!(formatted.contains("^^^")); // 하이라이트 문자

  // 힌트 포함 확인
  assert!(formatted.contains("Did you mean 'x'?"));
}

#[test]
fn test_diagnostic_without_span() {
  let diag = Diagnostic {
    message: "general error".to_string(),
    span: None,
    hint: None,
  };

  let formatted = diag.format_with_source("some code", "test.px");

  assert!(formatted.contains("test.px: general error"));
  assert!(!formatted.contains("^^^")); // 하이라이트 없음
}

#[test]
fn test_parse_error_with_span() {
  // 잘못된 구문: 닫히지 않은 괄호
  let source = "let x = (1 + 2;";
  let result = parse_pnix_module(source, "test.px", Some("test.px"));

  assert!(result.is_err());
  let err = result.unwrap_err();

  // Parse 에러에 span이 포함되어 있는지 확인
  match err {
    pnix_core::lang::pnix::error::PnixError::Parse { code, span, .. } => {
      assert_eq!(code, error_codes::PARSE_ERROR);
      assert!(span.start < source.len());
      assert!(span.end <= source.len());
    }
    _ => panic!("Expected Parse error"),
  }
}

#[test]
fn test_parse_error_position() {
  // 잘못된 구문: 닫히지 않은 괄호
  let source = "let x = (1 + 2;";
  let result = parse_pnix_module(source, "test.px", Some("test.px"));

  // 파싱은 실패해야 함
  assert!(result.is_err());
  let err = result.unwrap_err();

  // Parse 에러에 위치 정보가 포함되어 있는지 확인
  match err {
    pnix_core::lang::pnix::error::PnixError::Parse {
      code,
      line,
      column,
      span,
      ..
    } => {
      assert_eq!(code, error_codes::PARSE_ERROR);
      assert!(line > 0);
      assert!(column > 0);
      assert!(span.start < source.len());
      assert!(span.end <= source.len());
      let (expected_line, expected_column) = line_col_at(source, span.start);
      assert_eq!(line, expected_line);
      assert_eq!(column, expected_column);
    }
    _ => panic!("Expected Parse error with position"),
  }
}

#[test]
fn test_diagnostics_sorted() {
  let mut diags = Diagnostics::default();

  // 서로 다른 위치의 진단 메시지 추가
  diags.push("error at line 10", Some(Span::new(100, 110)));
  diags.push("error at line 5", Some(Span::new(50, 55)));
  diags.push("error at line 20", Some(Span::new(200, 210)));

  let sorted = diags.sorted();

  // 위치 순서로 정렬되었는지 확인
  assert_eq!(sorted[0].message, "error at line 5");
  assert_eq!(sorted[1].message, "error at line 10");
  assert_eq!(sorted[2].message, "error at line 20");
}

#[test]
fn test_diagnostic_hint() {
  let mut diags = Diagnostics::default();

  diags.push_with_hint(
    "unknown variable 'foo'",
    Some(Span::new(10, 13)),
    "Did you mean 'bar'?",
  );

  assert_eq!(diags.items.len(), 1);
  assert_eq!(diags.items[0].hint, Some("Did you mean 'bar'?".to_string()));

  let formatted = diags.items[0].format_with_source("let x = foo;", "test.px");
  assert!(formatted.contains("Did you mean 'bar'?"));
}

#[test]
fn test_diagnostic_multiline_highlight() {
  let source = "let x = 1;\nlet y = undefined_var;\nlet z = 2;";
  let span = Span::new(20, 33); // "undefined_var" 위치 (두 번째 라인)

  let diag = Diagnostic {
    message: "unknown variable".to_string(),
    span: Some(span),
    hint: None,
  };

  let formatted = diag.format_with_source(source, "test.px");

  // 두 번째 라인만 하이라이트되어야 함
  assert!(formatted.contains("let y = undefined_var;"));
  assert!(!formatted.contains("let x = 1;"));
  assert!(!formatted.contains("let z = 2;"));
}

#[test]
fn test_diagnostic_unicode_width() {
  // 한글 문자 테스트 (유니코드 너비 고려)
  let source = "let 한글 = 1;";
  // "한글"의 실제 바이트 위치 계산 (한=3바이트, 글=3바이트)
  // "let " = 4바이트, "한" = 4..7바이트, "글" = 7..10바이트
  let span = Span::new(4, 10); // "한글" 위치 (바이트 단위)

  let diag = Diagnostic {
    message: "invalid identifier".to_string(),
    span: Some(span),
    hint: None,
  };

  let formatted = diag.format_with_source(source, "test.px");

  // 한글 문자도 올바르게 하이라이트되어야 함
  assert!(formatted.contains("한글"));
  assert!(formatted.contains("^^")); // 한글은 2칸 너비
}
