//! Block Context - 블록 간 심볼 추출 및 의존성 분석
//!
//! pnix-old의 pnix_block_context/src/lib.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 정적 텍스트 분석만, 값 계산 없음
//!
//! ## 주요 기능
//!
//! - **심볼 추출**: 언어별 심볼(변수, 함수, 클래스) 추출
//! - **의존성 분석**: 블록 간 의존성 그래프 생성
//! - **다중 언어 지원**: Python, Clojure, Nix 등 언어별 분석

use super::block_parser::{BlockParser, LanguageBlock};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 블록 간 컨텍스트 정보
/// 블록 컨텍스트: 코드 블록의 컨텍스트 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockContext {
  /// 블록 ID (고유 식별자)
  pub block_id: String,
  /// 블록의 언어 (블록의 프로그래밍 언어)
  pub language: String,
  /// 블록에서 정의된 심볼들 (변수, 함수 등, 심볼 이름 → 정보 매핑)
  pub symbols: HashMap<String, SymbolInfo>,
  /// 블록에서 사용하는 외부 심볼들 (다른 블록에서 정의된 것, 의존성 목록)
  pub dependencies: Vec<String>,
  /// 블록에서 export하는 심볼들 (다른 블록에서 사용 가능, export 목록)
  pub exports: Vec<String>,
}

/// 심볼 정보: 코드 블록 내 심볼의 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
  /// 심볼 이름 (심볼의 이름)
  pub name: String,
  /// 심볼 타입 (변수, 함수, 클래스 등)
  pub symbol_type: SymbolType,
  /// 정의 위치 (블록 내 상대 라인 번호)
  pub line: usize,
  /// 정의 위치 (블록 내 상대 컬럼 번호)
  pub column: usize,
  /// 문서 주석 (있는 경우, 선택적)
  pub doc: Option<String>,
}

/// 심볼 타입: 코드 심볼의 타입
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolType {
  /// 변수
  Variable,
  /// 함수
  Function,
  /// 클래스
  Class,
  /// 상수
  Constant,
  /// Import
  Import,
  /// 기타 (사용자 정의 타입)
  Other(
    /// 타입 이름
    String,
  ),
}

/// 블록 컨텍스트 분석기
pub struct BlockContextAnalyzer;

impl BlockContextAnalyzer {
  /// 블록에서 심볼 추출 (언어별 파싱)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn analyze_block(block: &LanguageBlock) -> BlockContext {
    let mut context = BlockContext {
      block_id: format!("block_{}", block.start),
      language: block.language.clone(),
      symbols: HashMap::new(),
      dependencies: Vec::new(),
      exports: Vec::new(),
    };

    // 언어별 심볼 추출
    match block.language.as_str() {
      "python" => Self::analyze_python_block(block, &mut context),
      "clojure" => Self::analyze_clojure_block(block, &mut context),
      "nix" => Self::analyze_nix_block(block, &mut context),
      _ => {
        // 기본 분석 (키워드 기반)
        Self::analyze_generic_block(block, &mut context);
      }
    }

    context
  }

  /// Python 블록 분석
  fn analyze_python_block(block: &LanguageBlock, context: &mut BlockContext) {
    let lines: Vec<&str> = block.content.lines().collect();

    for (line_num, line) in lines.iter().enumerate() {
      let trimmed = line.trim();

      // 함수 정의: def function_name(...):
      if let Some(rest) = trimmed.strip_prefix("def ") {
        if let Some(name_end) = rest.find('(') {
          let name = rest[..name_end].trim().to_string();
          context.symbols.insert(
            name.clone(),
            SymbolInfo {
              name: name.clone(),
              symbol_type: SymbolType::Function,
              line: line_num,
              column: trimmed.find("def").unwrap_or(0),
              doc: Self::extract_python_doc(&lines, line_num),
            },
          );
          context.exports.push(name);
        }
      }

      // 클래스 정의: class ClassName:
      if let Some(rest) = trimmed.strip_prefix("class ") {
        if let Some(name_end) = rest.find(':') {
          let name = rest[..name_end].trim().to_string();
          // 상속 구문 제거 (class Foo(Bar):)
          let clean_name = name.split('(').next().unwrap_or(&name).to_string();
          context.symbols.insert(
            clean_name.clone(),
            SymbolInfo {
              name: clean_name.clone(),
              symbol_type: SymbolType::Class,
              line: line_num,
              column: trimmed.find("class").unwrap_or(0),
              doc: None,
            },
          );
          context.exports.push(clean_name);
        }
      }

      // 변수 할당: variable = value
      if trimmed.contains('=')
        && !trimmed.starts_with("def ")
        && !trimmed.starts_with("class ")
        && !trimmed.contains("==")
        && !trimmed.contains("!=")
      {
        if let Some(equal_pos) = trimmed.find('=') {
          let name = trimmed[..equal_pos].trim().to_string();
          if let Some(first_char) = name.chars().next() {
            if first_char.is_alphabetic() && !name.contains('.') {
              context.symbols.insert(
                name.clone(),
                SymbolInfo {
                  name: name.clone(),
                  symbol_type: SymbolType::Variable,
                  line: line_num,
                  column: 0,
                  doc: None,
                },
              );
            }
          }
        }
      }

      // import 문: import module or from module import name
      if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
        context.dependencies.push(trimmed.to_string());
      }
    }
  }

  /// Clojure 블록 분석
  fn analyze_clojure_block(block: &LanguageBlock, context: &mut BlockContext) {
    let content = &block.content;

    // (def name value) 패턴 찾기
    let mut pos = 0;
    while let Some(def_start) = content[pos..].find("(def ") {
      let def_pos = pos + def_start;
      let after_def = &content[def_pos + 5..];

      if let Some(name_end) = after_def.find(|c: char| c.is_whitespace() || c == ')') {
        let name = after_def[..name_end].trim().to_string();
        let (line, col) = BlockParser::byte_to_line_column(content, def_pos);

        context.symbols.insert(
          name.clone(),
          SymbolInfo {
            name: name.clone(),
            symbol_type: SymbolType::Variable,
            line,
            column: col,
            doc: None,
          },
        );
        context.exports.push(name);
      }

      pos = def_pos + 5;
    }

    // (defn name ...) 패턴 찾기
    pos = 0;
    while let Some(defn_start) = content[pos..].find("(defn ") {
      let defn_pos = pos + defn_start;
      let after_defn = &content[defn_pos + 6..];

      if let Some(name_end) = after_defn.find(|c: char| c.is_whitespace() || c == '[' || c == ')') {
        let name = after_defn[..name_end].trim().to_string();
        let (line, col) = BlockParser::byte_to_line_column(content, defn_pos);

        context.symbols.insert(
          name.clone(),
          SymbolInfo {
            name: name.clone(),
            symbol_type: SymbolType::Function,
            line,
            column: col,
            doc: None,
          },
        );
        context.exports.push(name);
      }

      pos = defn_pos + 6;
    }
  }

  /// Nix 블록 분석
  fn analyze_nix_block(block: &LanguageBlock, context: &mut BlockContext) {
    let content = &block.content;
    let lines: Vec<&str> = content.lines().collect();

    // let ... in 패턴에서 변수 추출 (라인 기반)
    let mut in_let_block = false;

    for (line_num, line) in lines.iter().enumerate() {
      let trimmed = line.trim();

      // let 블록 시작 감지
      if trimmed == "let" || trimmed.starts_with("let ") {
        in_let_block = true;
        continue;
      }

      // in 키워드에서 let 블록 종료
      if trimmed == "in" || trimmed.starts_with("in ") {
        in_let_block = false;
        continue;
      }

      // let 블록 내 변수 할당: name = value;
      if in_let_block && trimmed.contains('=') && !trimmed.contains("==") {
        if let Some(equal_pos) = trimmed.find('=') {
          let name = trimmed[..equal_pos].trim().to_string();
          if let Some(first_char) = name.chars().next() {
            if first_char.is_alphabetic() && !name.contains(' ') {
              context.symbols.insert(
                name.clone(),
                SymbolInfo {
                  name: name.clone(),
                  symbol_type: SymbolType::Variable,
                  line: line_num,
                  column: 0,
                  doc: None,
                },
              );
              context.exports.push(name);
            }
          }
        }
      }
    }
  }

  /// 일반 블록 분석 (키워드 기반)
  fn analyze_generic_block(block: &LanguageBlock, context: &mut BlockContext) {
    let content = &block.content;
    let lines: Vec<&str> = content.lines().collect();

    for (line_num, line) in lines.iter().enumerate() {
      let trimmed = line.trim();

      // 일반적인 변수 할당 패턴 감지
      if trimmed.contains('=') && !trimmed.contains("==") {
        if let Some(equal_pos) = trimmed.find('=') {
          let name = trimmed[..equal_pos].trim().to_string();
          if let Some(first_char) = name.chars().next() {
            if first_char.is_alphabetic() {
              context.symbols.insert(
                name.clone(),
                SymbolInfo {
                  name: name.clone(),
                  symbol_type: SymbolType::Variable,
                  line: line_num,
                  column: 0,
                  doc: None,
                },
              );
            }
          }
        }
      }
    }
  }

  /// Python docstring 추출
  fn extract_python_doc(lines: &[&str], function_line: usize) -> Option<String> {
    // 함수 정의 다음 줄에서 docstring 찾기
    if function_line + 1 < lines.len() {
      let next_line = lines[function_line + 1].trim();
      if next_line.starts_with("\"\"\"") || next_line.starts_with("'''") {
        let quote = if next_line.starts_with("\"\"\"") {
          "\"\"\""
        } else {
          "'''"
        };
        let mut doc = String::new();
        let mut line_idx = function_line + 1;

        while line_idx < lines.len() {
          let line = lines[line_idx];
          if line.contains(quote) && line_idx > function_line + 1 {
            break;
          }
          if line_idx > function_line + 1 {
            doc.push('\n');
          }
          doc.push_str(line.trim_start_matches(quote).trim_end_matches(quote));
          line_idx += 1;
        }

        if !doc.is_empty() {
          return Some(doc);
        }
      }
    }
    None
  }

  /// 여러 블록의 컨텍스트를 통합하여 의존성 그래프 생성
  pub fn build_dependency_graph(contexts: &[BlockContext]) -> DependencyGraph {
    let mut graph = DependencyGraph {
      nodes: Vec::new(),
      edges: Vec::new(),
    };

    for context in contexts {
      graph.nodes.push(context.block_id.clone());

      // 의존성 엣지 추가
      for dep in &context.dependencies {
        // 의존성이 다른 블록의 export와 매칭되는지 확인
        for other_context in contexts {
          if other_context.block_id != context.block_id
            && other_context.exports.iter().any(|exp| dep.contains(exp))
          {
            graph.edges.push(DependencyEdge {
              from: other_context.block_id.clone(),
              to: context.block_id.clone(),
              symbol: dep.clone(),
            });
          }
        }
      }
    }

    graph
  }
}

/// 의존성 그래프: 블록 간 의존성 관계 그래프
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
  /// 노드 목록 (블록 ID 목록)
  pub nodes: Vec<String>,
  /// 엣지 목록 (의존성 관계 목록)
  pub edges: Vec<DependencyEdge>,
}

/// 의존성 엣지: 블록 간 의존성 관계
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
  /// 출발 블록 ID (의존성을 제공하는 블록)
  pub from: String,
  /// 도착 블록 ID (의존성을 사용하는 블록)
  pub to: String,
  /// 심볼 이름 (의존하는 심볼 이름)
  pub symbol: String,
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_analyze_python_block() {
    let block = LanguageBlock {
      language: "python".to_string(),
      start: 0,
      end: 100,
      content: r#"
def hello(name):
    """Greet someone"""
    return f"Hello, {name}!"

x = 10
"#
      .to_string(),
      start_line: 0,
      start_column: 0,
    };

    let context = BlockContextAnalyzer::analyze_block(&block);

    assert!(context.symbols.contains_key("hello"));
    assert_eq!(context.symbols["hello"].symbol_type, SymbolType::Function);
    assert!(context.symbols.contains_key("x"));
    assert_eq!(context.symbols["x"].symbol_type, SymbolType::Variable);
  }

  #[test]
  fn test_analyze_clojure_block() {
    let block = LanguageBlock {
      language: "clojure".to_string(),
      start: 0,
      end: 100,
      content: r#"
(def x 10)
(defn add [a b]
  (+ a b))
"#
      .to_string(),
      start_line: 0,
      start_column: 0,
    };

    let context = BlockContextAnalyzer::analyze_block(&block);

    assert!(context.symbols.contains_key("x"));
    assert!(context.symbols.contains_key("add"));
    assert_eq!(context.symbols["add"].symbol_type, SymbolType::Function);
  }

  #[test]
  fn test_analyze_nix_block() {
    let block = LanguageBlock {
      language: "nix".to_string(),
      start: 0,
      end: 100,
      content: r#"
let
  x = 10;
  y = 20;
in
  x + y
"#
      .to_string(),
      start_line: 0,
      start_column: 0,
    };

    let context = BlockContextAnalyzer::analyze_block(&block);
    assert!(context.symbols.contains_key("x"));
    assert!(context.symbols.contains_key("y"));
  }

  #[test]
  fn test_dependency_graph() {
    let context1 = BlockContext {
      block_id: "block_0".to_string(),
      language: "python".to_string(),
      symbols: HashMap::new(),
      dependencies: vec![],
      exports: vec!["helper".to_string()],
    };

    let context2 = BlockContext {
      block_id: "block_100".to_string(),
      language: "python".to_string(),
      symbols: HashMap::new(),
      dependencies: vec!["from utils import helper".to_string()],
      exports: vec![],
    };

    let graph = BlockContextAnalyzer::build_dependency_graph(&[context1, context2]);

    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.edges[0].from, "block_0");
    assert_eq!(graph.edges[0].to, "block_100");
  }
}
