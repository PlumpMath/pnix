//! Desktop 플랫폼 구조 정의
//!
//! pnix-old의 pnix_platform/src/desktop.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 파일 시스템/로깅 실행 로직 제외
//!
//! ## 참고
//!
//! 실제 파일 시스템/로깅 실행 로직은 executor에서 구현합니다.
//! 이 모듈은 구조 정의만 포함합니다.

use serde::{Deserialize, Serialize};

/// Desktop 파일 시스템 구조
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopFileSystem {
  /// 파일 시스템 초기화 상태 (실제 초기화는 executor에서)
  pub initialized: bool,
}

impl DesktopFileSystem {
  /// 새 Desktop 파일 시스템 구조 생성
  pub fn new() -> Self {
    Self { initialized: false }
  }
}

impl Default for DesktopFileSystem {
  fn default() -> Self {
    Self::new()
  }
}

/// Desktop 로깅 구조
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopLog {
  /// 로그 레벨 (실제 로깅은 executor에서)
  pub level: String,
}

impl DesktopLog {
  /// 새 Desktop 로그 구조 생성
  pub fn new() -> Self {
    Self {
      level: "info".to_string(),
    }
  }
}

impl Default for DesktopLog {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_desktop_file_system() {
    let fs = DesktopFileSystem::new();
    assert!(!fs.initialized);
  }

  #[test]
  fn test_desktop_log() {
    let log = DesktopLog::new();
    assert_eq!(log.level, "info");
  }
}
