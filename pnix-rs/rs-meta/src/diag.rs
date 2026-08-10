//! Positional diagnostics (Plan Phase E4, v1). The parser and AST stay
//! structurally UNCHANGED (the plan flags full span plumbing as a self-host
//! regression risk): parser errors already carry a token index ("at token
//! N"), so this layer maps index -> char offset (lex_spanned) -> line/col
//! and renders the offending source line with a caret. Errors without a
//! token index pass through untouched. Typeck-level spans (AST-wide) stay
//! HELD — typeck errors keep their fn-context prefix.

use crate::lexer;

/// Extract N from a trailing "at token N" (parser error convention).
fn token_index_of(err: &str) -> Option<usize> {
    // Trailing "at token N" (parser error convention): scan for the LAST
    // marker occurrence without str::rfind (outside the evaluated subset).
    let marker = " at token ";
    let chars: Vec<char> = err.chars().collect();
    let mchars: Vec<char> = marker.chars().collect();
    let mut found: Option<usize> = None;
    if chars.len() >= mchars.len() {
        let mut i = 0usize;
        while i + mchars.len() <= chars.len() {
            let mut hit = true;
            let mut j = 0usize;
            while j < mchars.len() {
                if chars[i + j] != mchars[j] {
                    hit = false;
                    break;
                }
                j += 1;
            }
            if hit {
                found = Some(i + mchars.len());
            }
            i += 1;
        }
    }
    let start = found?;
    let mut digits = String::new();
    let mut k = start;
    while k < chars.len() && chars[k].is_ascii_digit() {
        digits.push(chars[k]);
        k += 1;
    }
    if digits.is_empty() {
        return None;
    }
    match digits.parse::<i64>() {
        Ok(n) => Some(n as usize),
        Err(_) => None,
    }
}

fn source_line(src: &str, line: usize) -> String {
    let mut current = 1usize;
    let mut out = String::new();
    for c in src.chars() {
        if c == '\n' {
            if current == line {
                return out;
            }
            current += 1;
            out = String::new();
        } else if current == line {
            out.push(c);
        }
    }
    out
}

/// Extract NAME from a "typeck: in fn NAME:" prefix (typeck error convention).
fn fn_name_of(err: &str) -> Option<String> {
    let marker = "in fn ";
    let chars: Vec<char> = err.chars().collect();
    let mchars: Vec<char> = marker.chars().collect();
    let mut start: Option<usize> = None;
    if chars.len() >= mchars.len() {
        let mut i = 0usize;
        while i + mchars.len() <= chars.len() {
            let mut hit = true;
            let mut j = 0usize;
            while j < mchars.len() {
                if chars[i + j] != mchars[j] {
                    hit = false;
                    break;
                }
                j += 1;
            }
            if hit {
                start = Some(i + mchars.len());
                break;
            }
            i += 1;
        }
    }
    let s = start?;
    let mut name = String::new();
    let mut k = s;
    while k < chars.len() && (chars[k].is_ascii_alphanumeric() || chars[k] == '_') {
        name.push(chars[k]);
        k += 1;
    }
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Char offset of a top-level `fn NAME` definition in the source.
fn fn_offset(src: &str, name: &str) -> Option<usize> {
    let chars: Vec<char> = src.chars().collect();
    let needle: Vec<char> = format!("fn {}", name).chars().collect();
    let mut i = 0usize;
    while i + needle.len() <= chars.len() {
        let mut hit = true;
        let mut j = 0usize;
        while j < needle.len() {
            if chars[i + j] != needle[j] {
                hit = false;
                break;
            }
            j += 1;
        }
        // Require the char after the name to be a boundary (not part of a
        // longer identifier) — `fn main` must not match `fn mainish`.
        if hit {
            let after = i + needle.len();
            let boundary = after >= chars.len()
                || chars[after] == '('
                || chars[after] == '<'
                || chars[after] == ' ';
            if boundary {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// Render an error with position: token index (parser) or `in fn NAME`
/// (typeck, function granularity — the plan's expression-level spans stay
/// held); otherwise return it unchanged.
pub fn render_error(src: &str, err: &str) -> String {
    let offset = match token_index_of(err) {
        Some(n) => {
            let spanned = match lexer::lex_spanned(src) {
                Ok(pair) => pair,
                Err(_) => return String::from(err),
            };
            let offsets = spanned.1;
            if n < offsets.len() {
                offsets[n]
            } else {
                src.chars().count()
            }
        }
        None => match fn_name_of(err) {
            Some(name) => match fn_offset(src, &name) {
                Some(o) => o,
                None => return String::from(err),
            },
            None => return String::from(err),
        },
    };
    render_at(src, err, offset)
}

fn render_at(src: &str, err: &str, offset: usize) -> String {
    let (line, col) = lexer::offset_line_col(src, offset);
    let text = source_line(src, line);
    let mut caret = String::new();
    let mut k = 1usize;
    while k < col {
        caret.push(' ');
        k += 1;
    }
    caret.push('^');
    let line_label = format!("{}", line);
    let mut pad = String::new();
    let mut p = 0usize;
    while p < line_label.len() {
        pad.push(' ');
        p += 1;
    }
    format!(
        "{}\n  --> line {}, col {}\n  {} | {}\n  {} | {}",
        err, line, col, line_label, text, pad, caret
    )
}
