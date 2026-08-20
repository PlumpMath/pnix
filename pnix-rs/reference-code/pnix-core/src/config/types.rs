//! 설정 타입 정의
//!
//! pnix-old의 pnix_config에서 마이그레이션된 순수 데이터 타입.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 데이터 구조만, I/O 없음, 런타임 의존 없음

use serde::{Deserialize, Serialize};

// ============================================================================
// REPL Types
// ============================================================================

/// REPL 모드: REPL의 실행 모드
///
/// # OWNER-LAW NOTE (2026-05-10)
///
/// **pnix 는 LLM 없이 작동하는 deterministic AI substrate** (`CLAUDE.md`
/// OWNER-LAW CONSTITUTION). `ReplMode::Llm` variant 는 외부 Claude 연동
/// REPL mode 로 substrate 의 *외부 도구 시작 lane* 이지 substrate 안의
/// 의미/판단 owner 가 아니다. 이 mode 가 활성일 때도 substrate 의 ontology
/// lifecycle 은 변하지 않고, 외부 LLM output 은 untrusted external prose
/// 로만 들어온다. 향후 refactor 에서 variant 이름을
/// `ExternalProse` / `ExternalAgent` 같은 LLM-free 이름으로 rename 한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ReplMode {
  /// 프로그래밍 모드 (기본 Pnix 인터프리터)
  #[default]
  Programming,
  /// 외부 LLM 도구 launcher mode (legacy 이름 "LLM 모드 / Claude 연동").
  /// substrate 안의 의미 owner 가 아니라 외부 도구 surface 다 — 외부 LLM
  /// output 은 untrusted external prose 로만 들어온다.
  Llm,
  /// Symbolic 모드 (NL Parser 기반 수학/물리 계산)
  Symbolic,
}

impl ReplMode {
  /// 다음 모드로 순환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn next(self) -> Self {
    match self {
      ReplMode::Programming => ReplMode::Llm,
      ReplMode::Llm => ReplMode::Symbolic,
      ReplMode::Symbolic => ReplMode::Programming,
    }
  }

  /// 모드 이름
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn name(&self) -> &'static str {
    match self {
      ReplMode::Programming => "Programming",
      ReplMode::Llm => "LLM",
      ReplMode::Symbolic => "Symbolic",
    }
  }

  /// 프롬프트 접두사
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn prompt_prefix(&self) -> &'static str {
    match self {
      ReplMode::Programming => "pnix",
      ReplMode::Llm => "llm",
      ReplMode::Symbolic => "sym",
    }
  }

  /// 모드 설명
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn description(&self) -> &'static str {
    match self {
      ReplMode::Programming => "Clojure/Nix 프로그래밍",
      ReplMode::Llm => "LLM 대화 (Claude)",
      ReplMode::Symbolic => "수학/물리 심볼릭 계산",
    }
  }
}

impl std::fmt::Display for ReplMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.name())
  }
}

impl std::str::FromStr for ReplMode {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_lowercase().as_str() {
      "programming" | "prog" | "pnix" => Ok(ReplMode::Programming),
      "llm" | "claude" | "ai" => Ok(ReplMode::Llm),
      "symbolic" | "sym" | "math" => Ok(ReplMode::Symbolic),
      _ => Err(format!(
        "Unknown mode: {}. Use: programming, llm, symbolic",
        s
      )),
    }
  }
}

/// 프롬프트 스타일: REPL 프롬프트의 표시 스타일
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PromptStyle {
  /// 최소화 (모드만 표시)
  #[default]
  Minimal,
  /// 전체 (모드 + 상태)
  Full,
  /// 커스텀 (사용자 정의)
  Custom,
}

/// REPL 설정: REPL의 설정 (순수 데이터만)
///
/// 파일 경로 등 런타임 의존 필드는 executor에서 처리
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReplConfig {
  /// 기본 REPL 모드 (시작 시 기본 모드)
  pub default_mode: ReplMode,
  /// 프롬프트 스타일 (프롬프트 표시 방식)
  pub prompt_style: PromptStyle,
  /// 커스텀 프롬프트 (prompt_style이 Custom일 때 사용할 프롬프트 문자열)
  pub custom_prompt: Option<String>,
  /// 히스토리 파일 경로 (문자열로만 저장, 실제 경로 확인은 executor에서)
  pub history_file: Option<String>,
  /// 최대 히스토리 항목 수 (저장할 최대 히스토리 항목 수)
  pub max_history: usize,
  /// 모드 전환 키 (기본: F2, 모드 전환에 사용할 키)
  pub mode_toggle_key: String,
}

impl Default for ReplConfig {
  fn default() -> Self {
    Self {
      default_mode: ReplMode::Programming,
      prompt_style: PromptStyle::Minimal,
      custom_prompt: None,
      history_file: None,
      max_history: 1000,
      mode_toggle_key: "F2".to_string(),
    }
  }
}

// ============================================================================
// External LLM Tool Launcher Types (legacy section name "LLM Types")
// ============================================================================
//
// OWNER-LAW NOTE (2026-05-10):
//   pnix 는 LLM 없이 작동하는 deterministic AI substrate (CLAUDE.md OWNER-LAW
//   CONSTITUTION). 이 section 의 LlmProvider / LlmConfig 는 substrate 안의
//   의미/판단 owner 가 아니라, 인간이 명시적으로 외부 LLM 도구 (Claude API
//   등) 를 launcher 처럼 시작할 때 쓰는 outbound HTTP client 설정이다.
//   외부 LLM output 은 substrate 의 다른 untrusted external prose 와 동일하게
//   provenance / Held / replay discipline 아래의 candidate 로 들어오고,
//   substrate enumeration category 가 아니다. 향후 refactor 에서 이 type 들
//   을 `ExternalLlmToolConfig` / `ExternalLlmProvider` 같은 outbound-tool
//   이름으로 rename 한다.

/// 외부 LLM tool 제공자 (legacy 이름 "LLM 제공자")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
  /// Claude (Anthropic)
  #[default]
  Claude,
  /// OpenAI
  OpenAi,
  /// 로컬 LLM
  Local,
  /// 커스텀 제공자
  #[serde(untagged)]
  Custom(
    /// 커스텀 제공자 이름
    String,
  ),
}

/// LLM 설정: LLM의 설정 (순수 데이터만)
///
/// API 키 자체는 환경변수에서 읽어야 하므로 executor에서 처리
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LlmConfig {
  /// LLM 제공자 (사용할 LLM 서비스 제공자)
  pub provider: LlmProvider,
  /// API 키 환경 변수 이름 (키 값이 아닌 변수 이름만)
  pub api_key_env: String,
  /// 모델 이름 (사용할 모델 이름)
  pub model: String,
  /// 최대 토큰 수 (최대 생성 토큰 수)
  pub max_tokens: usize,
  /// Temperature (창의성 조절, 0.0 ~ 1.0)
  pub temperature: f64,
  /// 시스템 프롬프트 (시스템 레벨 지시사항, 선택적)
  pub system_prompt: Option<String>,
  /// API 엔드포인트 (Custom provider용, 선택적)
  pub endpoint: Option<String>,
}

impl Default for LlmConfig {
  fn default() -> Self {
    Self {
      provider: LlmProvider::Claude,
      api_key_env: "ANTHROPIC_API_KEY".to_string(),
      model: "claude-3-5-sonnet-20241022".to_string(),
      max_tokens: 4096,
      temperature: 0.7,
      system_prompt: None,
      endpoint: None,
    }
  }
}

// ============================================================================
// Symbolic Types
// ============================================================================

/// Symbolic 모드 설정: Symbolic 모드의 설정
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SymbolicConfig {
  /// 자동 단순화 (표현식을 자동으로 단순화할지 여부)
  pub auto_simplify: bool,
  /// 단계별 표시 (계산 단계를 표시할지 여부)
  pub show_steps: bool,
  /// LaTeX 출력 (결과를 LaTeX 형식으로 출력할지 여부)
  pub latex_output: bool,
  /// 신뢰도 임계값 (NL 파서, 0.0 ~ 1.0, 이 값 이상이어야 파싱 성공)
  pub confidence_threshold: f64,
  /// 기본 변수 (미분 등에서 사용할 기본 변수 이름)
  pub default_variable: String,
}

impl Default for SymbolicConfig {
  fn default() -> Self {
    Self {
      auto_simplify: true,
      show_steps: true,
      latex_output: false,
      confidence_threshold: 0.5,
      default_variable: "x".to_string(),
    }
  }
}

// ============================================================================
// Combined Config (순수 구조만)
// ============================================================================

/// 전체 Pnix 설정: 전체 Pnix 설정 (순수 데이터)
///
/// 파일 로드/저장은 executor에서 처리
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PnixConfig {
  /// REPL 설정 (REPL 관련 설정)
  pub repl: ReplConfig,
  /// LLM 설정 (LLM 관련 설정)
  pub llm: LlmConfig,
  /// Symbolic 설정 (Symbolic 모드 관련 설정)
  pub symbolic: SymbolicConfig,
}

impl PnixConfig {
  /// 기본 설정 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self::default()
  }

  /// JSON 문자열에서 파싱 (순수 함수)
  ///
  /// YAML 파싱은 executor에서 serde_yaml을 사용하여 처리
  /// (pnix-core는 실행/IO 금지; 런타임 처리는 상위 계층에서 담당)
  pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
    serde_json::from_str(json)
  }

  /// JSON 문자열로 직렬화 (순수 함수)
  ///
  /// ## 헌법 준수 (P0-1, C1)
  ///
  /// 텍스트 생성만, 파일 I/O 없음
  pub fn to_json(&self) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(self)
  }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_repl_mode_cycle() {
    let mode = ReplMode::Programming;
    assert_eq!(mode.next(), ReplMode::Llm);
    assert_eq!(mode.next().next(), ReplMode::Symbolic);
    assert_eq!(mode.next().next().next(), ReplMode::Programming);
  }

  #[test]
  fn test_repl_mode_from_str() {
    assert_eq!(
      "programming".parse::<ReplMode>().unwrap(),
      ReplMode::Programming
    );
    assert_eq!("llm".parse::<ReplMode>().unwrap(), ReplMode::Llm);
    assert_eq!("symbolic".parse::<ReplMode>().unwrap(), ReplMode::Symbolic);
    assert!("invalid".parse::<ReplMode>().is_err());
  }

  #[test]
  fn test_pnix_config_json_roundtrip() {
    let config = PnixConfig::default();
    let json = config.to_json().unwrap();
    let parsed = PnixConfig::from_json(&json).unwrap();
    assert_eq!(config, parsed);
  }

  #[test]
  fn test_llm_provider_serde() {
    let provider = LlmProvider::Claude;
    let json = serde_json::to_string(&provider).unwrap();
    assert_eq!(json, "\"claude\"");
  }

  #[test]
  fn test_symbolic_config_defaults() {
    let config = SymbolicConfig::default();
    assert!(config.auto_simplify);
    assert!(config.show_steps);
    assert!(!config.latex_output);
    assert_eq!(config.default_variable, "x");
  }
}
