//! Identifier helpers for pnix.

/// 문자가 식별자 시작 문자인지 확인
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
pub fn is_ident_start(ch: char) -> bool {
  ch == '_' || ch.is_ascii_alphabetic() || (!ch.is_ascii() && ch.is_alphabetic())
}

/// 문자가 식별자 연속 문자인지 확인
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
pub fn is_ident_continue(ch: char) -> bool {
  ch == '_'
    || ch == '-'
    || ch == '\''
    || ch.is_ascii_alphanumeric()
    || (!ch.is_ascii() && ch.is_alphanumeric())
}

/// 문자열이 유효한 식별자인지 확인
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
pub fn is_ident(name: &str) -> bool {
  let mut chars = name.chars();
  let Some(first) = chars.next() else {
    return false;
  };
  if !is_ident_start(first) {
    return false;
  }
  chars.all(is_ident_continue)
}
