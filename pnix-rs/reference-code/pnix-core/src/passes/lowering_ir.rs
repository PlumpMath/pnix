//! Lowering functions: FxProgram → SSAProgram → IrModule
//!
//! pnix-old의 pipeline.rs에서 lowering 함수들을 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! - 구조 변환만 수행, 값 계산 없음
//! - run_* 함수들은 제외 (실행 로직)
//!
//! ## 참고
//!
//! - pnix-core의 SSA 구조는 단순하므로, 실제로는 FxCoreExpr → IrExpr 직접 변환이 더 적합할 수 있음
//! - 하지만 pnix-old 호환성을 위해 이 함수들을 제공

use crate::effects::EffectZone;
use crate::fx::meaning_op::MeaningOpId;
use crate::fx::{FxBinding, FxCoreExpr, FxDrawCmd, FxProgram};
use crate::ir::{DrawCmd, IrExpr, IrModule};
use crate::ssa::{lower_fx_to_ssa, SSAOp, SSAValue, SsaBlock, SsaLoweringError};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

// ============================================================
// SSAProgram - 여러 바인딩의 SSA 표현
// ============================================================

/// SSA 프로그램: 여러 fx 바인딩 각각의 SSA 블록을 포함하는 프로그램
#[derive(Debug, Clone)]
pub struct SSAProgram {
  /// 블록 목록 (바인딩 이름 → SSA 블록)
  pub blocks: Vec<(String, SsaBlock)>,
}

impl SSAProgram {
  pub fn new() -> Self {
    Self { blocks: Vec::new() }
  }

  pub fn add(&mut self, name: impl Into<String>, block: SsaBlock) {
    self.blocks.push((name.into(), block));
  }

  pub fn get(&self, name: &str) -> Option<&SsaBlock> {
    self.blocks.iter().find(|(n, _)| n == name).map(|(_, b)| b)
  }

  pub fn names(&self) -> Vec<&str> {
    self.blocks.iter().map(|(n, _)| n.as_str()).collect()
  }
}

impl Default for SSAProgram {
  fn default() -> Self {
    Self::new()
  }
}

/// IR Lowering 에러: IR Lowering 중 발생하는 에러 타입
#[derive(Debug, Error)]
pub enum LoweringIrError {
  #[error("ssa block missing return register {reg}")]
  MissingReturn {
    /// 누락된 레지스터 번호
    reg: usize,
  },
  #[error("ssa register {reg} not found")]
  MissingRegister {
    /// 찾을 수 없는 레지스터 번호
    reg: usize,
  },
  #[error("derived op '{op}' missing argument {index}")]
  MissingDerivedArg {
    /// 연산 ID
    op: MeaningOpId,
    /// 누락된 인자 인덱스
    index: usize,
  },
  #[error("derived op '{op}' not supported in IR lowering")]
  UnsupportedDerivedOp {
    /// 지원하지 않는 연산 ID
    op: MeaningOpId,
  },
  #[error("fx unary op '{op}' not supported in IR lowering")]
  UnsupportedUnaryOp {
    /// 지원하지 않는 단항 연산 ID
    op: MeaningOpId,
  },
  #[error("fx binary op '{op}' not supported in IR lowering")]
  UnsupportedBinaryOp {
    /// 지원하지 않는 이항 연산 ID
    op: MeaningOpId,
  },
  #[error("interop is not supported in IR lowering")]
  InteropUnsupported,
  #[error(transparent)]
  SsaLowering(
    /// SSA Lowering 에러
    #[from]
    SsaLoweringError,
  ),
}

// ============================================================
// FxProgram → SSAProgram
// ============================================================

/// FxProgram을 SSAProgram으로 변환
///
/// 각 바인딩을 개별 SSA 블록으로 변환합니다.
/// Note: pnix-core의 SSA 구조가 단순하므로, 실제 구현은 FxCoreExpr → IrExpr 직접 변환을 권장합니다.
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn lower_fx_program_to_ssa(prog: &FxProgram) -> Result<SSAProgram, LoweringIrError> {
  let mut ssa_prog = SSAProgram::new();

  for FxBinding { name, expr } in &prog.bindings {
    // FxCoreExpr → SsaBlock 변환
    // pnix-core의 SSA 구조가 단순하므로, 기본적인 변환만 수행
    let block = lower_fx_expr_to_ssa_block(expr)?;
    ssa_prog.add(name.clone(), block);
  }

  Ok(ssa_prog)
}

/// FxCoreExpr를 SsaBlock으로 변환
///
/// pnix-old의 lower_fx_to_ssa 함수를 사용하여 변환합니다.
fn lower_fx_expr_to_ssa_block(expr: &FxCoreExpr) -> Result<SsaBlock, LoweringIrError> {
  Ok(lower_fx_to_ssa(expr)?)
}

// ============================================================
// SSAProgram → IrModule
// ============================================================

/// SSAProgram을 IrModule로 변환
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn lower_ssa_program_to_ir(ssa_prog: &SSAProgram) -> Result<IrModule, LoweringIrError> {
  let mut ir_module = IrModule::new();

  for (name, block) in &ssa_prog.blocks {
    let ir_expr = ssa_block_to_ir(block)?;
    ir_module.add(name.clone(), ir_expr);
  }

  Ok(ir_module)
}

/// SSA 블록을 IR 표현식으로 변환
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn ssa_block_to_ir(block: &SsaBlock) -> Result<IrExpr, LoweringIrError> {
  let regs = ssa_block_to_ir_regs(block)?;

  // 반환값
  regs
    .get(&block.ret.index())
    .cloned()
    .ok_or(LoweringIrError::MissingReturn {
      reg: block.ret.index(),
    })
}

/// SSA 블록의 모든 레지스터를 IR 표현식으로 변환 (레지스터 맵 반환)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn ssa_block_to_ir_regs(block: &SsaBlock) -> Result<HashMap<usize, IrExpr>, LoweringIrError> {
  let mut regs: HashMap<usize, IrExpr> = HashMap::new();

  for (val, op) in &block.ops {
    let reg_idx = val.index();
    let expr = ssa_op_to_ir(op, &regs)?;
    regs.insert(reg_idx, expr);
  }

  Ok(regs)
}

fn collect_ir_names(expr: &IrExpr) -> HashSet<String> {
  fn walk(expr: &IrExpr, names: &mut HashSet<String>) {
    match expr {
      IrExpr::VarRef(name) => {
        names.insert(name.clone());
      }
      IrExpr::Lambda { params, body } => {
        for param in params {
          names.insert(param.clone());
        }
        walk(body, names);
      }
      IrExpr::Let { bindings, body } => {
        for (name, value) in bindings {
          names.insert(name.clone());
          walk(value, names);
        }
        walk(body, names);
      }
      IrExpr::List(items) | IrExpr::Tuple(items) => {
        for item in items {
          walk(item, names);
        }
      }
      IrExpr::AttrSet(pairs) => {
        for (_, value) in pairs {
          walk(value, names);
        }
      }
      IrExpr::Add(lhs, rhs)
      | IrExpr::Sub(lhs, rhs)
      | IrExpr::Mul(lhs, rhs)
      | IrExpr::Div(lhs, rhs)
      | IrExpr::Mod(lhs, rhs)
      | IrExpr::Pow(lhs, rhs)
      | IrExpr::Lt(lhs, rhs)
      | IrExpr::Gt(lhs, rhs)
      | IrExpr::Le(lhs, rhs)
      | IrExpr::Ge(lhs, rhs)
      | IrExpr::Eq(lhs, rhs)
      | IrExpr::Ne(lhs, rhs)
      | IrExpr::And(lhs, rhs)
      | IrExpr::Or(lhs, rhs)
      | IrExpr::Concat(lhs, rhs)
      | IrExpr::ListConcat(lhs, rhs)
      | IrExpr::AttrSetMerge(lhs, rhs) => {
        walk(lhs, names);
        walk(rhs, names);
      }
      IrExpr::Neg(arg)
      | IrExpr::Floor(arg)
      | IrExpr::Ceil(arg)
      | IrExpr::Abs(arg)
      | IrExpr::Sqrt(arg)
      | IrExpr::Sin(arg)
      | IrExpr::Cos(arg)
      | IrExpr::Tan(arg)
      | IrExpr::Exp(arg)
      | IrExpr::Log(arg)
      | IrExpr::Not(arg)
      | IrExpr::ListLength(arg)
      | IrExpr::StringLength(arg)
      | IrExpr::AttrSetKeys(arg) => {
        walk(arg, names);
      }
      IrExpr::Select(cond, then_, else_) => {
        walk(cond, names);
        walk(then_, names);
        walk(else_, names);
      }
      IrExpr::Substring { str, start, end } => {
        walk(str, names);
        walk(start, names);
        walk(end, names);
      }
      IrExpr::StringEq(lhs, rhs) => {
        walk(lhs, names);
        walk(rhs, names);
      }
      IrExpr::ListGet { list, index } => {
        walk(list, names);
        walk(index, names);
      }
      IrExpr::TupleGet { tuple, index } => {
        walk(tuple, names);
        walk(index, names);
      }
      IrExpr::ListMap { list, func } => {
        walk(list, names);
        walk(func, names);
      }
      IrExpr::ListFilter { list, pred } => {
        walk(list, names);
        walk(pred, names);
      }
      IrExpr::GetAttr { attrs, key } => {
        walk(attrs, names);
        walk(key, names);
      }
      IrExpr::SetAttr { attrs, key, value } => {
        walk(attrs, names);
        walk(key, names);
        walk(value, names);
      }
      IrExpr::HasAttr { attrs, key } => {
        walk(attrs, names);
        walk(key, names);
      }
      IrExpr::Apply { func, arg } => {
        walk(func, names);
        walk(arg, names);
      }
      IrExpr::ConstFloat(_)
      | IrExpr::ConstInt(_)
      | IrExpr::ConstBool(_)
      | IrExpr::ConstString(_)
      | IrExpr::TimeParam
      | IrExpr::DeltaTime
      | IrExpr::SignalRef(_)
      | IrExpr::Throw(_) => {}
    }
  }

  let mut names = HashSet::new();
  walk(expr, &mut names);
  names
}

fn fresh_ir_name(base: String, reserved: &mut HashSet<String>) -> String {
  if !reserved.contains(&base) {
    reserved.insert(base.clone());
    return base;
  }
  let mut idx = 1;
  loop {
    let candidate = format!("{}_{}", base, idx);
    if !reserved.contains(&candidate) {
      reserved.insert(candidate.clone());
      return candidate;
    }
    idx += 1;
  }
}

/// SSA op을 IR 표현식으로 변환
fn ssa_op_to_ir(op: &SSAOp, regs: &HashMap<usize, IrExpr>) -> Result<IrExpr, LoweringIrError> {
  let get_reg = |v: &SSAValue| {
    regs
      .get(&v.index())
      .cloned()
      .ok_or(LoweringIrError::MissingRegister { reg: v.index() })
  };

  Ok(match op {
    // ========== Constants ==========
    SSAOp::ConstInt(v) => IrExpr::ConstInt(*v),
    SSAOp::ConstFloat(v) => IrExpr::ConstFloat(*v),
    SSAOp::ConstBool(v) => IrExpr::ConstBool(*v),
    SSAOp::ConstString(s) => IrExpr::ConstString(s.clone()),

    // ========== Parameters ==========
    SSAOp::LoadTime => IrExpr::TimeParam,
    SSAOp::LoadDeltaTime => IrExpr::DeltaTime,
    SSAOp::LoadSignal(id) => IrExpr::SignalRef(id.0),
    SSAOp::LoadVar(name) => IrExpr::VarRef(name.clone()),

    // ========== Functions ==========
    SSAOp::Lambda {
      param,
      body,
      captures,
      self_name: _,
    } => {
      let body_expr = ssa_block_to_ir(body)?;
      if captures.is_empty() {
        IrExpr::Lambda {
          params: vec![param.clone()],
          body: Box::new(body_expr),
        }
      } else {
        let mut reserved = collect_ir_names(&body_expr);
        reserved.insert(param.clone());
        for (name, _) in captures {
          reserved.insert(name.clone());
        }

        let mut outer_bindings = Vec::new();
        let mut inner_bindings = Vec::new();

        for (name, value) in captures {
          if name == param {
            continue;
          }
          let base = format!("__capture_{}", name);
          let fresh = fresh_ir_name(base, &mut reserved);
          outer_bindings.push((fresh.clone(), get_reg(value)?));
          inner_bindings.push((name.clone(), IrExpr::VarRef(fresh)));
        }

        let mut lambda_body = body_expr;
        if !inner_bindings.is_empty() {
          lambda_body = IrExpr::Let {
            bindings: inner_bindings,
            body: Box::new(lambda_body),
          };
        }
        let lambda_expr = IrExpr::Lambda {
          params: vec![param.clone()],
          body: Box::new(lambda_body),
        };
        IrExpr::Let {
          bindings: outer_bindings,
          body: Box::new(lambda_expr),
        }
      }
    }
    SSAOp::Call { func, args } | SSAOp::TailCall { func, args } => {
      let mut expr = get_reg(func)?;
      for arg in args {
        expr = IrExpr::Apply {
          func: Box::new(expr),
          arg: Box::new(get_reg(arg)?),
        };
      }
      expr
    }

    // ========== Arithmetic ==========
    SSAOp::Add(a, b) => IrExpr::Add(Box::new(get_reg(a)?), Box::new(get_reg(b)?)),
    SSAOp::Sub(a, b) => IrExpr::Sub(Box::new(get_reg(a)?), Box::new(get_reg(b)?)),
    SSAOp::Mul(a, b) => IrExpr::Mul(Box::new(get_reg(a)?), Box::new(get_reg(b)?)),
    SSAOp::Div(a, b) => IrExpr::Div(Box::new(get_reg(a)?), Box::new(get_reg(b)?)),
    SSAOp::Mod(a, b) => IrExpr::Mod(Box::new(get_reg(a)?), Box::new(get_reg(b)?)),
    SSAOp::Pow(a, b) => IrExpr::Pow(Box::new(get_reg(a)?), Box::new(get_reg(b)?)),
    SSAOp::Neg(a) => IrExpr::Neg(Box::new(get_reg(a)?)),

    // ========== Math Functions ==========
    SSAOp::Floor(a) => IrExpr::Floor(Box::new(get_reg(a)?)),
    SSAOp::Ceil(a) => IrExpr::Ceil(Box::new(get_reg(a)?)),
    SSAOp::Abs(a) => IrExpr::Abs(Box::new(get_reg(a)?)),
    SSAOp::Sqrt(a) => IrExpr::Sqrt(Box::new(get_reg(a)?)),
    SSAOp::Sin(a) => IrExpr::Sin(Box::new(get_reg(a)?)),
    SSAOp::Cos(a) => IrExpr::Cos(Box::new(get_reg(a)?)),
    SSAOp::Tan(a) => IrExpr::Tan(Box::new(get_reg(a)?)),
    SSAOp::Exp(a) => IrExpr::Exp(Box::new(get_reg(a)?)),
    SSAOp::Ln(a) => IrExpr::Log(Box::new(get_reg(a)?)),

    // ========== Comparison ==========
    SSAOp::Lt(a, b) => IrExpr::Lt(Box::new(get_reg(a)?), Box::new(get_reg(b)?)),
    SSAOp::Gt(a, b) => IrExpr::Gt(Box::new(get_reg(a)?), Box::new(get_reg(b)?)),
    SSAOp::Le(a, b) => IrExpr::Le(Box::new(get_reg(a)?), Box::new(get_reg(b)?)),
    SSAOp::Ge(a, b) => IrExpr::Ge(Box::new(get_reg(a)?), Box::new(get_reg(b)?)),
    SSAOp::Eq(a, b) => IrExpr::Eq(Box::new(get_reg(a)?), Box::new(get_reg(b)?)),
    SSAOp::Ne(a, b) => IrExpr::Ne(Box::new(get_reg(a)?), Box::new(get_reg(b)?)),

    // ========== Logic ==========
    SSAOp::And(a, b) => IrExpr::And(Box::new(get_reg(a)?), Box::new(get_reg(b)?)),
    SSAOp::Or(a, b) => IrExpr::Or(Box::new(get_reg(a)?), Box::new(get_reg(b)?)),
    SSAOp::Not(a) => IrExpr::Not(Box::new(get_reg(a)?)),

    // ========== Collections ==========
    SSAOp::ListConstruct(items) => {
      let item_exprs: Vec<IrExpr> = items
        .iter()
        .map(get_reg)
        .collect::<Result<Vec<IrExpr>, LoweringIrError>>()?;
      IrExpr::List(item_exprs)
    }
    SSAOp::AttrSetConstruct(pairs) => {
      let mut kv_pairs = Vec::new();
      for (key, value) in pairs {
        let value_expr = get_reg(value)?;
        kv_pairs.push((key.clone(), value_expr));
      }
      IrExpr::AttrSet(kv_pairs)
    }

    // ========== Control Flow ==========
    SSAOp::Select(c, t, e) => IrExpr::Select(
      Box::new(get_reg(c)?),
      Box::new(get_reg(t)?),
      Box::new(get_reg(e)?),
    ),

    // ========== Derived (High-level) ==========
    SSAOp::Derived(meta, args) => {
      // Derived ops를 기본 연산으로 확장
      return expand_derived_op(&meta.op, args, regs);
    }

    // ========== Attribute Access ==========
    SSAOp::LoadAttr(base, attr) => {
      // Attribute access: base.attr
      match get_reg(base)? {
        IrExpr::VarRef(name) => IrExpr::VarRef(format!("{}.{}", name, attr)),
        base_expr => IrExpr::GetAttr {
          attrs: Box::new(base_expr),
          key: Box::new(IrExpr::ConstString(attr.clone())),
        },
      }
    }

    // ========== Alias (CSE용) ==========
    SSAOp::Alias(a) => get_reg(a)?,

    // ========== CallExtern (외부 호출) ==========
    SSAOp::CallExtern { name, .. } => {
      // 외부 호출은 VarRef로 표현
      IrExpr::VarRef(name.clone())
    }

    // ========== Runtime Errors ==========
    SSAOp::Throw(msg) => IrExpr::Throw(msg.clone()),
  })
}

/// Derived op을 기본 IR로 확장
fn expand_derived_op(
  op: &MeaningOpId,
  args: &[SSAValue],
  regs: &HashMap<usize, IrExpr>,
) -> Result<IrExpr, LoweringIrError> {
  let get_arg = |idx: usize| {
    let v = args.get(idx).ok_or(LoweringIrError::MissingDerivedArg {
      op: *op,
      index: idx,
    })?;
    regs
      .get(&v.index())
      .cloned()
      .ok_or(LoweringIrError::MissingRegister { reg: v.index() })
  };

  Ok(match op {
    MeaningOpId::SecondsFromTime => {
      // floor(time) % 60
      IrExpr::Mod(
        Box::new(IrExpr::Floor(Box::new(IrExpr::TimeParam))),
        Box::new(IrExpr::ConstFloat(60.0)),
      )
    }
    MeaningOpId::MinutesFromTime => {
      // floor(time / 60) % 60
      IrExpr::Mod(
        Box::new(IrExpr::Floor(Box::new(IrExpr::Div(
          Box::new(IrExpr::TimeParam),
          Box::new(IrExpr::ConstFloat(60.0)),
        )))),
        Box::new(IrExpr::ConstFloat(60.0)),
      )
    }
    MeaningOpId::HoursFromTime => {
      // floor(time / 3600) % 12
      IrExpr::Mod(
        Box::new(IrExpr::Floor(Box::new(IrExpr::Div(
          Box::new(IrExpr::TimeParam),
          Box::new(IrExpr::ConstFloat(3600.0)),
        )))),
        Box::new(IrExpr::ConstFloat(12.0)),
      )
    }
    MeaningOpId::GetAttr => {
      // getAttr(key, attrs) → GetAttr { attrs, key }
      let key = get_arg(0)?;
      let attrs = get_arg(1)?;
      IrExpr::GetAttr {
        attrs: Box::new(attrs),
        key: Box::new(key),
      }
    }
    _ => {
      let _ = get_arg;
      return Err(LoweringIrError::UnsupportedDerivedOp { op: *op });
    }
  })
}

// ============================================================
// FxCoreExpr → IrExpr 직접 변환 (권장)
// ============================================================

/// FxCoreExpr → IrExpr 직접 변환 (SSA 없이)
///
/// pnix-core에서는 이 방법을 권장합니다.
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn fx_expr_to_ir(expr: &FxCoreExpr) -> Result<IrExpr, LoweringIrError> {
  Ok(match expr {
    FxCoreExpr::ConstInt(v) => IrExpr::ConstInt(*v),
    FxCoreExpr::ConstFloat(v) => IrExpr::ConstFloat(*v),
    FxCoreExpr::ConstBool(v) => IrExpr::ConstBool(*v),
    FxCoreExpr::ConstString(s) => IrExpr::ConstString(s.clone()),
    FxCoreExpr::ParamSysTime => IrExpr::TimeParam,
    FxCoreExpr::ParamDeltaTime => IrExpr::DeltaTime,
    FxCoreExpr::SignalVar(id) => IrExpr::SignalRef(id.0),
    FxCoreExpr::Var(name) => IrExpr::VarRef(name.clone()),

    FxCoreExpr::Unary { meta, arg } => {
      let a = Box::new(fx_expr_to_ir(arg)?);
      match meta.op {
        MeaningOpId::Neg => IrExpr::Neg(a),
        MeaningOpId::Sin => IrExpr::Sin(a),
        MeaningOpId::Cos => IrExpr::Cos(a),
        MeaningOpId::Tan => IrExpr::Tan(a),
        MeaningOpId::Exp => IrExpr::Exp(a),
        MeaningOpId::Ln => IrExpr::Log(a),
        MeaningOpId::Floor => IrExpr::Floor(a),
        MeaningOpId::Ceil => IrExpr::Ceil(a),
        MeaningOpId::Abs => IrExpr::Abs(a),
        MeaningOpId::Sqrt => IrExpr::Sqrt(a),
        MeaningOpId::Not => IrExpr::Not(a),
        _ => return Err(LoweringIrError::UnsupportedUnaryOp { op: meta.op }),
      }
    }

    FxCoreExpr::Binary { meta, lhs, rhs } => {
      let l = Box::new(fx_expr_to_ir(lhs)?);
      let r = Box::new(fx_expr_to_ir(rhs)?);
      match meta.op {
        MeaningOpId::Add => IrExpr::Add(l, r),
        MeaningOpId::Sub => IrExpr::Sub(l, r),
        MeaningOpId::Mul => IrExpr::Mul(l, r),
        MeaningOpId::Div => IrExpr::Div(l, r),
        MeaningOpId::Mod => IrExpr::Mod(l, r),
        MeaningOpId::Pow => IrExpr::Pow(l, r),
        MeaningOpId::Lt => IrExpr::Lt(l, r),
        MeaningOpId::Gt => IrExpr::Gt(l, r),
        MeaningOpId::Le => IrExpr::Le(l, r),
        MeaningOpId::Ge => IrExpr::Ge(l, r),
        MeaningOpId::Eq => IrExpr::Eq(l, r),
        MeaningOpId::Ne => IrExpr::Ne(l, r),
        MeaningOpId::And => IrExpr::And(l, r),
        MeaningOpId::Or => IrExpr::Or(l, r),
        MeaningOpId::Concat => IrExpr::Concat(l, r),
        MeaningOpId::ListCons => {
          // ListCons(elem, list) → ListConcat([elem], list)
          let elem_list = IrExpr::List(vec![*l]);
          IrExpr::ListConcat(Box::new(elem_list), r)
        }
        MeaningOpId::AttrSetUpdate => IrExpr::AttrSetMerge(l, r),
        _ => return Err(LoweringIrError::UnsupportedBinaryOp { op: meta.op }),
      }
    }

    FxCoreExpr::If { cond, then_, else_ } => IrExpr::Select(
      Box::new(fx_expr_to_ir(cond)?),
      Box::new(fx_expr_to_ir(then_)?),
      Box::new(fx_expr_to_ir(else_)?),
    ),

    FxCoreExpr::List(items) => {
      IrExpr::List(items.iter().map(fx_expr_to_ir).collect::<Result<_, _>>()?)
    }

    FxCoreExpr::AttrSet(pairs) => {
      let items = pairs
        .iter()
        .map(|(k, v)| fx_expr_to_ir(v).map(|expr| (k.clone(), expr)))
        .collect::<Result<Vec<_>, _>>()?;
      IrExpr::AttrSet(items)
    }

    FxCoreExpr::Select { expr, attr } => {
      // 숫자 문자열인 경우 Tuple 필드 접근으로 처리
      if let Ok(index) = attr.parse::<u32>() {
        IrExpr::TupleGet {
          tuple: Box::new(fx_expr_to_ir(expr)?),
          index: Box::new(IrExpr::ConstInt(index as i64)),
        }
      } else {
        // 문자열 키: AttrSet 속성 접근
        IrExpr::GetAttr {
          attrs: Box::new(fx_expr_to_ir(expr)?),
          key: Box::new(IrExpr::ConstString(attr.clone())),
        }
      }
    }

    FxCoreExpr::Derived { meta, args } => match meta.op {
      MeaningOpId::SecondsFromTime => IrExpr::Mod(
        Box::new(IrExpr::Floor(Box::new(IrExpr::TimeParam))),
        Box::new(IrExpr::ConstFloat(60.0)),
      ),
      MeaningOpId::MinutesFromTime => IrExpr::Mod(
        Box::new(IrExpr::Floor(Box::new(IrExpr::Div(
          Box::new(IrExpr::TimeParam),
          Box::new(IrExpr::ConstFloat(60.0)),
        )))),
        Box::new(IrExpr::ConstFloat(60.0)),
      ),
      MeaningOpId::HoursFromTime => IrExpr::Mod(
        Box::new(IrExpr::Floor(Box::new(IrExpr::Div(
          Box::new(IrExpr::TimeParam),
          Box::new(IrExpr::ConstFloat(3600.0)),
        )))),
        Box::new(IrExpr::ConstFloat(12.0)),
      ),
      MeaningOpId::GetAttr => {
        let key = args.first().ok_or(LoweringIrError::MissingDerivedArg {
          op: meta.op,
          index: 0,
        })?;
        let attrs = args.get(1).ok_or(LoweringIrError::MissingDerivedArg {
          op: meta.op,
          index: 1,
        })?;
        IrExpr::GetAttr {
          attrs: Box::new(fx_expr_to_ir(attrs)?),
          key: Box::new(fx_expr_to_ir(key)?),
        }
      }
      MeaningOpId::HasAttr => {
        // Nix 의미론: hasAttr key attrs → (key, attrs) 순서
        // lower.rs의 has_attr_expr와 ssa_eval.rs의 구현과 일치
        let key = args.first().ok_or(LoweringIrError::MissingDerivedArg {
          op: meta.op,
          index: 0,
        })?;
        let attrs = args.get(1).ok_or(LoweringIrError::MissingDerivedArg {
          op: meta.op,
          index: 1,
        })?;
        IrExpr::HasAttr {
          attrs: Box::new(fx_expr_to_ir(attrs)?),
          key: Box::new(fx_expr_to_ir(key)?),
        }
      }
      _ if meta.op.zone() == EffectZone::Interop => IrExpr::Throw(format!(
        "interop not supported in IR eval (op={})",
        meta.op.ir_symbol()
      )),
      _ => return Err(LoweringIrError::UnsupportedDerivedOp { op: meta.op }),
    },

    FxCoreExpr::Lambda { param, body } => IrExpr::Lambda {
      params: vec![param.clone()],
      body: Box::new(fx_expr_to_ir(body)?),
    },

    FxCoreExpr::Interop { meta, lang, .. } => IrExpr::Throw(format!(
      "interop not supported in IR eval (op={}, lang={})",
      meta.op.ir_symbol(),
      lang
    )),

    // Construct - ADT 값 생성자를 AttrSet으로 표현
    // { _variant = "Some"; _args = [ ... ]; }
    FxCoreExpr::Construct { variant, args } => IrExpr::AttrSet(vec![
      ("_variant".to_string(), IrExpr::ConstString(variant.clone())),
      (
        "_args".to_string(),
        IrExpr::List(args.iter().map(fx_expr_to_ir).collect::<Result<_, _>>()?),
      ),
    ]),

    // Y08a-11: Let - lazy semantics 보존을 위해 Let 노드 유지
    FxCoreExpr::Let { name, value, body } => {
      // FxCoreExpr::Let은 단일 바인딩만 지원하지만, IrExpr::Let은 여러 바인딩 지원
      // 단일 바인딩을 벡터로 변환
      let value_ir = fx_expr_to_ir(value)?;
      let body_ir = fx_expr_to_ir(body)?;
      IrExpr::Let {
        bindings: vec![(name.clone(), value_ir)],
        body: Box::new(body_ir),
      }
    }

    // Y08b-2: Throw - 런타임 에러
    FxCoreExpr::Throw { message } => IrExpr::Throw(message.clone()),
  })
}

/// FxDrawCmd → DrawCmd (IR) 변환
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn lower_fx_draw_to_ir(cmd: &FxDrawCmd) -> Result<DrawCmd, LoweringIrError> {
  Ok(match cmd {
    FxDrawCmd::Circle {
      cx,
      cy,
      r,
      fill,
      stroke,
      stroke_width,
    } => DrawCmd::Circle {
      cx: Box::new(fx_expr_to_ir(cx)?),
      cy: Box::new(fx_expr_to_ir(cy)?),
      r: Box::new(fx_expr_to_ir(r)?),
      fill: fill.clone(),
      stroke: stroke.clone(),
      stroke_width: *stroke_width,
    },
    FxDrawCmd::Line {
      x1,
      y1,
      x2,
      y2,
      color,
      width,
    } => DrawCmd::Line {
      x1: Box::new(fx_expr_to_ir(x1)?),
      y1: Box::new(fx_expr_to_ir(y1)?),
      x2: Box::new(fx_expr_to_ir(x2)?),
      y2: Box::new(fx_expr_to_ir(y2)?),
      color: color.clone(),
      width: *width,
    },
    FxDrawCmd::Rect {
      x,
      y,
      w,
      h,
      fill,
      stroke,
      corner_radius,
    } => DrawCmd::Rect {
      x: Box::new(fx_expr_to_ir(x)?),
      y: Box::new(fx_expr_to_ir(y)?),
      w: Box::new(fx_expr_to_ir(w)?),
      h: Box::new(fx_expr_to_ir(h)?),
      fill: fill.clone(),
      stroke: stroke.clone(),
      corner_radius: *corner_radius,
    },
    FxDrawCmd::Text {
      x,
      y,
      text,
      font_size,
      color,
    } => DrawCmd::Text {
      x: Box::new(fx_expr_to_ir(x)?),
      y: Box::new(fx_expr_to_ir(y)?),
      text: text.clone(),
      font_size: *font_size,
      color: color.clone(),
    },
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::fx::meaning_op::MeaningMeta;
  use std::collections::HashMap;

  #[test]
  fn test_fx_expr_to_ir() {
    let expr = FxCoreExpr::add(FxCoreExpr::int(1), FxCoreExpr::int(2));
    let ir = fx_expr_to_ir(&expr).unwrap();

    match ir {
      IrExpr::Add(left, right) => {
        assert!(matches!(*left, IrExpr::ConstInt(1)));
        assert!(matches!(*right, IrExpr::ConstInt(2)));
      }
      other => panic!("Expected Add, got {:?}", other),
    }
  }

  #[test]
  fn test_fx_expr_to_ir_time() {
    let expr = FxCoreExpr::time();
    let ir = fx_expr_to_ir(&expr).unwrap();

    assert!(matches!(ir, IrExpr::TimeParam));
  }

  #[test]
  fn test_fx_expr_to_ir_unsupported_derived() {
    let expr = FxCoreExpr::Derived {
      meta: MeaningMeta::continuous(MeaningOpId::AngleFromSecond),
      args: vec![FxCoreExpr::time()],
    };
    let err = fx_expr_to_ir(&expr).unwrap_err();
    assert!(matches!(err, LoweringIrError::UnsupportedDerivedOp { .. }));
  }

  #[test]
  fn test_fx_expr_to_ir_interop_throw() {
    let expr = FxCoreExpr::Interop {
      meta: MeaningMeta::interop(MeaningOpId::InteropClj),
      lang: "clj".to_string(),
      code: "(+ 1 2)".to_string(),
    };
    let ir = fx_expr_to_ir(&expr).unwrap();
    match ir {
      IrExpr::Throw(msg) => {
        assert!(msg.contains("interop not supported"));
        assert!(msg.contains("fx_interop_clj"));
      }
      other => panic!("Expected Throw, got {:?}", other),
    }
  }

  #[test]
  fn test_fx_expr_to_ir_derived_interop_throw() {
    let expr = FxCoreExpr::Derived {
      meta: MeaningMeta::interop(MeaningOpId::InteropCall),
      args: vec![FxCoreExpr::string("interop_fn")],
    };
    let ir = fx_expr_to_ir(&expr).unwrap();
    match ir {
      IrExpr::Throw(msg) => {
        assert!(msg.contains("interop not supported"));
        assert!(msg.contains("fx_interop_call"));
      }
      other => panic!("Expected Throw, got {:?}", other),
    }
  }

  #[test]
  fn test_lower_fx_program_to_ssa() {
    let mut prog = FxProgram::new();
    prog.add("x", FxCoreExpr::int(42));
    prog.add(
      "y",
      FxCoreExpr::add(FxCoreExpr::var("x"), FxCoreExpr::int(1)),
    );

    let ssa_prog = lower_fx_program_to_ssa(&prog).unwrap();
    assert_eq!(ssa_prog.names(), vec!["x", "y"]);
  }

  #[test]
  fn test_lower_ssa_program_to_ir() {
    let mut ssa_prog = SSAProgram::new();
    let block = SsaBlock {
      label: "test".into(),
      ops: vec![(
        SSAValue(0),
        SSAOp::CallExtern {
          name: "test".into(),
          args: vec![],
        },
      )],
      ret: SSAValue(0),
    };
    ssa_prog.add("x", block);

    let ir_module = lower_ssa_program_to_ir(&ssa_prog).unwrap();
    assert_eq!(ir_module.names(), vec!["x"]);
  }

  #[test]
  fn test_fx_expr_to_ir_throw() {
    let expr = FxCoreExpr::Throw {
      message: "non-exhaustive match".to_string(),
    };
    let ir = fx_expr_to_ir(&expr).unwrap();

    match ir {
      IrExpr::Throw(msg) => {
        assert_eq!(msg, "non-exhaustive match");
      }
      other => panic!("Expected Throw, got {:?}", other),
    }
  }

  #[test]
  fn test_lambda_capture_env_preserved_in_ir() {
    let body_block = SsaBlock {
      label: "lambda".into(),
      ops: vec![(SSAValue(0), SSAOp::LoadVar("x".to_string()))],
      ret: SSAValue(0),
    };
    let op = SSAOp::Lambda {
      param: "y".to_string(),
      body: Box::new(body_block),
      captures: vec![("x".to_string(), SSAValue(1))],
      self_name: None,
    };

    let mut regs = HashMap::new();
    regs.insert(1, IrExpr::VarRef("x".to_string()));

    let ir = ssa_op_to_ir(&op, &regs).unwrap();

    match ir {
      IrExpr::Let { bindings, body } => {
        assert_eq!(bindings.len(), 1);
        let (outer_name, outer_value) = &bindings[0];
        assert!(outer_name.starts_with("__capture_x"));
        assert!(matches!(outer_value, IrExpr::VarRef(name) if name == "x"));

        match *body {
          IrExpr::Lambda { params, body } => {
            assert_eq!(params, vec!["y"]);
            match *body {
              IrExpr::Let { bindings, body } => {
                assert_eq!(bindings.len(), 1);
                let (inner_name, inner_value) = &bindings[0];
                assert_eq!(inner_name, "x");
                assert!(matches!(inner_value, IrExpr::VarRef(name) if name == outer_name));
                assert!(matches!(*body, IrExpr::VarRef(name) if name == "x"));
              }
              other => panic!("Expected inner Let, got {:?}", other),
            }
          }
          other => panic!("Expected Lambda, got {:?}", other),
        }
      }
      other => panic!("Expected outer Let, got {:?}", other),
    }
  }
}
