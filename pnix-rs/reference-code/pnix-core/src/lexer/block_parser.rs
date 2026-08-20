//! 블록 단위 언어 파서
//!
//! pnix-old의 block_parser/src/lib.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 텍스트 파싱만, 값 계산 없음
//!
//! ## 설계 철학
//!
//! pnix/CT 파일은 Nix를 베이스로 하되, 다른 언어 블록(Python, Clojure 등)이
//! 포함될 수 있습니다. 이 모듈은 파일을 블록 단위로 분석합니다.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
// NOTE: std::path::Path 제거 - 경로 비의존성 원칙 (P0-1)
// 파일 확장자 추출은 문자열 기반으로 수행

/// 언어 블록 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageBlock {
  /// 블록의 언어 (nix, python, clojure 등)
  pub language: String,
  /// 블록의 시작 위치 (바이트 오프셋)
  pub start: usize,
  /// 블록의 끝 위치 (바이트 오프셋)
  pub end: usize,
  /// 블록의 내용
  pub content: String,
  /// 블록의 시작 라인
  pub start_line: usize,
  /// 블록의 시작 컬럼
  pub start_column: usize,
}

/// 블록 파서 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockParserConfig {
  /// 지원되는 언어 목록
  pub supported_languages: Vec<String>,
  /// 블록 시작 패턴 (기본: { language | 또는 { language: )
  pub block_start_patterns: Vec<String>,
  /// 엄격 모드 (블록이 제대로 닫히지 않으면 에러)
  pub strict_mode: bool,
}

impl Default for BlockParserConfig {
  fn default() -> Self {
    Self {
      supported_languages: vec![
        "python".to_string(),
        "py".to_string(),
        "clojure".to_string(),
        "clj".to_string(),
        "nix".to_string(),
        "rust".to_string(),
        "rs".to_string(),
        "javascript".to_string(),
        "js".to_string(),
        "typescript".to_string(),
        "ts".to_string(),
        "haskell".to_string(),
        "hs".to_string(),
      ],
      block_start_patterns: vec!["|".to_string(), ":".to_string()],
      strict_mode: false,
    }
  }
}

/// 블록 파서
pub struct BlockParser {
  config: BlockParserConfig,
}

impl Default for BlockParser {
  fn default() -> Self {
    Self::new()
  }
}

impl BlockParser {
  /// 기본 설정으로 블록 파서 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self {
      config: BlockParserConfig::default(),
    }
  }

  /// 커스텀 설정으로 블록 파서 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn with_config(config: BlockParserConfig) -> Self {
    Self { config }
  }

  /// 기본 설정으로 파일을 언어 블록으로 분해 (편의 메서드)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 파싱만, 값 계산 없음
  pub fn parse_blocks(source: &str, base_language: Option<&str>) -> Vec<LanguageBlock> {
    let parser = Self::new();
    parser.parse_blocks_with_config(source, base_language)
  }

  /// 설정을 사용하여 파일을 언어 블록으로 분해
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 파싱만, 값 계산 없음
  pub fn parse_blocks_with_config(
    &self,
    source: &str,
    base_language: Option<&str>,
  ) -> Vec<LanguageBlock> {
    let base_lang = base_language.unwrap_or("nix");
    let mut blocks = Vec::new();
    let mut current_pos = 0;

    // 간단한 블록 감지: { language | ... }
    for (i, ch) in source.char_indices() {
      if i < current_pos {
        continue;
      }
      if ch == '{' {
        // { 다음에 언어 이름이 오는지 확인
        if let Some((lang, content_start, content_end)) = self.try_parse_block(source, i) {
          // { 이전의 내용을 base 언어 블록으로 추가
          if i > current_pos {
            let content = &source[current_pos..i];
            if !content.trim().is_empty() {
              let (start_line, start_column) = Self::byte_to_line_column(source, current_pos);
              blocks.push(LanguageBlock {
                language: base_lang.to_string(),
                start: current_pos,
                end: i,
                content: content.to_string(),
                start_line,
                start_column,
              });
            }
          }

          // 언어 블록 추가
          let content = &source[content_start..content_end];
          let (start_line, start_column) = Self::byte_to_line_column(source, content_start);
          blocks.push(LanguageBlock {
            language: lang,
            start: content_start,
            end: content_end,
            content: content.to_string(),
            start_line,
            start_column,
          });

          // } 이후로 이동
          current_pos = content_end + 1;
        }
      }
    }

    // 마지막 남은 내용 추가
    if current_pos < source.len() {
      let content = &source[current_pos..];
      if !content.trim().is_empty() {
        let (start_line, start_column) = Self::byte_to_line_column(source, current_pos);
        blocks.push(LanguageBlock {
          language: base_lang.to_string(),
          start: current_pos,
          end: source.len(),
          content: content.to_string(),
          start_line,
          start_column,
        });
      }
    }

    blocks
  }

  /// 블록 파싱 시도
  fn try_parse_block(&self, source: &str, brace_pos: usize) -> Option<(String, usize, usize)> {
    if brace_pos >= source.len() || !source.is_char_boundary(brace_pos) {
      return None;
    }

    let mut idx = brace_pos + 1;
    let len = source.len();

    // 공백 스킵
    while idx < len {
      let ch = source[idx..].chars().next()?;
      if ch.is_ascii_whitespace() {
        idx += ch.len_utf8();
      } else {
        break;
      }
    }

    // 언어 이름 읽기
    let lang_start = idx;
    while idx < len {
      let ch = source[idx..].chars().next()?;
      if ch.is_ascii_alphanumeric() || ch == '_' {
        idx += ch.len_utf8();
      } else {
        break;
      }
    }

    if idx == lang_start {
      return None;
    }

    let lang_name = &source[lang_start..idx];
    let lang = self.detect_language(&lang_name.to_lowercase())?;

    // 공백 스킵
    while idx < len {
      let ch = source[idx..].chars().next()?;
      if ch.is_ascii_whitespace() {
        idx += ch.len_utf8();
      } else {
        break;
      }
    }

    // | 또는 : 확인
    if idx >= len {
      return None;
    }

    let ch = source[idx..].chars().next()?;
    let is_block_start = self
      .config
      .block_start_patterns
      .iter()
      .any(|p| p.len() == 1 && ch.is_ascii() && p.as_bytes()[0] == ch as u8);

    if !is_block_start {
      return None;
    }

    // 내용 시작
    let content_start = idx + ch.len_utf8();

    // 매칭되는 } 찾기
    let content_end = self.find_matching_brace(source, content_start)?;

    Some((lang, content_start, content_end))
  }

  /// 매칭되는 닫는 괄호 찾기
  fn find_matching_brace(&self, source: &str, start: usize) -> Option<usize> {
    if start > source.len() || !source.is_char_boundary(start) {
      return None;
    }

    let mut depth = 1;
    let mut in_string = false;
    let mut string_char = '\0';
    let mut escape = false;

    for (offset, ch) in source[start..].char_indices() {
      let idx = start + offset;

      if in_string {
        if escape {
          escape = false;
          continue;
        }
        if ch == '\\' {
          escape = true;
          continue;
        }
        if ch == string_char {
          in_string = false;
        }
        continue;
      }

      match ch {
        '"' | '\'' => {
          in_string = true;
          string_char = ch;
        }
        '{' => {
          depth += 1;
        }
        '}' => {
          depth -= 1;
          if depth == 0 {
            return Some(idx);
          }
        }
        _ => {}
      }
    }

    None
  }

  /// 바이트 오프셋을 라인/컬럼으로 변환
  pub fn byte_to_line_column(source: &str, byte_offset: usize) -> (usize, usize) {
    let mut line = 0;
    let mut column = 0;

    for (i, ch) in source.char_indices() {
      if i >= byte_offset {
        break;
      }
      if ch == '\n' {
        line += 1;
        column = 0;
      } else {
        column += 1;
      }
    }

    (line, column)
  }

  /// 언어 이름 감지 및 정규화
  fn detect_language(&self, lang_name: &str) -> Option<String> {
    let lang_lower = lang_name.to_lowercase();

    if self.config.supported_languages.contains(&lang_lower) {
      return Some(match lang_lower.as_str() {
        "py" => "python".to_string(),
        "clj" => "clojure".to_string(),
        "rs" => "rust".to_string(),
        "js" => "javascript".to_string(),
        "ts" => "typescript".to_string(),
        "hs" => "haskell".to_string(),
        other => other.to_string(),
      });
    }

    None
  }

  /// 파일 확장자로 기본 언어 감지
  ///
  /// 경로 비의존성 원칙 (P0-1): std::path::Path 대신 문자열 기반 처리
  pub fn detect_base_language(filename: &str) -> String {
    // 마지막 '.'을 기준으로 확장자 추출
    if let Some(dot_pos) = filename.rfind('.') {
      let ext = &filename[dot_pos + 1..];
      return match ext {
        "px" | "nix" => "nix".to_string(),
        "clj" | "cljs" | "cljc" => "clojure".to_string(),
        "py" => "python".to_string(),
        "ct" => "clojure".to_string(),
        "rs" => "rust".to_string(),
        "js" | "jsx" => "javascript".to_string(),
        "ts" | "tsx" => "typescript".to_string(),
        "hs" => "haskell".to_string(),
        _ => "nix".to_string(),
      };
    }
    "nix".to_string()
  }

  /// 블록 파싱 결과 요약 정보
  pub fn summarize_blocks(blocks: &[LanguageBlock]) -> BlockSummary {
    let mut lang_counts = HashMap::new();
    let mut total_size = 0;

    for block in blocks {
      *lang_counts.entry(block.language.clone()).or_insert(0) += 1;
      total_size += block.content.len();
    }

    BlockSummary {
      total_blocks: blocks.len(),
      language_counts: lang_counts,
      total_size,
    }
  }
}

/// 블록 파싱 결과 요약: 블록 파싱 결과의 요약 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockSummary {
  /// 총 블록 수 (발견된 블록의 총 개수)
  pub total_blocks: usize,
  /// 언어별 블록 수 (언어 이름 → 블록 수 매핑)
  pub language_counts: HashMap<String, usize>,
  /// 총 크기 (모든 블록의 총 바이트 수)
  pub total_size: usize,
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_simple_nix() {
    let source = r#"
let
  x = 10;
in
  x
"#;
    let blocks = BlockParser::parse_blocks(source, Some("nix"));
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].language, "nix");
  }

  #[test]
  fn test_parse_nix_with_python_block() {
    let source = r#"
let
  x = 10;
in
  { python |
    def hello():
        return "world"
  }
"#;
    let blocks = BlockParser::parse_blocks(source, Some("nix"));
    assert!(blocks.len() >= 2);

    let python_block = blocks.iter().find(|b| b.language == "python");
    assert!(python_block.is_some());
  }

  #[test]
  fn test_parse_blocks_with_unicode_prefix() {
    let source = "한글 { python | print('ok') } 끝";
    let blocks = BlockParser::parse_blocks(source, Some("nix"));
    assert!(blocks.iter().any(|b| b.language == "python"));
    assert!(blocks.iter().any(|b| b.content.contains("한글")));
  }

  #[test]
  fn test_byte_to_line_column() {
    let source = "line1\nline2\nline3";
    assert_eq!(BlockParser::byte_to_line_column(source, 0), (0, 0));
    assert_eq!(BlockParser::byte_to_line_column(source, 6), (1, 0));
    assert_eq!(BlockParser::byte_to_line_column(source, 8), (1, 2));
  }

  #[test]
  fn test_detect_base_language() {
    assert_eq!(BlockParser::detect_base_language("test.py"), "python");
    assert_eq!(BlockParser::detect_base_language("test.nix"), "nix");
    assert_eq!(BlockParser::detect_base_language("test.clj"), "clojure");
    assert_eq!(BlockParser::detect_base_language("test.rs"), "rust");
    assert_eq!(BlockParser::detect_base_language("no_extension"), "nix"); // 기본값
  }

  #[test]
  fn test_block_summary() {
    let blocks = vec![
      LanguageBlock {
        language: "python".to_string(),
        start: 0,
        end: 10,
        content: "def x(): 1".to_string(),
        start_line: 0,
        start_column: 0,
      },
      LanguageBlock {
        language: "python".to_string(),
        start: 10,
        end: 20,
        content: "def y(): 2".to_string(),
        start_line: 1,
        start_column: 0,
      },
      LanguageBlock {
        language: "nix".to_string(),
        start: 20,
        end: 30,
        content: "let x = 1;".to_string(),
        start_line: 2,
        start_column: 0,
      },
    ];

    let summary = BlockParser::summarize_blocks(&blocks);
    assert_eq!(summary.total_blocks, 3);
    assert_eq!(summary.language_counts.get("python"), Some(&2));
    assert_eq!(summary.language_counts.get("nix"), Some(&1));
  }
}
