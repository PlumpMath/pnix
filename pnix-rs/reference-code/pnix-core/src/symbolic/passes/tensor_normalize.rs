//! 텐서 정규화 패스
//!
//! pnix-old의 symbolic_core/passes/tensor_normalize.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 변환만, 값 계산 없음
//!
//! ## 기능
//!
//! - 인덱스 순서 정렬 (대칭성 활용)
//! - 자유/수축 인덱스 분리 메타 계산

use crate::symbolic::expr::{SymExpr, SymKind, Symmetry, TensorSymbol};
use std::collections::HashSet;

/// 텐서 표현 정규화
///
/// - 대칭 텐서의 인덱스 정렬
/// - 곱에서 수축 인덱스 식별
pub fn tensor_normalize(expr: SymExpr) -> SymExpr {
  match &expr.kind {
    SymKind::Tensor(t) => {
      // 대칭 텐서의 인덱스 정렬
      let sorted_t = sort_symmetric_indices(t.clone());
      SymExpr::tensor(sorted_t)
    }
    SymKind::Mul(xs) => {
      // 곱에서 각 항 정규화
      let normalized: Vec<_> = xs.iter().map(|e| tensor_normalize(e.clone())).collect();
      SymExpr::mul(normalized)
    }
    SymKind::Add(xs) => {
      // 합에서 각 항 정규화
      let normalized: Vec<_> = xs.iter().map(|e| tensor_normalize(e.clone())).collect();
      SymExpr::add(normalized)
    }
    _ => expr,
  }
}

/// 대칭 텐서의 인덱스 정렬
fn sort_symmetric_indices(mut t: TensorSymbol) -> TensorSymbol {
  for sym in &t.symmetries {
    match sym {
      Symmetry::Symmetric(positions) => {
        // 해당 위치들의 인덱스를 이름순 정렬
        let mut to_sort: Vec<_> = positions
          .iter()
          .filter_map(|&p| t.indices.get(p).cloned().map(|i| (p, i)))
          .collect();
        to_sort.sort_by(|a, b| a.1.name.cmp(&b.1.name));
        for (orig_pos, (_, sorted_idx)) in positions.iter().zip(to_sort.iter()) {
          if let Some(idx) = t.indices.get_mut(*orig_pos) {
            *idx = sorted_idx.clone();
          }
        }
      }
      Symmetry::AntiSymmetric(_) => {
        // 반대칭은 정렬 시 부호 추적 필요 (v2.1)
      }
    }
  }
  t
}

/// 수축 인덱스 추출 (곱에서)
///
/// 2번 나타나는 인덱스들이 수축 인덱스
pub fn find_contracted_indices(exprs: &[SymExpr]) -> HashSet<String> {
  let mut all: Vec<&str> = vec![];
  for e in exprs {
    if let SymKind::Tensor(t) = &e.kind {
      for idx in &t.indices {
        all.push(&idx.name);
      }
    }
  }

  // 2번 나타나는 것들이 수축 인덱스
  let mut count: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
  for name in &all {
    *count.entry(name).or_insert(0) += 1;
  }

  count
    .into_iter()
    .filter(|(_, c)| *c == 2)
    .map(|(n, _)| n.to_string())
    .collect()
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;
  use crate::symbolic::expr::{IndexPosition, TensorIndex};

  fn symmetric_metric(i1: &str, i2: &str) -> TensorSymbol {
    TensorSymbol {
      name: "g".to_string(),
      indices: vec![
        TensorIndex::with_space(i1, IndexPosition::Lower, "spacetime"),
        TensorIndex::with_space(i2, IndexPosition::Lower, "spacetime"),
      ],
      symmetries: vec![Symmetry::Symmetric(vec![0, 1])],
    }
  }

  #[test]
  fn test_symmetric_sort() {
    // g_{νμ} with symmetry → g_{μν}
    let g = symmetric_metric("ν", "μ");
    let sorted = sort_symmetric_indices(g);

    assert_eq!(sorted.indices[0].name, "μ");
    assert_eq!(sorted.indices[1].name, "ν");
  }

  #[test]
  fn test_contracted_indices() {
    // A^{μ}_{ν} B^{ν}_{ρ} → contracted: ν
    let a = SymExpr::tensor(TensorSymbol {
      name: "A".to_string(),
      indices: vec![
        TensorIndex::with_space("μ", IndexPosition::Upper, "spacetime"),
        TensorIndex::with_space("ν", IndexPosition::Lower, "spacetime"),
      ],
      symmetries: vec![],
    });
    let b = SymExpr::tensor(TensorSymbol {
      name: "B".to_string(),
      indices: vec![
        TensorIndex::with_space("ν", IndexPosition::Upper, "spacetime"),
        TensorIndex::with_space("ρ", IndexPosition::Lower, "spacetime"),
      ],
      symmetries: vec![],
    });

    let contracted = find_contracted_indices(&[a, b]);
    assert!(contracted.contains("ν"));
    assert!(!contracted.contains("μ"));
    assert!(!contracted.contains("ρ"));
  }

  #[test]
  fn test_normalize_mul() {
    let g = SymExpr::tensor(symmetric_metric("ν", "μ"));
    let mul = SymExpr::mul(vec![g]);
    let normalized = tensor_normalize(mul);

    if let SymKind::Mul(xs) = &normalized.kind {
      if let SymKind::Tensor(t) = &xs[0].kind {
        assert_eq!(t.indices[0].name, "μ");
        assert_eq!(t.indices[1].name, "ν");
      } else {
        panic!("Expected Tensor");
      }
    } else {
      panic!("Expected Mul");
    }
  }
}
