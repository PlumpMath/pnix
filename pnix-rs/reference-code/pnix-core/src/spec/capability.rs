//! Capability catalog (data only)
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Capability 종류
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CapabilityKind {
  /// 순수 계산 (부작용 없음)
  Pure,
  /// 세계 효과 (IO/시간/상태)
  World,
  /// IO 연산
  Io,
  /// 네트워크 연산
  Network,
  /// 파일 시스템 연산
  FileSystem,
  /// 수학 연산
  Math,
  /// 산술 연산
  Arithmetic,
  /// 삼각 함수
  Trigonometry,
  /// 비교 연산
  Comparison,
  /// 논리 연산
  Logic,
}

/// Capability 선언
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityDecl {
  /// Capability 이름
  pub name: String,
  /// Capability 종류
  pub kind: CapabilityKind,
  /// 설명
  pub description: String,
  /// 부모 capability (상속 관계)
  pub parent: Option<String>,
}

/// Capability 카탈로그
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityCatalog {
  /// 등록된 capabilities (이름 → 선언)
  pub capabilities: BTreeMap<String, CapabilityDecl>,
}

impl CapabilityCatalog {
  /// 빈 카탈로그 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self {
      capabilities: BTreeMap::new(),
    }
  }

  /// 기본 capability 포함 카탈로그 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn with_defaults() -> Self {
    let mut catalog = Self::new();

    catalog.register(CapabilityDecl {
      name: "Pure".to_string(),
      kind: CapabilityKind::Pure,
      description: "Pure computation (no side effects)".to_string(),
      parent: None,
    });
    catalog.register(CapabilityDecl {
      name: "World".to_string(),
      kind: CapabilityKind::World,
      description: "World effect (IO/time/state)".to_string(),
      parent: None,
    });
    catalog.register(CapabilityDecl {
      name: "Io".to_string(),
      kind: CapabilityKind::Io,
      description: "IO operations".to_string(),
      parent: Some("World".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "Network".to_string(),
      kind: CapabilityKind::Network,
      description: "Network operations".to_string(),
      parent: Some("World".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "RuntimeCall".to_string(),
      kind: CapabilityKind::Network,
      description: "Cross-runtime RPC / interop call boundary".to_string(),
      parent: Some("Network".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "FileSystem".to_string(),
      kind: CapabilityKind::FileSystem,
      description: "File system operations".to_string(),
      parent: Some("Io".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "Math".to_string(),
      kind: CapabilityKind::Math,
      description: "Mathematical operations".to_string(),
      parent: Some("Pure".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "Arithmetic".to_string(),
      kind: CapabilityKind::Arithmetic,
      description: "Arithmetic operations".to_string(),
      parent: Some("Math".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "Trigonometry".to_string(),
      kind: CapabilityKind::Trigonometry,
      description: "Trigonometric functions".to_string(),
      parent: Some("Math".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "Comparison".to_string(),
      kind: CapabilityKind::Comparison,
      description: "Comparison operations".to_string(),
      parent: Some("Pure".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "Logic".to_string(),
      kind: CapabilityKind::Logic,
      description: "Logical operations".to_string(),
      parent: Some("Pure".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "Schema".to_string(),
      kind: CapabilityKind::Pure,
      description: "Schema validation and normalization".to_string(),
      parent: Some("Pure".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "Xml".to_string(),
      kind: CapabilityKind::Pure,
      description: "XML parsing and emission".to_string(),
      parent: Some("Pure".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "X3d".to_string(),
      kind: CapabilityKind::Pure,
      description: "X3D XML parsing, schema, and FRP helpers".to_string(),
      parent: Some("Xml".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "MathML".to_string(),
      kind: CapabilityKind::Pure,
      description: "MathML XML parsing and generation".to_string(),
      parent: Some("Xml".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "OpenMath".to_string(),
      kind: CapabilityKind::Pure,
      description: "OpenMath XML parsing and generation".to_string(),
      parent: Some("Xml".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "Svg".to_string(),
      kind: CapabilityKind::Pure,
      description: "SVG XML parsing and emission".to_string(),
      parent: Some("Xml".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "Ifcxml".to_string(),
      kind: CapabilityKind::Pure,
      description: "IFCXML parsing and schema helpers".to_string(),
      parent: Some("Xml".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "SBML".to_string(),
      kind: CapabilityKind::Pure,
      description: "SBML XML parsing and schema helpers".to_string(),
      parent: Some("Xml".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "CellML".to_string(),
      kind: CapabilityKind::Pure,
      description: "CellML XML parsing and schema helpers".to_string(),
      parent: Some("Xml".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "NeuroML".to_string(),
      kind: CapabilityKind::Pure,
      description: "NeuroML XML parsing and schema helpers".to_string(),
      parent: Some("Xml".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "LEMS".to_string(),
      kind: CapabilityKind::Pure,
      description: "LEMS XML parsing and schema helpers".to_string(),
      parent: Some("Xml".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "SED-ML".to_string(),
      kind: CapabilityKind::Pure,
      description: "SED-ML XML parsing and schema helpers".to_string(),
      parent: Some("Xml".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "OMEX".to_string(),
      kind: CapabilityKind::Pure,
      description: "OMEX archive schema helpers".to_string(),
      parent: Some("Xml".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "PharmML".to_string(),
      kind: CapabilityKind::Pure,
      description: "PharmML XML parsing and schema helpers".to_string(),
      parent: Some("Xml".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "CML".to_string(),
      kind: CapabilityKind::Pure,
      description: "CML XML parsing and schema helpers".to_string(),
      parent: Some("Xml".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "PDBML".to_string(),
      kind: CapabilityKind::Pure,
      description: "PDBML XML parsing and schema helpers".to_string(),
      parent: Some("Xml".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "SBGN-ML".to_string(),
      kind: CapabilityKind::Pure,
      description: "SBGN-ML XML parsing and schema helpers".to_string(),
      parent: Some("Xml".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "BioPAX".to_string(),
      kind: CapabilityKind::Pure,
      description: "BioPAX XML parsing and schema helpers".to_string(),
      parent: Some("Xml".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "VTK".to_string(),
      kind: CapabilityKind::Pure,
      description: "VTK XML parsing and schema helpers".to_string(),
      parent: Some("Xml".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "XDMF".to_string(),
      kind: CapabilityKind::Pure,
      description: "XDMF XML parsing and schema helpers".to_string(),
      parent: Some("Xml".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "GIFTI".to_string(),
      kind: CapabilityKind::Pure,
      description: "GIFTI XML parsing and schema helpers".to_string(),
      parent: Some("Xml".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "Frp".to_string(),
      kind: CapabilityKind::Pure,
      description: "Functional reactive graph helpers".to_string(),
      parent: Some("Pure".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "Html".to_string(),
      kind: CapabilityKind::Pure,
      description: "HTML parsing and emission".to_string(),
      parent: Some("Pure".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "Patch".to_string(),
      kind: CapabilityKind::Pure,
      description: "Structural patch planning and packet diff helpers".to_string(),
      parent: Some("Pure".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "Sync".to_string(),
      kind: CapabilityKind::Pure,
      description: "Stable sync-id planning and reconciliation helpers".to_string(),
      parent: Some("Patch".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "X3dom".to_string(),
      kind: CapabilityKind::Pure,
      description: "X3DOM HTML lowering helpers".to_string(),
      parent: Some("Html".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "Webview".to_string(),
      kind: CapabilityKind::Pure,
      description: "Webview/document packet lowering helpers".to_string(),
      parent: Some("Html".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "HAnim".to_string(),
      kind: CapabilityKind::Pure,
      description: "Humanoid animation skeleton/state helpers".to_string(),
      parent: Some("X3d".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "Physics".to_string(),
      kind: CapabilityKind::Pure,
      description: "Physics summary and constraint helpers".to_string(),
      parent: Some("Pure".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "Symbolic".to_string(),
      kind: CapabilityKind::Pure,
      description: "Symbolic equation/constraint helpers".to_string(),
      parent: Some("Pure".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "Process".to_string(),
      kind: CapabilityKind::World,
      description: "Process execution".to_string(),
      parent: Some("World".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "ProcessSpawn".to_string(),
      kind: CapabilityKind::World,
      description: "Spawn/start external processes".to_string(),
      parent: Some("Process".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "ProcessSignal".to_string(),
      kind: CapabilityKind::World,
      description: "Send signals/terminate processes".to_string(),
      parent: Some("Process".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "ProcessObserve".to_string(),
      kind: CapabilityKind::World,
      description: "Observe process status/metrics/logs".to_string(),
      parent: Some("Process".to_string()),
    });

    // Ontology operations (convergence Phase 4)
    catalog.register(CapabilityDecl {
      name: "Ontology".to_string(),
      kind: CapabilityKind::Pure,
      description: "Ontology semantic operations".to_string(),
      parent: Some("Pure".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "Meaning".to_string(),
      kind: CapabilityKind::Pure,
      description: "Meaning lifecycle (lift/evaluate/select/promote)".to_string(),
      parent: Some("Ontology".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "Query".to_string(),
      kind: CapabilityKind::Pure,
      description: "Ontology fact query".to_string(),
      parent: Some("Ontology".to_string()),
    });
    catalog.register(CapabilityDecl {
      name: "Emit".to_string(),
      kind: CapabilityKind::World,
      description: "Emit new ontology facts (store write)".to_string(),
      parent: Some("World".to_string()),
    });

    catalog
  }

  /// Capability 등록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn register(&mut self, decl: CapabilityDecl) {
    self.capabilities.insert(decl.name.clone(), decl);
  }

  /// Capability 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn get(&self, name: &str) -> Option<&CapabilityDecl> {
    self.capabilities.get(name)
  }

  /// Capability 존재 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn contains(&self, name: &str) -> bool {
    self.capabilities.contains_key(name)
  }

  /// Capability 상속 체크 (name이 parent의 자식인지)
  ///
  /// CRITICAL: 사이클 감지 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn inherits_from(&self, name: &str, parent: &str) -> bool {
    let mut current = name;
    let mut visited = std::collections::HashSet::new();

    while let Some(decl) = self.get(current) {
      // 사이클 감지: 이미 방문한 노드를 다시 방문하면 사이클
      if !visited.insert(current) {
        // 사이클 감지됨 - 무한 루프 방지
        return false;
      }

      if current == parent {
        return true;
      }
      if let Some(ref p) = decl.parent {
        current = p;
      } else {
        break;
      }
    }
    false
  }
}

impl Default for CapabilityCatalog {
  fn default() -> Self {
    Self::with_defaults()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_capability_catalog_creation() {
    let catalog = CapabilityCatalog::new();
    assert!(catalog.capabilities.is_empty());
  }

  #[test]
  fn test_capability_catalog_with_defaults() {
    let catalog = CapabilityCatalog::with_defaults();
    assert!(catalog.contains("Pure"));
    assert!(catalog.contains("Math"));
    assert!(catalog.contains("Arithmetic"));
    assert!(catalog.contains("Process"));
    assert!(catalog.contains("ProcessSpawn"));
    assert!(catalog.contains("ProcessSignal"));
    assert!(catalog.contains("ProcessObserve"));
    assert!(catalog.contains("Patch"));
    assert!(catalog.contains("Sync"));
    assert!(catalog.contains("X3dom"));
    assert!(catalog.contains("Webview"));
    assert!(catalog.contains("HAnim"));
    assert!(catalog.contains("Physics"));
    assert!(catalog.contains("Symbolic"));
  }

  #[test]
  fn test_capability_inheritance() {
    let catalog = CapabilityCatalog::with_defaults();
    assert!(catalog.inherits_from("Arithmetic", "Math"));
    assert!(catalog.inherits_from("Arithmetic", "Pure"));
    assert!(catalog.inherits_from("FileSystem", "Io"));
    assert!(catalog.inherits_from("FileSystem", "World"));
    assert!(catalog.inherits_from("ProcessSpawn", "Process"));
    assert!(catalog.inherits_from("ProcessSpawn", "World"));
    assert!(catalog.inherits_from("ProcessSignal", "Process"));
    assert!(catalog.inherits_from("ProcessObserve", "Process"));
    assert!(catalog.inherits_from("Sync", "Patch"));
    assert!(catalog.inherits_from("Patch", "Pure"));
    assert!(catalog.inherits_from("X3dom", "Html"));
    assert!(catalog.inherits_from("Webview", "Html"));
    assert!(catalog.inherits_from("HAnim", "X3d"));
  }
}
