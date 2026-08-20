//! Test 구조 정의
//!
//! pnix-old의 pnix_test_runner/src/lib.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! - TestMetadata: 테스트 메타데이터 구조 정의
//! - TestResult: 테스트 결과 구조 정의
//! - 실제 테스트 실행, 파싱, 실행 로직은 executor에서 구현

use serde::{Deserialize, Serialize};

/// 테스트 메타데이터 구조: 테스트 실행을 위한 메타데이터 구조
///
/// 헌법 P0-1 준수: 구조 정의만, 값 계산 없음
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestMetadata {
  /// 예상 실패 (XFAIL) - 테스트가 실패해야 함
  pub xfail: bool,
  /// XFAIL 이유
  pub xfail_reason: Option<String>,
  /// 타임아웃 (초, 0 = 기본값)
  pub timeout: Option<u64>,
  /// 설정할 환경 변수들
  pub env: Vec<(String, String)>,
  /// 스텁된 테스트 - 실제로 테스트하지 않는 플레이스홀더
  pub stubbed: bool,
  /// 스텁 이유
  pub stubbed_reason: Option<String>,
  /// 스킵 테스트 - 테스트를 건너뛰어야 함 (실행하지 않음)
  pub skip: bool,
  /// 스킵 이유
  pub skip_reason: Option<String>,
  /// World zone 작업 허용 (EvalPolicy)
  pub allow_world: bool,
  /// 특정 IO 작업 허용 (쉼표로 구분)
  pub allow_io: Option<String>,
}

impl TestMetadata {
  /// 새로운 테스트 메타데이터 생성 (구조만)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self {
      xfail: false,
      xfail_reason: None,
      timeout: None,
      env: Vec::new(),
      stubbed: false,
      stubbed_reason: None,
      skip: false,
      skip_reason: None,
      allow_world: false,
      allow_io: None,
    }
  }

  /// XFAIL 설정 (구조 변경만)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 설정만, 값 계산 없음
  pub fn with_xfail(mut self, reason: Option<String>) -> Self {
    self.xfail = true;
    self.xfail_reason = reason;
    self
  }

  /// 타임아웃 설정 (구조 변경만)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 설정만, 값 계산 없음
  pub fn with_timeout(mut self, timeout: u64) -> Self {
    self.timeout = Some(timeout);
    self
  }

  /// 환경 변수 추가 (구조 변경만)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
    self.env.push((key.into(), value.into()));
    self
  }

  /// 스텁 설정 (구조 변경만)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 설정만, 값 계산 없음
  pub fn with_stubbed(mut self, reason: Option<String>) -> Self {
    self.stubbed = true;
    self.stubbed_reason = reason;
    self
  }

  /// 스킵 설정 (구조 변경만)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 설정만, 값 계산 없음
  pub fn with_skip(mut self, reason: Option<String>) -> Self {
    self.skip = true;
    self.skip_reason = reason;
    self
  }

  /// World zone 허용 설정 (구조 변경만)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 설정만, 값 계산 없음
  pub fn with_allow_world(mut self, allow_io: Option<String>) -> Self {
    self.allow_world = true;
    self.allow_io = allow_io;
    self
  }
}

impl Default for TestMetadata {
  fn default() -> Self {
    Self::new()
  }
}

/// 테스트 결과 구조: 테스트 실행 결과를 저장하는 구조
///
/// 헌법 P0-1 준수: 구조 정의만, 값 계산 없음
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
  /// 테스트 파일 경로
  pub file_path: String,
  /// 테스트 이름
  pub name: String,
  /// 테스트 상태
  pub status: TestStatus,
  /// 실행 시간 (밀리초, 구조 정의만, 실제 측정은 executor에서)
  pub duration_ms: u64,
  /// 에러 메시지 (실패 시)
  pub error: Option<String>,
  /// 출력 (선택적)
  pub output: Option<String>,
  /// 메타데이터
  pub metadata: TestMetadata,
}

/// 테스트 상태: 테스트 실행 결과 상태 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestStatus {
  /// 통과
  Pass,
  /// 실패
  Fail,
  /// 스킵됨
  Skip,
  /// 스텁됨
  Stubbed,
  /// 타임아웃
  Timeout,
}

impl TestResult {
  /// 새로운 테스트 결과 생성 (구조만)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(file_path: impl Into<String>, name: impl Into<String>, status: TestStatus) -> Self {
    Self {
      file_path: file_path.into(),
      name: name.into(),
      status,
      duration_ms: 0,
      error: None,
      output: None,
      metadata: TestMetadata::default(),
    }
  }

  /// 에러 메시지 설정 (구조 변경만)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 설정만, 값 계산 없음
  pub fn with_error(mut self, error: impl Into<String>) -> Self {
    self.error = Some(error.into());
    self
  }

  /// 출력 설정 (구조 변경만)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 설정만, 값 계산 없음
  pub fn with_output(mut self, output: impl Into<String>) -> Self {
    self.output = Some(output.into());
    self
  }

  /// 메타데이터 설정 (구조 변경만)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 설정만, 값 계산 없음
  pub fn with_metadata(mut self, metadata: TestMetadata) -> Self {
    self.metadata = metadata;
    self
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - parse_test_metadata() (파일 파싱, 메타데이터 추출)
// - TestRunner 구조체 및 메서드들 (테스트 실행, 결과 수집)
// - EventLoop 구조체 및 메서드들 (이벤트 루프 실행)
//
// 이 함수들은 파일 I/O, 테스트 실행, 또는 상태 관리를 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_metadata_creation() {
    let metadata = TestMetadata::new();
    assert!(!metadata.xfail);
    assert!(!metadata.stubbed);
    assert!(!metadata.skip);
  }

  #[test]
  fn test_metadata_with_xfail() {
    let metadata = TestMetadata::new().with_xfail(Some("Known issue".to_string()));
    assert!(metadata.xfail);
    assert_eq!(metadata.xfail_reason, Some("Known issue".to_string()));
  }

  #[test]
  fn test_result_creation() {
    let result = TestResult::new("test.sam", "test1", TestStatus::Pass);
    assert_eq!(result.file_path, "test.sam");
    assert_eq!(result.name, "test1");
    assert_eq!(result.status, TestStatus::Pass);
  }

  #[test]
  fn test_result_with_error() {
    let result = TestResult::new("test.sam", "test1", TestStatus::Fail).with_error("Test failed");
    assert_eq!(result.error, Some("Test failed".to_string()));
  }
}
