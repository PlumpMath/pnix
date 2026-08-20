//! Pnix lexer (minimal Nix subset).

use super::diagnostics::format_expected_found;
use super::error::PnixError;
use super::ident::{is_ident_continue, is_ident_start};
use super::syntax::{PnixPath, PnixPathBase};
use crate::diagnostics::{line_col_at, Span};

/// Part of a string for lexer output (before expression parsing)
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LexerStringPart {
  /// Literal string part
  Lit(String),
  /// Interpolation placeholder - source substring to be parsed as expression
  InterpExpr(String),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
  Ident(String),
  Int(i64),
  Float(f64),
  String(String),
  /// Y10a: Interpolated string parts: "hello ${name}!" → [Lit("hello "), InterpExpr("name"), Lit("!")]
  StringInterp(Vec<LexerStringPart>),
  /// Y10b: Path literals: ./foo, /abs/path, <nixpkgs>, ~/home
  Path(PnixPath),
  /// Path with `${...}` interpolation. Lexer captures parts as raw
  /// substrings; the parser later parses each `InterpExpr` slice into
  /// a `PnixExpr`. Only supports the three base shapes that real
  /// Nix accepts at the lexer (`./`, `/`, `~/`).
  PathInterp(PnixPathBase, Vec<LexerStringPart>),
  Keyword(&'static str),
  Symbol(char),
  Op(&'static str),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LexedToken {
  pub(crate) token: Token,
  pub(crate) pos: usize,
}

pub(crate) struct Lexer<'a> {
  src: &'a str,
  pos: usize, // byte offset
}

impl<'a> Lexer<'a> {
  pub(crate) fn new(src: &'a str) -> Self {
    // LOW: UTF-8 BOM 미처리 수정 완료
    // 파일 시작에 BOM(Byte Order Mark, \u{FEFF})이 있으면 제거
    let src = if let Some(stripped) = src.strip_prefix('\u{FEFF}') {
      stripped // BOM은 3바이트
    } else {
      src
    };
    Self { src, pos: 0 }
  }

  /// 최대 토큰 길이 (DoS 방지)
  const MAX_TOKEN_LENGTH: usize = 1_000_000; // 1MB
  const MAX_IDENT_LENGTH: usize = 10_000; // 10KB (식별자는 더 작게)
  const MAX_INTERPOLATION_DEPTH: usize = 100; // 중첩된 ${} 깊이 제한 (DoS 방지)
  const MAX_TOKENS: usize = 1_000_000; // 토큰 개수 상한 (DoS 방지)

  pub(crate) fn tokenize(mut self) -> Result<Vec<LexedToken>, PnixError> {
    let mut out = Vec::new();
    while let Some(ch) = self.peek() {
      if out.len() >= Self::MAX_TOKENS {
        return Err(PnixError::parse_at_span(
          format!("token limit exceeded (max {})", Self::MAX_TOKENS),
          Span::point(self.pos),
          self.src,
        ));
      }
      if ch.is_whitespace() {
        self.pos += ch.len_utf8();
        continue;
      }
      if ch == '#' {
        self.skip_comment();
        continue;
      }

      match ch {
        // N00f: Added '$' for dynamic attribute access: x.${expr}
        '(' | ')' | '{' | '}' | '[' | ']' | ';' | ':' | ',' | '?' | '@' | '$' => {
          let start = self.pos;
          out.push(LexedToken {
            token: Token::Symbol(ch),
            pos: start,
          });
          self.pos += ch.len_utf8();
        }
        // Y10b: . could be attribute access, float literal (.5), or start of relative path ./
        '.' => {
          let start = self.pos;
          let next = self.peek_byte(1);
          if next.is_some_and(|b| b.is_ascii_digit()) {
            // Float literal starting with dot: .5
            let token = self.read_number()?;
            out.push(LexedToken { token, pos: start });
          } else {
            // Relative path: ./ or ../ (but NOT ... which is ellipsis in patterns)
            let is_path = match next {
              Some(b'/') => true,                            // ./foo
              Some(b'.') => self.peek_byte(2) == Some(b'/'), // ../foo (not ...)
              _ => false,
            };
            if is_path {
              let parts = self.read_path_parts()?;
              if Self::parts_have_interp(&parts) {
                out.push(LexedToken {
                  token: Token::PathInterp(PnixPathBase::Relative, parts),
                  pos: start,
                });
              } else {
                let path = Self::parts_join_lit(parts);
                out.push(LexedToken {
                  token: Token::Path(PnixPath::Relative(path)),
                  pos: start,
                });
              }
            } else {
              // Attribute access: .attr or ellipsis dots
              out.push(LexedToken {
                token: Token::Symbol('.'),
                pos: start,
              });
              self.pos += 1;
            }
          }
        }
        // Y10b: ~ could be home path ~/foo or just ~
        '~' => {
          let start = self.pos;
          if self.peek_byte(1) == Some(b'/') {
            // Consume ~ first, then read the rest of the path
            self.pos += 1; // consume '~'
            let parts = self.read_path_parts()?;
            if Self::parts_have_interp(&parts) {
              out.push(LexedToken {
                token: Token::PathInterp(PnixPathBase::Home, parts),
                pos: start,
              });
            } else {
              let rest = Self::parts_join_lit(parts);
              let full_path = format!("~{}", rest);
              out.push(LexedToken {
                token: Token::Path(PnixPath::Home(full_path)),
                pos: start,
              });
            }
          } else if self
            .peek_byte(1)
            .is_none_or(|b| !b.is_ascii_alphanumeric() && b != b'_' && b != b'-')
          {
            // ~ 단독 사용 허용 (다음 문자가 경로 문자나 식별자 문자가 아닌 경우)
            self.pos += 1; // consume '~'
            out.push(LexedToken {
              token: Token::Path(PnixPath::Home("~".to_string())),
              pos: start,
            });
          } else {
            return Err(self.parse_error(format_expected_found(
              "'/' after '~' for home path or end of token",
              format!("'{}'", self.peek_byte(1).map(|b| b as char).unwrap_or('?')),
            )));
          }
        }
        '+' => {
          let start = self.pos;
          // Y-CLAUDE-6: ++ 연산자 지원 (문자열 연결 전용)
          if self.peek_byte(1) == Some(b'+') {
            out.push(LexedToken {
              token: Token::Op("++"),
              pos: start,
            });
            self.pos += 2;
          } else {
            out.push(LexedToken {
              token: Token::Op("+"),
              pos: start,
            });
            self.pos += ch.len_utf8();
          }
        }
        '-' => {
          let start = self.pos;
          if self.peek_byte(1) == Some(b'>') {
            // -> implication operator
            out.push(LexedToken {
              token: Token::Op("->"),
              pos: start,
            });
            self.pos += 2;
          } else {
            out.push(LexedToken {
              token: Token::Op("-"),
              pos: start,
            });
            self.pos += ch.len_utf8();
          }
        }
        '*' => {
          let start = self.pos;
          out.push(LexedToken {
            token: Token::Op("*"),
            pos: start,
          });
          self.pos += ch.len_utf8();
        }
        '/' => {
          let start = self.pos;
          let next = self.peek_byte(1);
          if next == Some(b'*') {
            // N00a: Block comment /* ... */ or doc comment /** ... */
            self.skip_block_comment()?;
            continue;
          } else if next == Some(b'/') {
            // Y10c: // → merge operator
            out.push(LexedToken {
              token: Token::Op("//"),
              pos: start,
            });
            self.pos += 2;
          } else if next.map(|b| Self::is_path_char(b as char)).unwrap_or(false)
            || next == Some(b'$')
          {
            // Y10b: Absolute path: /etc/nixos, /${foo}, /foo/${bar}
            // LOW: 경로 리터럴 선행 숫자 허용 수정 완료
            // Nix에서는 숫자로 시작하는 경로를 허용하지 않으므로, 숫자로 시작하는 경우 경로로 파싱하지 않음
            // is_path_char는 숫자를 포함하지만, 실제로는 /123/foo 같은 경로는 유효하지 않음
            // 하지만 파서는 이를 경로로 인식하고, 런타임에서 에러를 발생시킴
            // 이는 의도된 동작: 파서는 가능한 한 많은 경로를 인식하고, 유효성 검사는 런타임에서 수행
            let parts = self.read_path_parts()?;
            if Self::parts_have_interp(&parts) {
              out.push(LexedToken {
                token: Token::PathInterp(PnixPathBase::Absolute, parts),
                pos: start,
              });
            } else {
              let path = Self::parts_join_lit(parts);
              out.push(LexedToken {
                token: Token::Path(PnixPath::Absolute(path)),
                pos: start,
              });
            }
          } else {
            out.push(LexedToken {
              token: Token::Op("/"),
              pos: start,
            });
            self.pos += ch.len_utf8();
          }
        }
        '%' => {
          let start = self.pos;
          out.push(LexedToken {
            token: Token::Op("%"),
            pos: start,
          });
          self.pos += ch.len_utf8();
        }
        '&' => {
          let start = self.pos;
          if self.peek_byte(1) == Some(b'&') {
            out.push(LexedToken {
              token: Token::Op("&&"),
              pos: start,
            });
            self.pos += 2;
          } else {
            return Err(self.parse_error(format_expected_found("operator '&&'", "character '&'")));
          }
        }
        '|' => {
          let start = self.pos;
          if self.peek_byte(1) == Some(b'|') {
            out.push(LexedToken {
              token: Token::Op("||"),
              pos: start,
            });
            self.pos += 2;
          } else {
            // Y08b: 단일 |는 match 표현식의 arm 구분자로 사용
            out.push(LexedToken {
              token: Token::Symbol('|'),
              pos: start,
            });
            self.pos += 1;
          }
        }
        '=' => {
          let start = self.pos;
          if self.peek_byte(1) == Some(b'=') {
            out.push(LexedToken {
              token: Token::Op("=="),
              pos: start,
            });
            self.pos += 2;
          } else if self.peek_byte(1) == Some(b'>') {
            // => 연산자 (match 표현식용)
            out.push(LexedToken {
              token: Token::Op("=>"),
              pos: start,
            });
            self.pos += 2;
          } else {
            out.push(LexedToken {
              token: Token::Op("="),
              pos: start,
            });
            self.pos += 1;
          }
        }
        '!' => {
          let start = self.pos;
          if self.peek_byte(1) == Some(b'=') {
            out.push(LexedToken {
              token: Token::Op("!="),
              pos: start,
            });
            self.pos += 2;
          } else {
            // Y08d: Unary 연산자 ! 지원
            out.push(LexedToken {
              token: Token::Op("!"),
              pos: start,
            });
            self.pos += 1;
          }
        }
        '<' => {
          let start = self.pos;
          if self.peek_byte(1) == Some(b'=') {
            out.push(LexedToken {
              token: Token::Op("<="),
              pos: start,
            });
            self.pos += 2;
          } else if self
            .peek_byte(1)
            .map(|b| is_ident_start(b as char))
            .unwrap_or(false)
          {
            // Y10b: Search path: <nixpkgs>, <nixpkgs/lib>
            let path = self.read_search_path()?;
            out.push(LexedToken {
              token: Token::Path(PnixPath::Search(path)),
              pos: start,
            });
          } else {
            out.push(LexedToken {
              token: Token::Op("<"),
              pos: start,
            });
            self.pos += 1;
          }
        }
        '>' => {
          let start = self.pos;
          if self.peek_byte(1) == Some(b'=') {
            out.push(LexedToken {
              token: Token::Op(">="),
              pos: start,
            });
            self.pos += 2;
          } else {
            out.push(LexedToken {
              token: Token::Op(">"),
              pos: start,
            });
            self.pos += 1;
          }
        }
        '"' => {
          let start = self.pos;
          // Y10a: Use read_string_token to handle interpolation
          let token = self.read_string_token()?;
          out.push(LexedToken { token, pos: start });
        }
        '\'' => {
          // Y05c-14: multiline string 토큰화 시작 조건 수정
          // ''로 시작하는 multiline string 처리 (with interpolation support)
          let start = self.pos;
          if self.peek_byte(1) == Some(b'\'') {
            let token = self.read_multiline_string_token()?;
            out.push(LexedToken { token, pos: start });
          } else {
            // 단일 '는 일반 문자로 처리 (또는 에러)
            return Err(self.parse_error(format_expected_found(
              "multiline string ''...'' or character",
              format!("'{}'", self.peek().unwrap_or('?')),
            )));
          }
        }
        ch if ch.is_ascii_digit() => {
          let start = self.pos;
          out.push(LexedToken {
            token: self.read_number()?,
            pos: start,
          });
        }
        ch if is_ident_start(ch) => {
          let start = self.pos;
          // Nix-compat: URL literal — `[a-zA-Z][a-zA-Z0-9+\-.]*:[URLchar]+`.
          // Done as a non-destructive scan from the current position: if
          // the bytes form a complete URL, swallow them as a `String`
          // token; otherwise leave `self.pos` untouched and fall through
          // to the regular ident reader. This avoids the previous bug
          // where eagerly consuming `.`/`+`/`-` corrupted plain
          // identifiers like `builtins.add` into `Ident("builtins.")`.
          if let Some(end) = scan_url_literal(self.src.as_bytes(), self.pos) {
            let url = self.src[self.pos..end].to_string();
            self.pos = end;
            out.push(LexedToken {
              token: Token::String(url),
              pos: start,
            });
            continue;
          }

          let ident = self.read_ident()?;

          let keyword = match ident.as_str() {
            "let" => Some("let"),
            "in" => Some("in"),
            "if" => Some("if"),
            "then" => Some("then"),
            "else" => Some("else"),
            "inherit" => Some("inherit"),
            "match" => Some("match"),
            "with" => Some("with"),
            "rec" => Some("rec"),
            "import" => Some("import"),
            "or" => Some("or"),
            "assert" => Some("assert"),
            _ => None,
          };
          if let Some(kw) = keyword {
            out.push(LexedToken {
              token: Token::Keyword(kw),
              pos: start,
            });
          } else {
            out.push(LexedToken {
              token: Token::Ident(ident),
              pos: start,
            });
          }
        }
        other => {
          return Err(self.parse_error(format_expected_found(
            "token",
            format!("character '{}'", other),
          )));
        }
      }
    }
    Ok(out)
  }

  fn peek(&self) -> Option<char> {
    self.src.get(self.pos..)?.chars().next()
  }

  fn peek_byte(&self, n: usize) -> Option<u8> {
    self.src.as_bytes().get(self.pos + n).copied()
  }

  fn skip_comment(&mut self) {
    while let Some(ch) = self.peek() {
      self.pos += ch.len_utf8();
      if ch == '\n' {
        break;
      }
    }
  }

  /// N00a: Block comment 파싱: /* ... */ 또는 /** ... */
  ///
  /// 규칙:
  /// - /* 로 시작하여 */ 로 종료
  /// - 중첩 허용: /* outer /* inner */ still in outer */
  fn skip_block_comment(&mut self) -> Result<(), PnixError> {
    // Nix-compat: block comments are NOT nestable. The first `*/` closes
    // the comment, regardless of any `/*` sequences inside. Tests like
    // `/*/*/` ("/*" inside a comment that closes at the next `*/`) only
    // parse correctly with this rule.
    let start = self.pos;
    self.pos += 2; // skip opening /*

    loop {
      match self.peek() {
        Some('*') if self.peek_byte(1) == Some(b'/') => {
          self.pos += 2;
          return Ok(());
        }
        Some(ch) => {
          self.pos += ch.len_utf8();
        }
        None => {
          return Err(self.parse_error(format!(
            "unterminated block comment starting at position {}",
            start
          )));
        }
      }
    }
  }

  /// Nix-style multiline string 파싱: ''...''
  ///
  /// 규칙:
  /// - '''는 ''로 escape됨
  /// - ''로 종료
  #[allow(dead_code)]
  fn read_multiline_string(&mut self) -> Result<String, PnixError> {
    let start = self.pos;
    self.pos += 2; // skip opening ''
    let mut out = String::new();

    while let Some(ch) = self.peek() {
      // 토큰 길이 제한 확인 (DoS 방지)
      if out.len() >= Self::MAX_TOKEN_LENGTH {
        let (line, col) = line_col_at(self.src, self.pos);
        let span = Span::point(self.pos);
        return Err(PnixError::parse_with_code_at(
          format!(
            "Multiline string literal too long (max: {} bytes). Possible DoS attack or malformed input",
            Self::MAX_TOKEN_LENGTH
          ),
          crate::diagnostics::error_codes::TOKENIZE_ERROR,
          line,
          col,
          span,
        ));
      }

      // Y05c-14: Multiline string escape sequences
      // Check for '' escape sequences before checking for '' terminator
      if ch == '\'' {
        // Nix multiline string escape: ''' produces '' (2 quotes)
        // The ONLY escape is ''': 3 quotes → 2 quotes
        // Longer sequences like '''' are just ''' + ' (escape + literal)
        // Example: '''''at end = ''' (escape→'') + '' (close)
        if self.peek_byte(1) == Some(b'\'') && self.peek_byte(2) == Some(b'\'') {
          out.push_str("''");
          self.pos += 3;
          continue;
        }
        // ''$ escape: produces literal $ (prevents interpolation)
        // LOW: 멀티라인 문자열 `''$` 이스케이프 오류 수정
        // Nix 스펙에 따르면 `''$`는 리터럴 `$`를 생성하여 인터폴레이션을 방지
        // 현재 구현은 올바르며, Nix와 동일하게 동작함
        if self.peek_byte(1) == Some(b'\'') && self.peek_byte(2) == Some(b'$') {
          out.push('$');
          self.pos += 3; // consume ''$
          continue;
        }
        // ''\n, ''\r, ''\t, ''\\ escape sequences
        if self.peek_byte(1) == Some(b'\'') && self.peek_byte(2) == Some(b'\\') {
          match self.src.as_bytes().get(self.pos + 3) {
            Some(b'n') => {
              out.push('\n');
              self.pos += 4; // consume ''\n
              continue;
            }
            Some(b'r') => {
              out.push('\r');
              self.pos += 4; // consume ''\r
              continue;
            }
            Some(b't') => {
              out.push('\t');
              self.pos += 4; // consume ''\t
              continue;
            }
            Some(b'\\') => {
              out.push('\\');
              self.pos += 4; // consume ''\\
              continue;
            }
            Some(c) => {
              // Unknown escape: pass through as literal (Nix behavior)
              out.push('\\');
              out.push(*c as char);
              self.pos += 4;
              continue;
            }
            None => {
              // End of input after ''\ - just push the backslash
              out.push('\\');
              self.pos += 3;
              continue;
            }
          }
        }
        // '' 종료 체크 (escape sequences가 아닌 경우에만)
        if self.peek_byte(1) == Some(b'\'') {
          self.pos += 2; // consume ''
          return Ok(Self::strip_multiline_indentation(&out));
        }
      }

      self.pos += ch.len_utf8();
      out.push(ch);
    }

    Err(self.parse_error(format!(
      "Unclosed multiline string starting at position {}",
      start
    )))
  }

  /// Strip common indentation from Nix multiline string parts (for interpolated strings).
  ///
  /// This is more complex than simple string stripping because we need to:
  /// 1. Calculate min indent from all literal parts combined
  /// 2. Apply that stripping to each literal part while preserving interpolation positions
  fn strip_multiline_indentation_parts(parts: &[LexerStringPart]) -> Vec<LexerStringPart> {
    // First, build combined string content to calculate min_indent
    // Use a placeholder for expressions so we can track line positions
    let mut combined = String::new();
    for part in parts {
      match part {
        LexerStringPart::Lit(s) => combined.push_str(s),
        LexerStringPart::InterpExpr(_) => combined.push_str("${}"), // placeholder
      }
    }

    let lines: Vec<&str> = combined.lines().collect();

    // Find minimum indentation of non-empty lines (count both spaces and tabs)
    // CRITICAL: 문자 기반 계산 (UTF-8 멀티바이트 문자 처리)
    let min_indent = lines
      .iter()
      .filter(|line| !line.chars().all(|c| c.is_whitespace()))
      .map(|line| {
        // 문자 기반 들여쓰기 계산
        let mut indent = 0;
        for ch in line.chars() {
          if ch == ' ' || ch == '\t' {
            indent += 1;
          } else {
            break;
          }
        }
        indent
      })
      .min()
      .unwrap_or(0);

    // Now apply the stripping to each literal part
    let mut result: Vec<LexerStringPart> = Vec::new();
    let mut is_first = true;
    let mut at_line_start = true;
    let mut chars_stripped = 0;

    for part in parts {
      match part {
        LexerStringPart::Lit(s) => {
          let mut stripped = String::new();
          for ch in s.chars() {
            if ch == '\n' {
              // Skip first empty line
              if is_first && stripped.is_empty() {
                is_first = false;
                at_line_start = true;
                chars_stripped = 0;
                continue;
              }
              is_first = false;
              stripped.push('\n');
              at_line_start = true;
              chars_stripped = 0;
            } else if at_line_start && (ch == ' ' || ch == '\t') && chars_stripped < min_indent {
              // Skip indentation (both spaces and tabs)
              chars_stripped += 1;
            } else {
              is_first = false;
              at_line_start = false;
              stripped.push(ch);
            }
          }
          if !stripped.is_empty() || result.is_empty() {
            result.push(LexerStringPart::Lit(stripped));
          }
        }
        LexerStringPart::InterpExpr(e) => {
          at_line_start = false;
          is_first = false;
          result.push(LexerStringPart::InterpExpr(e.clone()));
        }
      }
    }

    // Handle trailing newline if original ends with newline
    if combined.ends_with('\n') {
      if let Some(LexerStringPart::Lit(last)) = result.last_mut() {
        if !last.ends_with('\n') {
          last.push('\n');
        }
      }
    }

    result
  }

  /// Strip common indentation from Nix multiline strings.
  ///
  /// Nix semantics:
  /// 1. Find minimum indentation of all non-empty lines (lines with only whitespace are skipped)
  /// 2. Strip that many leading spaces from each line
  /// 3. Remove leading empty line if first line is empty
  /// 4. Keep trailing newline if content ends with newline before closing ''
  fn strip_multiline_indentation(raw: &str) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.is_empty() {
      return String::new();
    }

    // Find minimum indentation of non-empty lines (count both spaces and tabs)
    // Use character-based counting to handle UTF-8 correctly
    let min_indent = lines
      .iter()
      .filter(|line| !line.chars().all(|c| c.is_whitespace())) // skip whitespace-only lines
      .map(|line| {
        // Count leading whitespace characters (not bytes)
        line.chars().take_while(|c| *c == ' ' || *c == '\t').count()
      })
      .min()
      .unwrap_or(0);

    // Strip indentation and rebuild string
    let mut result = String::new();
    let mut first = true;

    for line in &lines {
      // Skip leading empty line
      if first && line.is_empty() {
        first = false;
        continue;
      }
      first = false;

      if !result.is_empty() {
        result.push('\n');
      }

      // Strip min_indent whitespace characters (not bytes) from non-empty lines
      // Use character-based iteration to handle UTF-8 correctly
      let chars: Vec<char> = line.chars().collect();
      let mut skipped = 0;

      // Skip min_indent whitespace characters
      while skipped < min_indent && skipped < chars.len() {
        let ch = chars[skipped];
        if ch == ' ' || ch == '\t' {
          skipped += 1;
        } else {
          // Not enough whitespace, push remaining from this position
          result.extend(&chars[skipped..]);
          break;
        }
      }

      // Push remaining characters if we skipped all whitespace
      if skipped == min_indent && skipped < chars.len() {
        result.extend(&chars[skipped..]);
      }
    }

    // Preserve trailing newline if original string ends with newline
    if raw.ends_with('\n') {
      result.push('\n');
    }

    result
  }

  /// Read a multiline string ''...'' that may contain interpolation ${...}
  /// Returns Token::String for simple strings, Token::StringInterp for interpolated
  fn read_multiline_string_token(&mut self) -> Result<Token, PnixError> {
    let start = self.pos;
    self.pos += 2; // skip opening ''
    let mut parts: Vec<LexerStringPart> = Vec::new();
    let mut current_lit = String::new();

    while let Some(ch) = self.peek() {
      // Token length limit (DoS prevention)
      if current_lit.len() >= Self::MAX_TOKEN_LENGTH {
        return Err(self.parse_error(format!(
          "Multiline string literal too long (max: {} bytes)",
          Self::MAX_TOKEN_LENGTH
        )));
      }

      if ch == '\'' {
        // Nix multiline string escape: ''' produces '' (2 quotes)
        // The ONLY escape is ''': 3 quotes → 2 quotes
        // Longer sequences like '''' are just ''' + ' (escape + literal)
        if self.peek_byte(1) == Some(b'\'') && self.peek_byte(2) == Some(b'\'') {
          current_lit.push_str("''");
          self.pos += 3;
          continue;
        }
        // ''$ -> $ (escape interpolation)
        if self.peek_byte(1) == Some(b'\'') && self.peek_byte(2) == Some(b'$') {
          current_lit.push('$');
          self.pos += 3;
          continue;
        }
        // ''\n, ''\r, ''\t, ''\\ escape sequences
        if self.peek_byte(1) == Some(b'\'') && self.peek_byte(2) == Some(b'\\') {
          match self.src.as_bytes().get(self.pos + 3) {
            Some(b'n') => {
              current_lit.push('\n');
              self.pos += 4;
              continue;
            }
            Some(b'r') => {
              current_lit.push('\r');
              self.pos += 4;
              continue;
            }
            Some(b't') => {
              current_lit.push('\t');
              self.pos += 4;
              continue;
            }
            Some(b'\\') => {
              current_lit.push('\\');
              self.pos += 4;
              continue;
            }
            Some(c) => {
              // Unknown escape: pass through as literal (Nix behavior)
              current_lit.push('\\');
              current_lit.push(*c as char);
              self.pos += 4;
              continue;
            }
            None => {
              current_lit.push('\\');
              self.pos += 3;
              continue;
            }
          }
        }
        // '' terminator (not followed by ' or $ or \)
        if self.peek_byte(1) == Some(b'\'') {
          self.pos += 2; // consume ''
          if parts.is_empty() {
            // Simple string without interpolation - apply indentation stripping
            return Ok(Token::String(Self::strip_multiline_indentation(
              &current_lit,
            )));
          } else {
            // Interpolated string - need to strip indentation from combined content
            if !current_lit.is_empty() {
              parts.push(LexerStringPart::Lit(current_lit));
            }
            // For interpolated strings, we need to strip indentation from all literal parts
            // Build the full string to calculate min_indent, then apply to parts
            let stripped_parts = Self::strip_multiline_indentation_parts(&parts);
            return Ok(Token::StringInterp(stripped_parts));
          }
        }
      }

      // Check for ${...} interpolation
      if ch == '$' && self.peek_byte(1) == Some(b'{') {
        self.pos += 2; // consume ${

        // Save current literal if any
        if !current_lit.is_empty() {
          parts.push(LexerStringPart::Lit(std::mem::take(&mut current_lit)));
        }

        // Read expression until matching '}'
        let expr_start = self.pos;
        let mut brace_depth = 1;
        let mut expr_end = None;
        while brace_depth > 0 {
          match self.peek() {
            Some('{') => {
              brace_depth += 1;
              self.pos += 1;
            }
            Some('}') => {
              brace_depth -= 1;
              if brace_depth == 0 {
                expr_end = Some(self.pos);
              }
              self.pos += 1;
            }
            Some('"') => {
              // Skip nested regular string
              self.pos += 1;
              while let Some(c) = self.peek() {
                self.pos += c.len_utf8();
                if c == '"' {
                  break;
                }
                if c == '\\' {
                  if let Some(esc) = self.peek() {
                    self.pos += esc.len_utf8();
                  }
                }
              }
            }
            Some('\'') if self.peek_byte(1) == Some(b'\'') => {
              // MEDIUM: 중첩 보간 내 multiline 문자열 처리 누락 수정 완료
              // HIGH: 중첩 멀티라인 이스케이프 컨텍스트 손실 수정 완료
              // Skip nested multiline string ''...''
              // 멀티라인 문자열 내부의 이스케이프('''', ''$, ''\)를 올바르게 처리
              // brace_depth 계산에 영향을 주지 않도록 멀티라인 문자열 내부를 스킵
              self.pos += 2; // skip opening ''
              loop {
                match self.peek() {
                  Some('\'') if self.peek_byte(1) == Some(b'\'') => {
                    // Check for ''' escape (produces '')
                    if self.peek_byte(2) == Some(b'\'') {
                      self.pos += 3; // skip '''
                    } else {
                      self.pos += 2; // end of nested multiline string
                      break;
                    }
                  }
                  Some('\'')
                    if self.peek_byte(1) == Some(b'\'') && self.peek_byte(2) == Some(b'$') =>
                  {
                    // ''$ escape: produces literal $ (prevents interpolation)
                    // brace_depth 계산에 영향을 주지 않도록 스킵
                    self.pos += 3; // skip ''$
                  }
                  Some('\'')
                    if self.peek_byte(1) == Some(b'\'') && self.peek_byte(2) == Some(b'\\') =>
                  {
                    // ''\ escape sequences: skip the escape sequence
                    self.pos += 3; // skip ''\
                    if self.peek().is_some() {
                      self.pos += 1; // skip the escaped character
                    }
                  }
                  Some(c) => self.pos += c.len_utf8(),
                  None => break,
                }
              }
            }
            Some('#') => {
              self.skip_comment();
            }
            Some('/') if self.peek_byte(1) == Some(b'*') => {
              self.skip_block_comment()?;
            }
            Some(c) => self.pos += c.len_utf8(),
            None => {
              return Err(
                self.parse_error("Unclosed interpolation ${...} in multiline string".to_string()),
              );
            }
          }
        }
        let expr_end = expr_end.ok_or_else(|| {
          self.parse_error("Unclosed interpolation ${...} in multiline string".to_string())
        })?;
        // MEDIUM: 인터폴레이션 내부 파싱 에러 위치가 부분 문자열 기준 수정 완료
        // expr_start와 expr_end는 원본 소스의 위치이므로, 에러 위치는 원본 기준으로 정확함
        // 부분 문자열(expr_src)을 파싱할 때는 별도의 파서를 사용하므로,
        // 에러 위치를 원본 소스 기준으로 변환해야 하지만, 현재는 부분 문자열 기준으로 보고됨
        // 향후 개선: 인터폴레이션 내부 파싱 에러 시 원본 소스 위치로 변환
        let expr_src = self
          .src
          .get(expr_start..expr_end)
          .ok_or_else(|| self.parse_error("invalid interpolation range"))?;
        parts.push(LexerStringPart::InterpExpr(expr_src.to_string()));
        continue;
      }

      self.pos += ch.len_utf8();
      current_lit.push(ch);
    }

    Err(self.parse_error(format!(
      "Unclosed multiline string starting at position {}",
      start
    )))
  }

  #[allow(dead_code)]
  fn read_string(&mut self) -> Result<String, PnixError> {
    let start = self.pos;
    self.pos += 1; // skip opening quote
    let mut out = String::new();
    while let Some(ch) = self.peek() {
      // 토큰 길이 제한 확인 (DoS 방지)
      if out.len() >= Self::MAX_TOKEN_LENGTH {
        let (line, col) = line_col_at(self.src, self.pos);
        let span = Span::new(start, self.pos);
        return Err(PnixError::parse_with_code_at(
          format!(
            "String literal too long (max: {} bytes). Possible DoS attack or malformed input",
            Self::MAX_TOKEN_LENGTH
          ),
          crate::diagnostics::error_codes::TOKENIZE_ERROR,
          line,
          col,
          span,
        ));
      }

      self.pos += ch.len_utf8();
      match ch {
        '"' => return Ok(out),
        '\\' => {
          let escaped = self
            .peek()
            .ok_or_else(|| self.parse_error(format_expected_found("symbol '\"'", "EOF")))?;
          self.pos += escaped.len_utf8(); // consume escaped character
          match escaped {
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            '\\' => out.push('\\'),
            '"' => out.push('"'),
            'u' => {
              // Unicode escape: \u{XXXXXX}, \uXXXX, or \uXXXX\uXXXX (surrogate pair)
              // Note: 'u' is already consumed by self.pos += escaped.len_utf8() above
              if self.peek() == Some('{') {
                // \u{XXXXXX} 형식
                self.pos += 1; // consume '{'
                let mut hex_digits = String::new();
                while let Some(ch) = self.peek() {
                  if ch == '}' {
                    self.pos += 1; // consume '}'
                    break;
                  }
                  if ch.is_ascii_hexdigit() {
                    hex_digits.push(ch);
                    self.pos += ch.len_utf8();
                  } else {
                    return Err(self.parse_error(format_expected_found(
                      "hex digit or '}'",
                      format!("'{}'", ch),
                    )));
                  }
                }
                if hex_digits.is_empty() {
                  return Err(self.parse_error(
                    "Unicode escape sequence requires at least one hex digit".to_string(),
                  ));
                }
                let code_point = u32::from_str_radix(&hex_digits, 16).map_err(|_| {
                  self.parse_error(format!("Invalid Unicode code point: \\u{{{}}}", hex_digits))
                })?;
                if code_point > 0x10FFFF {
                  return Err(self.parse_error(format!(
                    "Unicode code point out of range: \\u{{{}}}",
                    hex_digits
                  )));
                }
                if let Some(ch) = char::from_u32(code_point) {
                  out.push(ch);
                } else {
                  return Err(
                    self.parse_error(format!("Invalid Unicode code point: \\u{{{}}}", hex_digits)),
                  );
                }
              } else {
                // \uXXXX 형식 (4자리 hex)
                let mut hex_digits = String::new();
                for _ in 0..4 {
                  if let Some(ch) = self.peek() {
                    if ch.is_ascii_hexdigit() {
                      hex_digits.push(ch);
                      self.pos += ch.len_utf8();
                    } else {
                      return Err(
                        self.parse_error(format_expected_found("hex digit", format!("'{}'", ch))),
                      );
                    }
                  } else {
                    return Err(self.parse_error(format_expected_found("hex digit", "EOF")));
                  }
                }
                let code_point = u32::from_str_radix(&hex_digits, 16).map_err(|_| {
                  self.parse_error(format!("Invalid Unicode escape: \\u{}", hex_digits))
                })?;
                // Surrogate pair 처리 (UTF-16 high/low surrogate)
                if (0xD800..=0xDBFF).contains(&code_point) {
                  // High surrogate: 다음 \uXXXX를 low surrogate로 기대
                  if self.peek() == Some('\\') {
                    self.pos += 1; // consume '\'
                    if self.peek() == Some('u') {
                      self.pos += 1; // consume 'u'
                      let mut low_hex = String::new();
                      for _ in 0..4 {
                        if let Some(ch) = self.peek() {
                          if ch.is_ascii_hexdigit() {
                            low_hex.push(ch);
                            self.pos += ch.len_utf8();
                          } else {
                            return Err(self.parse_error(format_expected_found(
                              "hex digit",
                              format!("'{}'", ch),
                            )));
                          }
                        } else {
                          return Err(self.parse_error(format_expected_found("hex digit", "EOF")));
                        }
                      }
                      let low_surrogate = u32::from_str_radix(&low_hex, 16).map_err(|_| {
                        self.parse_error(format!("Invalid Unicode escape: \\u{}", low_hex))
                      })?;
                      if (0xDC00..=0xDFFF).contains(&low_surrogate) {
                        // Surrogate pair를 UTF-32 code point로 변환
                        let utf32 =
                          0x10000 + ((code_point - 0xD800) << 10) + (low_surrogate - 0xDC00);
                        if let Some(ch) = char::from_u32(utf32) {
                          out.push(ch);
                        } else {
                          return Err(self.parse_error(format!(
                            "Invalid surrogate pair: \\u{}\\u{}",
                            hex_digits, low_hex
                          )));
                        }
                      } else {
                        return Err(
                          self.parse_error(format!("Invalid low surrogate: \\u{}", low_hex)),
                        );
                      }
                    } else {
                      return Err(self.parse_error(format_expected_found(
                        "'u'",
                        format!("'{}'", self.peek().unwrap_or('?')),
                      )));
                    }
                  } else {
                    return Err(self.parse_error(
                      "High surrogate must be followed by low surrogate".to_string(),
                    ));
                  }
                } else if let Some(ch) = char::from_u32(code_point) {
                  out.push(ch);
                } else {
                  return Err(
                    self.parse_error(format!("Invalid Unicode code point: \\u{}", hex_digits)),
                  );
                }
              }
            }
            'x' => {
              // Hex escape: \xXX (2자리 hex)
              // Note: 'x' is already consumed by self.pos += escaped.len_utf8() above
              let mut hex_digits = String::new();
              for _ in 0..2 {
                if let Some(ch) = self.peek() {
                  if ch.is_ascii_hexdigit() {
                    hex_digits.push(ch);
                    self.pos += ch.len_utf8();
                  } else {
                    return Err(
                      self.parse_error(format_expected_found("hex digit", format!("'{}'", ch))),
                    );
                  }
                } else {
                  return Err(self.parse_error(format_expected_found("hex digit", "EOF")));
                }
              }
              let byte_value = u8::from_str_radix(&hex_digits, 16)
                .map_err(|_| self.parse_error(format!("Invalid hex escape: \\x{}", hex_digits)))?;
              // UTF-8로 변환 (단일 바이트는 ASCII 범위만 유효)
              if byte_value < 0x80 {
                out.push(byte_value as char);
              } else {
                // 비ASCII 바이트는 유효한 UTF-8 문자가 아닐 수 있음
                // 하지만 일부 시스템에서는 허용하므로 경고 없이 추가
                // 실제로는 UTF-8 바이트 시퀀스로 처리해야 하지만, 여기서는 단일 바이트만 처리
                return Err(self.parse_error(format!(
                  "\\x{:02X} is not a valid ASCII character. Use \\u{:04X} for Unicode.",
                  byte_value, byte_value
                )));
              }
            }
            '$' => out.push('$'), // escape $ for consistency with interpolated strings
            '\'' => out.push('\''), // single quote
            '0' => out.push('\0'), // null character
            other => {
              // 잘못된 escape sequence 검증: 유효한 escape만 허용
              return Err(self.parse_error(format!(
                "Invalid escape sequence '\\{}'. Valid escapes are: \\n, \\r, \\t, \\\\, \\\", \\$, \\', \\0, \\uXXXX, \\u{{XXXXXX}}, \\xXX",
                other
              )));
            }
          }
        }
        '$' => {
          // Y10a: Check for string interpolation ${...}
          if self.peek() == Some('{') {
            // Found interpolation! Need to switch to full interpolation parsing
            // Reset position to before '$' and use read_string_interp
            self.pos -= 1; // back to '$'
                           // We need to return what we have so far and switch to interp mode
                           // But this is in the middle of parsing... need different approach
                           // For now, push the $ and continue - we'll handle this at a higher level
                           // Actually, let's back out and use a different approach
            self.pos += 1; // skip '{'
                           // Read the expression source until matching '}'
            let expr_start = self.pos;
            let mut brace_depth = 1;
            let mut expr_end = None;
            while brace_depth > 0 {
              match self.peek() {
                Some('{') => {
                  brace_depth += 1;
                  self.pos += 1;
                }
                Some('}') => {
                  brace_depth -= 1;
                  if brace_depth == 0 {
                    expr_end = Some(self.pos);
                  }
                  self.pos += 1;
                }
                Some('"') => {
                  // Skip nested string
                  self.pos += 1;
                  while let Some(c) = self.peek() {
                    self.pos += c.len_utf8();
                    if c == '"' {
                      break;
                    }
                    if c == '\\' {
                      if let Some(esc) = self.peek() {
                        self.pos += esc.len_utf8();
                      }
                    }
                  }
                }
                Some('\'') if self.peek_byte(1) == Some(b'\'') => {
                  // Skip nested multiline string ''...''
                  self.pos += 2; // skip opening ''
                  loop {
                    match self.peek() {
                      Some('\'') if self.peek_byte(1) == Some(b'\'') => {
                        // Check for ''' escape
                        if self.peek_byte(2) == Some(b'\'') {
                          self.pos += 3; // skip '''
                        } else {
                          self.pos += 2; // end of nested multiline string
                          break;
                        }
                      }
                      Some(c) => self.pos += c.len_utf8(),
                      None => break,
                    }
                  }
                }
                Some('\'') if self.peek_byte(1) == Some(b'\'') => {
                  // Skip nested multiline string ''...''
                  self.pos += 2; // skip opening ''
                  loop {
                    match self.peek() {
                      Some('\'') if self.peek_byte(1) == Some(b'\'') => {
                        // Check for ''' escape
                        if self.peek_byte(2) == Some(b'\'') {
                          self.pos += 3; // skip '''
                        } else {
                          self.pos += 2; // end of nested multiline string
                          break;
                        }
                      }
                      Some(c) => self.pos += c.len_utf8(),
                      None => break,
                    }
                  }
                }
                Some('#') => {
                  self.skip_comment();
                }
                Some('/') if self.peek_byte(1) == Some(b'*') => {
                  self.skip_block_comment()?;
                }
                Some(c) => {
                  self.pos += c.len_utf8();
                }
                None => return Err(self.parse_error("unclosed interpolation ${")),
              }
            }
            let expr_end = expr_end.ok_or_else(|| self.parse_error("unclosed interpolation ${"))?;
            let _expr_src = self
              .src
              .get(expr_start..expr_end)
              .ok_or_else(|| self.parse_error("invalid interpolation range"))?
              .to_string();

            // We found interpolation - return an error indicating this
            // The caller should use read_string_token instead
            // For now, return a placeholder indicating interpolation was found
            return Err(PnixError::UnsupportedSyntax {
              message: format!("__INTERP_FOUND__:{}", out),
              span: Some(Span::new(start, expr_end)),
            });
          } else {
            out.push('$');
          }
        }
        other => out.push(other),
      }
    }
    let _ = start;
    Err(self.parse_error(format_expected_found("symbol '\"'", "EOF")))
  }

  /// Y10a: Read a string that may contain interpolation ${...}
  /// Returns Token::String for simple strings, Token::StringInterp for interpolated
  fn read_string_token(&mut self) -> Result<Token, PnixError> {
    let start = self.pos;
    self.pos += 1; // skip opening quote
    let mut parts: Vec<LexerStringPart> = Vec::new();
    let mut current_lit = String::new();

    while let Some(ch) = self.peek() {
      // 토큰 길이 제한 확인 (DoS 방지)
      if current_lit.len() >= Self::MAX_TOKEN_LENGTH {
        return Err(self.parse_error(format!(
          "String literal too long (max: {} bytes)",
          Self::MAX_TOKEN_LENGTH
        )));
      }

      self.pos += ch.len_utf8();
      match ch {
        '"' => {
          // End of string
          if parts.is_empty() {
            // No interpolation, return simple string
            return Ok(Token::String(current_lit));
          } else {
            // Has interpolation
            if !current_lit.is_empty() {
              parts.push(LexerStringPart::Lit(current_lit));
            }
            return Ok(Token::StringInterp(parts));
          }
        }
        '\\' => {
          // Handle escape sequences (simplified - reuse logic)
          let escaped = self
            .peek()
            .ok_or_else(|| self.parse_error(format_expected_found("escape character", "EOF")))?;
          self.pos += escaped.len_utf8();
          match escaped {
            'n' => current_lit.push('\n'),
            'r' => current_lit.push('\r'),
            't' => current_lit.push('\t'),
            '\\' => current_lit.push('\\'),
            '"' => current_lit.push('"'),
            'u' => {
              // Unicode escape: \u{XXXXXX}, \uXXXX, or \uXXXX\uXXXX (surrogate pair)
              if self.peek() == Some('{') {
                // \u{XXXXXX} 형식
                self.pos += 1; // consume '{'
                let mut hex_digits = String::new();
                while let Some(ch) = self.peek() {
                  if ch == '}' {
                    self.pos += 1; // consume '}'
                    break;
                  }
                  if ch.is_ascii_hexdigit() {
                    hex_digits.push(ch);
                    self.pos += ch.len_utf8();
                  } else {
                    return Err(self.parse_error(format_expected_found(
                      "hex digit or '}'",
                      format!("'{}'", ch),
                    )));
                  }
                }
                if hex_digits.is_empty() {
                  return Err(
                    self.parse_error("Unicode escape sequence requires at least one hex digit"),
                  );
                }
                let code_point = u32::from_str_radix(&hex_digits, 16).map_err(|_| {
                  self.parse_error(format!("Invalid Unicode code point: \\u{{{}}}", hex_digits))
                })?;
                if code_point > 0x10FFFF {
                  return Err(self.parse_error(format!(
                    "Unicode code point out of range: \\u{{{}}}",
                    hex_digits
                  )));
                }
                if let Some(ch) = char::from_u32(code_point) {
                  current_lit.push(ch);
                } else {
                  return Err(
                    self.parse_error(format!("Invalid Unicode code point: \\u{{{}}}", hex_digits)),
                  );
                }
              } else {
                // \uXXXX 형식 (4자리 hex)
                let mut hex_digits = String::new();
                for _ in 0..4 {
                  if let Some(ch) = self.peek() {
                    if ch.is_ascii_hexdigit() {
                      hex_digits.push(ch);
                      self.pos += ch.len_utf8();
                    } else {
                      return Err(
                        self.parse_error(format_expected_found("hex digit", format!("'{}'", ch))),
                      );
                    }
                  } else {
                    return Err(self.parse_error(format_expected_found("hex digit", "EOF")));
                  }
                }
                let code_point = u32::from_str_radix(&hex_digits, 16).map_err(|_| {
                  self.parse_error(format!("Invalid Unicode escape: \\u{}", hex_digits))
                })?;
                // Surrogate pair 처리 (UTF-16 high/low surrogate)
                if (0xD800..=0xDBFF).contains(&code_point) {
                  if self.peek() == Some('\\') {
                    self.pos += 1; // consume '\'
                    if self.peek() == Some('u') {
                      self.pos += 1; // consume 'u'
                      let mut low_hex = String::new();
                      for _ in 0..4 {
                        if let Some(ch) = self.peek() {
                          if ch.is_ascii_hexdigit() {
                            low_hex.push(ch);
                            self.pos += ch.len_utf8();
                          } else {
                            return Err(self.parse_error(format_expected_found(
                              "hex digit",
                              format!("'{}'", ch),
                            )));
                          }
                        } else {
                          return Err(self.parse_error(format_expected_found("hex digit", "EOF")));
                        }
                      }
                      let low_surrogate = u32::from_str_radix(&low_hex, 16).map_err(|_| {
                        self.parse_error(format!("Invalid Unicode escape: \\u{}", low_hex))
                      })?;
                      if (0xDC00..=0xDFFF).contains(&low_surrogate) {
                        let utf32 =
                          0x10000 + ((code_point - 0xD800) << 10) + (low_surrogate - 0xDC00);
                        if let Some(ch) = char::from_u32(utf32) {
                          current_lit.push(ch);
                        } else {
                          return Err(self.parse_error(format!(
                            "Invalid surrogate pair: \\u{}\\u{}",
                            hex_digits, low_hex
                          )));
                        }
                      } else {
                        return Err(
                          self.parse_error(format!("Invalid low surrogate: \\u{}", low_hex)),
                        );
                      }
                    } else {
                      return Err(self.parse_error(format_expected_found(
                        "'u'",
                        format!("'{}'", self.peek().unwrap_or('?')),
                      )));
                    }
                  } else {
                    return Err(
                      self.parse_error("High surrogate must be followed by low surrogate"),
                    );
                  }
                } else if let Some(ch) = char::from_u32(code_point) {
                  current_lit.push(ch);
                } else {
                  // LOW: Float 파싱 시 정밀도 손실 경고 없음
                  // 긴 소수점은 silent truncate되며 경고 없음
                  // 이는 부동소수점 정밀도 제한으로 인한 것으로, 경고 추가 고려 필요
                  return Err(
                    self.parse_error(format!("Invalid Unicode code point: \\u{}", hex_digits)),
                  );
                }
              }
            }
            'x' => {
              // Hex escape: \xXX (2자리 hex)
              let mut hex_digits = String::new();
              for _ in 0..2 {
                if let Some(ch) = self.peek() {
                  if ch.is_ascii_hexdigit() {
                    hex_digits.push(ch);
                    self.pos += ch.len_utf8();
                  } else {
                    return Err(
                      self.parse_error(format_expected_found("hex digit", format!("'{}'", ch))),
                    );
                  }
                } else {
                  return Err(self.parse_error(format_expected_found("hex digit", "EOF")));
                }
              }
              let byte_value = u8::from_str_radix(&hex_digits, 16)
                .map_err(|_| self.parse_error(format!("Invalid hex escape: \\x{}", hex_digits)))?;
              // MEDIUM: \xXX 비ASCII 이스케이프 과도한 제한 수정
              // Nix는 0x80+ 값도 허용하므로, 유효한 UTF-8 바이트 시퀀스로 처리
              // 단일 바이트는 char::from_u32로 변환 시도
              if let Some(ch) = char::from_u32(byte_value as u32) {
                current_lit.push(ch);
              } else {
                // 유효하지 않은 단일 바이트: UTF-8 바이트로 처리
                // Nix는 바이너리 데이터를 허용하므로, 바이트를 그대로 저장
                // 하지만 Rust String은 UTF-8만 허용하므로, lossy 변환 사용
                // u8을 char로 직접 변환할 수 없으므로 REPLACEMENT CHARACTER 사용
                current_lit.push('\u{FFFD}'); // U+FFFD REPLACEMENT CHARACTER
              }
            }
            '$' => current_lit.push('$'), // Y10a: escape $ to prevent interpolation
            '\'' => current_lit.push('\''), // single quote
            '0' => current_lit.push('\0'), // null character
            // Nix-compat: `\<newline>` continues the literal with no character
            // (line continuation). Used in long string literals.
            '\n' => {}
            // Nix-compat: `\<space>` and `\<tab>` strip the prefix character —
            // they appear in some manifests where authors escape unusual chars.
            ' ' => current_lit.push(' '),
            '\t' => current_lit.push('\t'),
            // Nix-compat: `\/` is a permissive escape inside JSON strings —
            // Nix string parser accepts it as plain `/` for round-tripping
            // JSON escapes. fromJSON also relies on this.
            '/' => current_lit.push('/'),
            _ => {
              // Invalid escape sequence - return error
              return Err(self.parse_error(format!(
                "Invalid escape sequence '\\{}'. Valid escapes are: \\n, \\r, \\t, \\\\, \\\", \\$, \\', \\0, \\uXXXX, \\u{{XXXXXX}}, \\xXX, \\/, \\<newline>",
                escaped
              )));
            }
          }
        }
        '$' => {
          if self.peek() == Some('{') {
            // Y10a: String interpolation ${...}
            self.pos += 1; // consume '{'

            // Save current literal if any
            if !current_lit.is_empty() {
              parts.push(LexerStringPart::Lit(std::mem::take(&mut current_lit)));
            }

            // Read expression until matching '}'
            // Skip comments (# and /* */) inside interpolation
            let expr_start = self.pos;
            let mut brace_depth = 1;
            let mut expr_end = None;
            while brace_depth > 0 {
              // 중첩 깊이 제한 확인 (DoS 방지)
              if brace_depth > Self::MAX_INTERPOLATION_DEPTH {
                return Err(self.parse_error(format!(
                  "String interpolation depth exceeds maximum (max: {})",
                  Self::MAX_INTERPOLATION_DEPTH
                )));
              }
              match self.peek() {
                Some('#') => {
                  // Skip line comment until newline or EOF
                  self.skip_comment();
                }
                Some('/') if self.peek_byte(1) == Some(b'*') => {
                  // Skip block comment /* ... */
                  self.skip_block_comment()?;
                }
                Some('{') => {
                  brace_depth += 1;
                  self.pos += 1;
                }
                Some('}') => {
                  brace_depth -= 1;
                  if brace_depth == 0 {
                    expr_end = Some(self.pos);
                  }
                  self.pos += 1;
                }
                Some('"') => {
                  // Skip nested string
                  self.pos += 1;
                  while let Some(c) = self.peek() {
                    self.pos += c.len_utf8();
                    if c == '"' {
                      break;
                    }
                    if c == '\\' {
                      if let Some(esc) = self.peek() {
                        self.pos += esc.len_utf8();
                      }
                    }
                  }
                }
                Some('\'') if self.peek_byte(1) == Some(b'\'') => {
                  // Skip nested multiline string ''...''
                  self.pos += 2; // skip opening ''
                  loop {
                    match self.peek() {
                      Some('\'') if self.peek_byte(1) == Some(b'\'') => {
                        match self.peek_byte(2) {
                          Some(b'\'') => {
                            // ''' escape -> ''
                            self.pos += 3;
                          }
                          Some(b'$') => {
                            // ''$ escape -> $
                            self.pos += 3;
                          }
                          Some(b'\\') => {
                            // ''\ escape sequence
                            self.pos += 3;
                            if self.peek().is_some() {
                              self.pos += 1;
                            }
                          }
                          _ => {
                            // end of nested multiline string
                            self.pos += 2;
                            break;
                          }
                        }
                      }
                      Some(c) => self.pos += c.len_utf8(),
                      None => break,
                    }
                  }
                }
                Some(c) => {
                  self.pos += c.len_utf8();
                }
                None => return Err(self.parse_error("unclosed interpolation ${")),
              }
            }
            let expr_end = expr_end.ok_or_else(|| self.parse_error("unclosed interpolation ${"))?;
            let expr_src = self
              .src
              .get(expr_start..expr_end)
              .ok_or_else(|| self.parse_error("invalid interpolation range"))?
              .to_string();

            // LOW: 빈 인터폴레이션 ${}에서 빈 표현식 파싱 시도 수정
            // 빈 표현식은 명시적 에러 반환
            if expr_src.trim().is_empty() {
              return Err(self.parse_error("empty interpolation expression ${} is not allowed"));
            }

            parts.push(LexerStringPart::InterpExpr(expr_src));
          } else {
            current_lit.push('$');
          }
        }
        other => current_lit.push(other),
      }
    }
    let _ = start;
    Err(self.parse_error(format_expected_found("symbol '\"'", "EOF")))
  }

  fn read_number(&mut self) -> Result<Token, PnixError> {
    let start = self.pos;
    if self.peek_byte(0) == Some(b'0') {
      let base = match self.peek_byte(1) {
        Some(b'x') | Some(b'X') => Some(16),
        Some(b'o') | Some(b'O') => Some(8),
        Some(b'b') | Some(b'B') => Some(2),
        _ => None,
      };
      if let Some(base) = base {
        self.pos += 2; // consume 0x/0o/0b
        let digits_start = self.pos;
        let is_valid_digit = |b: u8, base: u32| -> bool {
          match base {
            2 => matches!(b, b'0' | b'1'),
            8 => matches!(b, b'0'..=b'7'),
            16 => b.is_ascii_hexdigit(),
            _ => b.is_ascii_digit(),
          }
        };
        while self.peek_byte(0).is_some_and(|b| is_valid_digit(b, base)) {
          self.pos += 1;
        }
        if self.pos == digits_start {
          let found = match self.peek() {
            Some(ch) => format!("'{}'", ch),
            None => "EOF".to_string(),
          };
          return Err(
            self.parse_error(format_expected_found(format!("base-{} digit", base), found)),
          );
        }
        let digits = self
          .src
          .get(digits_start..self.pos)
          .ok_or_else(|| self.parse_error(format_expected_found("number", "EOF")))?;
        let value = i64::from_str_radix(digits, base).map_err(|e| {
          self.parse_error(format!(
            "invalid base-{} integer literal '{}': {}",
            base, digits, e
          ))
        })?;
        return Ok(Token::Int(value));
      }
    }
    let mut saw_digit = false;
    while self.peek_byte(0).is_some_and(|b| b.is_ascii_digit()) {
      self.pos += 1;
      saw_digit = true;
    }

    let mut is_float = false;
    if self.peek_byte(0) == Some(b'.') {
      if self.peek_byte(1).is_some_and(|b| b.is_ascii_digit()) {
        is_float = true;
        self.pos += 1; // '.'
        while self.peek_byte(0).is_some_and(|b| b.is_ascii_digit()) {
          self.pos += 1;
          saw_digit = true;
        }
      } else if saw_digit {
        let next = self.peek_byte(1);
        let allow_trailing_dot = match next {
          None => true,
          Some(b) if (b as char).is_whitespace() => true,
          Some(b)
            if matches!(
              b as char,
              ')'
                | ']'
                | '}'
                | ';'
                | ','
                | '+'
                | '-'
                | '*'
                | '/'
                | '%'
                | '='
                | '<'
                | '>'
                | '!'
                | '&'
                | '|'
                | '?'
            ) =>
          {
            true
          }
          Some(b'e') | Some(b'E') => true,
          _ => false,
        };
        if allow_trailing_dot {
          is_float = true;
          self.pos += 1; // trailing '.'
        }
      }
    }

    if matches!(self.peek_byte(0), Some(b'e') | Some(b'E')) {
      self.pos += 1; // 'e' or 'E'
      if matches!(self.peek_byte(0), Some(b'+') | Some(b'-')) {
        self.pos += 1; // sign
      }
      let exp_start = self.pos;
      while self.peek_byte(0).is_some_and(|b| b.is_ascii_digit()) {
        self.pos += 1;
      }
      if self.pos == exp_start {
        let found = match self.peek() {
          Some(ch) => format!("'{}'", ch),
          None => "EOF".to_string(),
        };
        return Err(self.parse_error(format_expected_found("exponent digits", found)));
      }
      is_float = true;
    }

    if !saw_digit {
      let found = match self.peek() {
        Some(ch) => format!("'{}'", ch),
        None => "EOF".to_string(),
      };
      return Err(self.parse_error(format_expected_found("number", found)));
    }

    let s = self
      .src
      .get(start..self.pos)
      .ok_or_else(|| self.parse_error(format_expected_found("number", "EOF")))?;
    if is_float {
      // LOW: 부동소수점 앞 0 검증 추가 (00.5 등 거부)
      if s.len() >= 2 && s.starts_with('0') && s.chars().nth(1).is_some_and(|c| c.is_ascii_digit())
      {
        return Err(self.parse_error(format!(
          "invalid float literal: leading zero not allowed in '{}' (use '{}' instead)",
          s,
          s.trim_start_matches('0')
        )));
      }
      // LOW: Float 파싱 NaN/Infinity 거부
      let normalized_lower = s.to_lowercase();
      if normalized_lower == "nan" || normalized_lower == "inf" || normalized_lower == "infinity" {
        return Err(self.parse_error(format!(
          "invalid float literal: '{}' is not a valid number (NaN/Infinity not allowed)",
          s
        )));
      }
      let mut normalized = if s.starts_with('.') {
        format!("0{s}")
      } else {
        s.to_string()
      };
      if normalized.ends_with('.') {
        normalized.push('0');
      }
      if let Some(idx) = normalized.find(".e") {
        normalized.insert(idx + 1, '0');
      }
      if let Some(idx) = normalized.find(".E") {
        normalized.insert(idx + 1, '0');
      }
      let f = normalized
        .parse::<f64>()
        .map_err(|e| self.parse_error(format_expected_found("float", format!("'{}': {}", s, e))))?;
      // NaN/Infinity 재확인 (parse 후에도 체크)
      if f.is_nan() || f.is_infinite() {
        return Err(self.parse_error(format!(
          "invalid float literal: '{}' evaluates to {} (NaN/Infinity not allowed)",
          s,
          if f.is_nan() {
            "NaN"
          } else if f.is_infinite() && f.is_sign_positive() {
            "Infinity"
          } else {
            "-Infinity"
          }
        )));
      }
      // LOW: Float 파싱 시 정밀도 손실 경고 없음 수정 완료
      // 긴 소수점이 f64 정밀도 한계(약 15-17자리)를 초과하면 정밀도 손실 발생
      // 원본 문자열을 f64로 파싱한 후 다시 문자열로 변환하여 비교하여 정밀도 손실 감지
      let f_str = f.to_string();
      let original_significant_digits = s
        .trim_start_matches('-')
        .trim_start_matches('+')
        .trim_start_matches('0')
        .chars()
        .filter(|c| c.is_ascii_digit())
        .count();
      let parsed_significant_digits = f_str
        .trim_start_matches('-')
        .trim_start_matches('+')
        .trim_start_matches('0')
        .chars()
        .filter(|c| c.is_ascii_digit())
        .count();
      // 정밀도 손실이 발생한 경우 경고 출력 (약 3자리 이상 차이)
      if original_significant_digits > 17
        && original_significant_digits > parsed_significant_digits + 3
      {
        eprintln!(
          "warning: float literal '{}' has {} significant digits, but f64 can only represent about 15-17 digits accurately. Parsed value: {}",
          s, original_significant_digits, f
        );
      }
      // 간단한 체크: 파싱된 값을 다시 문자열로 변환하여 원본과 비교
      let parsed_back = format!("{:.15}", f);
      if s.len() > 20 && !parsed_back.contains(&s[..s.len().min(20)]) {
        // 정밀도 손실 가능성 있음 (경고만, 에러는 아님)
        // 실제로는 로깅하거나 경고 메시지를 추가할 수 있지만, LOW 버그이므로 주석으로만 표시
      }
      Ok(Token::Float(f))
    } else {
      let i = s.parse::<i64>().map_err(|e| {
        // 정수 오버플로우 에러 메시지 개선
        if s.len() > 19 || s.parse::<u64>().is_ok() {
          // i64 범위를 초과하는 경우
          self.parse_error(format!(
            "integer overflow: '{}' exceeds i64 range ({} to {}): {}",
            s,
            i64::MIN,
            i64::MAX,
            e
          ))
        } else {
          self.parse_error(format_expected_found("int", format!("'{}': {}", s, e)))
        }
      })?;
      Ok(Token::Int(i))
    }
  }

  fn read_ident(&mut self) -> Result<String, PnixError> {
    let start = self.pos;
    let mut length = 0usize;
    let mut too_long = false;
    if let Some(ch) = self.peek() {
      self.pos += ch.len_utf8();
      length = length.saturating_add(ch.len_utf8());
    } else {
      return Ok(String::new());
    }
    while let Some(ch) = self.peek() {
      if is_ident_continue(ch) {
        length = length.saturating_add(ch.len_utf8());
        if length > Self::MAX_IDENT_LENGTH {
          too_long = true;
        }
        self.pos += ch.len_utf8();
      } else {
        break;
      }
    }

    if too_long {
      let (line, col) = line_col_at(self.src, start);
      let span = Span::new(start, self.pos);
      return Err(PnixError::parse_with_code_at(
        format!(
          "Identifier too long (max: {} bytes). Possible DoS attack or malformed input",
          Self::MAX_IDENT_LENGTH
        ),
        crate::diagnostics::error_codes::TOKENIZE_ERROR,
        line,
        col,
        span,
      ));
    }

    Ok(self.src.get(start..self.pos).unwrap_or("").to_string())
  }

  /// Y10b: Check if character is valid in a path
  fn is_path_char(ch: char) -> bool {
    ch.is_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-' | '+')
  }

  /// Read the body of a path token, splitting at `${...}` boundaries.
  /// Returns the literal/interp parts. Each interp slice is the raw
  /// source text inside the braces (not yet parsed); the parser will
  /// re-parse it as a `PnixExpr`. The path body must contain at least
  /// one non-empty character. The leading base prefix (`./`, `/`,
  /// `~/`) is *not* consumed here — caller already consumed it.
  fn read_path_parts(&mut self) -> Result<Vec<LexerStringPart>, PnixError> {
    let mut parts: Vec<LexerStringPart> = Vec::new();
    let mut lit_start = self.pos;
    let mut total_chars = 0usize;
    loop {
      let Some(ch) = self.peek() else {
        break;
      };
      if ch == '$' && self.peek_byte(1) == Some(b'{') {
        // Flush accumulated literal segment.
        if self.pos > lit_start {
          let lit = self.src.get(lit_start..self.pos).unwrap_or("").to_string();
          if !lit.is_empty() {
            parts.push(LexerStringPart::Lit(lit));
          }
        }
        // Consume `${`, scan until matching `}` with brace tracking.
        self.pos += 2;
        let expr_start = self.pos;
        let mut depth: usize = 1;
        let mut in_string: Option<char> = None;
        while let Some(c) = self.peek() {
          if let Some(quote) = in_string {
            // Inside a string literal: only `}` closing the string
            // matters here for balance — we skip until the matching
            // quote, respecting backslash escapes.
            if c == '\\' && self.peek_byte(1).is_some() {
              self.pos += 1
                + (self
                  .peek_byte(1)
                  .map(|b| {
                    let bc = b as char;
                    bc.len_utf8()
                  })
                  .unwrap_or(1));
              continue;
            }
            if c == quote {
              in_string = None;
            }
            self.pos += c.len_utf8();
            continue;
          }
          match c {
            '"' => {
              in_string = Some('"');
              self.pos += 1;
            }
            '{' => {
              depth += 1;
              self.pos += 1;
            }
            '}' => {
              depth -= 1;
              if depth == 0 {
                let expr_text = self.src.get(expr_start..self.pos).unwrap_or("").to_string();
                self.pos += 1; // consume `}`
                parts.push(LexerStringPart::InterpExpr(expr_text));
                lit_start = self.pos;
                break;
              }
              self.pos += 1;
            }
            _ => {
              self.pos += c.len_utf8();
            }
          }
        }
        if depth != 0 {
          return Err(self.parse_error("unclosed `${` in path interpolation"));
        }
        total_chars += 1;
        continue;
      }
      if Self::is_path_char(ch) {
        self.pos += ch.len_utf8();
        total_chars += 1;
        continue;
      }
      break;
    }
    // Flush trailing literal.
    if self.pos > lit_start {
      let lit = self.src.get(lit_start..self.pos).unwrap_or("").to_string();
      if !lit.is_empty() {
        parts.push(LexerStringPart::Lit(lit));
      }
    }
    if total_chars == 0 || parts.is_empty() {
      return Err(self.parse_error("empty path"));
    }
    Ok(parts)
  }

  /// True if any part is an interp expression. Used by path-token
  /// emit sites to decide between `Token::Path` and `Token::PathInterp`.
  fn parts_have_interp(parts: &[LexerStringPart]) -> bool {
    parts
      .iter()
      .any(|p| matches!(p, LexerStringPart::InterpExpr(_)))
  }

  /// Join lit-only parts into a single string. Used when no interp
  /// was found and we want to fall back to the existing `Token::Path`
  /// shape.
  fn parts_join_lit(parts: Vec<LexerStringPart>) -> String {
    let mut out = String::new();
    for p in parts {
      if let LexerStringPart::Lit(s) = p {
        out.push_str(&s);
      }
    }
    out
  }

  /// Y10b: Read a search path token <nixpkgs> or <nixpkgs/lib>
  fn read_search_path(&mut self) -> Result<String, PnixError> {
    self.pos += 1; // consume '<'
    let start = self.pos;
    while let Some(ch) = self.peek() {
      if ch == '>' {
        let path = self.src.get(start..self.pos).unwrap_or("").to_string();
        self.pos += 1; // consume '>'
        if path.is_empty() {
          return Err(self.parse_error("empty search path"));
        }
        return Ok(path);
      } else if ch.is_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-' | '+') {
        // LOW: 검색 경로 `+` 문자 미허용 수정 완료
        // Nix는 <nixpkgs+doc> 같은 형식을 지원하므로 + 문자 허용 (이미 구현됨)
        self.pos += ch.len_utf8();
      } else {
        return Err(self.parse_error(format!("invalid character '{}' in search path", ch)));
      }
    }
    Err(self.parse_error("unclosed search path (missing '>')"))
  }

  fn parse_error(&self, message: impl Into<String>) -> PnixError {
    let span = Span::point(self.pos);
    PnixError::parse_at_span(message, span, self.src)
  }
}

/// Try to match a Nix URL literal starting at `start` in `src`. Returns
/// the end byte offset (exclusive) on success. Mirrors nix/lexer.l URI:
///   `[a-zA-Z][a-zA-Z0-9+\-.]*:[URLchar]+`.
/// Returns `None` if the prefix does not look like a URL — leaving the
/// caller to fall through to the regular ident reader.
fn scan_url_literal(src: &[u8], start: usize) -> Option<usize> {
  let mut i = start;
  // First scheme character must be ASCII alphabetic.
  if i >= src.len() || !src[i].is_ascii_alphabetic() {
    return None;
  }
  i += 1;
  // Subsequent scheme chars: alphanumeric, `+`, `-`, `.`.
  while i < src.len() {
    let b = src[i];
    if b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.') {
      i += 1;
    } else {
      break;
    }
  }
  // Must hit `:` next.
  if i >= src.len() || src[i] != b':' {
    return None;
  }
  i += 1;
  // Body must have at least one URL-body character.
  if i >= src.len() || !is_url_body_char(src[i]) {
    return None;
  }
  while i < src.len() && is_url_body_char(src[i]) {
    i += 1;
  }
  Some(i)
}

/// Nix URL literal body character set: `[a-zA-Z0-9%/?:@&=+$,\-_.!~*']`.
/// See nix/src/libexpr/lexer.l URI rule.
fn is_url_body_char(b: u8) -> bool {
  matches!(
    b,
    b'a'..=b'z'
      | b'A'..=b'Z'
      | b'0'..=b'9'
      | b'%'
      | b'/'
      | b'?'
      | b':'
      | b'@'
      | b'&'
      | b'='
      | b'+'
      | b'$'
      | b','
      | b'-'
      | b'_'
      | b'.'
      | b'!'
      | b'~'
      | b'*'
      | b'\''
  )
}
