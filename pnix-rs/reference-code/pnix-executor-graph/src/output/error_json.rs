//! Y15c: 구조화된 에러 출력 (JSON 포맷)
//!
//! AI 친화적인 구조화된 에러 포맷 제공

use pnix_core::diagnostics::{CompileError, ParseError, TokenizeError};
use serde::{Deserialize, Serialize};

/// JSON 에러 포맷
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // 향후 사용 예정
pub struct JsonError {
  pub code: String,
  pub message: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub location: Option<ErrorLocation>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub suggestions: Vec<String>,
}

/// 에러 위치 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // 향후 사용 예정
pub struct ErrorLocation {
  pub file: Option<String>,
  pub line: usize,
  pub column: usize,
}

/// 에러를 JSON 포맷으로 변환
#[allow(dead_code)] // 향후 사용 예정
pub fn error_to_json(error: &(dyn std::error::Error + 'static)) -> JsonError {
  // ParseError 처리
  if let Some(parse_err) = error.downcast_ref::<ParseError>() {
    return parse_error_to_json(parse_err);
  }

  // CompileError 처리
  if let Some(compile_err) = error.downcast_ref::<CompileError>() {
    return compile_error_to_json(compile_err);
  }

  // TokenizeError 처리
  if let Some(tokenize_err) = error.downcast_ref::<TokenizeError>() {
    return tokenize_error_to_json(tokenize_err);
  }

  // 기본 에러 (알 수 없는 타입)
  JsonError {
    code: "E0000".to_string(),
    message: error.to_string(),
    location: None,
    suggestions: Vec::new(),
  }
}

#[allow(dead_code)] // 향후 사용 예정
fn parse_error_to_json(err: &ParseError) -> JsonError {
  match err {
    ParseError::Parse {
      code,
      message,
      line,
      column,
    } => JsonError {
      code: code.to_string(),
      message: message.clone(),
      location: Some(ErrorLocation {
        file: None,
        line: *line,
        column: *column,
      }),
      suggestions: Vec::new(),
    },
    ParseError::UnexpectedToken {
      code,
      expected,
      found,
      line,
      column,
    } => JsonError {
      code: code.to_string(),
      message: format!("Unexpected token: expected {}, found {}", expected, found),
      location: Some(ErrorLocation {
        file: None,
        line: *line,
        column: *column,
      }),
      suggestions: vec![format!("Replace '{}' with '{}'", found, expected)],
    },
    ParseError::ExpectedToken {
      code,
      expected,
      found,
      line,
      column,
    } => JsonError {
      code: code.to_string(),
      message: format!("Expected token: expected {}, found {}", expected, found),
      location: Some(ErrorLocation {
        file: None,
        line: *line,
        column: *column,
      }),
      suggestions: vec![format!("Insert '{}' before '{}'", expected, found)],
    },
    ParseError::UnexpectedEof {
      code,
      expected,
      line,
      column,
    } => JsonError {
      code: code.to_string(),
      message: format!("Unexpected end of file: expected {}", expected),
      location: Some(ErrorLocation {
        file: None,
        line: *line,
        column: *column,
      }),
      suggestions: vec![format!("Add '{}' before end of file", expected)],
    },
    ParseError::NestingTooDeep { code, line, column } => JsonError {
      code: code.to_string(),
      message: "Nesting too deep".to_string(),
      location: Some(ErrorLocation {
        file: None,
        line: *line,
        column: *column,
      }),
      suggestions: vec!["Simplify nested expressions".to_string()],
    },
  }
}

#[allow(dead_code)] // 향후 사용 예정
fn compile_error_to_json(err: &CompileError) -> JsonError {
  match err {
    CompileError::Compile { code, message, .. } => JsonError {
      code: code.to_string(),
      message: message.clone(),
      location: None,
      suggestions: Vec::new(),
    },
    CompileError::CompileAt {
      code,
      message,
      location,
      ..
    } => JsonError {
      code: code.to_string(),
      message: message.clone(),
      location: Some(ErrorLocation {
        file: location.file.clone(),
        line: location.line,
        column: location.column,
      }),
      suggestions: Vec::new(),
    },
    CompileError::TypeError {
      code,
      expected,
      actual,
      ..
    } => JsonError {
      code: code.to_string(),
      message: format!("Type error: expected {}, found {}", expected, actual),
      location: None,
      suggestions: vec![format!("Change type from {} to {}", actual, expected)],
    },
    CompileError::TypeErrorAt {
      code,
      expected,
      actual,
      location,
      ..
    } => JsonError {
      code: code.to_string(),
      message: format!("Type error: expected {}, found {}", expected, actual),
      location: Some(ErrorLocation {
        file: location.file.clone(),
        line: location.line,
        column: location.column,
      }),
      suggestions: vec![format!("Change type from {} to {}", actual, expected)],
    },
    CompileError::UndefinedVariable {
      code, name, hint, ..
    } => JsonError {
      code: code.to_string(),
      message: format!("Undefined variable: {}{}", name, hint),
      location: None,
      suggestions: vec![format!("Define variable '{}'", name)],
    },
    CompileError::UndefinedVariableAt {
      code,
      name,
      location,
      hint,
      ..
    } => JsonError {
      code: code.to_string(),
      message: format!("Undefined variable: {}{}", name, hint),
      location: Some(ErrorLocation {
        file: location.file.clone(),
        line: location.line,
        column: location.column,
      }),
      suggestions: vec![format!("Define variable '{}'", name)],
    },
    // 모든 CompileError variant가 위에서 처리되었지만, 향후 확장을 위해 catch-all 유지
    #[allow(unreachable_patterns)]
    _ => JsonError {
      code: "E0000".to_string(),
      message: err.to_string(),
      location: None,
      suggestions: Vec::new(),
    },
  }
}

#[allow(dead_code)] // 향후 사용 예정
fn tokenize_error_to_json(err: &TokenizeError) -> JsonError {
  match err {
    TokenizeError::InvalidChar { code, ch, position } => {
      // position을 line/column으로 변환 (간단한 추정)
      let line = 1; // 정확한 line/column 계산은 복잡하므로 기본값 사용
      let column = *position;
      JsonError {
        code: code.to_string(),
        message: format!("Invalid character: '{}'", ch),
        location: Some(ErrorLocation {
          file: None,
          line,
          column,
        }),
        suggestions: vec![format!("Remove or escape character '{}'", ch)],
      }
    }
    TokenizeError::Tokenize {
      code,
      message,
      position,
    } => {
      let line = 1;
      let column = *position;
      JsonError {
        code: code.to_string(),
        message: message.clone(),
        location: Some(ErrorLocation {
          file: None,
          line,
          column,
        }),
        suggestions: Vec::new(),
      }
    }
    TokenizeError::UnclosedString { code } => JsonError {
      code: code.to_string(),
      message: "Unclosed string".to_string(),
      location: None,
      suggestions: vec!["Close string with matching quote".to_string()],
    },
  }
}

/// 에러를 JSON 문자열로 출력
#[allow(dead_code)] // 향후 사용 예정
pub fn print_error_json(error: &(dyn std::error::Error + 'static)) {
  let json_err = error_to_json(error);
  let json_str = serde_json::to_string_pretty(&json_err).unwrap_or_else(|_| {
    serde_json::json!({
      "code": "E0000",
      "message": error.to_string(),
      "location": null,
      "suggestions": []
    })
    .to_string()
  });
  eprintln!("{}", json_str);
}

#[cfg(test)]
mod tests {
  use super::*;
  use pnix_core::diagnostics::error_codes;

  #[test]
  fn test_parse_error_to_json() {
    let err = ParseError::Parse {
      code: error_codes::PARSE_ERROR,
      message: "test error".to_string(),
      line: 10,
      column: 5,
    };
    let json_err = parse_error_to_json(&err);
    assert_eq!(json_err.code, error_codes::PARSE_ERROR.to_string());
    assert_eq!(json_err.message, "test error");
    assert_eq!(json_err.location.as_ref().unwrap().line, 10);
    assert_eq!(json_err.location.as_ref().unwrap().column, 5);
  }

  #[test]
  fn test_compile_error_to_json() {
    let err = CompileError::TypeError {
      code: error_codes::TYPE_MISMATCH,
      expected: "Int".to_string(),
      actual: "Bool".to_string(),
      source: None,
    };
    let json_err = compile_error_to_json(&err);
    assert_eq!(json_err.code, error_codes::TYPE_MISMATCH.to_string());
    assert!(json_err.message.contains("Int"));
    assert!(json_err.message.contains("Bool"));
    assert!(!json_err.suggestions.is_empty());
  }
}
