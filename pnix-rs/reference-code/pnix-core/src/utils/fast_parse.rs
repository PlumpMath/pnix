//! 빠른 파싱 유틸리티: 일반적인 경우를 위한 빠른 파싱 함수
//!
//! P0-1 합헌: 파싱만 수행하며 값 계산은 없음 (구조 변환만)

/// 빠른 정수 파싱 (할당 없음)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
#[inline]
pub fn fast_parse_int(s: &str) -> Option<i64> {
  if s.is_empty() {
    return None;
  }

  let bytes = s.as_bytes();
  let (start, negative) = if bytes[0] == b'-' {
    (1, true)
  } else if bytes[0] == b'+' {
    (1, false)
  } else {
    (0, false)
  };

  if start >= bytes.len() {
    return None;
  }

  let mut result = 0i64;

  for &byte in &bytes[start..] {
    if !byte.is_ascii_digit() {
      return None;
    }
    let digit = (byte - b'0') as i64;
    result = result.checked_mul(10)?.checked_add(digit)?;
  }

  Some(if negative { -result } else { result })
}

/// 빠른 부동소수점 파싱 (간단한 경우용)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
#[inline]
pub fn fast_parse_float(s: &str) -> Option<f64> {
  // Fall back to standard library for now
  s.parse::<f64>().ok()
}

/// 문자열이 유효한 정수인지 확인 (파싱 없이)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
#[inline]
pub fn is_integer_str(s: &str) -> bool {
  if s.is_empty() {
    return false;
  }

  let bytes = s.as_bytes();
  let start = if bytes[0] == b'-' || bytes[0] == b'+' {
    1
  } else {
    0
  };

  if start >= bytes.len() {
    return false;
  }

  bytes[start..].iter().all(|b| b.is_ascii_digit())
}

/// 문자열이 유효한 부동소수점인지 확인 (파싱 없이)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
#[inline]
pub fn is_float_str(s: &str) -> bool {
  if s.is_empty() {
    return false;
  }

  let mut has_dot = false;
  let mut has_e = false;
  let bytes = s.as_bytes();
  let mut i = 0;

  // Optional sign
  if bytes[0] == b'-' || bytes[0] == b'+' {
    i += 1;
  }

  if i >= bytes.len() {
    return false;
  }

  while i < bytes.len() {
    let b = bytes[i];
    match b {
      b'0'..=b'9' => {}
      b'.' => {
        if has_dot || has_e {
          return false;
        }
        has_dot = true;
      }
      b'e' | b'E' => {
        if has_e {
          return false;
        }
        has_e = true;
        // Next char can be sign
        if i + 1 < bytes.len() && (bytes[i + 1] == b'-' || bytes[i + 1] == b'+') {
          i += 1;
        }
      }
      _ => return false,
    }
    i += 1;
  }

  true
}

/// 문자열이 특정 문자로 시작하는지 빠르게 확인
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
#[inline(always)]
pub fn starts_with_char(s: &str, c: char) -> bool {
  s.as_bytes().first() == Some(&(c as u8))
}

/// 문자열이 특정 문자로 끝나는지 빠르게 확인
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
#[inline(always)]
pub fn ends_with_char(s: &str, c: char) -> bool {
  s.as_bytes().last() == Some(&(c as u8))
}

/// 문자열에서 특정 문자의 발생 횟수 계산
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
#[inline]
pub fn count_char(s: &str, c: char) -> usize {
  s.bytes().filter(|&b| b == c as u8).count()
}

/// 빠른 문자열 트리밍
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
#[inline]
pub fn fast_trim(s: &str) -> &str {
  s.trim()
}

/// 공백 문자인지 빠르게 확인
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
#[inline(always)]
pub fn is_whitespace(c: char) -> bool {
  matches!(c, ' ' | '\t' | '\n' | '\r')
}

/// 공백을 건너뛰고 첫 번째 비공백 문자의 인덱스 반환
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
#[inline]
pub fn skip_whitespace(s: &str) -> usize {
  s.bytes()
    .position(|b| !is_whitespace(b as char))
    .unwrap_or(s.len())
}

// ============================================================================
// Extended parsing utilities
// ============================================================================

/// 빠른 부호 없는 정수 파싱 (할당 없음)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
#[inline]
pub fn fast_parse_u64(s: &str) -> Option<u64> {
  if s.is_empty() {
    return None;
  }

  let bytes = s.as_bytes();
  let start = if bytes[0] == b'+' { 1 } else { 0 };

  if start >= bytes.len() {
    return None;
  }

  let mut result = 0u64;

  for &byte in &bytes[start..] {
    if !byte.is_ascii_digit() {
      return None;
    }
    let digit = (byte - b'0') as u64;
    result = result.checked_mul(10)?.checked_add(digit)?;
  }

  Some(result)
}

/// 빠른 16진수 파싱 (0x 접두사 유무 무관)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
#[inline]
pub fn fast_parse_hex(s: &str) -> Option<u64> {
  if s.is_empty() {
    return None;
  }

  let s = s
    .strip_prefix("0x")
    .or_else(|| s.strip_prefix("0X"))
    .unwrap_or(s);

  if s.is_empty() {
    return None;
  }

  let mut result = 0u64;

  for &byte in s.as_bytes() {
    let digit = match byte {
      b'0'..=b'9' => byte - b'0',
      b'a'..=b'f' => byte - b'a' + 10,
      b'A'..=b'F' => byte - b'A' + 10,
      _ => return None,
    };
    result = result.checked_mul(16)?.checked_add(digit as u64)?;
  }

  Some(result)
}

/// 빠른 2진수 파싱 (0b 접두사 유무 무관)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
#[inline]
pub fn fast_parse_binary(s: &str) -> Option<u64> {
  if s.is_empty() {
    return None;
  }

  let s = s
    .strip_prefix("0b")
    .or_else(|| s.strip_prefix("0B"))
    .unwrap_or(s);

  if s.is_empty() {
    return None;
  }

  let mut result = 0u64;

  for &byte in s.as_bytes() {
    let digit = match byte {
      b'0' => 0,
      b'1' => 1,
      _ => return None,
    };
    result = result.checked_mul(2)?.checked_add(digit)?;
  }

  Some(result)
}

/// 빠른 8진수 파싱 (0o 접두사 유무 무관)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
#[inline]
pub fn fast_parse_octal(s: &str) -> Option<u64> {
  if s.is_empty() {
    return None;
  }

  let s = s
    .strip_prefix("0o")
    .or_else(|| s.strip_prefix("0O"))
    .unwrap_or(s);

  if s.is_empty() {
    return None;
  }

  let mut result = 0u64;

  for &byte in s.as_bytes() {
    if !matches!(byte, b'0'..=b'7') {
      return None;
    }
    let digit = (byte - b'0') as u64;
    result = result.checked_mul(8)?.checked_add(digit)?;
  }

  Some(result)
}

/// 진법 자동 감지 파싱 (0x=16진수, 0b=2진수, 0o=8진수, 그 외=10진수)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
#[inline]
pub fn parse_number_auto_radix(s: &str) -> Option<u64> {
  if s.is_empty() {
    return None;
  }

  if s.starts_with("0x") || s.starts_with("0X") {
    fast_parse_hex(s)
  } else if s.starts_with("0b") || s.starts_with("0B") {
    fast_parse_binary(s)
  } else if s.starts_with("0o") || s.starts_with("0O") {
    fast_parse_octal(s)
  } else {
    fast_parse_u64(s)
  }
}

/// 빠른 불리언 파싱 (true/false, yes/no, 1/0, on/off)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
#[inline]
pub fn fast_parse_bool(s: &str) -> Option<bool> {
  match s.to_ascii_lowercase().as_str() {
    "true" | "yes" | "1" | "on" => Some(true),
    "false" | "no" | "0" | "off" => Some(false),
    _ => None,
  }
}

/// 첫 번째 문자 발생 지점에서 빠르게 분할
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
#[inline]
pub fn split_once_char(s: &str, c: char) -> Option<(&str, &str)> {
  let pos = s.find(c)?;
  Some((&s[..pos], &s[pos + c.len_utf8()..]))
}

/// 첫 번째 공백에서 빠르게 분할
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
#[inline]
pub fn split_at_whitespace(s: &str) -> Option<(&str, &str)> {
  let pos = s.find(|c: char| c.is_whitespace())?;
  let (first, rest) = s.split_at(pos);
  // Skip the whitespace
  let rest = rest.trim_start();
  Some((first, rest))
}

/// 따옴표 사이의 내용 추출 (작은따옴표 또는 큰따옴표)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
#[inline]
pub fn extract_quoted(s: &str) -> Option<&str> {
  let s = s.trim();
  if s.len() < 2 {
    return None;
  }

  let bytes = s.as_bytes();
  let quote = bytes[0];

  if !matches!(quote, b'"' | b'\'') {
    return None;
  }

  if bytes[bytes.len() - 1] != quote {
    return None;
  }

  Some(&s[1..s.len() - 1])
}

/// key=value 쌍 파싱
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
#[inline]
pub fn parse_key_value(s: &str) -> Option<(&str, &str)> {
  let (key, value) = split_once_char(s, '=')?;
  Some((key.trim(), value.trim()))
}

/// key:value 쌍 파싱 (대체 구분자)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
#[inline]
pub fn parse_key_value_colon(s: &str) -> Option<(&str, &str)> {
  let (key, value) = split_once_char(s, ':')?;
  Some((key.trim(), value.trim()))
}

/// 문자열이 유효한 16진수인지 확인
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
#[inline]
pub fn is_hex_str(s: &str) -> bool {
  if s.is_empty() {
    return false;
  }

  let s = s
    .strip_prefix("0x")
    .or_else(|| s.strip_prefix("0X"))
    .unwrap_or(s);

  if s.is_empty() {
    return false;
  }

  s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// 문자열이 유효한 2진수인지 확인
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
#[inline]
pub fn is_binary_str(s: &str) -> bool {
  if s.is_empty() {
    return false;
  }

  let s = s
    .strip_prefix("0b")
    .or_else(|| s.strip_prefix("0B"))
    .unwrap_or(s);

  if s.is_empty() {
    return false;
  }

  s.bytes().all(|b| matches!(b, b'0' | b'1'))
}

/// 문자열이 유효한 8진수인지 확인
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
#[inline]
pub fn is_octal_str(s: &str) -> bool {
  if s.is_empty() {
    return false;
  }

  let s = s
    .strip_prefix("0o")
    .or_else(|| s.strip_prefix("0O"))
    .unwrap_or(s);

  if s.is_empty() {
    return false;
  }

  s.bytes().all(|b| matches!(b, b'0'..=b'7'))
}

/// 쉼표로 구분된 값 목록 파싱
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn parse_csv_line(s: &str) -> Vec<&str> {
  s.split(',').map(|part| part.trim()).collect()
}

/// 쉼표로 구분된 정수 목록 파싱
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn parse_int_list(s: &str) -> Option<Vec<i64>> {
  s.split(',')
    .map(|part| fast_parse_int(part.trim()))
    .collect()
}

/// 문자열이 유효한 식별자인지 빠르게 확인
///
/// (문자 또는 언더스코어로 시작하고, 그 뒤에 영숫자 또는 언더스코어가 옴)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
#[inline]
pub fn is_identifier_fast(s: &str) -> bool {
  if s.is_empty() {
    return false;
  }

  let bytes = s.as_bytes();
  let first = bytes[0];

  if !matches!(first, b'a'..=b'z' | b'A'..=b'Z' | b'_') {
    return false;
  }

  bytes[1..]
    .iter()
    .all(|&b| matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_'))
}

/// 문자열에서 다음 단어 추출 (비공백 문자 시퀀스)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
#[inline]
pub fn next_word(s: &str) -> Option<(&str, &str)> {
  let s = s.trim_start();
  if s.is_empty() {
    return None;
  }

  let end = s.find(char::is_whitespace).unwrap_or(s.len());
  Some((&s[..end], &s[end..]))
}

/// 공백 또는 쉼표로 구분된 여러 key=value 쌍 파싱
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn parse_key_values(s: &str) -> Vec<(&str, &str)> {
  s.split(|c: char| c.is_whitespace() || c == ',')
    .filter(|part| !part.is_empty())
    .filter_map(parse_key_value)
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_fast_parse_int() {
    assert_eq!(fast_parse_int("123"), Some(123));
    assert_eq!(fast_parse_int("-456"), Some(-456));
    assert_eq!(fast_parse_int("+789"), Some(789));
    assert_eq!(fast_parse_int("0"), Some(0));
    assert_eq!(fast_parse_int("abc"), None);
    assert_eq!(fast_parse_int(""), None);
    assert_eq!(fast_parse_int("12.34"), None);
  }

  #[test]
  fn test_is_integer_str() {
    assert!(is_integer_str("123"));
    assert!(is_integer_str("-456"));
    assert!(is_integer_str("+789"));
    assert!(!is_integer_str("12.34"));
    assert!(!is_integer_str("abc"));
    assert!(!is_integer_str(""));
  }

  #[test]
  fn test_is_float_str() {
    assert!(is_float_str("123"));
    assert!(is_float_str("123.456"));
    assert!(is_float_str("-123.456"));
    assert!(is_float_str("1.23e10"));
    assert!(is_float_str("1.23E-10"));
    assert!(!is_float_str("abc"));
    assert!(!is_float_str("1.2.3"));
  }

  #[test]
  fn test_starts_ends_with_char() {
    assert!(starts_with_char("hello", 'h'));
    assert!(!starts_with_char("hello", 'e'));
    assert!(ends_with_char("hello", 'o'));
    assert!(!ends_with_char("hello", 'l'));
  }

  #[test]
  fn test_count_char() {
    assert_eq!(count_char("hello", 'l'), 2);
    assert_eq!(count_char("hello", 'h'), 1);
    assert_eq!(count_char("hello", 'x'), 0);
  }

  #[test]
  fn test_skip_whitespace() {
    assert_eq!(skip_whitespace("   hello"), 3);
    assert_eq!(skip_whitespace("hello"), 0);
    assert_eq!(skip_whitespace("   "), 3);
  }

  #[test]
  fn test_fast_parse_u64() {
    assert_eq!(fast_parse_u64("123"), Some(123));
    assert_eq!(fast_parse_u64("+456"), Some(456));
    assert_eq!(fast_parse_u64("0"), Some(0));
    assert_eq!(fast_parse_u64(""), None);
    assert_eq!(fast_parse_u64("-1"), None); // unsigned only
    assert_eq!(fast_parse_u64("abc"), None);
  }

  #[test]
  fn test_fast_parse_hex() {
    assert_eq!(fast_parse_hex("0xff"), Some(255));
    assert_eq!(fast_parse_hex("0xFF"), Some(255));
    assert_eq!(fast_parse_hex("FF"), Some(255));
    assert_eq!(fast_parse_hex("0x10"), Some(16));
    assert_eq!(fast_parse_hex("deadbeef"), Some(0xdeadbeef));
    assert_eq!(fast_parse_hex(""), None);
    assert_eq!(fast_parse_hex("0x"), None);
    assert_eq!(fast_parse_hex("0xGG"), None);
  }

  #[test]
  fn test_fast_parse_binary() {
    assert_eq!(fast_parse_binary("0b1010"), Some(10));
    assert_eq!(fast_parse_binary("0B1111"), Some(15));
    assert_eq!(fast_parse_binary("1010"), Some(10));
    assert_eq!(fast_parse_binary("0b0"), Some(0));
    assert_eq!(fast_parse_binary(""), None);
    assert_eq!(fast_parse_binary("0b"), None);
    assert_eq!(fast_parse_binary("0b102"), None);
  }

  #[test]
  fn test_fast_parse_octal() {
    assert_eq!(fast_parse_octal("0o777"), Some(511));
    assert_eq!(fast_parse_octal("0O10"), Some(8));
    assert_eq!(fast_parse_octal("755"), Some(493));
    assert_eq!(fast_parse_octal(""), None);
    assert_eq!(fast_parse_octal("0o"), None);
    assert_eq!(fast_parse_octal("0o89"), None);
  }

  #[test]
  fn test_parse_number_auto_radix() {
    assert_eq!(parse_number_auto_radix("123"), Some(123));
    assert_eq!(parse_number_auto_radix("0xff"), Some(255));
    assert_eq!(parse_number_auto_radix("0b1010"), Some(10));
    assert_eq!(parse_number_auto_radix("0o777"), Some(511));
    assert_eq!(parse_number_auto_radix(""), None);
  }

  #[test]
  fn test_fast_parse_bool() {
    assert_eq!(fast_parse_bool("true"), Some(true));
    assert_eq!(fast_parse_bool("TRUE"), Some(true));
    assert_eq!(fast_parse_bool("yes"), Some(true));
    assert_eq!(fast_parse_bool("1"), Some(true));
    assert_eq!(fast_parse_bool("on"), Some(true));
    assert_eq!(fast_parse_bool("false"), Some(false));
    assert_eq!(fast_parse_bool("FALSE"), Some(false));
    assert_eq!(fast_parse_bool("no"), Some(false));
    assert_eq!(fast_parse_bool("0"), Some(false));
    assert_eq!(fast_parse_bool("off"), Some(false));
    assert_eq!(fast_parse_bool("maybe"), None);
  }

  #[test]
  fn test_split_once_char() {
    assert_eq!(split_once_char("key=value", '='), Some(("key", "value")));
    assert_eq!(split_once_char("a:b:c", ':'), Some(("a", "b:c")));
    assert_eq!(split_once_char("nodelim", '='), None);
  }

  #[test]
  fn test_split_at_whitespace() {
    assert_eq!(split_at_whitespace("hello world"), Some(("hello", "world")));
    assert_eq!(
      split_at_whitespace("one two three"),
      Some(("one", "two three"))
    );
    assert_eq!(split_at_whitespace("nospace"), None);
  }

  #[test]
  fn test_extract_quoted() {
    assert_eq!(extract_quoted("\"hello\""), Some("hello"));
    assert_eq!(extract_quoted("'world'"), Some("world"));
    assert_eq!(extract_quoted("  \"trimmed\"  "), Some("trimmed"));
    assert_eq!(extract_quoted("noquotes"), None);
    assert_eq!(extract_quoted("\"mismatched'"), None);
    assert_eq!(extract_quoted("\""), None);
  }

  #[test]
  fn test_parse_key_value() {
    assert_eq!(parse_key_value("name=John"), Some(("name", "John")));
    assert_eq!(parse_key_value("key = value"), Some(("key", "value")));
    assert_eq!(parse_key_value("noequals"), None);
  }

  #[test]
  fn test_parse_key_value_colon() {
    assert_eq!(parse_key_value_colon("name:John"), Some(("name", "John")));
    assert_eq!(parse_key_value_colon("key : value"), Some(("key", "value")));
    assert_eq!(parse_key_value_colon("nocolon"), None);
  }

  #[test]
  fn test_is_hex_str() {
    assert!(is_hex_str("0xff"));
    assert!(is_hex_str("deadbeef"));
    assert!(is_hex_str("0X123ABC"));
    assert!(!is_hex_str(""));
    assert!(!is_hex_str("0x"));
    assert!(!is_hex_str("0xGHI"));
  }

  #[test]
  fn test_is_binary_str() {
    assert!(is_binary_str("0b1010"));
    assert!(is_binary_str("1111"));
    assert!(!is_binary_str(""));
    assert!(!is_binary_str("0b"));
    assert!(!is_binary_str("0b123"));
  }

  #[test]
  fn test_is_octal_str() {
    assert!(is_octal_str("0o777"));
    assert!(is_octal_str("123"));
    assert!(!is_octal_str(""));
    assert!(!is_octal_str("0o"));
    assert!(!is_octal_str("0o89"));
  }

  #[test]
  fn test_parse_csv_line() {
    assert_eq!(parse_csv_line("a,b,c"), vec!["a", "b", "c"]);
    assert_eq!(
      parse_csv_line("one , two , three"),
      vec!["one", "two", "three"]
    );
    assert_eq!(parse_csv_line("single"), vec!["single"]);
  }

  #[test]
  fn test_parse_int_list() {
    assert_eq!(parse_int_list("1,2,3"), Some(vec![1, 2, 3]));
    assert_eq!(parse_int_list("-1, 0, 1"), Some(vec![-1, 0, 1]));
    assert_eq!(parse_int_list("1,a,3"), None);
  }

  #[test]
  fn test_is_identifier_fast() {
    assert!(is_identifier_fast("hello"));
    assert!(is_identifier_fast("_private"));
    assert!(is_identifier_fast("camelCase"));
    assert!(is_identifier_fast("snake_case"));
    assert!(is_identifier_fast("with123"));
    assert!(!is_identifier_fast("123start"));
    assert!(!is_identifier_fast(""));
    assert!(!is_identifier_fast("has-dash"));
    assert!(!is_identifier_fast("has space"));
  }

  #[test]
  fn test_next_word() {
    assert_eq!(next_word("hello world"), Some(("hello", " world")));
    assert_eq!(next_word("  hello world"), Some(("hello", " world")));
    assert_eq!(next_word("single"), Some(("single", "")));
    assert_eq!(next_word(""), None);
    assert_eq!(next_word("   "), None);
  }

  #[test]
  fn test_parse_key_values() {
    let result = parse_key_values("a=1 b=2 c=3");
    assert_eq!(result.len(), 3);
    assert!(result.contains(&("a", "1")));
    assert!(result.contains(&("b", "2")));
    assert!(result.contains(&("c", "3")));

    let result2 = parse_key_values("x=10,y=20");
    assert_eq!(result2.len(), 2);
  }
}
