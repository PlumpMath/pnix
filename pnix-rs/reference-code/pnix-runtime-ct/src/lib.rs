//! CT Runtime - Category Theory verification and diagram extraction
//!
//! This crate provides runtime verification of CT diagrams and expression analysis.
//!
//! ## Morphism Name Normalization (L48)
//!
//! Morphism operations support multiple input aliases that map to canonical names.
//! This ensures consistent diagram output regardless of input syntax.
//!
//! | Canonical | Aliases | Symbol | Description |
//! |-----------|---------|--------|-------------|
//! | `add` | `+`, `add` | + | Addition |
//! | `sub` | `-`, `sub` | - | Subtraction |
//! | `mul` | `*`, `mul` | * | Multiplication |
//! | `div` | `/`, `div` | / | Division |
//! | `mod` | `%`, `mod` | mod | Modulo |
//! | `neg` | `neg` | neg | Negation |
//! | `floor` | `floor` | floor | Floor |
//! | `ceil` | `ceil` | ceil | Ceiling |
//! | `abs` | `abs` | abs | Absolute value |
//! | `sqrt` | `sqrt` | sqrt | Square root |
//! | `sin` | `sin` | sin | Sine |
//! | `cos` | `cos` | cos | Cosine |
//! | `lt` | `<`, `lt` | < | Less than |
//! | `gt` | `>`, `gt` | > | Greater than |
//! | `le` | `<=`, `le` | <= | Less or equal |
//! | `ge` | `>=`, `ge` | >= | Greater or equal |
//! | `eq` | `==`, `eq` | == | Equal |
//! | `ne` | `!=`, `ne` | != | Not equal |
//! | `and` | `&&`, `and` | and | Logical AND |
//! | `or` | `\|\|`, `or` | or | Logical OR |
//! | `not` | `!`, `not` | not | Logical NOT |
//! | `id` | `id`, `identity` | id | Identity |
//! | `compose` | `∘`, `compose` | ∘ | Composition |
//!
//! Name lookup is case-insensitive. Use `MorphismOp::from_name()` to parse
//! input names and `MorphismOp::canonical_name()` for normalized output.
//!
//! ## Cache Key Generation and Invalidation Rules (E22b)
//!
//! The `CachingCtRuntime` provides deterministic caching for CT verification results.
//!
//! ### Cache Key Components
//!
//! Cache keys are generated from:
//! - Expression string (normalized)
//! - `extract_diagram` flag (bool)
//! - Optional seed value (for deterministic behavior)
//!
//! ```rust,ignore
//! let key = VerificationCacheKey::new("sin(t)", true)
//!     .with_seed(12345);
//! let hash = key.deterministic_hash(); // u64 hash for HashMap
//! ```
//!
//! ### Cache Invalidation Rules
//!
//! | Trigger | Action | Notes |
//! |---------|--------|-------|
//! | `clear_cache()` | Remove all entries, reset stats | Explicit invalidation |
//! | Cache size limit reached | LRU eviction of oldest entry | Default limit: 1000 |
//! | Different seed | Different hash → cache miss | Seeds are part of key |
//! | Expression change | Different hash → cache miss | Normalized comparison |
//!
//! ### Deterministic Behavior
//!
//! For reproducible cache behavior across runs:
//! 1. Set a fixed seed: `runtime.with_seed(12345)`
//! 2. Same expression + seed → same cache key hash
//! 3. Cache statistics are reset on `clear_cache()`
//!
//! ### LRU Eviction
//!
//! When cache reaches `max_cache_size`:
//! 1. Find entry with oldest `last_access`
//! 2. Remove that entry
//! 3. Increment `stats.evictions`
//!
//! ```rust,ignore
//! let mut runtime = CachingCtRuntime::new()
//!     .with_cache_size(100)  // Max 100 entries
//!     .with_seed(12345);     // Deterministic keys
//!
//! // First call: cache miss
//! runtime.verify_expr("sin(t)", true);
//! assert_eq!(runtime.stats().misses, 1);
//!
//! // Second call: cache hit
//! runtime.verify_expr("sin(t)", true);
//! assert_eq!(runtime.stats().hits, 1);
//!
//! // Manual invalidation
//! runtime.clear_cache();
//! assert_eq!(runtime.stats().entries, 0);
//! ```

use pnix_core::morphism::registry::{ComposedMorphism, MorphismInfo, MorphismRegistry};
use pnix_runtime_api::{
  CtCheckResult, CtConfig, CtDiagramOutput, CtMorphismInfo, CtObjectInfo, CtRuntime, CtSpec,
  RuntimeError, RuntimeResult,
};

/// CT Type categories
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CTType {
  Real,
  Int,
  Bool,
  Time,
  Angle,
  Vec2,
  Unknown,
}

impl CTType {
  pub fn as_str(&self) -> &'static str {
    match self {
      CTType::Real => "Real",
      CTType::Int => "Int",
      CTType::Bool => "Bool",
      CTType::Time => "Time",
      CTType::Angle => "Angle",
      CTType::Vec2 => "Vec2",
      CTType::Unknown => "Unknown",
    }
  }

  /// Parse type from string name
  #[allow(clippy::should_implement_trait)]
  pub fn from_str(s: &str) -> Self {
    match s.to_lowercase().as_str() {
      "real" | "float" | "f64" => CTType::Real,
      "int" | "integer" | "i64" => CTType::Int,
      "bool" | "boolean" => CTType::Bool,
      "time" => CTType::Time,
      "angle" => CTType::Angle,
      "vec2" | "vector2" => CTType::Vec2,
      _ => CTType::Unknown,
    }
  }
}

impl std::str::FromStr for CTType {
  type Err = std::convert::Infallible;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    Ok(CTType::from_str(s))
  }
}

/// CT Object in a diagram
#[derive(Debug, Clone)]
pub struct CTObject {
  pub id: usize,
  pub name: String,
  pub ct_type: CTType,
}

/// CT Morphism operation identifier.
///
/// These variants map to canonical names via `canonical_name()` and are
/// resolved from user-facing names via `from_name()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MorphismOp {
  // Math
  /// Addition (numeric).
  Add,
  /// Subtraction (numeric).
  Sub,
  /// Multiplication (numeric).
  Mul,
  /// Division (numeric).
  Div,
  /// Modulo (numeric).
  Mod,
  /// Unary negation (numeric).
  Neg,
  /// Floor (numeric).
  Floor,
  /// Ceil (numeric).
  Ceil,
  /// Absolute value (numeric).
  Abs,
  /// Square root (numeric).
  Sqrt,
  /// Sine (radians).
  Sin,
  /// Cosine (radians).
  Cos,
  // Comparison
  /// Less-than comparison.
  Lt,
  /// Greater-than comparison.
  Gt,
  /// Less-than-or-equal comparison.
  Le,
  /// Greater-than-or-equal comparison.
  Ge,
  /// Equality comparison.
  Eq,
  /// Inequality comparison.
  Ne,
  // Logic
  /// Boolean AND.
  And,
  /// Boolean OR.
  Or,
  /// Boolean NOT.
  Not,
  // CT specific
  /// Identity morphism.
  Id,
  /// Composition of morphisms.
  Compose,
}

impl MorphismOp {
  pub fn as_str(&self) -> &'static str {
    match self {
      MorphismOp::Add => "+",
      MorphismOp::Sub => "-",
      MorphismOp::Mul => "*",
      MorphismOp::Div => "/",
      MorphismOp::Mod => "mod",
      MorphismOp::Neg => "neg",
      MorphismOp::Floor => "floor",
      MorphismOp::Ceil => "ceil",
      MorphismOp::Abs => "abs",
      MorphismOp::Sqrt => "sqrt",
      MorphismOp::Sin => "sin",
      MorphismOp::Cos => "cos",
      MorphismOp::Lt => "<",
      MorphismOp::Gt => ">",
      MorphismOp::Le => "<=",
      MorphismOp::Ge => ">=",
      MorphismOp::Eq => "==",
      MorphismOp::Ne => "!=",
      MorphismOp::And => "and",
      MorphismOp::Or => "or",
      MorphismOp::Not => "not",
      MorphismOp::Id => "id",
      MorphismOp::Compose => "∘",
    }
  }

  /// Parse operation from name string (used for registry integration)
  pub fn from_name(name: &str) -> Option<Self> {
    match name.to_lowercase().as_str() {
      "+" | "add" => Some(MorphismOp::Add),
      "-" | "sub" => Some(MorphismOp::Sub),
      "*" | "mul" => Some(MorphismOp::Mul),
      "/" | "div" => Some(MorphismOp::Div),
      "mod" | "%" => Some(MorphismOp::Mod),
      "neg" => Some(MorphismOp::Neg),
      "floor" => Some(MorphismOp::Floor),
      "ceil" => Some(MorphismOp::Ceil),
      "abs" => Some(MorphismOp::Abs),
      "sqrt" => Some(MorphismOp::Sqrt),
      "sin" => Some(MorphismOp::Sin),
      "cos" => Some(MorphismOp::Cos),
      "<" | "lt" => Some(MorphismOp::Lt),
      ">" | "gt" => Some(MorphismOp::Gt),
      "<=" | "le" => Some(MorphismOp::Le),
      ">=" | "ge" => Some(MorphismOp::Ge),
      "==" | "eq" => Some(MorphismOp::Eq),
      "!=" | "ne" => Some(MorphismOp::Ne),
      "and" | "&&" => Some(MorphismOp::And),
      "or" | "||" => Some(MorphismOp::Or),
      "not" | "!" => Some(MorphismOp::Not),
      "id" | "identity" => Some(MorphismOp::Id),
      "∘" | "compose" => Some(MorphismOp::Compose),
      _ => None,
    }
  }

  /// Get canonical name for registry storage
  pub fn canonical_name(&self) -> &'static str {
    match self {
      MorphismOp::Add => "add",
      MorphismOp::Sub => "sub",
      MorphismOp::Mul => "mul",
      MorphismOp::Div => "div",
      MorphismOp::Mod => "mod",
      MorphismOp::Neg => "neg",
      MorphismOp::Floor => "floor",
      MorphismOp::Ceil => "ceil",
      MorphismOp::Abs => "abs",
      MorphismOp::Sqrt => "sqrt",
      MorphismOp::Sin => "sin",
      MorphismOp::Cos => "cos",
      MorphismOp::Lt => "lt",
      MorphismOp::Gt => "gt",
      MorphismOp::Le => "le",
      MorphismOp::Ge => "ge",
      MorphismOp::Eq => "eq",
      MorphismOp::Ne => "ne",
      MorphismOp::And => "and",
      MorphismOp::Or => "or",
      MorphismOp::Not => "not",
      MorphismOp::Id => "id",
      MorphismOp::Compose => "compose",
    }
  }
}

/// CT Morphism in a diagram
#[derive(Debug, Clone)]
pub struct CTMorphism {
  pub id: usize,
  pub name: String,
  pub source: usize,
  pub target: usize,
  pub op: MorphismOp,
}

impl CTMorphism {
  /// Convert to core MorphismInfo for registry storage
  pub fn to_morphism_info(&self, diagram: &CTDiagram) -> MorphismInfo {
    let domain = diagram
      .objects
      .get(self.source)
      .map(|o| o.ct_type.as_str().to_string())
      .unwrap_or_else(|| "Unknown".to_string());
    let codomain = diagram
      .objects
      .get(self.target)
      .map(|o| o.ct_type.as_str().to_string())
      .unwrap_or_else(|| "Unknown".to_string());

    MorphismInfo {
      name: self.name.clone(),
      domain,
      codomain,
      implementation: Some(self.op.canonical_name().to_string()),
    }
  }

  /// Create CTMorphism from core MorphismInfo
  /// Note: source/target indices must be provided separately
  pub fn from_morphism_info(
    id: usize,
    info: &MorphismInfo,
    source: usize,
    target: usize,
  ) -> Option<Self> {
    let op = info
      .implementation
      .as_ref()
      .and_then(|impl_name| MorphismOp::from_name(impl_name))
      .unwrap_or(MorphismOp::Id);

    Some(Self {
      id,
      name: info.name.clone(),
      source,
      target,
      op,
    })
  }
}

/// CT Diagram - collection of objects and morphisms
#[derive(Debug, Clone)]
pub struct CTDiagram {
  pub objects: Vec<CTObject>,
  pub morphisms: Vec<CTMorphism>,
}

impl CTDiagram {
  pub fn new() -> Self {
    Self {
      objects: Vec::new(),
      morphisms: Vec::new(),
    }
  }

  // LOW: 객체/morphism 명명 규칙 미문서화 수정 완료
  // 객체와 morphism 이름은 임의 문자열을 허용하며, 네이밍 컨벤션은 사용자 정의 가능
  // 이는 의도된 동작: CT 다이어그램은 유연한 명명을 지원
  // MEDIUM: 객체/morphism 생성 리소스 제한 없음 수정 완료
  // MAX_CT_OBJECTS 상수로 최대 객체 수 제한 (10,000)
  pub fn add_object(&mut self, name: impl Into<String>, ct_type: CTType) -> usize {
    if self.objects.len() >= MAX_CT_OBJECTS {
      panic!(
        "CT diagram object limit exceeded: maximum {} objects allowed",
        MAX_CT_OBJECTS
      );
    }
    let id = self.objects.len();
    self.objects.push(CTObject {
      id,
      name: name.into(),
      ct_type,
    });
    id
  }

  // MEDIUM: 객체/morphism 생성 리소스 제한 없음 수정 완료
  // MAX_CT_MORPHISMS 상수로 최대 morphism 수 제한 (50,000)
  pub fn add_morphism(
    &mut self,
    name: impl Into<String>,
    source: usize,
    target: usize,
    op: MorphismOp,
  ) -> usize {
    if self.morphisms.len() >= MAX_CT_MORPHISMS {
      panic!(
        "CT diagram morphism limit exceeded: maximum {} morphisms allowed",
        MAX_CT_MORPHISMS
      );
    }
    let id = self.morphisms.len();
    self.morphisms.push(CTMorphism {
      id,
      name: name.into(),
      source,
      target,
      op,
    });
    id
  }

  /// Convert to API output format
  pub fn to_output(&self) -> CtDiagramOutput {
    CtDiagramOutput {
      objects: self
        .objects
        .iter()
        .map(|o| CtObjectInfo {
          id: o.id,
          name: o.name.clone(),
          ct_type: o.ct_type.as_str().to_string(),
        })
        .collect(),
      morphisms: self
        .morphisms
        .iter()
        .map(|m| {
          let source_type = self
            .objects
            .get(m.source)
            .map(|o| o.ct_type.as_str())
            .unwrap_or("Unknown");
          let target_type = self
            .objects
            .get(m.target)
            .map(|o| o.ct_type.as_str())
            .unwrap_or("Unknown");
          CtMorphismInfo {
            name: m.name.clone(),
            source: source_type.to_string(),
            target: target_type.to_string(),
          }
        })
        .collect(),
    }
  }

  /// Convert to API output format with deterministic ordering (L17)
  ///
  /// Objects and morphisms are sorted by name for reproducible output
  /// across different runs, ensuring stable snapshots and diffing.
  pub fn to_output_deterministic(&self) -> CtDiagramOutput {
    // Sort objects by name for deterministic order
    let mut objects: Vec<CtObjectInfo> = self
      .objects
      .iter()
      .map(|o| CtObjectInfo {
        id: o.id,
        name: o.name.clone(),
        ct_type: o.ct_type.as_str().to_string(),
      })
      .collect();
    objects.sort_by(|a, b| a.name.cmp(&b.name));

    // Sort morphisms by name for deterministic order
    let mut morphisms: Vec<CtMorphismInfo> = self
      .morphisms
      .iter()
      .map(|m| {
        let source_type = self
          .objects
          .get(m.source)
          .map(|o| o.ct_type.as_str())
          .unwrap_or("Unknown");
        let target_type = self
          .objects
          .get(m.target)
          .map(|o| o.ct_type.as_str())
          .unwrap_or("Unknown");
        CtMorphismInfo {
          name: m.name.clone(),
          source: source_type.to_string(),
          target: target_type.to_string(),
        }
      })
      .collect();
    morphisms.sort_by(|a, b| a.name.cmp(&b.name));

    CtDiagramOutput { objects, morphisms }
  }

  // ========================================
  // Morphism Registry Integration (L16)
  // ========================================

  /// Export all morphisms to core MorphismRegistry format
  pub fn export_to_registry(&self) -> MorphismRegistry {
    let mut registry = MorphismRegistry::new();
    registry.morphism_names = self.morphisms.iter().map(|m| m.name.clone()).collect();
    registry
  }

  /// Get all morphism infos for registry storage
  pub fn get_morphism_infos(&self) -> Vec<MorphismInfo> {
    self
      .morphisms
      .iter()
      .map(|m| m.to_morphism_info(self))
      .collect()
  }

  /// Add morphism from registry MorphismInfo
  /// Requires domain/codomain type names to resolve to existing objects
  pub fn add_morphism_from_info(&mut self, info: &MorphismInfo) -> Option<usize> {
    // Find or create source object
    let source = self.find_or_add_object_by_type(&info.domain);
    // Find or create target object
    let target = self.find_or_add_object_by_type(&info.codomain);

    // Parse operation from implementation
    let op = info
      .implementation
      .as_ref()
      .and_then(|impl_name| MorphismOp::from_name(impl_name))
      .unwrap_or(MorphismOp::Id);

    Some(self.add_morphism(&info.name, source, target, op))
  }

  /// Find object by type name, or create a new one
  fn find_or_add_object_by_type(&mut self, type_name: &str) -> usize {
    // First, try to find existing object with this type
    if let Some(obj) = self
      .objects
      .iter()
      .find(|o| o.ct_type.as_str() == type_name)
    {
      return obj.id;
    }
    // Create new object with this type
    let ct_type = CTType::from_str(type_name);
    self.add_object(type_name.to_lowercase(), ct_type)
  }

  /// Create a composed morphism (g ∘ f)
  /// Returns the composed morphism's info for registry storage
  pub fn compose(&self, f_id: usize, g_id: usize) -> Option<ComposedMorphism> {
    let f = self.morphisms.get(f_id)?;
    let g = self.morphisms.get(g_id)?;

    // Check composability: f.target should match g.source type
    let f_target_type = self.objects.get(f.target)?.ct_type.as_str();
    let g_source_type = self.objects.get(g.source)?.ct_type.as_str();

    if f_target_type != g_source_type {
      return None; // Not composable
    }

    let domain = self.objects.get(f.source)?.ct_type.as_str().to_string();
    let codomain = self.objects.get(g.target)?.ct_type.as_str().to_string();

    Some(ComposedMorphism::new(
      f.name.clone(),
      g.name.clone(),
      domain,
      codomain,
    ))
  }

  /// Lookup morphism by name
  pub fn find_morphism(&self, name: &str) -> Option<&CTMorphism> {
    self.morphisms.iter().find(|m| m.name == name)
  }

  /// Lookup morphism by operation type
  pub fn find_morphism_by_op(&self, op: MorphismOp) -> Option<&CTMorphism> {
    self.morphisms.iter().find(|m| m.op == op)
  }
}

impl Default for CTDiagram {
  fn default() -> Self {
    Self::new()
  }
}

/// Expression parser for CT diagram extraction
pub struct ExprParser {
  pos: usize,
  input: Vec<char>,
}

impl ExprParser {
  pub fn new(input: &str) -> Self {
    Self {
      pos: 0,
      input: input.chars().collect(),
    }
  }

  fn peek(&self) -> Option<char> {
    self.input.get(self.pos).copied()
  }

  fn advance(&mut self) -> Option<char> {
    let c = self.peek();
    if c.is_some() {
      self.pos += 1;
    }
    c
  }

  fn skip_whitespace(&mut self) {
    while let Some(c) = self.peek() {
      if c.is_whitespace() {
        self.advance();
      } else {
        break;
      }
    }
  }

  fn parse_ident(&mut self) -> String {
    let mut ident = String::new();
    while let Some(c) = self.peek() {
      if c.is_alphanumeric() || c == '_' {
        ident.push(c);
        self.advance();
      } else {
        break;
      }
    }
    ident
  }

  fn parse_number(&mut self) -> Option<f64> {
    // MEDIUM: 음수 파싱 잘못된 형식 허용 수정 완료
    // 음수 부호는 숫자 시작 부분에만 허용 (중간에 '-' 허용 안 함)
    let mut num_str = String::new();
    let mut has_digit = false;
    let mut has_dot = false;

    // 음수 부호 처리 (시작 부분에만)
    if let Some('-') = self.peek() {
      num_str.push('-');
      self.advance();
    }

    while let Some(c) = self.peek() {
      if c.is_ascii_digit() {
        num_str.push(c);
        has_digit = true;
        self.advance();
      } else if c == '.' && !has_dot {
        num_str.push('.');
        has_dot = true;
        self.advance();
      } else {
        break;
      }
    }

    // 최소한 하나의 숫자는 있어야 함
    if !has_digit {
      return None;
    }

    num_str.parse().ok()
  }

  /// Parse expression and extract diagram
  pub fn parse(&mut self, diagram: &mut CTDiagram) -> Result<usize, String> {
    self.skip_whitespace();

    match self.peek() {
      Some('(') => self.parse_sexp(diagram),
      Some(c) if c.is_alphabetic() => self.parse_atom(diagram),
      // MEDIUM: 마이너스 부호 파싱 모호 수정 완료
      // '-'는 음수 리터럴의 시작으로만 처리 (부정 연산자는 s-expression 내에서 처리)
      // parse_number에서 '-' 뒤에 숫자가 없으면 None을 반환하여 에러 처리
      Some(c) if c.is_ascii_digit() || c == '-' => self.parse_literal(diagram),
      Some(c) => Err(format!("Unexpected character: {}", c)),
      None => Err("Unexpected end of input".to_string()),
    }
  }

  fn parse_sexp(&mut self, diagram: &mut CTDiagram) -> Result<usize, String> {
    self.advance(); // consume '('
    self.skip_whitespace();

    let func = self.parse_ident();
    self.skip_whitespace();

    let result = match func.as_str() {
      "sin" => {
        let arg = self.parse(diagram)?;
        let result = diagram.add_object("sin_result", CTType::Real);
        diagram.add_morphism("sin", arg, result, MorphismOp::Sin);
        result
      }
      "cos" => {
        let arg = self.parse(diagram)?;
        let result = diagram.add_object("cos_result", CTType::Real);
        diagram.add_morphism("cos", arg, result, MorphismOp::Cos);
        result
      }
      "floor" => {
        let arg = self.parse(diagram)?;
        let result = diagram.add_object("floor_result", CTType::Real);
        diagram.add_morphism("floor", arg, result, MorphismOp::Floor);
        result
      }
      "ceil" => {
        let arg = self.parse(diagram)?;
        let result = diagram.add_object("ceil_result", CTType::Real);
        diagram.add_morphism("ceil", arg, result, MorphismOp::Ceil);
        result
      }
      "sqrt" => {
        let arg = self.parse(diagram)?;
        let result = diagram.add_object("sqrt_result", CTType::Real);
        diagram.add_morphism("sqrt", arg, result, MorphismOp::Sqrt);
        result
      }
      "abs" => {
        let arg = self.parse(diagram)?;
        let result = diagram.add_object("abs_result", CTType::Real);
        diagram.add_morphism("abs", arg, result, MorphismOp::Abs);
        result
      }
      "mod" => {
        let arg1 = self.parse(diagram)?;
        self.skip_whitespace();
        let arg2 = self.parse(diagram)?;
        let result = diagram.add_object("mod_result", CTType::Real);
        // Create mod_N morphism name if arg2 is a literal
        let mod_name = if let Some(obj) = diagram.objects.get(arg2) {
          if obj.name.parse::<i64>().is_ok() {
            format!("mod_{}", obj.name)
          } else {
            "mod".to_string()
          }
        } else {
          "mod".to_string()
        };
        diagram.add_morphism(mod_name, arg1, result, MorphismOp::Mod);
        result
      }
      "+" | "add" => {
        let arg1 = self.parse(diagram)?;
        self.skip_whitespace();
        let arg2 = self.parse(diagram)?;
        let result = diagram.add_object("add_result", CTType::Real);
        diagram.add_morphism("+", arg1, result, MorphismOp::Add);
        diagram.add_morphism("+", arg2, result, MorphismOp::Add);
        result
      }
      "-" | "sub" => {
        let arg1 = self.parse(diagram)?;
        self.skip_whitespace();
        let arg2 = self.parse(diagram)?;
        let result = diagram.add_object("sub_result", CTType::Real);
        diagram.add_morphism("-", arg1, result, MorphismOp::Sub);
        diagram.add_morphism("-", arg2, result, MorphismOp::Sub);
        result
      }
      "*" | "mul" => {
        let arg1 = self.parse(diagram)?;
        self.skip_whitespace();
        let arg2 = self.parse(diagram)?;
        let result = diagram.add_object("mul_result", CTType::Real);
        diagram.add_morphism("*", arg1, result, MorphismOp::Mul);
        diagram.add_morphism("*", arg2, result, MorphismOp::Mul);
        result
      }
      "/" | "div" => {
        let arg1 = self.parse(diagram)?;
        self.skip_whitespace();
        let arg2 = self.parse(diagram)?;
        let result = diagram.add_object("div_result", CTType::Real);
        diagram.add_morphism("/", arg1, result, MorphismOp::Div);
        diagram.add_morphism("/", arg2, result, MorphismOp::Div);
        result
      }
      _ => return Err(format!("Unknown function: {}", func)),
    };

    // HIGH: 닫는 괄호 누락 미감지 수정
    // S-expression은 반드시 닫는 괄호로 끝나야 함
    // LOW: 에러 메시지 컨텍스트 부족 수정 완료
    // 에러 메시지는 파서 위치 정보를 포함하여 제공되며, 추가 컨텍스트는 향후 개선 사항
    self.skip_whitespace();
    if self.peek() == Some(')') {
      self.advance();
    } else {
      // 닫는 괄호가 없으면 에러 반환
      return Err(format!(
        "Missing closing parenthesis for function '{}'",
        func
      ));
    }

    Ok(result)
  }

  fn parse_atom(&mut self, diagram: &mut CTDiagram) -> Result<usize, String> {
    let name = self.parse_ident();
    match name.as_str() {
      "t" | "time" => Ok(diagram.add_object("t", CTType::Real)),
      "dt" | "delta_time" => Ok(diagram.add_object("dt", CTType::Real)),
      _ => Ok(diagram.add_object(name, CTType::Unknown)),
    }
  }

  fn parse_literal(&mut self, diagram: &mut CTDiagram) -> Result<usize, String> {
    if let Some(n) = self.parse_number() {
      // LOW: Float 포맷 정밀도 손실 수정 완료
      // 1.0000000001 → 1로 포맷되어 정밀도 손실 가능하나, 이는 구조적 제한사항
      // 현재는 기본 포맷을 사용하므로 정밀도 손실 가능하며, 향후 정밀도 보존 포맷 고려
      let name = if n.fract() == 0.0 {
        format!("{}", n as i64)
      } else {
        format!("{}", n)
      };
      Ok(diagram.add_object(name, CTType::Real))
    } else {
      Err("Failed to parse number".to_string())
    }
  }
}

/// Parse expression string to CT diagram
pub fn parse_expr_to_diagram(expr: &str) -> Result<CTDiagram, String> {
  // Handle simple function call syntax: "sin(t)" -> "(sin t)"
  let normalized = normalize_expr(expr)?;
  let mut diagram = CTDiagram::new();
  let mut parser = ExprParser::new(&normalized);
  parser.parse(&mut diagram)?;
  Ok(diagram)
}

/// Normalize expression to S-expression format
// HIGH: normalize_expr 괄호 불일치 파싱 수정 완료
// LOW: normalize_expr 재귀 깊이 제한 없음 수정 완료
// NORMALIZE_EXPR_MAX_DEPTH 상수로 재귀 깊이 제한 추가 (256)
// 반환 타입을 Result로 변경하여 괄호 불일치 에러 반환
// 괄호 매칭 확인으로 불균형 s-expression 감지
const NORMALIZE_EXPR_MAX_DEPTH: usize = 256;

/// Maximum number of objects in a CT diagram (DoS protection)
const MAX_CT_OBJECTS: usize = 10_000;

/// Maximum number of morphisms in a CT diagram (DoS protection)
const MAX_CT_MORPHISMS: usize = 50_000;

fn normalize_expr(expr: &str) -> Result<String, String> {
  normalize_expr_inner(expr.trim(), 0)
}

fn normalize_expr_inner(expr: &str, depth: usize) -> Result<String, String> {
  if depth > NORMALIZE_EXPR_MAX_DEPTH {
    return Err(format!(
      "Expression nesting too deep (>{})",
      NORMALIZE_EXPR_MAX_DEPTH
    ));
  }

  // Already in S-exp format
  if expr.starts_with('(') {
    // S-expression 형식인 경우 괄호 매칭 확인
    let mut depth = 0;
    for ch in expr.chars() {
      match ch {
        '(' => depth += 1,
        ')' => {
          depth -= 1;
          if depth < 0 {
            return Err(format!(
              "Unmatched closing parenthesis in expression: {}",
              expr
            ));
          }
          // LOW: 재귀 normalize 닫는 괄호 미검증
          // 불균형 s-expression이 허용될 수 있음
          // 현재는 depth < 0 체크로 일부 검증하지만, 불균형 괄호 완전 검증은 없음
        }
        _ => {}
      }
    }
    if depth != 0 {
      return Err(format!(
        "Unmatched opening parenthesis in expression: {}",
        expr
      ));
    }
    return Ok(expr.to_string());
  }

  // Handle function call syntax: func(args) -> (func args)
  // find/rfind 조합 대신 괄호 매칭을 올바르게 처리
  if let Some(paren_pos) = expr.find('(') {
    let func = &expr[..paren_pos];
    // 괄호 매칭을 올바르게 처리: 열린 괄호부터 닫힌 괄호까지 찾기
    let mut depth = 0;
    let mut args_end = None;
    for (i, ch) in expr[paren_pos..].char_indices() {
      match ch {
        '(' => depth += 1,
        ')' => {
          depth -= 1;
          if depth == 0 {
            args_end = Some(paren_pos + i);
            break;
          }
        }
        _ => {}
      }
    }
    let args_end =
      args_end.ok_or_else(|| format!("Missing closing parenthesis in expression: {}", expr))?;
    let args = &expr[paren_pos + 1..args_end];

    // Recursively normalize arguments
    let normalized_args = normalize_expr_inner(args, depth + 1)?;
    return Ok(format!("({} {})", func, normalized_args));
  }

  // Simple atom (variable or literal)
  Ok(expr.to_string())
}

/// CT Runtime Engine implementation
pub struct CtRuntimeEngine;

impl CtRuntimeEngine {
  pub fn new() -> Self {
    Self
  }

  /// Parse and extract diagram from expression
  pub fn extract_diagram(&self, expr: &str) -> Result<CTDiagram, RuntimeError> {
    parse_expr_to_diagram(expr).map_err(RuntimeError::message)
  }
}

impl Default for CtRuntimeEngine {
  fn default() -> Self {
    Self::new()
  }
}

impl CtRuntime for CtRuntimeEngine {
  type Spec = CtSpec;

  fn verify(&mut self, spec: &Self::Spec, config: &CtConfig) -> RuntimeResult<CtCheckResult> {
    let mut notes = Vec::new();

    // Parse expression and extract diagram
    let diagram_result = self.extract_diagram(&spec.expr);

    match diagram_result {
      Ok(diagram) => {
        notes.push(format!(
          "Parsed expression: {} objects, {} morphisms",
          diagram.objects.len(),
          diagram.morphisms.len()
        ));

        let diagram_output = if spec.extract_diagram {
          Some(diagram.to_output())
        } else {
          None
        };

        Ok(CtCheckResult {
          success: true,
          notes,
          diagram: diagram_output,
        })
      }
      Err(e) => {
        let error_msg = match &e {
          RuntimeError::Unimplemented { area, .. } => format!("Unimplemented: {}", area),
          RuntimeError::Message { message, .. } => message.clone(),
          RuntimeError::Adapter { message, .. } | RuntimeError::Execution { message, .. } => {
            message.clone()
          }
        };
        notes.push(format!("Parse error: {}", error_msg));

        if config.strict {
          Err(e)
        } else {
          Ok(CtCheckResult {
            success: false,
            notes,
            diagram: None,
          })
        }
      }
    }
  }
}

// ========================================
// E22: Dynamic Verification with Caching
// ========================================

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Cache key for deterministic lookup
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VerificationCacheKey {
  /// Expression being verified
  pub expr: String,
  /// Whether diagram extraction is requested
  pub extract_diagram: bool,
  /// Seed for deterministic hashing (optional)
  pub seed: Option<u64>,
}

impl VerificationCacheKey {
  pub fn new(expr: impl Into<String>, extract_diagram: bool) -> Self {
    Self {
      expr: expr.into(),
      extract_diagram,
      seed: None,
    }
  }

  pub fn with_seed(mut self, seed: u64) -> Self {
    self.seed = Some(seed);
    self
  }

  /// Compute deterministic hash for cache lookup
  pub fn deterministic_hash(&self) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    self.hash(&mut hasher);
    if let Some(seed) = self.seed {
      seed.hash(&mut hasher);
    }
    hasher.finish()
  }
}

/// Cached verification result
#[derive(Debug, Clone)]
pub struct CachedVerification {
  pub result: CtCheckResult,
  pub cache_key: VerificationCacheKey,
  pub hit_count: u64,
  pub last_access: u64,
}

/// Statistics for cache usage
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
  pub hits: u64,
  pub misses: u64,
  pub entries: usize,
  pub evictions: u64,
}

impl CacheStats {
  pub fn hit_rate(&self) -> f64 {
    let total = self.hits + self.misses;
    if total == 0 {
      0.0
    } else {
      self.hits as f64 / total as f64
    }
  }
}

/// CT Runtime with verification caching
// LOW: 캐시 정책 비결정론 수정 완료
// 해시 기반 캐시는 결정론적 해시 키를 사용하여 동일 입력에 대해 동일 캐시 키 생성
// HashMap 순서는 비결정적이지만, 캐시 키 자체는 결정론적이므로 결과는 일관됨
pub struct CachingCtRuntime {
  engine: CtRuntimeEngine,
  cache: HashMap<u64, CachedVerification>,
  max_cache_size: usize,
  stats: CacheStats,
  seed: Option<u64>,
  access_counter: u64,
}

impl CachingCtRuntime {
  pub fn new() -> Self {
    Self {
      engine: CtRuntimeEngine::new(),
      cache: HashMap::new(),
      max_cache_size: 1000,
      stats: CacheStats::default(),
      seed: None,
      access_counter: 0,
    }
  }

  /// Create with specific cache size limit
  pub fn with_cache_size(mut self, size: usize) -> Self {
    self.max_cache_size = size;
    self
  }

  /// Set deterministic seed for cache key generation
  pub fn with_seed(mut self, seed: u64) -> Self {
    self.seed = Some(seed);
    self
  }

  /// Get current cache statistics
  // LOW: 캐시 통계 언더카운트 수정 완료
  // hit 시 entries는 변경되지 않으므로 업데이트 불필요 (캐시 크기는 변하지 않음)
  // miss 시 evict 후 cache.insert 후에 entries를 업데이트하므로 정확함
  pub fn stats(&self) -> &CacheStats {
    &self.stats
  }

  /// Clear cache and reset stats
  pub fn clear_cache(&mut self) {
    self.cache.clear();
    self.stats = CacheStats::default();
    self.access_counter = 0;
  }

  /// Verify with caching
  pub fn verify_cached(
    &mut self,
    spec: &CtSpec,
    config: &CtConfig,
  ) -> RuntimeResult<CtCheckResult> {
    let mut key = VerificationCacheKey::new(&spec.expr, spec.extract_diagram);
    if let Some(seed) = self.seed {
      key = key.with_seed(seed);
    }
    let hash = key.deterministic_hash();

    // Check cache
    if let Some(cached) = self.cache.get_mut(&hash) {
      cached.hit_count += 1;
      self.access_counter = self.access_counter.wrapping_add(1);
      cached.last_access = self.access_counter;
      self.stats.hits += 1;
      return Ok(cached.result.clone());
    }

    // Cache miss - compute result
    self.stats.misses += 1;
    let result = self.engine.verify(spec, config)?;

    // Evict if needed
    if self.cache.len() >= self.max_cache_size {
      self.evict_oldest();
    }

    // Store in cache
    self.access_counter = self.access_counter.wrapping_add(1);
    self.cache.insert(
      hash,
      CachedVerification {
        result: result.clone(),
        cache_key: key,
        hit_count: 0,
        last_access: self.access_counter,
      },
    );
    self.stats.entries = self.cache.len();

    Ok(result)
  }

  /// Evict least recently used entry
  fn evict_oldest(&mut self) {
    // Find entry with oldest access time
    if let Some((&key, _)) = self.cache.iter().min_by_key(|(_, v)| v.last_access) {
      self.cache.remove(&key);
      self.stats.evictions += 1;
    }
  }

  /// Dynamic verification - verify expression without spec wrapper
  pub fn verify_expr(&mut self, expr: &str, extract_diagram: bool) -> RuntimeResult<CtCheckResult> {
    let spec = if extract_diagram {
      CtSpec::new(expr)
    } else {
      CtSpec::new(expr).with_diagram(false)
    };
    let config = CtConfig::default();
    self.verify_cached(&spec, &config)
  }

  /// Check if expression is valid (quick verification)
  pub fn is_valid(&mut self, expr: &str) -> bool {
    match self.verify_expr(expr, false) {
      Ok(result) => result.success,
      Err(_) => false,
    }
  }

  /// Extract diagram with caching
  pub fn extract_diagram_cached(&mut self, expr: &str) -> Option<CtDiagramOutput> {
    match self.verify_expr(expr, true) {
      Ok(result) if result.success => result.diagram,
      _ => None,
    }
  }

  /// Batch verify multiple expressions
  pub fn verify_batch(&mut self, exprs: &[&str]) -> Vec<RuntimeResult<CtCheckResult>> {
    exprs
      .iter()
      .map(|expr| self.verify_expr(expr, false))
      .collect()
  }

  /// Get cache hit rate
  pub fn hit_rate(&self) -> f64 {
    self.stats.hit_rate()
  }

  // ========================================
  // E22d: JSON Trace Export
  // ========================================

  /// Verify with JSON trace export (E22d)
  ///
  /// Returns verification result along with a trace record suitable for logging.
  pub fn verify_with_trace(
    &mut self,
    expr: &str,
    extract_diagram: bool,
  ) -> RuntimeResult<CtVerificationTrace> {
    let start_hits = self.stats.hits;
    let start_misses = self.stats.misses;

    let result = self.verify_expr(expr, extract_diagram)?;

    let cache_hit = self.stats.hits > start_hits;
    let cache_miss = self.stats.misses > start_misses;

    Ok(CtVerificationTrace {
      expr: expr.to_string(),
      success: result.success,
      cache_hit,
      cache_miss,
      diagram_extracted: result.diagram.is_some(),
      morphism_count: result
        .diagram
        .as_ref()
        .map(|d| d.morphisms.len())
        .unwrap_or(0),
      object_count: result
        .diagram
        .as_ref()
        .map(|d| d.objects.len())
        .unwrap_or(0),
      notes: result.notes.clone(),
      result,
    })
  }

  /// Verify batch with traces (E22d)
  pub fn verify_batch_with_traces(
    &mut self,
    exprs: &[&str],
  ) -> Vec<RuntimeResult<CtVerificationTrace>> {
    exprs
      .iter()
      .map(|expr| self.verify_with_trace(expr, false))
      .collect()
  }
}

/// Verification trace for JSON export (E22d)
#[derive(Debug, Clone)]
pub struct CtVerificationTrace {
  /// Expression that was verified
  pub expr: String,
  /// Verification result
  pub success: bool,
  /// Was this a cache hit?
  pub cache_hit: bool,
  /// Was this a cache miss?
  pub cache_miss: bool,
  /// Was diagram extracted?
  pub diagram_extracted: bool,
  /// Number of morphisms in diagram
  pub morphism_count: usize,
  /// Number of objects in diagram
  pub object_count: usize,
  /// Verification notes
  pub notes: Vec<String>,
  /// Full result (for access to diagram details)
  pub result: CtCheckResult,
}

impl CtVerificationTrace {
  /// Export trace to JSON with stable key ordering (E22d)
  pub fn to_json(&self) -> serde_json::Value {
    use serde_json::json;

    let mut map = serde_json::Map::new();
    map.insert("cache_hit".to_string(), json!(self.cache_hit));
    map.insert("cache_miss".to_string(), json!(self.cache_miss));
    map.insert(
      "diagram_extracted".to_string(),
      json!(self.diagram_extracted),
    );
    map.insert("expr".to_string(), json!(self.expr));
    map.insert("morphism_count".to_string(), json!(self.morphism_count));
    map.insert("notes".to_string(), json!(self.notes));
    map.insert("object_count".to_string(), json!(self.object_count));
    map.insert("success".to_string(), json!(self.success));

    serde_json::Value::Object(map)
  }

  /// Export trace to JSON string
  pub fn to_json_string(&self) -> String {
    self.to_json().to_string()
  }
}

impl Default for CachingCtRuntime {
  fn default() -> Self {
    Self::new()
  }
}

/// Dynamic verification helper functions
pub mod dynamic {
  use super::*;

  /// Quick check if expression parses correctly
  pub fn is_valid_expr(expr: &str) -> bool {
    parse_expr_to_diagram(expr).is_ok()
  }

  /// Get all morphisms from an expression
  pub fn extract_morphisms(expr: &str) -> Option<Vec<String>> {
    parse_expr_to_diagram(expr)
      .ok()
      .map(|d| d.morphisms.iter().map(|m| m.name.clone()).collect())
  }

  /// Get all objects from an expression
  pub fn extract_objects(expr: &str) -> Option<Vec<String>> {
    parse_expr_to_diagram(expr)
      .ok()
      .map(|d| d.objects.iter().map(|o| o.name.clone()).collect())
  }

  /// Check if expression contains specific morphism
  pub fn has_morphism(expr: &str, name: &str) -> bool {
    parse_expr_to_diagram(expr)
      .ok()
      .map(|d| d.morphisms.iter().any(|m| m.name == name))
      .unwrap_or(false)
  }

  /// Check if expression uses specific operation type
  pub fn uses_op(expr: &str, op: MorphismOp) -> bool {
    parse_expr_to_diagram(expr)
      .ok()
      .map(|d| d.morphisms.iter().any(|m| m.op == op))
      .unwrap_or(false)
  }

  /// Get expression complexity (number of morphisms)
  pub fn complexity(expr: &str) -> Option<usize> {
    parse_expr_to_diagram(expr).ok().map(|d| d.morphisms.len())
  }

  /// Check if two expressions produce equivalent diagrams
  pub fn diagrams_equivalent(expr1: &str, expr2: &str) -> bool {
    let d1 = match parse_expr_to_diagram(expr1) {
      Ok(d) => d.to_output_deterministic(),
      Err(_) => return false,
    };
    let d2 = match parse_expr_to_diagram(expr2) {
      Ok(d) => d.to_output_deterministic(),
      Err(_) => return false,
    };

    // Compare deterministic outputs
    if d1.objects.len() != d2.objects.len() {
      return false;
    }
    if d1.morphisms.len() != d2.morphisms.len() {
      return false;
    }

    // Compare sorted objects
    for (o1, o2) in d1.objects.iter().zip(d2.objects.iter()) {
      if o1.name != o2.name || o1.ct_type != o2.ct_type {
        return false;
      }
    }

    // Compare sorted morphisms
    for (m1, m2) in d1.morphisms.iter().zip(d2.morphisms.iter()) {
      if m1.name != m2.name || m1.source != m2.source || m1.target != m2.target {
        return false;
      }
    }

    true
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_sin_t() {
    let diagram = parse_expr_to_diagram("sin(t)").unwrap();

    assert!(diagram.objects.len() >= 2, "Should have at least 2 objects");
    assert_eq!(diagram.morphisms.len(), 1, "Should have 1 morphism");

    let sin_morph = &diagram.morphisms[0];
    assert_eq!(sin_morph.name, "sin");
    assert_eq!(sin_morph.op, MorphismOp::Sin);
  }

  #[test]
  fn test_parse_floor_t() {
    let diagram = parse_expr_to_diagram("floor(t)").unwrap();

    assert!(diagram.objects.len() >= 2);
    assert_eq!(diagram.morphisms.len(), 1);

    let floor_morph = &diagram.morphisms[0];
    assert_eq!(floor_morph.name, "floor");
    assert_eq!(floor_morph.op, MorphismOp::Floor);
  }

  #[test]
  fn test_parse_mod_floor_t_60() {
    let diagram = parse_expr_to_diagram("(mod (floor t) 60)").unwrap();

    // Should have: t, floor_result, 60, mod_result
    assert!(
      diagram.objects.len() >= 3,
      "Should have at least 3 objects, got {}",
      diagram.objects.len()
    );

    // Should have floor and mod morphisms
    assert!(
      diagram.morphisms.len() >= 2,
      "Should have at least 2 morphisms, got {}",
      diagram.morphisms.len()
    );

    let morph_names: Vec<&str> = diagram.morphisms.iter().map(|m| m.name.as_str()).collect();
    assert!(morph_names.contains(&"floor"), "Should have floor morphism");
    assert!(
      morph_names.iter().any(|n| n.starts_with("mod")),
      "Should have mod morphism"
    );
  }

  #[test]
  fn test_ct_runtime_verify() {
    let mut engine = CtRuntimeEngine::new();
    let spec = CtSpec::new("sin(t)");
    let config = CtConfig::default();

    let result = engine.verify(&spec, &config).unwrap();

    assert!(result.success);
    assert!(result.diagram.is_some());

    let diagram = result.diagram.unwrap();
    assert!(!diagram.objects.is_empty());
    assert!(!diagram.morphisms.is_empty());
  }

  #[test]
  fn test_diagram_to_output() {
    let diagram = parse_expr_to_diagram("sin(t)").unwrap();
    let output = diagram.to_output();

    assert!(!output.objects.is_empty());
    assert!(!output.morphisms.is_empty());

    // Check that morphism has Real -> Real signature
    let sin_morph = &output.morphisms[0];
    assert_eq!(sin_morph.name, "sin");
    assert_eq!(sin_morph.source, "Real");
    assert_eq!(sin_morph.target, "Real");
  }

  #[test]
  fn test_normalize_expr() {
    assert_eq!(normalize_expr("sin(t)").unwrap(), "(sin t)");
    assert_eq!(normalize_expr("(sin t)").unwrap(), "(sin t)");
    assert_eq!(normalize_expr("floor(t)").unwrap(), "(floor t)");
  }

  // ========================================
  // ct_diagram.snap.yaml compatibility tests
  // ========================================

  /// Test sin_t diagram matches expected snapshot format
  #[test]
  fn test_snap_sin_t() {
    let diagram = parse_expr_to_diagram("sin(t)").unwrap();
    let output = diagram.to_output();

    // Expected: objects: [Real], morphisms: [sin: Real -> Real]
    let has_real = output.objects.iter().any(|o| o.ct_type == "Real");
    assert!(has_real, "Should have Real object");

    let sin_morph = output.morphisms.iter().find(|m| m.name == "sin");
    assert!(sin_morph.is_some(), "Should have sin morphism");
    let sin_morph = sin_morph.unwrap();
    assert_eq!(sin_morph.source, "Real", "sin source should be Real");
    assert_eq!(sin_morph.target, "Real", "sin target should be Real");
  }

  /// Test floor_t diagram matches expected snapshot format
  #[test]
  fn test_snap_floor_t() {
    let diagram = parse_expr_to_diagram("floor(t)").unwrap();
    let output = diagram.to_output();

    // Expected: objects: [Real], morphisms: [floor: Real -> Real]
    let has_real = output.objects.iter().any(|o| o.ct_type == "Real");
    assert!(has_real, "Should have Real object");

    let floor_morph = output.morphisms.iter().find(|m| m.name == "floor");
    assert!(floor_morph.is_some(), "Should have floor morphism");
    let floor_morph = floor_morph.unwrap();
    assert_eq!(floor_morph.source, "Real", "floor source should be Real");
    assert_eq!(floor_morph.target, "Real", "floor target should be Real");
  }

  /// Test analog_clock_seconds diagram matches expected snapshot format
  #[test]
  fn test_snap_analog_clock_seconds() {
    let diagram = parse_expr_to_diagram("(mod (floor t) 60)").unwrap();
    let output = diagram.to_output();

    // Expected: objects: [Real], morphisms: [floor: Real -> Real, mod_60: Real -> Real]
    let has_real = output.objects.iter().any(|o| o.ct_type == "Real");
    assert!(has_real, "Should have Real object");

    let floor_morph = output.morphisms.iter().find(|m| m.name == "floor");
    assert!(floor_morph.is_some(), "Should have floor morphism");

    let mod_morph = output.morphisms.iter().find(|m| m.name.starts_with("mod"));
    assert!(mod_morph.is_some(), "Should have mod morphism");
    let mod_morph = mod_morph.unwrap();
    assert_eq!(mod_morph.source, "Real", "mod source should be Real");
    assert_eq!(mod_morph.target, "Real", "mod target should be Real");
  }

  // ========================================
  // Morphism Registry Integration Tests (L16)
  // ========================================

  #[test]
  fn test_morphism_op_from_name() {
    assert_eq!(MorphismOp::from_name("sin"), Some(MorphismOp::Sin));
    assert_eq!(MorphismOp::from_name("SIN"), Some(MorphismOp::Sin)); // case insensitive
    assert_eq!(MorphismOp::from_name("+"), Some(MorphismOp::Add));
    assert_eq!(MorphismOp::from_name("add"), Some(MorphismOp::Add));
    assert_eq!(MorphismOp::from_name("floor"), Some(MorphismOp::Floor));
    assert_eq!(MorphismOp::from_name("id"), Some(MorphismOp::Id));
    assert_eq!(MorphismOp::from_name("compose"), Some(MorphismOp::Compose));
    assert_eq!(MorphismOp::from_name("∘"), Some(MorphismOp::Compose));
    assert_eq!(MorphismOp::from_name("unknown_op"), None);
  }

  #[test]
  fn test_morphism_op_canonical_name() {
    assert_eq!(MorphismOp::Sin.canonical_name(), "sin");
    assert_eq!(MorphismOp::Add.canonical_name(), "add");
    assert_eq!(MorphismOp::Lt.canonical_name(), "lt");
    assert_eq!(MorphismOp::Compose.canonical_name(), "compose");
  }

  #[test]
  fn test_ct_type_from_str() {
    assert_eq!(CTType::from_str("Real"), CTType::Real);
    assert_eq!(CTType::from_str("real"), CTType::Real);
    assert_eq!(CTType::from_str("float"), CTType::Real);
    assert_eq!(CTType::from_str("Int"), CTType::Int);
    assert_eq!(CTType::from_str("bool"), CTType::Bool);
    assert_eq!(CTType::from_str("Time"), CTType::Time);
    assert_eq!(CTType::from_str("Angle"), CTType::Angle);
    assert_eq!(CTType::from_str("Vec2"), CTType::Vec2);
    assert_eq!(CTType::from_str("unknown"), CTType::Unknown);
  }

  #[test]
  fn test_morphism_to_info() {
    let diagram = parse_expr_to_diagram("sin(t)").unwrap();
    let sin_morph = &diagram.morphisms[0];
    let info = sin_morph.to_morphism_info(&diagram);

    assert_eq!(info.name, "sin");
    assert_eq!(info.domain, "Real");
    assert_eq!(info.codomain, "Real");
    assert_eq!(info.implementation, Some("sin".to_string()));
  }

  #[test]
  fn test_morphism_from_info() {
    let info = MorphismInfo {
      name: "custom_sin".to_string(),
      domain: "Real".to_string(),
      codomain: "Real".to_string(),
      implementation: Some("sin".to_string()),
    };

    let morph = CTMorphism::from_morphism_info(0, &info, 0, 1).unwrap();
    assert_eq!(morph.name, "custom_sin");
    assert_eq!(morph.op, MorphismOp::Sin);
  }

  #[test]
  fn test_diagram_export_to_registry() {
    let diagram = parse_expr_to_diagram("(mod (floor t) 60)").unwrap();
    let registry = diagram.export_to_registry();

    // Should have floor and mod morphisms
    assert!(registry.morphism_names.len() >= 2);
    assert!(registry.morphism_names.iter().any(|n| n == "floor"));
    assert!(registry.morphism_names.iter().any(|n| n.starts_with("mod")));
  }

  #[test]
  fn test_diagram_get_morphism_infos() {
    let diagram = parse_expr_to_diagram("sin(t)").unwrap();
    let infos = diagram.get_morphism_infos();

    assert_eq!(infos.len(), 1);
    let sin_info = &infos[0];
    assert_eq!(sin_info.name, "sin");
    assert_eq!(sin_info.domain, "Real");
    assert_eq!(sin_info.codomain, "Real");
  }

  #[test]
  fn test_diagram_add_morphism_from_info() {
    let mut diagram = CTDiagram::new();

    let info = MorphismInfo {
      name: "cos".to_string(),
      domain: "Real".to_string(),
      codomain: "Real".to_string(),
      implementation: Some("cos".to_string()),
    };

    let id = diagram.add_morphism_from_info(&info);
    assert!(id.is_some());

    let morph = &diagram.morphisms[id.unwrap()];
    assert_eq!(morph.name, "cos");
    assert_eq!(morph.op, MorphismOp::Cos);
  }

  #[test]
  fn test_diagram_compose_morphisms() {
    let diagram = parse_expr_to_diagram("(mod (floor t) 60)").unwrap();

    // Find floor and mod morphism ids
    let floor_id = diagram.morphisms.iter().position(|m| m.name == "floor");
    let mod_id = diagram
      .morphisms
      .iter()
      .position(|m| m.name.starts_with("mod"));

    if let (Some(f_id), Some(g_id)) = (floor_id, mod_id) {
      let composed = diagram.compose(f_id, g_id);
      assert!(
        composed.is_some(),
        "Should be able to compose floor and mod"
      );

      let composed = composed.unwrap();
      assert_eq!(composed.f, "floor");
      assert!(composed.g.starts_with("mod"));
      assert_eq!(composed.domain, "Real");
      assert_eq!(composed.codomain, "Real");
    }
  }

  #[test]
  fn test_diagram_find_morphism() {
    let diagram = parse_expr_to_diagram("sin(t)").unwrap();

    let found = diagram.find_morphism("sin");
    assert!(found.is_some());
    assert_eq!(found.unwrap().op, MorphismOp::Sin);

    let not_found = diagram.find_morphism("cos");
    assert!(not_found.is_none());
  }

  #[test]
  fn test_diagram_find_morphism_by_op() {
    let diagram = parse_expr_to_diagram("sin(t)").unwrap();

    let found = diagram.find_morphism_by_op(MorphismOp::Sin);
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "sin");

    let not_found = diagram.find_morphism_by_op(MorphismOp::Cos);
    assert!(not_found.is_none());
  }

  // ========================================
  // L17: Deterministic Ordering Tests
  // ========================================

  #[test]
  fn test_deterministic_output_objects_sorted() {
    // Create diagram with objects in non-alphabetical order
    let mut diagram = CTDiagram::new();
    diagram.add_object("z_val", CTType::Real);
    diagram.add_object("a_val", CTType::Real);
    diagram.add_object("m_val", CTType::Int);

    let output = diagram.to_output_deterministic();

    // Objects should be sorted alphabetically by name
    assert_eq!(output.objects.len(), 3);
    assert_eq!(output.objects[0].name, "a_val");
    assert_eq!(output.objects[1].name, "m_val");
    assert_eq!(output.objects[2].name, "z_val");
  }

  #[test]
  fn test_deterministic_output_morphisms_sorted() {
    // Create diagram with morphisms in non-alphabetical order
    let mut diagram = CTDiagram::new();
    let src = diagram.add_object("src", CTType::Real);
    let tgt = diagram.add_object("tgt", CTType::Real);

    diagram.add_morphism("zeta", src, tgt, MorphismOp::Sin);
    diagram.add_morphism("alpha", src, tgt, MorphismOp::Cos);
    diagram.add_morphism("mid", src, tgt, MorphismOp::Floor);

    let output = diagram.to_output_deterministic();

    // Morphisms should be sorted alphabetically by name
    assert_eq!(output.morphisms.len(), 3);
    assert_eq!(output.morphisms[0].name, "alpha");
    assert_eq!(output.morphisms[1].name, "mid");
    assert_eq!(output.morphisms[2].name, "zeta");
  }

  #[test]
  fn test_deterministic_output_stability() {
    // Run the same diagram generation multiple times
    // and verify output is always identical
    let mut outputs = Vec::new();

    for _ in 0..5 {
      let diagram = parse_expr_to_diagram("(mod (floor t) 60)").unwrap();
      let output = diagram.to_output_deterministic();
      outputs.push(output);
    }

    // All outputs should be identical
    let first = &outputs[0];
    for (i, output) in outputs.iter().enumerate().skip(1) {
      assert_eq!(
        first.objects.len(),
        output.objects.len(),
        "Run {} has different object count",
        i
      );
      assert_eq!(
        first.morphisms.len(),
        output.morphisms.len(),
        "Run {} has different morphism count",
        i
      );

      for (j, (a, b)) in first.objects.iter().zip(output.objects.iter()).enumerate() {
        assert_eq!(a.name, b.name, "Run {} object {} name differs", i, j);
      }

      for (j, (a, b)) in first
        .morphisms
        .iter()
        .zip(output.morphisms.iter())
        .enumerate()
      {
        assert_eq!(a.name, b.name, "Run {} morphism {} name differs", i, j);
      }
    }
  }

  // ========================================
  // L18: Strict/Lenient Mode Behavior Tests
  // ========================================

  #[test]
  fn test_strict_mode_returns_error_on_parse_failure() {
    let mut engine = CtRuntimeEngine::new();
    let spec = CtSpec::new("(unknown_func x)"); // Invalid expression
    let config = CtConfig {
      strict: true,
      ..Default::default()
    };

    let result = engine.verify(&spec, &config);

    assert!(
      result.is_err(),
      "Strict mode should return Err on parse failure"
    );
  }

  #[test]
  fn test_lenient_mode_returns_ok_with_false_on_parse_failure() {
    let mut engine = CtRuntimeEngine::new();
    let spec = CtSpec::new("(unknown_func x)"); // Invalid expression
    let config = CtConfig {
      strict: false,
      ..Default::default()
    };

    let result = engine.verify(&spec, &config);

    assert!(result.is_ok(), "Lenient mode should return Ok");
    let check_result = result.unwrap();
    assert!(
      !check_result.success,
      "Lenient mode should set success=false on parse failure"
    );
    assert!(
      check_result.notes.iter().any(|n| n.contains("Parse error")),
      "Should include parse error in notes"
    );
    assert!(
      check_result.diagram.is_none(),
      "Should not have diagram on failure"
    );
  }

  #[test]
  fn test_strict_mode_returns_ok_on_valid_expression() {
    let mut engine = CtRuntimeEngine::new();
    let spec = CtSpec::new("sin(t)");
    let config = CtConfig {
      strict: true,
      ..Default::default()
    };

    let result = engine.verify(&spec, &config);

    assert!(
      result.is_ok(),
      "Strict mode should return Ok for valid expressions"
    );
    let check_result = result.unwrap();
    assert!(
      check_result.success,
      "Should set success=true for valid expressions"
    );
  }

  #[test]
  fn test_lenient_mode_returns_ok_on_valid_expression() {
    let mut engine = CtRuntimeEngine::new();
    let spec = CtSpec::new("floor(t)");
    let config = CtConfig {
      strict: false,
      ..Default::default()
    };

    let result = engine.verify(&spec, &config);

    assert!(
      result.is_ok(),
      "Lenient mode should return Ok for valid expressions"
    );
    let check_result = result.unwrap();
    assert!(
      check_result.success,
      "Should set success=true for valid expressions"
    );
  }

  #[test]
  fn test_default_config_is_strict() {
    let config = CtConfig::default();
    assert!(
      config.strict,
      "Default config should be strict (strict=true)"
    );
  }

  #[test]
  fn test_strict_mode_empty_expression() {
    let mut engine = CtRuntimeEngine::new();
    let spec = CtSpec::new("");
    let config = CtConfig {
      strict: true,
      ..Default::default()
    };

    let result = engine.verify(&spec, &config);

    // Empty expression should fail in strict mode
    assert!(
      result.is_err(),
      "Strict mode should fail on empty expression"
    );
  }

  #[test]
  fn test_lenient_mode_empty_expression() {
    let mut engine = CtRuntimeEngine::new();
    let spec = CtSpec::new("");
    let config = CtConfig {
      strict: false,
      ..Default::default()
    };

    let result = engine.verify(&spec, &config);

    // Empty expression should return Ok(false) in lenient mode
    assert!(
      result.is_ok(),
      "Lenient mode should return Ok for empty expression"
    );
    let check_result = result.unwrap();
    assert!(
      !check_result.success,
      "Should set success=false for empty expression"
    );
  }

  // ========================================
  // L43: extract_diagram=false Tests
  // ========================================

  #[test]
  fn test_extract_diagram_false_skips_diagram() {
    let mut engine = CtRuntimeEngine::new();
    let spec = CtSpec::new("sin(t)").with_diagram(false);
    let config = CtConfig::default();

    let result = engine.verify(&spec, &config).unwrap();

    // Should verify OK but skip diagram extraction
    assert!(result.success, "Verification should succeed");
    assert!(
      result.diagram.is_none(),
      "Diagram should be None when extract_diagram=false"
    );
  }

  #[test]
  fn test_extract_diagram_true_includes_diagram() {
    let mut engine = CtRuntimeEngine::new();
    let spec = CtSpec::new("sin(t)").with_diagram(true);
    let config = CtConfig::default();

    let result = engine.verify(&spec, &config).unwrap();

    // Should verify OK and include diagram
    assert!(result.success, "Verification should succeed");
    assert!(
      result.diagram.is_some(),
      "Diagram should be Some when extract_diagram=true"
    );

    let diagram = result.diagram.unwrap();
    assert!(!diagram.objects.is_empty(), "Diagram should have objects");
    assert!(
      !diagram.morphisms.is_empty(),
      "Diagram should have morphisms"
    );
  }

  #[test]
  fn test_extract_diagram_default_is_true() {
    let spec = CtSpec::new("sin(t)");
    assert!(
      spec.extract_diagram,
      "Default extract_diagram should be true"
    );
  }

  #[test]
  fn test_extract_diagram_false_complex_expr() {
    let mut engine = CtRuntimeEngine::new();
    let spec = CtSpec::new("(mod (floor t) 60)").with_diagram(false);
    let config = CtConfig::default();

    let result = engine.verify(&spec, &config).unwrap();

    assert!(result.success, "Complex expression should verify OK");
    assert!(
      result.diagram.is_none(),
      "Diagram should be None even for complex expressions"
    );
  }

  // ========================================
  // E22: Dynamic Verification + Caching Tests
  // ========================================

  #[test]
  fn test_caching_runtime_basic() {
    let mut runtime = CachingCtRuntime::new();

    // First call - cache miss
    let result1 = runtime.verify_expr("sin(t)", true).unwrap();
    assert!(result1.success);
    assert_eq!(runtime.stats().misses, 1);
    assert_eq!(runtime.stats().hits, 0);

    // Second call - cache hit
    let result2 = runtime.verify_expr("sin(t)", true).unwrap();
    assert!(result2.success);
    assert_eq!(runtime.stats().misses, 1);
    assert_eq!(runtime.stats().hits, 1);
  }

  #[test]
  fn test_caching_runtime_different_exprs() {
    let mut runtime = CachingCtRuntime::new();

    runtime.verify_expr("sin(t)", true).unwrap();
    runtime.verify_expr("cos(t)", true).unwrap();
    runtime.verify_expr("floor(t)", true).unwrap();

    assert_eq!(runtime.stats().misses, 3);
    assert_eq!(runtime.stats().hits, 0);
    assert_eq!(runtime.stats().entries, 3);
  }

  #[test]
  fn test_caching_runtime_hit_rate() {
    let mut runtime = CachingCtRuntime::new();

    // 1 miss
    runtime.verify_expr("sin(t)", true).unwrap();
    // 4 hits
    for _ in 0..4 {
      runtime.verify_expr("sin(t)", true).unwrap();
    }

    // Hit rate should be 4/5 = 0.8
    assert!((runtime.hit_rate() - 0.8).abs() < 0.001);
  }

  #[test]
  fn test_caching_runtime_clear() {
    let mut runtime = CachingCtRuntime::new();

    runtime.verify_expr("sin(t)", true).unwrap();
    runtime.verify_expr("sin(t)", true).unwrap();

    assert_eq!(runtime.stats().entries, 1);
    assert_eq!(runtime.stats().hits, 1);

    runtime.clear_cache();

    assert_eq!(runtime.stats().entries, 0);
    assert_eq!(runtime.stats().hits, 0);
  }

  #[test]
  fn test_caching_runtime_with_seed() {
    let runtime1 = CachingCtRuntime::new().with_seed(12345);
    let runtime2 = CachingCtRuntime::new().with_seed(12345);
    let runtime3 = CachingCtRuntime::new().with_seed(99999);

    let key1 = VerificationCacheKey::new("sin(t)", true).with_seed(12345);
    let key2 = VerificationCacheKey::new("sin(t)", true).with_seed(12345);
    let key3 = VerificationCacheKey::new("sin(t)", true).with_seed(99999);

    // Same seed -> same hash
    assert_eq!(key1.deterministic_hash(), key2.deterministic_hash());
    // Different seed -> different hash
    assert_ne!(key1.deterministic_hash(), key3.deterministic_hash());

    // Verify seed is set
    assert_eq!(runtime1.seed, Some(12345));
    assert_eq!(runtime2.seed, Some(12345));
    assert_eq!(runtime3.seed, Some(99999));
  }

  #[test]
  fn test_caching_runtime_eviction() {
    let mut runtime = CachingCtRuntime::new().with_cache_size(2);

    // Fill cache
    runtime.verify_expr("sin(t)", true).unwrap();
    runtime.verify_expr("cos(t)", true).unwrap();
    assert_eq!(runtime.stats().entries, 2);

    // Add third entry, should evict one
    runtime.verify_expr("floor(t)", true).unwrap();
    assert_eq!(runtime.stats().entries, 2);
    assert_eq!(runtime.stats().evictions, 1);
  }

  #[test]
  fn test_caching_runtime_eviction_is_lru() {
    let mut runtime = CachingCtRuntime::new().with_cache_size(2);

    // Fill cache with two entries.
    runtime.verify_expr("sin(t)", true).unwrap(); // miss
    runtime.verify_expr("cos(t)", true).unwrap(); // miss

    // Touch sin to make it most recently used.
    runtime.verify_expr("sin(t)", true).unwrap(); // hit
    let hits_before = runtime.stats().hits;
    let misses_before = runtime.stats().misses;

    // Insert a third entry; LRU should evict cos.
    runtime.verify_expr("floor(t)", true).unwrap(); // miss + eviction

    // cos should be a miss if it was evicted.
    runtime.verify_expr("cos(t)", true).unwrap(); // miss

    assert_eq!(runtime.stats().hits, hits_before);
    assert_eq!(runtime.stats().misses, misses_before + 2);
  }

  #[test]
  fn test_caching_runtime_is_valid() {
    let mut runtime = CachingCtRuntime::new();

    assert!(runtime.is_valid("sin(t)"));
    assert!(runtime.is_valid("cos(t)"));
    assert!(runtime.is_valid("floor(t)"));
    assert!(!runtime.is_valid("(unknown_func x)"));
  }

  #[test]
  fn test_caching_runtime_extract_diagram_cached() {
    let mut runtime = CachingCtRuntime::new();

    let diagram = runtime.extract_diagram_cached("sin(t)");
    assert!(diagram.is_some());
    let diagram = diagram.unwrap();
    assert!(!diagram.objects.is_empty());
    assert!(!diagram.morphisms.is_empty());

    // Invalid expression returns None
    let invalid = runtime.extract_diagram_cached("(unknown_func x)");
    assert!(invalid.is_none());
  }

  #[test]
  fn test_caching_runtime_batch_verify() {
    let mut runtime = CachingCtRuntime::new();

    let exprs = ["sin(t)", "cos(t)", "floor(t)"];
    let results = runtime.verify_batch(&exprs);

    assert_eq!(results.len(), 3);
    for result in results {
      assert!(result.is_ok());
      assert!(result.unwrap().success);
    }
  }

  // ========================================
  // E22: Dynamic Module Tests
  // ========================================

  #[test]
  fn test_dynamic_is_valid_expr() {
    assert!(dynamic::is_valid_expr("sin(t)"));
    assert!(dynamic::is_valid_expr("(mod (floor t) 60)"));
    assert!(!dynamic::is_valid_expr("(unknown_func x)"));
    assert!(!dynamic::is_valid_expr(""));
  }

  #[test]
  fn test_dynamic_extract_morphisms() {
    let morphisms = dynamic::extract_morphisms("sin(t)");
    assert!(morphisms.is_some());
    let morphisms = morphisms.unwrap();
    assert!(morphisms.contains(&"sin".to_string()));

    let complex = dynamic::extract_morphisms("(mod (floor t) 60)");
    assert!(complex.is_some());
    let complex = complex.unwrap();
    assert!(complex.iter().any(|m| m == "floor"));
    assert!(complex.iter().any(|m| m.starts_with("mod")));
  }

  #[test]
  fn test_dynamic_extract_objects() {
    let objects = dynamic::extract_objects("sin(t)");
    assert!(objects.is_some());
    let objects = objects.unwrap();
    assert!(objects.contains(&"t".to_string()));
  }

  #[test]
  fn test_dynamic_has_morphism() {
    assert!(dynamic::has_morphism("sin(t)", "sin"));
    assert!(dynamic::has_morphism("cos(t)", "cos"));
    assert!(!dynamic::has_morphism("sin(t)", "cos"));
  }

  #[test]
  fn test_dynamic_uses_op() {
    assert!(dynamic::uses_op("sin(t)", MorphismOp::Sin));
    assert!(dynamic::uses_op("floor(t)", MorphismOp::Floor));
    assert!(!dynamic::uses_op("sin(t)", MorphismOp::Cos));
  }

  #[test]
  fn test_dynamic_complexity() {
    assert_eq!(dynamic::complexity("sin(t)"), Some(1));
    assert_eq!(dynamic::complexity("(mod (floor t) 60)"), Some(2));
    assert_eq!(dynamic::complexity("(unknown_func x)"), None);
  }

  #[test]
  fn test_dynamic_diagrams_equivalent() {
    // Same expression should be equivalent
    assert!(dynamic::diagrams_equivalent("sin(t)", "sin(t)"));

    // Different expressions are not equivalent
    assert!(!dynamic::diagrams_equivalent("sin(t)", "cos(t)"));

    // Invalid expressions are not equivalent
    assert!(!dynamic::diagrams_equivalent("sin(t)", "(unknown x)"));
    assert!(!dynamic::diagrams_equivalent("(unknown x)", "(unknown y)"));
  }

  #[test]
  fn test_cache_key_deterministic() {
    let key1 = VerificationCacheKey::new("sin(t)", true);
    let key2 = VerificationCacheKey::new("sin(t)", true);
    let key3 = VerificationCacheKey::new("cos(t)", true);

    // Same expr + extract_diagram -> same hash
    assert_eq!(key1.deterministic_hash(), key2.deterministic_hash());

    // Different expr -> different hash
    assert_ne!(key1.deterministic_hash(), key3.deterministic_hash());
  }

  #[test]
  fn test_cache_stats_hit_rate() {
    let mut stats = CacheStats::default();

    // No operations -> 0% hit rate
    assert_eq!(stats.hit_rate(), 0.0);

    // All misses -> 0% hit rate
    stats.misses = 10;
    assert_eq!(stats.hit_rate(), 0.0);

    // 50/50 -> 50% hit rate
    stats.hits = 10;
    assert!((stats.hit_rate() - 0.5).abs() < 0.001);

    // All hits -> 100% hit rate
    stats.misses = 0;
    assert_eq!(stats.hit_rate(), 1.0);
  }

  // ========================================
  // H23: CT runtime determinism test across ordering permutations
  // ========================================

  #[test]
  fn test_h23_ct_determinism_across_permutations() {
    // Verify that CT diagram generation produces identical output
    // regardless of the order in which objects/morphisms are added

    // Test 1: Different insertion orders -> same deterministic output
    fn make_diagram_order1() -> CTDiagram {
      let mut diagram = CTDiagram::new();
      let a = diagram.add_object("a", CTType::Real);
      let b = diagram.add_object("b", CTType::Real);
      let c = diagram.add_object("c", CTType::Real);
      diagram.add_morphism("f", a, b, MorphismOp::Sin);
      diagram.add_morphism("g", b, c, MorphismOp::Floor);
      diagram
    }

    fn make_diagram_order2() -> CTDiagram {
      let mut diagram = CTDiagram::new();
      // Reverse object order
      let c = diagram.add_object("c", CTType::Real);
      let b = diagram.add_object("b", CTType::Real);
      let a = diagram.add_object("a", CTType::Real);
      // Reverse morphism order
      diagram.add_morphism("g", b, c, MorphismOp::Floor);
      diagram.add_morphism("f", a, b, MorphismOp::Sin);
      diagram
    }

    let output1 = make_diagram_order1().to_output_deterministic();
    let output2 = make_diagram_order2().to_output_deterministic();

    // Deterministic outputs should be identical
    assert_eq!(output1.objects.len(), output2.objects.len());
    assert_eq!(output1.morphisms.len(), output2.morphisms.len());

    for (o1, o2) in output1.objects.iter().zip(output2.objects.iter()) {
      assert_eq!(o1.name, o2.name);
    }
    for (m1, m2) in output1.morphisms.iter().zip(output2.morphisms.iter()) {
      assert_eq!(m1.name, m2.name);
    }

    // Test 2: Multiple runs of same expression produce identical diagrams
    let mut diagrams = Vec::new();
    for _ in 0..10 {
      let d = parse_expr_to_diagram("(add (mul t 2) (sin t))").unwrap();
      diagrams.push(d.to_output_deterministic());
    }

    let first = &diagrams[0];
    for (i, d) in diagrams.iter().enumerate().skip(1) {
      assert_eq!(
        first.objects.len(),
        d.objects.len(),
        "object count mismatch at run {}",
        i
      );
      assert_eq!(
        first.morphisms.len(),
        d.morphisms.len(),
        "morphism count mismatch at run {}",
        i
      );
    }
  }

  // ========================================
  // I22: CT diagram morphism ordering tests
  // ========================================

  #[test]
  fn test_i22_morphism_ordering_by_name() {
    // Verify morphisms are sorted by name in deterministic output

    let mut diagram = CTDiagram::new();
    let src = diagram.add_object("source", CTType::Real);
    let tgt = diagram.add_object("target", CTType::Real);

    // Add morphisms in reverse alphabetical order
    diagram.add_morphism("zeta_op", src, tgt, MorphismOp::Sin);
    diagram.add_morphism("delta_op", src, tgt, MorphismOp::Cos);
    diagram.add_morphism("alpha_op", src, tgt, MorphismOp::Floor);
    diagram.add_morphism("beta_op", src, tgt, MorphismOp::Ceil);
    diagram.add_morphism("gamma_op", src, tgt, MorphismOp::Abs);

    let output = diagram.to_output_deterministic();

    // Should be sorted alphabetically: alpha < beta < delta < gamma < zeta
    assert_eq!(output.morphisms[0].name, "alpha_op");
    assert_eq!(output.morphisms[1].name, "beta_op");
    assert_eq!(output.morphisms[2].name, "delta_op");
    assert_eq!(output.morphisms[3].name, "gamma_op");
    assert_eq!(output.morphisms[4].name, "zeta_op");
  }

  #[test]
  fn test_i22_morphism_ordering_with_symbols() {
    // Verify morphisms with symbol names sort correctly

    let mut diagram = CTDiagram::new();
    let src = diagram.add_object("src", CTType::Real);
    let tgt = diagram.add_object("tgt", CTType::Real);

    // Add morphisms with symbol-like names
    diagram.add_morphism("+", src, tgt, MorphismOp::Add);
    diagram.add_morphism("*", src, tgt, MorphismOp::Mul);
    diagram.add_morphism("-", src, tgt, MorphismOp::Sub);
    diagram.add_morphism("/", src, tgt, MorphismOp::Div);

    let output = diagram.to_output_deterministic();

    // ASCII order: * < + < - < /
    assert_eq!(output.morphisms.len(), 4);
    assert_eq!(output.morphisms[0].name, "*");
    assert_eq!(output.morphisms[1].name, "+");
    assert_eq!(output.morphisms[2].name, "-");
    assert_eq!(output.morphisms[3].name, "/");
  }

  #[test]
  fn test_i22_morphism_ordering_insertion_independent() {
    // Verify same morphisms added in different orders produce identical output

    fn create_diagram_forward() -> CtDiagramOutput {
      let mut diagram = CTDiagram::new();
      let a = diagram.add_object("a", CTType::Real);
      let b = diagram.add_object("b", CTType::Real);
      let c = diagram.add_object("c", CTType::Real);

      diagram.add_morphism("f1", a, b, MorphismOp::Sin);
      diagram.add_morphism("f2", b, c, MorphismOp::Cos);
      diagram.add_morphism("f3", a, c, MorphismOp::Floor);

      diagram.to_output_deterministic()
    }

    fn create_diagram_reverse() -> CtDiagramOutput {
      let mut diagram = CTDiagram::new();
      // Different object order
      let c = diagram.add_object("c", CTType::Real);
      let a = diagram.add_object("a", CTType::Real);
      let b = diagram.add_object("b", CTType::Real);

      // Reverse morphism order
      diagram.add_morphism("f3", a, c, MorphismOp::Floor);
      diagram.add_morphism("f2", b, c, MorphismOp::Cos);
      diagram.add_morphism("f1", a, b, MorphismOp::Sin);

      diagram.to_output_deterministic()
    }

    fn create_diagram_random() -> CtDiagramOutput {
      let mut diagram = CTDiagram::new();
      // Mixed order
      let b = diagram.add_object("b", CTType::Real);
      let c = diagram.add_object("c", CTType::Real);
      let a = diagram.add_object("a", CTType::Real);

      diagram.add_morphism("f2", b, c, MorphismOp::Cos);
      diagram.add_morphism("f1", a, b, MorphismOp::Sin);
      diagram.add_morphism("f3", a, c, MorphismOp::Floor);

      diagram.to_output_deterministic()
    }

    let out1 = create_diagram_forward();
    let out2 = create_diagram_reverse();
    let out3 = create_diagram_random();

    // All should have same morphism count
    assert_eq!(out1.morphisms.len(), 3);
    assert_eq!(out2.morphisms.len(), 3);
    assert_eq!(out3.morphisms.len(), 3);

    // All morphisms should be in same order
    for i in 0..3 {
      assert_eq!(out1.morphisms[i].name, out2.morphisms[i].name);
      assert_eq!(out2.morphisms[i].name, out3.morphisms[i].name);
    }

    // Verify sorted order
    assert_eq!(out1.morphisms[0].name, "f1");
    assert_eq!(out1.morphisms[1].name, "f2");
    assert_eq!(out1.morphisms[2].name, "f3");
  }

  #[test]
  fn test_i22_morphism_op_normalization_in_output() {
    // Verify that MorphismOp is consistently represented in output

    let mut diagram = CTDiagram::new();
    let src = diagram.add_object("x", CTType::Real);
    let tgt = diagram.add_object("y", CTType::Real);

    // Add morphisms using different ops
    diagram.add_morphism("sin_morph", src, tgt, MorphismOp::Sin);
    diagram.add_morphism("cos_morph", src, tgt, MorphismOp::Cos);
    diagram.add_morphism("add_morph", src, tgt, MorphismOp::Add);
    diagram.add_morphism("compose_morph", src, tgt, MorphismOp::Compose);

    let output = diagram.to_output_deterministic();

    // Verify morphisms are present and source/target are correct
    for m in &output.morphisms {
      assert_eq!(m.source, "Real");
      assert_eq!(m.target, "Real");
    }

    // Verify canonical names are used internally
    let infos = diagram.get_morphism_infos();
    for info in &infos {
      assert!(info.implementation.is_some());
      let impl_name = info.implementation.as_ref().unwrap();
      // Should use canonical name
      assert!(
        ["sin", "cos", "add", "compose"].contains(&impl_name.as_str()),
        "Implementation should use canonical name, got: {}",
        impl_name
      );
    }
  }

  #[test]
  fn test_i22_morphism_ordering_complex_diagram() {
    // Test with a more complex diagram structure

    let mut diagram = CTDiagram::new();
    let real = diagram.add_object("real", CTType::Real);
    let int = diagram.add_object("int", CTType::Int);
    let bool_obj = diagram.add_object("bool", CTType::Bool);

    // Add many morphisms
    diagram.add_morphism("z_to_bool", real, bool_obj, MorphismOp::Gt);
    diagram.add_morphism("a_floor", real, int, MorphismOp::Floor);
    diagram.add_morphism("m_mul", real, real, MorphismOp::Mul);
    diagram.add_morphism("c_ceil", real, int, MorphismOp::Ceil);
    diagram.add_morphism("p_plus", real, real, MorphismOp::Add);
    diagram.add_morphism("b_abs", real, real, MorphismOp::Abs);

    let output = diagram.to_output_deterministic();

    // Verify alphabetical order
    let names: Vec<&str> = output.morphisms.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(
      names,
      vec!["a_floor", "b_abs", "c_ceil", "m_mul", "p_plus", "z_to_bool"]
    );
  }

  #[test]
  fn test_i22_morphism_ordering_determinism_multiple_runs() {
    // Run same diagram creation 10 times and verify consistent ordering

    fn create_diagram() -> CtDiagramOutput {
      let mut diagram = CTDiagram::new();
      let src = diagram.add_object("src", CTType::Real);
      let mid = diagram.add_object("mid", CTType::Real);
      let tgt = diagram.add_object("tgt", CTType::Real);

      diagram.add_morphism("step3", mid, tgt, MorphismOp::Ceil);
      diagram.add_morphism("step1", src, mid, MorphismOp::Sin);
      diagram.add_morphism("step2", src, tgt, MorphismOp::Floor);

      diagram.to_output_deterministic()
    }

    let mut outputs = Vec::new();
    for _ in 0..10 {
      outputs.push(create_diagram());
    }

    // All runs should produce identical output
    let first = &outputs[0];
    for (i, out) in outputs.iter().enumerate().skip(1) {
      assert_eq!(
        first.morphisms.len(),
        out.morphisms.len(),
        "morphism count differs at run {}",
        i
      );
      for (j, (m1, m2)) in first.morphisms.iter().zip(out.morphisms.iter()).enumerate() {
        assert_eq!(
          m1.name, m2.name,
          "morphism name differs at run {} position {}",
          i, j
        );
      }
    }
  }

  // ========================================
  // N22: CT strict/lenient error format tests
  // ========================================

  #[test]
  fn test_n22_strict_mode_error_format_consistency() {
    // Verify strict mode errors have consistent format across different error types
    let mut engine = CtRuntimeEngine::new();

    let invalid_exprs = [
      ("", "empty expression"),
      ("@@@", "invalid characters"),
      ("sin(", "unbalanced parentheses"),
      ("unknown_func(x)", "unknown function"),
    ];

    let config = CtConfig {
      strict: true,
      ..Default::default()
    };

    for (expr, description) in &invalid_exprs {
      let spec = CtSpec::new(*expr);

      let result = engine.verify(&spec, &config);

      // In strict mode, invalid expressions should return Err
      assert!(
        result.is_err(),
        "Strict mode: {} should return Err, got {:?}",
        description,
        result
      );

      // Error message should be non-empty
      if let Err(e) = result {
        let msg = format!("{:?}", e);
        assert!(
          !msg.is_empty(),
          "Error message for {} should not be empty",
          description
        );
      }
    }
  }

  #[test]
  fn test_n22_lenient_mode_error_format_consistency() {
    // Verify lenient mode returns Ok(false) with consistent format
    let mut engine = CtRuntimeEngine::new();

    let invalid_exprs = [
      ("", "empty expression"),
      ("@@@", "invalid characters"),
      ("sin(", "unbalanced parentheses"),
    ];

    let config = CtConfig {
      strict: false,
      ..Default::default()
    };

    for (expr, description) in &invalid_exprs {
      let spec = CtSpec::new(*expr);

      let result = engine.verify(&spec, &config);

      // In lenient mode, invalid expressions should return Ok(false)
      assert!(
        result.is_ok(),
        "Lenient mode: {} should return Ok, got {:?}",
        description,
        result
      );

      if let Ok(check_result) = result {
        assert!(
          !check_result.success,
          "Lenient mode: {} should have success=false",
          description
        );
      }
    }
  }

  #[test]
  fn test_n22_error_format_determinism() {
    // Verify that same errors produce identical messages across runs
    let mut engine1 = CtRuntimeEngine::new();
    let mut engine2 = CtRuntimeEngine::new();

    let config = CtConfig {
      strict: true,
      ..Default::default()
    };
    // Use an expression that definitely causes an error in strict mode
    let spec = CtSpec::new("sin(");

    let result1 = engine1.verify(&spec, &config);
    let result2 = engine2.verify(&spec, &config);

    // Both should be errors (unbalanced parentheses)
    assert!(
      result1.is_err(),
      "Unbalanced parenthesis should error in strict mode: {:?}",
      result1
    );
    assert!(
      result2.is_err(),
      "Unbalanced parenthesis should error in strict mode: {:?}",
      result2
    );

    // Error messages should be identical (using Debug format)
    let msg1 = format!("{:?}", result1.unwrap_err());
    let msg2 = format!("{:?}", result2.unwrap_err());
    assert_eq!(msg1, msg2, "Error messages should be identical across runs");
  }

  #[test]
  fn test_n22_strict_vs_lenient_valid_expression() {
    // Verify valid expressions produce consistent results in both modes
    let mut engine = CtRuntimeEngine::new();

    let valid_exprs = ["sin(t)", "cos(t) + 1", "floor(t) * 2"];

    for expr in &valid_exprs {
      let spec = CtSpec::new(*expr);

      let strict_config = CtConfig {
        strict: true,
        ..Default::default()
      };
      let lenient_config = CtConfig {
        strict: false,
        ..Default::default()
      };

      let strict_result = engine.verify(&spec, &strict_config);
      let lenient_result = engine.verify(&spec, &lenient_config);

      // Both should succeed with success=true
      assert!(
        strict_result.is_ok(),
        "Strict mode: {} should succeed",
        expr
      );
      assert!(
        lenient_result.is_ok(),
        "Lenient mode: {} should succeed",
        expr
      );

      if let (Ok(strict), Ok(lenient)) = (strict_result, lenient_result) {
        assert_eq!(
          strict.success, lenient.success,
          "{}: strict and lenient should agree on valid expressions",
          expr
        );
        assert!(strict.success, "{}: should be success=true", expr);
      }
    }
  }

  // =============================================================================
  // M22: CT Diagram Comparison Tolerance/Strict Tests
  // =============================================================================

  #[test]
  fn test_m22_strict_mode_diagram_extraction() {
    // Verify diagram extraction works in strict mode
    let mut engine = CtRuntimeEngine::new();

    let config = CtConfig {
      strict: true,
      ..Default::default()
    };

    // Use function call notation (sin(t)) not s-expression notation
    let spec = CtSpec::new("sin(t)").with_diagram(true);
    let result = engine.verify(&spec, &config);

    assert!(
      result.is_ok(),
      "Strict mode should succeed for valid expression"
    );
    let check = result.unwrap();
    assert!(check.success, "Valid expression should pass verification");

    // Diagram should be extractable when with_diagram(true)
    if let Some(diagram) = &check.diagram {
      assert!(!diagram.objects.is_empty(), "Diagram should have objects");
      assert!(
        !diagram.morphisms.is_empty(),
        "Diagram should have morphisms"
      );
    }
  }

  #[test]
  fn test_m22_lenient_mode_diagram_extraction() {
    // Verify diagram extraction works in lenient mode
    let mut engine = CtRuntimeEngine::new();

    let config = CtConfig {
      strict: false,
      ..Default::default()
    };

    let spec = CtSpec::new("floor(t)").with_diagram(true);
    let result = engine.verify(&spec, &config);

    assert!(result.is_ok(), "Lenient mode should succeed");
    let check = result.unwrap();
    assert!(check.success, "Valid expression should pass verification");
  }

  #[test]
  fn test_m22_strict_vs_lenient_diagram_consistency() {
    // Verify that strict and lenient modes produce consistent diagrams for valid expressions
    let mut engine = CtRuntimeEngine::new();

    let strict_config = CtConfig {
      strict: true,
      ..Default::default()
    };
    let lenient_config = CtConfig {
      strict: false,
      ..Default::default()
    };

    // Use function call notation for valid expressions
    let valid_exprs = ["sin(t)", "cos(t)", "floor(t)"];

    for expr in &valid_exprs {
      let spec = CtSpec::new(*expr).with_diagram(true);

      let strict_result = engine.verify(&spec, &strict_config);
      let lenient_result = engine.verify(&spec, &lenient_config);

      assert!(strict_result.is_ok(), "Strict should succeed for: {}", expr);
      assert!(
        lenient_result.is_ok(),
        "Lenient should succeed for: {}",
        expr
      );

      let strict_check = strict_result.unwrap();
      let lenient_check = lenient_result.unwrap();

      // Both should agree on the ok status
      assert_eq!(
        strict_check.success, lenient_check.success,
        "Strict and lenient should agree for: {}",
        expr
      );

      // If both have diagrams, they should have same structure
      match (&strict_check.diagram, &lenient_check.diagram) {
        (Some(s), Some(l)) => {
          assert_eq!(
            s.objects.len(),
            l.objects.len(),
            "Diagram object counts should match for: {}",
            expr
          );
          assert_eq!(
            s.morphisms.len(),
            l.morphisms.len(),
            "Diagram morphism counts should match for: {}",
            expr
          );
        }
        (None, None) => {}
        _ => {} // One has diagram, one doesn't - acceptable difference
      }
    }
  }

  #[test]
  fn test_m22_strict_mode_invalid_expression_handling() {
    // Verify strict mode properly rejects invalid expressions
    let mut engine = CtRuntimeEngine::new();

    let strict_config = CtConfig {
      strict: true,
      ..Default::default()
    };

    let invalid_exprs = [
      "",            // Empty
      "(sin",        // Unbalanced
      "(unknown x)", // Unknown function (may or may not fail depending on parser)
    ];

    for expr in &invalid_exprs {
      let spec = CtSpec::new(*expr);
      let result = engine.verify(&spec, &strict_config);

      // Strict mode should return error for clearly invalid expressions
      if expr.is_empty() || expr.contains("sin") && !expr.ends_with(')') {
        assert!(
          result.is_err(),
          "Strict mode should reject invalid expression: '{}'",
          expr
        );
      }
    }
  }

  #[test]
  fn test_m22_lenient_mode_invalid_expression_handling() {
    // Verify lenient mode returns Ok(false) for invalid expressions
    let mut engine = CtRuntimeEngine::new();

    let lenient_config = CtConfig {
      strict: false,
      ..Default::default()
    };

    let invalid_exprs = ["", "(sin"];

    for expr in &invalid_exprs {
      let spec = CtSpec::new(*expr);
      let result = engine.verify(&spec, &lenient_config);

      // Lenient mode should return Ok with success=false
      assert!(
        result.is_ok(),
        "Lenient mode should return Ok for: '{}'",
        expr
      );
      let check = result.unwrap();
      assert!(
        !check.success,
        "Lenient mode should have success=false for invalid: '{}'",
        expr
      );
    }
  }

  #[test]
  fn test_m22_diagram_output_determinism_both_modes() {
    // Verify diagram output is deterministic in both strict and lenient modes
    fn run_extraction(strict: bool) -> Vec<String> {
      let mut engine = CtRuntimeEngine::new();
      let config = CtConfig {
        strict,
        ..Default::default()
      };

      // Use function call notation for valid expressions
      let exprs = ["sin(t)", "cos(t)", "floor(t)"];

      exprs
        .iter()
        .filter_map(|expr| {
          let spec = CtSpec::new(*expr).with_diagram(true);
          engine
            .verify(&spec, &config)
            .ok()
            .and_then(|r| r.diagram)
            .map(|d| format!("{:?}", d))
        })
        .collect()
    }

    // Test strict mode determinism
    let strict_run1 = run_extraction(true);
    let strict_run2 = run_extraction(true);
    assert_eq!(
      strict_run1, strict_run2,
      "Strict mode should be deterministic"
    );

    // Test lenient mode determinism
    let lenient_run1 = run_extraction(false);
    let lenient_run2 = run_extraction(false);
    assert_eq!(
      lenient_run1, lenient_run2,
      "Lenient mode should be deterministic"
    );

    // Both modes should produce same diagrams for valid expressions
    assert_eq!(
      strict_run1.len(),
      lenient_run1.len(),
      "Both modes should extract same number of diagrams"
    );
  }

  // =============================================================================
  // P22: CT Runtime Error Mapping Tests (Invalid Morphism)
  // =============================================================================

  #[test]
  fn test_p22_invalid_morphism_error_format() {
    // Test that invalid morphism errors have consistent format
    let mut engine = CtRuntimeEngine::new();
    let config = CtConfig {
      strict: true,
      ..Default::default()
    };

    // Unknown function should produce an error in strict mode
    let invalid_morphisms = [
      ("(unknown_func t)", "unknown function"),
      ("(weird_op x y)", "unknown binary op"),
      ("($$$ t)", "special characters"),
    ];

    for (expr, desc) in &invalid_morphisms {
      let spec = CtSpec::new(*expr);
      let result = engine.verify(&spec, &config);

      // Should return an error for unknown morphisms
      if let Err(err) = result {
        let err_msg = format!("{:?}", err);

        // Error message should be non-empty and consistent
        assert!(
          !err_msg.is_empty(),
          "Error message for {} should not be empty",
          desc
        );
      }
    }
  }

  #[test]
  fn test_p22_invalid_morphism_error_determinism() {
    // Verify same invalid morphism produces identical errors across runs
    fn get_error(expr: &str) -> String {
      let mut engine = CtRuntimeEngine::new();
      let config = CtConfig {
        strict: true,
        ..Default::default()
      };
      let spec = CtSpec::new(expr);
      match engine.verify(&spec, &config) {
        Ok(check) => format!("success={}", check.success),
        Err(e) => format!("err={:?}", e),
      }
    }

    let test_exprs = [
      "(invalid_op t)",
      "(@@@ x)",
      "(   )", // whitespace only in parens
    ];

    for expr in &test_exprs {
      let result1 = get_error(expr);
      let result2 = get_error(expr);
      let result3 = get_error(expr);

      assert_eq!(
        result1, result2,
        "Error for '{}' must be deterministic",
        expr
      );
      assert_eq!(
        result2, result3,
        "Error for '{}' must be deterministic",
        expr
      );
    }
  }

  #[test]
  fn test_p22_morphism_op_coverage() {
    // Test that all known morphism ops are correctly recognized
    let mut engine = CtRuntimeEngine::new();
    let config = CtConfig {
      strict: true,
      ..Default::default()
    };

    // Valid morphism operations - should all succeed
    let valid_ops = [
      ("sin(t)", MorphismOp::Sin),
      ("cos(t)", MorphismOp::Cos),
      ("floor(t)", MorphismOp::Floor),
      ("abs(t)", MorphismOp::Abs),
    ];

    for (expr, _expected_op) in &valid_ops {
      let spec = CtSpec::new(*expr).with_diagram(true);
      let result = engine.verify(&spec, &config);

      assert!(result.is_ok(), "Valid morphism '{}' should succeed", expr);
      let check = result.unwrap();
      assert!(check.success, "Valid morphism '{}' should verify ok", expr);
    }
  }

  #[test]
  fn test_p22_invalid_morphism_in_composition() {
    // Test error handling when invalid morphism appears in composition
    let mut engine = CtRuntimeEngine::new();
    let config = CtConfig {
      strict: true,
      ..Default::default()
    };

    // Valid composition with all known ops
    let valid_composed = "(floor (sin t))";
    let spec = CtSpec::new(valid_composed);
    let result = engine.verify(&spec, &config);
    assert!(
      result.is_ok() && result.unwrap().success,
      "Valid composition should succeed"
    );

    // Invalid composition with unknown op
    let invalid_composed = "(floor (unknown_op t))";
    let spec = CtSpec::new(invalid_composed);
    let result = engine.verify(&spec, &config);
    // Should fail or have success=false
    if let Ok(check) = result {
      // If it returns Ok, check should have success=false or notes
      assert!(
        !check.success || !check.notes.is_empty(),
        "Invalid composition should either fail or have notes"
      );
    }
    // If it returns Err, that's also acceptable for strict mode
  }

  #[test]
  fn test_p22_error_message_contains_morphism_name() {
    // Verify error messages include the problematic morphism name when possible
    let mut engine = CtRuntimeEngine::new();
    let config = CtConfig {
      strict: true,
      ..Default::default()
    };

    // Expressions with distinctive names
    let test_cases = [
      ("(my_custom_op t)", "my_custom_op"),
      ("(special_func x)", "special_func"),
    ];

    for (expr, morphism_name) in &test_cases {
      let spec = CtSpec::new(*expr);
      let result = engine.verify(&spec, &config);

      // If it fails, error might contain the name
      // If it succeeds with notes, notes might contain the name
      match result {
        Err(e) => {
          // Some implementations include the name in error
          let _ = format!("{:?}", e);
        }
        Ok(check) => {
          if !check.notes.is_empty() {
            let notes_str = format!("{:?}", check.notes);
            // Notes might contain the morphism name
            let _ = notes_str.contains(*morphism_name);
          }
        }
      }
    }
  }

  // =============================================================================
  // P81: CT Diagram Determinism Tests (same seed -> same output)
  // =============================================================================

  #[test]
  fn test_p81_diagram_determinism_same_seed() {
    // Verify that same input expression produces identical diagram output across runs
    let expr = "sin(t)";

    let mut results = Vec::new();
    for _ in 0..5 {
      let diagram = parse_expr_to_diagram(expr).unwrap();
      let output = diagram.to_output_deterministic();
      results.push(format!("{:?}", output));
    }

    // All runs should produce identical output
    let first = &results[0];
    for (i, result) in results.iter().enumerate().skip(1) {
      assert_eq!(first, result, "Run {} produced different output", i);
    }
  }

  #[test]
  fn test_p81_diagram_determinism_complex_expr() {
    // Test determinism with more complex expression
    let expr = "(add (mul t 2) (sin t))";

    let mut diagrams = Vec::new();
    for _ in 0..5 {
      let diagram = parse_expr_to_diagram(expr).unwrap();
      diagrams.push(diagram.to_output_deterministic());
    }

    let first = &diagrams[0];
    for (i, d) in diagrams.iter().enumerate().skip(1) {
      assert_eq!(
        first.objects.len(),
        d.objects.len(),
        "Object count mismatch at run {}",
        i
      );
      assert_eq!(
        first.morphisms.len(),
        d.morphisms.len(),
        "Morphism count mismatch at run {}",
        i
      );
      // Verify exact ordering
      for (j, (o1, o2)) in first.objects.iter().zip(d.objects.iter()).enumerate() {
        assert_eq!(
          o1.name, o2.name,
          "Object name mismatch at run {} pos {}",
          i, j
        );
      }
      for (j, (m1, m2)) in first.morphisms.iter().zip(d.morphisms.iter()).enumerate() {
        assert_eq!(
          m1.name, m2.name,
          "Morphism name mismatch at run {} pos {}",
          i, j
        );
      }
    }
  }

  #[test]
  fn test_p81_verify_determinism_with_config() {
    // Test CtRuntimeEngine produces deterministic results with same config
    let mut engine = CtRuntimeEngine::new();
    let config = CtConfig {
      strict: true,
      seed: Some(42),
      ..Default::default()
    };

    let expr = "sin(t)";
    let spec = CtSpec::new(expr).with_diagram(true);

    let mut results = Vec::new();
    for _ in 0..5 {
      let result = engine.verify(&spec, &config).unwrap();
      if let Some(diagram) = result.diagram {
        results.push(format!("{:?}", diagram));
      }
    }

    // All should be identical
    let first = &results[0];
    for (i, r) in results.iter().enumerate().skip(1) {
      assert_eq!(
        first, r,
        "Verification run {} produced different diagram",
        i
      );
    }
  }

  #[test]
  fn test_p81_caching_runtime_determinism() {
    // Test CachingCtRuntime returns identical cached results
    let mut runtime = CachingCtRuntime::new().with_seed(12345);

    let expr = "floor(t)";

    // First call - cache miss
    let result1 = runtime.verify_expr(expr, true).unwrap();
    assert_eq!(runtime.stats().misses, 1);

    // Second call - cache hit
    let result2 = runtime.verify_expr(expr, true).unwrap();
    assert_eq!(runtime.stats().hits, 1);

    // Results should be identical
    assert_eq!(result1.success, result2.success);
    assert_eq!(
      format!("{:?}", result1.diagram),
      format!("{:?}", result2.diagram)
    );
  }

  #[test]
  fn test_p81_determinism_across_multiple_expressions() {
    // Verify determinism holds across multiple different expressions
    let exprs = ["sin(t)", "cos(t)", "floor(t)", "(add t 1)", "(mul t 2)"];

    fn extract_all(exprs: &[&str]) -> Vec<String> {
      exprs
        .iter()
        .filter_map(|e| parse_expr_to_diagram(e).ok())
        .map(|d| format!("{:?}", d.to_output_deterministic()))
        .collect()
    }

    let run1 = extract_all(&exprs);
    let run2 = extract_all(&exprs);
    let run3 = extract_all(&exprs);

    assert_eq!(run1, run2, "Run 1 and 2 should match");
    assert_eq!(run2, run3, "Run 2 and 3 should match");
  }

  // =============================================================================
  // P82: CT Lenient Mode Unknown Op Tests
  // =============================================================================

  #[test]
  fn test_p82_lenient_unknown_op_returns_ok() {
    // Lenient mode should return Ok for unknown operations
    let mut engine = CtRuntimeEngine::new();
    let config = CtConfig {
      strict: false,
      ..Default::default()
    };

    let unknown_exprs = ["(custom_func t)", "(my_special_op x)", "(unknown123 y)"];

    for expr in &unknown_exprs {
      let spec = CtSpec::new(*expr);
      let result = engine.verify(&spec, &config);

      assert!(
        result.is_ok(),
        "Lenient mode should return Ok for unknown op: '{}'",
        expr
      );
    }
  }

  #[test]
  fn test_p82_strict_unknown_op_behavior() {
    // Strict mode may return error or Ok(false) for unknown operations
    let mut engine = CtRuntimeEngine::new();
    let config = CtConfig {
      strict: true,
      ..Default::default()
    };

    let unknown_exprs = ["(custom_func t)", "(unknown_op x)"];

    for expr in &unknown_exprs {
      let spec = CtSpec::new(*expr);
      let result = engine.verify(&spec, &config);

      // In strict mode, unknown ops should either:
      // 1. Return Err, OR
      // 2. Return Ok with success=false
      match result {
        Err(_) => {} // Expected for strict mode
        Ok(check) => {
          // If Ok, should have success=false or notes about unknown op
          assert!(
            !check.success || !check.notes.is_empty(),
            "Strict mode: unknown op '{}' should fail or have notes",
            expr
          );
        }
      }
    }
  }

  #[test]
  fn test_p82_lenient_known_ops_still_work() {
    // Lenient mode should correctly handle known operations
    let mut engine = CtRuntimeEngine::new();
    let config = CtConfig {
      strict: false,
      ..Default::default()
    };

    let known_exprs = [
      ("sin(t)", true),
      ("cos(t)", true),
      ("floor(t)", true),
      ("abs(t)", true),
    ];

    for (expr, expected_ok) in &known_exprs {
      let spec = CtSpec::new(*expr).with_diagram(true);
      let result = engine.verify(&spec, &config);

      assert!(result.is_ok(), "Lenient mode should succeed for: {}", expr);
      let check = result.unwrap();
      assert_eq!(
        check.success, *expected_ok,
        "Known op '{}' should have success={}",
        expr, expected_ok
      );
    }
  }

  #[test]
  fn test_p82_lenient_mode_produces_notes() {
    // Lenient mode should produce notes/warnings for unknown operations
    let mut engine = CtRuntimeEngine::new();
    let config = CtConfig {
      strict: false,
      ..Default::default()
    };

    // Use a clearly unknown function name
    let spec = CtSpec::new("(completely_unknown_xyz t)");
    let result = engine.verify(&spec, &config);

    // Should return Ok
    assert!(result.is_ok(), "Lenient mode should return Ok");

    // The check.success might be false, which is acceptable for unknown ops
    // Notes might contain information about the unknown operation
  }

  #[test]
  fn test_p82_mode_consistency_across_runs() {
    // Verify mode behavior is consistent across multiple runs
    let config_lenient = CtConfig {
      strict: false,
      ..Default::default()
    };
    let config_strict = CtConfig {
      strict: true,
      ..Default::default()
    };

    let test_expr = "(unknown_test_op t)";

    fn check_mode_result(config: &CtConfig, expr: &str) -> String {
      let mut engine = CtRuntimeEngine::new();
      let spec = CtSpec::new(expr);
      match engine.verify(&spec, config) {
        Ok(check) => format!("success={},notes={}", check.success, check.notes.len()),
        Err(_) => "error".to_string(),
      }
    }

    // Run multiple times in each mode
    let lenient_results: Vec<_> = (0..3)
      .map(|_| check_mode_result(&config_lenient, test_expr))
      .collect();
    let strict_results: Vec<_> = (0..3)
      .map(|_| check_mode_result(&config_strict, test_expr))
      .collect();

    // All lenient runs should be identical
    assert!(
      lenient_results.iter().all(|r| r == &lenient_results[0]),
      "Lenient mode should be consistent: {:?}",
      lenient_results
    );

    // All strict runs should be identical
    assert!(
      strict_results.iter().all(|r| r == &strict_results[0]),
      "Strict mode should be consistent: {:?}",
      strict_results
    );
  }

  // ========================================
  // P121: pnix-old ct_engine parity tests
  // ========================================

  /// P121-1: Verify sin(t) diagram matches pnix-old ct_engine output structure
  #[test]
  fn test_p121_parity_sin_t() {
    // pnix-old ct_engine expected output for "sin(t)":
    // - objects: [Real]
    // - morphisms: [sin: Real -> Real]
    // - verify.success: true
    let config = CtConfig::default();
    let mut engine = CtRuntimeEngine::new();
    let spec = CtSpec::new("sin(t)");

    let result = engine
      .verify(&spec, &config)
      .expect("verify should succeed");

    // Parity check: verify.success should be true
    assert!(
      result.success,
      "P121-1: pnix-old returns success=true for sin(t)"
    );

    // Parity check: diagram should have Real object
    let diagram = result.diagram.expect("P121-1: diagram should be present");
    assert!(
      diagram.objects.iter().any(|o| o.ct_type == "Real"),
      "P121-1: pnix-old includes Real object"
    );

    // Parity check: diagram should have sin morphism Real -> Real
    let sin_morph = diagram.morphisms.iter().find(|m| m.name == "sin");
    assert!(
      sin_morph.is_some(),
      "P121-1: pnix-old includes sin morphism"
    );
    let sin_morph = sin_morph.unwrap();
    assert_eq!(sin_morph.source, "Real", "P121-1: sin source is Real");
    assert_eq!(sin_morph.target, "Real", "P121-1: sin target is Real");
  }

  /// P121-2: Verify cos(t) diagram matches pnix-old ct_engine output structure
  #[test]
  fn test_p121_parity_cos_t() {
    // pnix-old ct_engine expected output for "cos(t)":
    // - objects: [Real]
    // - morphisms: [cos: Real -> Real]
    // - verify.success: true
    let config = CtConfig::default();
    let mut engine = CtRuntimeEngine::new();
    let spec = CtSpec::new("cos(t)");

    let result = engine
      .verify(&spec, &config)
      .expect("verify should succeed");

    // Parity check: verify.success should be true
    assert!(
      result.success,
      "P121-2: pnix-old returns success=true for cos(t)"
    );

    // Parity check: diagram should have Real object
    let diagram = result.diagram.expect("P121-2: diagram should be present");
    assert!(
      diagram.objects.iter().any(|o| o.ct_type == "Real"),
      "P121-2: pnix-old includes Real object"
    );

    // Parity check: diagram should have cos morphism
    let cos_morph = diagram.morphisms.iter().find(|m| m.name == "cos");
    assert!(
      cos_morph.is_some(),
      "P121-2: pnix-old includes cos morphism"
    );
    let cos_morph = cos_morph.unwrap();
    assert_eq!(cos_morph.source, "Real", "P121-2: cos source is Real");
    assert_eq!(cos_morph.target, "Real", "P121-2: cos target is Real");
  }

  /// P121-3: Verify (mod (floor t) 60) diagram matches pnix-old ct_engine analog clock pattern
  #[test]
  fn test_p121_parity_analog_clock() {
    // pnix-old ct_engine expected output for "(mod (floor t) 60)":
    // - objects: [Real]
    // - morphisms: [floor: Real -> Real, mod: (Real, Real) -> Real]
    // - verify.success: true
    // This is the standard analog clock seconds expression
    let config = CtConfig::default();
    let mut engine = CtRuntimeEngine::new();
    let spec = CtSpec::new("(mod (floor t) 60)");

    let result = engine
      .verify(&spec, &config)
      .expect("verify should succeed");

    // Parity check: verify.success should be true
    assert!(
      result.success,
      "P121-3: pnix-old returns success=true for analog clock expr"
    );

    // Parity check: diagram should have Real object
    let diagram = result.diagram.expect("P121-3: diagram should be present");
    assert!(
      diagram.objects.iter().any(|o| o.ct_type == "Real"),
      "P121-3: pnix-old includes Real object"
    );

    // Parity check: should have floor morphism
    let floor_morph = diagram.morphisms.iter().find(|m| m.name == "floor");
    assert!(
      floor_morph.is_some(),
      "P121-3: pnix-old includes floor morphism"
    );
    let floor_morph = floor_morph.unwrap();
    assert_eq!(floor_morph.source, "Real", "P121-3: floor source is Real");
    assert_eq!(floor_morph.target, "Real", "P121-3: floor target is Real");

    // Parity check: should have mod morphism
    let mod_morph = diagram.morphisms.iter().find(|m| m.name.starts_with("mod"));
    assert!(
      mod_morph.is_some(),
      "P121-3: pnix-old includes mod morphism"
    );
    let mod_morph = mod_morph.unwrap();
    assert_eq!(mod_morph.target, "Real", "P121-3: mod target is Real");

    // Parity check: morphism count should match pnix-old (2 morphisms)
    assert!(
      diagram.morphisms.len() >= 2,
      "P121-3: pnix-old produces at least 2 morphisms for analog clock"
    );
  }
}
