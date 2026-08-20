//! Korean grammar-oriented analysis primitives for ontology lowering.
//!
//! This module does not execute tools. It extracts stable language structure
//! that higher ontology layers can persist as canonical semantic facts.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HangulSyllable {
  pub syllable: char,
  pub choseong: char,
  pub jungseong: char,
  pub jongseong: Option<char>,
}

impl HangulSyllable {
  pub fn jamo_string(&self) -> String {
    match self.jongseong {
      Some(jongseong) => format!("{}+{}+{}", self.choseong, self.jungseong, jongseong),
      None => format!("{}+{}", self.choseong, self.jungseong),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KoreanParticleKind {
  Subject,
  Topic,
  Object,
  Locative,
  Genitive,
  Conjunctive,
  Instrumental,
  Dative,
}

impl KoreanParticleKind {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Subject => "subject",
      Self::Topic => "topic",
      Self::Object => "object",
      Self::Locative => "locative",
      Self::Genitive => "genitive",
      Self::Conjunctive => "conjunctive",
      Self::Instrumental => "instrumental",
      Self::Dative => "dative",
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KoreanParticleHit {
  pub token: String,
  pub stem: String,
  pub surface: String,
  pub kind: KoreanParticleKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KoreanSentenceMood {
  Declarative,
  Interrogative,
  Imperative,
  Propositive,
  Unknown,
}

impl KoreanSentenceMood {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Declarative => "declarative",
      Self::Interrogative => "interrogative",
      Self::Imperative => "imperative",
      Self::Propositive => "propositive",
      Self::Unknown => "unknown",
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KoreanTextAnalysis {
  pub original: String,
  pub normalized: String,
  pub tokens: Vec<String>,
  pub hangul_syllables: Vec<HangulSyllable>,
  pub particles: Vec<KoreanParticleHit>,
  pub sentence_mood: KoreanSentenceMood,
  pub final_token: Option<String>,
}

const HANGUL_SYLLABLE_BASE: u32 = 0xAC00;
const HANGUL_SYLLABLE_END: u32 = 0xD7A3;
const HANGUL_N_COUNT: u32 = 21 * 28;
const HANGUL_T_COUNT: u32 = 28;

const CHOSEONG: [char; 19] = [
  'ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ', 'ㅊ', 'ㅋ',
  'ㅌ', 'ㅍ', 'ㅎ',
];
const JUNGSEONG: [char; 21] = [
  'ㅏ', 'ㅐ', 'ㅑ', 'ㅒ', 'ㅓ', 'ㅔ', 'ㅕ', 'ㅖ', 'ㅗ', 'ㅘ', 'ㅙ', 'ㅚ', 'ㅛ', 'ㅜ', 'ㅝ', 'ㅞ',
  'ㅟ', 'ㅠ', 'ㅡ', 'ㅢ', 'ㅣ',
];
const JONGSEONG: [Option<char>; 28] = [
  None,
  Some('ㄱ'),
  Some('ㄲ'),
  Some('ㄳ'),
  Some('ㄴ'),
  Some('ㄵ'),
  Some('ㄶ'),
  Some('ㄷ'),
  Some('ㄹ'),
  Some('ㄺ'),
  Some('ㄻ'),
  Some('ㄼ'),
  Some('ㄽ'),
  Some('ㄾ'),
  Some('ㄿ'),
  Some('ㅀ'),
  Some('ㅁ'),
  Some('ㅂ'),
  Some('ㅄ'),
  Some('ㅅ'),
  Some('ㅆ'),
  Some('ㅇ'),
  Some('ㅈ'),
  Some('ㅊ'),
  Some('ㅋ'),
  Some('ㅌ'),
  Some('ㅍ'),
  Some('ㅎ'),
];

const PARTICLE_SUFFIXES: [(&str, KoreanParticleKind); 15] = [
  ("에게서", KoreanParticleKind::Dative),
  ("에서는", KoreanParticleKind::Locative),
  ("에게", KoreanParticleKind::Dative),
  ("에서", KoreanParticleKind::Locative),
  ("으로", KoreanParticleKind::Instrumental),
  ("와", KoreanParticleKind::Conjunctive),
  ("과", KoreanParticleKind::Conjunctive),
  ("을", KoreanParticleKind::Object),
  ("를", KoreanParticleKind::Object),
  ("은", KoreanParticleKind::Topic),
  ("는", KoreanParticleKind::Topic),
  ("이", KoreanParticleKind::Subject),
  ("가", KoreanParticleKind::Subject),
  ("의", KoreanParticleKind::Genitive),
  ("에", KoreanParticleKind::Locative),
];

const TOKEN_TRIM_CHARS: &[char] = &[
  ' ', '\t', '\n', '\r', '.', ',', '!', '?', ':', ';', '"', '\'', '(', ')', '[', ']', '{', '}',
];

pub fn decompose_hangul_syllable(ch: char) -> Option<HangulSyllable> {
  let codepoint = ch as u32;
  if !(HANGUL_SYLLABLE_BASE..=HANGUL_SYLLABLE_END).contains(&codepoint) {
    return None;
  }

  let syllable_index = codepoint - HANGUL_SYLLABLE_BASE;
  let choseong_index = (syllable_index / HANGUL_N_COUNT) as usize;
  let jungseong_index = ((syllable_index % HANGUL_N_COUNT) / HANGUL_T_COUNT) as usize;
  let jongseong_index = (syllable_index % HANGUL_T_COUNT) as usize;

  Some(HangulSyllable {
    syllable: ch,
    choseong: CHOSEONG[choseong_index],
    jungseong: JUNGSEONG[jungseong_index],
    jongseong: JONGSEONG[jongseong_index],
  })
}

pub fn analyze_korean_text(input: &str) -> KoreanTextAnalysis {
  let tokens = input
    .split_whitespace()
    .map(trim_token)
    .filter(|token| !token.is_empty())
    .collect::<Vec<_>>();
  let normalized = if tokens.is_empty() {
    input.trim().to_string()
  } else {
    tokens.join(" ")
  };
  let hangul_syllables = normalized
    .chars()
    .filter_map(decompose_hangul_syllable)
    .collect::<Vec<_>>();
  let particles = tokens
    .iter()
    .filter_map(|token| detect_particle(token))
    .collect::<Vec<_>>();
  let final_token = tokens.last().cloned();
  let sentence_mood = detect_sentence_mood(input, final_token.as_deref());

  KoreanTextAnalysis {
    original: input.to_string(),
    normalized,
    tokens,
    hangul_syllables,
    particles,
    sentence_mood,
    final_token,
  }
}

fn trim_token(token: &str) -> String {
  token.trim_matches(TOKEN_TRIM_CHARS).to_string()
}

fn detect_particle(token: &str) -> Option<KoreanParticleHit> {
  for (surface, kind) in PARTICLE_SUFFIXES {
    if let Some(stem) = token.strip_suffix(surface) {
      if !stem.is_empty() {
        return Some(KoreanParticleHit {
          token: token.to_string(),
          stem: stem.to_string(),
          surface: surface.to_string(),
          kind,
        });
      }
    }
  }
  None
}

fn detect_sentence_mood(input: &str, final_token: Option<&str>) -> KoreanSentenceMood {
  let trimmed = input.trim();
  let final_token = final_token.unwrap_or_default();

  if trimmed.ends_with('?')
    || matches!(
      final_token,
      "누구야" | "뭐야" | "어때" | "왜야" | "일까" | "일까요"
    )
  {
    return KoreanSentenceMood::Interrogative;
  }

  if matches!(final_token, "가자" | "하자" | "보자") {
    return KoreanSentenceMood::Propositive;
  }

  if final_token.ends_with("해")
    || final_token.ends_with("줘")
    || final_token.ends_with("그어")
    || final_token.ends_with("만들어")
    || final_token.ends_with("열어")
    || final_token.ends_with("풀어줘")
  {
    return KoreanSentenceMood::Imperative;
  }

  if final_token.ends_with("다")
    || final_token.ends_with("야")
    || final_token.ends_with("입니다")
    || final_token.ends_with("이다")
  {
    return KoreanSentenceMood::Declarative;
  }

  KoreanSentenceMood::Unknown
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn decompose_hangul_syllable_returns_modern_jamo() {
    let syllable = decompose_hangul_syllable('한').expect("hangul syllable");

    assert_eq!(syllable.choseong, 'ㅎ');
    assert_eq!(syllable.jungseong, 'ㅏ');
    assert_eq!(syllable.jongseong, Some('ㄴ'));
    assert_eq!(syllable.jamo_string(), "ㅎ+ㅏ+ㄴ");
  }

  #[test]
  fn analyze_korean_text_detects_particles_and_mood() {
    let analysis = analyze_korean_text("라이노에서 선 그어");

    assert_eq!(analysis.sentence_mood, KoreanSentenceMood::Imperative);
    assert_eq!(analysis.final_token.as_deref(), Some("그어"));
    assert!(analysis.hangul_syllables.len() >= 3);
    assert!(analysis
      .particles
      .iter()
      .any(|particle| particle.kind == KoreanParticleKind::Locative));
    assert!(analysis
      .particles
      .iter()
      .any(|particle| particle.token == "라이노에서"));
  }

  #[test]
  fn analyze_korean_text_detects_interrogative_structure() {
    let analysis = analyze_korean_text("아인슈타인 퍼즐에서 물고기를 기르는 사람은 누구야?");

    assert_eq!(analysis.sentence_mood, KoreanSentenceMood::Interrogative);
    assert!(analysis
      .particles
      .iter()
      .any(|particle| particle.kind == KoreanParticleKind::Locative));
    assert!(analysis
      .particles
      .iter()
      .any(|particle| particle.kind == KoreanParticleKind::Object));
    assert!(analysis
      .particles
      .iter()
      .any(|particle| particle.kind == KoreanParticleKind::Topic));
  }
}
