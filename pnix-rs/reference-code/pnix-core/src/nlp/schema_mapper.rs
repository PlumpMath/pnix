//! NLP Schema Mapper - Natural Language to Schema/Morphism Mapping
//!
//! pnix-old의 nlp_schema_mapper.rs를 pnix-new 패러다임에 적응
//!
//! ## Tasks (from pnix-old; legacy "LLM Mode" 어휘는 *superseded*)
//! - Task 638: NL 명사 → Schema 매핑 (legacy 이름 "LLM Mode 명사 → Schema 매핑")
//! - Task 639: NL 동사 → Morphism 매핑 (legacy 이름 "LLM Mode 동사 → Morphism 매핑")
//!
//! **pnix 는 LLM 없이 작동하는 deterministic AI substrate** 다 (`CLAUDE.md`
//! OWNER-LAW CONSTITUTION). 이전 task 이름의 "LLM Mode" 어휘는 owner-law
//! 위반으로 *superseded* — 이 mapper 는 deterministic 한 NL → Schema/Morphism
//! mapping 만 한다.
//!
//! ## pnix-old vs pnix-new
//!
//! | 측면 | pnix-old | pnix-new |
//! |-----|----------|----------|
//! | 타입 | SchemaType | CoreType |
//! | 화살표 | SchemaArrow | FxMorphism |
//! | 그래프 | 없음 | FxCoreModule로 통합 |
//!
//! 헌법 준수: 구조 매핑만, 값 계산 없음, LLM call site 없음.

use crate::types::CoreType;
use std::collections::HashMap;

// ============================================================
// Error Types
// ============================================================

/// NLP 매핑 에러: 자연어를 스키마/모피즘으로 매핑하는 과정에서 발생하는 에러
#[derive(Debug, Clone)]
pub enum NlpMappingError {
  /// 호환되지 않는 타입
  IncompatibleTypes {
    /// 소스 타입
    from: String,
    /// 타겟 타입
    to: String,
  },
  /// 알 수 없는 명사
  UnknownNoun {
    /// 명사 텍스트
    noun: String,
  },
  /// 알 수 없는 동사
  UnknownVerb {
    /// 동사 텍스트
    verb: String,
  },
}

impl std::fmt::Display for NlpMappingError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      NlpMappingError::IncompatibleTypes { from, to } => {
        write!(f, "Incompatible types: {} cannot merge with {}", from, to)
      }
      NlpMappingError::UnknownNoun { noun } => write!(f, "Unknown noun: {}", noun),
      NlpMappingError::UnknownVerb { verb } => write!(f, "Unknown verb: {}", verb),
    }
  }
}

impl std::error::Error for NlpMappingError {}

// ============================================================
// Language Detection
// ============================================================

/// 지원하는 언어: 자연어 처리에서 지원하는 언어 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
  /// 한국어
  Korean,
  /// 영어
  English,
  /// 알 수 없음
  Unknown,
}

// ============================================================
// Noun Extraction (Task 638)
// ============================================================

/// 추출된 명사: 자연어 텍스트에서 추출된 명사 정보
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedNoun {
  /// 원본 텍스트
  pub text: String,
  /// 정규화된 형태 (소문자, 단수형)
  pub normalized: String,
  /// 원본에서의 위치 (단어 인덱스)
  pub position: usize,
  /// 감지된 언어
  pub language: Language,
}

/// 명사 추출 트레잇: 텍스트에서 명사를 추출하는 기능
pub trait NounExtractor {
  /// 텍스트에서 명사 추출
  ///
  /// 헌법 P0-1 준수: 구조 분석만, 값 계산 없음
  fn extract_nouns(&self, text: &str) -> Vec<ExtractedNoun>;
}

/// 명사 → 타입 매핑 결과: 명사를 CoreType으로 매핑한 결과
#[derive(Debug, Clone)]
pub enum NounMappingResult {
  /// 레지스트리에 존재하는 타입
  Existing(CoreType),
  /// 알려진 매핑에서 찾은 타입
  Mapped(CoreType),
  /// 자동 생성된 타입
  Created(CoreType),
  /// 알 수 없는 명사
  Unknown(String),
}

impl NounMappingResult {
  /// 매핑된 타입 반환
  pub fn core_type(&self) -> Option<&CoreType> {
    match self {
      NounMappingResult::Existing(t)
      | NounMappingResult::Mapped(t)
      | NounMappingResult::Created(t) => Some(t),
      NounMappingResult::Unknown(_) => None,
    }
  }

  /// 매핑 성공 여부
  pub fn is_success(&self) -> bool {
    !matches!(self, NounMappingResult::Unknown(_))
  }
}

/// 명사 → CoreType 매퍼: 명사를 CoreType으로 매핑하는 매퍼
///
/// Task 638: NL 명사 → Schema 매핑 (legacy 이름 "LLM Mode 명사 → Schema 매핑";
/// owner-law 위반으로 *superseded* — substrate 안에 LLM Mode 는 없다)
/// 헌법 준수: 구조 매핑만, 값 계산 없음, LLM call site 없음
#[derive(Debug, Clone)]
pub struct NounTypeMapper {
  /// 알려진 명사 → 타입 매핑
  noun_mappings: HashMap<String, CoreType>,
  /// 매핑된 타입 레지스트리
  type_registry: HashMap<String, CoreType>,
  /// 알 수 없는 명사 자동 생성 여부
  auto_create_unknown: bool,
}

impl Default for NounTypeMapper {
  fn default() -> Self {
    let mut noun_mappings = HashMap::new();

    // 한국어 명사 매핑
    noun_mappings.insert("사용자".to_string(), CoreType::named("User"));
    noun_mappings.insert("파일".to_string(), CoreType::named("File"));
    noun_mappings.insert("숫자".to_string(), CoreType::named("Int"));
    noun_mappings.insert("문자열".to_string(), CoreType::named("String"));
    noun_mappings.insert("정수".to_string(), CoreType::named("Int"));
    noun_mappings.insert("실수".to_string(), CoreType::named("Float"));
    noun_mappings.insert("불리언".to_string(), CoreType::named("Bool"));
    noun_mappings.insert("시간".to_string(), CoreType::named("Time"));
    noun_mappings.insert("날짜".to_string(), CoreType::named("Date"));
    noun_mappings.insert("위치".to_string(), CoreType::named("Position"));

    // 영어 명사 매핑
    noun_mappings.insert("user".to_string(), CoreType::named("User"));
    noun_mappings.insert("file".to_string(), CoreType::named("File"));
    noun_mappings.insert("number".to_string(), CoreType::named("Int"));
    noun_mappings.insert("string".to_string(), CoreType::named("String"));
    noun_mappings.insert("text".to_string(), CoreType::named("String"));
    noun_mappings.insert("integer".to_string(), CoreType::named("Int"));
    noun_mappings.insert("real".to_string(), CoreType::named("Float"));
    noun_mappings.insert("boolean".to_string(), CoreType::named("Bool"));
    noun_mappings.insert("time".to_string(), CoreType::named("Time"));
    noun_mappings.insert("date".to_string(), CoreType::named("Date"));
    noun_mappings.insert("position".to_string(), CoreType::named("Position"));

    Self {
      noun_mappings,
      type_registry: HashMap::new(),
      // OWNER-LAW (2026-05-10): default false. 모르는 명사를 자동으로
      // CoreType::named(...) 으로 Created 하면 deterministic hallucination 이다.
      // pnix 는 LLM 없이 작동하는 deterministic AI substrate 이고, 모르는
      // 것은 Held / CandidateTypeNeed 로 emit 한다 — 자동 생성하지 않는다.
      // owner-law proof 가 있는 domain library absorption 에서만
      // `with_auto_create()` 로 켠다.
      auto_create_unknown: false,
    }
  }
}

impl NounTypeMapper {
  /// 새 매퍼 생성
  pub fn new() -> Self {
    Self::default()
  }

  /// 자동 생성 비활성화
  pub fn without_auto_create(mut self) -> Self {
    self.auto_create_unknown = false;
    self
  }

  /// 자동 생성 활성화 (owner-law proof 있는 domain library absorption 전용).
  ///
  /// OWNER-LAW (2026-05-10): default 는 false. unknown noun 을 자동으로
  /// `CoreType::named(...)` 로 Created 하는 것은 deterministic hallucination
  /// 이며 owner-law 위반이다. 이 method 는 owner-law proof 가 닫힌 domain
  /// library absorption 에서만 호출한다.
  pub fn with_auto_create(mut self) -> Self {
    self.auto_create_unknown = true;
    self
  }

  /// 명사를 CoreType으로 매핑
  pub fn map_noun(&mut self, noun: &ExtractedNoun) -> NounMappingResult {
    let normalized = &noun.normalized;

    // 레지스트리 확인
    if let Some(ty) = self.type_registry.get(normalized) {
      return NounMappingResult::Existing(ty.clone());
    }

    // 알려진 매핑 확인
    if let Some(ty) = self.noun_mappings.get(normalized) {
      self.type_registry.insert(normalized.clone(), ty.clone());
      return NounMappingResult::Mapped(ty.clone());
    }

    // 자동 생성
    if self.auto_create_unknown {
      let type_name = to_pascal_case(normalized);
      let ty = CoreType::named(&type_name);
      self.type_registry.insert(normalized.clone(), ty.clone());
      return NounMappingResult::Created(ty);
    }

    NounMappingResult::Unknown(normalized.clone())
  }

  /// 여러 명사 매핑
  pub fn map_nouns<E: NounExtractor>(
    &mut self,
    extractor: &E,
    text: &str,
  ) -> Vec<(ExtractedNoun, NounMappingResult)> {
    let nouns = extractor.extract_nouns(text);
    nouns
      .into_iter()
      .map(|noun| {
        let result = self.map_noun(&noun);
        (noun, result)
      })
      .collect()
  }

  /// 레지스트리 반환
  pub fn registry(&self) -> &HashMap<String, CoreType> {
    &self.type_registry
  }
}

// ============================================================
// Verb Extraction (Task 639)
// ============================================================

/// 추출된 동사: 자연어 텍스트에서 추출된 동사 정보
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedVerb {
  /// 원본 텍스트
  pub text: String,
  /// 정규화된 형태 (원형)
  pub normalized: String,
  /// 원본에서의 위치 (단어 인덱스)
  pub position: usize,
  /// 감지된 언어
  pub language: Language,
  /// 주어 (감지된 경우)
  pub subject: Option<String>,
  /// 목적어 (감지된 경우)
  pub object: Option<String>,
}

/// 동사 추출 트레잇: 텍스트에서 동사를 추출하는 기능
pub trait VerbExtractor {
  /// 텍스트에서 동사 추출
  ///
  /// 헌법 P0-1 준수: 구조 분석만, 값 계산 없음
  fn extract_verbs(&self, text: &str) -> Vec<ExtractedVerb>;
}

/// 화살표 타입: morphism의 분류 타입
///
/// Task 639: NL 동사 → Morphism 매핑 (legacy 이름 "LLM Mode 동사 → Morphism 매핑"; *superseded*)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArrowType {
  /// 변환: A → B (일반 morphism)
  Transform,
  /// 생성: () → A (constructor)
  Create,
  /// 삭제: A → () (destructor)
  Delete,
  /// 업데이트: A → A (endomorphism)
  Update,
  /// 조회: A → B (read-only)
  Query,
  /// 합성: A → B → C
  Compose,
}

/// 매핑된 morphism: 동사에서 매핑된 morphism 정보
#[derive(Debug, Clone)]
pub struct MappedMorphism {
  /// morphism 이름
  pub name: String,
  /// 화살표 타입
  pub arrow_type: ArrowType,
  /// 소스 타입 (주어에서 추출)
  pub source: Option<CoreType>,
  /// 타겟 타입 (목적어에서 추출)
  pub target: Option<CoreType>,
}

/// 동사 → Morphism 매핑 결과: 동사를 Morphism으로 매핑한 결과
#[derive(Debug, Clone)]
pub enum VerbMappingResult {
  /// 레지스트리에 존재하는 morphism
  Existing(MappedMorphism),
  /// 알려진 매핑에서 찾은 morphism
  Mapped(MappedMorphism),
  /// 자동 생성된 morphism (owner-law proof 있는 domain absorption 에서만 emit)
  Created(MappedMorphism),
  /// OWNER-LAW (2026-05-10): unknown verb. default 모드에서 모르는 동사를
  /// 자동으로 `Transform` morphism 으로 생성하면 deterministic hallucination
  /// 이다. unknown 은 owner-law gate / Held / CandidateMorphismNeed lane 으로
  /// 보낸다.
  Unknown(String),
}

impl VerbMappingResult {
  /// Morphism 반환 (Unknown 이면 None)
  pub fn morphism(&self) -> Option<&MappedMorphism> {
    match self {
      VerbMappingResult::Existing(m)
      | VerbMappingResult::Mapped(m)
      | VerbMappingResult::Created(m) => Some(m),
      VerbMappingResult::Unknown(_) => None,
    }
  }

  /// 매핑 성공 여부 (Unknown 이 아니면 true)
  pub fn is_mapped(&self) -> bool {
    !matches!(self, VerbMappingResult::Unknown(_))
  }
}

/// 동사 → Morphism 매퍼: 동사를 Morphism으로 매핑하는 매퍼
///
/// Task 639: NL 동사 → Morphism 매핑 (legacy 이름 "LLM Mode 동사 → Morphism 매핑"; *superseded*)
/// 헌법 P0-1 준수: 구조 매핑만, 값 계산 없음
#[derive(Debug, Clone)]
pub struct VerbMorphismMapper {
  /// 알려진 동사 → 화살표 타입 매핑
  verb_mappings: HashMap<String, ArrowType>,
  /// 매핑된 morphism 레지스트리
  morphism_registry: HashMap<String, MappedMorphism>,
  /// OWNER-LAW (2026-05-10): default false. 모르는 동사를 자동으로 `Transform`
  /// morphism 으로 Created 하면 deterministic hallucination. owner-law proof 가
  /// 있는 domain library absorption 에서만 `with_auto_create()` 로 켠다.
  auto_create_unknown: bool,
}

impl VerbMorphismMapper {
  /// 자동 생성 비활성화 (default)
  pub fn without_auto_create(mut self) -> Self {
    self.auto_create_unknown = false;
    self
  }

  /// 자동 생성 활성화 (owner-law proof 있는 domain absorption 전용)
  ///
  /// OWNER-LAW (2026-05-10): default 는 false. unknown verb 를 자동으로
  /// `Transform` morphism 으로 Created 하는 것은 deterministic hallucination
  /// 이며 owner-law 위반이다.
  pub fn with_auto_create(mut self) -> Self {
    self.auto_create_unknown = true;
    self
  }
}

impl Default for VerbMorphismMapper {
  fn default() -> Self {
    let mut verb_mappings = HashMap::new();

    // 한국어 동사 매핑
    verb_mappings.insert("저장하다".to_string(), ArrowType::Create);
    verb_mappings.insert("생성하다".to_string(), ArrowType::Create);
    verb_mappings.insert("만들다".to_string(), ArrowType::Create);
    verb_mappings.insert("삭제하다".to_string(), ArrowType::Delete);
    verb_mappings.insert("제거하다".to_string(), ArrowType::Delete);
    verb_mappings.insert("수정하다".to_string(), ArrowType::Update);
    verb_mappings.insert("변경하다".to_string(), ArrowType::Update);
    verb_mappings.insert("읽다".to_string(), ArrowType::Query);
    verb_mappings.insert("조회하다".to_string(), ArrowType::Query);
    verb_mappings.insert("변환하다".to_string(), ArrowType::Transform);

    // 영어 동사 매핑
    verb_mappings.insert("save".to_string(), ArrowType::Create);
    verb_mappings.insert("create".to_string(), ArrowType::Create);
    verb_mappings.insert("make".to_string(), ArrowType::Create);
    verb_mappings.insert("delete".to_string(), ArrowType::Delete);
    verb_mappings.insert("remove".to_string(), ArrowType::Delete);
    verb_mappings.insert("update".to_string(), ArrowType::Update);
    verb_mappings.insert("modify".to_string(), ArrowType::Update);
    verb_mappings.insert("change".to_string(), ArrowType::Update);
    verb_mappings.insert("read".to_string(), ArrowType::Query);
    verb_mappings.insert("get".to_string(), ArrowType::Query);
    verb_mappings.insert("fetch".to_string(), ArrowType::Query);
    verb_mappings.insert("transform".to_string(), ArrowType::Transform);
    verb_mappings.insert("convert".to_string(), ArrowType::Transform);

    Self {
      verb_mappings,
      morphism_registry: HashMap::new(),
      // OWNER-LAW (2026-05-10): default false. unknown verb 자동 Transform
      // 생성은 deterministic hallucination. owner-law proof 있는 domain
      // library absorption 에서만 `with_auto_create()` 로 켠다.
      auto_create_unknown: false,
    }
  }
}

impl VerbMorphismMapper {
  /// 새 매퍼 생성
  pub fn new() -> Self {
    Self::default()
  }

  /// 동사를 Morphism으로 매핑
  pub fn map_verb(&mut self, verb: &ExtractedVerb) -> VerbMappingResult {
    let normalized = &verb.normalized;

    // 레지스트리 확인
    if let Some(morphism) = self.morphism_registry.get(normalized) {
      return VerbMappingResult::Existing(morphism.clone());
    }

    // 알려진 매핑 확인
    if let Some(&arrow_type) = self.verb_mappings.get(normalized) {
      let morphism = MappedMorphism {
        name: normalized.clone(),
        arrow_type,
        source: None,
        target: None,
      };
      self
        .morphism_registry
        .insert(normalized.clone(), morphism.clone());
      return VerbMappingResult::Mapped(morphism);
    }

    // OWNER-LAW (2026-05-10): unknown verb 는 default 로 Unknown 반환.
    // owner-law proof 있는 domain absorption 에서만 `with_auto_create()` 로
    // 자동 Transform 생성을 켠다.
    if self.auto_create_unknown {
      let morphism = MappedMorphism {
        name: normalized.clone(),
        arrow_type: ArrowType::Transform,
        source: None,
        target: None,
      };
      self
        .morphism_registry
        .insert(normalized.clone(), morphism.clone());
      return VerbMappingResult::Created(morphism);
    }

    VerbMappingResult::Unknown(normalized.clone())
  }

  /// 명사 매퍼 컨텍스트와 함께 동사 매핑
  pub fn map_verb_with_context(
    &mut self,
    verb: &ExtractedVerb,
    noun_mapper: &NounTypeMapper,
  ) -> VerbMappingResult {
    let mut result = self.map_verb(verb);

    // source/target 추가
    if let VerbMappingResult::Existing(ref mut m)
    | VerbMappingResult::Mapped(ref mut m)
    | VerbMappingResult::Created(ref mut m) = result
    {
      if let Some(ref subject) = verb.subject {
        m.source = noun_mapper.registry().get(subject).cloned();
      }
      if let Some(ref object) = verb.object {
        m.target = noun_mapper.registry().get(object).cloned();
      }
    }

    result
  }

  /// 레지스트리 반환
  pub fn registry(&self) -> &HashMap<String, MappedMorphism> {
    &self.morphism_registry
  }
}

// ============================================================
// Simple Rule-Based Extractor
// ============================================================

/// 간단한 규칙 기반 명사/동사 추출기 (한국어/영어)
///
/// 헌법 P0-1 준수: 구조 분석만, 값 계산 없음
#[derive(Debug, Clone)]
pub struct SimpleExtractor {
  /// 한국어 명사 접미사 (조사)
  korean_noun_suffixes: Vec<&'static str>,
  /// 한국어 동사 접미사
  korean_verb_suffixes: Vec<&'static str>,
  /// 영어 불용어
  english_stopwords: Vec<&'static str>,
}

impl Default for SimpleExtractor {
  fn default() -> Self {
    Self {
      korean_noun_suffixes: vec!["가", "는", "을", "를", "의", "에", "로", "와", "과"],
      korean_verb_suffixes: vec!["한다", "하다", "된다", "되다", "시킨다", "시키다"],
      english_stopwords: vec![
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had",
        "do", "does", "did", "will", "would", "could", "should", "may", "might", "must", "shall",
        "can", "to", "of", "in", "for", "on", "with", "at", "by", "from", "as", "into", "through",
      ],
    }
  }
}

impl SimpleExtractor {
  /// 새 추출기 생성
  pub fn new() -> Self {
    Self::default()
  }

  /// 언어 감지
  pub fn detect_language(&self, text: &str) -> Language {
    for ch in text.chars() {
      // 한글 유니코드 범위
      if ('\u{AC00}'..='\u{D7A3}').contains(&ch) || ('\u{1100}'..='\u{11FF}').contains(&ch) {
        return Language::Korean;
      }
    }
    Language::English
  }

  /// 한국어 명사 추출
  fn extract_korean_nouns(&self, text: &str) -> Vec<ExtractedNoun> {
    let mut nouns = Vec::new();
    let words: Vec<&str> = text.split_whitespace().collect();

    for (i, word) in words.iter().enumerate() {
      for suffix in &self.korean_noun_suffixes {
        if word.ends_with(suffix) {
          // Fix: Use char-based slicing instead of byte slicing for Korean text
          // Hangul characters are 3 bytes in UTF-8, so byte slicing can panic on misalignment
          let suffix_char_len = suffix.chars().count();
          let word_char_len = word.chars().count();
          if word_char_len >= suffix_char_len {
            let noun_text: String = word.chars().take(word_char_len - suffix_char_len).collect();
            if !noun_text.is_empty() {
              nouns.push(ExtractedNoun {
                text: noun_text.clone(),
                normalized: noun_text.to_lowercase(),
                position: i,
                language: Language::Korean,
              });
            }
          }
          break;
        }
      }
    }

    nouns
  }

  /// 영어 명사 추출
  fn extract_english_nouns(&self, text: &str) -> Vec<ExtractedNoun> {
    let mut nouns = Vec::new();
    let words: Vec<&str> = text.split_whitespace().collect();

    for (i, word) in words.iter().enumerate() {
      let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric());
      let lower = clean_word.to_lowercase();

      // 불용어와 짧은 단어 제외
      if clean_word.len() < 2 || self.english_stopwords.contains(&lower.as_str()) {
        continue;
      }

      // 대문자로 시작하거나 동사 접미사가 없는 단어
      if clean_word.chars().next().is_some_and(|c| c.is_uppercase())
        || (!lower.ends_with("ing")
          && !lower.ends_with("ed")
          && !lower.ends_with("es")
          && !lower.ends_with("s"))
      {
        nouns.push(ExtractedNoun {
          text: clean_word.to_string(),
          normalized: lower,
          position: i,
          language: Language::English,
        });
      }
    }

    nouns
  }

  /// 한국어 동사 추출
  fn extract_korean_verbs(&self, text: &str) -> Vec<ExtractedVerb> {
    let mut verbs = Vec::new();
    let words: Vec<&str> = text.split_whitespace().collect();

    for (i, word) in words.iter().enumerate() {
      for suffix in &self.korean_verb_suffixes {
        if word.ends_with(suffix) {
          // Fix: Use char-based slicing instead of byte slicing for Korean text
          // Hangul characters are 3 bytes in UTF-8, so byte slicing can panic on misalignment
          let suffix_char_len = suffix.chars().count();
          let word_char_len = word.chars().count();
          if word_char_len >= suffix_char_len {
            let verb_stem: String = word.chars().take(word_char_len - suffix_char_len).collect();
            if !verb_stem.is_empty() {
              verbs.push(ExtractedVerb {
                text: word.to_string(),
                normalized: format!("{}하다", verb_stem),
                position: i,
                language: Language::Korean,
                subject: None,
                object: None,
              });
            }
          }
          break;
        }
      }
    }

    verbs
  }

  /// 영어 동사 추출
  fn extract_english_verbs(&self, text: &str) -> Vec<ExtractedVerb> {
    let mut verbs = Vec::new();
    let words: Vec<&str> = text.split_whitespace().collect();
    let default_config = VerbMorphismMapper::default();

    for (i, word) in words.iter().enumerate() {
      let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric());
      let lower = clean_word.to_lowercase();

      if clean_word.len() < 2 {
        continue;
      }

      // 동사 원형 추출
      let normalized = if lower.ends_with("ing") && lower.len() > 3 {
        lower[..lower.len() - 3].to_string()
      } else if (lower.ends_with("ed") || lower.ends_with("es")) && lower.len() > 2 {
        lower[..lower.len() - 2].to_string()
      } else if lower.ends_with("s") && lower.len() > 2 {
        lower[..lower.len() - 1].to_string()
      } else {
        lower.clone()
      };

      // 알려진 동사인지 확인
      if default_config.verb_mappings.contains_key(&normalized) {
        verbs.push(ExtractedVerb {
          text: clean_word.to_string(),
          normalized,
          position: i,
          language: Language::English,
          subject: None,
          object: None,
        });
      }
    }

    verbs
  }
}

impl NounExtractor for SimpleExtractor {
  fn extract_nouns(&self, text: &str) -> Vec<ExtractedNoun> {
    match self.detect_language(text) {
      Language::Korean => self.extract_korean_nouns(text),
      Language::English | Language::Unknown => self.extract_english_nouns(text),
    }
  }
}

impl VerbExtractor for SimpleExtractor {
  fn extract_verbs(&self, text: &str) -> Vec<ExtractedVerb> {
    match self.detect_language(text) {
      Language::Korean => self.extract_korean_verbs(text),
      Language::English | Language::Unknown => self.extract_english_verbs(text),
    }
  }
}

// ============================================================
// Utility Functions
// ============================================================

/// snake_case로 변환
///
/// 문자열을 snake_case 형식으로 변환합니다.
pub fn to_snake_case(s: &str) -> String {
  let mut result = String::new();
  for (i, ch) in s.chars().enumerate() {
    if ch.is_uppercase() && i > 0 {
      result.push('_');
    }
    result.push(ch.to_lowercase().next().unwrap_or(ch));
  }
  result.replace([' ', '-'], "_")
}

/// PascalCase로 변환
///
/// 문자열을 PascalCase 형식으로 변환합니다.
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
/// 문자열을 kebab-case 형식으로 변환합니다.
pub fn to_kebab_case(s: &str) -> String {
  let mut result = String::new();
  for (i, ch) in s.chars().enumerate() {
    if ch.is_uppercase() && i > 0 {
      result.push('-');
    }
    result.push(ch.to_lowercase().next().unwrap_or(ch));
  }
  result.replace([' ', '_'], "-")
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
  use super::*;

  // Task 638 Tests: Noun to Type Mapping

  #[test]
  fn test_noun_type_mapper_default() {
    let mapper = NounTypeMapper::new();
    assert!(!mapper.noun_mappings.is_empty());
  }

  #[test]
  fn test_map_known_korean_noun() {
    let mut mapper = NounTypeMapper::new();
    let noun = ExtractedNoun {
      text: "사용자".to_string(),
      normalized: "사용자".to_string(),
      position: 0,
      language: Language::Korean,
    };

    let result = mapper.map_noun(&noun);
    assert!(result.is_success());
    assert!(matches!(result, NounMappingResult::Mapped(_)));
  }

  #[test]
  fn test_map_known_english_noun() {
    let mut mapper = NounTypeMapper::new();
    let noun = ExtractedNoun {
      text: "user".to_string(),
      normalized: "user".to_string(),
      position: 0,
      language: Language::English,
    };

    let result = mapper.map_noun(&noun);
    assert!(result.is_success());
    assert!(matches!(result, NounMappingResult::Mapped(_)));
  }

  #[test]
  fn test_map_unknown_noun_auto_create() {
    let mut mapper = NounTypeMapper::new();
    let noun = ExtractedNoun {
      text: "CustomEntity".to_string(),
      normalized: "customentity".to_string(),
      position: 0,
      language: Language::English,
    };

    let result = mapper.map_noun(&noun);
    assert!(result.is_success());
    assert!(matches!(result, NounMappingResult::Created(_)));
  }

  #[test]
  fn test_map_unknown_noun_no_auto_create() {
    let mut mapper = NounTypeMapper::new().without_auto_create();
    let noun = ExtractedNoun {
      text: "CustomEntity".to_string(),
      normalized: "customentity".to_string(),
      position: 0,
      language: Language::English,
    };

    let result = mapper.map_noun(&noun);
    assert!(!result.is_success());
    assert!(matches!(result, NounMappingResult::Unknown(_)));
  }

  #[test]
  fn test_noun_registry_caching() {
    let mut mapper = NounTypeMapper::new();
    let noun = ExtractedNoun {
      text: "user".to_string(),
      normalized: "user".to_string(),
      position: 0,
      language: Language::English,
    };

    let result1 = mapper.map_noun(&noun);
    assert!(matches!(result1, NounMappingResult::Mapped(_)));

    let result2 = mapper.map_noun(&noun);
    assert!(matches!(result2, NounMappingResult::Existing(_)));
  }

  // Task 639 Tests: Verb to Morphism Mapping

  #[test]
  fn test_verb_morphism_mapper_default() {
    let mapper = VerbMorphismMapper::new();
    assert!(!mapper.verb_mappings.is_empty());
  }

  #[test]
  fn test_map_known_korean_verb() {
    let mut mapper = VerbMorphismMapper::new();
    let verb = ExtractedVerb {
      text: "저장한다".to_string(),
      normalized: "저장하다".to_string(),
      position: 0,
      language: Language::Korean,
      subject: None,
      object: None,
    };

    let result = mapper.map_verb(&verb);
    assert!(matches!(result, VerbMappingResult::Mapped(_)));
    assert_eq!(result.morphism().unwrap().arrow_type, ArrowType::Create);
  }

  #[test]
  fn test_map_known_english_verb() {
    let mut mapper = VerbMorphismMapper::new();
    let verb = ExtractedVerb {
      text: "save".to_string(),
      normalized: "save".to_string(),
      position: 0,
      language: Language::English,
      subject: None,
      object: None,
    };

    let result = mapper.map_verb(&verb);
    assert!(matches!(result, VerbMappingResult::Mapped(_)));
    assert_eq!(result.morphism().unwrap().arrow_type, ArrowType::Create);
  }

  #[test]
  fn test_map_delete_verb() {
    let mut mapper = VerbMorphismMapper::new();
    let verb = ExtractedVerb {
      text: "delete".to_string(),
      normalized: "delete".to_string(),
      position: 0,
      language: Language::English,
      subject: None,
      object: None,
    };

    let result = mapper.map_verb(&verb);
    assert_eq!(result.morphism().unwrap().arrow_type, ArrowType::Delete);
  }

  #[test]
  fn test_map_query_verb() {
    let mut mapper = VerbMorphismMapper::new();
    let verb = ExtractedVerb {
      text: "read".to_string(),
      normalized: "read".to_string(),
      position: 0,
      language: Language::English,
      subject: None,
      object: None,
    };

    let result = mapper.map_verb(&verb);
    assert_eq!(result.morphism().unwrap().arrow_type, ArrowType::Query);
  }

  #[test]
  fn test_map_unknown_verb_creates_transform() {
    let mut mapper = VerbMorphismMapper::new();
    let verb = ExtractedVerb {
      text: "randomize".to_string(),
      normalized: "randomize".to_string(),
      position: 0,
      language: Language::English,
      subject: None,
      object: None,
    };

    let result = mapper.map_verb(&verb);
    assert!(matches!(result, VerbMappingResult::Created(_)));
    assert_eq!(result.morphism().unwrap().arrow_type, ArrowType::Transform);
  }

  // Simple Extractor Tests

  #[test]
  fn test_detect_korean() {
    let extractor = SimpleExtractor::new();
    assert_eq!(
      extractor.detect_language("사용자가 파일을 저장한다"),
      Language::Korean
    );
  }

  #[test]
  fn test_detect_english() {
    let extractor = SimpleExtractor::new();
    assert_eq!(
      extractor.detect_language("The user saves a file"),
      Language::English
    );
  }

  #[test]
  fn test_extract_korean_nouns() {
    let extractor = SimpleExtractor::new();
    let nouns = extractor.extract_nouns("사용자가 파일을 저장한다");

    assert!(!nouns.is_empty());
    let noun_texts: Vec<&str> = nouns.iter().map(|n| n.text.as_str()).collect();
    assert!(noun_texts.contains(&"사용자"));
    assert!(noun_texts.contains(&"파일"));
  }

  #[test]
  fn test_extract_korean_verbs() {
    let extractor = SimpleExtractor::new();
    let verbs = extractor.extract_verbs("사용자가 파일을 저장한다");

    assert!(!verbs.is_empty());
    assert!(verbs.iter().any(|v| v.normalized.contains("저장")));
  }

  // Utility Function Tests

  #[test]
  fn test_to_snake_case() {
    assert_eq!(to_snake_case("UserProfile"), "user_profile");
    assert_eq!(to_snake_case("HelloWorld"), "hello_world");
    assert_eq!(to_snake_case("simple"), "simple");
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
  }

  // Integration Tests

  #[test]
  fn test_full_pipeline_korean() {
    let extractor = SimpleExtractor::new();
    let mut noun_mapper = NounTypeMapper::new();
    let mut verb_mapper = VerbMorphismMapper::new();

    let text = "사용자가 파일을 저장한다";

    // Extract and map nouns
    let noun_results = noun_mapper.map_nouns(&extractor, text);
    assert!(!noun_results.is_empty());

    // Extract and map verbs
    let verbs = extractor.extract_verbs(text);
    for verb in verbs {
      let result = verb_mapper.map_verb_with_context(&verb, &noun_mapper);
      assert!(matches!(
        result,
        VerbMappingResult::Mapped(_) | VerbMappingResult::Created(_)
      ));
    }
  }

  #[test]
  fn test_full_pipeline_english() {
    let extractor = SimpleExtractor::new();
    let mut noun_mapper = NounTypeMapper::new();
    let mut verb_mapper = VerbMorphismMapper::new();

    let text = "User saves the file";

    // Extract and map nouns
    let noun_results = noun_mapper.map_nouns(&extractor, text);
    assert!(!noun_results.is_empty());

    // Extract and map verbs
    let verbs = extractor.extract_verbs(text);
    for verb in verbs {
      let result = verb_mapper.map_verb(&verb);
      if verb.normalized == "save" {
        assert_eq!(result.morphism().unwrap().arrow_type, ArrowType::Create);
      }
    }
  }
}
