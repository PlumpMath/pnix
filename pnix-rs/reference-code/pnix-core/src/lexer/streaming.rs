//! 스트리밍 렉싱 모듈
//!
//! pnix-old의 pnix_tokenizer/src/streaming.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 없음 (Iterator 기반 토큰 생성 구조)

#![allow(dead_code)]

/// 스트리밍 토큰화기
///
/// Iterator 기반으로 토큰을 점진적으로 생성합니다.
/// 큰 파일을 처리할 때 메모리 효율적입니다.
/// 스트리밍 토크나이저: 스트림에서 문자를 읽어 토큰화하는 토크나이저
pub struct StreamingTokenizer<I>
where
  I: Iterator<Item = char>,
{
  /// 소스 문자 스트림 (문자 반복자)
  source: I,
  /// 미리 읽은 문자 (peeked character, 선택적)
  peeked: Option<char>,
  /// 현재 라인 번호
  line: usize,
  /// 현재 컬럼 번호
  column: usize,
  /// Clojure 블록 내부 여부
  in_clj_block: bool,
  /// Clojure 블록 중첩 깊이
  clj_block_depth: usize,
  /// 현재 토큰을 구성하는 문자 버퍼 (향후 사용)
  _buffer: Vec<char>,
}

impl<I> StreamingTokenizer<I>
where
  I: Iterator<Item = char>,
{
  /// 새로운 스트리밍 토큰화기 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(source: I) -> Self {
    Self {
      source,
      peeked: None,
      line: 1,
      column: 1,
      in_clj_block: false,
      clj_block_depth: 0,
      _buffer: Vec::new(),
    }
  }

  /// 다음 문자 확인 (소비하지 않음)
  fn peek(&mut self) -> Option<char> {
    if self.peeked.is_none() {
      self.peeked = self.source.next();
    }
    self.peeked
  }

  /// 다음 다음 문자 확인 (peek의 다음 문자)
  ///
  /// 주의: 이 함수는 lookahead를 위해 두 번째 문자를 소비합니다.
  /// 현재 구현에서는 단일 버퍼만 사용하므로, peek_next 호출 후
  /// 반드시 advance()로 첫 번째 문자를 소비해야 합니다.
  /// 그렇지 않으면 두 번째 문자가 손실됩니다.
  fn peek_next(&mut self) -> Option<char> {
    // 먼저 peeked를 채움 (비어있으면)
    if self.peeked.is_none() {
      self.peeked = self.source.next();
    }

    // 두 번째 문자 가져오기 (소비됨 - 주의 필요)
    self.source.next()
  }

  /// 문자 소비
  fn advance(&mut self) -> Option<char> {
    let ch = if let Some(c) = self.peeked.take() {
      c
    } else {
      self.source.next()?
    };

    if ch == '\n' {
      self.line += 1;
      self.column = 1;
    } else {
      self.column += 1;
    }

    Some(ch)
  }

  /// 공백 건너뛰기
  fn skip_whitespace(&mut self) {
    while let Some(ch) = self.peek() {
      if ch.is_whitespace() {
        self.advance();
      } else {
        break;
      }
    }
  }

  /// 현재 라인 번호
  pub fn line(&self) -> usize {
    self.line
  }

  /// 현재 컬럼 번호
  pub fn column(&self) -> usize {
    self.column
  }

  /// Clojure 블록 내부인지 확인
  pub fn in_clj_block(&self) -> bool {
    self.in_clj_block
  }
}

/// 문자열로부터 스트리밍 토큰화기 생성
impl<'a> StreamingTokenizer<std::str::Chars<'a>> {
  /// 문자열로부터 스트리밍 토큰화기 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  #[allow(clippy::should_implement_trait)]
  pub fn from_str(source: &'a str) -> Self {
    Self::new(source.chars())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_streaming_tokenizer_new() {
    let tokenizer = StreamingTokenizer::from_str("test");
    assert_eq!(tokenizer.line(), 1);
    assert_eq!(tokenizer.column(), 1);
    assert!(!tokenizer.in_clj_block());
  }

  #[test]
  fn test_peek_and_advance() {
    let mut tokenizer = StreamingTokenizer::from_str("abc");
    assert_eq!(tokenizer.peek(), Some('a'));
    assert_eq!(tokenizer.peek(), Some('a')); // peek는 소비하지 않음
    assert_eq!(tokenizer.advance(), Some('a'));
    assert_eq!(tokenizer.peek(), Some('b'));
  }

  #[test]
  fn test_line_column_tracking() {
    let mut tokenizer = StreamingTokenizer::from_str("a\nb");
    assert_eq!(tokenizer.line(), 1);
    assert_eq!(tokenizer.column(), 1);
    tokenizer.advance(); // 'a'
    assert_eq!(tokenizer.column(), 2);
    tokenizer.advance(); // '\n'
    assert_eq!(tokenizer.line(), 2);
    assert_eq!(tokenizer.column(), 1);
  }
}
