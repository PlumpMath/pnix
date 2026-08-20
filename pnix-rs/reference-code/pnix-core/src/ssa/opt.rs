//! SSA Optimizer - CSE, DCE
//!
//! pnix-old의 meaning_core/src/ssa_opt.rs에서 마이그레이션.
//!
//! 컴파일러 레벨 최적화
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조적 변환만, 값 계산 없음
//! - CSE: 동일 연산을 Alias로 대체 (그래프 변환)
//! - DCE: live set 분석 후 미사용 코드 제거 (그래프 변환)
//! - Alias Removal: 체인 해결 후 직접 참조로 변경 (그래프 변환)

use crate::ssa::{SSABlock, SSAOp, SSAValue};
use std::collections::{HashMap, HashSet};

/// SSA 최적화 파이프라인
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn optimize_ssa(block: SSABlock) -> SSABlock {
  ssa_remove_aliases(ssa_dce(ssa_cse(block)))
}

// ============================================================
// CSE - Common Subexpression Elimination
// ============================================================

/// CSE: 동일한 연산을 중복 계산하지 않음
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CseUnaryOp {
  Neg,
  Floor,
  Ceil,
  Abs,
  Sqrt,
  Sin,
  Cos,
  Tan,
  Exp,
  Ln,
  Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CseBinaryOp {
  Add,
  Sub,
  Mul,
  Div,
  Mod,
  Pow,
  Lt,
  Gt,
  Le,
  Ge,
  Eq,
  Ne,
  And,
  Or,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum CseKey {
  Unary(CseUnaryOp, SSAValue),
  Binary(CseBinaryOp, SSAValue, SSAValue),
}

fn cse_key(op: &SSAOp) -> Option<CseKey> {
  match op {
    SSAOp::Neg(a) => Some(CseKey::Unary(CseUnaryOp::Neg, *a)),
    SSAOp::Floor(a) => Some(CseKey::Unary(CseUnaryOp::Floor, *a)),
    SSAOp::Ceil(a) => Some(CseKey::Unary(CseUnaryOp::Ceil, *a)),
    SSAOp::Abs(a) => Some(CseKey::Unary(CseUnaryOp::Abs, *a)),
    SSAOp::Sqrt(a) => Some(CseKey::Unary(CseUnaryOp::Sqrt, *a)),
    SSAOp::Sin(a) => Some(CseKey::Unary(CseUnaryOp::Sin, *a)),
    SSAOp::Cos(a) => Some(CseKey::Unary(CseUnaryOp::Cos, *a)),
    SSAOp::Tan(a) => Some(CseKey::Unary(CseUnaryOp::Tan, *a)),
    SSAOp::Exp(a) => Some(CseKey::Unary(CseUnaryOp::Exp, *a)),
    SSAOp::Ln(a) => Some(CseKey::Unary(CseUnaryOp::Ln, *a)),
    SSAOp::Not(a) => Some(CseKey::Unary(CseUnaryOp::Not, *a)),
    SSAOp::Add(a, b) => Some(CseKey::Binary(CseBinaryOp::Add, *a, *b)),
    SSAOp::Sub(a, b) => Some(CseKey::Binary(CseBinaryOp::Sub, *a, *b)),
    SSAOp::Mul(a, b) => Some(CseKey::Binary(CseBinaryOp::Mul, *a, *b)),
    SSAOp::Div(a, b) => Some(CseKey::Binary(CseBinaryOp::Div, *a, *b)),
    SSAOp::Mod(a, b) => Some(CseKey::Binary(CseBinaryOp::Mod, *a, *b)),
    SSAOp::Pow(a, b) => Some(CseKey::Binary(CseBinaryOp::Pow, *a, *b)),
    SSAOp::Lt(a, b) => Some(CseKey::Binary(CseBinaryOp::Lt, *a, *b)),
    SSAOp::Gt(a, b) => Some(CseKey::Binary(CseBinaryOp::Gt, *a, *b)),
    SSAOp::Le(a, b) => Some(CseKey::Binary(CseBinaryOp::Le, *a, *b)),
    SSAOp::Ge(a, b) => Some(CseKey::Binary(CseBinaryOp::Ge, *a, *b)),
    SSAOp::Eq(a, b) => Some(CseKey::Binary(CseBinaryOp::Eq, *a, *b)),
    SSAOp::Ne(a, b) => Some(CseKey::Binary(CseBinaryOp::Ne, *a, *b)),
    SSAOp::And(a, b) => Some(CseKey::Binary(CseBinaryOp::And, *a, *b)),
    SSAOp::Or(a, b) => Some(CseKey::Binary(CseBinaryOp::Or, *a, *b)),
    _ => None,
  }
}

/// CSE (Common Subexpression Elimination): 동일한 연산을 중복 계산하지 않음
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn ssa_cse(block: SSABlock) -> SSABlock {
  let mut table: HashMap<CseKey, SSAValue> = HashMap::new();
  let mut remap: HashMap<SSAValue, SSAValue> = HashMap::new();
  let mut new_ops = Vec::new();

  for (val, op) in block.ops {
    // 상수와 로드는 CSE하지 않음 (side-effect 가능성)
    let should_cse = matches!(
      op,
      SSAOp::Add(_, _)
        | SSAOp::Sub(_, _)
        | SSAOp::Mul(_, _)
        | SSAOp::Div(_, _)
        | SSAOp::Mod(_, _)
        | SSAOp::Pow(_, _)
        | SSAOp::Neg(_)
        | SSAOp::Floor(_)
        | SSAOp::Ceil(_)
        | SSAOp::Abs(_)
        | SSAOp::Sqrt(_)
        | SSAOp::Sin(_)
        | SSAOp::Cos(_)
        | SSAOp::Tan(_)
        | SSAOp::Exp(_)
        | SSAOp::Ln(_)
        | SSAOp::Lt(_, _)
        | SSAOp::Gt(_, _)
        | SSAOp::Le(_, _)
        | SSAOp::Ge(_, _)
        | SSAOp::Eq(_, _)
        | SSAOp::Ne(_, _)
        | SSAOp::And(_, _)
        | SSAOp::Or(_, _)
        | SSAOp::Not(_)
    );

    if should_cse {
      // 입력을 remap된 값으로 대체
      let remapped_op = remap_op_inputs(&op, &remap);
      let key = match cse_key(&remapped_op) {
        Some(key) => key,
        None => {
          new_ops.push((val, remapped_op));
          continue;
        }
      };

      if let Some(&existing) = table.get(&key) {
        // 이미 계산된 값 재사용
        remap.insert(val, existing);
        new_ops.push((val, SSAOp::Alias(existing)));
      } else {
        table.insert(key, val);
        new_ops.push((val, remapped_op));
      }
    } else {
      let remapped_op = remap_op_inputs(&op, &remap);
      new_ops.push((val, remapped_op));
    }
  }

  // ret도 remap
  let ret = *remap.get(&block.ret).unwrap_or(&block.ret);

  SSABlock {
    label: block.label,
    ops: new_ops,
    ret,
  }
}

/// op의 입력 레지스터를 remap
fn remap_op_inputs(op: &SSAOp, remap: &HashMap<SSAValue, SSAValue>) -> SSAOp {
  let r = |v: SSAValue| *remap.get(&v).unwrap_or(&v);

  match op {
    SSAOp::ConstInt(v) => SSAOp::ConstInt(*v),
    SSAOp::ConstFloat(v) => SSAOp::ConstFloat(*v),
    SSAOp::ConstBool(v) => SSAOp::ConstBool(*v),
    SSAOp::ConstString(s) => SSAOp::ConstString(s.clone()),
    SSAOp::LoadTime => SSAOp::LoadTime,
    SSAOp::LoadDeltaTime => SSAOp::LoadDeltaTime,
    SSAOp::LoadSignal(id) => SSAOp::LoadSignal(*id),
    SSAOp::LoadVar(name) => SSAOp::LoadVar(name.clone()),
    SSAOp::LoadAttr(base, attr) => SSAOp::LoadAttr(r(*base), attr.clone()),
    SSAOp::Lambda {
      param,
      body,
      captures,
      self_name,
    } => SSAOp::Lambda {
      param: param.clone(),
      body: body.clone(),
      captures: captures
        .iter()
        .map(|(name, value)| (name.clone(), r(*value)))
        .collect(),
      self_name: self_name.clone(),
    },
    SSAOp::Call { func, args } => SSAOp::Call {
      func: r(*func),
      args: args.iter().map(|a| r(*a)).collect(),
    },
    SSAOp::TailCall { func, args } => SSAOp::TailCall {
      func: r(*func),
      args: args.iter().map(|a| r(*a)).collect(),
    },

    SSAOp::Neg(a) => SSAOp::Neg(r(*a)),
    SSAOp::Floor(a) => SSAOp::Floor(r(*a)),
    SSAOp::Ceil(a) => SSAOp::Ceil(r(*a)),
    SSAOp::Abs(a) => SSAOp::Abs(r(*a)),
    SSAOp::Sqrt(a) => SSAOp::Sqrt(r(*a)),
    SSAOp::Sin(a) => SSAOp::Sin(r(*a)),
    SSAOp::Cos(a) => SSAOp::Cos(r(*a)),
    SSAOp::Tan(a) => SSAOp::Tan(r(*a)),
    SSAOp::Exp(a) => SSAOp::Exp(r(*a)),
    SSAOp::Ln(a) => SSAOp::Ln(r(*a)),
    SSAOp::Not(a) => SSAOp::Not(r(*a)),

    SSAOp::Add(a, b) => SSAOp::Add(r(*a), r(*b)),
    SSAOp::Sub(a, b) => SSAOp::Sub(r(*a), r(*b)),
    SSAOp::Mul(a, b) => SSAOp::Mul(r(*a), r(*b)),
    SSAOp::Div(a, b) => SSAOp::Div(r(*a), r(*b)),
    SSAOp::Mod(a, b) => SSAOp::Mod(r(*a), r(*b)),
    SSAOp::Pow(a, b) => SSAOp::Pow(r(*a), r(*b)),
    SSAOp::Lt(a, b) => SSAOp::Lt(r(*a), r(*b)),
    SSAOp::Gt(a, b) => SSAOp::Gt(r(*a), r(*b)),
    SSAOp::Le(a, b) => SSAOp::Le(r(*a), r(*b)),
    SSAOp::Ge(a, b) => SSAOp::Ge(r(*a), r(*b)),
    SSAOp::Eq(a, b) => SSAOp::Eq(r(*a), r(*b)),
    SSAOp::Ne(a, b) => SSAOp::Ne(r(*a), r(*b)),
    SSAOp::And(a, b) => SSAOp::And(r(*a), r(*b)),
    SSAOp::Or(a, b) => SSAOp::Or(r(*a), r(*b)),

    SSAOp::Select(c, t, e) => SSAOp::Select(r(*c), r(*t), r(*e)),

    SSAOp::ListConstruct(items) => {
      SSAOp::ListConstruct(items.iter().map(|item| r(*item)).collect())
    }
    SSAOp::AttrSetConstruct(pairs) => SSAOp::AttrSetConstruct(
      pairs
        .iter()
        .map(|(key, value)| (key.clone(), r(*value)))
        .collect(),
    ),

    SSAOp::Derived(meta, args) => {
      SSAOp::Derived(meta.clone(), args.iter().map(|a| r(*a)).collect())
    }

    SSAOp::Alias(a) => SSAOp::Alias(r(*a)),

    SSAOp::CallExtern { name, args } => SSAOp::CallExtern {
      name: name.clone(),
      args: args.clone(),
    },

    SSAOp::Throw(msg) => SSAOp::Throw(msg.clone()),
  }
}

// ============================================================
// DCE - Dead Code Elimination
// ============================================================

/// DCE: 사용되지 않는 연산 제거
/// DCE (Dead Code Elimination): 사용되지 않는 코드 제거
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn ssa_dce(block: SSABlock) -> SSABlock {
  let SSABlock { label, ops, ret } = block;

  // Lambda body도 재귀적으로 DCE 적용 (중첩 블록 최적화)
  let ops_with_inner: Vec<(SSAValue, SSAOp)> = ops
    .into_iter()
    .map(|(val, op)| {
      let op = match op {
        SSAOp::Lambda {
          param,
          body,
          captures,
          self_name,
        } => {
          let optimized_body = ssa_dce(*body);
          SSAOp::Lambda {
            param,
            body: Box::new(optimized_body),
            captures,
            self_name,
          }
        }
        other => other,
      };
      (val, op)
    })
    .collect();
  // 1. ret에서 시작해서 역방향으로 live set 계산
  let mut live: HashSet<usize> = HashSet::new();
  live.insert(ret.0);

  // 역방향 순회
  for (val, op) in ops_with_inner.iter().rev() {
    if live.contains(&val.0) {
      for input in op.inputs() {
        live.insert(input.0);
      }
    }
  }

  // 2. live한 op만 유지
  let new_ops: Vec<_> = ops_with_inner
    .into_iter()
    .filter(|(val, _)| live.contains(&val.0))
    .collect();

  SSABlock {
    label,
    ops: new_ops,
    ret,
  }
}

// ============================================================
// Alias Removal
// ============================================================

/// Alias op 제거 (실제로 사용하는 레지스터로 대체)
/// Alias 제거: 체인 해결 후 직접 참조로 변경
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn ssa_remove_aliases(block: SSABlock) -> SSABlock {
  // 1. Alias 체인 해결 (사이클 감지 포함)
  // 먼저 모든 Alias 관계를 수집
  let mut alias_edges: HashMap<SSAValue, SSAValue> = HashMap::new();
  for (val, op) in &block.ops {
    if let SSAOp::Alias(target) = op {
      alias_edges.insert(*val, *target);
    }
  }

  // 각 Alias에 대해 체인을 따라가면서 사이클 감지
  let mut alias_map: HashMap<SSAValue, SSAValue> = HashMap::new();
  for (val, target) in &alias_edges {
    let mut final_target = *target;
    let mut visited = std::collections::HashSet::new();
    visited.insert(*val);

    // 체인을 따라가면서 사이클 감지
    while let Some(&next) = alias_edges.get(&final_target) {
      if visited.contains(&final_target) {
        // 사이클 감지: 이미 방문한 노드를 다시 방문
        // 사이클을 깨기 위해 원본 target 사용
        final_target = *target;
        break;
      }
      visited.insert(final_target);
      final_target = next;
    }

    // 최종적으로 자기 자신을 참조하는 경우 원본 target 사용
    if final_target == *val {
      final_target = *target;
    }

    alias_map.insert(*val, final_target);
  }

  // 2. Alias가 아닌 op들만 유지하고 입력 remap
  let new_ops: Vec<_> = block
    .ops
    .into_iter()
    .filter(|(_, op)| !matches!(op, SSAOp::Alias(_)))
    .map(|(val, op)| (val, remap_op_inputs(&op, &alias_map)))
    .collect();

  // 3. ret remap
  let ret = *alias_map.get(&block.ret).unwrap_or(&block.ret);

  SSABlock {
    label: block.label,
    ops: new_ops,
    ret,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn make_block(ops: Vec<(SSAValue, SSAOp)>, ret: SSAValue) -> SSABlock {
    SSABlock {
      label: "test".to_string(),
      ops,
      ret,
    }
  }

  #[test]
  fn test_ssa_cse() {
    // 동일한 연산이 두 번 나오면 CSE가 Alias로 대체
    let ops = vec![
      (SSAValue(0), SSAOp::ConstInt(1)),
      (SSAValue(1), SSAOp::ConstInt(2)),
      (SSAValue(2), SSAOp::Add(SSAValue(0), SSAValue(1))), // 1 + 2
      (SSAValue(3), SSAOp::Add(SSAValue(0), SSAValue(1))), // 1 + 2 again (중복)
      (SSAValue(4), SSAOp::Add(SSAValue(2), SSAValue(3))), // 결과 사용
    ];
    let block = make_block(ops, SSAValue(4));
    let optimized = ssa_cse(block);

    // %3은 %2의 Alias가 되어야 함
    let has_alias = optimized
      .ops
      .iter()
      .any(|(_, op)| matches!(op, SSAOp::Alias(_)));
    assert!(has_alias, "CSE should create an alias");
  }

  #[test]
  fn test_ssa_dce() {
    // 사용되지 않는 연산 제거
    let ops = vec![
      (SSAValue(0), SSAOp::ConstInt(1)),
      (SSAValue(1), SSAOp::ConstInt(2)),
      (SSAValue(2), SSAOp::ConstInt(3)), // unused
      (SSAValue(3), SSAOp::Add(SSAValue(0), SSAValue(1))),
    ];
    let block = make_block(ops, SSAValue(3));
    let optimized = ssa_dce(block);

    // %2 (unused) 가 제거되어야 함
    assert_eq!(optimized.len(), 3);
  }

  #[test]
  fn test_ssa_dce_keeps_lambda_captures() {
    let lambda_body = SSABlock {
      label: "lambda".to_string(),
      ops: vec![(SSAValue(0), SSAOp::ConstInt(1))],
      ret: SSAValue(0),
    };
    let ops = vec![
      (SSAValue(0), SSAOp::ConstInt(42)),
      (
        SSAValue(1),
        SSAOp::Lambda {
          param: "x".to_string(),
          body: Box::new(lambda_body),
          captures: vec![("cap".to_string(), SSAValue(0))],
          self_name: None,
        },
      ),
    ];
    let block = make_block(ops, SSAValue(1));
    let optimized = ssa_dce(block);

    assert_eq!(optimized.len(), 2);
  }

  #[test]
  fn test_ssa_dce_recurses_into_lambda_body() {
    let lambda_body = SSABlock {
      label: "lambda".to_string(),
      ops: vec![
        (SSAValue(0), SSAOp::ConstInt(1)),
        (SSAValue(1), SSAOp::ConstInt(2)), // unused
      ],
      ret: SSAValue(0),
    };
    let ops = vec![(
      SSAValue(0),
      SSAOp::Lambda {
        param: "x".to_string(),
        body: Box::new(lambda_body),
        captures: vec![],
        self_name: None,
      },
    )];
    let block = make_block(ops, SSAValue(0));
    let optimized = ssa_dce(block);

    let body_len = match &optimized.ops[0].1 {
      SSAOp::Lambda { body, .. } => body.ops.len(),
      _ => panic!("Expected Lambda"),
    };
    assert_eq!(body_len, 1);
  }

  #[test]
  fn test_ssa_remove_aliases() {
    let ops = vec![
      (SSAValue(0), SSAOp::ConstInt(42)),
      (SSAValue(1), SSAOp::Alias(SSAValue(0))),
      (SSAValue(2), SSAOp::Add(SSAValue(1), SSAValue(0))),
    ];
    let block = make_block(ops, SSAValue(2));
    let optimized = ssa_remove_aliases(block);

    // Alias가 제거되고 입력이 remap됨
    assert_eq!(optimized.len(), 2);
    match &optimized.ops[1].1 {
      SSAOp::Add(a, b) => {
        assert_eq!(*a, SSAValue(0));
        assert_eq!(*b, SSAValue(0));
      }
      _ => panic!("Expected Add"),
    }
  }

  #[test]
  fn test_optimize_ssa_pipeline() {
    // CSE → DCE → Alias Removal 전체 파이프라인 테스트
    let ops = vec![
      (SSAValue(0), SSAOp::ConstInt(1)),
      (SSAValue(1), SSAOp::ConstInt(2)),
      (SSAValue(2), SSAOp::Add(SSAValue(0), SSAValue(1))),
      (SSAValue(3), SSAOp::Add(SSAValue(0), SSAValue(1))), // CSE될 중복
      (SSAValue(4), SSAOp::ConstInt(999)),                 // DCE될 미사용
      (SSAValue(5), SSAOp::Mul(SSAValue(2), SSAValue(3))),
    ];
    let block = make_block(ops, SSAValue(5));
    let optimized = optimize_ssa(block);

    // 최적화 후: ConstInt(1), ConstInt(2), Add, Mul (4개)
    // - %3은 %2로 대체됨 (CSE)
    // - %4는 제거됨 (DCE)
    // - Alias는 제거됨 (Alias Removal)
    assert!(
      optimized.len() <= 4,
      "Expected 4 or fewer ops after optimization, got {}",
      optimized.len()
    );
  }
}
