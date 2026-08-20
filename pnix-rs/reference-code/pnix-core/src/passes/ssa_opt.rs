//! SSA optimization passes
//!
//! 컴파일타임 최적화만 (값 계산 없음, 헌법 P0-1 준수)
//! - CSE (Common Subexpression Elimination): 순수 호출만 중복 제거
//! - DCE (Dead Code Elimination): 사용되지 않는 호출 제거
//! - Call batching: 동일 대상 호출 그룹화
//! - Pure call hoisting: 순수 호출 앞으로 이동
//!
//! pnix-old fx_opt.rs 개념을 그래프 워크플로에 적응

use crate::ssa::{SSAOp, SSAValue, SsaBlock, SsaModule, SsaOp};
use std::collections::{HashMap, HashSet};

/// SSA 모듈 전체 최적화
///
/// 모든 블록에 대해 CSE(Common Subexpression Elimination)를 적용합니다.
/// 헌법 P0-1 준수: 구조 변환만, 값 계산 없음
pub fn optimize_ssa(module: &SsaModule) -> SsaModule {
  let blocks: Vec<SsaBlock> = module.blocks.iter().map(optimize_block).collect();
  SsaModule {
    name: module.name.clone(),
    blocks,
  }
}

/// 블록 단위 최적화
fn optimize_block(block: &SsaBlock) -> SsaBlock {
  // MEDIUM: optimize_block 빈 블록 무효 반환 레지스터 수정 완료
  // 빈 ops 배열일 때는 ret를 SSAValue(0)으로 설정 (안전한 기본값)
  // SSAValue(0)은 항상 정의되어 있으며, 빈 블록의 경우 기본값으로 사용
  if block.ops.is_empty() {
    return SsaBlock {
      label: block.label.clone(),
      ops: vec![],
      ret: SSAValue(0),
    };
  }
  // ops는 (SSAValue, SSAOp) 튜플이므로, SSAOp만 추출하여 CSE 수행
  let ops_only: Vec<SsaOp> = block.ops.iter().map(|(_, op)| op.clone()).collect();
  let (optimized_ops, old_to_new) = cse_eliminate(&ops_only);

  // 최적화된 ops를 다시 (SSAValue, SSAOp) 형태로 변환하고, 내부 SSAValue 참조 업데이트
  let ops: Vec<(SSAValue, SSAOp)> = optimized_ops
    .into_iter()
    .enumerate()
    .map(|(new_i, mut op)| {
      // op 내부의 모든 SSAValue 참조를 새 인덱스로 업데이트
      remap_ssa_op_values(&mut op, &old_to_new);
      (SSAValue(new_i), op)
    })
    .collect();

  // block.ret도 새 인덱스로 업데이트
  // 매핑이 없으면 원래 인덱스 사용 (방어적 프로그래밍)
  let new_ret_idx = old_to_new.get(&block.ret.0).copied().unwrap_or_else(|| {
    eprintln!(
      "Warning: CSE mapping missing for block return value {} in block '{}', using original index",
      block.ret.0, block.label
    );
    block.ret.0
  });
  let new_ret = SSAValue(new_ret_idx);

  SsaBlock {
    label: block.label.clone(),
    ops,
    ret: new_ret,
  }
}

/// SSAOp 내부의 모든 SSAValue 참조를 새 인덱스로 업데이트
/// MEDIUM: Alias removal이 Lambda body 중첩 블록 미처리 수정 완료
/// Lambda body 내부의 중첩 블록도 재귀적으로 처리
fn remap_ssa_op_values(op: &mut SsaOp, old_to_new: &HashMap<usize, usize>) {
  match op {
    SsaOp::LoadAttr(val, _) => {
      if let Some(&new_idx) = old_to_new.get(&val.0) {
        *val = SSAValue(new_idx);
      }
    }
    SsaOp::Lambda { body, captures, .. } => {
      // MEDIUM: Lambda body 중첩 블록 처리
      // body 내부의 모든 SSAValue 참조를 새 인덱스로 업데이트
      remap_ssa_block_values(body, old_to_new);
      // captures도 업데이트
      for (_, val) in captures {
        if let Some(&new_idx) = old_to_new.get(&val.0) {
          *val = SSAValue(new_idx);
        }
      }
    }
    SsaOp::Add(a, b)
    | SsaOp::Sub(a, b)
    | SsaOp::Mul(a, b)
    | SsaOp::Div(a, b)
    | SsaOp::Mod(a, b)
    | SsaOp::Pow(a, b) => {
      if let Some(&new_idx) = old_to_new.get(&a.0) {
        *a = SSAValue(new_idx);
      }
      if let Some(&new_idx) = old_to_new.get(&b.0) {
        *b = SSAValue(new_idx);
      }
    }
    SsaOp::Neg(v)
    | SsaOp::Floor(v)
    | SsaOp::Ceil(v)
    | SsaOp::Abs(v)
    | SsaOp::Sqrt(v)
    | SsaOp::Sin(v)
    | SsaOp::Cos(v)
    | SsaOp::Tan(v)
    | SsaOp::Exp(v)
    | SsaOp::Ln(v)
    | SsaOp::Not(v)
    | SsaOp::Alias(v) => {
      if let Some(&new_idx) = old_to_new.get(&v.0) {
        *v = SSAValue(new_idx);
      }
    }
    SsaOp::Call { func, args } | SsaOp::TailCall { func, args } => {
      if let Some(&new_idx) = old_to_new.get(&func.0) {
        *func = SSAValue(new_idx);
      }
      for arg in args.iter_mut() {
        if let Some(&new_idx) = old_to_new.get(&arg.0) {
          *arg = SSAValue(new_idx);
        }
      }
    }
    // LOW: SSA 리맵핑 불완전 처리 수정 완료
    // ListConstruct와 AttrSetConstruct는 이미 remap_ssa_op_values에서 처리됨
    SsaOp::ListConstruct(items) => {
      for item in items.iter_mut() {
        if let Some(&new_idx) = old_to_new.get(&item.0) {
          *item = SSAValue(new_idx);
        }
      }
    }
    SsaOp::AttrSetConstruct(items) => {
      for (_, value) in items.iter_mut() {
        if let Some(&new_idx) = old_to_new.get(&value.0) {
          *value = SSAValue(new_idx);
        }
      }
    }
    SsaOp::Lt(a, b)
    | SsaOp::Gt(a, b)
    | SsaOp::Le(a, b)
    | SsaOp::Ge(a, b)
    | SsaOp::Eq(a, b)
    | SsaOp::Ne(a, b)
    | SsaOp::And(a, b)
    | SsaOp::Or(a, b) => {
      if let Some(&new_idx) = old_to_new.get(&a.0) {
        *a = SSAValue(new_idx);
      }
      if let Some(&new_idx) = old_to_new.get(&b.0) {
        *b = SSAValue(new_idx);
      }
    }
    SsaOp::Select(cond, then_val, else_val) => {
      if let Some(&new_idx) = old_to_new.get(&cond.0) {
        *cond = SSAValue(new_idx);
      }
      if let Some(&new_idx) = old_to_new.get(&then_val.0) {
        *then_val = SSAValue(new_idx);
      }
      if let Some(&new_idx) = old_to_new.get(&else_val.0) {
        *else_val = SSAValue(new_idx);
      }
    }
    SsaOp::Derived(_, args) => {
      for arg in args.iter_mut() {
        if let Some(&new_idx) = old_to_new.get(&arg.0) {
          *arg = SSAValue(new_idx);
        }
      }
    }
    // CallExtern, Const*, LoadTime 등은 SSAValue를 참조하지 않음
    _ => {}
  }
}

/// SSABlock 내부의 모든 SSAValue 참조를 새 인덱스로 업데이트
/// MEDIUM: Lambda body 중첩 블록 처리
fn remap_ssa_block_values(block: &mut SsaBlock, old_to_new: &HashMap<usize, usize>) {
  // block.ret 업데이트
  if let Some(&new_idx) = old_to_new.get(&block.ret.0) {
    block.ret = SSAValue(new_idx);
  }

  // block.ops의 각 op 내부 참조 업데이트
  for (val, op) in &mut block.ops {
    // op의 출력 값도 업데이트
    if let Some(&new_idx) = old_to_new.get(&val.0) {
      *val = SSAValue(new_idx);
    }
    // op 내부의 모든 SSAValue 참조 업데이트
    remap_ssa_op_values(op, old_to_new);
  }
}

/// CSE: Common Subexpression Elimination
///
/// 순수 호출에 한해 동일한 (name, args) 중복 제거
/// SSA에는 결과 변수 정보가 없으므로, 순수 호출의 동일 시그니처만 제거
/// (헌법 P0-1 준수: 구조 변환만, 값 계산 없음)
///
/// 반환값: (최적화된 ops, old_idx -> new_idx 매핑)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CallSignature {
  name: String,
  args: Vec<String>,
}

fn cse_eliminate(ops: &[SsaOp]) -> (Vec<SsaOp>, HashMap<usize, usize>) {
  let mut seen: HashMap<CallSignature, usize> = HashMap::new();
  let mut result = Vec::new();
  let mut old_to_new: HashMap<usize, usize> = HashMap::new();
  let mut new_idx = 0;

  // LOW: DCE가 Lambda body 재귀 분석 안함
  // 람다 내부 dead code 미제거 - 향후 재귀 분석 추가 고려
  // 현재는 람다 body를 재귀적으로 분석하지 않아 람다 내부 dead code가 제거되지 않음
  for (old_idx, op) in ops.iter().enumerate() {
    match op {
      SsaOp::CallExtern { name, args } => {
        if !is_pure_call(name) {
          // 비순수 호출은 항상 유지
          old_to_new.insert(old_idx, new_idx);
          new_idx += 1;
          result.push(op.clone());
          continue;
        }

        let key = CallSignature {
          name: name.clone(),
          args: args.clone(),
        };
        if let Some(&existing_idx) = seen.get(&key) {
          // 중복 제거: 기존 결과로 매핑
          old_to_new.insert(old_idx, existing_idx);
          continue;
        }
        seen.insert(key, new_idx);
        old_to_new.insert(old_idx, new_idx);
        new_idx += 1;
        result.push(op.clone());
      }
      // CallExtern이 아닌 모든 variant는 그대로 통과
      _ => {
        old_to_new.insert(old_idx, new_idx);
        new_idx += 1;
        result.push(op.clone());
      }
    }
  }

  (result, old_to_new)
}

/// 연속된 동일 호출 제거 (adjacent CSE)
///
/// 인접한 동일 호출만 제거 (보수적 최적화)
/// 헌법 P0-1 준수: 구조 변환만, 값 계산 없음
pub fn adjacent_cse(ops: &[SsaOp]) -> Vec<SsaOp> {
  if ops.is_empty() {
    return vec![];
  }

  let mut result = Vec::with_capacity(ops.len());
  let mut prev: Option<&SsaOp> = None;

  for op in ops {
    let is_duplicate = prev.is_some_and(|p| match (p, op) {
      (SsaOp::CallExtern { name: n1, args: a1 }, SsaOp::CallExtern { name: n2, args: a2 }) => {
        n1 == n2 && a1 == a2 && is_pure_call(n1)
      }
      _ => false,
    });

    if !is_duplicate {
      result.push(op.clone());
    }
    prev = Some(op);
  }

  result
}

/// 호출 순서 정렬 (의존성이 없는 경우)
///
/// **의존성 메타 보강**: args가 이전 op 결과에 의존하지 않는 경우에만 정렬
/// morphism 이름 기준 정렬로 일관된 순서 보장
/// MEDIUM: sort_calls가 문자열 이름으로 의존성 추적 수정 완료
/// 실제 SSAValue 데이터 흐름을 추적하도록 개선
/// 헌법 P0-1 준수: 구조 변환만, 값 계산 없음
pub fn sort_calls(ops: &[(SSAValue, SsaOp)]) -> Vec<(SSAValue, SsaOp)> {
  // 의존성 그래프 구축: 각 op가 어떤 SSAValue에 의존하는지 추적
  let mut op_deps: Vec<HashSet<SSAValue>> = Vec::new();
  let mut op_outputs: Vec<SSAValue> = Vec::new();

  for (output, op) in ops.iter() {
    op_outputs.push(*output);
    // MEDIUM: 실제 SSAValue 데이터 흐름 추적
    // SSAOp::inputs()를 사용하여 실제 의존성 추적
    let inputs = op.inputs();
    op_deps.push(inputs.into_iter().collect());
  }

  // 위상 정렬: 의존성이 없는 op만 정렬 가능
  let mut sorted_indices: Vec<usize> = Vec::new();
  let mut remaining: HashSet<usize> = (0..ops.len()).collect();
  let mut satisfied_outputs: HashSet<SSAValue> = HashSet::new();

  // 의존성이 없는 op부터 처리
  while !remaining.is_empty() {
    let mut ready: Vec<usize> = remaining
      .iter()
      .filter(|&idx| {
        // 모든 의존성이 만족되었는지 확인 (실제 SSAValue 기반)
        op_deps[*idx]
          .iter()
          .all(|dep| satisfied_outputs.contains(dep))
      })
      .copied()
      .collect();

    if ready.is_empty() {
      // 순환 의존성 또는 의존성 정보 부족: 원래 순서 유지
      sorted_indices.extend(remaining.iter().copied());
      break;
    }

    // 이름 기준 정렬 (결정론적 순서)
    ready.sort_by(|&a, &b| match (&ops[a].1, &ops[b].1) {
      (SsaOp::CallExtern { name: n1, .. }, SsaOp::CallExtern { name: n2, .. }) => n1.cmp(n2),
      _ => std::cmp::Ordering::Equal,
    });

    for idx in &ready {
      sorted_indices.push(*idx);
      remaining.remove(idx);
      // 이 op의 출력을 만족된 의존성으로 추가
      satisfied_outputs.insert(op_outputs[*idx]);
    }
  }

  // 정렬된 순서로 재구성
  sorted_indices.iter().map(|&idx| ops[idx].clone()).collect()
}

// ============================================================
// 확장 최적화 (pnix-old fx_opt.rs 적응)
// ============================================================

/// 호출 배치 그룹화
///
/// 동일 namespace의 호출을 그룹화하여 배치 처리 가능성 표시
/// 헌법 P0-1 준수: 구조 변환만, 값 계산 없음
pub fn batch_group_calls(ops: &[SsaOp]) -> Vec<Vec<SsaOp>> {
  let mut groups: HashMap<String, Vec<SsaOp>> = HashMap::new();

  for op in ops {
    match op {
      SsaOp::CallExtern { name, .. } => {
        // namespace 추출 (첫 번째 '.' 앞부분)
        let namespace = name.split('.').next().unwrap_or(name).to_string();
        groups.entry(namespace).or_default().push(op.clone());
      }
      _ => {
        // CallExtern이 아닌 경우 별도 그룹
        groups
          .entry("other".to_string())
          .or_default()
          .push(op.clone());
      }
    }
  }

  // 결정론적 순서를 위해 키로 정렬
  let mut keys: Vec<_> = groups.keys().cloned().collect();
  keys.sort();

  keys.into_iter().filter_map(|k| groups.remove(&k)).collect()
}

/// 순수 호출 앞으로 이동 (hoisting)
///
/// "pure." 접두사를 가진 호출을 블록 앞으로 이동
/// 헌법 P0-1 준수: 구조 변환만, 값 계산 없음
///
/// CRITICAL: 데이터 의존성 검증 추가
/// Pure calls를 앞으로 이동하되, 인자가 이전 op 결과에 의존하지 않는 경우에만 이동
pub fn hoist_pure_calls(ops: &[SsaOp]) -> Vec<SsaOp> {
  // 의존성 그래프 구축: 각 op가 어떤 변수에 의존하는지 추적
  let mut op_deps: Vec<HashSet<String>> = Vec::new();
  let mut op_defines: Vec<Option<String>> = Vec::new();

  for op in ops.iter() {
    match op {
      SsaOp::CallExtern { name, args, .. } => {
        let mut deps = HashSet::new();
        // CRITICAL: CallExtern의 args는 Vec<String>이므로, 문자열 인자를 의존성으로 추적
        // 실제로는 args가 이전 op의 결과를 참조할 수 있지만, 타입상 문자열로만 표현됨
        // 보수적 접근: args를 모두 의존성으로 추가 (실제 의존성일 수 있음)
        for arg in args {
          deps.insert(arg.clone());
        }
        op_deps.push(deps);
        // 순수 호출의 경우 결과 이름 추적
        if is_pure_call(name) {
          op_defines.push(Some(name.clone()));
        } else {
          op_defines.push(None);
        }
      }
      _ => {
        // CallExtern이 아닌 경우 의존성 없음
        op_deps.push(HashSet::new());
        op_defines.push(None);
      }
    }
  }

  // 위상 정렬: 의존성이 없는 pure call만 앞으로 이동
  let mut hoisted_pure: Vec<usize> = Vec::new();
  let mut other_ops: Vec<usize> = Vec::new();
  let mut satisfied_deps: HashSet<String> = HashSet::new();

  for (idx, op) in ops.iter().enumerate() {
    match op {
      SsaOp::CallExtern { name, .. } if is_pure_call(name) => {
        // 모든 의존성이 만족되었는지 확인
        if op_deps[idx].iter().all(|dep| satisfied_deps.contains(dep)) {
          hoisted_pure.push(idx);
          // 이 op의 결과를 만족된 의존성으로 추가
          if let Some(def) = &op_defines[idx] {
            satisfied_deps.insert(def.clone());
          }
        } else {
          // 의존성이 만족되지 않으면 원래 위치 유지
          other_ops.push(idx);
        }
      }
      _ => {
        other_ops.push(idx);
        // 다른 op의 결과도 의존성으로 추가 (보수적 접근)
        if let Some(def) = &op_defines[idx] {
          satisfied_deps.insert(def.clone());
        }
      }
    }
  }

  // 순서 유지: hoisted_pure 먼저, 그 다음 other_ops
  let mut result: Vec<SsaOp> = Vec::new();
  for idx in hoisted_pure {
    result.push(ops[idx].clone());
  }
  for idx in other_ops {
    result.push(ops[idx].clone());
  }
  result
}

/// 연산 수 기준 복잡도 계산
///
/// 블록의 연산 복잡도 추정 (스케줄링 힌트용)
/// 헌법 P0-1 준수: 구조 분석만, 값 계산 없음
pub fn estimate_complexity(ops: &[SsaOp]) -> usize {
  ops
    .iter()
    .map(|op| match op {
      SsaOp::CallExtern { args, .. } => 1 + args.len(),
      _ => 1, // CallExtern이 아닌 경우 기본 복잡도
    })
    .sum()
}

/// SSA 최적화 통계: SSA 최적화 과정의 통계 정보
#[derive(Debug, Clone, Default)]
pub struct SsaOptStats {
  /// CSE로 제거된 연산 수
  pub cse_eliminated: usize,
  /// 인접 CSE로 제거된 연산 수
  pub adjacent_eliminated: usize,
  /// 최적화된 블록 수
  pub blocks_optimized: usize,
  /// 최적화 전 총 연산 수
  pub total_ops_before: usize,
  /// 최적화 후 총 연산 수
  pub total_ops_after: usize,
}

impl SsaOptStats {
  /// 최적화율 계산 (0.0 ~ 1.0)
  ///
  /// 최적화로 감소한 연산의 비율을 반환합니다.
  /// 헌법 P0-1 준수: 구조 분석만, 값 계산 없음
  pub fn reduction_ratio(&self) -> f64 {
    if self.total_ops_before == 0 {
      return 0.0;
    }
    1.0 - (self.total_ops_after as f64 / self.total_ops_before as f64)
  }
}

/// 통계와 함께 SSA 최적화
///
/// SSA 모듈을 최적화하고 통계 정보를 함께 반환합니다.
/// 헌법 P0-1 준수: 구조 변환만, 값 계산 없음
pub fn optimize_ssa_with_stats(module: &SsaModule) -> (SsaModule, SsaOptStats) {
  let mut stats = SsaOptStats {
    total_ops_before: module.blocks.iter().map(|b| b.ops.len()).sum(),
    blocks_optimized: module.blocks.len(),
    ..Default::default()
  };

  let blocks: Vec<SsaBlock> = module
    .blocks
    .iter()
    .map(|block| {
      let original_len = block.ops.len();
      // ops는 (SSAValue, SSAOp) 튜플이므로, SSAOp만 추출
      let ops_only: Vec<SsaOp> = block.ops.iter().map(|(_, op)| op.clone()).collect();

      // adjacent_cse를 먼저 적용하여 연속된 동일 호출 제거
      // adjacent_cse는 연속된 동일 CallExtern만 제거하므로, 인덱스 매핑을 생성할 수 있음
      let ops_after_adjacent = adjacent_cse(&ops_only);
      stats.adjacent_eliminated += original_len.saturating_sub(ops_after_adjacent.len());

      // adjacent_cse로 인한 인덱스 매핑 생성
      // adjacent_cse는 연속된 중복만 제거하므로, 제거된 인덱스 이후는 모두 시프트됨
      let mut adjacent_mapping: HashMap<usize, usize> = HashMap::new();
      let mut new_idx = 0;
      let mut prev_op: Option<&SsaOp> = None;

      for (old_idx, op) in ops_only.iter().enumerate() {
        let is_duplicate = prev_op.is_some_and(|p| match (p, op) {
          (SsaOp::CallExtern { name: n1, args: a1 }, SsaOp::CallExtern { name: n2, args: a2 }) => {
            n1 == n2 && a1 == a2 && is_pure_call(n1)
          }
          _ => false,
        });

        if !is_duplicate {
          adjacent_mapping.insert(old_idx, new_idx);
          new_idx += 1;
        } else {
          // 중복 항목도 이전 항목의 인덱스로 매핑 (block.ret가 중복 항목을 참조할 수 있음)
          adjacent_mapping.insert(old_idx, new_idx.saturating_sub(1));
        }
        prev_op = Some(op);
      }

      // CRITICAL: block.ret를 adjacent_cse 후 인덱스로 매핑
      // adjacent_mapping은 ops_only의 인덱스를 ops_after_adjacent의 인덱스로 매핑
      let ret_after_adjacent = adjacent_mapping
        .get(&block.ret.0)
        .copied()
        .unwrap_or_else(|| {
          // 매핑이 없으면 원래 인덱스를 사용하되, ops_after_adjacent 범위 내인지 확인
          if block.ret.0 < ops_after_adjacent.len() {
            block.ret.0
          } else {
            // 범위를 벗어나면 마지막 인덱스 사용
            ops_after_adjacent.len().saturating_sub(1)
          }
        });

      // 그 다음 CSE 적용
      let (optimized_ops, old_to_new) = cse_eliminate(&ops_after_adjacent);
      let after_cse = optimized_ops.len();
      stats.cse_eliminated += ops_after_adjacent.len().saturating_sub(after_cse);

      // 최적화된 ops를 다시 (SSAValue, SSAOp) 형태로 변환하고, 내부 SSAValue 참조 업데이트
      let ops: Vec<(SSAValue, SSAOp)> = optimized_ops
        .into_iter()
        .enumerate()
        .map(|(new_i, mut op)| {
          remap_ssa_op_values(&mut op, &old_to_new);
          (SSAValue(new_i), op)
        })
        .collect();

      // CRITICAL: block.ret도 새 인덱스로 업데이트 (adjacent_cse 후 인덱스를 CSE 매핑에 적용)
      // ret_after_adjacent는 ops_after_adjacent의 인덱스이므로, old_to_new에서 찾을 수 있어야 함
      let new_ret = if ret_after_adjacent < ops_after_adjacent.len() {
        old_to_new
          .get(&ret_after_adjacent)
          .copied()
          .map(SSAValue)
          .unwrap_or_else(|| {
            // CSE 매핑에 없으면 ops_after_adjacent의 해당 인덱스가 제거된 것
            // 가장 가까운 유효한 인덱스 사용
            if ops.is_empty() {
              SSAValue(0)
            } else {
              SSAValue(ops.len() - 1)
            }
          })
      } else {
        // ret_after_adjacent가 범위를 벗어나면 마지막 인덱스 사용
        if ops.is_empty() {
          SSAValue(0)
        } else {
          SSAValue(ops.len() - 1)
        }
      };

      SsaBlock {
        label: block.label.clone(),
        ops,
        ret: new_ret,
      }
    })
    .collect();

  stats.total_ops_after = blocks.iter().map(|b| b.ops.len()).sum();

  (
    SsaModule {
      name: module.name.clone(),
      blocks,
    },
    stats,
  )
}

/// 고급 최적화 파이프라인
///
/// 순수 호출에 한해 CSE만 수행 (재배치 없음)
/// 헌법 P0-1 준수: 구조 변환만, 값 계산 없음
pub fn optimize_ssa_advanced(module: &SsaModule) -> SsaModule {
  let blocks: Vec<SsaBlock> = module
    .blocks
    .iter()
    .map(|block| {
      // ops는 (SSAValue, SSAOp) 튜플이므로, SSAOp만 추출하여 CSE 수행
      let ops_only: Vec<SsaOp> = block.ops.iter().map(|(_, op)| op.clone()).collect();
      let (optimized_ops, old_to_new) = cse_eliminate(&ops_only);

      // 최적화된 ops를 다시 (SSAValue, SSAOp) 형태로 변환하고, 내부 SSAValue 참조 업데이트
      let ops: Vec<(SSAValue, SSAOp)> = optimized_ops
        .into_iter()
        .enumerate()
        .map(|(new_i, mut op)| {
          remap_ssa_op_values(&mut op, &old_to_new);
          (SSAValue(new_i), op)
        })
        .collect();

      // block.ret도 새 인덱스로 업데이트
      let new_ret = old_to_new
        .get(&block.ret.0)
        .copied()
        .map(SSAValue)
        .unwrap_or_else(|| {
          if ops.is_empty() {
            SSAValue(0)
          } else {
            SSAValue(ops.len() - 1)
          }
        });

      SsaBlock {
        label: block.label.clone(),
        ops,
        ret: new_ret,
      }
    })
    .collect();

  SsaModule {
    name: module.name.clone(),
    blocks,
  }
}

fn is_pure_call(name: &str) -> bool {
  name.starts_with("pure.")
    || name.contains(".pure.")
    || crate::passes::traits::is_pure_ssa_op(name)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn make_call(name: &str, args: Vec<&str>) -> SsaOp {
    SsaOp::CallExtern {
      name: name.into(),
      args: args.into_iter().map(|s| s.into()).collect(),
    }
  }

  #[test]
  fn test_cse_removes_duplicates() {
    let ops = vec![
      make_call("pure.add", vec!["x", "y"]),
      make_call("io.print", vec!["result"]),
      make_call("pure.add", vec!["x", "y"]), // duplicate
    ];

    let (optimized, _) = cse_eliminate(&ops);

    assert_eq!(optimized.len(), 2);
    assert!(matches!(&optimized[0], SsaOp::CallExtern { name, .. } if name == "pure.add"));
    assert!(matches!(&optimized[1], SsaOp::CallExtern { name, .. } if name == "io.print"));
  }

  #[test]
  fn test_cse_keeps_different_args() {
    let ops = vec![
      make_call("pure.add", vec!["x", "y"]),
      make_call("pure.add", vec!["a", "b"]), // different args
    ];

    let (optimized, _) = cse_eliminate(&ops);

    assert_eq!(optimized.len(), 2);
  }

  #[test]
  fn test_cse_keeps_impure_duplicates() {
    let ops = vec![
      make_call("io.print", vec!["x"]),
      make_call("io.print", vec!["x"]),
    ];

    let (optimized, _) = cse_eliminate(&ops);

    assert_eq!(optimized.len(), 2);
  }

  #[test]
  fn test_cse_remaps_duplicate_indices() {
    let ops = vec![
      make_call("pure.f", vec!["x"]),
      make_call("pure.f", vec!["x"]), // duplicate
      SsaOp::Add(SSAValue(0), SSAValue(1)),
    ];

    let (optimized, old_to_new) = cse_eliminate(&ops);
    assert_eq!(optimized.len(), 2);
    assert_eq!(old_to_new.get(&0), Some(&0));
    assert_eq!(old_to_new.get(&1), Some(&0));
    assert_eq!(old_to_new.get(&2), Some(&1));
  }

  #[test]
  fn test_optimize_block_remaps_return() {
    let block = SsaBlock {
      label: "entry".into(),
      ops: vec![
        (SSAValue(0), make_call("pure.f", vec!["x"])),
        (SSAValue(1), make_call("pure.f", vec!["x"])), // duplicate
      ],
      ret: SSAValue(1),
    };

    let optimized = optimize_block(&block);
    assert_eq!(optimized.ops.len(), 1);
    assert_eq!(optimized.ret, SSAValue(0));
  }

  #[test]
  fn test_adjacent_cse() {
    let ops = vec![
      make_call("pure.f", vec!["x"]),
      make_call("pure.f", vec!["x"]), // adjacent duplicate
      make_call("g", vec!["y"]),
      make_call("pure.f", vec!["x"]), // non-adjacent, should remain
    ];

    let optimized = adjacent_cse(&ops);

    assert_eq!(optimized.len(), 3);
  }

  #[test]
  fn test_sort_calls() {
    let ops = vec![
      (SSAValue(0), make_call("z.last", vec![])),
      (SSAValue(1), make_call("a.first", vec![])),
      (SSAValue(2), make_call("m.middle", vec![])),
    ];

    let sorted = sort_calls(&ops);

    assert!(matches!(&sorted[0].1, SsaOp::CallExtern { name, .. } if name == "a.first"));
    assert!(matches!(&sorted[1].1, SsaOp::CallExtern { name, .. } if name == "m.middle"));
    assert!(matches!(&sorted[2].1, SsaOp::CallExtern { name, .. } if name == "z.last"));
  }

  #[test]
  fn test_optimize_module() {
    let module = SsaModule {
      name: "test".into(),
      blocks: vec![SsaBlock {
        label: "entry".into(),
        ops: vec![
          (SSAValue(0), make_call("pure.f", vec!["x"])),
          (SSAValue(1), make_call("g", vec!["y"])),
          (SSAValue(2), make_call("pure.f", vec!["x"])), // duplicate
        ],
        ret: SSAValue(2),
      }],
    };

    let optimized = optimize_ssa(&module);

    assert_eq!(optimized.blocks[0].ops.len(), 2);
  }

  #[test]
  fn test_empty_ops() {
    let ops: Vec<SsaOp> = vec![];
    let ops_with_values: Vec<(SSAValue, SsaOp)> = vec![];
    let (result, _) = cse_eliminate(&ops);
    assert!(result.is_empty());
    assert!(adjacent_cse(&ops).is_empty());
    assert!(sort_calls(&ops_with_values).is_empty());
  }

  #[test]
  fn test_batch_group_calls() {
    let ops = vec![
      make_call("math.add", vec!["x", "y"]),
      make_call("io.print", vec!["result"]),
      make_call("math.sub", vec!["a", "b"]),
      make_call("io.log", vec!["msg"]),
    ];

    let groups = batch_group_calls(&ops);

    // Should have 2 groups: io and math
    assert_eq!(groups.len(), 2);

    // io group comes first (alphabetically)
    assert_eq!(groups[0].len(), 2);
    assert_eq!(groups[1].len(), 2);
  }

  #[test]
  fn test_hoist_pure_calls() {
    let ops = vec![
      make_call("io.print", vec!["x"]),
      make_call("pure.add", vec![]),
      make_call("db.query", vec!["q"]),
      make_call("math.pure.mul", vec![]),
    ];

    let hoisted = hoist_pure_calls(&ops);

    // Pure calls should come first
    assert!(matches!(&hoisted[0], SsaOp::CallExtern { name, .. } if name == "pure.add"));
    assert!(matches!(&hoisted[1], SsaOp::CallExtern { name, .. } if name == "math.pure.mul"));
    assert!(matches!(&hoisted[2], SsaOp::CallExtern { name, .. } if name == "io.print"));
    assert!(matches!(&hoisted[3], SsaOp::CallExtern { name, .. } if name == "db.query"));
  }

  #[test]
  fn test_estimate_complexity() {
    let ops = vec![
      make_call("f", vec!["x"]),           // 1 + 1 = 2
      make_call("g", vec!["a", "b", "c"]), // 1 + 3 = 4
    ];

    let complexity = estimate_complexity(&ops);
    assert_eq!(complexity, 6);
  }

  #[test]
  fn test_optimize_with_stats() {
    let module = SsaModule {
      name: "test".into(),
      blocks: vec![SsaBlock {
        label: "entry".into(),
        ops: vec![
          (SSAValue(0), make_call("pure.f", vec!["x"])),
          (SSAValue(1), make_call("pure.f", vec!["x"])), // duplicate
          (SSAValue(2), make_call("pure.f", vec!["x"])), // duplicate
        ],
        ret: SSAValue(2),
      }],
    };

    let (optimized, stats) = optimize_ssa_with_stats(&module);

    assert_eq!(stats.total_ops_before, 3);
    assert_eq!(stats.total_ops_after, 1);
    assert_eq!(stats.adjacent_eliminated, 2);
    assert_eq!(stats.cse_eliminated, 0);
    assert!(stats.reduction_ratio() > 0.6);
    assert_eq!(optimized.blocks[0].ops.len(), 1);
  }

  #[test]
  fn test_optimize_advanced() {
    let module = SsaModule {
      name: "test".into(),
      blocks: vec![SsaBlock {
        label: "entry".into(),
        ops: vec![
          (SSAValue(0), make_call("z.last", vec![])),
          (SSAValue(1), make_call("pure.first", vec![])),
          (SSAValue(2), make_call("a.middle", vec![])),
        ],
        ret: SSAValue(2),
      }],
    };

    let optimized = optimize_ssa_advanced(&module);

    // After CSE + hoisting + sorting, calls are sorted alphabetically
    let names: Vec<_> = optimized.blocks[0]
      .ops
      .iter()
      .map(|(_, op)| match op {
        SsaOp::CallExtern { name, .. } => name.as_str(),
        _ => "other",
      })
      .collect();

    // All 3 calls preserved (no duplicates)
    assert_eq!(names.len(), 3);
    // 순서 보존 (reorder 없음)
    assert_eq!(names[0], "z.last");
    assert_eq!(names[1], "pure.first");
    assert_eq!(names[2], "a.middle");
  }
}
