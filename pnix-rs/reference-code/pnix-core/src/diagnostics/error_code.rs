//! 에러 코드: 컴파일 에러 코드 정의 및 조회

/// 에러 코드: 정적 문자열로 표현된 에러 코드
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorCode(pub &'static str);

impl std::fmt::Display for ErrorCode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl ErrorCode {
  /// 에러 코드를 문자열로 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn as_str(&self) -> &'static str {
    self.0
  }

  /// 문자열로부터 에러 코드 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 조회만, 값 계산 없음
  pub fn lookup(code: &str) -> ErrorCode {
    match code {
      error_codes::PARSE_ERROR_STR => error_codes::PARSE_ERROR,
      error_codes::UNEXPECTED_TOKEN_STR => error_codes::UNEXPECTED_TOKEN,
      error_codes::EXPECTED_TOKEN_STR => error_codes::EXPECTED_TOKEN,
      error_codes::UNEXPECTED_EOF_STR => error_codes::UNEXPECTED_EOF,
      error_codes::NESTING_TOO_DEEP_STR => error_codes::NESTING_TOO_DEEP,
      error_codes::INVALID_CHAR_STR => error_codes::INVALID_CHAR,
      error_codes::TOKENIZE_ERROR_STR => error_codes::TOKENIZE_ERROR,
      error_codes::UNCLOSED_STRING_STR => error_codes::UNCLOSED_STRING,
      error_codes::UNDEFINED_VARIABLE_STR => error_codes::UNDEFINED_VARIABLE,
      error_codes::TYPE_MISMATCH_STR => error_codes::TYPE_MISMATCH,
      error_codes::COMPILE_ERROR_STR => error_codes::COMPILE_ERROR,
      _ => error_codes::UNKNOWN,
    }
  }
}

impl std::str::FromStr for ErrorCode {
  type Err = ();

  fn from_str(code: &str) -> Result<Self, Self::Err> {
    Ok(ErrorCode::lookup(code))
  }
}

pub mod error_codes {
  use super::ErrorCode;

  pub const UNKNOWN: ErrorCode = ErrorCode("E0000");
  pub const UNKNOWN_STR: &str = "E0000";

  // Parse errors (E00xx)
  pub const PARSE_ERROR: ErrorCode = ErrorCode("E0001");
  pub const PARSE_ERROR_STR: &str = "E0001";
  pub const UNEXPECTED_TOKEN: ErrorCode = ErrorCode("E0002");
  pub const UNEXPECTED_TOKEN_STR: &str = "E0002";
  pub const EXPECTED_TOKEN: ErrorCode = ErrorCode("E0003");
  pub const EXPECTED_TOKEN_STR: &str = "E0003";
  pub const UNEXPECTED_EOF: ErrorCode = ErrorCode("E0004");
  pub const UNEXPECTED_EOF_STR: &str = "E0004";
  pub const NESTING_TOO_DEEP: ErrorCode = ErrorCode("E0005");
  pub const NESTING_TOO_DEEP_STR: &str = "E0005";
  pub const INVALID_CHAR: ErrorCode = ErrorCode("E0006");
  pub const INVALID_CHAR_STR: &str = "E0006";
  pub const TOKENIZE_ERROR: ErrorCode = ErrorCode("E0007");
  pub const TOKENIZE_ERROR_STR: &str = "E0007";
  pub const UNCLOSED_STRING: ErrorCode = ErrorCode("E0008");
  pub const UNCLOSED_STRING_STR: &str = "E0008";

  // Name resolution errors (E01xx)
  pub const UNDEFINED_VARIABLE: ErrorCode = ErrorCode("E0101");
  pub const UNDEFINED_VARIABLE_STR: &str = "E0101";

  // Type errors (E02xx)
  pub const TYPE_MISMATCH: ErrorCode = ErrorCode("E0201");
  pub const TYPE_MISMATCH_STR: &str = "E0201";

  // Compile errors (E03xx)
  pub const COMPILE_ERROR: ErrorCode = ErrorCode("E0301");
  pub const COMPILE_ERROR_STR: &str = "E0301";
}

#[cfg(test)]
mod tests {
  use super::{error_codes, ErrorCode};

  #[test]
  fn error_code_lookup_falls_back() {
    let code = ErrorCode::lookup("E9999");
    assert_eq!(code, error_codes::UNKNOWN);
  }

  #[test]
  fn error_code_lookup_known() {
    let code = ErrorCode::lookup(error_codes::PARSE_ERROR_STR);
    assert_eq!(code, error_codes::PARSE_ERROR);
  }
}
