//! Target language definitions for code generation
//!
//! 텍스트 생성만 허용 (헌법 C1 준수)
//! 컴파일러/링커/툴체인 호출 금지
//!
//! ## Supported Languages
//!
//! ### Implemented
//! - JavaScript (JS)
//! - TypeScript (TS)
//! - Python (Py)
//! - Clojure (Clj)
//! - Nix

use serde::{Deserialize, Serialize};
use std::fmt;

/// Target programming language for code generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum TargetLanguage {
  /// JavaScript - Dynamic, web-native
  JavaScript,
  /// TypeScript - Typed JavaScript superset
  TypeScript,
  /// Python - Readable, scientific computing
  Python,
  /// Clojure - Lisp on JVM, immutable data
  Clojure,
  /// Nix - Functional package language
  Nix,
}

impl TargetLanguage {
  /// 파일 확장자 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn extension(&self) -> &'static str {
    match self {
      TargetLanguage::JavaScript => "js",
      TargetLanguage::TypeScript => "ts",
      TargetLanguage::Python => "py",
      TargetLanguage::Clojure => "clj",
      TargetLanguage::Nix => "nix",
    }
  }

  /// 주석 접두사 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn comment_prefix(&self) -> &'static str {
    match self {
      TargetLanguage::JavaScript | TargetLanguage::TypeScript => "//",
      TargetLanguage::Python | TargetLanguage::Nix => "#",
      TargetLanguage::Clojure => ";;",
    }
  }

  /// 언어 표시 이름 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn display_name(&self) -> &'static str {
    match self {
      TargetLanguage::JavaScript => "JavaScript",
      TargetLanguage::TypeScript => "TypeScript",
      TargetLanguage::Python => "Python",
      TargetLanguage::Clojure => "Clojure",
      TargetLanguage::Nix => "Nix",
    }
  }

  /// 언어 ID 반환 (파싱용)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn lang_id(&self) -> &'static str {
    match self {
      TargetLanguage::JavaScript => "js",
      TargetLanguage::TypeScript => "ts",
      TargetLanguage::Python => "py",
      TargetLanguage::Clojure => "clj",
      TargetLanguage::Nix => "nix",
    }
  }

  /// 문자열에서 언어 ID 파싱
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn from_lang_id(id: &str) -> Option<Self> {
    let id = id.trim();
    let id = id.strip_prefix('.').unwrap_or(id);
    if id.eq_ignore_ascii_case("js")
      || id.eq_ignore_ascii_case("javascript")
      || id.eq_ignore_ascii_case("mjs")
      || id.eq_ignore_ascii_case("cjs")
    {
      return Some(TargetLanguage::JavaScript);
    }
    if id.eq_ignore_ascii_case("ts") || id.eq_ignore_ascii_case("typescript") {
      return Some(TargetLanguage::TypeScript);
    }
    if id.eq_ignore_ascii_case("py")
      || id.eq_ignore_ascii_case("python")
      || id.eq_ignore_ascii_case("py3")
    {
      return Some(TargetLanguage::Python);
    }
    if id.eq_ignore_ascii_case("clj")
      || id.eq_ignore_ascii_case("clojure")
      || id.eq_ignore_ascii_case("cljs")
      || id.eq_ignore_ascii_case("cljc")
    {
      return Some(TargetLanguage::Clojure);
    }
    if id.eq_ignore_ascii_case("nix") {
      return Some(TargetLanguage::Nix);
    }
    None
  }

  /// 구현된 모든 언어 목록 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn all_implemented() -> &'static [TargetLanguage] {
    &[
      TargetLanguage::JavaScript,
      TargetLanguage::TypeScript,
      TargetLanguage::Python,
      TargetLanguage::Clojure,
      TargetLanguage::Nix,
    ]
  }

  /// 두 언어 간 변환 안전성 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn conversion_safety(&self, to: TargetLanguage) -> ConversionSafety {
    use ConversionSafety::*;

    match (self, to) {
      // Same language - perfect
      (a, b) if *a == b => Lossless,

      // TypeScript → JavaScript loses types
      (TargetLanguage::TypeScript, TargetLanguage::JavaScript) => TypeInfoLoss,

      // JavaScript → TypeScript adds types (possible)
      (TargetLanguage::JavaScript, TargetLanguage::TypeScript) => PossibleAdaptation,

      // * → Nix safe for pure functional code
      (_, TargetLanguage::Nix) => RequiresPurity,

      // Default: possible but may need adaptation
      _ => PossibleAdaptation,
    }
  }
}

impl fmt::Display for TargetLanguage {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.display_name())
  }
}

/// Conversion safety level between languages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionSafety {
  /// Lossless conversion - no information lost
  Lossless,
  /// Type information may be lost
  TypeInfoLoss,
  /// Requires pure functional code only
  RequiresPurity,
  /// May need manual adaptation
  PossibleAdaptation,
}

impl ConversionSafety {
  /// 변환이 안전한지 확인 (무손실)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_safe(&self) -> bool {
    matches!(self, ConversionSafety::Lossless)
  }

  /// 안전하지 않은 변환에 대한 경고 메시지 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn warning_message(&self) -> Option<&'static str> {
    match self {
      ConversionSafety::Lossless => None,
      ConversionSafety::TypeInfoLoss => Some("Type information will be lost in this conversion"),
      ConversionSafety::RequiresPurity => {
        Some("Only pure functional code can be converted to this target")
      }
      ConversionSafety::PossibleAdaptation => Some("Some constructs may need manual adaptation"),
    }
  }
}

/// Generated code result
#[derive(Debug, Clone)]
pub struct GeneratedCode {
  /// The generated source code
  pub code: String,
  /// Target language
  pub language: TargetLanguage,
  /// Required imports/dependencies
  pub imports: Vec<String>,
  /// Generation warnings
  pub warnings: Vec<String>,
  /// Conversion safety level
  pub safety: ConversionSafety,
}

impl GeneratedCode {
  /// 새 생성 코드 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(code: String, language: TargetLanguage) -> Self {
    Self {
      code,
      language,
      imports: vec![],
      warnings: vec![],
      safety: ConversionSafety::Lossless,
    }
  }

  /// import 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn with_import(mut self, import: impl Into<String>) -> Self {
    self.imports.push(import.into());
    self
  }

  /// 경고 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
    self.warnings.push(warning.into());
    self
  }

  /// 안전성 레벨 설정
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn with_safety(mut self, safety: ConversionSafety) -> Self {
    self.safety = safety;
    self
  }

  /// import를 포함한 전체 코드 반환
  ///
  /// ## 헌법 준수 (P0-1, C1)
  ///
  /// 텍스트 생성만, 파일 I/O 없음
  pub fn full_code(&self) -> String {
    if self.imports.is_empty() {
      self.code.clone()
    } else {
      let imports = self.imports.join("\n");
      format!("{}\n\n{}", imports, self.code)
    }
  }
}

/// Code generation error
#[derive(Debug, Clone)]
pub enum CodeGenError {
  /// Language is not yet implemented
  UnsupportedLanguage { language: TargetLanguage },

  /// Unsupported operation in target language
  UnsupportedOp {
    op: String,
    language: TargetLanguage,
    reason: String,
  },

  /// Type not expressible in target language
  UnsupportedType {
    ty: String,
    language: TargetLanguage,
  },

  /// Internal error
  Internal(String),
}

impl fmt::Display for CodeGenError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      CodeGenError::UnsupportedLanguage { language } => {
        write!(f, "Language '{}' is not yet implemented", language)
      }
      CodeGenError::UnsupportedOp {
        op,
        language,
        reason,
      } => {
        write!(
          f,
          "Operation '{}' not supported in {}: {}",
          op, language, reason
        )
      }
      CodeGenError::UnsupportedType { ty, language } => {
        write!(f, "Type '{}' cannot be expressed in {}", ty, language)
      }
      CodeGenError::Internal(msg) => write!(f, "Internal error: {}", msg),
    }
  }
}

impl std::error::Error for CodeGenError {}

pub type CodeGenResult<T> = Result<T, CodeGenError>;

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_implemented_languages() {
    let langs = TargetLanguage::all_implemented();
    assert_eq!(langs.len(), 5);
    assert!(langs.contains(&TargetLanguage::JavaScript));
    assert!(langs.contains(&TargetLanguage::TypeScript));
    assert!(langs.contains(&TargetLanguage::Python));
    assert!(langs.contains(&TargetLanguage::Clojure));
    assert!(langs.contains(&TargetLanguage::Nix));
  }

  #[test]
  fn test_lang_id_parsing() {
    assert_eq!(
      TargetLanguage::from_lang_id("js"),
      Some(TargetLanguage::JavaScript)
    );
    assert_eq!(
      TargetLanguage::from_lang_id("typescript"),
      Some(TargetLanguage::TypeScript)
    );
    assert_eq!(
      TargetLanguage::from_lang_id("  .PY3  "),
      Some(TargetLanguage::Python)
    );
    assert_eq!(
      TargetLanguage::from_lang_id("CLJS"),
      Some(TargetLanguage::Clojure)
    );
    assert_eq!(TargetLanguage::from_lang_id("unknown"), None);
  }

  #[test]
  fn test_file_extensions() {
    assert_eq!(TargetLanguage::JavaScript.extension(), "js");
    assert_eq!(TargetLanguage::TypeScript.extension(), "ts");
    assert_eq!(TargetLanguage::Python.extension(), "py");
  }

  #[test]
  fn test_conversion_safety() {
    // Same language is lossless
    assert_eq!(
      TargetLanguage::JavaScript.conversion_safety(TargetLanguage::JavaScript),
      ConversionSafety::Lossless
    );

    // TS → JS loses types
    assert_eq!(
      TargetLanguage::TypeScript.conversion_safety(TargetLanguage::JavaScript),
      ConversionSafety::TypeInfoLoss
    );
  }

  #[test]
  fn test_all_implemented() {
    let langs = TargetLanguage::all_implemented();
    assert_eq!(langs.len(), 5);
    assert!(langs.contains(&TargetLanguage::JavaScript));
    assert!(langs.contains(&TargetLanguage::TypeScript));
  }

  #[test]
  fn test_generated_code() {
    let code = GeneratedCode::new("const x = 1;".to_string(), TargetLanguage::JavaScript)
      .with_import("import { foo } from 'bar';")
      .with_warning("Type inference may be incomplete");

    assert_eq!(code.language, TargetLanguage::JavaScript);
    assert_eq!(code.imports.len(), 1);
    assert_eq!(code.warnings.len(), 1);
    assert!(code.full_code().starts_with("import"));
  }
}
