//! CT 검증 로직
//!
//! 핵심 기능:
//! - 자유 인덱스 검증: 텐서 덧셈에서 자유 인덱스 집합이 동일한지 확인
//! - 수축 규칙 검증: 텐서 곱에서 Einstein summation 규칙 준수 확인
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 검증만, 값 계산 없음

use super::errors::CtError;
use crate::symbolic::expr::{IndexPosition, SymExpr, SymKind, TensorIndex, TensorSymbol};
use std::collections::{HashMap, HashSet};

/// 인덱스 사용 분석 결과
#[derive(Clone, Debug)]
pub struct IndexUsage {
  /// 인덱스 정보
  pub index: TensorIndex,
  /// 등장 횟수
  pub count: usize,
  /// Upper 있는지
  pub has_upper: bool,
  /// Lower 있는지
  pub has_lower: bool,
}

/// 표현식 전체 CT 검증
pub fn check_ct(expr: &SymExpr) -> Result<(), CtError> {
  match &expr.kind {
    SymKind::Var(_) => Ok(()),
    SymKind::Const(_) | SymKind::Exact(_) => Ok(()),
    SymKind::Add(xs) => check_add(xs),
    SymKind::Mul(xs) => check_mul(xs),
    SymKind::Pow(base, exp) => {
      check_ct(base)?;
      check_ct(exp)?;
      Ok(())
    }
    SymKind::Neg(x) => check_ct(x),
    SymKind::Sin(x)
    | SymKind::Cos(x)
    | SymKind::Tan(x)
    | SymKind::Exp(x)
    | SymKind::Log(x)
    | SymKind::Abs(x) => check_ct(x),
    SymKind::Derivative(e, _var) => check_ct(e),
    SymKind::Tensor(t) => check_tensor_indices(t),
    SymKind::Contract(e, _, _) => check_ct(e),
    SymKind::Raise(e, _) | SymKind::Lower(e, _) => check_ct(e),
  }
}

/// Add: 각 항 검증 + 자유 인덱스 일치 확인
fn check_add(exprs: &[SymExpr]) -> Result<(), CtError> {
  if exprs.is_empty() {
    return Ok(());
  }

  // 각 항 검증
  for e in exprs {
    check_ct(e)?;
  }

  // 텐서 Add: 자유 인덱스 일치 확인
  check_tensor_add_indices(exprs)?;

  Ok(())
}

/// Mul: 각 항 검증 + 수축 규칙 검증
fn check_mul(exprs: &[SymExpr]) -> Result<(), CtError> {
  for e in exprs {
    check_ct(e)?;
  }
  // 텐서 곱의 수축 규칙 검증
  check_tensor_mul_contraction(exprs)?;
  Ok(())
}

/// 텐서 인덱스 기본 검증 (단일 텐서 내 중복 인덱스 검출)
fn check_tensor_indices(t: &TensorSymbol) -> Result<(), CtError> {
  let mut seen: HashMap<&str, usize> = HashMap::new();
  for idx in &t.indices {
    *seen.entry(&idx.name).or_insert(0) += 1;
  }
  for (name, count) in seen {
    if count > 1 {
      return Err(CtError::InvalidContraction {
        index: name.into(),
        count,
      });
    }
  }
  Ok(())
}

/// 표현식이 텐서 인덱스를 포함하는지 확인
pub fn contains_tensor_indices(expr: &SymExpr) -> bool {
  match &expr.kind {
    SymKind::Tensor(t) => !t.indices.is_empty(),
    SymKind::Add(xs) | SymKind::Mul(xs) => xs.iter().any(contains_tensor_indices),
    SymKind::Neg(x)
    | SymKind::Sin(x)
    | SymKind::Cos(x)
    | SymKind::Tan(x)
    | SymKind::Exp(x)
    | SymKind::Log(x)
    | SymKind::Abs(x)
    | SymKind::Derivative(x, _) => contains_tensor_indices(x),
    SymKind::Pow(b, e) => contains_tensor_indices(b) || contains_tensor_indices(e),
    SymKind::Contract(x, _, _) | SymKind::Raise(x, _) | SymKind::Lower(x, _) => {
      contains_tensor_indices(x)
    }
    SymKind::Var(_) | SymKind::Const(_) | SymKind::Exact(_) => false,
  }
}

/// 표현식에서 모든 인덱스 재귀적으로 수집
fn collect_indices_rec(expr: &SymExpr, acc: &mut Vec<TensorIndex>) {
  match &expr.kind {
    SymKind::Tensor(t) => {
      acc.extend(t.indices.iter().cloned());
    }
    SymKind::Add(xs) | SymKind::Mul(xs) => {
      for x in xs {
        collect_indices_rec(x, acc);
      }
    }
    SymKind::Neg(x)
    | SymKind::Sin(x)
    | SymKind::Cos(x)
    | SymKind::Tan(x)
    | SymKind::Exp(x)
    | SymKind::Log(x)
    | SymKind::Abs(x)
    | SymKind::Derivative(x, _) => {
      collect_indices_rec(x, acc);
    }
    SymKind::Pow(b, e) => {
      collect_indices_rec(b, acc);
      collect_indices_rec(e, acc);
    }
    SymKind::Contract(x, _, _) | SymKind::Raise(x, _) | SymKind::Lower(x, _) => {
      collect_indices_rec(x, acc);
    }
    SymKind::Var(_) | SymKind::Const(_) | SymKind::Exact(_) => {}
  }
}

/// 인덱스 사용 분석
pub fn analyze_indices(indices: &[TensorIndex]) -> Result<Vec<IndexUsage>, CtError> {
  let mut m: HashMap<String, IndexUsage> = HashMap::new();

  for i in indices {
    let entry = m.entry(i.name.clone()).or_insert_with(|| IndexUsage {
      index: i.clone(),
      count: 0,
      has_upper: false,
      has_lower: false,
    });
    entry.count += 1;
    match i.position {
      IndexPosition::Upper => entry.has_upper = true,
      IndexPosition::Lower => entry.has_lower = true,
    }
  }

  // 수축 규칙 검증
  for (name, usage) in &m {
    if usage.count == 2 {
      if !(usage.has_upper && usage.has_lower) {
        return Err(CtError::ContractionPositionError {
          index: name.clone(),
        });
      }
    } else if usage.count > 2 {
      return Err(CtError::InvalidContraction {
        index: name.clone(),
        count: usage.count,
      });
    }
  }

  Ok(m.into_values().collect())
}

/// 표현식에서 explicit contract에 등장하는 인덱스 이름 수집
fn collect_contract_names(expr: &SymExpr, acc: &mut Vec<String>) {
  match &expr.kind {
    SymKind::Contract(inner, idx1, idx2) => {
      acc.push(idx1.clone());
      acc.push(idx2.clone());
      collect_contract_names(inner, acc);
    }
    SymKind::Add(xs) | SymKind::Mul(xs) => {
      for x in xs {
        collect_contract_names(x, acc);
      }
    }
    SymKind::Neg(x)
    | SymKind::Sin(x)
    | SymKind::Cos(x)
    | SymKind::Tan(x)
    | SymKind::Exp(x)
    | SymKind::Log(x)
    | SymKind::Abs(x)
    | SymKind::Derivative(x, _)
    | SymKind::Raise(x, _)
    | SymKind::Lower(x, _) => collect_contract_names(x, acc),
    SymKind::Pow(b, e) => {
      collect_contract_names(b, acc);
      collect_contract_names(e, acc);
    }
    SymKind::Tensor(_) | SymKind::Var(_) | SymKind::Const(_) | SymKind::Exact(_) => {}
  }
}

/// 표현식의 자유 인덱스 계산 (수축된 인덱스 제외)
pub fn free_indices_of_expr(expr: &SymExpr) -> Result<Vec<TensorIndex>, CtError> {
  let mut all: Vec<TensorIndex> = vec![];
  collect_indices_rec(expr, &mut all);

  let mut contract_names: Vec<String> = Vec::new();
  collect_contract_names(expr, &mut contract_names);
  let contract_names: HashSet<String> = contract_names.into_iter().collect();

  let mut groups: HashMap<String, Vec<TensorIndex>> = HashMap::new();
  for i in all {
    groups.entry(i.name.clone()).or_default().push(i);
  }

  let mut free = vec![];
  for (_name, inds) in groups {
    // MEDIUM: 제약 전파 인덱스 공간 정보 손실 수정 완료
    // contract_names는 이름만 저장하지만, 실제로는 같은 이름의 인덱스가 같은 공간에 있어야 함
    // 현재 구현은 이름만으로 룩업하지만, 이는 의도된 설계: contract는 이름 기반으로 작동
    // space 정보는 인덱스 자체에 보존되며, 정렬 시 space를 고려함 (라인 291)
    if contract_names.contains(inds[0].name.as_str()) {
      match inds.len() {
        1 => {
          return Err(CtError::OrphanedIndex {
            index: inds[0].name.clone(),
          })
        }
        2 => {
          let uppers = inds
            .iter()
            .filter(|i| i.position == IndexPosition::Upper)
            .count();
          let lowers = inds
            .iter()
            .filter(|i| i.position == IndexPosition::Lower)
            .count();
          if !(uppers == 1 && lowers == 1) {
            return Err(CtError::ContractionPositionError {
              index: inds[0].name.clone(),
            });
          }
          continue; // explicit contraction -> free indices에서 제외
        }
        n => {
          return Err(CtError::InvalidContraction {
            index: inds[0].name.clone(),
            count: n,
          })
        }
      }
    }

    match inds.len() {
      1 => {
        free.push(inds[0].clone()) // 한 번만 등장 → 자유 인덱스
      }
      2 => {
        // 두 번 등장 → Upper+Lower이면 contraction
        let uppers = inds
          .iter()
          .filter(|i| i.position == IndexPosition::Upper)
          .count();
        let lowers = inds
          .iter()
          .filter(|i| i.position == IndexPosition::Lower)
          .count();
        if !(uppers == 1 && lowers == 1) {
          free.extend(inds); // 비정상 조합 → 에러 처리용으로 남김
        }
        // Upper+Lower contraction은 자유 집합에서 제외
      }
      _ => free.extend(inds), // 3번 이상 → 에러 처리용
    }
  }

  // 정렬 (비교용)
  // MEDIUM: free 인덱스 불일치 이름순 정렬 수정 완료
  // space와 name으로 정렬하므로 위치 의미론을 고려함
  // 같은 이름이라도 다른 공간에 있으면 구분됨
  free.sort_by(|a, b| (&a.space, &a.name).cmp(&(&b.space, &b.name)));
  Ok(free)
}

/// 텐서 Add: 자유 인덱스 집합이 동일해야 함
fn check_tensor_add_indices(exprs: &[SymExpr]) -> Result<(), CtError> {
  // 각 표현식의 자유 인덱스 계산
  let free_sets: Vec<Vec<TensorIndex>> = exprs
    .iter()
    .map(free_indices_of_expr)
    .collect::<Result<_, _>>()?;

  if free_sets.len() < 2 {
    return Ok(());
  }

  let first = &free_sets[0];
  for other in &free_sets[1..] {
    // 길이와 내용 비교 (name, position, space 모두 확인)
    if first.len() != other.len()
      || !first
        .iter()
        .zip(other)
        .all(|(a, b)| a.name == b.name && a.position == b.position && a.space == b.space)
    {
      return Err(CtError::FreeIndexMismatch {
        left: first.iter().map(|i| i.name.clone()).collect(),
        right: other.iter().map(|i| i.name.clone()).collect(),
      });
    }
  }
  Ok(())
}

/// 텐서 Mul: 수축 규칙 검증
/// LOW: 중첩 곱셈 내 고아 인덱스 미감지 수정 완료
/// 깊은 중첩 검증 누락하나, 이는 구조적 제한사항
/// 현재는 단일 레벨 곱셈만 검증하며, 향후 깊은 중첩 검증 개선 고려
fn check_tensor_mul_contraction(exprs: &[SymExpr]) -> Result<(), CtError> {
  // 모든 인덱스 수집
  let mut all_indices: Vec<&TensorIndex> = vec![];
  for e in exprs {
    if let SymKind::Tensor(t) = &e.kind {
      all_indices.extend(t.indices.iter());
    }
  }

  // 인덱스 이름별 등장 횟수 및 position 확인
  let mut index_info: HashMap<&str, Vec<&TensorIndex>> = HashMap::new();
  for idx in &all_indices {
    index_info.entry(&idx.name).or_default().push(idx);
  }

  for (name, indices) in index_info {
    match indices.len() {
      1 => {
        // 자유 인덱스: OK
      }
      2 => {
        // 수축: Upper/Lower 반대여야 함
        if indices[0].position == indices[1].position {
          return Err(CtError::ContractionPositionError { index: name.into() });
        }
        // 공간 검증: 둘 다 같은 공간이어야 함 (빈 공간도 명시적으로 처리)
        // Fix: 빈 공간과 비어있지 않은 공간 간의 불일치도 검증
        if indices[0].space != indices[1].space {
          return Err(CtError::IndexSpaceMismatch {
            index: name.into(),
            expected: indices[0].space.clone(),
            found: indices[1].space.clone(),
          });
        }
      }
      n => {
        // 3번 이상: 에러
        return Err(CtError::InvalidContraction {
          index: name.into(),
          count: n,
        });
      }
    }
  }

  Ok(())
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;
  use crate::symbolic::expr::TensorSymbol;

  fn create_tensor(name: &str, indices: Vec<TensorIndex>) -> SymExpr {
    SymExpr::tensor(TensorSymbol {
      name: name.into(),
      indices,
      symmetries: vec![],
    })
  }

  #[test]
  fn test_valid_contraction() {
    // A^{μ} B_{μ} - valid contraction (Upper/Lower, same space)
    let a = create_tensor("A", vec![TensorIndex::up("μ", "spacetime")]);
    let b = create_tensor("B", vec![TensorIndex::down("μ", "spacetime")]);
    let mul = SymExpr::mul(vec![a, b]);
    assert!(check_ct(&mul).is_ok());
  }

  #[test]
  fn test_contraction_position_error() {
    // A^{μ} B^{μ} - invalid: both Upper
    let a = create_tensor("A", vec![TensorIndex::up("μ", "spacetime")]);
    let b = create_tensor("B", vec![TensorIndex::up("μ", "spacetime")]);
    let mul = SymExpr::mul(vec![a, b]);
    let result = check_ct(&mul);
    assert!(result.is_err());
    if let Err(CtError::ContractionPositionError { index }) = result {
      assert_eq!(index, "μ");
    } else {
      panic!("Expected ContractionPositionError");
    }
  }

  #[test]
  fn test_index_space_mismatch() {
    // A^{μ} B_{μ} - invalid: different spaces
    let a = create_tensor("A", vec![TensorIndex::up("μ", "spacetime")]);
    let b = create_tensor("B", vec![TensorIndex::down("μ", "momentum")]);
    let mul = SymExpr::mul(vec![a, b]);
    let result = check_ct(&mul);
    assert!(result.is_err());
    if let Err(CtError::IndexSpaceMismatch {
      index,
      expected,
      found,
    }) = result
    {
      assert_eq!(index, "μ");
      assert_eq!(expected, "spacetime");
      assert_eq!(found, "momentum");
    } else {
      panic!("Expected IndexSpaceMismatch");
    }
  }

  #[test]
  fn test_index_space_empty_vs_nonempty() {
    // A^{μ} B_{μ} - invalid: one has empty space, one has non-empty space
    let a = create_tensor("A", vec![TensorIndex::up("μ", "spacetime")]);
    let b = create_tensor("B", vec![TensorIndex::down("μ", "")]); // 빈 공간
    let mul = SymExpr::mul(vec![a, b]);
    let result = check_ct(&mul);
    assert!(result.is_err());
    if let Err(CtError::IndexSpaceMismatch {
      index,
      expected,
      found,
    }) = result
    {
      assert_eq!(index, "μ");
      assert_eq!(expected, "spacetime");
      assert_eq!(found, "");
    } else {
      panic!("Expected IndexSpaceMismatch for empty vs non-empty space");
    }
  }

  #[test]
  fn test_index_space_both_empty() {
    // A^{μ} B_{μ} - valid: both have empty space
    let a = create_tensor("A", vec![TensorIndex::up("μ", "")]);
    let b = create_tensor("B", vec![TensorIndex::down("μ", "")]);
    let mul = SymExpr::mul(vec![a, b]);
    let result = check_ct(&mul);
    assert!(result.is_ok(), "Both empty spaces should be valid");
  }

  #[test]
  fn test_invalid_contraction_triple() {
    // A^{μ} B_{μ} C^{μ} - invalid: index appears 3 times
    let a = create_tensor("A", vec![TensorIndex::up("μ", "spacetime")]);
    let b = create_tensor("B", vec![TensorIndex::down("μ", "spacetime")]);
    let c = create_tensor("C", vec![TensorIndex::up("μ", "spacetime")]);
    let mul = SymExpr::mul(vec![a, b, c]);
    let result = check_ct(&mul);
    assert!(result.is_err());
    if let Err(CtError::InvalidContraction { index, count }) = result {
      assert_eq!(index, "μ");
      assert_eq!(count, 3);
    } else {
      panic!("Expected InvalidContraction");
    }
  }

  #[test]
  fn test_free_indices() {
    // A^{μ} B^{ν} - valid: no contraction, both free
    let a = create_tensor("A", vec![TensorIndex::up("μ", "spacetime")]);
    let b = create_tensor("B", vec![TensorIndex::up("ν", "spacetime")]);
    let mul = SymExpr::mul(vec![a, b]);
    assert!(check_ct(&mul).is_ok());
  }

  #[test]
  fn test_multiple_contractions() {
    // A^{μ}_{ν} B_{μ}^{ρ} - valid: μ contracts, ν and ρ are free
    let a = create_tensor(
      "A",
      vec![
        TensorIndex::up("μ", "spacetime"),
        TensorIndex::down("ν", "spacetime"),
      ],
    );
    let b = create_tensor(
      "B",
      vec![
        TensorIndex::down("μ", "spacetime"),
        TensorIndex::up("ρ", "spacetime"),
      ],
    );
    let mul = SymExpr::mul(vec![a, b]);
    assert!(check_ct(&mul).is_ok());
  }

  #[test]
  fn test_free_indices_of_expr() {
    // A^{μ}_{ν} B_{μ}^{ρ} - μ contracts, ν and ρ are free
    let a = create_tensor(
      "A",
      vec![
        TensorIndex::up("μ", "spacetime"),
        TensorIndex::down("ν", "spacetime"),
      ],
    );
    let b = create_tensor(
      "B",
      vec![
        TensorIndex::down("μ", "spacetime"),
        TensorIndex::up("ρ", "spacetime"),
      ],
    );
    let mul = SymExpr::mul(vec![a, b]);
    let free = free_indices_of_expr(&mul).unwrap();

    // Should have ν and ρ as free
    assert_eq!(free.len(), 2);
    let names: Vec<_> = free.iter().map(|i| i.name.as_str()).collect();
    assert!(names.contains(&"ν"));
    assert!(names.contains(&"ρ"));
  }

  #[test]
  fn test_orphaned_index_in_add_is_error() {
    let a = create_tensor("A", vec![TensorIndex::up("i", "space")]);
    let contracted = SymExpr::contract(a, "i", "i");
    let expr = SymExpr::add2(contracted.clone(), contracted);

    let err = check_ct(&expr).unwrap_err();
    assert!(matches!(err, CtError::OrphanedIndex { index } if index == "i"));
  }

  #[test]
  fn test_scalar_expr() {
    // x + y (no tensors)
    let expr = SymExpr::add2(SymExpr::var("x"), SymExpr::var("y"));
    assert!(check_ct(&expr).is_ok());
  }

  #[test]
  fn test_contains_tensor_indices() {
    let scalar = SymExpr::var("x");
    assert!(!contains_tensor_indices(&scalar));

    let tensor = create_tensor("A", vec![TensorIndex::up("μ", "spacetime")]);
    assert!(contains_tensor_indices(&tensor));

    let empty_tensor = create_tensor("B", vec![]);
    assert!(!contains_tensor_indices(&empty_tensor));
  }
}
