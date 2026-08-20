//! Source Span for Error Reporting
//!
//! pnix-old의 Span 타입을 pnix-new에 맞게 확장
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 위치 정보, 값 연산 없음
//!
//! ## 사용 목적
//!
//! - 에러 메시지에 소스 위치 표시
//! - AST 노드의 원본 위치 추적
//! - 파일 단위 위치 정보 포함

use serde::{Deserialize, Serialize};

/// 소스 코드 위치
///
/// 파일 내의 바이트 오프셋 범위를 나타냅니다.
///
/// # 예시
/// ```ignore
/// use pnix_core::diagnostics::Span;
///
/// let span = Span::new(10, 20);
/// assert_eq!(span.len(), 10);
///
/// let span_with_file = Span::with_file(10, 20, "main.px");
/// assert_eq!(span_with_file.file.as_deref(), Some("main.px"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
  /// 시작 바이트 오프셋
  pub start: usize,
  /// 끝 바이트 오프셋 (exclusive)
  pub end: usize,
  /// 파일 이름 (선택적). Note (2026-05-16): `skip_serializing_if`
  /// attrs removed for positional binary compatibility — binary
  /// formats need every field present so the byte stream stays
  /// aligned across serialize/deserialize. Cost is small: each
  /// Span gains an Option-marker byte plus 8 zero bytes for unset
  /// line/column.
  pub file: Option<String>,
  /// 1-기반 line 번호 (파서가 채움; 0 은 unknown).
  #[serde(default)]
  pub line: u32,
  /// 1-기반 column 번호 (파서가 채움; 0 은 unknown).
  #[serde(default)]
  pub column: u32,
}

fn is_zero_u32(v: &u32) -> bool {
  *v == 0
}

impl Span {
  /// 새 Span 생성 (파일 없음)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(start: usize, end: usize) -> Self {
    Self {
      start,
      end,
      file: None,
      line: 0,
      column: 0,
    }
  }

  /// 파일과 함께 Span 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn with_file(start: usize, end: usize, file: impl Into<String>) -> Self {
    Self {
      start,
      end,
      file: Some(file.into()),
      line: 0,
      column: 0,
    }
  }

  /// 빈 Span (위치 0)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn empty() -> Self {
    Self {
      start: 0,
      end: 0,
      file: None,
      line: 0,
      column: 0,
    }
  }

  /// 단일 위치 Span
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn point(pos: usize) -> Self {
    Self {
      start: pos,
      end: pos,
      file: None,
      line: 0,
      column: 0,
    }
  }

  /// `Span::point` 변형 — 1-기반 line/column 도 함께 기록한다.
  /// 파서가 source 를 갖고 있을 때 사용해 eval 단계에서 source 없이
  /// 위치를 회수할 수 있게 한다.
  pub fn point_with_line(pos: usize, line: u32, column: u32) -> Self {
    Self {
      start: pos,
      end: pos,
      file: None,
      line,
      column,
    }
  }

  /// 길이 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn len(&self) -> usize {
    self.end.saturating_sub(self.start)
  }

  /// 빈 span인지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_empty(&self) -> bool {
    self.start >= self.end
  }

  /// 위치가 span 내에 있는지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn contains(&self, pos: usize) -> bool {
    pos >= self.start && pos < self.end
  }

  /// 두 span이 겹치는지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn overlaps(&self, other: &Span) -> bool {
    self.start < other.end && other.start < self.end
  }

  /// 두 span 병합 (가장 넓은 범위)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 병합만, 값 계산 없음
  pub fn merge(&self, other: &Span) -> Span {
    let start = self.start.min(other.start);
    let mut end = self.end.max(other.end);
    if end < start {
      end = start;
    }
    let file = match (&self.file, &other.file) {
      (Some(a), Some(b)) if a == b => Some(a.clone()),
      (Some(_), Some(_)) => None,
      (Some(a), None) => Some(a.clone()),
      (None, Some(b)) => Some(b.clone()),
      (None, None) => None,
    };
    // Take the lhs's line/col (closer to the merge anchor); merge is
    // mostly used for diagnostics, not for positional precision.
    Span {
      start,
      end,
      file,
      line: self.line,
      column: self.column,
    }
  }

  /// 파일 설정
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 설정만, 값 계산 없음
  pub fn set_file(&mut self, file: impl Into<String>) {
    self.file = Some(file.into());
  }

  /// 파일 이름 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn file_name(&self) -> Option<&str> {
    self.file.as_deref()
  }

  /// 오프셋으로 시작 위치 조정
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn offset(self, delta: isize) -> Span {
    let new_start = (self.start as isize + delta).max(0) as usize;
    let mut new_end = (self.end as isize + delta).max(0) as usize;
    if new_end < new_start {
      new_end = new_start;
    }
    Span {
      start: new_start,
      end: new_end,
      file: self.file,
      line: self.line,
      column: self.column,
    }
  }
}

impl std::fmt::Display for Span {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match &self.file {
      Some(file) => {
        write!(f, "{}:{}..{}", file, self.start, self.end)
      }
      None => write!(f, "{}..{}", self.start, self.end),
    }
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_span_new() {
    let span = Span::new(10, 20);
    assert_eq!(span.start, 10);
    assert_eq!(span.end, 20);
    assert!(span.file.is_none());
  }

  #[test]
  fn test_span_with_file() {
    let span = Span::with_file(5, 15, "test.px");
    assert_eq!(span.start, 5);
    assert_eq!(span.end, 15);
    assert_eq!(span.file_name(), Some("test.px"));
  }

  #[test]
  fn display_includes_file_offsets() {
    let span = Span::with_file(4, 9, "test.px");
    assert_eq!(span.to_string(), "test.px:4..9");
  }

  #[test]
  fn test_span_empty() {
    let span = Span::empty();
    assert!(span.is_empty());
    assert_eq!(span.len(), 0);
  }

  #[test]
  fn test_span_point() {
    let span = Span::point(42);
    assert_eq!(span.start, 42);
    assert_eq!(span.end, 42);
    assert!(span.is_empty());
  }

  #[test]
  fn test_span_len() {
    let span = Span::new(10, 25);
    assert_eq!(span.len(), 15);
  }

  #[test]
  fn test_span_is_empty() {
    assert!(Span::new(5, 5).is_empty());
    assert!(Span::new(10, 5).is_empty()); // invalid span
    assert!(!Span::new(5, 10).is_empty());
  }

  #[test]
  fn test_span_contains() {
    let span = Span::new(10, 20);
    assert!(!span.contains(9));
    assert!(span.contains(10));
    assert!(span.contains(15));
    assert!(span.contains(19));
    assert!(!span.contains(20)); // exclusive
  }

  #[test]
  fn test_span_overlaps() {
    let span1 = Span::new(10, 20);
    let span2 = Span::new(15, 25);
    let span3 = Span::new(25, 30);

    assert!(span1.overlaps(&span2));
    assert!(span2.overlaps(&span1));
    assert!(!span1.overlaps(&span3));
    assert!(!span3.overlaps(&span1));
  }

  #[test]
  fn test_span_merge() {
    let span1 = Span::new(10, 20);
    let span2 = Span::new(15, 30);
    let merged = span1.merge(&span2);

    assert_eq!(merged.start, 10);
    assert_eq!(merged.end, 30);
  }

  #[test]
  fn test_span_merge_with_file() {
    let span1 = Span::with_file(10, 20, "test.px");
    let span2 = Span::new(25, 30);
    let merged = span1.merge(&span2);

    assert_eq!(merged.file_name(), Some("test.px"));
  }

  #[test]
  fn test_span_offset() {
    let span = Span::new(10, 20);
    let offset_span = span.offset(5);
    assert_eq!(offset_span.start, 15);
    assert_eq!(offset_span.end, 25);

    let neg_offset = Span::new(10, 20).offset(-15);
    assert_eq!(neg_offset.start, 0); // clamped to 0
    assert_eq!(neg_offset.end, 5);
  }

  #[test]
  fn test_span_display() {
    let span1 = Span::new(10, 20);
    assert_eq!(format!("{}", span1), "10..20");

    let span2 = Span::with_file(10, 20, "main.px");
    assert_eq!(format!("{}", span2), "main.px:10..20");
  }

  #[test]
  fn test_span_serde() {
    let span = Span::with_file(10, 20, "test.px");
    let json = serde_json::to_string(&span).unwrap();
    let restored: Span = serde_json::from_str(&json).unwrap();
    assert_eq!(span, restored);
  }

  #[test]
  fn test_span_serde_no_file() {
    let span = Span::new(10, 20);
    let json = serde_json::to_string(&span).unwrap();
    // file 필드가 생략되어야 함
    assert!(!json.contains("file"));

    let restored: Span = serde_json::from_str(&json).unwrap();
    assert_eq!(span, restored);
  }
}
