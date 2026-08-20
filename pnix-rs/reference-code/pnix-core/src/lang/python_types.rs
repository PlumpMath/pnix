//! Python AST types
//!
//! pnix-old의 lang_python/src/python_types.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 파싱/실행 로직 제외

use serde::{Deserialize, Serialize};

/// Python AST 노드 타입: Pnix의 unified 형식으로 변환 가능한 Python AST 노드 타입
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PythonNode {
  /// 모듈 노드
  Module {
    /// 모듈 본문 (노드 목록)
    body: Vec<PythonNode>,
  },
  /// 함수 정의 노드
  FunctionDef {
    /// 함수 이름
    name: String,
    /// 인자 목록 (인자 이름 목록)
    args: Vec<String>,
    /// 함수 본문 (노드 목록)
    body: Vec<PythonNode>,
    /// 반환 타입 (선택적)
    returns: Option<Box<PythonNode>>,
  },
  /// 클래스 정의 노드
  ClassDef {
    /// 클래스 이름
    name: String,
    /// 베이스 클래스 목록 (상속받을 클래스 목록)
    bases: Vec<PythonNode>,
    /// 클래스 본문 (노드 목록)
    body: Vec<PythonNode>,
  },
  Return {
    value: Option<Box<PythonNode>>,
  },
  Assign {
    targets: Vec<PythonNode>,
    value: Box<PythonNode>,
  },
  Pass,
  Break,
  Continue,
  Name {
    id: String,
  },
  Constant {
    value: PythonConstant,
  },
  List {
    elts: Vec<PythonNode>,
  },
  Tuple {
    elts: Vec<PythonNode>,
  },
  BinOp {
    left: Box<PythonNode>,
    op: BinOperator,
    right: Box<PythonNode>,
  },
  UnaryOp {
    op: UnaryOperator,
    operand: Box<PythonNode>,
  },
  Compare {
    left: Box<PythonNode>,
    ops: Vec<CmpOperator>,
    comparators: Vec<PythonNode>,
  },
  BoolOp {
    op: BoolOperator,
    values: Vec<PythonNode>,
  },
  IfExp {
    test: Box<PythonNode>,
    body: Box<PythonNode>,
    orelse: Box<PythonNode>,
  },
  Call {
    func: Box<PythonNode>,
    args: Vec<PythonNode>,
    keywords: Vec<(String, PythonNode)>,
  },
  Lambda {
    args: Vec<String>,
    body: Box<PythonNode>,
  },
  Dict {
    keys: Vec<Option<PythonNode>>,
    values: Vec<PythonNode>,
  },
  Attribute {
    value: Box<PythonNode>,
    attr: String,
  },
  Subscript {
    value: Box<PythonNode>,
    slice: Box<PythonNode>,
  },
  Expr {
    value: Box<PythonNode>,
  },
  If {
    test: Box<PythonNode>,
    body: Vec<PythonNode>,
    orelse: Vec<PythonNode>,
  },
  For {
    target: Box<PythonNode>,
    iter: Box<PythonNode>,
    body: Vec<PythonNode>,
    orelse: Vec<PythonNode>,
  },
  While {
    test: Box<PythonNode>,
    body: Vec<PythonNode>,
    orelse: Vec<PythonNode>,
  },
}

/// 이항 연산자: Python의 이항 연산자
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BinOperator {
  /// 덧셈 (+)
  Add,
  /// 뺄셈 (-)
  Sub,
  /// 곱셈 (*)
  Mult,
  /// 나눗셈 (/)
  Div,
  /// 정수 나눗셈 (//)
  FloorDiv,
  /// 나머지 (%)
  Mod,
  /// 거듭제곱 (**)
  Pow,
  /// 왼쪽 시프트 (<<)
  LShift,
  /// 오른쪽 시프트 (>>)
  RShift,
  /// 비트 OR (|)
  BitOr,
  /// 비트 XOR (^)
  BitXor,
  /// 비트 AND (&)
  BitAnd,
  /// 행렬 곱셈 (@)
  MatMult,
}

/// 단항 연산자: Python의 단항 연산자
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum UnaryOperator {
  /// 비트 반전 (~)
  Invert,
  /// 논리 부정 (not)
  Not,
  /// 단항 덧셈 (+)
  UAdd,
  /// 단항 뺄셈 (-)
  USub,
}

/// 비교 연산자: Python의 비교 연산자
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CmpOperator {
  /// 같음 (==)
  Eq,
  /// 다름 (!=)
  NotEq,
  /// 작음 (<)
  Lt,
  /// 작거나 같음 (<=)
  LtE,
  /// 큼 (>)
  Gt,
  /// 크거나 같음 (>=)
  GtE,
  /// 동일성 (is)
  Is,
  /// 비동일성 (is not)
  IsNot,
  /// 포함 (in)
  In,
  /// 비포함 (not in)
  NotIn,
}

/// 불리언 연산자: Python의 불리언 연산자
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BoolOperator {
  /// 논리 AND (and)
  And,
  /// 논리 OR (or)
  Or,
}

/// Python 상수 값: Python의 상수 리터럴 값
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PythonConstant {
  /// None 값
  None,
  /// 불리언 값
  Bool(
    /// 불리언 값
    bool,
  ),
  /// 정수 값
  Int(
    /// 정수 값
    i64,
  ),
  /// 부동소수점 값
  Float(
    /// 실수 값
    f64,
  ),
  /// 복소수 값
  Complex {
    /// 실수부
    real: f64,
    /// 허수부
    imag: f64,
  },
  /// 문자열 값
  Str(
    /// 문자열 값
    String,
  ),
  /// 바이트 값
  Bytes(
    /// 바이트 벡터
    Vec<u8>,
  ),
  /// Ellipsis (...)
  Ellipsis,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_python_node_serialization() {
    let node = PythonNode::Constant {
      value: PythonConstant::Int(42),
    };
    let json = serde_json::to_string(&node).unwrap();
    let restored: PythonNode = serde_json::from_str(&json).unwrap();
    assert_eq!(node, restored);
  }

  #[test]
  fn test_bin_operator() {
    assert_eq!(BinOperator::Add, BinOperator::Add);
    assert_ne!(BinOperator::Add, BinOperator::Sub);
  }

  #[test]
  fn test_unary_operator() {
    assert_eq!(UnaryOperator::Not, UnaryOperator::Not);
    assert_ne!(UnaryOperator::Not, UnaryOperator::UAdd);
  }

  #[test]
  fn test_python_constant() {
    let c1 = PythonConstant::Int(42);
    let c2 = PythonConstant::Int(42);
    let c3 = PythonConstant::Int(43);
    assert_eq!(c1, c2);
    assert_ne!(c1, c3);
  }
}
