//! Type Inference: PnixExpr/AST 타입 추론
//!
//! 변수 타입 자동 추론을 위한 기본 구조.
//!
//! ## 설계 원칙
//!
//! 1. **Constraint-based**: 제약 생성 + unify 기반
//! 2. **Minimal support**: let/app/if부터 최소 지원
//! 3. **헌법 준수 (P0-1)**: 구조 정의만, 값 계산 없음

use super::CoreType;
use crate::lang::pnix::syntax::{
  PnixExpr, PnixLetBinding, PnixListPattern, PnixLiteralPattern, PnixMatchArm, PnixParamPattern,
  PnixPattern, PnixPatternField,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use thiserror::Error;

/// 타입 추론 결과: 타입 추론의 결과 정보
#[derive(Debug, Clone, Default)]
pub struct InferenceResult {
  /// 추론된 변수 타입 (변수 이름 → 타입 매핑)
  pub var_types: HashMap<String, CoreType>,
  /// 표현식 전체의 타입 (결정적 결과용, "_result" 키로도 저장됨)
  pub expr_type: Option<CoreType>,
  /// 추론된 표현식 타입 (표현식 ID → 타입 매핑)
  pub expr_types: HashMap<usize, CoreType>,
  /// 발견된 에러들 (타입 추론 중 발생한 에러 목록)
  pub errors: Vec<InferenceError>,
}

impl InferenceResult {
  /// 새 타입 추론 결과 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self::default()
  }

  /// 에러 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn add_error(&mut self, error: InferenceError) {
    self.errors.push(error);
  }

  /// 결과 타입을 설정하고 var_types에도 저장
  ///
  /// 표현식의 최종 타입을 설정하는 헬퍼 메서드입니다.
  /// `_result` 키로 var_types에 저장하고 expr_type도 설정합니다.
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 설정만, 값 계산 없음
  pub fn set_result_type(&mut self, ty: CoreType) {
    self.var_types.insert("_result".to_string(), ty.clone());
    self.expr_type = Some(ty);
  }
}

/// 타입 추론 에러: 타입 추론 중 발생하는 에러 타입
#[derive(Debug, Error, Clone)]
pub enum InferenceError {
  #[error("Cannot infer type for variable: {name}")]
  UninferrableVar {
    /// 변수 이름
    name: String,
  },

  #[error("Type mismatch: expected {expected}, got {actual}")]
  TypeMismatch {
    /// 예상 타입
    expected: CoreType,
    /// 실제 타입
    actual: CoreType,
  },

  #[error("Non-exhaustive match: {reason}")]
  NonExhaustiveMatch {
    /// 이유
    reason: String,
  },

  #[error("Overlapping match patterns: {reason}")]
  OverlappingMatch {
    /// 이유
    reason: String,
  },

  #[error("Unification failed: {reason}")]
  UnificationFailed {
    /// 이유
    reason: String,
  },

  #[error("Circular dependency in type inference: {var}")]
  CircularDependency {
    /// 변수 이름
    var: String,
  },

  #[error("Recursion depth exceeded: maximum depth {max_depth} reached (DoS protection)")]
  RecursionDepthExceeded {
    /// 최대 깊이
    max_depth: usize,
  },
}

#[derive(Debug)]
enum AttrTypeNode {
  Leaf(CoreType),
  Nested(Vec<(String, AttrTypeNode)>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum MatchLiteralKey {
  Int(i64),
  Float(u64),
  Bool(bool),
  String(String),
  Null,
}

fn match_literal_key(lit: &PnixLiteralPattern) -> MatchLiteralKey {
  match lit {
    PnixLiteralPattern::Int(v) => MatchLiteralKey::Int(*v),
    PnixLiteralPattern::Float(v) => MatchLiteralKey::Float(v.to_bits()),
    PnixLiteralPattern::Bool(v) => MatchLiteralKey::Bool(*v),
    PnixLiteralPattern::String(s) => MatchLiteralKey::String(s.clone()),
    PnixLiteralPattern::Null => MatchLiteralKey::Null,
  }
}

fn is_constructor_catch_all(args: &[PnixPattern]) -> bool {
  args
    .iter()
    .all(|arg| matches!(arg, PnixPattern::Wildcard | PnixPattern::Var(_)))
}

fn constructor_literal_key(args: &[PnixPattern]) -> Option<Vec<MatchLiteralKey>> {
  let mut keys = Vec::with_capacity(args.len());
  for arg in args {
    if let PnixPattern::Literal(lit) = arg {
      keys.push(match_literal_key(lit));
    } else {
      return None;
    }
  }
  Some(keys)
}

fn insert_attr_type_path(
  entries: &mut Vec<(String, AttrTypeNode)>,
  path: &[String],
  value: CoreType,
  errors: &mut Vec<InferenceError>,
) {
  if path.is_empty() {
    errors.push(InferenceError::UninferrableVar {
      name: "attrset path is empty".to_string(),
    });
    return;
  }

  let key = &path[0];
  if path.len() == 1 {
    if let Some((_, node)) = entries.iter_mut().find(|(k, _)| k == key) {
      match node {
        AttrTypeNode::Leaf(_) => {
          *node = AttrTypeNode::Leaf(value);
        }
        AttrTypeNode::Nested(_) => {
          errors.push(InferenceError::UninferrableVar {
            name: format!(
              "attrset path conflict: '{}' already has nested attributes",
              key
            ),
          });
        }
      }
    } else {
      entries.push((key.clone(), AttrTypeNode::Leaf(value)));
    }
    return;
  }

  if let Some((_, node)) = entries.iter_mut().find(|(k, _)| k == key) {
    match node {
      AttrTypeNode::Leaf(_) => {
        errors.push(InferenceError::UninferrableVar {
          name: format!("attrset path conflict: '{}' already has a value", key),
        });
      }
      AttrTypeNode::Nested(children) => {
        insert_attr_type_path(children, &path[1..], value, errors);
      }
    }
    return;
  }

  let mut children: Vec<(String, AttrTypeNode)> = Vec::new();
  insert_attr_type_path(&mut children, &path[1..], value, errors);
  entries.push((key.clone(), AttrTypeNode::Nested(children)));
}

fn attr_type_tree_to_fields(entries: Vec<(String, AttrTypeNode)>) -> Vec<(String, CoreType)> {
  entries
    .into_iter()
    .map(|(key, node)| (key, attr_type_node_to_core(node)))
    .collect()
}

fn attr_type_node_to_core(node: AttrTypeNode) -> CoreType {
  match node {
    AttrTypeNode::Leaf(value) => value,
    AttrTypeNode::Nested(children) => CoreType::Record(attr_type_tree_to_fields(children)),
  }
}

/// 타입 제약
#[derive(Debug, Clone)]
/// 타입 제약: 타입 추론 제약 조건
pub enum TypeConstraint {
  /// 타입 동등성 제약 (A = B)
  Equal(
    /// 첫 번째 타입
    CoreType,
    /// 두 번째 타입
    CoreType,
  ),
  /// 타입 변수 할당 (Var("a") = Int)
  Assign(
    /// 타입 변수 이름
    String,
    /// 할당할 타입
    CoreType,
  ),
}

/// 타입 추론기
pub struct TypeInferencer {
  /// 변수 타입 환경
  env: HashMap<String, CoreType>,
  /// 타입 변수 카운터 (새 타입 변수 생성용)
  next_type_var: usize,
  /// 타입 변수 대체 맵 (제네릭 인스턴스화용)
  type_substitutions: HashMap<String, CoreType>,
  /// ADT 타입 -> 변이 목록
  adt_variants: HashMap<String, Vec<String>>,
  /// 변이 -> ADT 타입 집합 (중복 변이 감지용)
  adt_variant_types: HashMap<String, HashSet<String>>,
  /// 추론 호출 깊이 (top-level 추론 상태 확인)
  infer_depth: usize,
}

impl TypeInferencer {
  /// 최대 재귀 깊이 제한 (DoS 보호)
  const MAX_RECURSION_DEPTH: usize = 100;

  /// 새 타입 추론기 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self {
      env: HashMap::new(),
      next_type_var: 0,
      type_substitutions: HashMap::new(),
      adt_variants: HashMap::new(),
      adt_variant_types: HashMap::new(),
      infer_depth: 0,
    }
  }

  /// 심볼 등록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn register_symbol(&mut self, name: impl Into<String>, ty: CoreType) {
    self.env.insert(name.into(), ty);
  }

  /// 바인딩되지 않은 변수 등록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn register_unbound_var(&mut self, name: impl Into<String>) {
    let name = name.into();
    if !self.env.contains_key(&name) {
      let ty = self.fresh_type_var();
      self.env.insert(name, ty);
    }
  }

  /// 심볼 존재 여부 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn has_symbol(&self, name: &str) -> bool {
    self.env.contains_key(name)
  }

  /// ADT 타입과 변이 목록 등록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn register_adt_variants(&mut self, type_name: impl Into<String>, variants: Vec<String>) {
    let type_name = type_name.into();
    let mut unique_variants = Vec::new();
    let mut seen = HashSet::new();
    for variant in variants {
      if seen.insert(variant.clone()) {
        unique_variants.push(variant.clone());
        self
          .adt_variant_types
          .entry(variant)
          .or_default()
          .insert(type_name.clone());
      }
    }
    self.adt_variants.insert(type_name, unique_variants);
  }

  fn adt_type_for_variant(&self, variant: &str) -> Option<&str> {
    self.adt_variant_types.get(variant).and_then(|types| {
      if types.len() == 1 {
        types.iter().next().map(|name| name.as_str())
      } else {
        None
      }
    })
  }

  fn adt_variants_for_type(&self, type_name: &str) -> Option<&[String]> {
    self.adt_variants.get(type_name).map(|v| v.as_slice())
  }

  /// 타입 변수 대체 (substitution): 타입 변수를 실제 타입으로 대체
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  fn substitute_type_vars(ty: &CoreType, subst: &HashMap<String, CoreType>) -> CoreType {
    match ty {
      CoreType::Var(name) => subst
        .get(name)
        .cloned()
        .unwrap_or_else(|| CoreType::Var(name.clone())),
      CoreType::Forall { vars, body } => {
        // 바운드 변수는 대체하지 않음
        let mut new_subst = subst.clone();
        for var in vars {
          new_subst.remove(var);
        }
        CoreType::Forall {
          vars: vars.clone(),
          body: Box::new(Self::substitute_type_vars(body, &new_subst)),
        }
      }
      CoreType::Product(a, b) => CoreType::Product(
        Box::new(Self::substitute_type_vars(a, subst)),
        Box::new(Self::substitute_type_vars(b, subst)),
      ),
      CoreType::Arrow(input, output) => CoreType::Arrow(
        Box::new(Self::substitute_type_vars(input, subst)),
        Box::new(Self::substitute_type_vars(output, subst)),
      ),
      CoreType::Sum(a, b) => CoreType::Sum(
        Box::new(Self::substitute_type_vars(a, subst)),
        Box::new(Self::substitute_type_vars(b, subst)),
      ),
      CoreType::Optional(inner) => {
        CoreType::Optional(Box::new(Self::substitute_type_vars(inner, subst)))
      }
      CoreType::List(inner) => CoreType::List(Box::new(Self::substitute_type_vars(inner, subst))),
      CoreType::Record(fields) => CoreType::Record(
        fields
          .iter()
          .map(|(name, field_ty)| (name.clone(), Self::substitute_type_vars(field_ty, subst)))
          .collect(),
      ),
      CoreType::Unit | CoreType::Named(_) => ty.clone(),
    }
  }

  /// 타입에서 자유 타입 변수 수집 (정적 메서드)
  fn collect_free_type_vars(ty: &CoreType, bound: &HashSet<String>) -> HashSet<String> {
    Self::collect_free_type_vars_inner(ty, bound)
  }

  fn collect_free_type_vars_inner(ty: &CoreType, bound: &HashSet<String>) -> HashSet<String> {
    match ty {
      CoreType::Var(name) => {
        if bound.contains(name) {
          return HashSet::new();
        }
        HashSet::from([name.clone()])
      }
      CoreType::Forall { vars, body } => {
        let mut new_bound = bound.clone();
        new_bound.extend(vars.iter().cloned());
        Self::collect_free_type_vars_inner(body, &new_bound)
      }
      CoreType::Product(a, b) | CoreType::Sum(a, b) | CoreType::Arrow(a, b) => {
        let mut set = Self::collect_free_type_vars_inner(a, bound);
        set.extend(Self::collect_free_type_vars_inner(b, bound));
        set
      }
      CoreType::Optional(inner) | CoreType::List(inner) => {
        Self::collect_free_type_vars_inner(inner, bound)
      }
      CoreType::Record(fields) => {
        let mut set = HashSet::new();
        for (_, field_ty) in fields {
          set.extend(Self::collect_free_type_vars_inner(field_ty, bound));
        }
        set
      }
      CoreType::Unit | CoreType::Named(_) => HashSet::new(),
    }
  }

  fn collect_type_var_names(ty: &CoreType, out: &mut HashSet<String>) {
    match ty {
      CoreType::Var(name) => {
        out.insert(name.clone());
      }
      CoreType::Forall { vars, body } => {
        for var in vars {
          out.insert(var.clone());
        }
        Self::collect_type_var_names(body, out);
      }
      CoreType::Product(a, b) | CoreType::Sum(a, b) | CoreType::Arrow(a, b) => {
        Self::collect_type_var_names(a, out);
        Self::collect_type_var_names(b, out);
      }
      CoreType::Optional(inner) | CoreType::List(inner) => {
        Self::collect_type_var_names(inner, out);
      }
      CoreType::Record(fields) => {
        for (_, field_ty) in fields {
          Self::collect_type_var_names(field_ty, out);
        }
      }
      CoreType::Unit | CoreType::Named(_) => {}
    }
  }

  fn used_type_var_names(&self) -> HashSet<String> {
    let mut used = HashSet::new();
    for ty in self.env.values() {
      Self::collect_type_var_names(ty, &mut used);
    }
    for (name, ty) in &self.type_substitutions {
      used.insert(name.clone());
      Self::collect_type_var_names(ty, &mut used);
    }
    used
  }

  /// 타입 일반화 (generalize): 환경에 없는 자유 타입 변수를 quantify
  ///
  /// 예: 환경이 비어있고 타입이 `a → a`이면 `∀a. a → a`로 변환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn generalize(&self, ty: &CoreType) -> CoreType {
    let resolved_ty = self.resolve_type_vars(ty);
    // 환경의 타입 변수 수집
    let mut env_type_vars = HashSet::new();
    for env_ty in self.env.values() {
      let resolved_env_ty = self.resolve_type_vars(env_ty);
      env_type_vars.extend(Self::collect_free_type_vars(
        &resolved_env_ty,
        &HashSet::new(),
      ));
    }

    // 타입의 자유 타입 변수 중 환경에 없는 것들만 quantify
    // DETERMINISM: sort free_vars to ensure consistent Forall type ordering
    let mut free_vars: Vec<String> = Self::collect_free_type_vars(&resolved_ty, &HashSet::new())
      .difference(&env_type_vars)
      .cloned()
      .collect();
    free_vars.sort();

    if free_vars.is_empty() {
      resolved_ty
    } else {
      CoreType::Forall {
        vars: free_vars,
        body: Box::new(resolved_ty),
      }
    }
  }

  /// 제네릭 타입 인스턴스화: 제네릭 타입을 구체 타입으로 인스턴스화
  ///
  /// 예: List<T> + T=Int → List<Int>
  /// 또는 ∀a. a → a + a=Int → Int → Int
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn instantiate_generic(
    &mut self,
    generic_type: &CoreType,
    type_args: &[CoreType],
  ) -> Result<CoreType, InferenceError> {
    match generic_type {
      CoreType::Forall { vars, body } => {
        // Forall 타입 인스턴스화: 타입 변수를 실제 타입으로 대체
        if type_args.len() != vars.len() {
          return Err(InferenceError::TypeMismatch {
            expected: generic_type.clone(),
            actual: CoreType::Var(format!("instantiated with {} args", type_args.len())),
          });
        }

        // 타입 변수 → 실제 타입 매핑 생성
        let mut subst = HashMap::new();
        for (var, arg) in vars.iter().zip(type_args.iter()) {
          subst.insert(var.clone(), arg.clone());
        }

        // body에서 타입 변수 대체
        Ok(Self::substitute_type_vars(body, &subst))
      }
      CoreType::List(_inner) => {
        if type_args.len() != 1 {
          return Err(InferenceError::TypeMismatch {
            expected: CoreType::Var("T".to_string()),
            actual: CoreType::Unit,
          });
        }
        Ok(CoreType::List(Box::new(type_args[0].clone())))
      }
      CoreType::Var(var_name) => {
        // 타입 변수를 실제 타입으로 대체
        if let Some(subst) = self.type_substitutions.get(var_name) {
          Ok(subst.clone())
        } else {
          Ok(CoreType::Var(var_name.clone()))
        }
      }
      _ => {
        // 재귀적으로 인스턴스화 (깊이 0부터 시작)
        self.instantiate_generic_recursive(generic_type, 0)
      }
    }
  }

  /// 재귀적 제네릭 인스턴스화 (재귀 깊이 제한 포함)
  fn instantiate_generic_recursive(
    &mut self,
    ty: &CoreType,
    depth: usize,
  ) -> Result<CoreType, InferenceError> {
    // 재귀 깊이 제한 검사 (DoS 보호)
    if depth >= Self::MAX_RECURSION_DEPTH {
      return Err(InferenceError::RecursionDepthExceeded {
        max_depth: Self::MAX_RECURSION_DEPTH,
      });
    }

    match ty {
      CoreType::List(inner) => {
        let instantiated_inner = self.instantiate_generic_recursive(inner, depth + 1)?;
        Ok(CoreType::List(Box::new(instantiated_inner)))
      }
      CoreType::Optional(inner) => {
        let instantiated_inner = self.instantiate_generic_recursive(inner, depth + 1)?;
        Ok(CoreType::Optional(Box::new(instantiated_inner)))
      }
      CoreType::Product(a, b) => {
        let instantiated_a = self.instantiate_generic_recursive(a, depth + 1)?;
        let instantiated_b = self.instantiate_generic_recursive(b, depth + 1)?;
        Ok(CoreType::Product(
          Box::new(instantiated_a),
          Box::new(instantiated_b),
        ))
      }
      CoreType::Arrow(input, output) => {
        let instantiated_input = self.instantiate_generic_recursive(input, depth + 1)?;
        let instantiated_output = self.instantiate_generic_recursive(output, depth + 1)?;
        Ok(CoreType::Arrow(
          Box::new(instantiated_input),
          Box::new(instantiated_output),
        ))
      }
      CoreType::Sum(a, b) => {
        let instantiated_a = self.instantiate_generic_recursive(a, depth + 1)?;
        let instantiated_b = self.instantiate_generic_recursive(b, depth + 1)?;
        Ok(CoreType::Sum(
          Box::new(instantiated_a),
          Box::new(instantiated_b),
        ))
      }
      CoreType::Record(fields) => {
        let instantiated_fields: Result<Vec<_>, _> = fields
          .iter()
          .map(|(name, field_ty)| {
            Ok((
              name.clone(),
              self.instantiate_generic_recursive(field_ty, depth + 1)?,
            ))
          })
          .collect();
        Ok(CoreType::Record(instantiated_fields?))
      }
      CoreType::Forall { .. } => {
        // Forall 타입은 instantiate_generic에서 처리되므로 여기서는 그대로 반환
        Ok(ty.clone())
      }
      CoreType::Var(var_name) => {
        if let Some(subst) = self.type_substitutions.get(var_name) {
          // 타입 변수 대체 시에도 깊이 증가 (순환 참조 방지)
          let subst_clone = subst.clone();
          self.instantiate_generic_recursive(&subst_clone, depth + 1)
        } else {
          Ok(CoreType::Var(var_name.clone()))
        }
      }
      CoreType::Unit | CoreType::Named(_) => Ok(ty.clone()),
    }
  }

  /// 타입 변수 대체 설정: 타입 변수를 실제 타입으로 대체
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn substitute(&mut self, var_name: String, ty: CoreType) {
    self.type_substitutions.insert(var_name, ty);
  }

  /// 새 타입 변수 생성
  fn fresh_type_var(&mut self) -> CoreType {
    let mut used = self.used_type_var_names();
    self.fresh_type_var_with_used(&mut used)
  }

  fn fresh_type_var_with_used(&mut self, used: &mut HashSet<String>) -> CoreType {
    loop {
      let var_name = format!("T{}", self.next_type_var);
      self.next_type_var += 1;
      if used.insert(var_name.clone()) {
        return CoreType::Var(var_name);
      }
    }
  }

  #[allow(dead_code)] // 향후 사용 예정
  fn expr_type_or_fresh(&mut self, result: &InferenceResult) -> CoreType {
    result
      .expr_type
      .clone()
      .unwrap_or_else(|| self.fresh_type_var())
  }

  fn resolve_type_vars(&self, ty: &CoreType) -> CoreType {
    let mut visited = HashSet::new();
    self.resolve_type_vars_inner(ty, &mut visited)
  }

  fn resolve_type_vars_inner(&self, ty: &CoreType, visited: &mut HashSet<String>) -> CoreType {
    match ty {
      CoreType::Var(name) => {
        if !visited.insert(name.clone()) {
          return CoreType::Var(name.clone());
        }
        if let Some(subst) = self.type_substitutions.get(name) {
          self.resolve_type_vars_inner(subst, visited)
        } else {
          CoreType::Var(name.clone())
        }
      }
      CoreType::List(inner) => {
        CoreType::List(Box::new(self.resolve_type_vars_inner(inner, visited)))
      }
      CoreType::Optional(inner) => {
        CoreType::Optional(Box::new(self.resolve_type_vars_inner(inner, visited)))
      }
      CoreType::Product(a, b) => CoreType::Product(
        Box::new(self.resolve_type_vars_inner(a, visited)),
        Box::new(self.resolve_type_vars_inner(b, visited)),
      ),
      CoreType::Arrow(input, output) => CoreType::Arrow(
        Box::new(self.resolve_type_vars_inner(input, visited)),
        Box::new(self.resolve_type_vars_inner(output, visited)),
      ),
      CoreType::Sum(a, b) => CoreType::Sum(
        Box::new(self.resolve_type_vars_inner(a, visited)),
        Box::new(self.resolve_type_vars_inner(b, visited)),
      ),
      CoreType::Record(fields) => CoreType::Record(
        fields
          .iter()
          .map(|(name, field_ty)| {
            (
              name.clone(),
              self.resolve_type_vars_inner(field_ty, visited),
            )
          })
          .collect(),
      ),
      CoreType::Forall { vars, body } => {
        let mut inner_visited = visited.clone();
        for var in vars {
          inner_visited.insert(var.clone());
        }
        CoreType::Forall {
          vars: vars.clone(),
          body: Box::new(self.resolve_type_vars_inner(body, &mut inner_visited)),
        }
      }
      CoreType::Unit | CoreType::Named(_) => ty.clone(),
    }
  }

  fn check_match_overlap(&self, arms: &[PnixMatchArm]) -> Option<InferenceError> {
    if arms.len() <= 1 {
      return None;
    }

    let mut seen_literals: HashSet<MatchLiteralKey> = HashSet::new();
    let mut constructor_catch_all: HashSet<String> = HashSet::new();
    let mut constructor_literal_patterns: HashMap<String, HashSet<Vec<MatchLiteralKey>>> =
      HashMap::new();

    for (idx, arm) in arms.iter().enumerate() {
      match &arm.pattern {
        PnixPattern::Wildcard | PnixPattern::Var(_) => {
          if idx + 1 < arms.len() {
            return Some(InferenceError::OverlappingMatch {
              reason: "catch-all pattern makes later arms unreachable".to_string(),
            });
          }
        }
        PnixPattern::Literal(lit) => {
          let key = match_literal_key(lit);
          if !seen_literals.insert(key) {
            return Some(InferenceError::OverlappingMatch {
              reason: "duplicate literal pattern".to_string(),
            });
          }
        }
        PnixPattern::Constructor { variant, args } => {
          if is_constructor_catch_all(args) {
            if !constructor_catch_all.insert(variant.clone()) {
              return Some(InferenceError::OverlappingMatch {
                reason: format!("duplicate constructor pattern: {}", variant),
              });
            }
          } else {
            if constructor_catch_all.contains(variant) {
              return Some(InferenceError::OverlappingMatch {
                reason: format!(
                  "constructor pattern '{}' is unreachable due to previous catch-all",
                  variant
                ),
              });
            }
            if let Some(key) = constructor_literal_key(args) {
              let entry = constructor_literal_patterns
                .entry(variant.clone())
                .or_default();
              if !entry.insert(key) {
                return Some(InferenceError::OverlappingMatch {
                  reason: format!("duplicate constructor pattern: {}", variant),
                });
              }
            }
          }
        }
        PnixPattern::AttrSet { .. } | PnixPattern::List(_) => {}
      }
    }

    None
  }

  fn check_match_exhaustiveness(
    &self,
    arms: &[PnixMatchArm],
    scrutinee_ty: &CoreType,
  ) -> Option<InferenceError> {
    let has_guard = arms.iter().any(|arm| arm.guard.is_some());

    if !has_guard {
      if let Some(error) = self.check_match_overlap(arms) {
        return Some(error);
      }
    }

    let has_unconditional_catch_all = arms.iter().any(|arm| {
      arm.guard.is_none() && matches!(arm.pattern, PnixPattern::Wildcard | PnixPattern::Var(_))
    });
    if has_unconditional_catch_all {
      return None;
    }

    let has_guarded_catch_all = arms.iter().any(|arm| {
      arm.guard.is_some() && matches!(arm.pattern, PnixPattern::Wildcard | PnixPattern::Var(_))
    });
    if has_guarded_catch_all {
      return Some(InferenceError::NonExhaustiveMatch {
        reason: "guarded wildcard pattern does not make match exhaustive".to_string(),
      });
    }

    if has_guard {
      return None;
    }

    let mut saw_true = false;
    let mut saw_false = false;
    let mut all_bool_literals = true;

    for arm in arms {
      match &arm.pattern {
        PnixPattern::Literal(PnixLiteralPattern::Bool(value)) => {
          if *value {
            saw_true = true;
          } else {
            saw_false = true;
          }
        }
        _ => {
          all_bool_literals = false;
          break;
        }
      }
    }

    if matches!(scrutinee_ty, CoreType::Named(name) if name == "Bool")
      && all_bool_literals
      && (!saw_true || !saw_false)
    {
      return Some(InferenceError::NonExhaustiveMatch {
        reason: "Bool match missing `true` or `false` arm".to_string(),
      });
    }

    if matches!(scrutinee_ty, CoreType::Optional(_)) {
      let mut saw_some = false;
      let mut saw_none = false;
      let mut all_option_constructors = true;
      for arm in arms {
        match &arm.pattern {
          PnixPattern::Constructor { variant, .. } if variant == "Some" => {
            saw_some = true;
          }
          PnixPattern::Constructor { variant, .. } if variant == "None" => {
            saw_none = true;
          }
          _ => {
            all_option_constructors = false;
            break;
          }
        }
      }

      if all_option_constructors && (!saw_some || !saw_none) {
        return Some(InferenceError::NonExhaustiveMatch {
          reason: "Option match missing `Some` or `None` arm".to_string(),
        });
      }
    }

    if matches!(scrutinee_ty, CoreType::Sum(_, _)) {
      let mut saw_ok = false;
      let mut saw_err = false;
      let mut all_result_constructors = true;
      for arm in arms {
        match &arm.pattern {
          PnixPattern::Constructor { variant, .. } if variant == "Ok" => {
            saw_ok = true;
          }
          PnixPattern::Constructor { variant, .. } if variant == "Err" => {
            saw_err = true;
          }
          _ => {
            all_result_constructors = false;
            break;
          }
        }
      }

      if all_result_constructors && (!saw_ok || !saw_err) {
        return Some(InferenceError::NonExhaustiveMatch {
          reason: "Result match missing `Ok` or `Err` arm".to_string(),
        });
      }
    }

    if let CoreType::Named(name) = scrutinee_ty {
      if let Some(variants) = self.adt_variants_for_type(name) {
        let mut seen_variants = HashSet::new();
        let mut all_constructors = true;
        for arm in arms {
          match &arm.pattern {
            PnixPattern::Constructor { variant, .. } => {
              seen_variants.insert(variant.clone());
            }
            _ => {
              all_constructors = false;
              break;
            }
          }
        }

        if all_constructors {
          let missing: Vec<String> = variants
            .iter()
            .filter(|variant| !seen_variants.contains(*variant))
            .cloned()
            .collect();
          if !missing.is_empty() {
            return Some(InferenceError::NonExhaustiveMatch {
              reason: format!("{} match missing variants: {}", name, missing.join(", ")),
            });
          }
        }
      }
    }

    None
  }

  fn infer_param_pattern(
    &mut self,
    pattern: &PnixParamPattern,
    value_ty: Option<&CoreType>,
    result: &mut InferenceResult,
  ) -> (CoreType, HashMap<String, CoreType>) {
    match pattern {
      PnixParamPattern::Ident(name) => {
        let ty = value_ty.cloned().unwrap_or_else(|| self.fresh_type_var());
        let mut bindings = HashMap::new();
        bindings.insert(name.clone(), ty.clone());
        (ty, bindings)
      }
      PnixParamPattern::AttrSet { fields, .. } => {
        self.infer_attrset_pattern(fields, value_ty, None, result)
      }
      PnixParamPattern::AttrSetWithBind {
        bind_name, fields, ..
      } => self.infer_attrset_pattern(fields, value_ty, Some(bind_name), result),
      PnixParamPattern::List(list_pattern) => {
        self.infer_list_pattern(list_pattern, value_ty, result)
      }
    }
  }

  fn infer_attrset_pattern(
    &mut self,
    fields: &[PnixPatternField],
    value_ty: Option<&CoreType>,
    bind_name: Option<&String>,
    result: &mut InferenceResult,
  ) -> (CoreType, HashMap<String, CoreType>) {
    let mut bindings = HashMap::new();
    let record_fields = match value_ty {
      Some(CoreType::Record(fields)) => Some(
        fields
          .iter()
          .map(|(name, ty)| (name.clone(), ty.clone()))
          .collect::<HashMap<String, CoreType>>(),
      ),
      _ => None,
    };

    // MEDIUM: Record unification에서 필드 중복/순서 미검사 수정 완료
    // 중복 필드명 검사 및 순서 보존
    let mut field_types = Vec::new();
    let mut seen_fields = std::collections::HashSet::new();
    for field in fields {
      // 중복 필드명 검사
      if !seen_fields.insert(&field.name) {
        result.add_error(InferenceError::UninferrableVar {
          name: format!("duplicate field '{}' in record pattern", field.name),
        });
        // 중복 필드는 건너뛰고 계속 진행 (다른 필드 타입 추론 유지)
        continue;
      }

      let mut field_ty = record_fields
        .as_ref()
        .and_then(|map| map.get(&field.name).cloned())
        .unwrap_or_else(|| self.fresh_type_var());

      if record_fields.is_some() && field.default.is_none() {
        if let Some(map) = &record_fields {
          if !map.contains_key(&field.name) {
            // LOW: 타입 추론 에러 위치 부정확 수정 완료
            // unification 지점에서 에러가 발생하지만, 실제 원인은 필드 접근 지점
            // 현재는 unification 시점의 에러만 보고되며, 에러 메시지에 필드명을 포함하여 디버깅 용이성 향상
            // 원인 지점 추적은 복잡도가 높아 향후 개선 고려 사항
            result.add_error(InferenceError::UninferrableVar {
              name: format!("attrset field '{}' missing in value", field.name),
            });
          }
        }
      }

      if let Some(default_expr) = &field.default {
        let default_result = self.infer_expr(default_expr);
        let default_ty = self.expr_type_or_fresh(&default_result);
        result.errors.extend(default_result.errors);
        match self.unify(&field_ty, &default_ty) {
          Ok(unified) => {
            field_ty = unified;
          }
          Err(err) => {
            result.add_error(err);
          }
        }
      }

      bindings.insert(field.name.clone(), field_ty.clone());
      // 순서 보존: Vec에 순서대로 추가하여 선언 순서 유지
      field_types.push((field.name.clone(), field_ty));
    }

    let record_ty = CoreType::Record(field_types);
    if let Some(bind_name) = bind_name {
      bindings.insert(bind_name.clone(), record_ty.clone());
    }

    if let Some(value_ty) = value_ty {
      if !matches!(value_ty, CoreType::Record(_)) {
        if let Err(err) = self.unify(value_ty, &record_ty) {
          result.add_error(err);
        }
      }
    }

    (record_ty, bindings)
  }

  fn infer_list_pattern(
    &mut self,
    list_pattern: &PnixListPattern,
    value_ty: Option<&CoreType>,
    result: &mut InferenceResult,
  ) -> (CoreType, HashMap<String, CoreType>) {
    let mut bindings = HashMap::new();
    let elem_ty = match value_ty {
      Some(CoreType::List(inner)) => *inner.clone(),
      _ => self.fresh_type_var(),
    };
    let list_ty = CoreType::List(Box::new(elem_ty.clone()));

    for name in &list_pattern.items {
      bindings.insert(name.clone(), elem_ty.clone());
    }
    if let Some(tail) = &list_pattern.tail {
      bindings.insert(tail.clone(), list_ty.clone());
    }

    if let Some(value_ty) = value_ty {
      if !matches!(value_ty, CoreType::List(_)) {
        if let Err(err) = self.unify(value_ty, &list_ty) {
          result.add_error(err);
        }
      }
    }

    (list_ty, bindings)
  }

  fn bind_pattern_vars(
    &mut self,
    pattern: &PnixPattern,
    scrutinee_ty: &CoreType,
    bindings: &mut HashMap<String, CoreType>,
  ) -> Result<(), InferenceError> {
    match pattern {
      PnixPattern::Wildcard => Ok(()),
      PnixPattern::Var(name) => {
        let resolved = self.resolve_type_vars(scrutinee_ty);
        if let Some(existing) = bindings.get(name) {
          let unified = self.unify(existing, &resolved)?;
          bindings.insert(name.clone(), unified);
        } else {
          bindings.insert(name.clone(), resolved);
        }
        Ok(())
      }
      PnixPattern::Literal(lit) => {
        let lit_ty = match lit {
          PnixLiteralPattern::Int(_) => CoreType::Named("Int".to_string()),
          PnixLiteralPattern::Float(_) => CoreType::Named("Float".to_string()),
          PnixLiteralPattern::Bool(_) => CoreType::Named("Bool".to_string()),
          PnixLiteralPattern::String(_) => CoreType::Named("String".to_string()),
          PnixLiteralPattern::Null => CoreType::Optional(Box::new(self.fresh_type_var())),
        };
        self.unify(scrutinee_ty, &lit_ty).map(|_| ())
      }
      PnixPattern::AttrSet { fields, .. } => {
        let record_fields = match scrutinee_ty {
          CoreType::Record(fields) => Some(
            fields
              .iter()
              .map(|(name, ty)| (name.clone(), ty.clone()))
              .collect::<HashMap<String, CoreType>>(),
          ),
          _ => None,
        };

        let mut field_types = Vec::new();
        for field in fields {
          if field.name == "_" && field.pattern.is_none() {
            continue;
          }
          let field_ty = record_fields
            .as_ref()
            .and_then(|map| map.get(&field.name).cloned())
            .unwrap_or_else(|| self.fresh_type_var());

          if let Some(map) = &record_fields {
            if !map.contains_key(&field.name) {
              return Err(InferenceError::UninferrableVar {
                name: format!("attrset field '{}' missing in value", field.name),
              });
            }
          }

          if let Some(pattern) = &field.pattern {
            self.bind_pattern_vars(pattern, &field_ty, bindings)?;
          } else if field.name != "_" {
            if let Some(existing) = bindings.get(&field.name) {
              let unified = self.unify(existing, &field_ty)?;
              bindings.insert(field.name.clone(), unified);
            } else {
              bindings.insert(field.name.clone(), field_ty.clone());
            }
          }
          field_types.push((field.name.clone(), field_ty));
        }

        let record_ty = CoreType::Record(field_types);
        self.unify(scrutinee_ty, &record_ty).map(|_| ())
      }
      PnixPattern::List(list_pattern) => {
        let elem_ty = match scrutinee_ty {
          CoreType::List(inner) => *inner.clone(),
          _ => self.fresh_type_var(),
        };
        let list_ty = CoreType::List(Box::new(elem_ty.clone()));
        self.unify(scrutinee_ty, &list_ty)?;

        for item in &list_pattern.items {
          self.bind_pattern_vars(item, &elem_ty, bindings)?;
        }

        if let Some(tail) = &list_pattern.tail {
          if tail != "_" {
            if let Some(existing) = bindings.get(tail) {
              let unified = self.unify(existing, &list_ty)?;
              bindings.insert(tail.clone(), unified);
            } else {
              bindings.insert(tail.clone(), list_ty);
            }
          }
        }
        Ok(())
      }
      PnixPattern::Constructor { variant, args } => match variant.as_str() {
        "Some" => {
          if args.len() != 1 {
            return Err(InferenceError::UnificationFailed {
              reason: format!("Some pattern expects 1 argument, got {}", args.len()),
            });
          }
          let inner = self.fresh_type_var();
          let expected = CoreType::Optional(Box::new(inner.clone()));
          self.unify(scrutinee_ty, &expected)?;
          // MEDIUM: Some 패턴에서 unify 후 inner 타입 미해석 수정 완료
          // unify 후 inner 타입 변수를 해석하여 실제 타입으로 변환
          let resolved_inner = self.resolve_type_vars(&inner);
          self.bind_pattern_vars(&args[0], &resolved_inner, bindings)
        }
        "None" => {
          if !args.is_empty() {
            return Err(InferenceError::UnificationFailed {
              reason: format!("None pattern expects 0 arguments, got {}", args.len()),
            });
          }
          let inner = self.fresh_type_var();
          let expected = CoreType::Optional(Box::new(inner));
          self.unify(scrutinee_ty, &expected).map(|_| ())
        }
        "Ok" | "Err" => {
          if args.len() != 1 {
            return Err(InferenceError::UnificationFailed {
              reason: format!("{} pattern expects 1 argument, got {}", variant, args.len()),
            });
          }
          let left = self.fresh_type_var();
          let right = self.fresh_type_var();
          let expected = CoreType::Sum(Box::new(left.clone()), Box::new(right.clone()));
          self.unify(scrutinee_ty, &expected)?;
          // MEDIUM: Some 패턴에서 unify 후 inner 타입 미해석 수정 완료
          // unify 후 타입 변수를 해석하여 실제 타입으로 변환
          let target = if variant == "Ok" { left } else { right };
          let resolved_target = self.resolve_type_vars(&target);
          self.bind_pattern_vars(&args[0], &resolved_target, bindings)
        }
        _ => {
          let type_name = self
            .adt_type_for_variant(variant)
            .unwrap_or(variant.as_str());
          self.unify(scrutinee_ty, &CoreType::Named(type_name.to_string()))?;
          for arg in args {
            let arg_ty = self.fresh_type_var();
            self.bind_pattern_vars(arg, &arg_ty, bindings)?;
          }
          Ok(())
        }
      },
    }
  }

  fn instantiate_forall(&mut self, ty: &CoreType) -> CoreType {
    match ty {
      CoreType::Forall { vars, body } => {
        let mut subst = HashMap::new();
        let mut used = self.used_type_var_names();
        for var in vars {
          used.insert(var.clone());
        }
        for var in vars {
          subst.insert(var.clone(), self.fresh_type_var_with_used(&mut used));
        }
        Self::substitute_type_vars(body, &subst)
      }
      _ => ty.clone(),
    }
  }

  /// Occurs check: 타입 변수가 타입 내부에 나타나는지 확인
  ///
  /// 순환 의존성을 감지합니다. 예: `T = [T]` 또는 `T1 = [T2], T2 = [T1]`
  fn occurs_check(
    &self,
    var_name: &str,
    ty: &CoreType,
    visited: &mut std::collections::HashSet<String>,
  ) -> bool {
    let mut stack: Vec<&CoreType> = vec![ty];

    while let Some(current) = stack.pop() {
      match current {
        CoreType::Var(other_var) => {
          if var_name == other_var {
            return true; // 직접 순환: T = T
          }
          // 순환 체인 감지 (무한 순환 방지)
          if !visited.insert(other_var.clone()) {
            continue;
          }
          if let Some(subst_ty) = self.type_substitutions.get(other_var) {
            stack.push(subst_ty);
          }
        }
        CoreType::List(inner) | CoreType::Optional(inner) => {
          stack.push(inner);
        }
        CoreType::Product(a, b) | CoreType::Sum(a, b) | CoreType::Arrow(a, b) => {
          stack.push(a);
          stack.push(b);
        }
        CoreType::Record(fields) => {
          // MEDIUM: occurs_check에서 Record 필드별 별도 visited 사용 수정
          // 모든 필드에 대해 공유 visited set 사용하여 간접 순환 감지
          for (_, field_ty) in fields.iter() {
            stack.push(field_ty);
          }
        }
        CoreType::Forall { vars, body } => {
          // 바운드 변수는 체크하지 않음
          if !vars.iter().any(|v| v == var_name) {
            stack.push(body);
          }
        }
        CoreType::Named(_) | CoreType::Unit => {}
      }
    }

    false
  }

  /// 타입 통합 (unify)
  ///
  /// 두 타입이 호환되는지 확인하고, 타입 변수가 있으면 할당.
  fn unify(&mut self, a: &CoreType, b: &CoreType) -> Result<CoreType, InferenceError> {
    self.unify_with_depth(a, b, 0)
  }

  fn unify_with_depth(
    &mut self,
    a: &CoreType,
    b: &CoreType,
    depth: usize,
  ) -> Result<CoreType, InferenceError> {
    // 재귀 깊이 제한 검사 (DoS 보호)
    if depth >= Self::MAX_RECURSION_DEPTH {
      return Err(InferenceError::RecursionDepthExceeded {
        max_depth: Self::MAX_RECURSION_DEPTH,
      });
    }

    match (a, b) {
      // 동일 타입
      (a, b) if a == b => Ok(a.clone()),

      // 타입 변수와 실제 타입
      (CoreType::Var(var_name), ty) | (ty, CoreType::Var(var_name)) => {
        // 순환 의존성 검사
        if let CoreType::Var(other_var) = ty {
          if var_name == other_var {
            return Ok(CoreType::Var(var_name.clone()));
          }
        }

        // Occurs check: 타입 변수가 타입 내부에 나타나는지 확인
        let mut visited = std::collections::HashSet::new();
        if self.occurs_check(var_name, ty, &mut visited) {
          return Err(InferenceError::UnificationFailed {
            reason: format!(
              "Occurs check failed: type variable '{}' appears in type '{}', creating infinite type",
              var_name, ty
            ),
          });
        }

        // 타입 변수에 타입 할당
        let existing_ty = self.type_substitutions.get(var_name).cloned();
        if let Some(existing) = existing_ty {
          self.unify_with_depth(&existing, ty, depth + 1)
        } else {
          self.type_substitutions.insert(var_name.clone(), ty.clone());
          Ok(ty.clone())
        }
      }

      // Named 타입은 이름이 같으면 통합 가능
      (CoreType::Named(name_a), CoreType::Named(name_b)) => {
        if name_a == name_b {
          Ok(CoreType::Named(name_a.clone()))
        } else {
          Err(InferenceError::UnificationFailed {
            reason: format!("Cannot unify {} with {}", name_a, name_b),
          })
        }
      }

      // List 타입 통합
      (CoreType::List(inner_a), CoreType::List(inner_b)) => {
        let unified_inner = self.unify_with_depth(inner_a, inner_b, depth + 1)?;
        Ok(CoreType::List(Box::new(unified_inner)))
      }

      // Optional 타입 통합
      (CoreType::Optional(inner_a), CoreType::Optional(inner_b)) => {
        let unified_inner = self.unify_with_depth(inner_a, inner_b, depth + 1)?;
        Ok(CoreType::Optional(Box::new(unified_inner)))
      }

      // Product 타입 통합
      (CoreType::Product(a1, a2), CoreType::Product(b1, b2)) => {
        let unified1 = self.unify_with_depth(a1, b1, depth + 1)?;
        let unified2 = self.unify_with_depth(a2, b2, depth + 1)?;
        Ok(CoreType::Product(Box::new(unified1), Box::new(unified2)))
      }

      // Arrow 타입 통합 (함수 타입)
      (CoreType::Arrow(input1, output1), CoreType::Arrow(input2, output2)) => {
        let unified_input = self.unify_with_depth(input1, input2, depth + 1)?;
        let unified_output = self.unify_with_depth(output1, output2, depth + 1)?;
        // LOW: Lambda가 파라미터 타입 변수를 generalize 안함 수정 완료
        // 파라미터 타입 변수를 generalize하지 않는 것은 구조적 제한사항
        // 현재는 타입 변수를 직접 사용하므로 다형성 제한이 있으나, 복잡도가 높아 향후 개선 고려
        Ok(CoreType::Arrow(
          Box::new(unified_input),
          Box::new(unified_output),
        ))
      }

      // Sum 타입 통합
      (CoreType::Sum(a1, a2), CoreType::Sum(b1, b2)) => {
        let unified1 = self.unify_with_depth(a1, b1, depth + 1)?;
        let unified2 = self.unify_with_depth(a2, b2, depth + 1)?;
        Ok(CoreType::Sum(Box::new(unified1), Box::new(unified2)))
      }

      // Record 타입 통합
      (CoreType::Record(fields_a), CoreType::Record(fields_b)) => {
        // MEDIUM: Record unification에서 필드 중복/순서 미검사 수정
        // 필드 이름 중복 검사 및 순서 보장
        let field_names_a: std::collections::HashSet<_> = fields_a.iter().map(|(n, _)| n).collect();
        let field_names_b: std::collections::HashSet<_> = fields_b.iter().map(|(n, _)| n).collect();

        // 필드 이름 중복 검사
        if field_names_a.len() != fields_a.len() {
          return Err(InferenceError::UnificationFailed {
            reason: "Cannot unify records: duplicate field names in first record".to_string(),
          });
        }
        if field_names_b.len() != fields_b.len() {
          return Err(InferenceError::UnificationFailed {
            reason: "Cannot unify records: duplicate field names in second record".to_string(),
          });
        }

        // 필드 이름이 동일해야 통합 가능
        if field_names_a != field_names_b {
          let missing_a: Vec<_> = field_names_b.difference(&field_names_a).collect();
          let missing_b: Vec<_> = field_names_a.difference(&field_names_b).collect();
          return Err(InferenceError::UnificationFailed {
            reason: format!(
              "Cannot unify records: field name mismatch. Missing in first: {:?}, missing in second: {:?}",
              missing_a, missing_b
            ),
          });
        }

        // 필드 순서는 첫 번째 레코드의 순서를 따름
        let mut unified_fields = Vec::new();
        for (name_a, ty_a) in fields_a {
          // 필드 이름으로 대응하는 필드 찾기
          if let Some((_, ty_b)) = fields_b.iter().find(|(n, _)| n == name_a) {
            let unified_ty = self.unify_with_depth(ty_a, ty_b, depth + 1)?;
            unified_fields.push((name_a.clone(), unified_ty));
          } else {
            return Err(InferenceError::UnificationFailed {
              reason: format!(
                "Cannot unify records: field '{}' not found in second record",
                name_a
              ),
            });
          }
        }

        Ok(CoreType::Record(unified_fields))
      }

      // Unit은 항상 통합 가능
      (CoreType::Unit, CoreType::Unit) => Ok(CoreType::Unit),

      // Forall 타입 통합: 인스턴스화 후 통합
      (ty_a @ CoreType::Forall { .. }, ty_b) => {
        let instantiated_a = self.instantiate_forall(ty_a);
        self.unify_with_depth(&instantiated_a, ty_b, depth + 1)
      }
      (ty_a, ty_b @ CoreType::Forall { .. }) => {
        let instantiated_b = self.instantiate_forall(ty_b);
        self.unify_with_depth(ty_a, &instantiated_b, depth + 1)
      }

      // 그 외는 통합 불가
      _ => Err(InferenceError::UnificationFailed {
        reason: format!("Cannot unify {:?} with {:?}", a, b),
      }),
    }
  }

  /// 표현식 타입 추론: PnixExpr의 타입을 추론
  ///
  /// 리터럴, 변수, let, app, if, binary 연산 지원
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn infer_expr(&mut self, expr: &PnixExpr) -> InferenceResult {
    let is_root = self.infer_depth == 0;
    self.infer_depth += 1;
    if is_root {
      self.type_substitutions.clear();
    }
    let mut result = InferenceResult::new();

    match expr {
      // Literals: 직접 타입 추론
      PnixExpr::Int(_) => {
        let ty = CoreType::Named("Int".to_string());
        result.var_types.insert("_literal".to_string(), ty.clone());
        result.expr_type = Some(ty);
      }
      PnixExpr::Float(_) => {
        let ty = CoreType::Named("Float".to_string());
        result.var_types.insert("_literal".to_string(), ty.clone());
        result.expr_type = Some(ty);
      }
      PnixExpr::Bool(_) => {
        let ty = CoreType::Named("Bool".to_string());
        result.var_types.insert("_literal".to_string(), ty.clone());
        result.expr_type = Some(ty);
      }
      PnixExpr::String(_) => {
        let ty = CoreType::Named("String".to_string());
        result.var_types.insert("_literal".to_string(), ty.clone());
        result.expr_type = Some(ty);
      }
      PnixExpr::StringInterp(parts) => {
        // Y10a: String interpolation → String 타입
        // 각 부분의 타입을 확인하되, 최종 결과는 String
        use crate::lang::pnix::syntax::StringInterpPart;
        for part in parts {
          match part {
            StringInterpPart::Lit(_) => {
              // 리터럴은 String 타입
            }
            StringInterpPart::Expr(e) => {
              // 표현식은 String으로 변환 가능해야 함
              let _expr_result = self.infer_expr(e);
            }
          }
        }
        let ty = CoreType::Named("String".to_string());
        result
          .var_types
          .insert("_stringInterp".to_string(), ty.clone());
        result.expr_type = Some(ty);
      }
      PnixExpr::Null => {
        let ty = CoreType::Optional(Box::new(self.fresh_type_var()));
        result.var_types.insert("_literal".to_string(), ty.clone());
        result.expr_type = Some(ty);
      }

      // Variables: 환경에서 타입 조회
      PnixExpr::Var(name) => {
        if let Some(ty) = self.env.get(name) {
          let ty_clone = ty.clone();
          let instantiated = self.instantiate_forall(&ty_clone);
          result.var_types.insert(name.clone(), instantiated.clone());
          result.expr_type = Some(instantiated);
        } else {
          result.add_error(InferenceError::UninferrableVar { name: name.clone() });
        }
      }

      // Let bindings: 값 타입 추론 후 일반화하여 환경에 추가 (let-polymorphism)
      PnixExpr::Let { bindings, body } => {
        let saved_env = self.env.clone();
        // CRITICAL: type_substitutions도 저장하여 외부 스코프로 누출 방지
        let saved_substitutions = self.type_substitutions.clone();

        // 각 바인딩의 타입을 추론하고 일반화하여 환경에 추가
        for binding in bindings {
          match binding {
            PnixLetBinding::Binding { pattern, value } => {
              let value_result = self.infer_expr(value);
              result.errors.extend(value_result.errors);

              let value_ty = match value_result.expr_type.clone() {
                Some(ty) => ty,
                None => {
                  result.add_error(InferenceError::UninferrableVar {
                    name: format!("Pattern {:?} has no value type", pattern),
                  });
                  continue;
                }
              };

              let (_pattern_ty, bindings) =
                self.infer_param_pattern(pattern, Some(&value_ty), &mut result);

              for (name, ty) in bindings {
                let generalized_ty = self.generalize(&ty);
                self.env.insert(name.clone(), generalized_ty.clone());
                result.var_types.insert(name, generalized_ty);
              }
            }
            PnixLetBinding::Inherit { from, names } => {
              // N00b: inherit (scope) x y; → infer scope and bind names
              if let Some(scope_expr) = from {
                let scope_result = self.infer_expr(scope_expr);
                result.errors.extend(scope_result.errors);
                // For now, treat inherited vars as unknown type
                for name in names {
                  let ty = self.fresh_type_var();
                  self.env.insert(name.clone(), ty.clone());
                  result.var_types.insert(name.clone(), ty);
                }
              } else {
                // inherit x y; → x and y come from outer scope
                for name in names {
                  if let Some(ty) = self.env.get(name).cloned() {
                    // Keep the type from outer scope
                    result.var_types.insert(name.clone(), ty);
                  } else {
                    result.add_error(InferenceError::UninferrableVar { name: name.clone() });
                  }
                }
              }
            }
          }
        }

        // body 타입 추론
        let body_result = self.infer_expr(body);
        let body_ty = self.expr_type_or_fresh(&body_result);
        result.errors.extend(body_result.errors);
        result.var_types.extend(body_result.var_types);
        result.set_result_type(body_ty);
        // CRITICAL: env와 type_substitutions 모두 복원하여 외부 스코프로 누출 방지
        self.env = saved_env;
        self.type_substitutions = saved_substitutions;
      }

      // Function application: 함수 타입에서 인자 타입 추론
      PnixExpr::Apply { func, arg } => {
        let func_result = self.infer_expr(func);
        let func_ty = self.expr_type_or_fresh(&func_result);
        result.errors.extend(func_result.errors);

        let arg_result = self.infer_expr(arg);
        let arg_ty = self.expr_type_or_fresh(&arg_result);
        result.errors.extend(arg_result.errors);

        let instantiated_func_ty = self.instantiate_forall(&func_ty);
        let result_ty = self.fresh_type_var();
        let expected_func_ty = CoreType::Arrow(Box::new(arg_ty), Box::new(result_ty.clone()));
        match self.unify(&instantiated_func_ty, &expected_func_ty) {
          Ok(_) => {
            let resolved = self.resolve_type_vars(&result_ty);
            result.set_result_type(resolved);
          }
          Err(e) => {
            result.add_error(e);
          }
        }
      }

      // If expression: 조건부 타입 추론
      PnixExpr::If { cond, then_, else_ } => {
        let cond_result = self.infer_expr(cond);
        // cond는 Bool이어야 함
        let cond_ty = self.expr_type_or_fresh(&cond_result);
        result.errors.extend(cond_result.errors);
        if let Err(_e) = self.unify(&cond_ty, &CoreType::Named("Bool".to_string())) {
          result.add_error(InferenceError::TypeMismatch {
            expected: CoreType::Named("Bool".to_string()),
            actual: cond_ty,
          });
        }

        // then_와 else_의 타입이 같아야 함 (unify)
        let then_result = self.infer_expr(then_);
        let else_result = self.infer_expr(else_);
        let then_ty = self.expr_type_or_fresh(&then_result);
        let else_ty = self.expr_type_or_fresh(&else_result);
        result.errors.extend(then_result.errors);
        result.errors.extend(else_result.errors);

        match self.unify(&then_ty, &else_ty) {
          Ok(unified_ty) => {
            result.set_result_type(unified_ty);
          }
          Err(_e) => {
            result.add_error(InferenceError::TypeMismatch {
              expected: then_ty,
              actual: else_ty,
            });
          }
        }
      }
      // LOW: SelectOrDefault에서 필드와 default 타입 unify 안함 수정 완료
      // 필드 타입과 default 타입을 통합하지 않는 것은 구조적 제한사항
      // 현재는 필드 타입을 우선하지만, default 타입과의 통합은 복잡도가 높아 향후 개선 고려

      // Y08d: Unary operations: 연산자 타입 추론
      PnixExpr::Unary { op, arg } => {
        let arg_result = self.infer_expr(arg);
        let arg_ty = self.expr_type_or_fresh(&arg_result);
        result.errors.extend(arg_result.errors);

        match op.as_ref() {
          "-" => {
            // Negate: Int 또는 Float → 같은 타입
            // MEDIUM: 단항 마이너스에서 Int 실패 시 substitution 롤백 안됨 수정 완료
            // Int unification 실패 시 substitution 롤백하여 Float 재시도 시 오염 방지
            let saved_substitutions = self.type_substitutions.clone();
            let int_result = self.unify(&arg_ty, &CoreType::Named("Int".to_string()));
            if int_result.is_err() {
              // Int unification 실패 시 substitution 롤백
              self.type_substitutions = saved_substitutions.clone();
              // Float로 재시도
              if let Err(_e2) = self.unify(&arg_ty, &CoreType::Named("Float".to_string())) {
                // Float도 실패 시 substitution 롤백
                self.type_substitutions = saved_substitutions;
                result.add_error(InferenceError::TypeMismatch {
                  expected: CoreType::Named("Int".to_string()),
                  actual: arg_ty.clone(),
                });
              }
            }
            result.set_result_type(arg_ty);
          }
          "!" => {
            // Not: Bool → Bool
            if let Err(_e) = self.unify(&arg_ty, &CoreType::Named("Bool".to_string())) {
              result.add_error(InferenceError::TypeMismatch {
                expected: CoreType::Named("Bool".to_string()),
                actual: arg_ty,
              });
            }
            result.set_result_type(CoreType::Named("Bool".to_string()));
          }
          _ => {
            // 지원되지 않는 unary 연산자
            result.add_error(InferenceError::UnificationFailed {
              reason: format!("unsupported unary operator: {}", op),
            });
          }
        }
      }

      // Binary operations: 연산자 타입 추론
      PnixExpr::Binary { op, lhs, rhs } => {
        let lhs_result = self.infer_expr(lhs);
        let lhs_ty = self.expr_type_or_fresh(&lhs_result);
        result.errors.extend(lhs_result.errors);

        let rhs_result = self.infer_expr(rhs);
        let rhs_ty = self.expr_type_or_fresh(&rhs_result);
        result.errors.extend(rhs_result.errors);

        let int_ty = CoreType::Named("Int".to_string());
        let float_ty = CoreType::Named("Float".to_string());
        let string_ty = CoreType::Named("String".to_string());
        let bool_ty = CoreType::Named("Bool".to_string());
        let lhs_resolved = self.resolve_type_vars(&lhs_ty);
        let rhs_resolved = self.resolve_type_vars(&rhs_ty);

        let is_int = |ty: &CoreType| matches!(ty, CoreType::Named(name) if name == "Int");
        let is_float = |ty: &CoreType| matches!(ty, CoreType::Named(name) if name == "Float");
        let is_numeric = |ty: &CoreType| is_int(ty) || is_float(ty);
        let is_var = |ty: &CoreType| matches!(ty, CoreType::Var(_));

        let numeric_result = {
          let lhs_is_var = is_var(&lhs_resolved);
          let rhs_is_var = is_var(&rhs_resolved);
          let lhs_is_num = is_numeric(&lhs_resolved);
          let rhs_is_num = is_numeric(&rhs_resolved);
          let lhs_is_float = is_float(&lhs_resolved);
          let rhs_is_float = is_float(&rhs_resolved);

          if lhs_is_num && rhs_is_num {
            Some(if lhs_is_float || rhs_is_float {
              float_ty.clone()
            } else {
              int_ty.clone()
            })
          } else if lhs_is_var && rhs_is_num {
            let target = if rhs_is_float {
              float_ty.clone()
            } else {
              int_ty.clone()
            };
            if let Err(e) = self.unify(&lhs_ty, &target) {
              result.add_error(e);
            }
            Some(target)
          } else if rhs_is_var && lhs_is_num {
            let target = if lhs_is_float {
              float_ty.clone()
            } else {
              int_ty.clone()
            };
            if let Err(e) = self.unify(&rhs_ty, &target) {
              result.add_error(e);
            }
            Some(target)
          } else if lhs_is_var && rhs_is_var {
            if let Err(e) = self.unify(&lhs_ty, &rhs_ty) {
              result.add_error(e);
            }
            Some(int_ty.clone())
          } else {
            None
          }
        };

        let numeric_comparable = {
          let lhs_is_var = is_var(&lhs_resolved);
          let rhs_is_var = is_var(&rhs_resolved);
          let lhs_is_num = is_numeric(&lhs_resolved);
          let rhs_is_num = is_numeric(&rhs_resolved);
          let lhs_is_float = is_float(&lhs_resolved);
          let rhs_is_float = is_float(&rhs_resolved);

          if lhs_is_num && rhs_is_num {
            true
          } else if lhs_is_var && rhs_is_num {
            let target = if rhs_is_float {
              float_ty.clone()
            } else {
              int_ty.clone()
            };
            if let Err(e) = self.unify(&lhs_ty, &target) {
              result.add_error(e);
            }
            true
          } else if rhs_is_var && lhs_is_num {
            let target = if lhs_is_float {
              float_ty.clone()
            } else {
              int_ty.clone()
            };
            if let Err(e) = self.unify(&rhs_ty, &target) {
              result.add_error(e);
            }
            true
          } else {
            false
          }
        };

        // 연산자에 따라 타입 추론
        match op.as_ref() {
          "+" | "-" | "*" | "/" => {
            if let Some(num_ty) = numeric_result {
              result.set_result_type(num_ty);
            } else {
              result.add_error(InferenceError::TypeMismatch {
                expected: CoreType::Named("Int or Float".to_string()),
                actual: lhs_ty,
              });
            }
          }
          "%" => {
            if let Some(num_ty) = numeric_result {
              if matches!(num_ty, CoreType::Named(ref n) if n == "Int") {
                result.set_result_type(num_ty);
              } else {
                result.add_error(InferenceError::TypeMismatch {
                  expected: int_ty.clone(),
                  actual: num_ty,
                });
              }
            } else {
              result.add_error(InferenceError::TypeMismatch {
                expected: int_ty.clone(),
                actual: lhs_ty,
              });
            }
          }
          "==" | "!=" | "<" | ">" | "<=" | ">=" => {
            // 비교 연산: Bool 반환
            let both_vars = is_var(&lhs_resolved) && is_var(&rhs_resolved);
            let comparable = if matches!(op.as_ref(), "==" | "!=") {
              numeric_comparable || self.unify(&lhs_ty, &rhs_ty).is_ok()
            } else {
              numeric_comparable
                || (!both_vars
                  && self.unify(&lhs_ty, &string_ty).is_ok()
                  && self.unify(&rhs_ty, &string_ty).is_ok())
                || (both_vars && self.unify(&lhs_ty, &rhs_ty).is_ok())
            };
            if comparable {
              result.set_result_type(bool_ty.clone());
            } else {
              result.add_error(InferenceError::TypeMismatch {
                expected: CoreType::Named("Comparable".to_string()),
                actual: lhs_ty,
              });
            }
          }
          "&&" | "||" => {
            // 논리 연산: Bool 반환
            if let Err(_e) = self.unify(&lhs_ty, &bool_ty) {
              result.add_error(InferenceError::TypeMismatch {
                expected: bool_ty.clone(),
                actual: lhs_ty,
              });
            }
            if let Err(_e) = self.unify(&rhs_ty, &bool_ty) {
              result.add_error(InferenceError::TypeMismatch {
                expected: bool_ty.clone(),
                actual: rhs_ty,
              });
            }
            result.set_result_type(bool_ty.clone());
          }
          "++" => {
            // HIGH: ++ 연산자 타입 추론 수정
            // 문자열 연결 또는 리스트 연결
            // 양쪽 타입이 같아야 하며, String 또는 List 타입이어야 함
            if let Err(e) = self.unify(&lhs_ty, &rhs_ty) {
              result.add_error(e);
            } else {
              // String 또는 List 타입인지 확인
              let is_string = matches!(lhs_ty, CoreType::Named(ref n) if n == "String");
              let is_list = matches!(lhs_ty, CoreType::List(_));
              if is_string {
                result.set_result_type(CoreType::Named("String".to_string()));
              } else if is_list {
                // 리스트 연결: List<T> ++ List<T> → List<T>
                result.set_result_type(lhs_ty);
              } else {
                // 타입 불일치: String 또는 List가 아님
                result.add_error(InferenceError::TypeMismatch {
                  expected: CoreType::Named("String or List".to_string()),
                  actual: lhs_ty,
                });
              }
            }
          }
          _ => {
            result.add_error(InferenceError::UninferrableVar {
              name: format!("Unknown binary operator: {}", op),
            });
          }
        }
      }

      // Lambda: Arrow 타입 생성
      PnixExpr::Lambda { param, body } => {
        // 파라미터 타입 추론 (패턴 지원)
        let saved_env = self.env.clone();
        let (param_ty, bindings) = self.infer_param_pattern(param, None, &mut result);
        for (name, ty) in bindings {
          self.env.insert(name, ty);
        }

        // 본체 타입 추론
        let body_result = self.infer_expr(body);
        // 본체 타입 추출
        let body_ty = self.expr_type_or_fresh(&body_result);
        result.errors.extend(body_result.errors);

        // 환경 복원
        self.env = saved_env;

        // LOW: Lambda가 파라미터 타입 변수를 generalize 안함
        // 현재는 타입 변수를 그대로 사용하므로 다형성 손실
        // 향후 개선: 타입 변수를 Forall로 일반화하여 다형성 지원
        // Arrow 타입 생성: param_ty -> body_ty
        let arrow_ty = CoreType::Arrow(Box::new(param_ty), Box::new(body_ty));
        result
          .var_types
          .insert("_lambda".to_string(), arrow_ty.clone());
        result.expr_type = Some(arrow_ty);
      }

      // Match expression: 모든 arm의 body 타입이 동일해야 함
      PnixExpr::Match { scrutinee, arms } => {
        // scrutinee 타입 추론
        let scrutinee_result = self.infer_expr(scrutinee);
        let scrutinee_ty = self.expr_type_or_fresh(&scrutinee_result);
        result.errors.extend(scrutinee_result.errors);

        if arms.is_empty() {
          result.add_error(InferenceError::UninferrableVar {
            name: "Match expression must have at least one arm".to_string(),
          });
          return result;
        }

        let mut expected_ty: Option<CoreType> = None;

        // arm별로 패턴 바인딩 후 guard/body 타입을 검사
        for arm in arms {
          let saved_env = self.env.clone();
          let mut bindings = HashMap::new();
          if let Err(err) = self.bind_pattern_vars(&arm.pattern, &scrutinee_ty, &mut bindings) {
            result.add_error(err);
          }
          for (name, ty) in bindings {
            self.env.insert(name, ty);
          }

          if let Some(guard) = &arm.guard {
            let guard_result = self.infer_expr(guard);
            let guard_ty = self.expr_type_or_fresh(&guard_result);
            result.errors.extend(guard_result.errors);
            if let Err(_e) = self.unify(&guard_ty, &CoreType::Named("Bool".to_string())) {
              result.add_error(InferenceError::TypeMismatch {
                expected: CoreType::Named("Bool".to_string()),
                actual: guard_ty,
              });
            }
          }

          let arm_result = self.infer_expr(&arm.body);
          let arm_ty = self.expr_type_or_fresh(&arm_result);
          result.errors.extend(arm_result.errors);

          match expected_ty.clone() {
            Some(expected) => match self.unify(&expected, &arm_ty) {
              Ok(unified_ty) => {
                expected_ty = Some(unified_ty);
              }
              Err(_e) => {
                result.add_error(InferenceError::TypeMismatch {
                  expected,
                  actual: arm_ty,
                });
              }
            },
            None => {
              expected_ty = Some(arm_ty);
            }
          }

          self.env = saved_env;
        }

        if let Some(error) = self.check_match_exhaustiveness(arms, &scrutinee_ty) {
          result.add_error(error);
        }

        if let Some(expected_ty) = expected_ty {
          result.set_result_type(expected_ty);
        }
      }

      // List: 모든 요소의 타입이 동일해야 함 → List<T>
      PnixExpr::List(items) => {
        if items.is_empty() {
          // 빈 리스트: List<T> (T는 타입 변수)
          let elem_ty = self.fresh_type_var();
          let list_ty = CoreType::List(Box::new(elem_ty));
          result
            .var_types
            .insert("_literal".to_string(), list_ty.clone());
          result.expr_type = Some(list_ty);
        } else {
          // 첫 번째 요소의 타입을 기준으로 설정
          let first_result = self.infer_expr(&items[0]);
          let mut elem_ty = self.expr_type_or_fresh(&first_result);
          result.errors.extend(first_result.errors);

          // 나머지 요소들의 타입이 첫 번째와 일치하는지 확인
          for item in items.iter().skip(1) {
            let item_result = self.infer_expr(item);
            let item_ty = self.expr_type_or_fresh(&item_result);
            result.errors.extend(item_result.errors);

            match self.unify(&elem_ty, &item_ty) {
              Ok(unified_ty) => {
                elem_ty = unified_ty;
              }
              Err(_e) => {
                result.add_error(InferenceError::TypeMismatch {
                  expected: elem_ty.clone(),
                  actual: item_ty,
                });
              }
            }
          }

          let list_ty = CoreType::List(Box::new(elem_ty));
          result
            .var_types
            .insert("_literal".to_string(), list_ty.clone());
          result.expr_type = Some(list_ty);
        }
      }

      // AttrSet: Record 타입으로 표현
      PnixExpr::AttrSet { items, .. } => {
        use crate::lang::pnix::syntax::PnixAttrItem;
        let mut entries: Vec<(String, AttrTypeNode)> = Vec::new();
        for item in items {
          match item {
            PnixAttrItem::Assign {
              key_path, value, ..
            } => {
              let value_result = self.infer_expr(value);
              let value_ty = self.expr_type_or_fresh(&value_result);
              result.errors.extend(value_result.errors);
              insert_attr_type_path(&mut entries, key_path, value_ty, &mut result.errors);
            }
            PnixAttrItem::Inherit { from, names, .. } => {
              // N00b: inherit (scope) x y; → infer types from scope expression
              if let Some(scope_expr) = from {
                let scope_result = self.infer_expr(scope_expr);
                result.errors.extend(scope_result.errors);
                // For now, treat inherited vars as unknown type from scope
                for name in names {
                  insert_attr_type_path(
                    &mut entries,
                    std::slice::from_ref(name),
                    self.fresh_type_var(),
                    &mut result.errors,
                  );
                }
              } else {
                // inherit x y; → get types from environment
                for name in names {
                  if let Some(ty) = self.env.get(name.as_str()) {
                    insert_attr_type_path(
                      &mut entries,
                      std::slice::from_ref(name),
                      ty.clone(),
                      &mut result.errors,
                    );
                  } else {
                    result.add_error(InferenceError::UninferrableVar {
                      name: format!("Inherited variable '{}' not found in scope", name),
                    });
                    insert_attr_type_path(
                      &mut entries,
                      std::slice::from_ref(name),
                      self.fresh_type_var(),
                      &mut result.errors,
                    );
                  }
                }
              }
            }
            PnixAttrItem::DynamicAssign {
              key_path, value, ..
            } => {
              // N00k: Dynamic keys - infer types but can't determine key statically
              use crate::lang::pnix::syntax::AttrKeySegment;
              for segment in key_path {
                if let AttrKeySegment::Dynamic(expr) = segment {
                  let key_result = self.infer_expr(expr);
                  result.errors.extend(key_result.errors);
                }
              }
              let value_result = self.infer_expr(value);
              // Dynamic key: we can't know the key name statically
              // Use a placeholder name with index to avoid collisions
              let value_ty = self.expr_type_or_fresh(&value_result);
              result.errors.extend(value_result.errors);
              let dynamic_key = format!("_dynamic_{}", entries.len());
              insert_attr_type_path(
                &mut entries,
                std::slice::from_ref(&dynamic_key),
                value_ty,
                &mut result.errors,
              );
            }
          }
        }

        let record_ty = CoreType::Record(attr_type_tree_to_fields(entries));
        result
          .var_types
          .insert("_literal".to_string(), record_ty.clone());
        result.expr_type = Some(record_ty);
      }

      // Select: AttrSet에서 필드 선택 (base.attr)
      PnixExpr::Select { base, attr } => {
        let base_result = self.infer_expr(base);
        let base_errors = base_result.errors.clone();
        result.errors.extend(base_errors);
        let base_ty = self.expr_type_or_fresh(&base_result);

        // base가 Record 타입이면 필드 타입 추출
        if let CoreType::Record(fields) = &base_ty {
          if let Some((_, field_ty)) = fields.iter().find(|(name, _)| name == attr) {
            result.set_result_type(field_ty.clone());
          } else {
            result.add_error(InferenceError::UninferrableVar {
              name: format!("Field '{}' not found in record type", attr),
            });
            // 타입 변수로 대체
            result.set_result_type(self.fresh_type_var());
          }
        } else {
          // base 타입이 Record가 아니면 타입 변수로 추론
          // (동적 타입이거나 아직 추론되지 않은 경우)
          result.set_result_type(self.fresh_type_var());
        }
      }

      // Y10d: SelectOrDefault - x.y or default
      PnixExpr::SelectOrDefault {
        base,
        attr,
        default,
      } => {
        // base.attr의 타입 추론
        let base_result = self.infer_expr(base);
        result.errors.extend(base_result.errors.clone());
        let base_ty = self.expr_type_or_fresh(&base_result);

        // default의 타입 추론
        let default_result = self.infer_expr(default);
        result.errors.extend(default_result.errors.clone());
        let default_ty = self.expr_type_or_fresh(&default_result);

        // base가 Record 타입이면 필드 타입과 default 타입을 통합
        // LOW: SelectOrDefault에서 필드와 default 타입 unify 안함
        // 필드 타입과 default 타입을 통합하지 않고 필드 타입만 사용
        // 향후 개선: unify를 통해 타입 안전성 향상
        if let CoreType::Record(fields) = &base_ty {
          if let Some((_, field_ty)) = fields.iter().find(|(name, _)| name == attr) {
            // 필드 타입과 default 타입이 일치해야 함 (또는 통합 가능)
            // 여기서는 필드 타입을 우선으로 사용 (default는 fallback용)
            result.set_result_type(field_ty.clone());
          } else {
            // 필드가 없으면 default 타입 사용
            result.set_result_type(default_ty);
          }
        } else {
          // base가 Record가 아니면 default 타입 사용
          result.set_result_type(default_ty);
        }
      }

      // Index: list[index] → 리스트 요소 타입
      PnixExpr::Index { base, index } => {
        // base의 타입 추론
        let base_result = self.infer_expr(base);
        result.errors.extend(base_result.errors.clone());
        let base_ty = self.expr_type_or_fresh(&base_result);

        // index의 타입 추론
        let index_result = self.infer_expr(index);
        result.errors.extend(index_result.errors.clone());
        // index 타입은 검사하지 않음 (런타임에서 Int 강제)

        // base가 List 타입이면 요소 타입 추출
        if let CoreType::List(elem_ty) = base_ty {
          result.set_result_type(*elem_ty);
        } else {
          // 타입을 알 수 없으면 fresh 타입 변수 생성
          result.set_result_type(self.fresh_type_var());
        }
      }

      // Construct: ADT 생성자 (Option, Result 등)
      PnixExpr::Construct { variant, args } => {
        // 각 인자의 타입 추론
        let arg_types: Vec<CoreType> = args
          .iter()
          .map(|arg| {
            let arg_result = self.infer_expr(arg);
            let arg_errors = arg_result.errors.clone();
            result.errors.extend(arg_errors);
            self.expr_type_or_fresh(&arg_result)
          })
          .collect();

        // variant에 따라 타입 결정
        match variant.as_str() {
          "Some" => {
            // Some(x) → Optional<T> where T is type of x
            if arg_types.len() == 1 {
              let optional_ty = CoreType::Optional(Box::new(arg_types[0].clone()));
              result
                .var_types
                .insert("_literal".to_string(), optional_ty.clone());
              result.expr_type = Some(optional_ty);
            } else {
              result.add_error(InferenceError::UnificationFailed {
                reason: format!("Some expects 1 argument, got {}", arg_types.len()),
              });
              let optional_ty = CoreType::Optional(Box::new(self.fresh_type_var()));
              result
                .var_types
                .insert("_literal".to_string(), optional_ty.clone());
              result.expr_type = Some(optional_ty);
            }
          }
          "None" => {
            // None → Optional<T> (T는 타입 변수)
            if arg_types.is_empty() {
              let optional_ty = CoreType::Optional(Box::new(self.fresh_type_var()));
              result
                .var_types
                .insert("_literal".to_string(), optional_ty.clone());
              result.expr_type = Some(optional_ty);
            } else {
              result.add_error(InferenceError::UnificationFailed {
                reason: format!("None expects 0 arguments, got {}", arg_types.len()),
              });
              let optional_ty = CoreType::Optional(Box::new(self.fresh_type_var()));
              result
                .var_types
                .insert("_literal".to_string(), optional_ty.clone());
              result.expr_type = Some(optional_ty);
            }
          }
          "Ok" | "Err" => {
            // Result 타입: Sum<Ok, Err>
            if arg_types.len() == 1 {
              let other_ty = self.fresh_type_var();
              if variant == "Ok" {
                let sum_ty = CoreType::Sum(Box::new(arg_types[0].clone()), Box::new(other_ty));
                result
                  .var_types
                  .insert("_literal".to_string(), sum_ty.clone());
                result.expr_type = Some(sum_ty);
              } else {
                let sum_ty = CoreType::Sum(Box::new(other_ty), Box::new(arg_types[0].clone()));
                result
                  .var_types
                  .insert("_literal".to_string(), sum_ty.clone());
                result.expr_type = Some(sum_ty);
              }
            } else {
              result.add_error(InferenceError::UnificationFailed {
                reason: format!("{} expects 1 argument, got {}", variant, arg_types.len()),
              });
              let sum_ty = CoreType::Sum(
                Box::new(self.fresh_type_var()),
                Box::new(self.fresh_type_var()),
              );
              result
                .var_types
                .insert("_literal".to_string(), sum_ty.clone());
              result.expr_type = Some(sum_ty);
            }
          }
          _ => {
            // 사용자 정의 ADT: Named 타입으로 표현
            let type_name = self
              .adt_type_for_variant(variant)
              .unwrap_or(variant.as_str());
            let named_ty = CoreType::Named(type_name.to_string());
            result
              .var_types
              .insert("_literal".to_string(), named_ty.clone());
            result.expr_type = Some(named_ty);
          }
        }
      }
      PnixExpr::Import { path } => {
        // N01a: import는 path를 받아 모듈(attrset)을 반환
        // path의 타입은 String 또는 Path
        let _path_result = self.infer_expr(path);
        // import의 반환 타입은 일반적으로 알 수 없음 (다형적)
        // 타입 변수로 표현
        let import_ty = self.fresh_type_var();
        result
          .var_types
          .insert("_import".to_string(), import_ty.clone());
        result.expr_type = Some(import_ty);
      }
      PnixExpr::With { env, body } => {
        // with 표현식: with pkgs; [gcc make] → 스코프 확장
        // env는 attrset이어야 하고, body의 타입을 반환
        let env_result = self.infer_expr(env);
        result.errors.extend(env_result.errors);

        let body_result = self.infer_expr(body);
        result.errors.extend(body_result.errors);

        // body의 타입을 결과로 사용
        if let Some(body_ty) = body_result.expr_type {
          result.expr_type = Some(body_ty);
        } else {
          result.add_error(InferenceError::UninferrableVar {
            name: "with expression body".to_string(),
          });
        }
      }
      // Y10b: Path literals have Path type (treated as String for now)
      PnixExpr::Path(_) => {
        let ty = CoreType::Named("Path".to_string());
        result.var_types.insert("_path".to_string(), ty.clone());
        result.expr_type = Some(ty);
      }
      // Y10e: Assert expression - cond should be Bool, result is body's type
      PnixExpr::Assert { cond, body } => {
        // cond should be Bool
        let cond_result = self.infer_expr(cond);
        result.errors.extend(cond_result.errors);

        // body determines the result type
        let body_result = self.infer_expr(body);
        result.errors.extend(body_result.errors);

        if let Some(body_ty) = body_result.expr_type {
          result.expr_type = Some(body_ty);
        } else {
          result.expr_type = Some(self.fresh_type_var());
        }
      }
      // N00e: HasAttr expression - x ? a → Bool
      PnixExpr::HasAttr { base, .. } => {
        // base should be an AttrSet (or at least something we can check attributes on)
        let base_result = self.infer_expr(base);
        result.errors.extend(base_result.errors);
        // Result is always Bool
        result.expr_type = Some(CoreType::Named("Bool".to_string()));
      }
      // N00p: DynamicHasAttr expression - x ? ${expr} → Bool
      PnixExpr::DynamicHasAttr { base, attr_expr } => {
        // base should be an AttrSet
        let base_result = self.infer_expr(base);
        result.errors.extend(base_result.errors);
        // attr_expr should be a String
        let attr_result = self.infer_expr(attr_expr);
        result.errors.extend(attr_result.errors);
        // Result is always Bool
        result.expr_type = Some(CoreType::Named("Bool".to_string()));
      }
      // N00f: DynamicSelect expression - x.${expr} → Any (we don't know the type at compile time)
      PnixExpr::DynamicSelect { base, attr_expr } => {
        // base should be an AttrSet
        let base_result = self.infer_expr(base);
        result.errors.extend(base_result.errors);
        // attr_expr should be a String
        let attr_result = self.infer_expr(attr_expr);
        result.errors.extend(attr_result.errors);
        // Result type is unknown at compile time
        result.expr_type = Some(self.fresh_type_var());
      }
      // N00m: DynamicSelectOrDefault expression - x.${expr} or default
      PnixExpr::DynamicSelectOrDefault {
        base,
        attr_expr,
        default,
      } => {
        // base should be an AttrSet
        let base_result = self.infer_expr(base);
        result.errors.extend(base_result.errors);
        // attr_expr should be a String
        let attr_result = self.infer_expr(attr_expr);
        result.errors.extend(attr_result.errors);
        // default expression
        let default_result = self.infer_expr(default);
        result.errors.extend(default_result.errors);
        // Result type could be the default type or the attribute type
        result.expr_type = default_result.expr_type.or(Some(self.fresh_type_var()));
      }
    }

    self.infer_depth = self.infer_depth.saturating_sub(1);
    result
  }
}

impl Default for TypeInferencer {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::lang::pnix::syntax::{PnixListPattern, PnixParamPattern, PnixPatternField};

  #[test]
  fn test_infer_literal_int() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Int(42);
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    assert_eq!(
      result.var_types.get("_literal"),
      Some(&CoreType::Named("Int".to_string()))
    );
  }

  #[test]
  fn test_infer_literal_float() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Float(2.71);
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    assert_eq!(
      result.var_types.get("_literal"),
      Some(&CoreType::Named("Float".to_string()))
    );
  }

  #[test]
  fn test_infer_literal_bool() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Bool(true);
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    assert_eq!(
      result.var_types.get("_literal"),
      Some(&CoreType::Named("Bool".to_string()))
    );
  }

  #[test]
  fn test_generalize_applies_substitutions() {
    let mut inferencer = TypeInferencer::new();
    let ty = inferencer.fresh_type_var();
    let CoreType::Var(name) = ty.clone() else {
      panic!("expected fresh type var");
    };
    inferencer.substitute(name, CoreType::Named("Int".to_string()));
    let generalized = inferencer.generalize(&ty);
    assert_eq!(generalized, CoreType::Named("Int".to_string()));
  }

  #[test]
  fn test_infer_clears_substitutions_on_root_call() {
    let mut inferencer = TypeInferencer::new();
    inferencer
      .type_substitutions
      .insert("T999".to_string(), CoreType::Unit);
    let expr = PnixExpr::Int(1);
    let _ = inferencer.infer_expr(&expr);
    assert!(inferencer.type_substitutions.is_empty());
  }

  #[test]
  fn test_infer_literal_string() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::String("hello".to_string());
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    assert_eq!(
      result.var_types.get("_literal"),
      Some(&CoreType::Named("String".to_string()))
    );
  }

  #[test]
  fn test_infer_literal_null() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Null;
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    let Some(CoreType::Optional(inner)) = result.var_types.get("_literal") else {
      panic!("expected Optional type for null literal");
    };
    assert!(matches!(inner.as_ref(), CoreType::Var(_)));
  }

  #[test]
  fn test_infer_var_unbound() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Var("x".to_string());
    let result = inferencer.infer_expr(&expr);
    assert!(!result.errors.is_empty());
    assert!(
      matches!(result.errors[0], InferenceError::UninferrableVar { name: ref n } if n == "x")
    );
  }

  #[test]
  fn test_infer_var_bound() {
    let mut inferencer = TypeInferencer::new();
    inferencer
      .env
      .insert("x".to_string(), CoreType::Named("Int".to_string()));
    let expr = PnixExpr::Var("x".to_string());
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    assert_eq!(
      result.var_types.get("x"),
      Some(&CoreType::Named("Int".to_string()))
    );
  }

  #[test]
  fn test_infer_let_binding() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Let {
      bindings: vec![PnixLetBinding::Binding {
        pattern: PnixParamPattern::Ident("x".to_string()),
        value: PnixExpr::Int(42),
      }],
      body: Arc::new(PnixExpr::Var("x".to_string())),
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    assert_eq!(
      result.var_types.get("x"),
      Some(&CoreType::Named("Int".to_string()))
    );
  }

  #[test]
  fn test_infer_if_expression() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::If {
      cond: Arc::new(PnixExpr::Bool(true)),
      then_: Arc::new(PnixExpr::Int(1)),
      else_: Arc::new(PnixExpr::Int(2)),
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    assert_eq!(
      result.var_types.get("_result"),
      Some(&CoreType::Named("Int".to_string()))
    );
  }

  #[test]
  fn test_infer_if_type_mismatch() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::If {
      cond: Arc::new(PnixExpr::Bool(true)),
      then_: Arc::new(PnixExpr::Int(1)),
      else_: Arc::new(PnixExpr::String("two".to_string())),
    };
    let result = inferencer.infer_expr(&expr);
    assert!(!result.errors.is_empty());
    assert!(matches!(
      result.errors[0],
      InferenceError::TypeMismatch { .. }
    ));
  }

  #[test]
  fn test_infer_if_cond_not_bool() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::If {
      cond: Arc::new(PnixExpr::Int(1)),
      then_: Arc::new(PnixExpr::Int(2)),
      else_: Arc::new(PnixExpr::Int(3)),
    };
    let result = inferencer.infer_expr(&expr);
    assert!(!result.errors.is_empty());
    assert!(
      matches!(result.errors[0], InferenceError::TypeMismatch { expected: ref e, .. } if matches!(e, CoreType::Named(ref n) if n == "Bool"))
    );
  }

  #[test]
  fn test_infer_binary_add() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Binary {
      op: "+",
      lhs: Arc::new(PnixExpr::Int(1)),
      rhs: Arc::new(PnixExpr::Int(2)),
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    assert_eq!(
      result.var_types.get("_result"),
      Some(&CoreType::Named("Int".to_string()))
    );
  }

  #[test]
  fn test_infer_binary_add_float() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Binary {
      op: "+",
      lhs: Arc::new(PnixExpr::Float(1.0)),
      rhs: Arc::new(PnixExpr::Float(2.0)),
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    assert_eq!(
      result.var_types.get("_result"),
      Some(&CoreType::Named("Float".to_string()))
    );
  }

  #[test]
  fn test_infer_binary_eq() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Binary {
      op: "==",
      lhs: Arc::new(PnixExpr::Int(1)),
      rhs: Arc::new(PnixExpr::Int(2)),
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    assert_eq!(
      result.var_types.get("_result"),
      Some(&CoreType::Named("Bool".to_string()))
    );
  }

  #[test]
  fn test_let_binding_does_not_leak_scope() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Let {
      bindings: vec![PnixLetBinding::Binding {
        pattern: PnixParamPattern::Ident("x".to_string()),
        value: PnixExpr::Int(1),
      }],
      body: Arc::new(PnixExpr::Int(0)),
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    assert!(!inferencer.env.contains_key("x"));
  }

  #[test]
  fn test_let_expr_type_is_body_type() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Let {
      bindings: vec![PnixLetBinding::Binding {
        pattern: PnixParamPattern::Ident("x".to_string()),
        value: PnixExpr::Int(1),
      }],
      body: Arc::new(PnixExpr::Bool(true)),
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    assert_eq!(result.expr_type, Some(CoreType::Named("Bool".to_string())));
  }

  #[test]
  fn test_lambda_does_not_leak_scope() {
    let mut inferencer = TypeInferencer::new();
    inferencer
      .env
      .insert("x".to_string(), CoreType::Named("Int".to_string()));
    let expr = PnixExpr::Lambda {
      param: PnixParamPattern::Ident("x".to_string()),
      body: Arc::new(PnixExpr::Var("x".to_string())),
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    assert_eq!(
      inferencer.env.get("x"),
      Some(&CoreType::Named("Int".to_string()))
    );
  }

  #[test]
  fn test_var_instantiates_forall() {
    let mut inferencer = TypeInferencer::new();
    inferencer.env.insert(
      "id".to_string(),
      CoreType::Forall {
        vars: vec!["a".to_string()],
        body: Box::new(CoreType::Var("a".to_string())),
      },
    );

    let result1 = inferencer.infer_expr(&PnixExpr::Var("id".to_string()));
    let ty1 = result1.var_types.get("id").cloned().unwrap();
    let result2 = inferencer.infer_expr(&PnixExpr::Var("id".to_string()));
    let ty2 = result2.var_types.get("id").cloned().unwrap();

    assert!(matches!(ty1, CoreType::Var(_)));
    assert!(matches!(ty2, CoreType::Var(_)));
    assert_ne!(ty1, ty2);
  }

  #[test]
  fn test_instantiate_forall_avoids_existing_type_var_names() {
    let mut inferencer = TypeInferencer::new();
    inferencer
      .type_substitutions
      .insert("T0".to_string(), CoreType::Named("Int".to_string()));
    inferencer
      .env
      .insert("x".to_string(), CoreType::Var("T1".to_string()));

    let ty = CoreType::Forall {
      vars: vec!["a".to_string()],
      body: Box::new(CoreType::Var("a".to_string())),
    };
    let instantiated = inferencer.instantiate_forall(&ty);

    match instantiated {
      CoreType::Var(name) => {
        assert_ne!(name, "T0");
        assert_ne!(name, "T1");
      }
      _ => panic!("expected type variable"),
    }
  }

  #[test]
  fn test_instantiate_forall_freshens_bound_var_name() {
    let mut inferencer = TypeInferencer::new();
    let ty = CoreType::Forall {
      vars: vec!["T0".to_string()],
      body: Box::new(CoreType::Var("T0".to_string())),
    };
    let instantiated = inferencer.instantiate_forall(&ty);
    match instantiated {
      CoreType::Var(name) => {
        assert_ne!(name, "T0");
      }
      _ => panic!("expected type variable"),
    }
  }

  #[test]
  fn test_apply_instantiates_forall_output() {
    let mut inferencer = TypeInferencer::new();
    inferencer.env.insert(
      "r".to_string(),
      CoreType::Record(vec![(
        "id".to_string(),
        CoreType::Forall {
          vars: vec!["a".to_string()],
          body: Box::new(CoreType::Arrow(
            Box::new(CoreType::Var("a".to_string())),
            Box::new(CoreType::Var("a".to_string())),
          )),
        },
      )]),
    );

    let expr = PnixExpr::Apply {
      func: Arc::new(PnixExpr::Select {
        base: Arc::new(PnixExpr::Var("r".to_string())),
        attr: "id".to_string(),
      }),
      arg: Arc::new(PnixExpr::Int(1)),
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    assert_eq!(result.expr_type, Some(CoreType::Named("Int".to_string())));
  }

  #[test]
  fn test_infer_binary_and() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Binary {
      op: "&&",
      lhs: Arc::new(PnixExpr::Bool(true)),
      rhs: Arc::new(PnixExpr::Bool(false)),
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    assert_eq!(
      result.var_types.get("_result"),
      Some(&CoreType::Named("Bool".to_string()))
    );
  }

  #[test]
  fn test_infer_binary_and_type_error() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Binary {
      op: "&&",
      lhs: Arc::new(PnixExpr::Int(1)),
      rhs: Arc::new(PnixExpr::Bool(false)),
    };
    let result = inferencer.infer_expr(&expr);
    assert!(!result.errors.is_empty());
    assert!(
      matches!(result.errors[0], InferenceError::TypeMismatch { expected: ref e, .. } if matches!(e, CoreType::Named(ref n) if n == "Bool"))
    );
  }

  #[test]
  fn test_instantiate_generic_list() {
    let mut inferencer = TypeInferencer::new();
    // List<T>를 List<Int>로 인스턴스화
    let generic_list = CoreType::List(Box::new(CoreType::Var("T".to_string())));
    let type_args = vec![CoreType::Named("Int".to_string())];
    let result = inferencer.instantiate_generic(&generic_list, &type_args);
    assert!(result.is_ok());
    let instantiated = result.unwrap();
    match instantiated {
      CoreType::List(inner) => {
        assert_eq!(*inner, CoreType::Named("Int".to_string()));
      }
      _ => panic!("Expected List type"),
    }
  }

  #[test]
  fn test_instantiate_generic_nested() {
    let mut inferencer = TypeInferencer::new();
    // List<List<T>>를 List<List<Int>>로 인스턴스화
    // 타입 변수 T를 Int로 대체
    inferencer.substitute("T".to_string(), CoreType::Named("Int".to_string()));
    let inner_list = CoreType::List(Box::new(CoreType::Var("T".to_string())));
    let outer_list = CoreType::List(Box::new(inner_list));
    // 재귀적 인스턴스화 사용 (타입 변수 대체 기반)
    let result = inferencer.instantiate_generic_recursive(&outer_list, 0);
    assert!(result.is_ok());
    let instantiated = result.unwrap();
    match instantiated {
      CoreType::List(outer_inner) => match *outer_inner {
        CoreType::List(inner_inner) => {
          assert_eq!(*inner_inner, CoreType::Named("Int".to_string()));
        }
        _ => panic!("Expected nested List, got {:?}", outer_inner),
      },
      _ => panic!("Expected List type, got {:?}", instantiated),
    }
  }

  #[test]
  fn test_substitute_type_var() {
    let mut inferencer = TypeInferencer::new();
    inferencer.substitute("T".to_string(), CoreType::Named("Int".to_string()));
    let var_type = CoreType::Var("T".to_string());
    let result = inferencer.instantiate_generic(&var_type, &[]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), CoreType::Named("Int".to_string()));
  }

  #[test]
  fn test_recursion_depth_limit() {
    let mut inferencer = TypeInferencer::new();
    // 매우 깊은 중첩 타입 생성 (재귀 깊이 제한 테스트)
    let mut deep_type = CoreType::Named("Int".to_string());
    // MAX_RECURSION_DEPTH보다 깊게 중첩
    for _ in 0..=TypeInferencer::MAX_RECURSION_DEPTH {
      deep_type = CoreType::List(Box::new(deep_type));
    }

    let result = inferencer.instantiate_generic_recursive(&deep_type, 0);
    assert!(result.is_err());
    if let Err(InferenceError::RecursionDepthExceeded { max_depth }) = result {
      assert_eq!(max_depth, TypeInferencer::MAX_RECURSION_DEPTH);
    } else {
      panic!("Expected RecursionDepthExceeded error");
    }
  }

  #[test]
  fn test_recursion_depth_within_limit() {
    let mut inferencer = TypeInferencer::new();
    // 제한 내의 깊은 중첩 타입 (정상 동작 확인)
    let mut deep_type = CoreType::Named("Int".to_string());
    // MAX_RECURSION_DEPTH보다 작게 중첩
    for _ in 0..(TypeInferencer::MAX_RECURSION_DEPTH - 10) {
      deep_type = CoreType::List(Box::new(deep_type));
    }

    let result = inferencer.instantiate_generic_recursive(&deep_type, 0);
    assert!(
      result.is_ok(),
      "Should succeed within recursion depth limit"
    );
  }

  #[test]
  fn test_unify_occurs_check_rejects_recursive_type() {
    let mut inferencer = TypeInferencer::new();
    let t0 = CoreType::Var("T0".to_string());
    let list_t0 = CoreType::List(Box::new(t0.clone()));
    let result = inferencer.unify(&t0, &list_t0);
    assert!(matches!(
      result,
      Err(InferenceError::UnificationFailed { .. })
    ));
  }

  #[test]
  fn test_unify_occurs_check_rejects_indirect_recursive_substitution() {
    let mut inferencer = TypeInferencer::new();
    let t0 = CoreType::Var("T0".to_string());
    let t1 = CoreType::Var("T1".to_string());
    inferencer.substitute("T1".to_string(), CoreType::List(Box::new(t0.clone())));
    let result = inferencer.unify(&t0, &t1);
    assert!(matches!(
      result,
      Err(InferenceError::UnificationFailed { .. })
    ));
  }

  // ========== New type inference tests for List/AttrSet/Select/Match/Construct ==========

  #[test]
  fn test_infer_list_int() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::List(vec![PnixExpr::Int(1), PnixExpr::Int(2), PnixExpr::Int(3)]);
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    assert_eq!(
      result.var_types.get("_literal"),
      Some(&CoreType::List(Box::new(CoreType::Named(
        "Int".to_string()
      ))))
    );
  }

  #[test]
  fn test_infer_list_empty() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::List(vec![]);
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    // 빈 리스트는 List<T> (T는 타입 변수)
    let ty = result.var_types.get("_literal").unwrap();
    assert!(matches!(ty, CoreType::List(_)));
  }

  #[test]
  fn test_infer_list_type_mismatch() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::List(vec![
      PnixExpr::Int(1),
      PnixExpr::String("hello".to_string()),
    ]);
    let result = inferencer.infer_expr(&expr);
    assert!(!result.errors.is_empty());
    assert!(matches!(
      result.errors[0],
      InferenceError::TypeMismatch { .. }
    ));
  }

  #[test]
  fn test_infer_attrset() {
    let mut inferencer = TypeInferencer::new();
    use crate::lang::pnix::syntax::PnixAttrItem;
    let expr = PnixExpr::AttrSet {
      items: vec![
        PnixAttrItem::Assign {
          key_path: vec!["x".to_string()],
          value: PnixExpr::Int(1),
          span: crate::diagnostics::Span::empty(),
        },
        PnixAttrItem::Assign {
          key_path: vec!["y".to_string()],
          value: PnixExpr::String("hello".to_string()),
          span: crate::diagnostics::Span::empty(),
        },
      ],
      recursive: false,
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    let ty = result.var_types.get("_literal").unwrap();
    match ty {
      CoreType::Record(fields) => {
        assert_eq!(fields.len(), 2);
        assert!(fields
          .iter()
          .any(|(name, ty)| name == "x" && *ty == CoreType::Named("Int".to_string())));
        assert!(fields
          .iter()
          .any(|(name, ty)| name == "y" && *ty == CoreType::Named("String".to_string())));
      }
      _ => panic!("Expected Record type, got {:?}", ty),
    }
  }

  #[test]
  fn test_infer_attrset_nested() {
    let mut inferencer = TypeInferencer::new();
    use crate::lang::pnix::syntax::PnixAttrItem;
    let expr = PnixExpr::AttrSet {
      items: vec![
        PnixAttrItem::Assign {
          key_path: vec!["a".to_string(), "b".to_string()],
          value: PnixExpr::Int(1),
          span: crate::diagnostics::Span::empty(),
        },
        PnixAttrItem::Assign {
          key_path: vec!["a".to_string(), "c".to_string()],
          value: PnixExpr::String("hello".to_string()),
          span: crate::diagnostics::Span::empty(),
        },
      ],
      recursive: false,
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    let ty = result.var_types.get("_literal").unwrap();
    match ty {
      CoreType::Record(fields) => {
        let Some((_, inner)) = fields.iter().find(|(name, _)| name == "a") else {
          panic!("Expected field 'a' in record type");
        };
        match inner {
          CoreType::Record(inner_fields) => {
            assert!(inner_fields
              .iter()
              .any(|(name, ty)| { name == "b" && *ty == CoreType::Named("Int".to_string()) }));
            assert!(inner_fields
              .iter()
              .any(|(name, ty)| { name == "c" && *ty == CoreType::Named("String".to_string()) }));
          }
          _ => panic!("Expected nested Record type for 'a', got {:?}", inner),
        }
      }
      _ => panic!("Expected Record type, got {:?}", ty),
    }
  }

  #[test]
  fn test_infer_select() {
    let mut inferencer = TypeInferencer::new();
    use crate::lang::pnix::syntax::PnixAttrItem;
    let base = PnixExpr::AttrSet {
      items: vec![PnixAttrItem::Assign {
        key_path: vec!["x".to_string()],
        value: PnixExpr::Int(42),
        span: crate::diagnostics::Span::empty(),
      }],
      recursive: false,
    };
    let expr = PnixExpr::Select {
      base: Box::new(base),
      attr: "x".to_string(),
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    assert_eq!(
      result.var_types.get("_result"),
      Some(&CoreType::Named("Int".to_string()))
    );
  }

  #[test]
  fn test_infer_select_field_not_found() {
    let mut inferencer = TypeInferencer::new();
    use crate::lang::pnix::syntax::PnixAttrItem;
    let base = PnixExpr::AttrSet {
      items: vec![PnixAttrItem::Assign {
        key_path: vec!["x".to_string()],
        value: PnixExpr::Int(42),
        span: crate::diagnostics::Span::empty(),
      }],
      recursive: false,
    };
    let expr = PnixExpr::Select {
      base: Box::new(base),
      attr: "y".to_string(),
    };
    let result = inferencer.infer_expr(&expr);
    assert!(!result.errors.is_empty());
    assert!(
      matches!(result.errors[0], InferenceError::UninferrableVar { ref name } if name.contains("Field 'y' not found"))
    );
  }

  #[test]
  fn test_infer_construct_some() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Construct {
      variant: "Some".to_string(),
      args: vec![PnixExpr::Int(42)],
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    assert_eq!(
      result.var_types.get("_literal"),
      Some(&CoreType::Optional(Box::new(CoreType::Named(
        "Int".to_string()
      ))))
    );
  }

  #[test]
  fn test_infer_construct_none() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Construct {
      variant: "None".to_string(),
      args: vec![],
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    let ty = result.var_types.get("_literal").unwrap();
    assert!(matches!(ty, CoreType::Optional(_)));
  }

  #[test]
  fn test_infer_construct_ok() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Construct {
      variant: "Ok".to_string(),
      args: vec![PnixExpr::Int(42)],
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    let ty = result.var_types.get("_literal").unwrap();
    // Ok는 Sum 타입의 왼쪽
    assert!(matches!(ty, CoreType::Sum(left, _) if **left == CoreType::Named("Int".to_string())));
  }

  #[test]
  fn test_infer_construct_err() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Construct {
      variant: "Err".to_string(),
      args: vec![PnixExpr::String("error".to_string())],
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    let ty = result.var_types.get("_literal").unwrap();
    // Err는 Sum 타입의 오른쪽
    assert!(
      matches!(ty, CoreType::Sum(_, right) if **right == CoreType::Named("String".to_string()))
    );
  }

  #[test]
  fn test_infer_match_int() {
    use crate::lang::pnix::syntax::{PnixLiteralPattern, PnixMatchArm, PnixPattern};
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Int(1)),
      arms: vec![
        PnixMatchArm {
          pattern: PnixPattern::Literal(PnixLiteralPattern::Int(1)),
          guard: None,
          body: PnixExpr::String("one".to_string()),
        },
        PnixMatchArm {
          pattern: PnixPattern::Wildcard,
          guard: None,
          body: PnixExpr::String("other".to_string()),
        },
      ],
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    assert_eq!(
      result.var_types.get("_result"),
      Some(&CoreType::Named("String".to_string()))
    );
  }

  #[test]
  fn test_infer_match_type_mismatch() {
    use crate::lang::pnix::syntax::{PnixLiteralPattern, PnixMatchArm, PnixPattern};
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Int(1)),
      arms: vec![
        PnixMatchArm {
          pattern: PnixPattern::Literal(PnixLiteralPattern::Int(1)),
          guard: None,
          body: PnixExpr::String("one".to_string()),
        },
        PnixMatchArm {
          pattern: PnixPattern::Wildcard,
          guard: None,
          body: PnixExpr::Int(0), // 타입 불일치!
        },
      ],
    };
    let result = inferencer.infer_expr(&expr);
    assert!(!result.errors.is_empty());
    assert!(matches!(
      result.errors[0],
      InferenceError::TypeMismatch { .. }
    ));
  }

  #[test]
  fn test_infer_match_with_guard() {
    use crate::lang::pnix::syntax::{PnixMatchArm, PnixPattern};
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Int(1)),
      arms: vec![PnixMatchArm {
        pattern: PnixPattern::Wildcard,
        guard: Some(Arc::new(PnixExpr::Bool(true))), // Bool 가드
        body: PnixExpr::Int(42),
      }],
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result
      .errors
      .iter()
      .any(|err| matches!(err, InferenceError::NonExhaustiveMatch { .. })));
  }

  #[test]
  fn test_infer_match_guard_not_bool() {
    use crate::lang::pnix::syntax::{PnixMatchArm, PnixPattern};
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Int(1)),
      arms: vec![PnixMatchArm {
        pattern: PnixPattern::Wildcard,
        guard: Some(Arc::new(PnixExpr::Int(1))), // Int 가드 (에러)
        body: PnixExpr::Int(42),
      }],
    };
    let result = inferencer.infer_expr(&expr);
    assert!(!result.errors.is_empty());
    assert!(
      matches!(result.errors[0], InferenceError::TypeMismatch { expected: ref e, .. } if matches!(e, CoreType::Named(ref n) if n == "Bool"))
    );
  }

  #[test]
  fn test_infer_match_bool_non_exhaustive() {
    use crate::lang::pnix::syntax::{PnixLiteralPattern, PnixMatchArm, PnixPattern};
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Bool(true)),
      arms: vec![PnixMatchArm {
        pattern: PnixPattern::Literal(PnixLiteralPattern::Bool(true)),
        guard: None,
        body: PnixExpr::Int(1),
      }],
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result
      .errors
      .iter()
      .any(|err| matches!(err, InferenceError::NonExhaustiveMatch { .. })));
  }

  #[test]
  fn test_infer_match_bool_exhaustive() {
    use crate::lang::pnix::syntax::{PnixLiteralPattern, PnixMatchArm, PnixPattern};
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Bool(true)),
      arms: vec![
        PnixMatchArm {
          pattern: PnixPattern::Literal(PnixLiteralPattern::Bool(true)),
          guard: None,
          body: PnixExpr::Int(1),
        },
        PnixMatchArm {
          pattern: PnixPattern::Literal(PnixLiteralPattern::Bool(false)),
          guard: None,
          body: PnixExpr::Int(0),
        },
      ],
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
  }

  #[test]
  fn test_infer_match_option_non_exhaustive() {
    use crate::lang::pnix::syntax::{PnixMatchArm, PnixPattern};
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Construct {
        variant: "Some".to_string(),
        args: vec![PnixExpr::Int(1)],
      }),
      arms: vec![PnixMatchArm {
        pattern: PnixPattern::Constructor {
          variant: "Some".to_string(),
          args: vec![PnixPattern::Var("x".to_string())],
        },
        guard: None,
        body: PnixExpr::Int(1),
      }],
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result
      .errors
      .iter()
      .any(|err| matches!(err, InferenceError::NonExhaustiveMatch { .. })));
  }

  #[test]
  fn test_infer_match_option_exhaustive() {
    use crate::lang::pnix::syntax::{PnixMatchArm, PnixPattern};
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Construct {
        variant: "Some".to_string(),
        args: vec![PnixExpr::Int(1)],
      }),
      arms: vec![
        PnixMatchArm {
          pattern: PnixPattern::Constructor {
            variant: "Some".to_string(),
            args: vec![PnixPattern::Var("x".to_string())],
          },
          guard: None,
          body: PnixExpr::Int(1),
        },
        PnixMatchArm {
          pattern: PnixPattern::Constructor {
            variant: "None".to_string(),
            args: vec![],
          },
          guard: None,
          body: PnixExpr::Int(0),
        },
      ],
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
  }

  #[test]
  fn test_infer_match_result_non_exhaustive() {
    use crate::lang::pnix::syntax::{PnixMatchArm, PnixPattern};
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Construct {
        variant: "Ok".to_string(),
        args: vec![PnixExpr::Int(1)],
      }),
      arms: vec![PnixMatchArm {
        pattern: PnixPattern::Constructor {
          variant: "Ok".to_string(),
          args: vec![PnixPattern::Var("x".to_string())],
        },
        guard: None,
        body: PnixExpr::Int(1),
      }],
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result
      .errors
      .iter()
      .any(|err| matches!(err, InferenceError::NonExhaustiveMatch { .. })));
  }

  #[test]
  fn test_infer_match_result_exhaustive() {
    use crate::lang::pnix::syntax::{PnixMatchArm, PnixPattern};
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Construct {
        variant: "Ok".to_string(),
        args: vec![PnixExpr::Int(1)],
      }),
      arms: vec![
        PnixMatchArm {
          pattern: PnixPattern::Constructor {
            variant: "Ok".to_string(),
            args: vec![PnixPattern::Var("x".to_string())],
          },
          guard: None,
          body: PnixExpr::Int(1),
        },
        PnixMatchArm {
          pattern: PnixPattern::Constructor {
            variant: "Err".to_string(),
            args: vec![PnixPattern::Var("e".to_string())],
          },
          guard: None,
          body: PnixExpr::Int(0),
        },
      ],
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
  }

  #[test]
  fn test_infer_construct_custom_adt_uses_type() {
    let mut inferencer = TypeInferencer::new();
    inferencer.register_adt_variants(
      "Color",
      vec!["Red".to_string(), "Green".to_string(), "Blue".to_string()],
    );
    let expr = PnixExpr::Construct {
      variant: "Red".to_string(),
      args: vec![],
    };
    let result = inferencer.infer_expr(&expr);
    assert_eq!(
      result.var_types.get("_literal"),
      Some(&CoreType::Named("Color".to_string()))
    );
  }

  #[test]
  fn test_infer_match_custom_adt_non_exhaustive() {
    use crate::lang::pnix::syntax::{PnixMatchArm, PnixPattern};
    let mut inferencer = TypeInferencer::new();
    inferencer.register_adt_variants(
      "Color",
      vec!["Red".to_string(), "Green".to_string(), "Blue".to_string()],
    );
    inferencer
      .env
      .insert("x".to_string(), CoreType::Named("Color".to_string()));
    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Var("x".to_string())),
      arms: vec![
        PnixMatchArm {
          pattern: PnixPattern::Constructor {
            variant: "Red".to_string(),
            args: vec![],
          },
          guard: None,
          body: PnixExpr::Int(1),
        },
        PnixMatchArm {
          pattern: PnixPattern::Constructor {
            variant: "Green".to_string(),
            args: vec![],
          },
          guard: None,
          body: PnixExpr::Int(2),
        },
      ],
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result
      .errors
      .iter()
      .any(|err| matches!(err, InferenceError::NonExhaustiveMatch { .. })));
  }

  #[test]
  fn test_infer_match_custom_adt_exhaustive() {
    use crate::lang::pnix::syntax::{PnixMatchArm, PnixPattern};
    let mut inferencer = TypeInferencer::new();
    inferencer.register_adt_variants(
      "Color",
      vec!["Red".to_string(), "Green".to_string(), "Blue".to_string()],
    );
    inferencer
      .env
      .insert("x".to_string(), CoreType::Named("Color".to_string()));
    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Var("x".to_string())),
      arms: vec![
        PnixMatchArm {
          pattern: PnixPattern::Constructor {
            variant: "Red".to_string(),
            args: vec![],
          },
          guard: None,
          body: PnixExpr::Int(1),
        },
        PnixMatchArm {
          pattern: PnixPattern::Constructor {
            variant: "Green".to_string(),
            args: vec![],
          },
          guard: None,
          body: PnixExpr::Int(2),
        },
        PnixMatchArm {
          pattern: PnixPattern::Constructor {
            variant: "Blue".to_string(),
            args: vec![],
          },
          guard: None,
          body: PnixExpr::Int(3),
        },
      ],
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
  }

  #[test]
  fn test_infer_match_overlapping_literal_patterns() {
    use crate::lang::pnix::syntax::{PnixLiteralPattern, PnixMatchArm, PnixPattern};
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Int(1)),
      arms: vec![
        PnixMatchArm {
          pattern: PnixPattern::Literal(PnixLiteralPattern::Int(1)),
          guard: None,
          body: PnixExpr::Int(1),
        },
        PnixMatchArm {
          pattern: PnixPattern::Literal(PnixLiteralPattern::Int(1)),
          guard: None,
          body: PnixExpr::Int(2),
        },
      ],
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result
      .errors
      .iter()
      .any(|err| matches!(err, InferenceError::OverlappingMatch { .. })));
  }

  #[test]
  fn test_infer_match_overlapping_constructor_patterns() {
    use crate::lang::pnix::syntax::{PnixMatchArm, PnixPattern};
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Construct {
        variant: "Some".to_string(),
        args: vec![PnixExpr::Int(1)],
      }),
      arms: vec![
        PnixMatchArm {
          pattern: PnixPattern::Constructor {
            variant: "Some".to_string(),
            args: vec![PnixPattern::Var("x".to_string())],
          },
          guard: None,
          body: PnixExpr::Int(1),
        },
        PnixMatchArm {
          pattern: PnixPattern::Constructor {
            variant: "Some".to_string(),
            args: vec![PnixPattern::Var("y".to_string())],
          },
          guard: None,
          body: PnixExpr::Int(2),
        },
      ],
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result
      .errors
      .iter()
      .any(|err| matches!(err, InferenceError::OverlappingMatch { .. })));
  }

  #[test]
  fn test_infer_match_pattern_binding_some() {
    use crate::lang::pnix::syntax::{PnixMatchArm, PnixPattern};
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Construct {
        variant: "Some".to_string(),
        args: vec![PnixExpr::Int(1)],
      }),
      arms: vec![
        PnixMatchArm {
          pattern: PnixPattern::Constructor {
            variant: "Some".to_string(),
            args: vec![PnixPattern::Var("x".to_string())],
          },
          guard: None,
          body: PnixExpr::Binary {
            op: "+",
            lhs: Arc::new(PnixExpr::Var("x".to_string())),
            rhs: Arc::new(PnixExpr::Int(1)),
          },
        },
        PnixMatchArm {
          pattern: PnixPattern::Constructor {
            variant: "None".to_string(),
            args: vec![],
          },
          guard: None,
          body: PnixExpr::Int(0),
        },
      ],
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty(), "errors={:?}", result.errors);
    assert_eq!(
      result.var_types.get("_result"),
      Some(&CoreType::Named("Int".to_string()))
    );
  }

  #[test]
  fn test_infer_match_pattern_binding_guard() {
    use crate::lang::pnix::syntax::{PnixMatchArm, PnixPattern};
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Construct {
        variant: "Some".to_string(),
        args: vec![PnixExpr::Int(1)],
      }),
      arms: vec![
        PnixMatchArm {
          pattern: PnixPattern::Constructor {
            variant: "Some".to_string(),
            args: vec![PnixPattern::Var("x".to_string())],
          },
          guard: Some(Arc::new(PnixExpr::Binary {
            op: ">",
            lhs: Arc::new(PnixExpr::Var("x".to_string())),
            rhs: Arc::new(PnixExpr::Int(0)),
          })),
          body: PnixExpr::Int(1),
        },
        PnixMatchArm {
          pattern: PnixPattern::Constructor {
            variant: "None".to_string(),
            args: vec![],
          },
          guard: None,
          body: PnixExpr::Int(0),
        },
      ],
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty(), "errors={:?}", result.errors);
    assert_eq!(
      result.var_types.get("_result"),
      Some(&CoreType::Named("Int".to_string()))
    );
  }

  #[test]
  fn test_infer_match_pattern_binding_result() {
    use crate::lang::pnix::syntax::{PnixMatchArm, PnixPattern};
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Construct {
        variant: "Ok".to_string(),
        args: vec![PnixExpr::String("ok".to_string())],
      }),
      arms: vec![
        PnixMatchArm {
          pattern: PnixPattern::Constructor {
            variant: "Ok".to_string(),
            args: vec![PnixPattern::Var("x".to_string())],
          },
          guard: None,
          body: PnixExpr::Var("x".to_string()),
        },
        PnixMatchArm {
          pattern: PnixPattern::Constructor {
            variant: "Err".to_string(),
            args: vec![PnixPattern::Var("e".to_string())],
          },
          guard: None,
          body: PnixExpr::Var("e".to_string()),
        },
      ],
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty(), "errors={:?}", result.errors);
    assert_eq!(
      result.var_types.get("_result"),
      Some(&CoreType::Named("String".to_string()))
    );
  }

  #[test]
  fn test_infer_lambda() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Lambda {
      param: PnixParamPattern::Ident("x".to_string()),
      body: Arc::new(PnixExpr::Binary {
        op: "+",
        lhs: Arc::new(PnixExpr::Var("x".to_string())),
        rhs: Arc::new(PnixExpr::Int(1)),
      }),
    };
    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty());
    let ty = result.var_types.get("_lambda").unwrap();
    assert!(matches!(ty, CoreType::Arrow(_, _)));
  }

  #[test]
  fn test_infer_let_attrset_pattern_binds_fields() {
    use crate::lang::pnix::syntax::PnixAttrItem;
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Let {
      bindings: vec![PnixLetBinding::Binding {
        pattern: PnixParamPattern::AttrSet {
          fields: vec![
            PnixPatternField {
              name: "x".to_string(),
              default: None,
            },
            PnixPatternField {
              name: "y".to_string(),
              default: None,
            },
          ],
          ellipsis: false,
        },
        value: PnixExpr::AttrSet {
          items: vec![
            PnixAttrItem::Assign {
              key_path: vec!["x".to_string()],
              value: PnixExpr::Int(1),
              span: crate::diagnostics::Span::empty(),
            },
            PnixAttrItem::Assign {
              key_path: vec!["y".to_string()],
              value: PnixExpr::String("hi".to_string()),
              span: crate::diagnostics::Span::empty(),
            },
          ],
          recursive: false,
        },
      }],
      body: Arc::new(PnixExpr::Binary {
        op: "+",
        lhs: Arc::new(PnixExpr::Var("x".to_string())),
        rhs: Arc::new(PnixExpr::Int(1)),
      }),
    };

    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty(), "errors={:?}", result.errors);
    assert_eq!(
      result.var_types.get("x"),
      Some(&CoreType::Named("Int".to_string()))
    );
    assert_eq!(
      result.var_types.get("y"),
      Some(&CoreType::Named("String".to_string()))
    );
  }

  #[test]
  fn test_infer_let_attrset_pattern_default_allows_missing() {
    use crate::lang::pnix::syntax::PnixAttrItem;
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Let {
      bindings: vec![PnixLetBinding::Binding {
        pattern: PnixParamPattern::AttrSet {
          fields: vec![
            PnixPatternField {
              name: "x".to_string(),
              default: Some(PnixExpr::Int(1)),
            },
            PnixPatternField {
              name: "y".to_string(),
              default: None,
            },
          ],
          ellipsis: false,
        },
        value: PnixExpr::AttrSet {
          items: vec![PnixAttrItem::Assign {
            key_path: vec!["y".to_string()],
            value: PnixExpr::Bool(true),
            span: crate::diagnostics::Span::empty(),
          }],
          recursive: false,
        },
      }],
      body: Arc::new(PnixExpr::Var("x".to_string())),
    };

    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty(), "errors={:?}", result.errors);
    assert_eq!(
      result.var_types.get("x"),
      Some(&CoreType::Named("Int".to_string()))
    );
    assert_eq!(
      result.var_types.get("y"),
      Some(&CoreType::Named("Bool".to_string()))
    );
  }

  #[test]
  fn test_infer_let_list_pattern_binds_items() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Let {
      bindings: vec![PnixLetBinding::Binding {
        pattern: PnixParamPattern::List(PnixListPattern {
          items: vec!["x".to_string(), "y".to_string()],
          tail: None,
        }),
        value: PnixExpr::List(vec![PnixExpr::Int(1), PnixExpr::Int(2)]),
      }],
      body: Arc::new(PnixExpr::Binary {
        op: "+",
        lhs: Arc::new(PnixExpr::Var("x".to_string())),
        rhs: Arc::new(PnixExpr::Var("y".to_string())),
      }),
    };

    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty(), "errors={:?}", result.errors);
    assert_eq!(
      result.var_types.get("x"),
      Some(&CoreType::Named("Int".to_string()))
    );
    assert_eq!(
      result.var_types.get("y"),
      Some(&CoreType::Named("Int".to_string()))
    );
  }

  #[test]
  fn test_infer_lambda_attrset_pattern_bind_name() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Lambda {
      param: PnixParamPattern::AttrSetWithBind {
        bind_name: "args".to_string(),
        fields: vec![PnixPatternField {
          name: "x".to_string(),
          default: None,
        }],
        ellipsis: false,
      },
      body: Arc::new(PnixExpr::Select {
        base: Arc::new(PnixExpr::Var("args".to_string())),
        attr: "x".to_string(),
      }),
    };

    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty(), "errors={:?}", result.errors);
    let ty = result.var_types.get("_lambda").unwrap();
    match ty {
      CoreType::Arrow(param, _) => match param.as_ref() {
        CoreType::Record(fields) => {
          assert!(fields.iter().any(|(name, _)| name == "x"));
        }
        _ => panic!("Expected record param type, got {:?}", param),
      },
      _ => panic!("Expected arrow type, got {:?}", ty),
    }
  }

  #[test]
  fn test_infer_lambda_list_pattern_tail() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Lambda {
      param: PnixParamPattern::List(PnixListPattern {
        items: vec!["x".to_string()],
        tail: Some("rest".to_string()),
      }),
      body: Arc::new(PnixExpr::Var("rest".to_string())),
    };

    let result = inferencer.infer_expr(&expr);
    assert!(result.errors.is_empty(), "errors={:?}", result.errors);
    let ty = result.var_types.get("_lambda").unwrap();
    match ty {
      CoreType::Arrow(param, body) => {
        assert!(matches!(param.as_ref(), CoreType::List(_)));
        assert!(matches!(body.as_ref(), CoreType::List(_)));
      }
      _ => panic!("Expected arrow type, got {:?}", ty),
    }
  }

  #[test]
  fn test_infer_let_attrset_pattern_missing_field_is_error() {
    use crate::lang::pnix::syntax::PnixAttrItem;
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Let {
      bindings: vec![PnixLetBinding::Binding {
        pattern: PnixParamPattern::AttrSet {
          fields: vec![PnixPatternField {
            name: "x".to_string(),
            default: None,
          }],
          ellipsis: false,
        },
        value: PnixExpr::AttrSet {
          items: vec![PnixAttrItem::Assign {
            key_path: vec!["y".to_string()],
            value: PnixExpr::Int(1),
            span: crate::diagnostics::Span::empty(),
          }],
          recursive: false,
        },
      }],
      body: Arc::new(PnixExpr::Var("x".to_string())),
    };

    let result = inferencer.infer_expr(&expr);
    assert!(
      result.errors.iter().any(|err| matches!(
        err,
        InferenceError::UninferrableVar { name } if name.contains("attrset field 'x' missing")
      )),
      "errors={:?}",
      result.errors
    );
  }

  #[test]
  fn test_infer_let_attrset_pattern_non_record_is_error() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Let {
      bindings: vec![PnixLetBinding::Binding {
        pattern: PnixParamPattern::AttrSet {
          fields: vec![PnixPatternField {
            name: "x".to_string(),
            default: None,
          }],
          ellipsis: false,
        },
        value: PnixExpr::Int(1),
      }],
      body: Arc::new(PnixExpr::Var("x".to_string())),
    };

    let result = inferencer.infer_expr(&expr);
    assert!(
      result
        .errors
        .iter()
        .any(|err| matches!(err, InferenceError::UnificationFailed { .. })),
      "errors={:?}",
      result.errors
    );
  }

  #[test]
  fn test_infer_let_list_pattern_non_list_is_error() {
    let mut inferencer = TypeInferencer::new();
    let expr = PnixExpr::Let {
      bindings: vec![PnixLetBinding::Binding {
        pattern: PnixParamPattern::List(PnixListPattern {
          items: vec!["x".to_string(), "y".to_string()],
          tail: None,
        }),
        value: PnixExpr::Int(1),
      }],
      body: Arc::new(PnixExpr::Var("x".to_string())),
    };

    let result = inferencer.infer_expr(&expr);
    assert!(
      result
        .errors
        .iter()
        .any(|err| matches!(err, InferenceError::UnificationFailed { .. })),
      "errors={:?}",
      result.errors
    );
  }
}
