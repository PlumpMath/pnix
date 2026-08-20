//! Symbol Index 구조 정의
//!
//! pnix-old의 pnix_symbol_index/src/lib.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 인덱싱 실행 로직(파일 해시 계산) 제외
//!
//! ## 참고
//!
//! 실제 인덱싱 실행 로직은 executor에서 구현합니다.
//! 이 모듈은 구조 정의만 포함합니다.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 심볼 정의 위치
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SymbolDefinition {
  /// 심볼 이름
  pub name: String,
  /// 정의된 파일 경로
  pub file_path: String,
  /// 정의 라인 (0-based)
  pub line: usize,
  /// 정의 컬럼 (0-based)
  pub column: usize,
  /// 심볼 타입 (문자열로 직렬화, executor에서 SymbolType enum으로 변환)
  pub symbol_type: String,
  /// 문서 주석
  pub doc: Option<String>,
  /// 정의된 블록 정보
  pub block_language: String,
}

/// 심볼 참조 위치
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SymbolReference {
  /// 심볼 이름
  pub name: String,
  /// 참조된 파일 경로
  pub file_path: String,
  /// 참조 라인 (0-based)
  pub line: usize,
  /// 참조 컬럼 (0-based)
  pub column: usize,
  /// 참조 컨텍스트 (선택적)
  pub context: Option<String>,
}

/// 심볼 인덱스 (파일 단위)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolIndex {
  /// 파일 경로
  pub file_path: String,
  /// 정의된 심볼들
  pub definitions: HashMap<String, SymbolDefinition>,
  /// 심볼 참조들
  pub references: Vec<SymbolReference>,
  /// 블록 컨텍스트들 (실제 분석은 executor에서)
  /// JSON으로 직렬화 가능한 구조 (executor에서 BlockContext로 변환)
  pub block_contexts: Vec<serde_json::Value>,
}

/// 최대 파일 수 제한 (DoS 방지)
const MAX_FILES: usize = 10_000;

/// 최대 심볼 수 제한 (DoS 방지)
const MAX_SYMBOLS: usize = 100_000;

/// 전역 심볼 인덱스 (워크스페이스 전체)
///
/// ## 크기 제한 (DoS 방지)
///
/// - indices: 최대 MAX_FILES (10,000) 개 파일
/// - symbol_to_definitions: 최대 MAX_SYMBOLS (100,000) 개 심볼
/// - file_hashes: 최대 MAX_FILES (10,000) 개 파일
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalSymbolIndex {
  /// 파일별 인덱스
  pub indices: HashMap<String, SymbolIndex>,
  /// 심볼 이름 → 정의 위치 매핑 (전역)
  pub symbol_to_definitions: HashMap<String, Vec<SymbolDefinition>>,
  /// 파일별 소스 해시 (증분 업데이트용, 실제 계산은 executor에서)
  pub file_hashes: HashMap<String, u64>,
}

impl GlobalSymbolIndex {
  /// 새 전역 인덱스 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self {
      indices: HashMap::new(),
      symbol_to_definitions: HashMap::new(),
      file_hashes: HashMap::new(),
    }
  }

  /// 파일 인덱스 추가 (크기 제한 검증)
  ///
  /// 파일 수가 MAX_FILES를 초과하면 에러를 반환합니다.
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn add_file_index(&mut self, file_path: String, index: SymbolIndex) -> Result<(), String> {
    if self.indices.len() >= MAX_FILES {
      return Err(format!(
        "File limit exceeded: cannot index more than {} files (DoS protection)",
        MAX_FILES
      ));
    }
    self.indices.insert(file_path, index);
    Ok(())
  }

  /// 심볼 정의 추가 (크기 제한 검증)
  ///
  /// 심볼 수가 MAX_SYMBOLS를 초과하면 에러를 반환합니다.
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn add_symbol_definition(
    &mut self,
    symbol: String,
    definition: SymbolDefinition,
  ) -> Result<(), String> {
    if self.symbol_to_definitions.len() >= MAX_SYMBOLS {
      return Err(format!(
        "Symbol limit exceeded: cannot index more than {} symbols (DoS protection)",
        MAX_SYMBOLS
      ));
    }
    self
      .symbol_to_definitions
      .entry(symbol)
      .or_default()
      .push(definition);
    Ok(())
  }

  /// 파일 해시 추가 (크기 제한 검증)
  ///
  /// 파일 수가 MAX_FILES를 초과하면 에러를 반환합니다.
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn add_file_hash(&mut self, file_path: String, hash: u64) -> Result<(), String> {
    if self.file_hashes.len() >= MAX_FILES {
      return Err(format!(
        "File limit exceeded: cannot index more than {} files (DoS protection)",
        MAX_FILES
      ));
    }
    self.file_hashes.insert(file_path, hash);
    Ok(())
  }

  /// 현재 파일 수 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn file_count(&self) -> usize {
    self.indices.len()
  }

  /// 현재 심볼 수 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn symbol_count(&self) -> usize {
    self.symbol_to_definitions.len()
  }
}

impl Default for GlobalSymbolIndex {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_global_symbol_index_creation() {
    let index = GlobalSymbolIndex::new();
    assert_eq!(index.indices.len(), 0);
    assert_eq!(index.symbol_to_definitions.len(), 0);
  }

  #[test]
  fn test_symbol_definition() {
    let def = SymbolDefinition {
      name: "test".to_string(),
      file_path: "test.rs".to_string(),
      line: 10,
      column: 5,
      symbol_type: "Function".to_string(),
      doc: Some("test function".to_string()),
      block_language: "rust".to_string(),
    };
    assert_eq!(def.name, "test");
    assert_eq!(def.line, 10);
  }

  #[test]
  fn test_symbol_reference() {
    let ref_ = SymbolReference {
      name: "test".to_string(),
      file_path: "test.rs".to_string(),
      line: 20,
      column: 3,
      context: Some("test()".to_string()),
    };
    assert_eq!(ref_.name, "test");
    assert_eq!(ref_.line, 20);
  }

  #[test]
  fn test_file_limit() {
    let mut index = GlobalSymbolIndex::new();

    // MAX_FILES까지 추가 가능
    for i in 0..MAX_FILES {
      let file_path = format!("file_{}.rs", i);
      let symbol_index = SymbolIndex {
        file_path: file_path.clone(),
        definitions: HashMap::new(),
        references: Vec::new(),
        block_contexts: Vec::new(),
      };
      assert!(index.add_file_index(file_path, symbol_index).is_ok());
    }
    assert_eq!(index.file_count(), MAX_FILES);

    // MAX_FILES 초과 시 에러
    let result = index.add_file_index(
      "overflow.rs".to_string(),
      SymbolIndex {
        file_path: "overflow.rs".to_string(),
        definitions: HashMap::new(),
        references: Vec::new(),
        block_contexts: Vec::new(),
      },
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("File limit exceeded"));
  }

  #[test]
  fn test_symbol_limit() {
    let mut index = GlobalSymbolIndex::new();

    // MAX_SYMBOLS까지 추가 가능
    for i in 0..MAX_SYMBOLS {
      let symbol = format!("symbol_{}", i);
      let definition = SymbolDefinition {
        name: symbol.clone(),
        file_path: "test.rs".to_string(),
        line: i,
        column: 0,
        symbol_type: "Function".to_string(),
        doc: None,
        block_language: "rust".to_string(),
      };
      assert!(index.add_symbol_definition(symbol, definition).is_ok());
    }
    assert_eq!(index.symbol_count(), MAX_SYMBOLS);

    // MAX_SYMBOLS 초과 시 에러
    let result = index.add_symbol_definition(
      "overflow".to_string(),
      SymbolDefinition {
        name: "overflow".to_string(),
        file_path: "test.rs".to_string(),
        line: 0,
        column: 0,
        symbol_type: "Function".to_string(),
        doc: None,
        block_language: "rust".to_string(),
      },
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Symbol limit exceeded"));
  }

  #[test]
  fn test_file_hash_limit() {
    let mut index = GlobalSymbolIndex::new();

    // MAX_FILES까지 추가 가능
    for i in 0..MAX_FILES {
      let file_path = format!("file_{}.rs", i);
      assert!(index.add_file_hash(file_path, i as u64).is_ok());
    }
    assert_eq!(index.file_hashes.len(), MAX_FILES);

    // MAX_FILES 초과 시 에러
    let result = index.add_file_hash("overflow.rs".to_string(), 0);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("File limit exceeded"));
  }
}
