//! 진단: Span, SourceMap, Error, Warning

mod diag;
mod error_code;
mod lang_errors;
mod lsp;
mod nix_macro_error;
mod pnix_error;
mod span;

pub use diag::{Diagnostic, Diagnostics};
pub use error_code::{error_codes, ErrorCode};
pub use lang_errors::{GhCodegenError, JsError, PnixError as LangPnixError, PythonError};
pub use nix_macro_error::{MacroExpansionError, SourceLocation};
pub use pnix_error::{
  format_error_with_context, is_recoverable_error, CompileError, ErrorContext, IntoPnixError,
  ParseError, PnixError, PnixResult, TokenizeError,
};
pub use span::Span;

/// 소스 코드에서 위치의 line/column 계산 (공통 유틸리티)
///
/// UTF-8 바이트 오프셋을 (line, column) 쌍으로 변환합니다.
/// CJK 문자와 이모지의 시각적 너비를 고려합니다.
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
///
/// # Arguments
///
/// * `source` - 소스 코드 문자열
/// * `pos` - 바이트 오프셋 위치
///
/// # Returns
///
/// `(line, column)` 쌍 (1부터 시작)
pub fn line_col_at(source: &str, pos: usize) -> (usize, usize) {
  let mut line = 1usize;
  let mut column = 1usize;
  let mut idx = 0usize;
  for ch in source.chars() {
    if idx >= pos {
      break;
    }
    if ch == '\n' {
      line += 1;
      column = 1;
    } else {
      column += crate::text_width::char_width(ch);
    }
    idx += ch.len_utf8();
  }
  (line, column)
}
