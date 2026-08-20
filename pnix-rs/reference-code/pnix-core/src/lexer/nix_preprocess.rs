//! Nix 전처리 모듈
//!
//! pnix-old의 nix_preprocess/src/lib.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 텍스트 변환만 수행, 값 계산 없음
//!
//! ## 기능
//!
//! `clj { ... }`, `py { ... }`, `js { ... }`, `ts { ... }`, `tsx { ... }` 블록을
//! `builtins.*` 호출로 변환합니다.

/// Nix 코드 전처리 - 언어 블록을 builtins.* 호출로 변환
///
/// Transforms:
/// - `clj { (def a 3) }` → `clj ''(def a 3)''`
/// - `py { x = 10 }` → `builtins.py-inject ''x = 10'' []`
/// - `js { const x = 1 + 2; }` → `builtins.js-interpret ''const x = 1 + 2;''`
/// - `ts { const x: number = 1; }` → `builtins.ts-interpret ''const x: number = 1;''`
/// - `tsx { <div>Hello</div> }` → `builtins.tsx-interpret ''<div>Hello</div>''`
/// - `#clj (+ 1 2)` → `builtins.clj-eval "(+ 1 2)"`
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 변환만, 파일 I/O 없음
pub fn preprocess_nix_code(input: &str) -> String {
  let mut output = String::with_capacity(input.len() + 100);
  let chars: Vec<(usize, char)> = input.char_indices().collect();
  let mut i = 0;

  while i < chars.len() {
    let (byte_pos, ch) = chars[i];

    // Check for `clj {`, `py {`, `js {`, `ts {`, `tsx {` pattern
    let is_clj = matches_keyword(&chars, i, "clj")
      && (i == 0 || !is_ident_char(chars[i - 1].1))
      && (i + 3 >= chars.len() || !is_ident_char(chars[i + 3].1));
    let is_py = matches_keyword(&chars, i, "py")
      && (i == 0 || (!is_ident_char(chars[i - 1].1) && chars[i - 1].1 != '.'))
      && (i + 2 >= chars.len() || !is_ident_char(chars[i + 2].1));
    let is_tsx = matches_keyword(&chars, i, "tsx")
      && (i == 0 || !is_ident_char(chars[i - 1].1))
      && (i + 3 >= chars.len() || !is_ident_char(chars[i + 3].1));
    let is_ts = !is_tsx
      && matches_keyword(&chars, i, "ts")
      && (i == 0 || !is_ident_char(chars[i - 1].1))
      && (i + 2 >= chars.len() || !is_ident_char(chars[i + 2].1));
    let is_js = matches_keyword(&chars, i, "js")
      && (i == 0 || !is_ident_char(chars[i - 1].1))
      && (i + 2 >= chars.len() || !is_ident_char(chars[i + 2].1));

    if is_clj || is_py || is_js || is_ts || is_tsx {
      // Found language keyword
      let lang = if is_py {
        "py"
      } else if is_tsx {
        "tsx"
      } else if is_ts {
        "ts"
      } else if is_js {
        "js"
      } else {
        "clj"
      };

      i += lang.chars().count();

      // Skip whitespace
      while i < chars.len() && chars[i].1.is_ascii_whitespace() {
        i += 1;
      }

      // Check for '{'
      if i < chars.len() && chars[i].1 == '{' {
        i += 1; // skip '{'

        // Collect the block body
        let (body, new_i) = collect_balanced_chars(&chars, i, '{', '}');
        i = new_i;

        // Transform based on language
        if is_py {
          output.push_str("builtins.py-inject ''");
          output.push_str(&escape_nix_string(&body));
          output.push_str("'' []");
        } else if is_js {
          output.push_str("builtins.js-interpret ''");
          output.push_str(&escape_nix_string(&body));
          output.push_str("''");
        } else if is_ts {
          output.push_str("builtins.ts-interpret ''");
          output.push_str(&escape_nix_string(&body));
          output.push_str("''");
        } else if is_tsx {
          output.push_str("builtins.tsx-interpret ''");
          output.push_str(&escape_nix_string(&body));
          output.push_str("''");
        } else {
          // Clojure
          output.push_str("clj ''");
          output.push_str(&escape_nix_string(&body));
          output.push_str("''");
        }
        continue;
      } else {
        // Not followed by '{', output keyword as-is
        output.push_str(lang);
        continue;
      }
    }

    // Check for `#clj (` pattern
    if i + 3 < chars.len()
      && ch == '#'
      && chars[i + 1].1 == 'c'
      && chars[i + 2].1 == 'l'
      && chars[i + 3].1 == 'j'
    {
      i += 4; // skip "#clj"

      // Skip whitespace
      while i < chars.len() && chars[i].1.is_ascii_whitespace() {
        i += 1;
      }

      // Check for '('
      if i < chars.len() && chars[i].1 == '(' {
        i += 1; // skip '('

        let (body, new_i) = collect_balanced_chars(&chars, i, '(', ')');
        i = new_i;

        output.push_str("builtins.clj-eval \"(");
        output.push_str(&escape_nix_string(body.trim()));
        output.push_str(")\"");
        continue;
      } else {
        output.push_str("#clj");
        continue;
      }
    }

    // Regular character
    let next_byte_pos = if i + 1 < chars.len() {
      chars[i + 1].0
    } else {
      input.len()
    };
    output.push_str(&input[byte_pos..next_byte_pos]);
    i += 1;
  }

  output
}

/// Collect characters until balanced closing delimiter
fn collect_balanced_chars(
  chars: &[(usize, char)],
  start: usize,
  open: char,
  close: char,
) -> (String, usize) {
  let mut result = String::new();
  let mut depth = 1;
  let mut i = start;

  while i < chars.len() {
    let (_byte_pos, ch) = chars[i];

    if ch == open {
      depth += 1;
      result.push(ch);
    } else if ch == close {
      depth -= 1;
      if depth == 0 {
        i += 1;
        break;
      }
      result.push(ch);
    } else {
      result.push(ch);
    }

    i += 1;
  }

  (result, i)
}

/// Escape string for Nix string literal
///
/// 단일 패스로 처리하여 double-escaping 방지
fn escape_nix_string(s: &str) -> String {
  let mut result = String::with_capacity(s.len() + s.len() / 10);
  for ch in s.chars() {
    match ch {
      '\\' => result.push_str("\\\\"),
      '"' => result.push_str("\\\""),
      '\n' => result.push_str("\\n"),
      '\t' => result.push_str("\\t"),
      '\r' => result.push_str("\\r"),
      _ => result.push(ch),
    }
  }
  result
}

fn is_ident_char(ch: char) -> bool {
  ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'
}

fn matches_keyword(chars: &[(usize, char)], start: usize, keyword: &str) -> bool {
  let kw_len = keyword.chars().count();
  if start + kw_len > chars.len() {
    return false;
  }
  keyword
    .chars()
    .enumerate()
    .all(|(idx, kw_ch)| chars[start + idx].1 == kw_ch)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_preprocess_clj_block() {
    let input = "clj { (def a 3) }";
    let output = preprocess_nix_code(input);
    assert!(output.contains("clj ''"));
    assert!(output.contains("(def a 3)"));
  }

  #[test]
  fn test_preprocess_py_block() {
    let input = "py { x = 10 }";
    let output = preprocess_nix_code(input);
    assert!(output.contains("builtins.py-inject"));
    assert!(output.contains("x = 10"));
  }

  #[test]
  fn test_preprocess_js_block() {
    let input = "js { const x = 1 + 2; }";
    let output = preprocess_nix_code(input);
    assert!(output.contains("builtins.js-interpret"));
    assert!(output.contains("const x = 1 + 2"));
  }

  #[test]
  fn test_preprocess_clj_eval() {
    let input = "#clj (+ 1 2)";
    let output = preprocess_nix_code(input);
    assert!(output.contains("builtins.clj-eval"));
    assert!(output.contains("(+ 1 2)"));
  }
}
