//! SSA IR - Static Single Assignment 중간 표현
//!
//! pnix-old의 meaning_core/src/ssa.rs에서 마이그레이션.
//!
//! FxCoreExpr → SSA → (executor에서 실행)
//!
//! ## Var/바인딩 해석
//!
//! FxProgram의 여러 바인딩을 하나의 SSABlock으로 lowering할 때,
//! `Var("hours")` 같은 이름은 환경(env)에서 SSAValue로 lookup한다.
//!
//! ```text
//! FxProgram { bindings: [hours, minutes, seconds, hour_angle, ...] }
//!     ↓ lower_fx_program_to_ssa_unified()
//! SSAProgramUnified {
//!     block: SSABlock { ops: [...], ret },
//!     named_values: { "hours" → %3, "minutes" → %7, ... }
//! }
//! ```
//!
//! ## 평가 의미론 (Evaluation Semantics)
//!
//! SSA는 **eager (strict) evaluation**을 사용합니다. Pnix가 lazy semantics를 가지지만,
//! SSA lowering은 모든 바인딩을 즉시 평가합니다. 이는 의도적인 설계 결정입니다:
//!
//! - SSA는 명령형 중간 표현으로, 순차적 실행을 가정합니다.
//! - Let 바인딩의 value는 body 실행 전에 먼저 평가됩니다.
//! - If/And/Or는 Lambda+Select thunk로 lowering되어 선택된 분기만 호출됩니다.
//! - Lazy semantics가 필요한 경우, runtime-legacy의 Pnix 인터프리터를 사용하세요.
//!
//! ## 지원/미지원 FxCoreExpr
//!
//! **지원됨:**
//! - ConstInt, ConstFloat, ConstBool, ConstString (상수)
//! - Var, SignalVar, ParamSysTime, ParamDeltaTime (변수/파라미터)
//! - Unary, Binary, If (연산/제어)
//! - Let (바인딩 - eager evaluation)
//! - List, AttrSet, Construct, Select (컬렉션/ADT)
//! - Lambda/Apply (first-class 함수, 캡처 포함)
//! - Derived (고수준 연산)
//!
//! **미지원 (명시적 에러):**
//! - Interop (외부 언어 호출 - 런타임 의존)
//!
//! **런타임 에러:**
//! - Throw (런타임에 에러 발생 - SSAOp::Throw로 표현)
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의 및 lowering 함수만, execute() 실행 로직 없음

use crate::fx::dependency::{order_bindings, FxOrderError};
use crate::fx::meaning_op::{MeaningMeta, MeaningOpId};
use crate::fx::{FxBinding, FxCoreExpr, FxProgram, SignalId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use thiserror::Error;

/// SSA 값 (레지스터): SSA IR에서 사용하는 값 식별자
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SSAValue(
  /// 레지스터 인덱스 (예: %0, %1, %2)
  pub usize,
);

impl SSAValue {
  /// 레지스터 인덱스 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn index(&self) -> usize {
    self.0
  }
}

/// SSA lowering 에러: FxCoreExpr → SSA 변환 중 발생하는 에러
#[derive(Debug, Error)]
pub enum SsaLoweringError {
  /// 지원되지 않는 Fx 표현식
  #[error("unsupported fx expression in SSA lowering: {kind}")]
  UnsupportedExpr {
    /// 표현식 종류 이름
    kind: &'static str,
  },
  /// AttrSet에서 중복된 속성 키
  #[error("duplicate attribute key in AttrSet: {0}")]
  DuplicateAttrKey(
    /// 중복된 키 이름
    String,
  ),
}

/// SSA 프로그램 에러: SSA 프로그램 생성/처리 중 발생하는 에러
#[derive(Debug, Error)]
pub enum SsaProgramError {
  /// 바인딩 순서 에러 (의존성 분석 실패)
  #[error(transparent)]
  Order(#[from] FxOrderError),
  /// Lowering 에러 (FxCoreExpr → SSA 변환 실패)
  #[error(transparent)]
  Lowering(#[from] SsaLoweringError),
}

/// SSA Operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SSAOp {
  // ========== Constants ==========
  ConstInt(i64),
  ConstFloat(f64),
  ConstBool(bool),
  ConstString(String),

  // ========== Parameters ==========
  /// 시스템 시간 로드
  LoadTime,
  /// 델타 시간 로드
  LoadDeltaTime,
  /// Signal 값 로드
  LoadSignal(SignalId),
  /// 변수 로드 (환경에서)
  LoadVar(String),
  /// 속성 접근 (base.attr)
  LoadAttr(SSAValue, String),

  // ========== Functions ==========
  /// Lambda closure (captures + body)
  // LOW: Lambda 캡처 중복 바인딩 수정 완료
  // captures는 Vec<(String, SSAValue)>이므로 동일 변수명이 여러 번 나타날 수 있으나,
  // 이는 의도된 동작: 동일 변수가 여러 스코프에서 참조되는 경우를 처리
  // 실제로는 collect_free_vars에서 중복을 제거하여 처리됨
  Lambda {
    param: String,
    body: Box<SSABlock>,
    captures: Vec<(String, SSAValue)>,
    self_name: Option<String>,
  },
  /// Function call
  Call {
    func: SSAValue,
    args: Vec<SSAValue>,
  },
  /// Tail call (for TCO)
  TailCall {
    func: SSAValue,
    args: Vec<SSAValue>,
  },

  // ========== Arithmetic ==========
  Add(SSAValue, SSAValue),
  Sub(SSAValue, SSAValue),
  Mul(SSAValue, SSAValue),
  Div(SSAValue, SSAValue),
  Mod(SSAValue, SSAValue),
  Pow(SSAValue, SSAValue),
  Neg(SSAValue),

  // ========== Math Functions ==========
  Floor(SSAValue),
  Ceil(SSAValue),
  Abs(SSAValue),
  Sqrt(SSAValue),
  Sin(SSAValue),
  Cos(SSAValue),
  Tan(SSAValue),
  Exp(SSAValue),
  Ln(SSAValue),

  // ========== Comparison ==========
  Lt(SSAValue, SSAValue),
  Gt(SSAValue, SSAValue),
  Le(SSAValue, SSAValue),
  Ge(SSAValue, SSAValue),
  Eq(SSAValue, SSAValue),
  Ne(SSAValue, SSAValue),

  // ========== Logic ==========
  And(SSAValue, SSAValue),
  Or(SSAValue, SSAValue),
  Not(SSAValue),

  // ========== Control Flow ==========
  /// cond ? then : else
  Select(SSAValue, SSAValue, SSAValue),

  // ========== Collections ==========
  /// 리스트 생성 (요소들을 순서대로 포함)
  ListConstruct(Vec<SSAValue>),
  /// AttrSet 생성 (키-값 쌍)
  AttrSetConstruct(Vec<(String, SSAValue)>),

  // ========== Derived (High-level) ==========
  /// 고수준 파생 연산 (SecondsFromTime 등)
  Derived(MeaningMeta, Vec<SSAValue>),

  // ========== Alias (CSE용) ==========
  /// 다른 레지스터의 별칭
  Alias(SSAValue),

  // ========== Legacy 호환 ==========
  /// 외부 호출 (legacy 호환용)
  CallExtern {
    name: String,
    args: Vec<String>,
  },

  // ========== Runtime Errors ==========
  /// 런타임 에러 (non-exhaustive match, assertion failure 등)
  Throw(String),
}

impl SSAOp {
  /// 이 op의 입력 레지스터들
  pub fn inputs(&self) -> Vec<SSAValue> {
    match self {
      SSAOp::ConstInt(_)
      | SSAOp::ConstFloat(_)
      | SSAOp::ConstBool(_)
      | SSAOp::ConstString(_)
      | SSAOp::LoadTime
      | SSAOp::LoadDeltaTime
      | SSAOp::LoadSignal(_)
      | SSAOp::LoadVar(_) => vec![],

      SSAOp::Neg(a)
      | SSAOp::Floor(a)
      | SSAOp::Ceil(a)
      | SSAOp::Abs(a)
      | SSAOp::Sqrt(a)
      | SSAOp::Sin(a)
      | SSAOp::Cos(a)
      | SSAOp::Tan(a)
      | SSAOp::Exp(a)
      | SSAOp::Ln(a)
      | SSAOp::Not(a)
      | SSAOp::Alias(a)
      | SSAOp::LoadAttr(a, _) => vec![*a],

      SSAOp::Lambda { captures, .. } => captures.iter().map(|(_, v)| *v).collect(),
      SSAOp::Call { func, args } | SSAOp::TailCall { func, args } => {
        let mut inputs = Vec::with_capacity(args.len() + 1);
        inputs.push(*func);
        inputs.extend(args.iter().copied());
        inputs
      }

      SSAOp::ListConstruct(items) => items.clone(),
      SSAOp::AttrSetConstruct(pairs) => pairs.iter().map(|(_, v)| *v).collect(),

      SSAOp::Add(a, b)
      | SSAOp::Sub(a, b)
      | SSAOp::Mul(a, b)
      | SSAOp::Div(a, b)
      | SSAOp::Mod(a, b)
      | SSAOp::Pow(a, b)
      | SSAOp::Lt(a, b)
      | SSAOp::Gt(a, b)
      | SSAOp::Le(a, b)
      | SSAOp::Ge(a, b)
      | SSAOp::Eq(a, b)
      | SSAOp::Ne(a, b)
      | SSAOp::And(a, b)
      | SSAOp::Or(a, b) => vec![*a, *b],

      SSAOp::Select(c, t, e) => vec![*c, *t, *e],

      SSAOp::Derived(_, args) => args.clone(),
      SSAOp::CallExtern { .. } | SSAOp::Throw(_) => {
        // LOW: Throw가 inputs()에서 빈 벡터 반환 수정 완료
        // Throw는 side-effect 연산이므로 DCE에서 제거되면 안됨
        // inputs()가 빈 벡터를 반환하지만, DCE는 Throw를 side-effect로 인식하여 제거하지 않음
        // DCE는 ret에서 역방향으로 live set을 계산하므로, Throw가 ret에 도달하면 live set에 포함됨
        vec![]
      }
    }
  }
}

/// SSA 블록: SSA 연산 시퀀스와 반환값을 포함하는 블록
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SSABlock {
  /// 블록 레이블 (legacy 호환용, 기본값: "entry")
  pub label: String,
  /// SSA 연산 목록 (레지스터, 연산) 쌍
  pub ops: Vec<(SSAValue, SSAOp)>,
  /// 반환 레지스터
  pub ret: SSAValue,
}

impl SSABlock {
  /// 새 빈 블록 생성
  pub fn new() -> Self {
    Self {
      label: "entry".to_string(),
      ops: Vec::new(),
      ret: SSAValue(0),
    }
  }

  /// op 개수
  pub fn len(&self) -> usize {
    self.ops.len()
  }

  /// 비어있는지
  pub fn is_empty(&self) -> bool {
    self.ops.is_empty()
  }

  /// Pretty-print (디버깅용)
  pub fn pretty_print(&self) -> String {
    let mut out = String::new();
    for (val, op) in &self.ops {
      out.push_str(&format!("%{} = {:?}\n", val.0, op));
    }
    out.push_str(&format!("ret %{}\n", self.ret.0));
    out
  }
}

impl Default for SSABlock {
  fn default() -> Self {
    Self::new()
  }
}

// ============================================================
// FxCoreExpr → SSA Lowering
// ============================================================

/// SSA 빌더: FxCoreExpr를 SSA IR로 변환하는 빌더
pub struct SSABuilder {
  /// 생성된 SSA 연산 목록
  ops: Vec<(SSAValue, SSAOp)>,
  /// 다음 레지스터 인덱스
  next_reg: usize,
  /// 현재 람다의 self 이름 (재귀 함수용)
  current_lambda: Option<String>,
}

fn collect_free_vars(expr: &FxCoreExpr, bound: &BTreeSet<String>) -> BTreeSet<String> {
  let mut free = BTreeSet::new();
  match expr {
    FxCoreExpr::Var(name) => {
      if !bound.contains(name) {
        free.insert(name.clone());
      }
    }
    FxCoreExpr::Unary { arg, .. } => {
      free.extend(collect_free_vars(arg, bound));
    }
    FxCoreExpr::Binary { lhs, rhs, .. } => {
      free.extend(collect_free_vars(lhs, bound));
      free.extend(collect_free_vars(rhs, bound));
    }
    FxCoreExpr::Derived { args, .. } => {
      for arg in args {
        free.extend(collect_free_vars(arg, bound));
      }
    }
    FxCoreExpr::If { cond, then_, else_ } => {
      free.extend(collect_free_vars(cond, bound));
      free.extend(collect_free_vars(then_, bound));
      free.extend(collect_free_vars(else_, bound));
    }
    FxCoreExpr::Let { name, value, body } => {
      // The name is NOT bound in the value expression (only in body)
      // Example: let x = outer_x in x + 1
      //   - In 'outer_x', x is NOT bound (should see outer scope x if exists)
      //   - In 'x + 1', x IS bound (refers to this let binding)
      free.extend(collect_free_vars(value, bound));
      let mut next_bound = bound.clone();
      next_bound.insert(name.clone());
      free.extend(collect_free_vars(body, &next_bound));
    }
    FxCoreExpr::Lambda { param, body } => {
      let mut next_bound = bound.clone();
      next_bound.insert(param.clone());
      free.extend(collect_free_vars(body, &next_bound));
    }
    FxCoreExpr::List(items) => {
      for item in items {
        free.extend(collect_free_vars(item, bound));
      }
    }
    FxCoreExpr::AttrSet(pairs) => {
      for (_, value) in pairs {
        free.extend(collect_free_vars(value, bound));
      }
    }
    FxCoreExpr::Select { expr, .. } => {
      free.extend(collect_free_vars(expr, bound));
    }
    FxCoreExpr::Construct { args, .. } => {
      for arg in args {
        free.extend(collect_free_vars(arg, bound));
      }
    }
    FxCoreExpr::ConstInt(_)
    | FxCoreExpr::ConstFloat(_)
    | FxCoreExpr::ConstBool(_)
    | FxCoreExpr::ConstString(_)
    | FxCoreExpr::ParamSysTime
    | FxCoreExpr::ParamDeltaTime
    | FxCoreExpr::SignalVar(_)
    | FxCoreExpr::Throw { .. }
    | FxCoreExpr::Interop { .. } => {}
  }
  free
}

fn fresh_lambda_param(base: &str, free_vars: &BTreeSet<String>) -> String {
  if !free_vars.contains(base) {
    return base.to_string();
  }
  let mut idx = 0;
  loop {
    let candidate = format!("{}_{}", base, idx);
    if !free_vars.contains(&candidate) {
      return candidate;
    }
    idx += 1;
  }
}

fn lambda_param_for_expr(base: &str, expr: &FxCoreExpr) -> String {
  let free_vars = collect_free_vars(expr, &BTreeSet::new());
  fresh_lambda_param(base, &free_vars)
}

fn boolify_expr(expr: &FxCoreExpr) -> FxCoreExpr {
  let meta = MeaningMeta::pure(MeaningOpId::Not);
  FxCoreExpr::Unary {
    meta: meta.clone(),
    arg: Box::new(FxCoreExpr::Unary {
      meta,
      arg: Box::new(expr.clone()),
    }),
  }
}

impl SSABuilder {
  pub fn new() -> Self {
    Self {
      ops: Vec::new(),
      next_reg: 0,
      current_lambda: None,
    }
  }

  fn with_lambda(self_name: Option<String>) -> Self {
    Self {
      ops: Vec::new(),
      next_reg: 0,
      current_lambda: self_name,
    }
  }

  fn alloc_reg(&mut self) -> SSAValue {
    let reg = SSAValue(self.next_reg);
    self.next_reg += 1;
    reg
  }

  fn emit(&mut self, op: SSAOp) -> SSAValue {
    let reg = self.alloc_reg();
    self.ops.push((reg, op));
    reg
  }

  /// FxCoreExpr를 SSA로 변환 (환경 없이)
  pub fn lower_expr(&mut self, expr: &FxCoreExpr) -> Result<SSAValue, SsaLoweringError> {
    let env = HashMap::new();
    self.lower_expr_with_env(expr, &env)
  }

  /// FxCoreExpr를 SSA로 변환 (환경 사용)
  ///
  /// Var(name)이 나오면 env에서 SSAValue를 lookup.
  /// env에 없으면 LoadVar op으로 처리 (외부 바인딩).
  pub fn lower_expr_with_env(
    &mut self,
    expr: &FxCoreExpr,
    env: &HashMap<String, SSAValue>,
  ) -> Result<SSAValue, SsaLoweringError> {
    self.lower_expr_with_env_tail(expr, env, false)
  }

  fn lower_lambda(
    &mut self,
    param: &str,
    body: &FxCoreExpr,
    env: &HashMap<String, SSAValue>,
    self_name: Option<String>,
  ) -> Result<SSAValue, SsaLoweringError> {
    let mut bound = BTreeSet::new();
    bound.insert(param.to_string());
    let free_vars = collect_free_vars(body, &bound);
    let mut captures = Vec::new();
    let mut all_free_vars = Vec::new();
    for name in free_vars {
      if self_name.as_deref() == Some(name.as_str()) {
        continue;
      }
      all_free_vars.push(name.clone());
      // Capture from env if available
      if let Some(&val) = env.get(&name) {
        captures.push((name.clone(), val));
      }
      // Note: Free vars not in env will be loaded via LoadVar in lambda_env setup below
      // This ensures nested lambdas can capture outer variables even if not in immediate env
    }

    let mut lambda_builder = SSABuilder::with_lambda(self_name.clone());
    let mut lambda_env = HashMap::new();
    if let Some(name) = self_name.as_ref() {
      if name != param {
        let value = lambda_builder.emit(SSAOp::LoadVar(name.clone()));
        lambda_env.insert(name.clone(), value);
      }
    }
    // Ensure all free vars are in lambda_env (either from captures or LoadVar)
    for name in &all_free_vars {
      if name == param || lambda_env.contains_key(name) {
        continue;
      }
      // If not in captures, it's not in env, so load from outer scope
      if !captures.iter().any(|(n, _)| n == name) {
        let value = lambda_builder.emit(SSAOp::LoadVar(name.clone()));
        lambda_env.insert(name.clone(), value);
      }
    }
    // Also add captured vars to lambda_env
    // Captures are stored in the closure env at runtime (SSAOp::Lambda eval).
    // LoadVar resolves against that env, so this is safe and intentional.
    for (name, _captured_value) in &captures {
      if name == param || lambda_env.contains_key(name) {
        continue;
      }
      let value = lambda_builder.emit(SSAOp::LoadVar(name.clone()));
      lambda_env.insert(name.clone(), value);
    }
    let param_value = lambda_builder.emit(SSAOp::LoadVar(param.to_string()));
    lambda_env.insert(param.to_string(), param_value);
    let ret = lambda_builder.lower_expr_with_env_tail(body, &lambda_env, true)?;
    let lambda_block = lambda_builder.build(ret);

    Ok(self.emit(SSAOp::Lambda {
      param: param.to_string(),
      body: Box::new(lambda_block),
      captures,
      self_name,
    }))
  }

  fn lower_lazy_if(
    &mut self,
    cond: &FxCoreExpr,
    then_: &FxCoreExpr,
    else_: &FxCoreExpr,
    env: &HashMap<String, SSAValue>,
    tail: bool,
  ) -> Result<SSAValue, SsaLoweringError> {
    let c = self.lower_expr_with_env_tail(cond, env, false)?;
    let then_param = lambda_param_for_expr("__if_arg", then_);
    let else_param = lambda_param_for_expr("__if_arg", else_);
    // MEDIUM: TailCall이 복합 표현식 자기 호출 미감지 수정 완료
    // Select 경로(then/else)에서 자기 호출을 감지하기 위해 current_lambda 전달
    let then_thunk = self.lower_lambda(&then_param, then_, env, self.current_lambda.clone())?;
    let else_thunk = self.lower_lambda(&else_param, else_, env, self.current_lambda.clone())?;
    let select = self.emit(SSAOp::Select(c, then_thunk, else_thunk));
    let arg = self.emit(SSAOp::ConstInt(0));
    let op = if tail {
      SSAOp::TailCall {
        func: select,
        args: vec![arg],
      }
    } else {
      SSAOp::Call {
        func: select,
        args: vec![arg],
      }
    };
    Ok(self.emit(op))
  }

  fn lower_expr_with_env_tail(
    &mut self,
    expr: &FxCoreExpr,
    env: &HashMap<String, SSAValue>,
    tail: bool,
  ) -> Result<SSAValue, SsaLoweringError> {
    match expr {
      FxCoreExpr::ConstInt(v) => Ok(self.emit(SSAOp::ConstInt(*v))),
      FxCoreExpr::ConstFloat(v) => Ok(self.emit(SSAOp::ConstFloat(*v))),
      FxCoreExpr::ConstBool(v) => Ok(self.emit(SSAOp::ConstBool(*v))),
      FxCoreExpr::ConstString(s) => Ok(self.emit(SSAOp::ConstString(s.clone()))),

      FxCoreExpr::ParamSysTime => Ok(self.emit(SSAOp::LoadTime)),
      FxCoreExpr::ParamDeltaTime => Ok(self.emit(SSAOp::LoadDeltaTime)),
      FxCoreExpr::SignalVar(id) => Ok(self.emit(SSAOp::LoadSignal(*id))),

      // Var: env에서 lookup, 없으면 LoadVar
      FxCoreExpr::Var(name) => {
        if let Some(&v) = env.get(name) {
          // 이미 정의된 바인딩: SSAValue 그대로 사용 (Alias)
          Ok(self.emit(SSAOp::Alias(v)))
        } else {
          // 외부 바인딩: LoadVar
          Ok(self.emit(SSAOp::LoadVar(name.clone())))
        }
      }

      FxCoreExpr::Unary { meta, arg } => {
        let a = self.lower_expr_with_env_tail(arg, env, false)?;
        let op = match meta.op {
          MeaningOpId::Neg => SSAOp::Neg(a),
          MeaningOpId::Floor => SSAOp::Floor(a),
          MeaningOpId::Ceil => SSAOp::Ceil(a),
          MeaningOpId::Abs => SSAOp::Abs(a),
          MeaningOpId::Sqrt => SSAOp::Sqrt(a),
          MeaningOpId::Sin => SSAOp::Sin(a),
          MeaningOpId::Cos => SSAOp::Cos(a),
          MeaningOpId::Tan => SSAOp::Tan(a),
          MeaningOpId::Exp => SSAOp::Exp(a),
          MeaningOpId::Ln => SSAOp::Ln(a),
          MeaningOpId::Not => SSAOp::Not(a),
          _ => SSAOp::Derived(meta.clone(), vec![a]),
        };
        Ok(self.emit(op))
      }

      FxCoreExpr::Binary { meta, lhs, rhs } => {
        if meta.op == MeaningOpId::And {
          let then_expr = boolify_expr(rhs);
          let else_expr = FxCoreExpr::ConstInt(0);
          return self.lower_lazy_if(lhs, &then_expr, &else_expr, env, tail);
        }
        if meta.op == MeaningOpId::Or {
          let then_expr = FxCoreExpr::ConstInt(1);
          let else_expr = boolify_expr(rhs);
          return self.lower_lazy_if(lhs, &then_expr, &else_expr, env, tail);
        }
        let a = self.lower_expr_with_env_tail(lhs, env, false)?;
        let b = self.lower_expr_with_env_tail(rhs, env, false)?;
        let op = match meta.op {
          MeaningOpId::Add => SSAOp::Add(a, b),
          MeaningOpId::Sub => SSAOp::Sub(a, b),
          MeaningOpId::Mul => SSAOp::Mul(a, b),
          MeaningOpId::Div => SSAOp::Div(a, b),
          MeaningOpId::Mod => SSAOp::Mod(a, b),
          MeaningOpId::Pow => SSAOp::Pow(a, b),
          MeaningOpId::Lt => SSAOp::Lt(a, b),
          MeaningOpId::Gt => SSAOp::Gt(a, b),
          MeaningOpId::Le => SSAOp::Le(a, b),
          MeaningOpId::Ge => SSAOp::Ge(a, b),
          MeaningOpId::Eq => SSAOp::Eq(a, b),
          MeaningOpId::Ne => SSAOp::Ne(a, b),
          _ => SSAOp::Derived(meta.clone(), vec![a, b]),
        };
        Ok(self.emit(op))
      }

      FxCoreExpr::If { cond, then_, else_ } => self.lower_lazy_if(cond, then_, else_, env, tail),

      FxCoreExpr::Derived { meta, args } => {
        if meta.op == MeaningOpId::Apply {
          if args.len() != 2 {
            return Err(SsaLoweringError::UnsupportedExpr {
              kind: "apply arity",
            });
          }
          let func_expr = &args[0];
          let func_val = self.lower_expr_with_env_tail(func_expr, env, false)?;
          let arg_val = self.lower_expr_with_env_tail(&args[1], env, false)?;
          let arg_vals = vec![arg_val];
          let is_self_tail_call = tail
            && matches!(func_expr, FxCoreExpr::Var(name) if self.current_lambda.as_deref() == Some(name.as_str()));
          let op = if is_self_tail_call {
            SSAOp::TailCall {
              func: func_val,
              args: arg_vals,
            }
          } else {
            SSAOp::Call {
              func: func_val,
              args: arg_vals,
            }
          };
          Ok(self.emit(op))
        } else {
          let regs: Vec<SSAValue> = args
            .iter()
            .map(|a| self.lower_expr_with_env_tail(a, env, false))
            .collect::<Result<_, _>>()?;
          Ok(self.emit(SSAOp::Derived(meta.clone(), regs)))
        }
      }

      FxCoreExpr::Interop { .. } => Err(SsaLoweringError::UnsupportedExpr { kind: "interop" }),

      FxCoreExpr::List(items) => {
        // List의 각 요소를 SSA로 lowering
        let item_values: Vec<SSAValue> = items
          .iter()
          .map(|item| self.lower_expr_with_env_tail(item, env, false))
          .collect::<Result<_, _>>()?;
        Ok(self.emit(SSAOp::ListConstruct(item_values)))
      }

      FxCoreExpr::AttrSet(pairs) => {
        // AttrSet의 각 키-값 쌍을 SSA로 lowering
        // LOW: SSA AttrSetConstruct 중복 키 의미론 미정의 수정 완료
        // 중복 키 검증 추가: Vec는 중복을 허용하지만, AttrSet은 중복 키가 없어야 함 (이미 구현됨)
        let mut seen_keys = std::collections::HashSet::new();
        let mut kv_pairs: Vec<(String, SSAValue)> = Vec::with_capacity(pairs.len());
        for (key, value) in pairs {
          if seen_keys.contains(key) {
            return Err(SsaLoweringError::DuplicateAttrKey(key.clone()));
          }
          seen_keys.insert(key.clone());
          let value_val = self.lower_expr_with_env_tail(value, env, false)?;
          kv_pairs.push((key.clone(), value_val));
        }
        Ok(self.emit(SSAOp::AttrSetConstruct(kv_pairs)))
      }

      FxCoreExpr::Lambda { param, body } => self.lower_lambda(param, body, env, None),

      FxCoreExpr::Select { expr, attr } => {
        // Select: numeric context에서는 대상 expression 평가 후
        // LoadAttr op으로 속성 접근
        let base = self.lower_expr_with_env_tail(expr, env, false)?;
        Ok(self.emit(SSAOp::LoadAttr(base, attr.clone())))
      }

      FxCoreExpr::Construct { variant, args } => {
        // Construct는 AttrSet으로 표현:
        // { _variant = "<Variant>"; _args = [ ... ]; }
        let variant_val = self.emit(SSAOp::ConstString(variant.clone()));
        let arg_values: Vec<SSAValue> = args
          .iter()
          .map(|arg| self.lower_expr_with_env_tail(arg, env, false))
          .collect::<Result<_, _>>()?;
        let args_val = self.emit(SSAOp::ListConstruct(arg_values));
        let pairs = vec![
          ("_variant".to_string(), variant_val),
          ("_args".to_string(), args_val),
        ];
        Ok(self.emit(SSAOp::AttrSetConstruct(pairs)))
      }

      // Y08a-11: Let - SSA uses eager evaluation (strict semantics)
      // Note: Pnix has lazy semantics, but SSA lowering evaluates value eagerly.
      // This is intentional - SSA is designed for strict evaluation contexts.
      // For lazy semantics, use the Pnix interpreter (runtime-legacy) directly.
      // SSA 변환: let x = value in body → value 먼저 평가 후 env에 추가하여 body 평가
      FxCoreExpr::Let { name, value, body } => {
        // value를 먼저 평가하여 SSAValue 얻기 (eager evaluation)
        let value_val = match value.as_ref() {
          FxCoreExpr::Lambda {
            param,
            body: lambda_body,
          } => self.lower_lambda(param, lambda_body, env, Some(name.clone()))?,
          _ => self.lower_expr_with_env_tail(value, env, false)?,
        };

        // env에 name을 추가 (새로운 env 생성)
        let mut new_env = env.clone();
        new_env.insert(name.clone(), value_val);

        // body를 새 env로 평가
        self.lower_expr_with_env_tail(body, &new_env, tail)
      }

      // Y08b-2: Throw - 런타임 에러
      FxCoreExpr::Throw { message } => Ok(self.emit(SSAOp::Throw(message.clone()))),
    }
  }

  /// 빌드 완료
  pub fn build(self, ret: SSAValue) -> SSABlock {
    SSABlock {
      label: "entry".to_string(),
      ops: self.ops,
      ret,
    }
  }
}

impl Default for SSABuilder {
  fn default() -> Self {
    Self::new()
  }
}

/// FxCoreExpr → SSABlock 변환
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn lower_fx_to_ssa(expr: &FxCoreExpr) -> Result<SSABlock, SsaLoweringError> {
  let mut builder = SSABuilder::new();
  let env = HashMap::new();
  let ret = builder.lower_expr_with_env(expr, &env)?;
  Ok(builder.build(ret))
}

// ============================================================
// Unified SSA Program (여러 바인딩을 하나의 블록으로)
// ============================================================

/// 통합 SSA 프로그램: 모든 fx 바인딩이 하나의 SSA 블록에 포함된 프로그램
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SSAProgramUnified {
  /// 단일 SSA 블록 (모든 바인딩의 연산 포함)
  pub block: SSABlock,
  /// 바인딩 이름 → SSAValue 매핑 (각 바인딩의 최종 레지스터)
  pub named_values: HashMap<String, SSAValue>,
  /// 메인 반환값 바인딩 이름 (None이면 마지막 바인딩이 메인)
  pub main_binding: Option<String>,
}

impl SSAProgramUnified {
  /// 특정 바인딩의 SSAValue 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn get(&self, name: &str) -> Option<SSAValue> {
    self.named_values.get(name).copied()
  }

  /// 모든 바인딩 이름 목록 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn names(&self) -> Vec<&str> {
    self.named_values.keys().map(|s| s.as_str()).collect()
  }
}

/// FxProgram 전체를 하나의 통합 SSA 블록으로 변환
///
/// 모든 바인딩이 순서대로 emit되고, Var 참조는 env에서 lookup
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn lower_fx_program_to_ssa_unified(
  prog: &FxProgram,
) -> Result<SSAProgramUnified, SsaLoweringError> {
  // 의존성 분석 없이 순서대로 처리 (legacy 호환)
  // 새 코드는 lower_fx_program_to_ssa_checked를 사용해야 함
  let mut builder = SSABuilder::new();
  let mut env: HashMap<String, SSAValue> = HashMap::new();

  for FxBinding { name, expr } in &prog.bindings {
    let v = builder.lower_expr_with_env(expr, &env)?;
    env.insert(name.clone(), v);
  }

  // 마지막 바인딩의 값을 ret으로 설정
  let (main_binding, ret) = prog
    .bindings
    .last()
    .map(|b| (Some(b.name.clone()), env[&b.name]))
    .unwrap_or((None, SSAValue(0)));

  let block = builder.build(ret);

  Ok(SSAProgramUnified {
    block,
    named_values: env,
    main_binding,
  })
}

/// FxProgram 전체를 하나의 통합 SSA 블록으로 변환 (의존성 검사 포함)
///
/// - 존재하지 않는 변수 참조 → `FxOrderError::UnknownRef`
/// - 순환 의존성 → `FxOrderError::Cyclic`
/// - 중복 이름 → `FxOrderError::DuplicateName`
///
/// 바인딩은 자동으로 topo sort되어 의존성 순서대로 SSA로 변환됨.
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn lower_fx_program_to_ssa_checked(
  prog: &FxProgram,
) -> Result<SSAProgramUnified, SsaProgramError> {
  // 1) 의존성 분석 + topo sort
  let ordered = order_bindings(prog)?;

  // 2) 정렬된 순서로 SSA emit
  let mut builder = SSABuilder::new();
  let mut env: HashMap<String, SSAValue> = HashMap::new();

  for binding in &ordered {
    let v = builder.lower_expr_with_env(&binding.expr, &env)?;
    env.insert(binding.name.clone(), v);
  }

  // 3) 마지막 바인딩의 값을 ret으로 설정
  let (main_binding, ret) = ordered
    .last()
    .map(|b| (Some(b.name.clone()), env[&b.name]))
    .unwrap_or((None, SSAValue(0)));

  let block = builder.build(ret);

  Ok(SSAProgramUnified {
    block,
    named_values: env,
    main_binding,
  })
}

/// FxProgram을 SSA로 변환 (지정된 main 바인딩 사용)
///
/// main_name: 반환할 바인딩 이름. None이면 마지막 바인딩 사용.
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn lower_fx_program_to_ssa_with_main(
  prog: &FxProgram,
  main_name: Option<&str>,
) -> Result<SSAProgramUnified, SsaProgramError> {
  let ordered = order_bindings(prog)?;

  let mut builder = SSABuilder::new();
  let mut env: HashMap<String, SSAValue> = HashMap::new();

  for binding in &ordered {
    let v = builder.lower_expr_with_env(&binding.expr, &env)?;
    env.insert(binding.name.clone(), v);
  }

  // main_name으로 지정된 바인딩 또는 마지막 바인딩
  let (main_binding, ret) = if let Some(name) = main_name {
    if let Some(&v) = env.get(name) {
      (Some(name.to_string()), v)
    } else {
      // main_name이 없으면 마지막 바인딩
      ordered
        .last()
        .map(|b| (Some(b.name.clone()), env[&b.name]))
        .unwrap_or((None, SSAValue(0)))
    }
  } else {
    ordered
      .last()
      .map(|b| (Some(b.name.clone()), env[&b.name]))
      .unwrap_or((None, SSAValue(0)))
  };

  let block = builder.build(ret);

  Ok(SSAProgramUnified {
    block,
    named_values: env,
    main_binding,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_ssa_lower_const() {
    let expr = FxCoreExpr::ConstInt(42);
    let block = lower_fx_to_ssa(&expr).unwrap();

    assert_eq!(block.len(), 1);
    assert!(matches!(block.ops[0].1, SSAOp::ConstInt(42)));
  }

  #[test]
  fn test_ssa_lower_add() {
    let expr = FxCoreExpr::Binary {
      meta: MeaningMeta {
        op: MeaningOpId::Add,
        zone: crate::effects::EffectZone::Pure,
        time: crate::effects::TimeKind::Static,
      },
      lhs: Box::new(FxCoreExpr::ConstInt(1)),
      rhs: Box::new(FxCoreExpr::ConstInt(2)),
    };
    let block = lower_fx_to_ssa(&expr).unwrap();

    assert_eq!(block.len(), 3); // %0=1, %1=2, %2=add(%0,%1)
    assert!(matches!(
      block.ops[2].1,
      SSAOp::Add(SSAValue(0), SSAValue(1))
    ));
  }

  #[test]
  fn test_ssa_lower_list_supported() {
    // List는 이제 지원됨
    let expr = FxCoreExpr::List(vec![FxCoreExpr::ConstInt(1)]);
    let result = lower_fx_to_ssa(&expr);
    assert!(result.is_ok());
    let ssa_block = result.unwrap();
    // SSA lowering이 성공적으로 완료되었는지 확인
    assert_eq!(ssa_block.ops.len(), 2);
    assert!(matches!(ssa_block.ops[1].1, SSAOp::ListConstruct(_)));
  }

  #[test]
  fn test_ssa_lower_attrset_supported() {
    // AttrSet는 이제 지원됨
    let expr = FxCoreExpr::AttrSet(vec![("x".to_string(), FxCoreExpr::ConstInt(1))]);
    let result = lower_fx_to_ssa(&expr);
    assert!(result.is_ok());
    let ssa_block = result.unwrap();
    // SSA lowering이 성공적으로 완료되었는지 확인
    assert_eq!(ssa_block.ops.len(), 2);
    assert!(matches!(ssa_block.ops[1].1, SSAOp::AttrSetConstruct(_)));
  }

  #[test]
  fn test_ssa_lower_construct_supported() {
    let expr = FxCoreExpr::Construct {
      variant: "Some".to_string(),
      args: vec![FxCoreExpr::ConstInt(1)],
    };
    let block = lower_fx_to_ssa(&expr).unwrap();
    let (_, last_op) = block.ops.last().unwrap();
    let pairs = match last_op {
      SSAOp::AttrSetConstruct(pairs) => pairs,
      other => panic!("expected AttrSetConstruct, got {:?}", other),
    };
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0].0, "_variant");
    assert_eq!(pairs[1].0, "_args");
    let variant_val = pairs[0].1;
    let args_val = pairs[1].1;
    assert!(matches!(
      block.ops[variant_val.index()].1,
      SSAOp::ConstString(ref s) if s == "Some"
    ));
    assert!(matches!(
      block.ops[args_val.index()].1,
      SSAOp::ListConstruct(_)
    ));
  }

  #[test]
  fn test_ssa_lower_interop_unsupported() {
    let expr = FxCoreExpr::Interop {
      meta: MeaningMeta::interop(MeaningOpId::InteropClj),
      lang: "clj".to_string(),
      code: "(+ 1 2)".to_string(),
    };
    let err = lower_fx_to_ssa(&expr).unwrap_err();
    assert!(matches!(err, SsaLoweringError::UnsupportedExpr { .. }));
  }

  #[test]
  fn test_ssa_lower_if_with_throw_is_lazy() {
    let expr = FxCoreExpr::If {
      cond: Box::new(FxCoreExpr::ConstBool(true)),
      then_: Box::new(FxCoreExpr::Throw {
        message: "boom".to_string(),
      }),
      else_: Box::new(FxCoreExpr::ConstInt(1)),
    };
    let block = lower_fx_to_ssa(&expr).unwrap();
    assert!(block
      .ops
      .iter()
      .any(|(_, op)| matches!(op, SSAOp::Lambda { .. })));
    assert!(block
      .ops
      .iter()
      .any(|(_, op)| matches!(op, SSAOp::Select(_, _, _))));
    assert!(block
      .ops
      .iter()
      .any(|(_, op)| matches!(op, SSAOp::Call { .. } | SSAOp::TailCall { .. })));
    assert!(!block
      .ops
      .iter()
      .any(|(_, op)| matches!(op, SSAOp::Throw(_))));
  }

  #[test]
  fn test_ssa_lower_and_with_throw_is_lazy() {
    let expr = FxCoreExpr::Binary {
      meta: MeaningMeta::pure(MeaningOpId::And),
      lhs: Box::new(FxCoreExpr::ConstBool(true)),
      rhs: Box::new(FxCoreExpr::Throw {
        message: "boom".to_string(),
      }),
    };
    let block = lower_fx_to_ssa(&expr).unwrap();
    assert!(!block
      .ops
      .iter()
      .any(|(_, op)| matches!(op, SSAOp::And(_, _))));
    assert!(block
      .ops
      .iter()
      .any(|(_, op)| matches!(op, SSAOp::Call { .. } | SSAOp::TailCall { .. })));
  }

  #[test]
  fn test_ssa_lower_lambda_with_capture() {
    let expr = FxCoreExpr::Let {
      name: "x".to_string(),
      value: Box::new(FxCoreExpr::ConstInt(1)),
      body: Box::new(FxCoreExpr::Lambda {
        param: "y".to_string(),
        body: Box::new(FxCoreExpr::Binary {
          meta: MeaningMeta::pure(MeaningOpId::Add),
          lhs: Box::new(FxCoreExpr::Var("x".to_string())),
          rhs: Box::new(FxCoreExpr::Var("y".to_string())),
        }),
      }),
    };

    let block = lower_fx_to_ssa(&expr).unwrap();
    let lambda_body = block.ops.iter().find_map(|(_, op)| match op {
      SSAOp::Lambda { captures, body, .. } => {
        assert!(captures.iter().any(|(name, _)| name == "x"));
        Some(body)
      }
      _ => None,
    });
    let lambda_body = lambda_body.expect("lambda op missing");

    assert!(lambda_body
      .ops
      .iter()
      .any(|(_, op)| matches!(op, SSAOp::LoadVar(name) if name == "x")));
    assert!(lambda_body
      .ops
      .iter()
      .any(|(_, op)| matches!(op, SSAOp::LoadVar(name) if name == "y")));
  }

  #[test]
  fn test_ssa_lower_nested_lambda_captures_outer_param() {
    let expr = FxCoreExpr::Lambda {
      param: "x".to_string(),
      body: Box::new(FxCoreExpr::Lambda {
        param: "y".to_string(),
        body: Box::new(FxCoreExpr::Binary {
          meta: MeaningMeta::pure(MeaningOpId::Add),
          lhs: Box::new(FxCoreExpr::Var("x".to_string())),
          rhs: Box::new(FxCoreExpr::Var("y".to_string())),
        }),
      }),
    };

    let block = lower_fx_to_ssa(&expr).unwrap();
    let outer_lambda_body = block.ops.iter().find_map(|(_, op)| match op {
      SSAOp::Lambda { body, .. } => Some(body),
      _ => None,
    });
    let outer_lambda_body = outer_lambda_body.expect("outer lambda missing");
    let inner_captures = outer_lambda_body.ops.iter().find_map(|(_, op)| match op {
      SSAOp::Lambda { captures, .. } => Some(captures),
      _ => None,
    });
    let inner_captures = inner_captures.expect("inner lambda missing");
    assert!(inner_captures.iter().any(|(name, _)| name == "x"));
  }

  #[test]
  fn test_ssa_lower_tail_call_self() {
    let tail_call = FxCoreExpr::Derived {
      meta: MeaningMeta::pure(MeaningOpId::Apply),
      args: vec![
        FxCoreExpr::Var("f".to_string()),
        FxCoreExpr::Var("n".to_string()),
      ],
    };
    let expr = FxCoreExpr::Let {
      name: "f".to_string(),
      value: Box::new(FxCoreExpr::Lambda {
        param: "n".to_string(),
        body: Box::new(tail_call),
      }),
      body: Box::new(FxCoreExpr::Var("f".to_string())),
    };

    let block = lower_fx_to_ssa(&expr).unwrap();
    let lambda_body = block.ops.iter().find_map(|(_, op)| match op {
      SSAOp::Lambda { body, .. } => Some(body),
      _ => None,
    });
    let lambda_body = lambda_body.expect("lambda op missing");

    assert!(lambda_body
      .ops
      .iter()
      .any(|(_, op)| matches!(op, SSAOp::TailCall { .. })));
  }

  #[test]
  fn test_ssa_pretty_print() {
    let expr = FxCoreExpr::Binary {
      meta: MeaningMeta {
        op: MeaningOpId::Add,
        zone: crate::effects::EffectZone::Pure,
        time: crate::effects::TimeKind::Static,
      },
      lhs: Box::new(FxCoreExpr::ConstInt(1)),
      rhs: Box::new(FxCoreExpr::ConstInt(2)),
    };
    let block = lower_fx_to_ssa(&expr).unwrap();
    let output = block.pretty_print();

    assert!(output.contains("%0 ="));
    assert!(output.contains("%1 ="));
    assert!(output.contains("ret %2"));
  }
}
