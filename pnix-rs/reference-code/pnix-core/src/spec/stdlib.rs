//! Standard library catalog (data only)
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 표준 라이브러리 타입 선언
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TypeDecl {
  /// 타입 이름
  pub name: String,
  /// 타입 설명
  pub description: String,
  /// 타입 카테고리 (예: "Primitive", "Container", "Numeric")
  pub category: String,
}

/// 표준 라이브러리 함수 선언
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StdlibFunctionDecl {
  /// 함수 이름
  pub name: String,
  /// 함수 시그니처
  pub signature: String,
  /// 설명
  pub description: String,
  /// 모듈 경로 (예: "List.map", "String.concat")
  pub module_path: Option<String>,
}

/// 표준 라이브러리 카탈로그
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StdlibCatalog {
  /// 등록된 타입들 (이름 → 선언)
  pub types: BTreeMap<String, TypeDecl>,
  /// 등록된 함수들 (이름 → 선언)
  pub functions: BTreeMap<String, StdlibFunctionDecl>,
}

impl StdlibCatalog {
  /// 빈 카탈로그 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self {
      types: BTreeMap::new(),
      functions: BTreeMap::new(),
    }
  }

  /// 기본 stdlib 포함 카탈로그 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn with_defaults() -> Self {
    let mut catalog = Self::new();

    // Primitive types
    catalog.register_type(TypeDecl {
      name: "Num".to_string(),
      description: "Numeric type (Int or Float)".to_string(),
      category: "Primitive".to_string(),
    });
    catalog.register_type(TypeDecl {
      name: "Int".to_string(),
      description: "Integer type".to_string(),
      category: "Primitive".to_string(),
    });
    catalog.register_type(TypeDecl {
      name: "Float".to_string(),
      description: "Floating point type".to_string(),
      category: "Primitive".to_string(),
    });
    catalog.register_type(TypeDecl {
      name: "Rat".to_string(),
      description: "Rational number type (normalized fraction)".to_string(),
      category: "Numeric".to_string(),
    });
    catalog.register_type(TypeDecl {
      name: "Bool".to_string(),
      description: "Boolean type".to_string(),
      category: "Primitive".to_string(),
    });
    catalog.register_type(TypeDecl {
      name: "String".to_string(),
      description: "String type".to_string(),
      category: "Primitive".to_string(),
    });
    catalog.register_type(TypeDecl {
      name: "List".to_string(),
      description: "List type".to_string(),
      category: "Container".to_string(),
    });
    catalog.register_type(TypeDecl {
      name: "AttrSet".to_string(),
      description: "Attribute set (map) type".to_string(),
      category: "Container".to_string(),
    });

    // Test/common types (used in tests and examples)
    catalog.register_type(TypeDecl {
      name: "Vector".to_string(),
      description: "Vector type (for testing/examples)".to_string(),
      category: "Container".to_string(),
    });
    catalog.register_type(TypeDecl {
      name: "Matrix".to_string(),
      description: "Matrix type (for testing/examples)".to_string(),
      category: "Container".to_string(),
    });
    catalog.register_type(TypeDecl {
      name: "Html".to_string(),
      description: "HTML type (for testing/examples)".to_string(),
      category: "Primitive".to_string(),
    });
    catalog.register_type(TypeDecl {
      name: "UiSpec".to_string(),
      description: "UI specification type (for testing/examples)".to_string(),
      category: "Primitive".to_string(),
    });
    catalog.register_type(TypeDecl {
      name: "XmlAst".to_string(),
      description: "XML AST (attrset/list tree)".to_string(),
      category: "Document".to_string(),
    });
    catalog.register_type(TypeDecl {
      name: "HtmlAst".to_string(),
      description: "HTML AST (DOM-like tree)".to_string(),
      category: "Document".to_string(),
    });
    catalog.register_type(TypeDecl {
      name: "SvgAst".to_string(),
      description: "SVG AST (XML-based tree)".to_string(),
      category: "Document".to_string(),
    });
    catalog.register_type(TypeDecl {
      name: "MathmlAst".to_string(),
      description: "MathML AST (XML-based tree)".to_string(),
      category: "Document".to_string(),
    });
    catalog.register_type(TypeDecl {
      name: "X3dAst".to_string(),
      description: "X3D AST (XML-based tree)".to_string(),
      category: "Document".to_string(),
    });
    catalog.register_type(TypeDecl {
      name: "X3domAst".to_string(),
      description: "X3DOM AST (HTML-based tree)".to_string(),
      category: "Document".to_string(),
    });
    catalog.register_type(TypeDecl {
      name: "ProcessSpec".to_string(),
      description: "Process specification (argv/env/cwd/policy)".to_string(),
      category: "Interop".to_string(),
    });
    catalog.register_type(TypeDecl {
      name: "ProcessHandle".to_string(),
      description: "Opaque process handle (supervisor token + pid snapshot)".to_string(),
      category: "Interop".to_string(),
    });
    catalog.register_type(TypeDecl {
      name: "ProcessStatus".to_string(),
      description: "Observed process status (running/exit/resource snapshot)".to_string(),
      category: "Interop".to_string(),
    });
    catalog.register_type(TypeDecl {
      name: "ProcessExit".to_string(),
      description: "Process exit info (code/signal/when)".to_string(),
      category: "Interop".to_string(),
    });
    catalog.register_type(TypeDecl {
      name: "ProcessObservation".to_string(),
      description: "Process observation sample (cpu/mem/io/threads/fds)".to_string(),
      category: "Interop".to_string(),
    });

    // Rat stdlib (math.rat)
    catalog.register_function(StdlibFunctionDecl {
      name: "Rat.make".to_string(),
      signature: "Int → Int → Rat".to_string(),
      description: "Construct a rational number and normalize".to_string(),
      module_path: Some("Rat.make".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "Rat.normalize".to_string(),
      signature: "Rat → Rat".to_string(),
      description: "Normalize a rational number (gcd, positive denominator)".to_string(),
      module_path: Some("Rat.normalize".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "Rat.add".to_string(),
      signature: "Rat → Rat → Rat".to_string(),
      description: "Add two rational numbers".to_string(),
      module_path: Some("Rat.add".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "Rat.sub".to_string(),
      signature: "Rat → Rat → Rat".to_string(),
      description: "Subtract two rational numbers".to_string(),
      module_path: Some("Rat.sub".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "Rat.mul".to_string(),
      signature: "Rat → Rat → Rat".to_string(),
      description: "Multiply two rational numbers".to_string(),
      module_path: Some("Rat.mul".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "Rat.div".to_string(),
      signature: "Rat → Rat → Rat".to_string(),
      description: "Divide two rational numbers".to_string(),
      module_path: Some("Rat.div".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "Rat.eq".to_string(),
      signature: "Rat → Rat → Bool".to_string(),
      description: "Equality for rational numbers".to_string(),
      module_path: Some("Rat.eq".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "Rat.num".to_string(),
      signature: "Rat → Int".to_string(),
      description: "Get numerator".to_string(),
      module_path: Some("Rat.num".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "Rat.den".to_string(),
      signature: "Rat → Int".to_string(),
      description: "Get denominator".to_string(),
      module_path: Some("Rat.den".to_string()),
    });

    // String stdlib (Y01a)
    catalog.register_function(StdlibFunctionDecl {
      name: "String.concat".to_string(),
      signature: "String → String → String".to_string(),
      description: "Concatenate two strings".to_string(),
      module_path: Some("String.concat".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "String.slice".to_string(),
      signature: "Int → Int → String → String".to_string(),
      description: "Slice a string by start and length".to_string(),
      module_path: Some("String.slice".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "String.length".to_string(),
      signature: "String → Int".to_string(),
      description: "Get string length".to_string(),
      module_path: Some("String.length".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "String.split".to_string(),
      signature: "String → String → [String]".to_string(),
      description: "Split a string by delimiter".to_string(),
      module_path: Some("String.split".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "String.join".to_string(),
      signature: "String → [String] → String".to_string(),
      description: "Join strings with delimiter".to_string(),
      module_path: Some("String.join".to_string()),
    });

    // XML stdlib
    catalog.register_function(StdlibFunctionDecl {
      name: "Xml.parse".to_string(),
      signature: "String → XmlAst".to_string(),
      description: "Parse XML string to XmlAst".to_string(),
      module_path: Some("Xml.parse".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "Xml.emit".to_string(),
      signature: "XmlAst → String".to_string(),
      description: "Emit XmlAst as XML string".to_string(),
      module_path: Some("Xml.emit".to_string()),
    });
    // HTML stdlib
    catalog.register_function(StdlibFunctionDecl {
      name: "Html.parse".to_string(),
      signature: "String → HtmlAst".to_string(),
      description: "Parse HTML string to HtmlAst".to_string(),
      module_path: Some("Html.parse".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "Html.emit".to_string(),
      signature: "HtmlAst → String".to_string(),
      description: "Emit HtmlAst as HTML string".to_string(),
      module_path: Some("Html.emit".to_string()),
    });

    // Process stdlib
    catalog.register_function(StdlibFunctionDecl {
      name: "Process.spawn".to_string(),
      signature: "ProcessSpec → ProcessHandle".to_string(),
      description: "Spawn a process under supervisor control".to_string(),
      module_path: Some("Process.spawn".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "Process.ensure".to_string(),
      signature: "ProcessSpec → ProcessHandle".to_string(),
      description: "Ensure a process exists (idempotent reconcile)".to_string(),
      module_path: Some("Process.ensure".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "Process.status".to_string(),
      signature: "ProcessHandle → ProcessStatus".to_string(),
      description: "Get process status".to_string(),
      module_path: Some("Process.status".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "Process.signal".to_string(),
      signature: "ProcessHandle → String → Bool".to_string(),
      description: "Send signal to process".to_string(),
      module_path: Some("Process.signal".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "Process.wait".to_string(),
      signature: "ProcessHandle → Num → ProcessExit".to_string(),
      description: "Wait process exit with timeout_ms".to_string(),
      module_path: Some("Process.wait".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "Process.observeSample".to_string(),
      signature: "ProcessHandle → AttrSet → ProcessObservation".to_string(),
      description: "Sample process observation by handle".to_string(),
      module_path: Some("Process.observeSample".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "Process.observeSampleById".to_string(),
      signature: "String → AttrSet → ProcessObservation".to_string(),
      description: "Sample process observation by logical id".to_string(),
      module_path: Some("Process.observeSampleById".to_string()),
    });

    // List stdlib (Y01b)
    catalog.register_function(StdlibFunctionDecl {
      name: "List.map".to_string(),
      signature: "Any → List → List".to_string(),
      description: "Map function over list".to_string(),
      module_path: Some("List.map".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "List.filter".to_string(),
      signature: "Any → List → List".to_string(),
      description: "Filter list by predicate".to_string(),
      module_path: Some("List.filter".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "List.fold".to_string(),
      signature: "Any → Any → List → Any".to_string(),
      description: "Left fold over list".to_string(),
      module_path: Some("List.fold".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "List.find".to_string(),
      signature: "Any → List → Any".to_string(),
      description: "Find element in list".to_string(),
      module_path: Some("List.find".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "List.sort".to_string(),
      signature: "List → List".to_string(),
      description: "Sort list".to_string(),
      module_path: Some("List.sort".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "List.reverse".to_string(),
      signature: "List → List".to_string(),
      description: "Reverse list".to_string(),
      module_path: Some("List.reverse".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "List.take".to_string(),
      signature: "Int → List → List".to_string(),
      description: "Take first n elements".to_string(),
      module_path: Some("List.take".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "List.drop".to_string(),
      signature: "Int → List → List".to_string(),
      description: "Drop first n elements".to_string(),
      module_path: Some("List.drop".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "List.zip".to_string(),
      signature: "List → List → List".to_string(),
      description: "Zip two lists".to_string(),
      module_path: Some("List.zip".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "List.flatten".to_string(),
      signature: "List → List".to_string(),
      description: "Flatten list of lists".to_string(),
      module_path: Some("List.flatten".to_string()),
    });

    // AttrSet stdlib (Y01c)
    catalog.register_function(StdlibFunctionDecl {
      name: "AttrSet.get".to_string(),
      signature: "AttrSet → String → Any".to_string(),
      description: "Get value by key from attrset".to_string(),
      module_path: Some("AttrSet.get".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "AttrSet.set".to_string(),
      signature: "AttrSet → String → Any → AttrSet".to_string(),
      description: "Set value by key in attrset".to_string(),
      module_path: Some("AttrSet.set".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "AttrSet.keys".to_string(),
      signature: "AttrSet → List".to_string(),
      description: "Get attrset keys".to_string(),
      module_path: Some("AttrSet.keys".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "AttrSet.values".to_string(),
      signature: "AttrSet → List".to_string(),
      description: "Get attrset values".to_string(),
      module_path: Some("AttrSet.values".to_string()),
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "AttrSet.merge".to_string(),
      signature: "AttrSet → AttrSet → AttrSet".to_string(),
      description: "Merge two attrsets".to_string(),
      module_path: Some("AttrSet.merge".to_string()),
    });

    // Test assertion functions (Y11b)
    catalog.register_function(StdlibFunctionDecl {
      name: "assert".to_string(),
      signature: "Bool → Unit".to_string(),
      description: "Assert that condition is true. Throws error if false.".to_string(),
      module_path: None,
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "assertEqual".to_string(),
      signature: "Any → Any → Unit".to_string(),
      description:
        "Assert that two values are equal. Throws error with expected/found if not equal."
          .to_string(),
      module_path: None,
    });
    catalog.register_function(StdlibFunctionDecl {
      name: "assertThrows".to_string(),
      signature: "Any → Unit".to_string(),
      description: "Assert that function throws an error. Throws error if function does not throw."
        .to_string(),
      module_path: None,
    });

    catalog
  }

  /// 타입 등록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn register_type(&mut self, decl: TypeDecl) {
    self.types.insert(decl.name.clone(), decl);
  }

  /// 함수 등록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn register_function(&mut self, decl: StdlibFunctionDecl) {
    self.functions.insert(decl.name.clone(), decl);
  }

  /// 타입 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn get_type(&self, name: &str) -> Option<&TypeDecl> {
    self.types.get(name)
  }

  /// 함수 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn get_function(&self, name: &str) -> Option<&StdlibFunctionDecl> {
    self.functions.get(name)
  }

  /// 함수 조회 (일관된 API를 위한 alias)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn get(&self, name: &str) -> Option<&StdlibFunctionDecl> {
    self.get_function(name)
  }

  /// 타입 존재 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn contains_type(&self, name: &str) -> bool {
    self.types.contains_key(name)
  }

  /// 함수 존재 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn contains_function(&self, name: &str) -> bool {
    self.functions.contains_key(name)
  }

  /// 함수 존재 확인 (일관된 API를 위한 alias)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn contains(&self, name: &str) -> bool {
    self.contains_function(name)
  }
}

impl Default for StdlibCatalog {
  fn default() -> Self {
    Self::with_defaults()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_stdlib_catalog_creation() {
    let catalog = StdlibCatalog::new();
    assert!(catalog.types.is_empty());
    assert!(catalog.functions.is_empty());
  }

  #[test]
  fn test_stdlib_catalog_with_defaults() {
    let catalog = StdlibCatalog::with_defaults();
    assert!(catalog.contains_type("Num"));
    assert!(catalog.contains_type("Int"));
    assert!(catalog.contains_type("Bool"));
  }

  #[test]
  fn test_stdlib_catalog_get_type() {
    let catalog = StdlibCatalog::with_defaults();
    let num_type = catalog.get_type("Num").unwrap();
    assert_eq!(num_type.name, "Num");
    assert_eq!(num_type.category, "Primitive");
  }

  #[test]
  fn test_stdlib_catalog_string_functions() {
    let catalog = StdlibCatalog::with_defaults();
    assert!(catalog.contains_function("String.concat"));
    assert!(catalog.contains_function("String.slice"));
    assert!(catalog.contains_function("String.length"));
    assert!(catalog.contains_function("String.split"));
    assert!(catalog.contains_function("String.join"));
  }

  #[test]
  fn test_stdlib_catalog_list_functions() {
    let catalog = StdlibCatalog::with_defaults();
    assert!(catalog.contains_function("List.map"));
    assert!(catalog.contains_function("List.filter"));
    assert!(catalog.contains_function("List.fold"));
    assert!(catalog.contains_function("List.find"));
    assert!(catalog.contains_function("List.sort"));
    assert!(catalog.contains_function("List.reverse"));
    assert!(catalog.contains_function("List.take"));
    assert!(catalog.contains_function("List.drop"));
    assert!(catalog.contains_function("List.zip"));
    assert!(catalog.contains_function("List.flatten"));
  }

  #[test]
  fn test_stdlib_catalog_attrset_functions() {
    let catalog = StdlibCatalog::with_defaults();
    assert!(catalog.contains_function("AttrSet.get"));
    assert!(catalog.contains_function("AttrSet.set"));
    assert!(catalog.contains_function("AttrSet.keys"));
    assert!(catalog.contains_function("AttrSet.values"));
    assert!(catalog.contains_function("AttrSet.merge"));
  }

  #[test]
  fn test_stdlib_catalog_process_functions_and_types() {
    let catalog = StdlibCatalog::with_defaults();
    assert!(catalog.contains_type("ProcessSpec"));
    assert!(catalog.contains_type("ProcessHandle"));
    assert!(catalog.contains_type("ProcessStatus"));
    assert!(catalog.contains_type("ProcessExit"));
    assert!(catalog.contains_type("ProcessObservation"));
    assert!(catalog.contains_function("Process.spawn"));
    assert!(catalog.contains_function("Process.ensure"));
    assert!(catalog.contains_function("Process.status"));
    assert!(catalog.contains_function("Process.signal"));
    assert!(catalog.contains_function("Process.wait"));
    assert!(catalog.contains_function("Process.observeSample"));
    assert!(catalog.contains_function("Process.observeSampleById"));
  }
}
