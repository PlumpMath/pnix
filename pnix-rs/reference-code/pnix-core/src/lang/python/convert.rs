//! PythonNode → FxCoreExpr 변환 구조 정의
//!
//! pnix-old의 lang_python/src/convert.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 변환 함수만, 파싱 실행 로직은 executor로 이동
//!
//! ## 설계 원칙
//!
//! - PythonNode를 FxCoreExpr로 변환하는 **순수 구조 변환**
//! - Python 텍스트 파싱/실행/IO는 core 범위 밖 (executor 또는 외부 파서)

use crate::fx::core_expr::FxCoreExpr;
use crate::lang::python_types::{
  BinOperator, BoolOperator, CmpOperator, PythonConstant, PythonNode, UnaryOperator,
};
use serde::{Deserialize, Serialize};

/// PythonNode를 FxCoreExpr로 변환
///
/// 이 함수는 **구조 변환만** 수행하며, 파싱/실행/IO 로직은 포함하지 않습니다.
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn convert_python_to_fx_core(node: &PythonNode) -> Result<FxCoreExpr, PythonConvertError> {
  convert_expr(node)
}

fn convert_expr(node: &PythonNode) -> Result<FxCoreExpr, PythonConvertError> {
  match node {
    PythonNode::Expr { value } => convert_expr(value),

    PythonNode::Name { id } => Ok(FxCoreExpr::var(id)),

    PythonNode::Constant { value } => convert_constant(value),

    PythonNode::List { elts } | PythonNode::Tuple { elts } => Ok(FxCoreExpr::List(
      elts
        .iter()
        .map(convert_expr)
        .collect::<Result<Vec<_>, _>>()?,
    )),

    PythonNode::Dict { keys, values } => convert_dict(keys, values),

    PythonNode::BinOp { left, op, right } => {
      let lhs = convert_expr(left)?;
      let rhs = convert_expr(right)?;
      convert_binop(op, lhs, rhs)
    }

    PythonNode::UnaryOp { op, operand } => {
      let arg = convert_expr(operand)?;
      convert_unaryop(op, arg)
    }

    PythonNode::Compare {
      left,
      ops,
      comparators,
    } => convert_compare(left, ops, comparators),

    PythonNode::BoolOp { op, values } => convert_boolop(op, values),

    PythonNode::IfExp { test, body, orelse } => Ok(FxCoreExpr::if_then_else(
      convert_expr(test)?,
      convert_expr(body)?,
      convert_expr(orelse)?,
    )),

    PythonNode::Attribute { value, attr } => Ok(FxCoreExpr::Select {
      expr: Box::new(convert_expr(value)?),
      attr: attr.clone(),
    }),

    PythonNode::Lambda { args, body } => convert_lambda(args, body),

    PythonNode::Call {
      func,
      args,
      keywords,
    } => convert_call(func, args, keywords),

    other => Err(PythonConvertError::UnsupportedSyntax(format!(
      "unsupported Python node: {}",
      node_kind(other)
    ))),
  }
}

fn convert_constant(value: &PythonConstant) -> Result<FxCoreExpr, PythonConvertError> {
  match value {
    PythonConstant::Bool(v) => Ok(FxCoreExpr::bool(*v)),
    PythonConstant::Int(v) => Ok(FxCoreExpr::int(*v)),
    PythonConstant::Float(v) => Ok(FxCoreExpr::float(*v)),
    PythonConstant::Str(v) => Ok(FxCoreExpr::string(v.clone())),

    PythonConstant::None
    | PythonConstant::Bytes(_)
    | PythonConstant::Complex { .. }
    | PythonConstant::Ellipsis => Err(PythonConvertError::UnsupportedSyntax(format!(
      "unsupported Python constant: {:?}",
      value
    ))),
  }
}

fn convert_dict(
  keys: &[Option<PythonNode>],
  values: &[PythonNode],
) -> Result<FxCoreExpr, PythonConvertError> {
  if keys.len() != values.len() {
    return Err(PythonConvertError::UnsupportedSyntax(format!(
      "dict keys/values length mismatch: {} != {}",
      keys.len(),
      values.len()
    )));
  }

  let mut out = Vec::with_capacity(keys.len());
  for (k, v) in keys.iter().zip(values) {
    let Some(k) = k else {
      return Err(PythonConvertError::UnsupportedSyntax(
        "dict unpacking (None key) is not supported".into(),
      ));
    };
    let key = match k {
      PythonNode::Constant {
        value: PythonConstant::Str(s),
      } => s.clone(),
      other => {
        return Err(PythonConvertError::UnsupportedSyntax(format!(
          "dict key must be a string literal, got {}",
          node_kind(other)
        )));
      }
    };
    out.push((key, convert_expr(v)?));
  }
  Ok(FxCoreExpr::AttrSet(out))
}

fn convert_binop(
  op: &BinOperator,
  lhs: FxCoreExpr,
  rhs: FxCoreExpr,
) -> Result<FxCoreExpr, PythonConvertError> {
  match op {
    BinOperator::Add => Ok(FxCoreExpr::add(lhs, rhs)),
    BinOperator::Sub => Ok(FxCoreExpr::sub(lhs, rhs)),
    BinOperator::Mult => Ok(FxCoreExpr::mul(lhs, rhs)),
    BinOperator::Div => Ok(FxCoreExpr::div(lhs, rhs)),
    BinOperator::Mod => Ok(FxCoreExpr::modulo(lhs, rhs)),
    BinOperator::Pow => Ok(FxCoreExpr::pow(lhs, rhs)),
    BinOperator::FloorDiv => Ok(FxCoreExpr::floor(FxCoreExpr::div(lhs, rhs))),

    BinOperator::LShift
    | BinOperator::RShift
    | BinOperator::BitOr
    | BinOperator::BitXor
    | BinOperator::BitAnd
    | BinOperator::MatMult => Err(PythonConvertError::UnsupportedSyntax(format!(
      "unsupported binary operator: {:?}",
      op
    ))),
  }
}

fn convert_unaryop(op: &UnaryOperator, arg: FxCoreExpr) -> Result<FxCoreExpr, PythonConvertError> {
  match op {
    UnaryOperator::Not => Ok(FxCoreExpr::not(arg)),
    UnaryOperator::USub => Ok(FxCoreExpr::neg(arg)),
    UnaryOperator::UAdd => Ok(arg),
    UnaryOperator::Invert => Err(PythonConvertError::UnsupportedSyntax(
      "bitwise invert (~) is not supported".into(),
    )),
  }
}

fn convert_compare(
  left: &PythonNode,
  ops: &[CmpOperator],
  comparators: &[PythonNode],
) -> Result<FxCoreExpr, PythonConvertError> {
  if ops.len() != comparators.len() {
    return Err(PythonConvertError::UnsupportedSyntax(format!(
      "compare ops/comparators length mismatch: {} != {}",
      ops.len(),
      comparators.len()
    )));
  }
  if ops.len() != 1 {
    return Err(PythonConvertError::UnsupportedSyntax(
      "chained comparisons are not supported".into(),
    ));
  }
  let lhs = convert_expr(left)?;
  let rhs = convert_expr(&comparators[0])?;
  match ops[0] {
    CmpOperator::Eq => Ok(FxCoreExpr::eq(lhs, rhs)),
    CmpOperator::NotEq => Ok(FxCoreExpr::ne(lhs, rhs)),
    CmpOperator::Lt => Ok(FxCoreExpr::lt(lhs, rhs)),
    CmpOperator::LtE => Ok(FxCoreExpr::le(lhs, rhs)),
    CmpOperator::Gt => Ok(FxCoreExpr::gt(lhs, rhs)),
    CmpOperator::GtE => Ok(FxCoreExpr::ge(lhs, rhs)),
    CmpOperator::Is | CmpOperator::IsNot | CmpOperator::In | CmpOperator::NotIn => Err(
      PythonConvertError::UnsupportedSyntax(format!("unsupported compare operator: {:?}", ops[0])),
    ),
  }
}

fn convert_boolop(
  op: &BoolOperator,
  values: &[PythonNode],
) -> Result<FxCoreExpr, PythonConvertError> {
  let mut it = values.iter();
  let Some(first) = it.next() else {
    return Err(PythonConvertError::UnsupportedSyntax(
      "boolop with no values".into(),
    ));
  };
  let mut acc = convert_expr(first)?;
  for v in it {
    let rhs = convert_expr(v)?;
    acc = match op {
      BoolOperator::And => FxCoreExpr::and(acc, rhs),
      BoolOperator::Or => FxCoreExpr::or(acc, rhs),
    };
  }
  Ok(acc)
}

fn convert_lambda(args: &[String], body: &PythonNode) -> Result<FxCoreExpr, PythonConvertError> {
  if args.is_empty() {
    return Err(PythonConvertError::UnsupportedSyntax(
      "lambda with no args is not supported".into(),
    ));
  }
  let mut expr = convert_expr(body)?;
  for param in args.iter().rev() {
    expr = FxCoreExpr::Lambda {
      param: param.clone(),
      body: Box::new(expr),
    };
  }
  Ok(expr)
}

fn convert_call(
  func: &PythonNode,
  args: &[PythonNode],
  keywords: &[(String, PythonNode)],
) -> Result<FxCoreExpr, PythonConvertError> {
  if !keywords.is_empty() {
    return Err(PythonConvertError::UnsupportedSyntax(
      "keyword arguments are not supported".into(),
    ));
  }
  let Some(name) = call_target_name(func) else {
    return Err(PythonConvertError::UnsupportedSyntax(format!(
      "unsupported call target: {}",
      node_kind(func)
    )));
  };

  match name.as_str() {
    "sin" | "math.sin" => unary_call(&name, args, FxCoreExpr::sin),
    "cos" | "math.cos" => unary_call(&name, args, FxCoreExpr::cos),
    "tan" | "math.tan" => unary_call(&name, args, FxCoreExpr::tan),
    "floor" | "math.floor" => unary_call(&name, args, FxCoreExpr::floor),
    "ceil" | "math.ceil" => unary_call(&name, args, FxCoreExpr::ceil),
    "abs" => unary_call(&name, args, FxCoreExpr::abs),
    "sqrt" | "math.sqrt" => unary_call(&name, args, FxCoreExpr::sqrt),
    "exp" | "math.exp" => unary_call(&name, args, FxCoreExpr::exp),
    "ln" | "log" | "math.log" => unary_call(&name, args, FxCoreExpr::ln),
    "pow" => binary_call(&name, args, FxCoreExpr::pow),
    _ => Err(PythonConvertError::UnsupportedSyntax(format!(
      "unsupported call: {}(...)",
      name
    ))),
  }
}

fn unary_call(
  name: &str,
  args: &[PythonNode],
  f: fn(FxCoreExpr) -> FxCoreExpr,
) -> Result<FxCoreExpr, PythonConvertError> {
  if args.len() != 1 {
    return Err(PythonConvertError::UnsupportedSyntax(format!(
      "{} expects 1 arg, got {}",
      name,
      args.len()
    )));
  }
  Ok(f(convert_expr(&args[0])?))
}

fn binary_call(
  name: &str,
  args: &[PythonNode],
  f: fn(FxCoreExpr, FxCoreExpr) -> FxCoreExpr,
) -> Result<FxCoreExpr, PythonConvertError> {
  if args.len() != 2 {
    return Err(PythonConvertError::UnsupportedSyntax(format!(
      "{} expects 2 args, got {}",
      name,
      args.len()
    )));
  }
  Ok(f(convert_expr(&args[0])?, convert_expr(&args[1])?))
}

fn call_target_name(func: &PythonNode) -> Option<String> {
  match func {
    PythonNode::Name { id } => Some(id.clone()),
    PythonNode::Attribute { value, attr } => {
      let prefix = call_target_name(value)?;
      Some(format!("{}.{}", prefix, attr))
    }
    _ => None,
  }
}

fn node_kind(node: &PythonNode) -> &'static str {
  match node {
    PythonNode::Module { .. } => "Module",
    PythonNode::FunctionDef { .. } => "FunctionDef",
    PythonNode::ClassDef { .. } => "ClassDef",
    PythonNode::Return { .. } => "Return",
    PythonNode::Assign { .. } => "Assign",
    PythonNode::Pass => "Pass",
    PythonNode::Break => "Break",
    PythonNode::Continue => "Continue",
    PythonNode::Name { .. } => "Name",
    PythonNode::Constant { .. } => "Constant",
    PythonNode::List { .. } => "List",
    PythonNode::Tuple { .. } => "Tuple",
    PythonNode::BinOp { .. } => "BinOp",
    PythonNode::UnaryOp { .. } => "UnaryOp",
    PythonNode::Compare { .. } => "Compare",
    PythonNode::BoolOp { .. } => "BoolOp",
    PythonNode::IfExp { .. } => "IfExp",
    PythonNode::Call { .. } => "Call",
    PythonNode::Lambda { .. } => "Lambda",
    PythonNode::Dict { .. } => "Dict",
    PythonNode::Attribute { .. } => "Attribute",
    PythonNode::Subscript { .. } => "Subscript",
    PythonNode::Expr { .. } => "Expr",
    PythonNode::If { .. } => "If",
    PythonNode::For { .. } => "For",
    PythonNode::While { .. } => "While",
  }
}

/// Python 변환 에러: Python AST를 Pnix 형식으로 변환 중 발생하는 에러 타입
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PythonConvertError {
  /// 지원하지 않는 구문: 변환할 수 없는 Python 구문
  UnsupportedSyntax(
    /// 에러 메시지
    String,
  ),
  /// 변환 미구현: 아직 구현되지 않은 변환 (executor에서 구현 필요)
  NotImplemented(
    /// 에러 메시지
    String,
  ),
  /// 타입 불일치: 예상 타입과 실제 타입이 일치하지 않음
  TypeMismatch {
    /// 예상 타입
    expected: String,
    /// 실제 타입
    found: String,
  },
}

impl std::fmt::Display for PythonConvertError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::UnsupportedSyntax(msg) => write!(f, "Unsupported Python syntax: {}", msg),
      Self::NotImplemented(msg) => write!(f, "Not implemented: {}", msg),
      Self::TypeMismatch { expected, found } => {
        write!(f, "Type mismatch: expected {}, found {}", expected, found)
      }
    }
  }
}

impl std::error::Error for PythonConvertError {}

// ─────────────────────────────────────────────
// 참고: 실제 변환 로직은 executor로 이동
// ─────────────────────────────────────────────
//
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - convert_python_to_fx_core(node) -> Result<FxCoreExpr, PythonConvertError>
// - convert_constant(value) -> Result<FxCoreExpr, PythonConvertError>
// - convert_comparison(left, op, right) -> Result<FxCoreExpr, PythonConvertError>
//
// 이 함수들은 구조 변환을 수행하지만, 일부 값 계산이 포함될 수 있으므로
// executor에서 구현하는 것이 안전합니다.

#[cfg(test)]
mod tests {
  use super::*;
  use crate::fx::meaning_op::MeaningOpId;

  #[test]
  fn test_python_convert_error_display() {
    let err = PythonConvertError::UnsupportedSyntax("while loop".to_string());
    assert!(err.to_string().contains("Unsupported Python syntax"));
  }

  #[test]
  fn test_python_convert_error_not_implemented() {
    let err = PythonConvertError::NotImplemented("conversion".to_string());
    assert!(err.to_string().contains("Not implemented"));
  }

  #[test]
  fn converts_simple_addition() {
    let node = PythonNode::BinOp {
      left: Box::new(PythonNode::Constant {
        value: PythonConstant::Int(1),
      }),
      op: BinOperator::Add,
      right: Box::new(PythonNode::Constant {
        value: PythonConstant::Int(2),
      }),
    };

    let out = convert_python_to_fx_core(&node).unwrap();
    match out {
      FxCoreExpr::Binary { meta, .. } => assert_eq!(meta.op, MeaningOpId::Add),
      other => panic!("expected binary add, got {:?}", other),
    }
  }

  #[test]
  fn converts_math_call_sin() {
    let node = PythonNode::Call {
      func: Box::new(PythonNode::Name { id: "sin".into() }),
      args: vec![PythonNode::Name { id: "x".into() }],
      keywords: vec![],
    };
    let out = convert_python_to_fx_core(&node).unwrap();
    match out {
      FxCoreExpr::Unary { meta, .. } => assert_eq!(meta.op, MeaningOpId::Sin),
      other => panic!("expected unary sin, got {:?}", other),
    }
  }

  #[test]
  fn converts_math_attribute_call() {
    let node = PythonNode::Call {
      func: Box::new(PythonNode::Attribute {
        value: Box::new(PythonNode::Name { id: "math".into() }),
        attr: "cos".into(),
      }),
      args: vec![PythonNode::Name { id: "t".into() }],
      keywords: vec![],
    };
    let out = convert_python_to_fx_core(&node).unwrap();
    match out {
      FxCoreExpr::Unary { meta, .. } => assert_eq!(meta.op, MeaningOpId::Cos),
      other => panic!("expected unary cos, got {:?}", other),
    }
  }
}
