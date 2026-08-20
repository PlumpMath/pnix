//! Nix 매크로 확장 에러 타입
//!
//! pnix-old의 pnix_nix_macro_error/src/lib.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 에러 타입/메시지 생성만, 실행 없음

use std::fmt;

/// 소스 코드 위치: 에러가 발생한 소스 코드의 위치 정보
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceLocation {
  /// 파일 경로 (선택적, 파일 경로)
  pub file: Option<String>,
  /// 라인 번호 (라인 번호, 1부터 시작)
  pub line: usize,
  /// 컬럼 번호 (컬럼 번호, 1부터 시작)
  pub column: usize,
}

impl fmt::Display for SourceLocation {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if let Some(file) = &self.file {
      write!(f, "{}:{}:{}", file, self.line, self.column)
    } else {
      write!(f, "line {}:{}", self.line, self.column)
    }
  }
}

/// 매크로 확장 에러: Nix 매크로 확장 중 발생하는 에러 타입 (상세 컨텍스트 포함)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MacroExpansionError {
  /// Nix 평가 실패: Nix 표현식 평가 중 발생한 에러
  NixEvaluationFailed {
    /// 표현식 (평가하려던 Nix 표현식)
    expression: String,
    /// Nix 에러 메시지 (Nix에서 반환한 에러 메시지)
    nix_error: String,
    /// 위치 (에러 발생 위치)
    location: SourceLocation,
    /// 제안 사항 (수정 제안, 선택적)
    suggestion: Option<String>,
  },
  /// 확장 깊이 초과: 매크로 확장 깊이가 최대값을 초과함 (무한 재귀 가능성)
  ExpansionDepthExceeded {
    /// 현재 깊이 (현재 확장 깊이)
    current_depth: usize,
    /// 최대 깊이 (허용된 최대 확장 깊이)
    max_depth: usize,
    /// 매크로 이름 (확장 중인 매크로 이름)
    macro_name: String,
    /// 위치 (에러 발생 위치)
    location: SourceLocation,
  },
  /// nix-eval 인자 오류
  InvalidNixEvalArgs {
    /// 예상 형식
    expected: String,
    /// 실제 형식
    actual: String,
    /// 위치
    location: SourceLocation,
  },
  /// 변수를 찾을 수 없음
  VariableNotFound {
    /// 변수 이름
    variable_name: String,
    /// 사용 가능한 변수 목록
    available_vars: Vec<String>,
    /// 위치
    location: SourceLocation,
  },
  /// 타입 변환 오류
  TypeConversionError {
    /// Nix 타입
    nix_type: String,
    /// 목표 타입
    target_type: String,
    /// 이유
    reason: String,
    /// 위치
    location: SourceLocation,
  },
  /// Nix 평가 비활성화됨
  NixEvalDisabled {
    /// 위치
    location: SourceLocation,
  },
  /// 매크로를 찾을 수 없음
  MacroNotFound {
    /// 매크로 이름
    macro_name: String,
    /// 위치
    location: SourceLocation,
  },
}

impl fmt::Display for MacroExpansionError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::NixEvaluationFailed {
        expression,
        nix_error,
        location,
        suggestion,
      } => {
        writeln!(f, "Nix evaluation failed at {}", location)?;
        writeln!(f, "  Expression: {}", expression)?;
        writeln!(f, "  Error: {}", nix_error)?;
        if let Some(hint) = suggestion {
          writeln!(f, "  Hint: {}", hint)?;
        }
        Ok(())
      }
      Self::ExpansionDepthExceeded {
        current_depth,
        max_depth,
        macro_name,
        location,
      } => {
        writeln!(f, "Macro expansion depth exceeded at {}", location)?;
        writeln!(f, "  Macro: {}", macro_name)?;
        writeln!(f, "  Depth: {} (max: {})", current_depth, max_depth)?;
        writeln!(
          f,
          "  This likely indicates infinite recursion in your macro."
        )?;
        writeln!(
          f,
          "  Check if the macro is calling itself without a base case."
        )?;
        Ok(())
      }
      Self::InvalidNixEvalArgs {
        expected,
        actual,
        location,
      } => {
        writeln!(f, "Invalid arguments to nix-eval at {}", location)?;
        writeln!(f, "  Expected: {}", expected)?;
        writeln!(f, "  Actual: {}", actual)?;
        writeln!(
          f,
          "  Usage: (nix-eval \"expression\") or (nix-eval variable)"
        )?;
        Ok(())
      }
      Self::VariableNotFound {
        variable_name,
        available_vars,
        location,
      } => {
        writeln!(f, "Variable '{}' not found at {}", variable_name, location)?;
        if !available_vars.is_empty() {
          writeln!(f, "  Available variables:")?;
          for var in available_vars.iter().take(10) {
            writeln!(f, "    - {}", var)?;
          }
          if available_vars.len() > 10 {
            writeln!(f, "    ... and {} more", available_vars.len() - 10)?;
          }
        } else {
          writeln!(f, "  No variables are currently bound in this context.")?;
          writeln!(
            f,
            "  Make sure you're using nix-eval inside a Nix let expression."
          )?;
        }
        Ok(())
      }
      Self::TypeConversionError {
        nix_type,
        target_type,
        reason,
        location,
      } => {
        writeln!(f, "Type conversion failed at {}", location)?;
        writeln!(f, "  Nix type: {}", nix_type)?;
        writeln!(f, "  Target type: {}", target_type)?;
        writeln!(f, "  Reason: {}", reason)?;
        Ok(())
      }
      Self::NixEvalDisabled { location } => {
        writeln!(f, "Nix evaluation is disabled at {}", location)?;
        writeln!(
          f,
          "  nix-eval can only be used inside macro definitions (defmacro)."
        )?;
        Ok(())
      }
      Self::MacroNotFound {
        macro_name,
        location,
      } => {
        writeln!(f, "Macro '{}' not found at {}", macro_name, location)?;
        writeln!(f, "  Make sure the macro is defined before it's used.")?;
        writeln!(f, "  Use (defmacro {} [...] ...) to define it.", macro_name)?;
        Ok(())
      }
    }
  }
}

impl std::error::Error for MacroExpansionError {}

impl MacroExpansionError {
  /// Nix 평가 실패 에러 생성 (제안 포함)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn nix_eval_failed(expression: &str, nix_error: &str, location: SourceLocation) -> Self {
    let suggestion = Self::suggest_fix_for_nix_error(expression, nix_error);
    Self::NixEvaluationFailed {
      expression: expression.to_string(),
      nix_error: nix_error.to_string(),
      location,
      suggestion,
    }
  }

  /// 에러 메시지 기반 수정 제안 생성
  fn suggest_fix_for_nix_error(expression: &str, error: &str) -> Option<String> {
    if error.contains("undefined variable") {
      Some("Check variable name spelling and ensure it's defined in the Nix scope.".to_string())
    } else if error.contains("syntax error") {
      Some("Check Nix syntax - missing semicolons, braces, or parentheses?".to_string())
    } else if error.contains("attribute") && error.contains("missing") {
      Some(
        "The attrset doesn't have this attribute. Use builtins.hasAttr to check first.".to_string(),
      )
    } else if error.contains("type") {
      Some(
        "Type mismatch - check that you're passing the right types to Nix functions.".to_string(),
      )
    } else if expression.contains("builtins.") && error.contains("undefined") {
      Some("This Nix builtin might not be available in this version of pnix.".to_string())
    } else {
      None
    }
  }

  /// 깊이 초과 에러 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn depth_exceeded(
    current: usize,
    max: usize,
    macro_name: &str,
    location: SourceLocation,
  ) -> Self {
    Self::ExpansionDepthExceeded {
      current_depth: current,
      max_depth: max,
      macro_name: macro_name.to_string(),
      location,
    }
  }

  /// 잘못된 인자 에러 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn invalid_args(expected: &str, actual: &str, location: SourceLocation) -> Self {
    Self::InvalidNixEvalArgs {
      expected: expected.to_string(),
      actual: actual.to_string(),
      location,
    }
  }

  /// 변수 없음 에러 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn variable_not_found(
    var_name: &str,
    available: Vec<String>,
    location: SourceLocation,
  ) -> Self {
    Self::VariableNotFound {
      variable_name: var_name.to_string(),
      available_vars: available,
      location,
    }
  }

  /// 타입 변환 에러 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn type_conversion(
    nix_type: &str,
    target: &str,
    reason: &str,
    location: SourceLocation,
  ) -> Self {
    Self::TypeConversionError {
      nix_type: nix_type.to_string(),
      target_type: target.to_string(),
      reason: reason.to_string(),
      location,
    }
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_source_location_display() {
    let loc = SourceLocation {
      file: Some("test.nix".to_string()),
      line: 42,
      column: 10,
    };
    assert_eq!(loc.to_string(), "test.nix:42:10");

    let loc_no_file = SourceLocation {
      file: None,
      line: 10,
      column: 5,
    };
    assert_eq!(loc_no_file.to_string(), "line 10:5");
  }

  #[test]
  fn test_nix_eval_error_display() {
    let err = MacroExpansionError::nix_eval_failed(
      "servers",
      "undefined variable 'servers'",
      SourceLocation {
        file: Some("config.nix".to_string()),
        line: 15,
        column: 8,
      },
    );

    let msg = err.to_string();
    assert!(msg.contains("Nix evaluation failed"));
    assert!(msg.contains("servers"));
    assert!(msg.contains("undefined variable"));
    assert!(msg.contains("config.nix:15:8"));
  }

  #[test]
  fn test_depth_exceeded_error() {
    let err =
      MacroExpansionError::depth_exceeded(101, 100, "recursive-macro", SourceLocation::default());

    let msg = err.to_string();
    assert!(msg.contains("depth exceeded"));
    assert!(msg.contains("101"));
    assert!(msg.contains("100"));
    assert!(msg.contains("infinite recursion"));
  }

  #[test]
  fn test_suggestion_for_undefined_variable() {
    let suggestion =
      MacroExpansionError::suggest_fix_for_nix_error("foo", "undefined variable 'foo'");
    assert!(suggestion.is_some());
    assert!(suggestion.unwrap().contains("Check variable name"));
  }

  #[test]
  fn test_suggestion_for_syntax_error() {
    let suggestion =
      MacroExpansionError::suggest_fix_for_nix_error("{ x = 1", "syntax error at line 1");
    assert!(suggestion.is_some());
    assert!(suggestion.unwrap().contains("syntax"));
  }
}
