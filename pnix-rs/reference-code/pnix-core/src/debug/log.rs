//! Debug Log 구조 정의
//!
//! pnix-old의 pnix_debug_console/src/lib.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! - LogEntry: 로그 엔트리 구조 정의
//! - ConsoleStats: 콘솔 통계 구조 정의
//! - ConsoleFilter: 콘솔 필터 구조 정의
//! - 실제 로깅, 필터링, 포맷팅 로직은 executor에서 구현

use crate::utils::log_level::LogLevel;
use serde::{Deserialize, Serialize};

/// 단일 로그 엔트리 구조 (순수 데이터)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
  /// 로그 레벨
  pub level: LogLevel,
  /// 로그 메시지
  pub message: String,
  /// 타임스탬프 (밀리초, 구조 정의만, 실제 시간은 executor에서 설정)
  pub timestamp_ms: u64,
  /// 벽시계 시간 (문자열, 구조 정의만, 실제 시간은 executor에서 설정)
  pub wall_time: String,
  /// 소스 파일 (선택적)
  pub source_file: Option<String>,
  /// 소스 라인 (선택적)
  pub source_line: Option<u32>,
  /// 프레임 번호 (로그가 기록된 시점)
  pub frame: u64,
  /// 카테고리/태그 (필터링용)
  pub category: Option<String>,
  /// 스택 트레이스 (에러용, 선택적)
  pub stack_trace: Option<String>,
  /// 관련 데이터 (JSON-like, 선택적)
  pub data: Option<String>,
}

impl LogEntry {
  /// 새로운 로그 엔트리 생성 (구조만)
  pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
    Self {
      level,
      message: message.into(),
      timestamp_ms: 0,          // 실제 시간은 executor에서 설정
      wall_time: String::new(), // 실제 시간은 executor에서 설정
      source_file: None,
      source_line: None,
      frame: 0,
      category: None,
      stack_trace: None,
      data: None,
    }
  }

  /// 소스 위치 설정 (구조 변경만)
  pub fn with_source(mut self, file: impl Into<String>, line: u32) -> Self {
    self.source_file = Some(file.into());
    self.source_line = Some(line);
    self
  }

  /// 프레임 번호 설정 (구조 변경만)
  pub fn with_frame(mut self, frame: u64) -> Self {
    self.frame = frame;
    self
  }

  /// 카테고리 설정 (구조 변경만)
  pub fn with_category(mut self, category: impl Into<String>) -> Self {
    self.category = Some(category.into());
    self
  }

  /// 데이터 설정 (구조 변경만)
  pub fn with_data(mut self, data: impl Into<String>) -> Self {
    self.data = Some(data.into());
    self
  }

  /// 스택 트레이스 설정 (구조 변경만)
  pub fn with_stack_trace(mut self, trace: impl Into<String>) -> Self {
    self.stack_trace = Some(trace.into());
    self
  }
}

/// 콘솔 통계 구조 (순수 데이터)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleStats {
  /// 총 로그 수
  pub total: usize,
  /// Debug 로그 수
  pub debug: usize,
  /// Info 로그 수
  pub info: usize,
  /// Warning 로그 수
  pub warning: usize,
  /// Error 로그 수
  pub error: usize,
}

impl ConsoleStats {
  /// 새로운 콘솔 통계 생성 (구조만)
  pub fn new() -> Self {
    Self {
      total: 0,
      debug: 0,
      info: 0,
      warning: 0,
      error: 0,
    }
  }
}

impl Default for ConsoleStats {
  fn default() -> Self {
    Self::new()
  }
}

/// 콘솔 필터 설정 구조 (순수 데이터)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsoleFilter {
  /// Debug 로그 표시 여부
  pub show_debug: bool,
  /// Info 로그 표시 여부
  pub show_info: bool,
  /// Warning 로그 표시 여부
  pub show_warning: bool,
  /// Error 로그 표시 여부
  pub show_error: bool,
  /// 카테고리 필터 (선택적)
  pub category_filter: Option<String>,
  /// 검색 텍스트 (선택적)
  pub search_text: Option<String>,
  /// 최소 로그 레벨 (선택적)
  pub min_level: Option<LogLevel>,
  /// 시간 범위 필터 (선택적, (start_ms, end_ms))
  pub time_range: Option<(u64, u64)>,
}

impl Default for ConsoleFilter {
  fn default() -> Self {
    Self {
      show_debug: false, // Debug는 기본적으로 숨김
      show_info: true,
      show_warning: true,
      show_error: true,
      category_filter: None,
      search_text: None,
      min_level: None,
      time_range: None,
    }
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - LogEntry::format_console() -> String (콘솔 출력 포맷팅)
// - ConsoleFilter::matches() -> bool (필터 매칭 로직, 값 계산)
// - DebugConsole 구조체 및 메서드들 (로그 수집, 관리)
// - DebugConsoleState 구조체 및 메서드들 (상태 관리)
//
// 이 함수들은 값 계산, 상태 변경, 또는 시간 측정을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_log_entry_creation() {
    let entry = LogEntry::new(LogLevel::Info, "test message");
    assert_eq!(entry.level, LogLevel::Info);
    assert_eq!(entry.message, "test message");
  }

  #[test]
  fn test_log_entry_with_source() {
    let entry = LogEntry::new(LogLevel::Error, "error").with_source("test.rs", 10);
    assert_eq!(entry.source_file, Some("test.rs".to_string()));
    assert_eq!(entry.source_line, Some(10));
  }

  #[test]
  fn test_console_stats_creation() {
    let stats = ConsoleStats::new();
    assert_eq!(stats.total, 0);
    assert_eq!(stats.debug, 0);
  }

  #[test]
  fn test_console_filter_default() {
    let filter = ConsoleFilter::default();
    assert!(!filter.show_debug);
    assert!(filter.show_info);
    assert!(filter.show_warning);
    assert!(filter.show_error);
  }
}
