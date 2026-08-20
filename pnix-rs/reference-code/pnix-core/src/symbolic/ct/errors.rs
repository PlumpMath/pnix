//! CT 검증 오류
//!
//! ## 헌법 준수 (P0-1)
//!
//! 에러 타입 정의만, 실행 없음

use std::error::Error;
use std::fmt;

/// CT 검증 오류
#[derive(Debug, Clone, PartialEq)]
pub enum CtError {
  /// 도메인 불일치
  DomainMismatch { expected: String, found: String },

  /// 코도메인 불일치
  CodomainMismatch { expected: String, found: String },

  /// 단위 불일치 (Add에서)
  UnitMismatch {
    operation: String,
    left: String,
    right: String,
  },

  /// 카테고리 불일치
  CategoryMismatch { left: String, right: String },

  /// 인덱스 공간 불일치 (수축에서)
  IndexSpaceMismatch {
    index: String,
    expected: String,
    found: String,
  },

  /// 잘못된 수축 (2번이 아닌 경우)
  InvalidContraction { index: String, count: usize },

  /// 수축 position 오류 (Up+Down이 아닌 경우)
  ContractionPositionError { index: String },

  /// 고아 인덱스 (explicit contract에 대응되는 짝이 없음)
  OrphanedIndex { index: String },

  /// 자유 인덱스 불일치 (Add에서)
  FreeIndexMismatch {
    left: Vec<String>,
    right: Vec<String>,
  },
}

impl fmt::Display for CtError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      CtError::DomainMismatch { expected, found } => {
        write!(f, "Domain mismatch: expected {}, found {}", expected, found)
      }
      CtError::CodomainMismatch { expected, found } => {
        write!(
          f,
          "Codomain mismatch: expected {}, found {}",
          expected, found
        )
      }
      CtError::UnitMismatch {
        operation,
        left,
        right,
      } => {
        write!(f, "Unit mismatch in {}: {} vs {}", operation, left, right)
      }
      CtError::CategoryMismatch { left, right } => {
        write!(
          f,
          "Category mismatch: cannot combine {} with {}",
          left, right
        )
      }
      CtError::IndexSpaceMismatch {
        index,
        expected,
        found,
      } => {
        write!(
          f,
          "Index space mismatch: {} is in {} but used in {}",
          index, expected, found
        )
      }
      CtError::InvalidContraction { index, count } => {
        write!(
          f,
          "Invalid contraction: index {} appears {} times (expected 2)",
          index, count
        )
      }
      CtError::ContractionPositionError { index } => {
        write!(
          f,
          "Contraction position error: index {} must have one Upper and one Lower",
          index
        )
      }
      CtError::OrphanedIndex { index } => {
        write!(f, "Orphaned index in contract: {}", index)
      }
      CtError::FreeIndexMismatch { left, right } => {
        write!(f, "Free index mismatch in Add: {:?} vs {:?}", left, right)
      }
    }
  }
}

impl Error for CtError {}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_display() {
    let err = CtError::InvalidContraction {
      index: "μ".into(),
      count: 3,
    };
    let msg = format!("{}", err);
    assert!(msg.contains("μ"));
    assert!(msg.contains("3"));
  }

  #[test]
  fn test_index_space_mismatch() {
    let err = CtError::IndexSpaceMismatch {
      index: "μ".into(),
      expected: "spacetime".into(),
      found: "momentum".into(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("spacetime"));
    assert!(msg.contains("momentum"));
  }
}
