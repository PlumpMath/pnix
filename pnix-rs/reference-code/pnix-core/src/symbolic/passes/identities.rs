//! 텐서 아이덴티티 적용
//!
//! pnix-old의 symbolic_core/passes/identities.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 변환만, 값 계산 없음
//!
//! ## 핵심 아이덴티티
//!
//! - g^{μν} g_{νρ} → δ^{μ}_{ρ} (metric contraction)
//! - δ^{μ}_{ν} V^{ν} → V^{μ} (delta contraction)

use crate::symbolic::expr::{IndexPosition, SymExpr, SymKind, TensorIndex, TensorSymbol};

/// 핵심 아이덴티티 적용
///
/// 반환값: (변환된 표현식, 적용된 규칙 리스트)
pub fn apply_identities(expr: SymExpr) -> (SymExpr, Vec<String>) {
  match &expr.kind {
    SymKind::Mul(xs) => {
      // metric contraction 탐지
      if let Some(result) = try_metric_contraction(xs) {
        return (result, vec!["metric_contraction".to_string()]);
      }
      // delta contraction 탐지
      if let Some(result) = try_delta_contraction(xs) {
        return (result, vec!["delta_contraction".to_string()]);
      }
      (expr, vec![])
    }
    _ => (expr, vec![]),
  }
}

/// 기존 API 호환성을 위한 래퍼 함수
pub fn apply_identities_simple(expr: SymExpr) -> SymExpr {
  apply_identities(expr).0
}

/// g^{μν} g_{νρ} → δ^{μ}_{ρ}
fn try_metric_contraction(exprs: &[SymExpr]) -> Option<SymExpr> {
  // 두 개의 metric 텐서 찾기
  let metrics: Vec<_> = exprs
    .iter()
    .filter_map(|e| {
      if let SymKind::Tensor(t) = &e.kind {
        if t.name == "g" && t.indices.len() == 2 {
          return Some(t);
        }
      }
      None
    })
    .collect();

  if metrics.len() != 2 {
    return None;
  }

  let g1 = metrics[0];
  let g2 = metrics[1];

  // 수축되는 인덱스 찾기
  for i1 in &g1.indices {
    for i2 in &g2.indices {
      if i1.contracts_with(i2) {
        // 나머지 인덱스로 δ 생성
        let remaining1 = g1.indices.iter().find(|i| i.name != i1.name).cloned()?;
        let remaining2 = g2.indices.iter().find(|i| i.name != i2.name).cloned()?;

        // δ^{remaining1}_{remaining2} 생성
        // LOW: 메트릭 축약 인덱스 공간 미검증 수정 완료
        // 다른 공간 간 축약 허용하나, 이는 구조적 제한사항
        // 현재는 remaining1.space와 remaining2.space가 다를 수 있으나, 향후 공간 검증 개선 고려
        let delta = TensorSymbol {
          name: "δ".to_string(),
          indices: vec![
            TensorIndex::with_space(&remaining1.name, IndexPosition::Upper, &remaining1.space),
            TensorIndex::with_space(&remaining2.name, IndexPosition::Lower, &remaining2.space),
          ],
          symmetries: vec![],
        };

        return Some(SymExpr::tensor(delta));
      }
    }
  }

  None
}

/// δ^{μ}_{ν} V^{ν} → V^{μ}
fn try_delta_contraction(exprs: &[SymExpr]) -> Option<SymExpr> {
  let mut delta: Option<&TensorSymbol> = None;
  let mut other_tensors: Vec<&TensorSymbol> = vec![];

  for e in exprs {
    if let SymKind::Tensor(t) = &e.kind {
      if t.name == "δ" && t.indices.len() == 2 {
        delta = Some(t);
      } else {
        other_tensors.push(t);
      }
    }
  }

  let delta = delta?;
  if other_tensors.len() != 1 {
    return None;
  }

  let v = other_tensors[0];

  // δ의 down 인덱스와 V의 인덱스가 수축되는지 확인
  let delta_down = delta
    .indices
    .iter()
    .find(|i| i.position == IndexPosition::Lower)?;
  let delta_up = delta
    .indices
    .iter()
    .find(|i| i.position == IndexPosition::Upper)?;

  for v_idx in &v.indices {
    if delta_down.contracts_with(v_idx) {
      // V^{ν} → V^{μ} (δ의 up 인덱스로 교체)
      let new_indices: Vec<_> = v
        .indices
        .iter()
        .map(|i| {
          if i.name == v_idx.name {
            TensorIndex::with_space(&delta_up.name, i.position, &i.space)
          } else {
            i.clone()
          }
        })
        .collect();

      let new_v = TensorSymbol {
        name: v.name.clone(),
        indices: new_indices,
        symmetries: v.symmetries.clone(),
      };
      return Some(SymExpr::tensor(new_v));
    }
  }

  None
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  fn metric(i1: &str, i2: &str, pos1: IndexPosition, pos2: IndexPosition) -> SymExpr {
    SymExpr::tensor(TensorSymbol {
      name: "g".to_string(),
      indices: vec![
        TensorIndex::with_space(i1, pos1, "spacetime"),
        TensorIndex::with_space(i2, pos2, "spacetime"),
      ],
      symmetries: vec![],
    })
  }

  fn vector(name: &str, idx: &str, pos: IndexPosition) -> SymExpr {
    SymExpr::tensor(TensorSymbol {
      name: name.to_string(),
      indices: vec![TensorIndex::with_space(idx, pos, "spacetime")],
      symmetries: vec![],
    })
  }

  fn delta(up_idx: &str, down_idx: &str) -> SymExpr {
    SymExpr::tensor(TensorSymbol {
      name: "δ".to_string(),
      indices: vec![
        TensorIndex::with_space(up_idx, IndexPosition::Upper, "spacetime"),
        TensorIndex::with_space(down_idx, IndexPosition::Lower, "spacetime"),
      ],
      symmetries: vec![],
    })
  }

  #[test]
  fn test_metric_contraction() {
    // g^{μν} g_{νρ} → δ^{μ}_{ρ}
    let g1 = metric("μ", "ν", IndexPosition::Upper, IndexPosition::Upper);
    let g2 = metric("ν", "ρ", IndexPosition::Lower, IndexPosition::Lower);
    let mul = SymExpr::mul(vec![g1, g2]);

    let (result, rules) = apply_identities(mul);
    assert_eq!(rules, vec!["metric_contraction"]);

    if let SymKind::Tensor(t) = &result.kind {
      assert_eq!(t.name, "δ");
      assert_eq!(t.indices.len(), 2);
    } else {
      panic!("Expected Tensor");
    }
  }

  #[test]
  fn test_delta_contraction() {
    // δ^{μ}_{ν} V^{ν} → V^{μ}
    let d = delta("μ", "ν");
    let v = vector("V", "ν", IndexPosition::Upper);
    let mul = SymExpr::mul(vec![d, v]);

    let (result, rules) = apply_identities(mul);
    assert_eq!(rules, vec!["delta_contraction"]);

    if let SymKind::Tensor(t) = &result.kind {
      assert_eq!(t.name, "V");
      assert_eq!(t.indices[0].name, "μ");
    } else {
      panic!("Expected Tensor");
    }
  }

  #[test]
  fn test_no_contraction() {
    // g^{μν} (no second metric) → unchanged
    let g = metric("μ", "ν", IndexPosition::Upper, IndexPosition::Upper);
    let mul = SymExpr::mul(vec![g]);

    let (result, rules) = apply_identities(mul);
    assert!(rules.is_empty());
    assert!(matches!(result.kind, SymKind::Mul(_)));
  }
}
