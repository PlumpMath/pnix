//! 설정 마이그레이션 구조 정의
//!
//! pnix-old의 pnix_config/src/migration.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 버전 파싱/구조 변환만, 파일 I/O 없음
//!
//! ## 설계 철학
//!
//! 설정 버전 관리와 마이그레이션 리포트를 위한
//! 데이터 구조를 정의합니다.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 설정 버전: 설정 파일의 버전
#[derive(
  Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub enum ConfigVersion {
  /// 버전 1.0 (초기 버전)
  V1_0,
  /// 버전 1.1 (현재 버전)
  V1_1,
  /// 최신 버전 (현재 최신 버전)
  #[default]
  Latest,
}

impl ConfigVersion {
  /// 버전 문자열
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn as_str(&self) -> &'static str {
    match self {
      ConfigVersion::V1_0 => "1.0",
      ConfigVersion::V1_1 => "1.1",
      ConfigVersion::Latest => "1.1",
    }
  }

  /// 버전 파싱
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn parse(s: &str) -> Option<Self> {
    match s {
      "1.0" => Some(ConfigVersion::V1_0),
      "1.1" => Some(ConfigVersion::V1_1),
      "latest" => Some(ConfigVersion::Latest),
      _ => None,
    }
  }

  /// 마이그레이션이 필요한지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 비교만, 값 계산 없음
  pub fn needs_migration(&self) -> bool {
    *self < ConfigVersion::V1_1
  }
}

impl std::fmt::Display for ConfigVersion {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.as_str())
  }
}

// ============================================================
// Backup Structures (pnix-old pnix_config/src/backup.rs에서 마이그레이션)
// ============================================================

/// 백업 메타데이터 구조: 백업 파일의 메타데이터
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 파일 I/O 실행 로직 제외
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
  /// 백업 ID (백업의 고유 식별자)
  pub id: String,
  /// 원본 경로 (구조 정의만, 백업한 원본 파일 경로)
  pub source: String,
  /// 백업 경로 (구조 정의만, 백업 파일 경로)
  pub backup_path: String,
  /// 백업 타임스탬프 (구조 정의만, 실제 측정은 executor에서, Unix 타임스탬프)
  pub timestamp: u64,
  /// 백업 타입 (백업의 종류)
  pub backup_type: BackupType,
  /// 파일 크기 (바이트, 구조 정의만, 백업 파일 크기)
  pub size: u64,
  /// 체크섬 (선택적, 구조 정의만, 파일 무결성 검증용)
  pub checksum: Option<String>,
  /// 기반 백업 ID (증분 백업의 경우, 기반이 되는 백업 ID)
  pub base_backup_id: Option<String>,
}

/// 백업 타입: 백업의 종류
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupType {
  /// 전체 백업 (전체 파일 백업)
  Full,
  /// 증분 백업 (변경된 부분만 백업)
  Incremental,
}

/// 백업 스케줄 설정 구조: 백업 스케줄의 설정
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 실제 스케줄링 실행 로직 제외
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSchedule {
  /// 백업 간격 (초, 구조 정의만, 백업 수행 간격)
  pub interval_seconds: u64,
  /// 최대 백업 개수 (오래된 백업 자동 삭제, 구조 정의만, None이면 무제한)
  pub max_backups: Option<usize>,
  /// 백업 유지 기간 (초, None이면 무제한, 구조 정의만, 백업 보관 기간)
  pub retention_seconds: Option<u64>,
  /// 자동 백업 활성화 여부 (구조 정의만, 자동 백업 사용 여부)
  pub enabled: bool,
}

impl Default for BackupSchedule {
  fn default() -> Self {
    Self {
      interval_seconds: 3600, // 1시간
      max_backups: Some(10),
      retention_seconds: Some(7 * 24 * 3600), // 7일
      enabled: false,
    }
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 구조체/메서드들은 executor/runtime 계층에서 구현하세요:
// - BackupManager 구조체 (파일 I/O, 상태 관리)
// - backup_file(), restore(), verify_checksum() 등 (파일 I/O)
// - init(), cleanup_old_backups() 등 (파일 시스템 작업)
//
// 이 함수들은 파일 I/O 및 상태 변경을 수행하므로 pnix-core에서 제외됩니다.

/// 버전 정보가 포함된 설정: 버전 정보가 포함된 설정 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedConfig {
  /// 설정 버전 (설정 파일 버전 문자열)
  #[serde(default)]
  pub version: String,
  /// 실제 설정 데이터 (설정 필드 → 값 매핑)
  #[serde(flatten)]
  pub config: HashMap<String, serde_json::Value>,
}

impl Default for VersionedConfig {
  fn default() -> Self {
    Self {
      version: "1.1".to_string(),
      config: HashMap::new(),
    }
  }
}

/// 마이그레이션 필드 변경 정보: 마이그레이션 중 변경된 필드 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigratedField {
  /// 필드 경로 (예: "repl.mode_toggle_key", 점으로 구분된 경로)
  pub path: String,
  /// 변경 타입 (필드 변경의 종류)
  pub change_type: FieldChangeType,
  /// 이전 값 (있는 경우, 변경 전 값)
  pub old_value: Option<String>,
  /// 새 값 (변경 후 값)
  pub new_value: String,
}

/// 필드 변경 타입: 설정 필드 변경의 종류
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldChangeType {
  /// 새로 추가됨 (필드가 새로 추가됨)
  Added,
  /// 값이 변경됨 (필드 값이 변경됨)
  Modified,
  /// 이름이 변경됨 (필드 이름이 변경됨)
  Renamed,
  /// 제거됨 (필드가 제거됨)
  Removed,
}

/// 마이그레이션 리포트: 설정 마이그레이션의 결과 리포트
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationReport {
  /// 원본 버전 (마이그레이션 전 버전)
  pub from_version: ConfigVersion,
  /// 대상 버전 (마이그레이션 후 버전)
  pub to_version: ConfigVersion,
  /// 마이그레이션된 필드 목록 (변경된 필드 목록)
  pub migrated_fields: Vec<MigratedField>,
  /// 경고 메시지 (마이그레이션 중 발생한 경고 목록)
  pub warnings: Vec<String>,
  /// 성공 여부 (마이그레이션 성공 여부)
  pub success: bool,
}

impl MigrationReport {
  /// 새 리포트 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(from: ConfigVersion, to: ConfigVersion) -> Self {
    Self {
      from_version: from,
      to_version: to,
      migrated_fields: Vec::new(),
      warnings: Vec::new(),
      success: true,
    }
  }

  /// 마이그레이션된 필드 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn add_field(&mut self, field: MigratedField) {
    self.migrated_fields.push(field);
  }

  /// 경고 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn add_warning(&mut self, warning: impl Into<String>) {
    self.warnings.push(warning.into());
  }

  /// 실패로 표시
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn mark_failed(&mut self) {
    self.success = false;
  }

  /// 리포트 포맷
  ///
  /// ## 헌법 준수 (P0-1, C1)
  ///
  /// 텍스트 생성만, 파일 I/O 없음
  pub fn format(&self) -> String {
    let mut report = String::new();
    report.push_str(&format!(
      "Migration Report: {} -> {}\n",
      self.from_version.as_str(),
      self.to_version.as_str()
    ));
    report.push_str(&format!(
      "Status: {}\n",
      if self.success { "Success" } else { "Failed" }
    ));

    if !self.migrated_fields.is_empty() {
      report.push_str("\nMigrated Fields:\n");
      for field in &self.migrated_fields {
        let change = match field.change_type {
          FieldChangeType::Added => "added",
          FieldChangeType::Modified => "modified",
          FieldChangeType::Renamed => "renamed",
          FieldChangeType::Removed => "removed",
        };
        report.push_str(&format!(
          "  - {} ({}): {}\n",
          field.path, change, field.new_value
        ));
      }
    }

    if !self.warnings.is_empty() {
      report.push_str("\nWarnings:\n");
      for warning in &self.warnings {
        report.push_str(&format!("  - {}\n", warning));
      }
    }

    report
  }
}

/// V1.0에서 V1.1로 마이그레이션 시 추가되는 기본 필드들
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 생성만, 값 계산 없음
pub fn v1_0_to_v1_1_defaults() -> Vec<MigratedField> {
  vec![
    MigratedField {
      path: "repl.mode_toggle_key".to_string(),
      change_type: FieldChangeType::Added,
      old_value: None,
      new_value: "F2".to_string(),
    },
    MigratedField {
      path: "symbolic.default_variable".to_string(),
      change_type: FieldChangeType::Added,
      old_value: None,
      new_value: "x".to_string(),
    },
  ]
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_config_version_parse() {
    assert_eq!(ConfigVersion::parse("1.0"), Some(ConfigVersion::V1_0));
    assert_eq!(ConfigVersion::parse("1.1"), Some(ConfigVersion::V1_1));
    assert_eq!(ConfigVersion::parse("latest"), Some(ConfigVersion::Latest));
    assert_eq!(ConfigVersion::parse("2.0"), None);
  }

  #[test]
  fn test_config_version_ordering() {
    assert!(ConfigVersion::V1_0 < ConfigVersion::V1_1);
    assert!(ConfigVersion::V1_1 <= ConfigVersion::Latest);
  }

  #[test]
  fn test_needs_migration() {
    assert!(ConfigVersion::V1_0.needs_migration());
    assert!(!ConfigVersion::V1_1.needs_migration());
    assert!(!ConfigVersion::Latest.needs_migration());
  }

  #[test]
  fn test_migration_report_format() {
    let mut report = MigrationReport::new(ConfigVersion::V1_0, ConfigVersion::V1_1);
    report.add_field(MigratedField {
      path: "repl.mode_toggle_key".to_string(),
      change_type: FieldChangeType::Added,
      old_value: None,
      new_value: "F2".to_string(),
    });

    let formatted = report.format();
    assert!(formatted.contains("1.0"));
    assert!(formatted.contains("1.1"));
    assert!(formatted.contains("mode_toggle_key"));
  }

  #[test]
  fn test_v1_0_to_v1_1_defaults() {
    let defaults = v1_0_to_v1_1_defaults();
    assert!(!defaults.is_empty());
    assert!(defaults.iter().any(|f| f.path.contains("mode_toggle_key")));
  }
}
