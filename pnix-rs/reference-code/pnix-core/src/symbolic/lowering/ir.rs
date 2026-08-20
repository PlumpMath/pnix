//! 중간 IR (SSA-like)
//!
//! SymExpr → IR 변환 후 수치 평가, 코드 생성 등에 사용
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 없음

use serde::{Deserialize, Serialize};

/// IR 명령어
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IrInst {
  /// 상수 로드: %dst = const
  Const { dst: String, value: f64 },
  /// 변수 로드: %dst = var
  LoadVar { dst: String, var: String },
  /// 이항 연산: %dst = %lhs op %rhs
  BinOp {
    dst: String,
    op: BinOpKind,
    lhs: String,
    rhs: String,
  },
  /// 단항 연산: %dst = op %src
  UnaryOp {
    dst: String,
    op: UnaryOpKind,
    src: String,
  },
  /// 함수 호출: %dst = func(%args...)
  Call {
    dst: String,
    func: String,
    args: Vec<String>,
  },
}

/// 이항 연산 종류
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BinOpKind {
  Add,
  Sub,
  Mul,
  Div,
  Pow,
}

/// 단항 연산 종류
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnaryOpKind {
  Neg,
  Sin,
  Cos,
  Tan,
  Exp,
  Log,
  Abs,
}

/// IR 프로그램 (명령어 시퀀스)
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct IrProgram {
  /// 명령어 리스트
  pub instructions: Vec<IrInst>,
  /// 최종 결과 레지스터
  pub result: Option<String>,
}

impl IrProgram {
  /// 새 빈 프로그램
  pub fn new() -> Self {
    Self::default()
  }

  /// 명령어 추가
  pub fn push(&mut self, inst: IrInst) {
    self.instructions.push(inst);
  }

  /// 결과 레지스터 설정
  pub fn set_result(&mut self, reg: impl Into<String>) {
    self.result = Some(reg.into());
  }

  /// 명령어 개수
  pub fn len(&self) -> usize {
    self.instructions.len()
  }

  /// 비어있는지
  pub fn is_empty(&self) -> bool {
    self.instructions.is_empty()
  }

  /// Pretty-print (디버깅용)
  pub fn pretty_print(&self) -> String {
    let mut out = String::new();
    for inst in &self.instructions {
      match inst {
        IrInst::Const { dst, value } => {
          out.push_str(&format!("{} = const {}\n", dst, value));
        }
        IrInst::LoadVar { dst, var } => {
          out.push_str(&format!("{} = load {}\n", dst, var));
        }
        IrInst::BinOp { dst, op, lhs, rhs } => {
          out.push_str(&format!("{} = {:?} {} {}\n", dst, op, lhs, rhs));
        }
        IrInst::UnaryOp { dst, op, src } => {
          out.push_str(&format!("{} = {:?} {}\n", dst, op, src));
        }
        IrInst::Call { dst, func, args } => {
          out.push_str(&format!("{} = {}({})\n", dst, func, args.join(", ")));
        }
      }
    }
    if let Some(ref res) = self.result {
      out.push_str(&format!("ret {}\n", res));
    }
    out
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
  use super::*;

  #[test]
  fn test_ir_program() {
    let mut prog = IrProgram::new();
    prog.push(IrInst::Const {
      dst: "%0".into(),
      value: 1.0,
    });
    prog.push(IrInst::Const {
      dst: "%1".into(),
      value: 2.0,
    });
    prog.push(IrInst::BinOp {
      dst: "%2".into(),
      op: BinOpKind::Add,
      lhs: "%0".into(),
      rhs: "%1".into(),
    });
    prog.set_result("%2");

    assert_eq!(prog.len(), 3);
    assert!(prog.result.is_some());
  }

  #[test]
  fn test_pretty_print() {
    let mut prog = IrProgram::new();
    prog.push(IrInst::Const {
      dst: "%0".into(),
      value: 42.0,
    });
    prog.set_result("%0");

    let output = prog.pretty_print();
    assert!(output.contains("%0 = const 42"));
    assert!(output.contains("ret %0"));
  }

  #[test]
  fn test_serde() {
    let mut prog = IrProgram::new();
    prog.push(IrInst::Const {
      dst: "%0".into(),
      value: 3.14,
    });
    prog.set_result("%0");

    let json = serde_json::to_string(&prog).unwrap();
    let restored: IrProgram = serde_json::from_str(&json).unwrap();
    assert_eq!(prog, restored);
  }
}
