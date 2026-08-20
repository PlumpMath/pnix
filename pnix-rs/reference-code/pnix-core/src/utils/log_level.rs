//! Common Log Level types
//!
//! pnix-old의 pnix_utils/src/log_level.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! enum 정의 및 순수 비교 함수만, 로그 실행 로직 제외

/// Common log level for application logging
#[derive(
  Clone,
  Copy,
  Debug,
  PartialEq,
  Eq,
  PartialOrd,
  Ord,
  Hash,
  Default,
  serde::Serialize,
  serde::Deserialize,
)]
#[serde(rename_all = "PascalCase")]
pub enum LogLevel {
  /// Detailed debugging information
  Debug,
  /// General informational messages
  #[default]
  Info,
  /// Warning messages (potential issues)
  Warning,
  /// Error messages (failures)
  Error,
}

impl LogLevel {
  /// 문자열 표현 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn as_str(&self) -> &'static str {
    match self {
      LogLevel::Debug => "DEBUG",
      LogLevel::Info => "INFO",
      LogLevel::Warning => "WARN",
      LogLevel::Error => "ERROR",
    }
  }

  /// 레벨 비교를 위한 우선순위 값 반환 (낮을수록 높은 우선순위)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn priority(&self) -> u8 {
    match self {
      LogLevel::Debug => 0,
      LogLevel::Info => 1,
      LogLevel::Warning => 2,
      LogLevel::Error => 3,
    }
  }

  /// 로그 레벨이 다른 레벨보다 높거나 같은지 확인 (순수 비교 함수)
  ///
  /// # Examples
  ///
  /// ```rust
  /// use pnix_core::utils::log_level::LogLevel;
  ///
  /// assert!(LogLevel::Error.is_at_least(LogLevel::Warning));
  /// assert!(!LogLevel::Debug.is_at_least(LogLevel::Info));
  /// ```
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 비교만, 값 계산 없음
  pub fn is_at_least(&self, other: Self) -> bool {
    self.priority() >= other.priority()
  }

  /// 문자열에서 LogLevel 파싱 (순수 파싱 함수)
  ///
  /// 대소문자 구분 없이 파싱합니다.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use pnix_core::utils::log_level::LogLevel;
  ///
  /// assert_eq!(LogLevel::from_str("debug"), Some(LogLevel::Debug));
  /// assert_eq!(LogLevel::from_str("INFO"), Some(LogLevel::Info));
  /// assert_eq!(LogLevel::from_str("Warning"), Some(LogLevel::Warning));
  /// ```
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 파싱만, 값 계산 없음
  #[allow(clippy::should_implement_trait)]
  pub fn from_str(s: &str) -> Option<Self> {
    match s.to_lowercase().as_str() {
      "debug" => Some(LogLevel::Debug),
      "info" => Some(LogLevel::Info),
      "warning" | "warn" => Some(LogLevel::Warning),
      "error" => Some(LogLevel::Error),
      _ => None,
    }
  }

  /// 모든 로그 레벨 목록 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn all() -> Vec<Self> {
    vec![
      LogLevel::Debug,
      LogLevel::Info,
      LogLevel::Warning,
      LogLevel::Error,
    ]
  }

  /// 특정 레벨 이상의 모든 레벨 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn from_minimum(min: Self) -> Vec<Self> {
    Self::all()
      .into_iter()
      .filter(|level| level.priority() >= min.priority())
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_from_str() {
    assert_eq!(LogLevel::from_str("debug"), Some(LogLevel::Debug));
    assert_eq!(LogLevel::from_str("DEBUG"), Some(LogLevel::Debug));
    assert_eq!(LogLevel::from_str("info"), Some(LogLevel::Info));
    assert_eq!(LogLevel::from_str("warning"), Some(LogLevel::Warning));
    assert_eq!(LogLevel::from_str("warn"), Some(LogLevel::Warning));
    assert_eq!(LogLevel::from_str("error"), Some(LogLevel::Error));
    assert_eq!(LogLevel::from_str("invalid"), None);
  }

  #[test]
  fn test_all() {
    let levels = LogLevel::all();
    assert_eq!(levels.len(), 4);
    assert!(levels.contains(&LogLevel::Debug));
    assert!(levels.contains(&LogLevel::Info));
    assert!(levels.contains(&LogLevel::Warning));
    assert!(levels.contains(&LogLevel::Error));
  }

  #[test]
  fn test_from_minimum() {
    let levels = LogLevel::from_minimum(LogLevel::Warning);
    assert_eq!(levels.len(), 2);
    assert!(levels.contains(&LogLevel::Warning));
    assert!(levels.contains(&LogLevel::Error));
    assert!(!levels.contains(&LogLevel::Debug));
    assert!(!levels.contains(&LogLevel::Info));
  }

  #[test]
  fn test_is_at_least() {
    assert!(LogLevel::Error.is_at_least(LogLevel::Warning));
    assert!(LogLevel::Warning.is_at_least(LogLevel::Warning));
    assert!(!LogLevel::Info.is_at_least(LogLevel::Warning));
    assert!(!LogLevel::Debug.is_at_least(LogLevel::Info));
  }

  #[test]
  fn test_priority() {
    assert!(LogLevel::Debug.priority() < LogLevel::Info.priority());
    assert!(LogLevel::Info.priority() < LogLevel::Warning.priority());
    assert!(LogLevel::Warning.priority() < LogLevel::Error.priority());
  }
}
