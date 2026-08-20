//! CT Optimizer - Category Theory 기반 최적화
//!
//! pnix-old의 ct_opt.rs를 pnix-new(그래프 기반)에 맞게 적응.
//!
//! ## 최적화 패스
//!
//! - `IdentityElimination`: 항등 morphism 제거
//! - `PureZoneFusion`: Pure zone 내 연속 morphism 융합
//! - `ZoneBarrier`: Foreign effect zone 경계 barrier
//! - `FunctorFusion`: Functor law (fmap f ∘ fmap g = fmap (f ∘ g))
//! - `MonadSimplify`: Monad law simplification hints
//! - `FrpLiftFusion`: FRP lift fusion (ct.md Section 23)
//!
//! ## 헌법 준수 (P0-1)
//!
//! 이 최적화기는 구조만 변환하며, 값을 계산하지 않는다.

use super::ctast::{CTMorphismOp, CTNode, CTAST};
use crate::core::FxCoreModule;
use crate::effects::EffectZone;
use std::collections::{HashMap, HashSet};

// ============================================================
// Optimization Passes
// ============================================================

/// CT Optimization Pass
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CTOptPass {
  /// id: A → A 형태의 morphism 제거
  IdentityElimination,
  /// Pure zone 내 연속 morphism 융합
  PureZoneFusion,
  /// Foreign effect zone 경계에서 융합 금지
  ZoneBarrier,
  /// Functor law: fmap f ∘ fmap g = fmap (f ∘ g)
  FunctorFusion,
  /// Monad law simplification
  MonadSimplify,
  /// FRP lift fusion: lift(f) ∘ lift(g) = lift(f ∘ g)
  FrpLiftFusion,
  /// Dead node elimination
  DeadNodeElimination,
}

/// CT Optimization Result
#[derive(Debug, Clone)]
pub struct CTOptResult {
  /// Optimized CTAST
  pub ctast: CTAST,
  /// Applied optimizations
  pub applied: Vec<String>,
  /// Optimization hints (not applied but detected)
  pub hints: Vec<String>,
}

// ============================================================
// CT Optimizer
// ============================================================

/// CT Optimizer
pub struct CTOptimizer {
  passes: Vec<CTOptPass>,
}

impl CTOptimizer {
  /// Create with default passes
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self {
      passes: vec![
        CTOptPass::IdentityElimination,
        CTOptPass::ZoneBarrier,
        CTOptPass::PureZoneFusion,
        CTOptPass::FunctorFusion,
        CTOptPass::MonadSimplify,
        CTOptPass::FrpLiftFusion,
        CTOptPass::DeadNodeElimination,
      ],
    }
  }

  /// Create with specific passes
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn with_passes(passes: Vec<CTOptPass>) -> Self {
    Self { passes }
  }

  /// All passes
  pub fn all_passes() -> Self {
    Self::new()
  }

  /// Optimize CTAST
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn optimize(&self, ctast: CTAST) -> CTOptResult {
    let mut result = ctast;
    let mut applied = Vec::new();
    let mut hints = Vec::new();

    for pass in &self.passes {
      let (new_ctast, changed, pass_hints) = match pass {
        CTOptPass::IdentityElimination => identity_elimination(result),
        CTOptPass::PureZoneFusion => pure_zone_fusion(result),
        CTOptPass::ZoneBarrier => zone_barrier(result),
        CTOptPass::FunctorFusion => functor_fusion(result),
        CTOptPass::MonadSimplify => monad_simplify(result),
        CTOptPass::FrpLiftFusion => frp_lift_fusion(result),
        CTOptPass::DeadNodeElimination => dead_node_elimination(result),
      };

      if changed {
        applied.push(format!("{:?}", pass));
      }
      hints.extend(pass_hints);
      result = new_ctast;
    }

    CTOptResult {
      ctast: result,
      applied,
      hints,
    }
  }

  /// Optimize FxCoreModule (convenience method)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn optimize_fxcore(&self, module: &FxCoreModule) -> (CTAST, CTOptResult) {
    let ctast = CTAST::from_fxcore(module);
    let result = self.optimize(ctast.clone());
    (ctast, result)
  }
}

impl Default for CTOptimizer {
  fn default() -> Self {
    Self::new()
  }
}

// ============================================================
// Individual Optimization Passes
// ============================================================

/// Identity elimination: remove id morphisms
fn identity_elimination(mut ctast: CTAST) -> (CTAST, bool, Vec<String>) {
  let hints = Vec::new();

  // Find identity nodes
  let identity_nodes: HashSet<String> = ctast
    .nodes
    .iter()
    .filter(|n| n.is_identity())
    .map(|n| n.name.clone())
    .collect();

  if identity_nodes.is_empty() {
    return (ctast, false, hints);
  }

  // Redirect edges that go through identity nodes
  let mut new_edges: Vec<(String, String)> = Vec::new();
  for (from, to) in &ctast.edges {
    if identity_nodes.contains(from) {
      // Find what feeds into the identity node
      for (src, dst) in &ctast.edges {
        if dst == from {
          new_edges.push((src.clone(), to.clone()));
        }
      }
    } else if identity_nodes.contains(to) {
      // Find what the identity node feeds into
      for (src, dst) in &ctast.edges {
        if src == to {
          new_edges.push((from.clone(), dst.clone()));
        }
      }
    } else {
      new_edges.push((from.clone(), to.clone()));
    }
  }

  // Remove identity nodes
  let original_len = ctast.nodes.len();
  ctast.nodes.retain(|n| !identity_nodes.contains(&n.name));
  ctast.edges = new_edges;

  let changed = ctast.nodes.len() < original_len;
  (ctast, changed, hints)
}

/// Pure zone fusion: fuse consecutive pure morphisms
fn pure_zone_fusion(mut ctast: CTAST) -> (CTAST, bool, Vec<String>) {
  let hints = Vec::new();

  // Find consecutive pure nodes
  let pure_nodes: HashSet<&str> = ctast
    .nodes
    .iter()
    .filter(|n| n.zone == EffectZone::Pure)
    .map(|n| n.name.as_str())
    .collect();

  // Find fusible pairs: A --f--> B --g--> C where both are pure
  let mut to_fuse: Option<(String, String)> = None;

  for (from, to) in &ctast.edges {
    if pure_nodes.contains(from.as_str()) && pure_nodes.contains(to.as_str()) {
      // Check if 'to' has only one input (can be fused)
      let inputs_to_node: Vec<_> = ctast.edges.iter().filter(|(_, t)| t == to).collect();
      if inputs_to_node.len() == 1 {
        to_fuse = Some((from.clone(), to.clone()));
        break;
      }
    }
  }

  let changed = to_fuse.is_some();

  if let Some((from_name, to_name)) = to_fuse {
    // Get node info before modifying
    let from_node = ctast.nodes.iter().find(|n| n.name == from_name).cloned();
    let to_node = ctast.nodes.iter().find(|n| n.name == to_name).cloned();

    if let (Some(f_node), Some(g_node)) = (from_node, to_node) {
      // Create fused node
      let fused_name = format!("{}∘{}", g_node.name, f_node.name);
      let fused = CTNode::new(
        &fused_name,
        g_node.op, // Keep the later operation
        f_node.src.clone(),
        g_node.tgt.clone(),
        f_node.inputs.clone(),
      );

      // Update edges: redirect edges to/from the fused nodes
      let mut new_edges: Vec<(String, String)> = Vec::new();
      for (src, dst) in &ctast.edges {
        if dst == &from_name {
          new_edges.push((src.clone(), fused_name.clone()));
        } else if src == &to_name {
          new_edges.push((fused_name.clone(), dst.clone()));
        } else if !(src == &from_name && dst == &to_name) {
          new_edges.push((src.clone(), dst.clone()));
        }
      }

      // Remove old nodes and add fused
      ctast
        .nodes
        .retain(|n| n.name != from_name && n.name != to_name);
      ctast.nodes.push(fused);
      ctast.edges = new_edges;
    }
  }

  (ctast, changed, hints)
}

/// Zone barrier: mark foreign effect zones
fn zone_barrier(ctast: CTAST) -> (CTAST, bool, Vec<String>) {
  let mut hints = Vec::new();

  // Detect zone transitions
  for (from, to) in &ctast.edges {
    if let (Some(f_node), Some(t_node)) = (ctast.get_node(from), ctast.get_node(to)) {
      if f_node.zone != t_node.zone {
        hints.push(format!(
          "Zone barrier: {} ({:?}) -> {} ({:?})",
          from, f_node.zone, to, t_node.zone
        ));
      }
    }
  }

  let changed = !hints.is_empty();
  (ctast, changed, hints)
}

/// Functor fusion: fmap f . fmap g = fmap (f . g)
fn functor_fusion(mut ctast: CTAST) -> (CTAST, bool, Vec<String>) {
  let hints = Vec::new();

  // Find consecutive fmap nodes
  let fmap_nodes: HashSet<&str> = ctast
    .nodes
    .iter()
    .filter(|n| n.op.is_fmap())
    .map(|n| n.name.as_str())
    .collect();

  let mut to_fuse: Option<(String, String)> = None;

  for (from, to) in &ctast.edges {
    if fmap_nodes.contains(from.as_str()) && fmap_nodes.contains(to.as_str()) {
      to_fuse = Some((from.clone(), to.clone()));
      break;
    }
  }

  let changed = to_fuse.is_some();

  if let Some((from_name, to_name)) = to_fuse {
    let from_node = ctast.nodes.iter().find(|n| n.name == from_name).cloned();
    let to_node = ctast.nodes.iter().find(|n| n.name == to_name).cloned();

    if let (Some(f_node), Some(g_node)) = (from_node, to_node) {
      let fused_name = format!("fmap({}∘{})", g_node.name, f_node.name);
      let fused = CTNode::new(
        &fused_name,
        CTMorphismOp::Fmap,
        f_node.src.clone(),
        g_node.tgt.clone(),
        f_node.inputs.clone(),
      );

      let mut new_edges: Vec<(String, String)> = Vec::new();
      for (src, dst) in &ctast.edges {
        if dst == &from_name {
          new_edges.push((src.clone(), fused_name.clone()));
        } else if src == &to_name {
          new_edges.push((fused_name.clone(), dst.clone()));
        } else if !(src == &from_name && dst == &to_name) {
          new_edges.push((src.clone(), dst.clone()));
        }
      }

      ctast
        .nodes
        .retain(|n| n.name != from_name && n.name != to_name);
      ctast.nodes.push(fused);
      ctast.edges = new_edges;
    }
  }

  (ctast, changed, hints)
}

/// Monad simplification: detect monad law patterns
fn monad_simplify(ctast: CTAST) -> (CTAST, bool, Vec<String>) {
  let mut hints = Vec::new();

  // Detect return x >>= f pattern (left identity)
  for node in &ctast.nodes {
    if node.op == CTMorphismOp::Bind {
      for input in &node.inputs {
        if let Some(input_node) = ctast.get_node(input) {
          if input_node.op == CTMorphismOp::Return {
            hints.push(format!(
              "Monad left identity: return x >>= f at '{}' can be simplified to f x",
              node.name
            ));
          }
        }
      }
    }
  }

  // Detect m >>= return pattern (right identity)
  for node in &ctast.nodes {
    if node.op == CTMorphismOp::Bind && node.inputs.len() >= 2 {
      if let Some(k_node) = ctast.get_node(&node.inputs[1]) {
        if k_node.op == CTMorphismOp::Return {
          hints.push(format!(
            "Monad right identity: m >>= return at '{}' can be simplified to m",
            node.name
          ));
        }
      }
    }
  }

  let changed = !hints.is_empty();
  (ctast, changed, hints)
}

/// FRP lift fusion: lift(f) . lift(g) = lift(f . g)
fn frp_lift_fusion(mut ctast: CTAST) -> (CTAST, bool, Vec<String>) {
  let hints = Vec::new();

  // Find consecutive lift nodes in FRP zone
  let lift_nodes: HashSet<&str> = ctast
    .nodes
    .iter()
    .filter(|n| n.op == CTMorphismOp::Lift && n.zone == EffectZone::Frp)
    .map(|n| n.name.as_str())
    .collect();

  let mut to_fuse: Option<(String, String)> = None;

  for (from, to) in &ctast.edges {
    if lift_nodes.contains(from.as_str()) && lift_nodes.contains(to.as_str()) {
      to_fuse = Some((from.clone(), to.clone()));
      break;
    }
  }

  let changed = to_fuse.is_some();

  if let Some((from_name, to_name)) = to_fuse {
    let from_node = ctast.nodes.iter().find(|n| n.name == from_name).cloned();
    let to_node = ctast.nodes.iter().find(|n| n.name == to_name).cloned();

    if let (Some(f_node), Some(g_node)) = (from_node, to_node) {
      let fused_name = format!("lift({}∘{})", g_node.name, f_node.name);
      let fused = CTNode::new(
        &fused_name,
        CTMorphismOp::Lift,
        f_node.src.clone(),
        g_node.tgt.clone(),
        f_node.inputs.clone(),
      );

      let mut new_edges: Vec<(String, String)> = Vec::new();
      for (src, dst) in &ctast.edges {
        if dst == &from_name {
          new_edges.push((src.clone(), fused_name.clone()));
        } else if src == &to_name {
          new_edges.push((fused_name.clone(), dst.clone()));
        } else if !(src == &from_name && dst == &to_name) {
          new_edges.push((src.clone(), dst.clone()));
        }
      }

      ctast
        .nodes
        .retain(|n| n.name != from_name && n.name != to_name);
      ctast.nodes.push(fused);
      ctast.edges = new_edges;
    }
  }

  (ctast, changed, hints)
}

/// Dead node elimination
fn dead_node_elimination(mut ctast: CTAST) -> (CTAST, bool, Vec<String>) {
  let hints = Vec::new();

  // Find all nodes that are targets of edges (have outputs used)
  let mut used_nodes: HashSet<String> = HashSet::new();

  // All edge sources are used
  for (from, _) in &ctast.edges {
    used_nodes.insert(from.clone());
  }

  // All edge targets are used
  for (_, to) in &ctast.edges {
    used_nodes.insert(to.clone());
  }

  // Find dead nodes (not used in any edge)
  let original_len = ctast.nodes.len();
  let keep_all = original_len <= 1;
  ctast
    .nodes
    .retain(|n| used_nodes.contains(&n.name) || keep_all);

  let changed = ctast.nodes.len() < original_len;
  (ctast, changed, hints)
}

// ============================================================
// Apply Optimization to FxCoreModule
// ============================================================

/// Apply CT optimizations to FxCoreModule
///
/// FxCoreModule을 CT 최적화로 최적화
///
/// Returns the optimized module with structural changes only.
/// Does NOT compute values (헌법 P0-1 준수).
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn optimize_fxcore_module(module: &FxCoreModule) -> (FxCoreModule, CTOptResult) {
  let optimizer = CTOptimizer::new();
  let (original_ctast, result) = optimizer.optimize_fxcore(module);

  // Build node name mapping from optimization
  let mut name_map: HashMap<String, String> = HashMap::new();

  // Track nodes that were removed or renamed
  let original_names: HashSet<_> = original_ctast
    .nodes
    .iter()
    .map(|n| n.name.clone())
    .collect();
  let optimized_names: HashSet<_> = result.ctast.nodes.iter().map(|n| n.name.clone()).collect();

  // Find renamed nodes (fused nodes have "∘" in name)
  for opt_node in &result.ctast.nodes {
    if opt_node.name.contains('∘') {
      // This is a fused node - find original nodes
      for orig_name in &original_names {
        if opt_node.name.contains(orig_name) {
          name_map.insert(orig_name.clone(), opt_node.name.clone());
        }
      }
    }
  }

  // Build optimized module
  let mut optimized = module.clone();

  // Remove nodes that were eliminated
  let removed: HashSet<_> = original_names
    .difference(&optimized_names)
    .filter(|n| !name_map.contains_key(*n))
    .cloned()
    .collect();

  optimized.nodes.retain(|n| !removed.contains(&n.name));

  // Update edges for removed/renamed nodes
  optimized
    .edges
    .retain(|e| !removed.contains(&e.from) && !removed.contains(&e.to));

  // Rename edges if needed
  for edge in &mut optimized.edges {
    if let Some(new_name) = name_map.get(&edge.from) {
      edge.from = new_name.clone();
    }
    if let Some(new_name) = name_map.get(&edge.to) {
      edge.to = new_name.clone();
    }
  }

  (optimized, result)
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
  use super::*;
  use crate::contracts::effect::Effect;
  use crate::core::{FxEdge, FxMorphism, FxNode};
  use crate::ct::ctast::CTType;

  fn make_test_module() -> FxCoreModule {
    FxCoreModule {
      meta: Default::default(),
      name: "test".to_string(),
      types: vec!["Real".to_string()],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![],
      morphisms: vec![
        FxMorphism::simple("sin".into(), "Real".into(), "Real".into(), Effect::Pure),
        FxMorphism::simple("cos".into(), "Real".into(), "Real".into(), Effect::Pure),
        FxMorphism::simple("id".into(), "Real".into(), "Real".into(), Effect::Pure),
      ],
      nodes: vec![
        FxNode {
          name: "n1".into(),
          uses: "sin".into(),
          meta: None,
          ..Default::default()
        },
        FxNode {
          name: "n2".into(),
          uses: "cos".into(),
          meta: None,
          ..Default::default()
        },
      ],
      edges: vec![FxEdge::simple("n1".into(), "n2".into())],
      scopes: vec![],
    }
  }

  #[test]
  fn test_ct_optimizer_new() {
    let opt = CTOptimizer::new();
    assert!(!opt.passes.is_empty());
  }

  #[test]
  fn test_ct_optimizer_optimize() {
    let module = make_test_module();
    let ctast = CTAST::from_fxcore(&module);

    let opt = CTOptimizer::new();
    let result = opt.optimize(ctast);

    // Optimizer should run without panicking
    // The specific optimizations applied depend on the graph structure
    assert!(result.ctast.nodes.len() <= 2);
  }

  #[test]
  fn test_identity_elimination() {
    let mut ctast = CTAST::new();
    ctast.nodes.push(CTNode::new(
      "id_node",
      CTMorphismOp::Id,
      CTType::Real,
      CTType::Real,
      vec![],
    ));
    ctast.nodes.push(CTNode::new(
      "sin_node",
      CTMorphismOp::Sin,
      CTType::Real,
      CTType::Real,
      vec![],
    ));
    ctast
      .edges
      .push(("id_node".to_string(), "sin_node".to_string()));

    let (result, changed, _) = identity_elimination(ctast);

    assert!(changed);
    assert_eq!(result.nodes.len(), 1);
    assert_eq!(result.nodes[0].name, "sin_node");
  }

  #[test]
  fn test_pure_zone_fusion() {
    let mut ctast = CTAST::new();
    ctast.nodes.push(CTNode::new(
      "sin_node",
      CTMorphismOp::Sin,
      CTType::Real,
      CTType::Real,
      vec![],
    ));
    ctast.nodes.push(CTNode::new(
      "cos_node",
      CTMorphismOp::Cos,
      CTType::Real,
      CTType::Real,
      vec!["sin_node".to_string()],
    ));
    ctast
      .edges
      .push(("sin_node".to_string(), "cos_node".to_string()));

    let (result, changed, _) = pure_zone_fusion(ctast);

    assert!(changed);
    assert_eq!(result.nodes.len(), 1);
    assert!(result.nodes[0].name.contains('∘'));
  }

  #[test]
  fn test_zone_barrier() {
    let mut ctast = CTAST::new();
    ctast.nodes.push(CTNode {
      name: "pure_node".to_string(),
      op: CTMorphismOp::Sin,
      src: CTType::Real,
      tgt: CTType::Real,
      zone: EffectZone::Pure,
      inputs: vec![],
    });
    ctast.nodes.push(CTNode {
      name: "frp_node".to_string(),
      op: CTMorphismOp::Time,
      src: CTType::Unit,
      tgt: CTType::signal(CTType::Real),
      zone: EffectZone::Frp,
      inputs: vec!["pure_node".to_string()],
    });
    ctast
      .edges
      .push(("pure_node".to_string(), "frp_node".to_string()));

    let (_, changed, hints) = zone_barrier(ctast);

    assert!(changed);
    assert!(!hints.is_empty());
    assert!(hints[0].contains("Zone barrier"));
  }

  #[test]
  fn test_functor_fusion() {
    let mut ctast = CTAST::new();
    ctast.nodes.push(CTNode::new(
      "fmap1",
      CTMorphismOp::Fmap,
      CTType::list(CTType::Real),
      CTType::list(CTType::Real),
      vec![],
    ));
    ctast.nodes.push(CTNode::new(
      "fmap2",
      CTMorphismOp::Fmap,
      CTType::list(CTType::Real),
      CTType::list(CTType::Real),
      vec!["fmap1".to_string()],
    ));
    ctast.edges.push(("fmap1".to_string(), "fmap2".to_string()));

    let (result, changed, _) = functor_fusion(ctast);

    assert!(changed);
    assert_eq!(result.nodes.len(), 1);
    assert!(result.nodes[0].name.contains("fmap"));
  }

  #[test]
  fn test_monad_simplify() {
    let mut ctast = CTAST::new();
    ctast.nodes.push(CTNode::new(
      "return_node",
      CTMorphismOp::Return,
      CTType::Real,
      CTType::list(CTType::Real),
      vec![],
    ));
    ctast.nodes.push(CTNode::new(
      "bind_node",
      CTMorphismOp::Bind,
      CTType::list(CTType::Real),
      CTType::list(CTType::Real),
      vec!["return_node".to_string()],
    ));
    ctast
      .edges
      .push(("return_node".to_string(), "bind_node".to_string()));

    let (_, changed, hints) = monad_simplify(ctast);

    assert!(changed);
    assert!(!hints.is_empty());
    assert!(hints[0].contains("left identity"));
  }

  #[test]
  fn test_optimize_fxcore_module() {
    let module = make_test_module();
    let (optimized, result) = optimize_fxcore_module(&module);

    // Should preserve module name
    assert_eq!(optimized.name, module.name);

    // Should have some optimizations applied
    assert!(!result.applied.is_empty());
  }

  #[test]
  fn test_ct_opt_pass_all() {
    let opt = CTOptimizer::all_passes();
    assert_eq!(opt.passes.len(), 7);
  }

  #[test]
  fn test_ct_opt_with_specific_passes() {
    let opt = CTOptimizer::with_passes(vec![CTOptPass::IdentityElimination]);
    assert_eq!(opt.passes.len(), 1);
  }
}
