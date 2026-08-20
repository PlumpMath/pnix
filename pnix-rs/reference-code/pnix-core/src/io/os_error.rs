//! OS Error 구조 정의
//!
//! pnix-old의 pnix_io_runtime/src/os_abstraction.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, OS 호출 실행 로직 제외
//! - OsError: OS 에러 타입 정의
//! - 실제 OS 호출 (read_file, write_file, exec 등)은 executor에서 구현

use serde::{Deserialize, Serialize};

/// OS 추상화 에러 타입: OS 작업 중 발생하는 에러 타입
///
/// 헌법 P0-1 준수: 구조 정의만, 실행 로직 제외
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OsError {
  /// 파일/리소스를 찾을 수 없음
  NotFound(
    /// 리소스 경로 또는 이름
    String,
  ),
  /// 권한 거부
  PermissionDenied(
    /// 리소스 경로 또는 이름
    String,
  ),
  /// 이미 존재함
  AlreadyExists(
    /// 리소스 경로 또는 이름
    String,
  ),
  /// 잘못된 입력
  InvalidInput(
    /// 입력 값 또는 설명
    String,
  ),
  /// IO 에러
  IoError(
    /// 에러 메시지
    String,
  ),
  /// 지원하지 않음
  Unsupported(
    /// 기능 또는 설명
    String,
  ),
  /// 타임아웃 에러
  Timeout(
    /// 작업 설명
    String,
  ),
  /// 취소 에러
  Cancelled(
    /// 작업 설명
    String,
  ),
  /// 에러 체이닝을 위한 컨텍스트
  WithContext {
    /// 컨텍스트 메시지
    context: String,
    /// 원본 에러
    source: Box<OsError>,
  },
}

impl OsError {
  /// 컨텍스트 추가 (구조 변경만)
  pub fn with_context(self, context: String) -> Self {
    Self::WithContext {
      context,
      source: Box::new(self),
    }
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - read_file(path) -> Result<String, OsError>
// - write_file(path, content) -> Result<(), OsError>
// - exec(command, args) -> Result<ProcessResult, OsError>
// - get(url) -> Result<HttpResponse, OsError>
// - post(url, body) -> Result<HttpResponse, OsError>
//
// 이 함수들은 실제 OS 호출 및 실행을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_os_error_creation() {
    let err = OsError::NotFound("file.txt".to_string());
    assert!(matches!(err, OsError::NotFound(_)));
  }

  #[test]
  fn test_os_error_with_context() {
    let err = OsError::NotFound("file.txt".to_string());
    let err_with_ctx = err.with_context("Reading config".to_string());
    assert!(matches!(err_with_ctx, OsError::WithContext { .. }));
  }
}
