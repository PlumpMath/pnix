//! Hand-written tokenizer for the Rust subset. Zero dependencies.

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Int(i64),
    IntSuffixed(i64, String),
    /// Hex integer with its source text preserved for emission (i64-wrapped
    /// value keeps interpreter semantics; text keeps rustc semantics).
    IntHex(i64, String),
    Float(String),
    Char(char),
    Lifetime(String),
    Ident(String),
    Str(String),
    // keywords
    KwFn,
    KwLet,
    KwIf,
    KwElse,
    KwReturn,
    KwTrue,
    KwFalse,
    KwStruct,
    KwEnum,
    KwMatch,
    KwWhile,
    KwLoop,
    KwFor,
    KwIn,
    KwBreak,
    KwContinue,
    KwMut,
    KwImpl,
    KwUse,
    KwPub,
    KwMod,
    KwAs,
    KwConst,
    KwMove,
    KwStatic,
    KwTrait,
    // punctuation / operators
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semi,
    Colon,
    ColonColon, // ::
    Dot,        // .
    DotDot,     // ..
    DotDotEq,   // ..=
    Pipe,       // |
    At,         // @
    FatArrow,   // =>
    Arrow,      // ->
    Plus,
    PlusEq,
    Minus,
    MinusEq,
    Star,
    StarEq,
    Slash,
    SlashEq,
    Percent,
    PercentEq,
    Caret,
    CaretEq,
    EqEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    AndAnd,
    Amp,
    AmpEq,
    PipeEq,
    ShlEq,
    ShrEq,
    OrOr,
    Bang, // ! (logical not and macro marker)
    Eq,   // =
    Hash, // #
    Question,
}

pub fn lex(src: &str) -> Result<Vec<Tok>, String> {
    match lex_spanned(src) {
        Ok((toks, _offsets)) => Ok(toks),
        Err(e) => Err(e),
    }
}

/// Phase E4: tokens plus each token's source CHAR OFFSET (parallel vector).
/// The parser stays offset-free — its errors carry token indices, and the
/// diagnostic layer maps index -> offset -> line/col after the fact.
pub fn lex_spanned(src: &str) -> Result<(Vec<Tok>, Vec<usize>), String> {
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut toks = Vec::new();
    let mut offsets: Vec<usize> = Vec::new();
    let mut last_start = 0usize;
    while i < chars.len() {
        while offsets.len() < toks.len() {
            offsets.push(last_start);
        }
        last_start = i;
        let c = chars[i];
        match c {
            c if c.is_whitespace() => i += 1,
            '/' if peek(&chars, i + 1) == Some('/') => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '/' if peek(&chars, i + 1) == Some('*') => {
                i += 2;
                while i < chars.len() && !(chars[i] == '*' && peek(&chars, i + 1) == Some('/')) {
                    i += 1;
                }
                i += 2; // skip */
            }
            '"' => {
                i += 1;
                let mut s = String::new();
                loop {
                    match chars.get(i) {
                        None => return Err("lex: unterminated string".to_string()),
                        Some('"') => {
                            i += 1;
                            break;
                        }
                        Some('\\') => {
                            i += 1;
                            match chars.get(i) {
                                Some('n') => s.push('\n'),
                                Some('t') => s.push('\t'),
                                Some('\\') => s.push('\\'),
                                Some('"') => s.push('"'),
                                Some('{') => s.push('{'),
                                Some('}') => s.push('}'),
                                Some('\n') => {
                                    i += 1;
                                    while matches!(chars.get(i), Some(c) if c.is_whitespace()) {
                                        i += 1;
                                    }
                                    continue;
                                }
                                Some('\r') if peek(&chars, i + 1) == Some('\n') => {
                                    i += 2;
                                    while matches!(chars.get(i), Some(c) if c.is_whitespace()) {
                                        i += 1;
                                    }
                                    continue;
                                }
                                Some(other) => return Err(format!("lex: bad escape \\{}", other)),
                                None => return Err("lex: dangling escape".to_string()),
                            }
                            i += 1;
                        }
                        Some(&ch) => {
                            s.push(ch);
                            i += 1;
                        }
                    }
                }
                toks.push(Tok::Str(s));
            }
            '\'' => {
                if matches!(peek(&chars, i + 1), Some(ch) if ch.is_ascii_alphabetic() || ch == '_')
                {
                    let start = i + 1;
                    let mut j = start;
                    while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '_') {
                        j += 1;
                    }
                    if chars.get(j) != Some(&'\'') {
                        let name: String = chars[start..j].iter().collect();
                        toks.push(Tok::Lifetime(name));
                        i = j;
                        continue;
                    }
                }
                i += 1;
                let ch = match chars.get(i) {
                    None => return Err("lex: unterminated char literal".to_string()),
                    Some('\\') => {
                        i += 1;
                        let escaped = match chars.get(i) {
                            Some('n') => '\n',
                            Some('r') => '\r',
                            Some('t') => '\t',
                            Some('0') => '\0',
                            Some('\\') => '\\',
                            Some('\'') => '\'',
                            Some(other) => return Err(format!("lex: bad char escape \\{}", other)),
                            None => return Err("lex: dangling char escape".to_string()),
                        };
                        i += 1;
                        escaped
                    }
                    Some(&ch) => {
                        i += 1;
                        ch
                    }
                };
                if chars.get(i) != Some(&'\'') {
                    return Err("lex: char literal must contain one character".to_string());
                }
                i += 1;
                toks.push(Tok::Char(ch));
            }
            '(' => push(&mut toks, &mut i, Tok::LParen),
            ')' => push(&mut toks, &mut i, Tok::RParen),
            '{' => push(&mut toks, &mut i, Tok::LBrace),
            '}' => push(&mut toks, &mut i, Tok::RBrace),
            '[' => push(&mut toks, &mut i, Tok::LBracket),
            ']' => push(&mut toks, &mut i, Tok::RBracket),
            ',' => push(&mut toks, &mut i, Tok::Comma),
            ';' => push(&mut toks, &mut i, Tok::Semi),
            '@' => push(&mut toks, &mut i, Tok::At),
            '.' => {
                if peek(&chars, i + 1) == Some('.') {
                    if peek(&chars, i + 2) == Some('=') {
                        i += 3;
                        toks.push(Tok::DotDotEq);
                    } else {
                        i += 2;
                        toks.push(Tok::DotDot);
                    }
                } else {
                    push(&mut toks, &mut i, Tok::Dot);
                }
            }
            ':' => {
                if peek(&chars, i + 1) == Some(':') {
                    i += 2;
                    toks.push(Tok::ColonColon);
                } else {
                    push(&mut toks, &mut i, Tok::Colon);
                }
            }
            '+' => {
                if peek(&chars, i + 1) == Some('=') {
                    i += 2;
                    toks.push(Tok::PlusEq);
                } else {
                    push(&mut toks, &mut i, Tok::Plus);
                }
            }
            '*' => {
                if peek(&chars, i + 1) == Some('=') {
                    i += 2;
                    toks.push(Tok::StarEq);
                } else {
                    push(&mut toks, &mut i, Tok::Star);
                }
            }
            '/' => {
                if peek(&chars, i + 1) == Some('=') {
                    i += 2;
                    toks.push(Tok::SlashEq);
                } else {
                    push(&mut toks, &mut i, Tok::Slash);
                }
            }
            '%' => {
                if peek(&chars, i + 1) == Some('=') {
                    i += 2;
                    toks.push(Tok::PercentEq);
                } else {
                    push(&mut toks, &mut i, Tok::Percent);
                }
            }
            '-' => {
                if peek(&chars, i + 1) == Some('>') {
                    i += 2;
                    toks.push(Tok::Arrow);
                } else if peek(&chars, i + 1) == Some('=') {
                    i += 2;
                    toks.push(Tok::MinusEq);
                } else {
                    push(&mut toks, &mut i, Tok::Minus);
                }
            }
            '=' => {
                if peek(&chars, i + 1) == Some('=') {
                    i += 2;
                    toks.push(Tok::EqEq);
                } else if peek(&chars, i + 1) == Some('>') {
                    i += 2;
                    toks.push(Tok::FatArrow);
                } else {
                    push(&mut toks, &mut i, Tok::Eq);
                }
            }
            '!' => {
                if peek(&chars, i + 1) == Some('=') {
                    i += 2;
                    toks.push(Tok::Ne);
                } else {
                    push(&mut toks, &mut i, Tok::Bang);
                }
            }
            '#' => push(&mut toks, &mut i, Tok::Hash),
            '?' => push(&mut toks, &mut i, Tok::Question),
            '<' => {
                if peek(&chars, i + 1) == Some('<') && peek(&chars, i + 2) == Some('=') {
                    i += 3;
                    toks.push(Tok::ShlEq);
                } else if peek(&chars, i + 1) == Some('=') {
                    i += 2;
                    toks.push(Tok::Le);
                } else {
                    push(&mut toks, &mut i, Tok::Lt);
                }
            }
            '>' => {
                if peek(&chars, i + 1) == Some('>') && peek(&chars, i + 2) == Some('=') {
                    i += 3;
                    toks.push(Tok::ShrEq);
                } else if peek(&chars, i + 1) == Some('=') {
                    i += 2;
                    toks.push(Tok::Ge);
                } else {
                    push(&mut toks, &mut i, Tok::Gt);
                }
            }
            '&' if peek(&chars, i + 1) == Some('&') => {
                i += 2;
                toks.push(Tok::AndAnd);
            }
            '&' if peek(&chars, i + 1) == Some('=') => {
                i += 2;
                toks.push(Tok::AmpEq);
            }
            '&' => push(&mut toks, &mut i, Tok::Amp),
            '|' if peek(&chars, i + 1) == Some('|') => {
                i += 2;
                toks.push(Tok::OrOr);
            }
            '|' if peek(&chars, i + 1) == Some('=') => {
                i += 2;
                toks.push(Tok::PipeEq);
            }
            '|' => push(&mut toks, &mut i, Tok::Pipe),
            '^' if peek(&chars, i + 1) == Some('=') => {
                i += 2;
                toks.push(Tok::CaretEq);
            }
            '^' => push(&mut toks, &mut i, Tok::Caret),
            c if c.is_ascii_digit() => {
                // Float literal: digits '.' digits (never `..`, never hex).
                let is_hex_start = c == '0' && matches!(peek(&chars, i + 1), Some('x' | 'X'));
                if !is_hex_start {
                    let mut j = i;
                    while j < chars.len() && (chars[j].is_ascii_digit() || chars[j] == '_') {
                        j += 1;
                    }
                    if j + 1 < chars.len() && chars[j] == '.' && chars[j + 1].is_ascii_digit() {
                        let mut text: String =
                            chars[i..j].iter().filter(|c| **c != '_').collect();
                        text.push('.');
                        let mut k = j + 1;
                        let frac_start = k;
                        while k < chars.len() && (chars[k].is_ascii_digit() || chars[k] == '_') {
                            k += 1;
                        }
                        let frac: String =
                            chars[frac_start..k].iter().filter(|c| **c != '_').collect();
                        text.push_str(&frac);
                        i = k;
                        toks.push(Tok::Float(text));
                        continue;
                    }
                }
                if c == '0' && matches!(peek(&chars, i + 1), Some('x' | 'X')) {
                    let text_start = i;
                    i += 2;
                    let start = i;
                    while i < chars.len() && (chars[i].is_ascii_hexdigit() || chars[i] == '_') {
                        i += 1;
                    }
                    let raw: String = chars[start..i].iter().filter(|c| **c != '_').collect();
                    if raw.is_empty() {
                        return Err("lex: bad hex integer".to_string());
                    }
                    let value = u64::from_str_radix(&raw, 16)
                        .map(|n| n as i64)
                        .map_err(|_| format!("lex: bad hex integer {:?}", raw))?;
                    let text: String = chars[text_start..i].iter().collect();
                    toks.push(Tok::IntHex(value, text));
                    continue;
                }
                if c == '0' && matches!(peek(&chars, i + 1), Some('o' | 'O' | 'b' | 'B')) {
                    let radix = if matches!(peek(&chars, i + 1), Some('o' | 'O')) {
                        8
                    } else {
                        2
                    };
                    i += 2;
                    let start = i;
                    while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                        i += 1;
                    }
                    let raw: String = chars[start..i].iter().filter(|c| **c != '_').collect();
                    if raw.is_empty() {
                        return Err("lex: bad octal/binary integer".to_string());
                    }
                    // Emit as a plain decimal Int (value identical, roundtrip-safe).
                    let value = u64::from_str_radix(&raw, radix)
                        .map(|n| n as i64)
                        .map_err(|_| format!("lex: bad radix-{} integer {:?}", radix, raw))?;
                    toks.push(Tok::Int(value));
                    continue;
                }
                let n = if c == '0' && matches!(peek(&chars, i + 1), Some('x' | 'X')) {
                    unreachable!("hex handled above")
                } else {
                    let start = i;
                    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '_') {
                        i += 1;
                    }
                    let raw: String = chars[start..i].iter().filter(|c| **c != '_').collect();
                    raw.parse::<i64>()
                        .map_err(|_| format!("lex: bad integer {:?}", raw))?
                };
                if i < chars.len() && (chars[i].is_ascii_alphabetic() || chars[i] == '_') {
                    let suffix_start = i;
                    while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                        i += 1;
                    }
                    let suffix: String = chars[suffix_start..i].iter().collect();
                    if !matches!(
                        suffix.as_str(),
                        "i64" | "i32" | "u32" | "u64" | "u8" | "usize"
                    ) {
                        return Err(format!("lex: unsupported integer suffix {}", suffix));
                    }
                    // A suffixed literal is CONCRETELY typed: keep the suffix so
                    // the parser can desugar to a cast (`5i64` == `(5 as i64)`).
                    toks.push(Tok::IntSuffixed(n, suffix));
                } else {
                    toks.push(Tok::Int(n));
                }
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                toks.push(keyword_or_ident(word));
            }
            other => return Err(format!("lex: unexpected character {:?}", other)),
        }
    }
    while offsets.len() < toks.len() {
        offsets.push(last_start);
    }
    Ok((toks, offsets))
}

/// Char offset -> 1-based (line, col).
pub fn offset_line_col(src: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    let mut seen = 0usize;
    for c in src.chars() {
        if seen == offset {
            break;
        }
        if c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
        seen += 1;
    }
    (line, col)
}

fn keyword_or_ident(word: String) -> Tok {
    match word.as_str() {
        "fn" => Tok::KwFn,
        "let" => Tok::KwLet,
        "if" => Tok::KwIf,
        "else" => Tok::KwElse,
        "return" => Tok::KwReturn,
        "true" => Tok::KwTrue,
        "false" => Tok::KwFalse,
        "struct" => Tok::KwStruct,
        "enum" => Tok::KwEnum,
        "match" => Tok::KwMatch,
        "while" => Tok::KwWhile,
        "loop" => Tok::KwLoop,
        "for" => Tok::KwFor,
        "in" => Tok::KwIn,
        "break" => Tok::KwBreak,
        "continue" => Tok::KwContinue,
        "mut" => Tok::KwMut,
        "impl" => Tok::KwImpl,
        "use" => Tok::KwUse,
        "pub" => Tok::KwPub,
        "mod" => Tok::KwMod,
        "as" => Tok::KwAs,
        "const" => Tok::KwConst,
        "move" => Tok::KwMove,
        "static" => Tok::KwStatic,
        "trait" => Tok::KwTrait,
        _ => Tok::Ident(word),
    }
}

fn peek(chars: &[char], i: usize) -> Option<char> {
    chars.get(i).copied()
}

fn push(toks: &mut Vec<Tok>, i: &mut usize, t: Tok) {
    toks.push(t);
    *i += 1;
}
