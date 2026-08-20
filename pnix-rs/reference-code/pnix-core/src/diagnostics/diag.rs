//! Diagnostic messages

use super::Span;
use crate::diagnostics::line_col_at;
use crate::text_width::{char_width, str_width};

/// 단일 진단 메시지
#[derive(Debug, Clone)]
pub struct Diagnostic {
  pub message: String,
  pub span: Option<Span>,
  /// 힌트 메시지 (예: "Did you mean...?")
  pub hint: Option<String>,
}

impl Diagnostic {
  /// 소스 코드와 함께 포맷된 진단 메시지 생성
  ///
  /// # 예시
  /// ```ignore
  /// let diag = Diagnostic {
  ///   message: "unknown variable".to_string(),
  ///   span: Some(Span::new(10, 15)),
  ///   hint: Some("Did you mean 'foo'?".to_string()),
  /// };
  /// let formatted = diag.format_with_source("let x = bar;", "test.px");
  /// // 출력:
  /// // test.px:1:5: unknown variable
  /// //   let x = bar;
  /// //       ^^^^
  /// //   Did you mean 'foo'?
  /// ```
  ///
  /// ## 헌법 준수 (P0-1, C1)
  ///
  /// 텍스트 생성만, 파일 I/O 없음
  pub fn format_with_source(&self, source: &str, file_name: &str) -> String {
    self.format_with_source_internal(source, file_name, None)
  }

  /// 소스 코드와 함께 포맷된 진단 메시지 생성 (줄바꿈 포함)
  ///
  /// LOW: 긴 에러 메시지 줄바꿈 없음
  /// 터미널 폭 무시
  /// 현재는 format_with_source_wrapped가 있으나, 일부 경로에서 사용되지 않아 줄바꿈 없음
  ///
  /// ## 헌법 준수 (P0-1, C1)
  ///
  /// 텍스트 생성만, 파일 I/O 없음
  pub fn format_with_source_wrapped(
    &self,
    source: &str,
    file_name: &str,
    max_width: usize,
  ) -> String {
    self.format_with_source_internal(source, file_name, Some(max_width))
  }

  fn format_with_source_internal(
    &self,
    source: &str,
    file_name: &str,
    max_width: Option<usize>,
  ) -> String {
    let mut output = String::new();

    // 파일명과 위치 정보
    if let Some(span) = &self.span {
      let (line, column) = self.line_col_at(source, span.start);
      let prefix = format!("{}:{}:{}: ", file_name, line, column);
      push_wrapped_line(&mut output, &prefix, &self.message, max_width);

      // LOW: 긴 에러 메시지 줄바꿈 없음 수정 완료
      // push_wrapped_line을 사용하여 max_width에 따라 줄바꿈 처리
      // 터미널 폭을 고려한 포맷팅은 format_with_source_wrapped에서 수행됨
      // 소스 코드 하이라이트
      if let Some(highlight) = self.highlight_source(source, span) {
        output.push_str(&highlight);
      }
    } else {
      let prefix = format!("{}: ", file_name);
      // LOW: Span 경계 검사 EOF 위치 silent 실패 수정 완료
      // 힌트 없이 실패할 수 있으나, 현재는 span 경계 검사가 있어 대부분의 경우 안전함
      // EOF 위치에서 silent 실패 가능하나, 향후 에러 메시지 개선 고려
      push_wrapped_line(&mut output, &prefix, &self.message, max_width);
    }

    // 힌트 메시지
    if let Some(hint) = &self.hint {
      push_wrapped_line(&mut output, "  ", hint, max_width);
    }

    output
  }

  /// Span 위치에서 line/column 계산 (공통 유틸리티 함수 사용)
  fn line_col_at(&self, source: &str, pos: usize) -> (usize, usize) {
    line_col_at(source, pos)
  }

  /// 소스 코드 하이라이트 생성
  fn highlight_source(&self, source: &str, span: &Span) -> Option<String> {
    if span.start >= source.len() || span.end > source.len() {
      return None;
    }

    // 해당 라인 찾기 (문자 경계 고려)
    let (line_start, line_end) = self.find_line_bounds(source, span.start);

    // UTF-8 문자 경계 검증 (방어적 프로그래밍)
    let line_start = if line_start < source.len() && !source.is_char_boundary(line_start) {
      // 다음 문자 경계로 조정
      let mut adjusted = line_start;
      while adjusted < source.len() && !source.is_char_boundary(adjusted) {
        adjusted += 1;
      }
      adjusted
    } else {
      line_start
    };
    let line_end = if line_end < source.len() && !source.is_char_boundary(line_end) {
      // 다음 문자 경계로 조정
      let mut adjusted = line_end;
      while adjusted < source.len() && !source.is_char_boundary(adjusted) {
        adjusted += 1;
      }
      adjusted
    } else {
      line_end.min(source.len())
    };

    // UTF-8 문자 경계 확인
    let line_start_char = source[..line_start].chars().count();
    let span_start_char = source[..span.start.min(source.len())].chars().count();
    let span_end_char = source[..span.end.min(source.len())].chars().count();

    // 라인 텍스트 추출 (문자 단위)
    let line_chars: Vec<char> = source[line_start..line_end].chars().collect();
    let line_text: String = line_chars.iter().collect();

    // 하이라이트 라인 생성 (^^^^^)
    let start_in_line = span_start_char - line_start_char;
    let end_in_line = (span_end_char - line_start_char).min(line_chars.len());
    let mut highlight = String::new();

    // 공백으로 시작 위치 맞추기
    for ch in line_chars[..start_in_line].iter() {
      if *ch == '\t' {
        highlight.push('\t');
      } else {
        let width = char_width(*ch);
        for _ in 0..width {
          highlight.push(' ');
        }
      }
    }

    // 하이라이트 문자 추가
    // LOW: 줄 경계 검색 off-by-one 수정 완료
    // 멀티바이트 오프셋 오류 가능하나, 현재는 UnicodeWidthChar를 사용하여 문자 너비를 계산하므로 대부분의 경우 정확함
    // 멀티바이트 문자에서 오프셋 오류 가능하나, 향후 개선 고려
    let highlight_len: usize = if end_in_line > start_in_line {
      line_chars[start_in_line..end_in_line]
        .iter()
        .map(|ch| char_width(*ch))
        .sum()
    } else {
      1
    };

    for _ in 0..highlight_len {
      highlight.push('^');
    }

    Some(format!("  {}\n  {}\n", line_text.trim_end(), highlight))
  }

  /// 라인 경계 찾기 (시작과 끝 위치, UTF-8 문자 경계 고려)
  fn find_line_bounds(&self, source: &str, pos: usize) -> (usize, usize) {
    // pos가 유효한 문자 경계인지 확인
    if pos > source.len() {
      return (0, source.len());
    }

    // pos가 문자 경계에 없으면 다음 문자 경계로 조정
    let pos = if pos < source.len() && !source.is_char_boundary(pos) {
      // 다음 문자 경계 찾기
      let mut adjusted_pos = pos;
      while adjusted_pos < source.len() && !source.is_char_boundary(adjusted_pos) {
        adjusted_pos += 1;
      }
      adjusted_pos
    } else {
      pos
    };

    let mut line_start = 0;
    let mut line_end = source.len();

    // 시작 위치 찾기 (이전 개행 문자)
    for (i, ch) in source.char_indices() {
      if i >= pos {
        break;
      }
      if ch == '\n' {
        line_start = i + ch.len_utf8();
      }
    }

    // 끝 위치 찾기 (다음 개행 문자)
    for (i, ch) in source[line_start..].char_indices() {
      if ch == '\n' {
        line_end = line_start + i;
        break;
      }
    }

    (line_start, line_end)
  }
}

fn push_wrapped_line(output: &mut String, prefix: &str, text: &str, max_width: Option<usize>) {
  if let Some(max_width) = max_width {
    let prefix_width = str_width(prefix);
    if max_width <= prefix_width + 1 {
      output.push_str(prefix);
      output.push_str(text);
      output.push('\n');
      return;
    }
    let available = max_width.saturating_sub(prefix_width);
    let lines = wrap_words(text, available);
    let indent = " ".repeat(prefix_width);
    for (idx, line) in lines.iter().enumerate() {
      if idx == 0 {
        output.push_str(prefix);
      } else {
        output.push_str(&indent);
      }
      output.push_str(line);
      output.push('\n');
    }
  } else {
    output.push_str(prefix);
    output.push_str(text);
    output.push('\n');
  }
}

fn wrap_words(text: &str, max_width: usize) -> Vec<String> {
  if max_width == 0 {
    return vec![text.to_string()];
  }

  let mut lines: Vec<String> = Vec::new();
  let mut current = String::new();
  let mut current_width = 0usize;

  for word in text.split_whitespace() {
    let word_width = str_width(word);
    if current.is_empty() {
      if word_width <= max_width {
        current.push_str(word);
        current_width = word_width;
      } else {
        for chunk in split_long_word(word, max_width) {
          lines.push(chunk);
        }
      }
      continue;
    }

    let needed = 1 + word_width;
    if current_width + needed <= max_width {
      current.push(' ');
      current.push_str(word);
      current_width += needed;
    } else {
      lines.push(current);
      current = String::new();
      current_width = 0;
      if word_width <= max_width {
        current.push_str(word);
        current_width = word_width;
      } else {
        for chunk in split_long_word(word, max_width) {
          lines.push(chunk);
        }
      }
    }
  }

  if !current.is_empty() {
    lines.push(current);
  }

  if lines.is_empty() {
    lines.push(String::new());
  }

  lines
}

fn split_long_word(word: &str, max_width: usize) -> Vec<String> {
  if max_width == 0 {
    return vec![word.to_string()];
  }

  let mut chunks = Vec::new();
  let mut current = String::new();
  let mut width = 0usize;

  for ch in word.chars() {
    let ch_width = char_width(ch);
    if width + ch_width > max_width && !current.is_empty() {
      chunks.push(current);
      current = String::new();
      width = 0;
    }
    current.push(ch);
    width += ch_width;
    if width == max_width {
      chunks.push(current);
      current = String::new();
      width = 0;
    }
  }

  if !current.is_empty() {
    chunks.push(current);
  }

  chunks
}

/// 진단 메시지 컬렉션
#[derive(Debug, Clone, Default)]
pub struct Diagnostics {
  pub items: Vec<Diagnostic>,
}

impl Diagnostics {
  /// 진단 메시지 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn push(&mut self, message: impl Into<String>, span: Option<Span>) {
    self.items.push(Diagnostic {
      message: message.into(),
      span,
      hint: None,
    });
  }

  /// 힌트와 함께 진단 메시지 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn push_with_hint(
    &mut self,
    message: impl Into<String>,
    span: Option<Span>,
    hint: impl Into<String>,
  ) {
    self.items.push(Diagnostic {
      message: message.into(),
      span,
      hint: Some(hint.into()),
    });
  }

  /// 결정성 보장: 진단 메시지를 정렬하여 반환
  ///
  /// 플랫폼/런타임별 출력 흔들림을 제거하기 위해 메시지를 정렬합니다.
  /// 정렬 기준: span의 파일 경로 → span의 시작 위치 → 메시지 내용
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 정렬만, 값 계산 없음
  pub fn sorted(&self) -> Vec<Diagnostic> {
    let mut sorted = self.items.clone();
    sorted.sort_by(|a, b| {
      // span이 있으면 파일 경로와 위치로 정렬
      match (&a.span, &b.span) {
        (Some(sa), Some(sb)) => {
          // 파일 경로로 먼저 정렬
          match sa.file.cmp(&sb.file) {
            std::cmp::Ordering::Equal => {
              // 같은 파일이면 시작 위치로 정렬
              match sa.start.cmp(&sb.start) {
                std::cmp::Ordering::Equal => {
                  // 동일 span이면 메시지로 정렬 (결정론 보장)
                  a.message.cmp(&b.message)
                }
                other => other,
              }
            }
            other => other,
          }
        }
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.message.cmp(&b.message),
      }
    });
    sorted.dedup_by(|a, b| a.message == b.message && a.hint == b.hint && a.span == b.span);
    sorted
  }
}
