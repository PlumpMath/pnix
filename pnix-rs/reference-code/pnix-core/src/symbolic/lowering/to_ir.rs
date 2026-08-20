//! SymExpr → IR 변환
//!
//! 핵심 규칙: 텐서가 포함된 표현식은 IR로 내리지 않는다.
//! CT/정규화/아이덴티티 패스를 통과해 "완전히 contracted scalar"로 확정된 식만 대상.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 변환만, 실제 수치 평가 없음

use super::ir::{BinOpKind, IrInst, IrProgram};
use crate::symbolic::expr::{SymExpr, SymKind};
use std::error::Error;
use std::fmt;

/// Lowering 에러
#[derive(Debug, Clone, PartialEq)]
pub enum LowerError {
  /// 텐서가 완전히 수축되지 않음
  TensorNotFullyContracted,
  /// 빈 표현식 리스트
  EmptyExprList,
  /// 지원되지 않는 표현식
  UnsupportedExpr(String),
}

impl fmt::Display for LowerError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      LowerError::TensorNotFullyContracted => {
        write!(
          f,
          "tensor not fully contracted to scalar - cannot lower to IR"
        )
      }
      LowerError::EmptyExprList => {
        write!(f, "empty expression list")
      }
      LowerError::UnsupportedExpr(msg) => {
        write!(f, "unsupported expression: {}", msg)
      }
    }
  }
}

impl Error for LowerError {}

/// 텐서 포함 여부 검사 (가드)
///
/// 텐서가 포함된 표현식은 IR로 내릴 수 없음
pub fn contains_tensor(expr: &SymExpr) -> bool {
  match &expr.kind {
    SymKind::Tensor(_) => true,
    SymKind::Contract(_, _, _) => true,
    SymKind::Raise(x, _) | SymKind::Lower(x, _) => contains_tensor(x),
    SymKind::Var(_) | SymKind::Const(_) | SymKind::Exact(_) => false,
    SymKind::Neg(x) => contains_tensor(x),
    SymKind::Add(xs) | SymKind::Mul(xs) => xs.iter().any(contains_tensor),
    SymKind::Pow(b, e) => contains_tensor(b) || contains_tensor(e),
    SymKind::Sin(x)
    | SymKind::Cos(x)
    | SymKind::Tan(x)
    | SymKind::Exp(x)
    | SymKind::Log(x)
    | SymKind::Abs(x) => contains_tensor(x),
    SymKind::Derivative(x, _) => contains_tensor(x),
  }
}

/// SymExpr → IR 변환기
pub struct IrLowering {
  reg_counter: usize,
}

impl Default for IrLowering {
  fn default() -> Self {
    Self::new()
  }
}

impl IrLowering {
  /// 새 인스턴스
  pub fn new() -> Self {
    Self { reg_counter: 0 }
  }

  /// 새 레지스터 이름 생성
  fn fresh_reg(&mut self) -> String {
    let r = format!("%{}", self.reg_counter);
    self.reg_counter += 1;
    r
  }

  /// SymExpr → IR 변환 (가드 포함)
  pub fn lower(&mut self, expr: &SymExpr) -> Result<IrProgram, LowerError> {
    // Contract 노드 정규화 시도
    let expr = self.normalize_contracts(expr.clone())?;

    // 가드: 텐서가 포함되어 있으면 거부
    if contains_tensor(&expr) {
      return Err(LowerError::TensorNotFullyContracted);
    }

    let mut program = IrProgram::new();
    let result = self.lower_expr(&expr, &mut program)?;
    program.set_result(result);
    Ok(program)
  }

  /// Contract 노드를 명시적으로 처리
  ///
  /// 완전히 수축되지 않은 경우 에러를 반환
  #[allow(clippy::only_used_in_recursion)]
  fn normalize_contracts(&mut self, expr: SymExpr) -> Result<SymExpr, LowerError> {
    match &expr.kind {
      SymKind::Contract(inner, _idx1, _idx2) => {
        // Contract 노드 처리: 내부 표현식 정규화
        let normalized_inner = self.normalize_contracts(*inner.clone())?;

        // 텐서가 남아있으면 수축 불완전
        if contains_tensor(&normalized_inner) {
          return Err(LowerError::TensorNotFullyContracted);
        }

        Ok(normalized_inner)
      }
      SymKind::Tensor(_) => {
        // 텐서 자체는 Contract 없이 수축 불가능
        Err(LowerError::TensorNotFullyContracted)
      }
      SymKind::Add(xs) => {
        let normalized: Result<Vec<_>, _> = xs
          .iter()
          .map(|x| self.normalize_contracts(x.clone()))
          .collect();
        Ok(SymExpr::add(normalized?))
      }
      SymKind::Mul(xs) => {
        let normalized: Result<Vec<_>, _> = xs
          .iter()
          .map(|x| self.normalize_contracts(x.clone()))
          .collect();
        Ok(SymExpr::mul(normalized?))
      }
      SymKind::Neg(x) => Ok(SymExpr::neg(self.normalize_contracts(*x.clone())?)),
      SymKind::Pow(b, e) => Ok(SymExpr::pow(
        self.normalize_contracts(*b.clone())?,
        self.normalize_contracts(*e.clone())?,
      )),
      SymKind::Raise(x, _idx) | SymKind::Lower(x, _idx) => {
        let normalized = self.normalize_contracts(*x.clone())?;
        if contains_tensor(&normalized) {
          return Err(LowerError::TensorNotFullyContracted);
        }
        Ok(normalized)
      }
      SymKind::Sin(x) => Ok(SymExpr::sin(self.normalize_contracts(*x.clone())?)),
      SymKind::Cos(x) => Ok(SymExpr::cos(self.normalize_contracts(*x.clone())?)),
      SymKind::Tan(x) => Ok(SymExpr::tan(self.normalize_contracts(*x.clone())?)),
      SymKind::Exp(x) => Ok(SymExpr::exp(self.normalize_contracts(*x.clone())?)),
      SymKind::Log(x) => Ok(SymExpr::log(self.normalize_contracts(*x.clone())?)),
      SymKind::Abs(x) => Ok(SymExpr::abs(self.normalize_contracts(*x.clone())?)),
      SymKind::Derivative(x, v) => Ok(SymExpr::derivative(
        self.normalize_contracts(*x.clone())?,
        v.clone(),
      )),
      _ => Ok(expr),
    }
  }

  /// 표현식을 IR로 변환
  fn lower_expr(&mut self, expr: &SymExpr, prog: &mut IrProgram) -> Result<String, LowerError> {
    match &expr.kind {
      SymKind::Const(c) => {
        let dst = self.fresh_reg();
        prog.push(IrInst::Const {
          dst: dst.clone(),
          value: *c,
        });
        Ok(dst)
      }
      SymKind::Exact(n) => {
        let dst = self.fresh_reg();
        prog.push(IrInst::Const {
          dst: dst.clone(),
          value: n.to_f64(),
        });
        Ok(dst)
      }
      SymKind::Var(name) => {
        let dst = self.fresh_reg();
        prog.push(IrInst::LoadVar {
          dst: dst.clone(),
          var: name.clone(),
        });
        Ok(dst)
      }
      SymKind::Add(xs) => {
        if xs.is_empty() {
          // 정규화 후에는 빈 Add가 존재하지 않아야 함 (불변량 위반)
          debug_assert!(
            false,
            "Empty Add expression after normalization - invariant violation"
          );
          return Err(LowerError::EmptyExprList);
        }
        let mut result = self.lower_expr(&xs[0], prog)?;
        for x in &xs[1..] {
          let rhs = self.lower_expr(x, prog)?;
          let dst = self.fresh_reg();
          prog.push(IrInst::BinOp {
            dst: dst.clone(),
            op: BinOpKind::Add,
            lhs: result,
            rhs,
          });
          result = dst;
        }
        Ok(result)
      }
      SymKind::Mul(xs) => {
        if xs.is_empty() {
          // 정규화 후에는 빈 Mul이 존재하지 않아야 함 (불변량 위반)
          debug_assert!(
            false,
            "Empty Mul expression after normalization - invariant violation"
          );
          return Err(LowerError::EmptyExprList);
        }
        let mut result = self.lower_expr(&xs[0], prog)?;
        for x in &xs[1..] {
          let rhs = self.lower_expr(x, prog)?;
          let dst = self.fresh_reg();
          prog.push(IrInst::BinOp {
            dst: dst.clone(),
            op: BinOpKind::Mul,
            lhs: result,
            rhs,
          });
          result = dst;
        }
        Ok(result)
      }
      SymKind::Pow(base, exp) => {
        let lhs = self.lower_expr(base, prog)?;
        let rhs = self.lower_expr(exp, prog)?;
        let dst = self.fresh_reg();
        prog.push(IrInst::BinOp {
          dst: dst.clone(),
          op: BinOpKind::Pow,
          lhs,
          rhs,
        });
        Ok(dst)
      }
      SymKind::Neg(x) => {
        let src = self.lower_expr(x, prog)?;
        let dst = self.fresh_reg();
        prog.push(IrInst::Call {
          dst: dst.clone(),
          func: "neg".into(),
          args: vec![src],
        });
        Ok(dst)
      }
      SymKind::Sin(x) => {
        let src = self.lower_expr(x, prog)?;
        let dst = self.fresh_reg();
        prog.push(IrInst::Call {
          dst: dst.clone(),
          func: "sin".into(),
          args: vec![src],
        });
        Ok(dst)
      }
      SymKind::Cos(x) => {
        let src = self.lower_expr(x, prog)?;
        let dst = self.fresh_reg();
        prog.push(IrInst::Call {
          dst: dst.clone(),
          func: "cos".into(),
          args: vec![src],
        });
        Ok(dst)
      }
      SymKind::Tan(x) => {
        let src = self.lower_expr(x, prog)?;
        let dst = self.fresh_reg();
        prog.push(IrInst::Call {
          dst: dst.clone(),
          func: "tan".into(),
          args: vec![src],
        });
        Ok(dst)
      }
      SymKind::Exp(x) => {
        let src = self.lower_expr(x, prog)?;
        let dst = self.fresh_reg();
        prog.push(IrInst::Call {
          dst: dst.clone(),
          func: "exp".into(),
          args: vec![src],
        });
        Ok(dst)
      }
      SymKind::Log(x) => {
        let src = self.lower_expr(x, prog)?;
        let dst = self.fresh_reg();
        prog.push(IrInst::Call {
          dst: dst.clone(),
          func: "log".into(),
          args: vec![src],
        });
        Ok(dst)
      }
      SymKind::Abs(x) => {
        let src = self.lower_expr(x, prog)?;
        let dst = self.fresh_reg();
        prog.push(IrInst::Call {
          dst: dst.clone(),
          func: "abs".into(),
          args: vec![src],
        });
        Ok(dst)
      }
      // 텐서/미분은 가드에서 이미 걸러짐
      _ => Err(LowerError::UnsupportedExpr(format!("{:?}", expr.kind))),
    }
  }
}

/// 편의 함수: SymExpr → IR 변환
pub fn lower_to_ir(expr: &SymExpr) -> Result<IrProgram, LowerError> {
  let mut lowering = IrLowering::new();
  lowering.lower(expr)
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;
  use crate::symbolic::expr::{TensorIndex, TensorSymbol};

  #[test]
  fn test_lower_const() {
    let expr = SymExpr::constant(42.0);
    let result = lower_to_ir(&expr);
    assert!(result.is_ok());

    let prog = result.unwrap();
    assert_eq!(prog.len(), 1);
    assert!(prog.result.is_some());
  }

  #[test]
  fn test_lower_var() {
    let expr = SymExpr::var("x");
    let result = lower_to_ir(&expr);
    assert!(result.is_ok());

    let prog = result.unwrap();
    assert_eq!(prog.len(), 1);
  }

  #[test]
  fn test_lower_add() {
    let expr = SymExpr::add2(SymExpr::constant(1.0), SymExpr::constant(2.0));
    let result = lower_to_ir(&expr);
    assert!(result.is_ok());

    let prog = result.unwrap();
    assert_eq!(prog.len(), 3); // const, const, add
  }

  #[test]
  fn test_lower_mul() {
    let expr = SymExpr::mul2(SymExpr::var("x"), SymExpr::constant(2.0));
    let result = lower_to_ir(&expr);
    assert!(result.is_ok());

    let prog = result.unwrap();
    assert_eq!(prog.len(), 3); // load, const, mul
  }

  #[test]
  fn test_lower_sin() {
    let expr = SymExpr::sin(SymExpr::var("x"));
    let result = lower_to_ir(&expr);
    assert!(result.is_ok());

    let prog = result.unwrap();
    assert_eq!(prog.len(), 2); // load, sin
  }

  #[test]
  fn test_lower_complex() {
    // sin(x)^2 + cos(x)^2
    let expr = SymExpr::add2(
      SymExpr::pow(SymExpr::sin(SymExpr::var("x")), SymExpr::int(2)),
      SymExpr::pow(SymExpr::cos(SymExpr::var("x")), SymExpr::int(2)),
    );
    let result = lower_to_ir(&expr);
    assert!(result.is_ok());

    let prog = result.unwrap();
    assert!(prog.len() > 5);
  }

  #[test]
  fn test_lower_exact() {
    let expr = SymExpr::ratio(1, 2).unwrap(); // 1/2
    let result = lower_to_ir(&expr);
    assert!(result.is_ok());

    let prog = result.unwrap();
    if let Some(IrInst::Const { value, .. }) = prog.instructions.first() {
      assert!((value - 0.5).abs() < 1e-10);
    } else {
      panic!("Expected Const instruction");
    }
  }

  #[test]
  fn test_lower_rejects_tensor() {
    let tensor = TensorSymbol {
      name: "g".into(),
      indices: vec![
        TensorIndex::down("mu", "spacetime"),
        TensorIndex::down("nu", "spacetime"),
      ],
      symmetries: vec![],
    };
    let expr = SymExpr::tensor(tensor);
    let result = lower_to_ir(&expr);

    assert!(result.is_err());
    assert!(matches!(
      result.unwrap_err(),
      LowerError::TensorNotFullyContracted
    ));
  }

  #[test]
  fn test_lower_rejects_unsupported_expr() {
    let expr = SymExpr::derivative(SymExpr::var("x"), "x");
    let result = lower_to_ir(&expr);
    assert!(matches!(
      result.unwrap_err(),
      LowerError::UnsupportedExpr(_)
    ));
  }

  #[test]
  fn test_contains_tensor_scalar() {
    let expr = SymExpr::add2(SymExpr::var("x"), SymExpr::constant(1.0));
    assert!(!contains_tensor(&expr));
  }

  #[test]
  fn test_contains_tensor_with_tensor() {
    let tensor = TensorSymbol {
      name: "R".into(),
      indices: vec![],
      symmetries: vec![],
    };
    let expr = SymExpr::tensor(tensor);
    assert!(contains_tensor(&expr));
  }

  #[test]
  fn test_pretty_print() {
    let expr = SymExpr::add2(SymExpr::constant(1.0), SymExpr::constant(2.0));
    let prog = lower_to_ir(&expr).unwrap();
    let output = prog.pretty_print();

    assert!(output.contains("const"));
    assert!(output.contains("Add"));
    assert!(output.contains("ret"));
  }
}
