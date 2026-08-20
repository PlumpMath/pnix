//! Small display-width helpers for diagnostics and editor adapters.

pub fn char_width(ch: char) -> usize {
  let cp = ch as u32;
  if ch == '\0' || is_combining_mark(cp) {
    return 0;
  }
  if ch == '\t' {
    return 1;
  }
  if ch.is_control() {
    return 1;
  }
  if is_wide(cp) {
    return 2;
  }
  1
}

pub fn str_width(text: &str) -> usize {
  text.chars().map(char_width).sum()
}

pub fn grapheme_slices(text: &str) -> Vec<&str> {
  let mut slices = Vec::new();
  let mut iter = text.char_indices().peekable();
  while let Some((start, ch)) = iter.next() {
    let mut end = start + ch.len_utf8();
    let mut include_next = false;
    while let Some(&(idx, next)) = iter.peek() {
      if include_next || is_grapheme_extend(next as u32) || next == '\u{200d}' {
        let (_, consumed) = iter.next().expect("peeked char");
        end = idx + consumed.len_utf8();
        include_next = consumed == '\u{200d}';
      } else {
        break;
      }
    }
    slices.push(&text[start..end]);
  }
  slices
}

fn is_combining_mark(cp: u32) -> bool {
  is_grapheme_extend(cp)
}

fn is_grapheme_extend(cp: u32) -> bool {
  matches!(
    cp,
    0x0300..=0x036f
      | 0x1ab0..=0x1aff
      | 0x1dc0..=0x1dff
      | 0x20d0..=0x20ff
      | 0xfe00..=0xfe0f
      | 0xfe20..=0xfe2f
      | 0x1f3fb..=0x1f3ff
  )
}

fn is_wide(cp: u32) -> bool {
  matches!(
    cp,
    0x1100..=0x115f
      | 0x231a..=0x231b
      | 0x2329..=0x232a
      | 0x23e9..=0x23ec
      | 0x23f0
      | 0x23f3
      | 0x25fd..=0x25fe
      | 0x2600..=0x27bf
      | 0x2b50
      | 0x2b55
      | 0x2e80..=0xa4cf
      | 0xac00..=0xd7a3
      | 0xf900..=0xfaff
      | 0xfe10..=0xfe19
      | 0xfe30..=0xfe6f
      | 0xff00..=0xff60
      | 0xffe0..=0xffe6
      | 0x1f000..=0x1faff
      | 0x20000..=0x3fffd
  )
}

#[cfg(test)]
mod tests {
  use super::{char_width, grapheme_slices, str_width};

  #[test]
  fn width_matches_project_diagnostics_expectations() {
    assert_eq!(char_width('a'), 1);
    assert_eq!(char_width('λ'), 1);
    assert_eq!(char_width('한'), 2);
    assert_eq!(char_width('😀'), 2);
    assert_eq!(char_width('\u{0301}'), 0);
    assert_eq!(str_width("한😀a"), 5);
  }

  #[test]
  fn grapheme_slices_keep_combining_and_zwj_clusters_together() {
    assert_eq!(grapheme_slices("e\u{0301}x"), vec!["e\u{0301}", "x"]);
    assert_eq!(grapheme_slices("a👩‍💻b"), vec!["a", "👩‍💻", "b"]);
  }
}
