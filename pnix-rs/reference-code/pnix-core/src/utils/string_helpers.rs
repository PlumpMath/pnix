//! String Utility Functions
//!
//! pnix-old의 pnix_utils/string_helpers.rs에서 마이그레이션
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 문자열 변환 함수만 포함, 실행 코드 없음

use std::borrow::Cow;

// ============================================================
// Quote Handling
// ============================================================

/// 문자열에서 따옴표 제거 (할당 없이)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
///
/// # 예시
/// ```ignore
/// assert_eq!(strip_quotes("\"hello\""), "hello");
/// assert_eq!(strip_quotes("hello"), "hello");
/// ```
#[inline]
pub fn strip_quotes(s: &str) -> Cow<'_, str> {
  if s.len() < 2 {
    return Cow::Borrowed(s);
  }

  let mut chars = s.chars();
  let Some(first) = chars.next() else {
    return Cow::Borrowed(s);
  };
  let Some(last) = s.chars().last() else {
    return Cow::Borrowed(s);
  };

  if !is_quote_pair(first, last) {
    return Cow::Borrowed(s);
  }

  let start = first.len_utf8();
  let end = s.len().saturating_sub(last.len_utf8());
  if start <= end && s.is_char_boundary(start) && s.is_char_boundary(end) {
    Cow::Borrowed(&s[start..end])
  } else {
    Cow::Borrowed(s)
  }
}

#[inline]
fn is_quote_pair(open: char, close: char) -> bool {
  matches!(
    (open, close),
    ('"', '"') | ('\'', '\'') | ('“', '”') | ('‘', '’') | ('«', '»') | ('‹', '›')
  )
}

/// 접두사 제거 (할당 없이)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
#[inline]
pub fn strip_prefix_cow<'a>(s: &'a str, prefix: &str) -> Option<Cow<'a, str>> {
  s.strip_prefix(prefix).map(Cow::Borrowed)
}

/// 접미사 제거 (할당 없이)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
#[inline]
pub fn strip_suffix_cow<'a>(s: &'a str, suffix: &str) -> Option<Cow<'a, str>> {
  s.strip_suffix(suffix).map(Cow::Borrowed)
}

// ============================================================
// Keyword Detection
// ============================================================

/// 키워드인지 확인 (: 로 시작)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
#[inline(always)]
pub const fn is_keyword_str(s: &str) -> bool {
  !s.is_empty() && s.as_bytes()[0] == b':'
}

// ============================================================
// Fast Concatenation
// ============================================================

/// 두 문자열 빠른 연결
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
#[inline]
pub fn concat2(a: &str, b: &str) -> String {
  let mut result = String::with_capacity(a.len() + b.len());
  result.push_str(a);
  result.push_str(b);
  result
}

/// 세 문자열 빠른 연결
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
#[inline]
pub fn concat3(a: &str, b: &str, c: &str) -> String {
  let mut result = String::with_capacity(a.len() + b.len() + c.len());
  result.push_str(a);
  result.push_str(b);
  result.push_str(c);
  result
}

/// 문자열 배열을 구분자로 연결
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
#[inline]
pub fn join_strings(strings: &[&str], sep: &str) -> String {
  if strings.is_empty() {
    return String::new();
  }
  if strings.len() == 1 {
    return strings[0].to_string();
  }

  let total_len: usize = strings.iter().map(|s| s.len()).sum();
  let sep_len = sep.len() * (strings.len() - 1);
  let mut result = String::with_capacity(total_len + sep_len);

  result.push_str(strings[0]);
  for s in &strings[1..] {
    result.push_str(sep);
    result.push_str(s);
  }

  result
}

// ============================================================
// Truncation
// ============================================================

/// 문자열 말줄임 (최대 길이 초과 시 ... 추가)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
///
/// # 예시
/// ```ignore
/// assert_eq!(truncate_with_ellipsis("hello world", 8), "hello...");
/// assert_eq!(truncate_with_ellipsis("short", 10), "short");
/// ```
#[inline]
pub fn truncate_with_ellipsis(s: &str, max_len: usize) -> Cow<'_, str> {
  if s.len() <= max_len {
    Cow::Borrowed(s)
  } else if max_len == 0 {
    Cow::Borrowed("")
  } else if max_len <= 3 {
    // Fix: Use char-based truncation even for small max_len to prevent panic on UTF-8 character boundary misalignment
    // For max_len <= 3, we can't add ellipsis, so just truncate at character boundary
    let mut end = max_len;
    while end > 0 && !s.is_char_boundary(end) {
      end -= 1;
    }
    Cow::Borrowed(&s[..end])
  } else {
    // Grapheme cluster 경계 확인하여 이모지/combining character 중간 분할 방지
    let graphemes = crate::text_width::grapheme_slices(s);
    let mut total_len = 0;
    let mut count = 0;
    let ellipsis_len = 3; // "..."

    for grapheme in &graphemes {
      let grapheme_len = grapheme.len();
      if total_len + grapheme_len + ellipsis_len > max_len {
        break;
      }
      total_len += grapheme_len;
      count += 1;
    }

    if count == 0 {
      // 너무 짧은 경우: 유니코드 문자 경계만 확인
      let mut end = max_len - ellipsis_len;
      while !s.is_char_boundary(end) && end > 0 {
        end -= 1;
      }
      Cow::Owned(format!("{}...", &s[..end]))
    } else {
      let truncated: String = graphemes[..count].concat();
      Cow::Owned(format!("{}...", truncated))
    }
  }
}

// ============================================================
// Case Conversion
// ============================================================

/// snake_case로 변환
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn to_snake_case(s: &str) -> String {
  let mut result = String::new();
  for (i, ch) in s.chars().enumerate() {
    if ch.is_ascii_uppercase() && i > 0 {
      result.push('_');
    }
    result.push(ch.to_ascii_lowercase());
  }
  result.replace([' ', '-'], "_")
}

/// PascalCase로 변환
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn to_pascal_case(s: &str) -> String {
  s.split(['_', '-', ' '])
    .map(|word| {
      let mut chars = word.chars();
      match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
      }
    })
    .collect()
}

/// kebab-case로 변환
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn to_kebab_case(s: &str) -> String {
  let mut result = String::new();
  for (i, ch) in s.chars().enumerate() {
    if ch.is_ascii_uppercase() && i > 0 {
      result.push('-');
    }
    result.push(ch.to_ascii_lowercase());
  }
  result.replace([' ', '_'], "-")
}

/// camelCase로 변환
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn to_camel_case(s: &str) -> String {
  let pascal = to_pascal_case(s);
  let mut chars = pascal.chars();
  match chars.next() {
    Some(first) => first.to_ascii_lowercase().to_string() + chars.as_str(),
    None => String::new(),
  }
}

// ============================================================
// Indentation
// ============================================================

/// 각 줄에 들여쓰기 추가
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn indent_lines(s: &str, indent: &str) -> String {
  s.lines()
    .map(|line| {
      if line.is_empty() {
        line.to_string()
      } else {
        format!("{}{}", indent, line)
      }
    })
    .collect::<Vec<_>>()
    .join("\n")
}

/// 공백 들여쓰기 문자열 생성
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn make_indent(level: usize, spaces_per_level: usize) -> String {
  " ".repeat(level * spaces_per_level)
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_strip_quotes() {
    assert_eq!(strip_quotes("\"hello\""), "hello");
    assert_eq!(strip_quotes("'hello'"), "hello");
    assert_eq!(strip_quotes("“hello”"), "hello");
    assert_eq!(strip_quotes("«hello»"), "hello");
    assert_eq!(strip_quotes("hello"), "hello");
    assert_eq!(strip_quotes("\"\""), "");
    assert_eq!(strip_quotes("\""), "\"");
    assert_eq!(strip_quotes("“hello\""), "“hello\"");
  }

  #[test]
  fn test_is_keyword_str() {
    assert!(is_keyword_str(":keyword"));
    assert!(!is_keyword_str("not-keyword"));
    assert!(!is_keyword_str(""));
  }

  #[test]
  fn test_concat2() {
    assert_eq!(concat2("hello", " world"), "hello world");
    assert_eq!(concat2("", "test"), "test");
  }

  #[test]
  fn test_concat3() {
    assert_eq!(concat3("a", "b", "c"), "abc");
  }

  #[test]
  fn test_join_strings() {
    assert_eq!(join_strings(&["a", "b", "c"], ", "), "a, b, c");
    assert_eq!(join_strings(&["single"], ", "), "single");
    assert_eq!(join_strings(&[], ", "), "");
  }

  #[test]
  fn test_truncate_with_ellipsis() {
    assert_eq!(truncate_with_ellipsis("hello world", 8), "hello...");
    assert_eq!(truncate_with_ellipsis("short", 10), "short");
    assert_eq!(truncate_with_ellipsis("ab", 2), "ab");
  }

  #[test]
  fn test_to_snake_case() {
    assert_eq!(to_snake_case("UserProfile"), "user_profile");
    assert_eq!(to_snake_case("HelloWorld"), "hello_world");
    assert_eq!(to_snake_case("simple"), "simple");
    assert_eq!(to_snake_case("Istanbul"), "istanbul");
  }

  #[test]
  fn test_to_pascal_case() {
    assert_eq!(to_pascal_case("user_profile"), "UserProfile");
    assert_eq!(to_pascal_case("hello-world"), "HelloWorld");
    assert_eq!(to_pascal_case("simple"), "Simple");
  }

  #[test]
  fn test_to_kebab_case() {
    assert_eq!(to_kebab_case("UserProfile"), "user-profile");
    assert_eq!(to_kebab_case("HelloWorld"), "hello-world");
    assert_eq!(to_kebab_case("simple"), "simple");
    assert_eq!(to_kebab_case("Istanbul"), "istanbul");
  }

  #[test]
  fn test_to_camel_case() {
    assert_eq!(to_camel_case("user_profile"), "userProfile");
    assert_eq!(to_camel_case("hello-world"), "helloWorld");
    assert_eq!(to_camel_case("simple"), "simple");
    assert_eq!(to_camel_case("Istanbul"), "istanbul");
  }

  #[test]
  fn test_indent_lines() {
    let input = "line1\nline2\nline3";
    let expected = "  line1\n  line2\n  line3";
    assert_eq!(indent_lines(input, "  "), expected);
  }

  #[test]
  fn test_make_indent() {
    assert_eq!(make_indent(2, 4), "        ");
    assert_eq!(make_indent(1, 2), "  ");
    assert_eq!(make_indent(0, 4), "");
  }
}
