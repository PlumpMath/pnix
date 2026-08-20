//! JSON Escape Utilities
//!
//! pnix-old의 pnix_utils/json_escape.rs에서 마이그레이션
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 문자열 변환 함수만 포함, 실행 코드 없음

/// JSON 문자열 이스케이프
///
/// 문자열 리터럴에 삽입할 때 특수 문자를 이스케이프합니다.
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
///
/// # 성능
/// - 단일 패스 (O(n))
/// - 필요할 때만 할당
///
/// # 예시
/// ```ignore
/// let input = r#"{"name":"test"}"#;
/// let output = escape_json_for_string(input);
/// assert_eq!(output, r#"{\"name\":\"test\"}"#);
/// ```
pub fn escape_json_for_string(json_str: &str) -> String {
  // 이스케이프가 필요한지 먼저 확인 (RFC 8259 준수)
  // 제어 문자(U+0000-U+001F)와 특수 문자 체크
  let needs_escape = json_str
    .chars()
    .any(|c| matches!(c, '"' | '\\' | '\x00'..='\x1F'));

  if !needs_escape {
    return json_str.to_string();
  }

  // 이스케이프가 필요할 때만 할당
  let mut escaped = String::with_capacity(json_str.len() + json_str.len() / 10);

  for ch in json_str.chars() {
    match ch {
      '"' => escaped.push_str("\\\""),
      '\\' => escaped.push_str("\\\\"),
      '\n' => escaped.push_str("\\n"),
      '\r' => escaped.push_str("\\r"),
      '\t' => escaped.push_str("\\t"),
      '\x08' => escaped.push_str("\\b"), // backspace
      '\x0C' => escaped.push_str("\\f"), // form feed
      // 기타 제어 문자 (U+0000-U+001F): \uXXXX 형식으로 escape
      // LOW: JSON 이스케이프 대소문자 불일치 수정 완료
      // JSON 표준에서는 \uXXXX의 hex digits 대소문자를 구분하지 않지만,
      // 일관성을 위해 대문자 사용 (RFC 7159 Section 7)
      // 대문자와 소문자 모두 유효한 JSON이므로 문제 없음
      c if (c as u32) < 0x20 => {
        escaped.push_str(&format!("\\u{:04X}", c as u32));
      }
      // LOW: unescape에서 알 수 없는 escape sequence silent 통과
      // 알 수 없는 escape sequence (예: \q)는 백슬래시와 문자를 그대로 보존
      // 이는 JSON 스펙에 맞지 않지만, 호환성을 위해 원본 문자를 유지
      _ => escaped.push(ch),
    }
  }

  escaped
}

/// JSON 문자열 언이스케이프
///
/// 이스케이프된 문자열을 원래 형태로 복원합니다.
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn unescape_json_string(s: &str) -> String {
  let mut result = String::with_capacity(s.len());
  let mut chars = s.chars().peekable();

  fn read_hex4_peek(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut digits = String::new();
    for _ in 0..4 {
      if let Some(&hex_ch) = chars.peek() {
        if hex_ch.is_ascii_hexdigit() {
          digits.push(hex_ch);
          chars.next();
        } else {
          break;
        }
      } else {
        break;
      }
    }
    digits
  }

  fn read_hex4_iter<I: Iterator<Item = char>>(chars: &mut I) -> Option<String> {
    let mut digits = String::new();
    for _ in 0..4 {
      let hex_ch = chars.next()?;
      if !hex_ch.is_ascii_hexdigit() {
        return None;
      }
      digits.push(hex_ch);
    }
    Some(digits)
  }

  while let Some(ch) = chars.next() {
    if ch == '\\' {
      if let Some(&next) = chars.peek() {
        match next {
          '"' => {
            result.push('"');
            chars.next();
          }
          '\\' => {
            result.push('\\');
            chars.next();
          }
          'n' => {
            result.push('\n');
            chars.next();
          }
          'r' => {
            result.push('\r');
            chars.next();
          }
          't' => {
            result.push('\t');
            chars.next();
          }
          'b' => {
            result.push('\x08'); // backspace
            chars.next();
          }
          'f' => {
            result.push('\x0C'); // form feed
            chars.next();
          }
          'u' => {
            // Unicode escape: \uXXXX
            chars.next(); // consume 'u'
            let hex_digits = read_hex4_peek(&mut chars);
            if hex_digits.len() < 4 {
              // LOW: unescape에서 알 수 없는 escape sequence 처리 개선
              // 잘못된 형식의 \u escape는 원본을 보존하되, 경고 로그 출력
              // JSON 표준에 따라 \uXXXX는 정확히 4자리 hex여야 함
              eprintln!(
                "Warning: Invalid Unicode escape sequence \\u{} (expected 4 hex digits)",
                hex_digits
              );
              result.push_str("\\u");
              result.push_str(&hex_digits);
              continue;
            }

            let code_point = match u32::from_str_radix(&hex_digits, 16) {
              Ok(value) => value,
              Err(_) => {
                // LOW: 파싱 실패 시 \uXXXX를 그대로 추가 (이미 처리됨)
                // 알 수 없는 escape sequence는 백슬래시만 남김
                result.push_str("\\u");
                result.push_str(&hex_digits);
                continue;
              }
            };

            // Surrogate pair handling: \uXXXX\uYYYY
            if (0xD800..=0xDBFF).contains(&code_point) {
              let mut lookahead = chars.clone();
              if lookahead.next() == Some('\\') && lookahead.next() == Some('u') {
                if let Some(low_digits) = read_hex4_iter(&mut lookahead) {
                  if let Ok(low_value) = u32::from_str_radix(&low_digits, 16) {
                    if (0xDC00..=0xDFFF).contains(&low_value) {
                      let high = code_point - 0xD800;
                      let low = low_value - 0xDC00;
                      let combined = 0x10000 + ((high << 10) | low);
                      if let Some(unicode_ch) = char::from_u32(combined) {
                        chars.next(); // consume '\\'
                        chars.next(); // consume 'u'
                        for _ in 0..4 {
                          chars.next();
                        }
                        result.push(unicode_ch);
                        continue;
                      }
                    }
                  }
                }
              }

              result.push_str("\\u");
              result.push_str(&hex_digits);
              continue;
            }

            // LOW: unescape_json_string lone surrogate 미검증 수정
            // 0xD800-0xDBFF (high surrogate)는 이미 위에서 처리됨
            // 0xDC00-0xDFFF (low surrogate)는 유효한 서로게이트 페어가 아니므로 검증 필요
            // char::from_u32에서 실패하므로 명시적으로 검증하여 원본 보존
            if (0xDC00..=0xDFFF).contains(&code_point) {
              // Lone low surrogate: 유효한 서로게이트 페어가 아닌 경우
              // 원본 escape sequence를 보존하여 데이터 손실 방지
              result.push_str("\\u");
              result.push_str(&hex_digits);
            } else if let Some(unicode_ch) = char::from_u32(code_point) {
              result.push(unicode_ch);
            } else {
              // 유효하지 않은 code point: 원본 보존
              result.push_str("\\u");
              result.push_str(&hex_digits);
            }
          }
          // LOW: 알 수 없는 escape sequence silent 통과 수정
          // 알 수 없는 escape sequence에 대해 경고 로그 출력
          // JSON 표준에 따라 유효한 escape만 허용하되, 호환성을 위해 원본 보존
          other => {
            // 알 수 없는 escape sequence: 경고 출력 후 원본 보존
            eprintln!("Warning: Unknown escape sequence \\{} in JSON string (valid escapes: \\\", \\\\, \\n, \\r, \\t, \\b, \\f, \\uXXXX)", other);
            result.push('\\');
            result.push(other);
            chars.next();
          }
        }
      } else {
        result.push(ch);
      }
    } else {
      result.push(ch);
    }
  }

  result
}

/// HTML 특수 문자 이스케이프
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn escape_html(s: &str) -> String {
  let needs_escape = s.chars().any(|c| matches!(c, '<' | '>' | '&' | '"' | '\''));

  if !needs_escape {
    return s.to_string();
  }

  let mut escaped = String::with_capacity(s.len() + s.len() / 10);

  for ch in s.chars() {
    match ch {
      '<' => escaped.push_str("&lt;"),
      '>' => escaped.push_str("&gt;"),
      '&' => escaped.push_str("&amp;"),
      '"' => escaped.push_str("&quot;"),
      '\'' => escaped.push_str("&#39;"),
      _ => escaped.push(ch),
    }
  }

  escaped
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_escape_simple() {
    let input = r#"{"name":"test"}"#;
    let output = escape_json_for_string(input);
    assert_eq!(output, r#"{\"name\":\"test\"}"#);
  }

  #[test]
  fn test_escape_with_newline() {
    let input = "{\"name\":\"test\nvalue\"}";
    let output = escape_json_for_string(input);
    assert_eq!(output, "{\\\"name\\\":\\\"test\\nvalue\\\"}");
  }

  #[test]
  fn test_escape_no_escape_needed() {
    let input = "{name:test}";
    let output = escape_json_for_string(input);
    assert_eq!(output, "{name:test}");
  }

  #[test]
  fn test_unescape() {
    let input = r#"{\"name\":\"test\"}"#;
    let output = unescape_json_string(input);
    assert_eq!(output, r#"{"name":"test"}"#);
  }

  #[test]
  fn test_unescape_newline() {
    let input = r#"line1\nline2"#;
    let output = unescape_json_string(input);
    assert_eq!(output, "line1\nline2");
  }

  #[test]
  fn test_escape_html() {
    let input = "<div class=\"test\">&</div>";
    let output = escape_html(input);
    assert_eq!(
      output,
      "&lt;div class=&quot;test&quot;&gt;&amp;&lt;/div&gt;"
    );
  }

  #[test]
  fn test_escape_html_no_escape_needed() {
    let input = "plain text";
    let output = escape_html(input);
    assert_eq!(output, "plain text");
  }

  #[test]
  fn test_escape_control_characters() {
    // RFC 8259에 따른 제어 문자 escape 테스트
    let input = "\x00\x01\x08\x0C\x1F"; // NUL, SOH, BS, FF, US
    let output = escape_json_for_string(input);
    assert_eq!(output, "\\u0000\\u0001\\b\\f\\u001F");
  }

  #[test]
  fn test_escape_backspace_and_formfeed() {
    let input = "test\x08\x0Cvalue"; // backspace, form feed
    let output = escape_json_for_string(input);
    assert_eq!(output, "test\\b\\fvalue");
  }

  #[test]
  fn test_unescape_control_characters() {
    let input = "\\u0000\\u0001\\b\\f\\u001F";
    let output = unescape_json_string(input);
    assert_eq!(output, "\x00\x01\x08\x0C\x1F");
  }

  #[test]
  fn test_unescape_backspace_and_formfeed() {
    let input = "test\\b\\fvalue";
    let output = unescape_json_string(input);
    assert_eq!(output, "test\x08\x0Cvalue");
  }

  #[test]
  fn test_unescape_surrogate_pair() {
    let input = "\\uD83D\\uDE00";
    let output = unescape_json_string(input);
    assert_eq!(output, "\u{1F600}");
  }

  #[test]
  fn test_unescape_invalid_surrogate_pair() {
    let input = "\\uD83D\\u0041";
    let output = unescape_json_string(input);
    assert_eq!(output, "\\uD83DA");
  }

  #[test]
  fn test_unescape_partial_unicode_escape() {
    let input = "\\u12";
    let output = unescape_json_string(input);
    assert_eq!(output, "\\u12");
  }
}
