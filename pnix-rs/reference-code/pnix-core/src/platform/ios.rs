//! iOS 플랫폼 구조 정의
//!
//! pnix-old의 pnix_platform/src/ios.rs에서 마이그레이션.
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

/// iOS 파일 시스템 구조
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IosFileSystem {
  /// 파일 시스템 초기화 상태 (실제 초기화는 executor에서)
  pub initialized: bool,
}

impl IosFileSystem {
  /// 새 iOS 파일 시스템 구조 생성
  pub fn new() -> Self {
    Self { initialized: false }
  }
}

impl Default for IosFileSystem {
  fn default() -> Self {
    Self::new()
  }
}

/// iOS 로깅 구조
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IosLog {
  /// 로그 레벨 (실제 로깅은 executor에서)
  pub level: String,
}

impl IosLog {
  /// 새 iOS 로그 구조 생성
  pub fn new() -> Self {
    Self {
      level: "info".to_string(),
    }
  }
}

impl Default for IosLog {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_ios_file_system() {
    let fs = IosFileSystem::new();
    assert!(!fs.initialized);
  }

  #[test]
  fn test_ios_log() {
    let log = IosLog::new();
    assert_eq!(log.level, "info");
  }
}
