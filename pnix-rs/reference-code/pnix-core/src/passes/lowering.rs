//! Lowering passes

use crate::ast::{AstItem, AstModule, EdgeCondAst, EdgeSource};
use crate::contracts::effect::Effect;
use crate::core::{
  CostHint, EdgeCond, ExecutionContract, FxAdtType, FxAdtVariant, FxCoreModule, FxEdge, FxInput,
  FxMorphism, FxNode, FxPort, FxScope, NodeKind, ScopePolicy, SkipPolicy,
};
use crate::diagnostics::Diagnostics;
use crate::spec::builtin::STDLIB_ALIAS_MAP;
use crate::spec::Spec;
use crate::ssa::{SsaBlock, SsaModule, SsaOp};
use crate::surface::{
  SurfaceAdtType, SurfaceAdtVariant, SurfaceDecl, SurfaceEdge, SurfaceEdgeCond, SurfaceInput,
  SurfaceModule, SurfaceNode, SurfacePort, SurfaceScope,
};
use crate::{MeaningError, MeaningResult};
use std::collections::{BTreeSet, HashMap, HashSet};

fn normalize_builtin_uses(name: &str, spec: &Spec) -> Option<String> {
  if let Some(stripped) = name.strip_prefix("builtins.") {
    if spec.builtins.contains(stripped) {
      return Some(stripped.to_string());
    }
    for (alias, target) in STDLIB_ALIAS_MAP {
      if stripped == *alias && spec.builtins.contains(target) {
        return Some((*target).to_string());
      }
    }
  }

  for (alias, target) in STDLIB_ALIAS_MAP {
    if name == *alias && spec.builtins.contains(target) {
      return Some((*target).to_string());
    }
  }

  if spec.builtins.contains(name) {
    return Some(name.to_string());
  }

  None
}

/// AST -> Surface IR
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn lower_to_surface(ast: &AstModule, diags: &mut Diagnostics) -> MeaningResult<SurfaceModule> {
  trace!(items = ast.items.len(), "lowering ast to surface");
  let mut types = Vec::new();
  let mut adt_types = Vec::new();
  let mut inputs = Vec::new();
  let mut decls = Vec::new();
  let mut nodes = Vec::new();
  let mut edges = Vec::new();
  let mut scopes = Vec::new();

  for it in &ast.items {
    match it {
      AstItem::TypeDecl { name, .. } => {
        types.push(name.clone());
      }
      // LOW: lowering rule target_pattern 검증 없음 수정
      // InputDecl의 name과 ty가 유효한지 검증
      AstItem::InputDecl { name, ty, .. } => {
        // 이름이 비어있지 않은지 검증
        if name.is_empty() {
          return Err(MeaningError::Lowering {
            message: "Input declaration has empty name".to_string(),
            span: None,
          });
        }
        // 타입이 비어있지 않은지 검증
        if ty.is_empty() {
          return Err(MeaningError::Lowering {
            message: format!("Input declaration '{}' has empty type", name),
            span: None,
          });
        }
        inputs.push(SurfaceInput {
          name: name.clone(),
          ty: ty.clone(),
        });
      }
      AstItem::ExternDecl { name, sig, .. } => {
        // Stage-2: 포트 정보 변환
        let sig_inputs: Vec<SurfacePort> = sig
          .inputs
          .iter()
          .map(|p| SurfacePort {
            name: p.name.clone(),
            ty: p.ty.clone(),
          })
          .collect();
        let sig_outputs: Vec<SurfacePort> = sig
          .outputs
          .iter()
          .map(|p| SurfacePort {
            name: p.name.clone(),
            ty: p.ty.clone(),
          })
          .collect();

        decls.push(SurfaceDecl::Extern {
          name: name.clone(),
          input: sig.input.clone(),
          output: sig.output.clone(),
          inputs: sig_inputs,
          outputs: sig_outputs,
        });
      }
      AstItem::NodeDecl {
        name,
        uses,
        kind,
        optional,
        scope,
        cost,
        priority,
        ..
      } => {
        nodes.push(SurfaceNode {
          name: name.clone(),
          uses: uses.clone(),
          kind: kind.clone(),
          optional: *optional,
          scope: scope.clone(),
          cost: cost.clone(),
          priority: *priority,
        });
      }
      AstItem::EdgeDecl { from, to, cond, .. } => {
        // Stage-2: EdgeSource 처리
        let mut edge = match from {
          EdgeSource::Input { name } => {
            SurfaceEdge::from_input(name.clone(), to.node.clone(), to.port.clone())
          }
          EdgeSource::Node { node, port } => {
            SurfaceEdge::ported(node.clone(), port.clone(), to.node.clone(), to.port.clone())
          }
        };

        // Stage-3.2: 조건 변환
        if let Some(c) = cond {
          edge.cond = Some(match c {
            EdgeCondAst::When(g) => SurfaceEdgeCond::When(g.clone()),
            EdgeCondAst::Unless(g) => SurfaceEdgeCond::Unless(g.clone()),
            EdgeCondAst::OnFail(n) => SurfaceEdgeCond::OnFail(n.clone()),
            EdgeCondAst::WhenUnless { when, unless } => SurfaceEdgeCond::WhenUnless {
              when: when.clone(),
              unless: unless.clone(),
            },
            EdgeCondAst::AllWhen(gates) => SurfaceEdgeCond::AllWhen(gates.clone()),
            EdgeCondAst::AllUnless(gates) => SurfaceEdgeCond::AllUnless(gates.clone()),
          });
        }

        edges.push(edge);
      }
      AstItem::ScopeDecl { name, policy, .. } => {
        scopes.push(SurfaceScope {
          name: name.clone(),
          policy: policy.clone(),
        });
      }
      AstItem::AdtTypeDecl(adt) => {
        // Phase Y09: ADT types
        // Register the type name in types list for compatibility
        types.push(adt.name.clone());

        // Convert variants to Surface IR
        let variants: Vec<SurfaceAdtVariant> = adt
          .variants
          .iter()
          .map(|v| SurfaceAdtVariant {
            name: v.name.clone(),
            fields: v.fields.clone(),
          })
          .collect();

        adt_types.push(SurfaceAdtType {
          name: adt.name.clone(),
          params: adt.params.clone(),
          variants,
        });
      }
      AstItem::ImportDecl { path, .. } => {
        // Y07a: Import 선언은 모듈 해석 단계에서 처리됨
        // lowering 단계에서는 무시 (향후 모듈 해석 로직에서 처리)
        diags.push(
          format!(
            "import '{}' will be resolved during module resolution (Y07a)",
            path
          ),
          None,
        );
      }
      AstItem::TestDecl { name, expr: _, .. } => {
        // Y11a: 테스트 선언은 테스트 러너에서 처리됨
        // lowering 단계에서는 무시 (향후 테스트 러너에서 처리)
        diags.push(
          format!("test '{}' will be executed by test runner (Y11a)", name),
          None,
        );
      }
    }
  }

  if decls.is_empty() {
    diags.push("no extern declarations found", None);
  }

  let module = SurfaceModule {
    name: ast.name.clone(),
    types,
    adt_types,
    inputs,
    decls,
    nodes,
    edges,
    scopes,
  };
  trace!(
    surface_inputs = module.inputs.len(),
    surface_nodes = module.nodes.len(),
    surface_edges = module.edges.len(),
    "surface module built"
  );
  Ok(module)
}

/// Surface -> FxCore (Meaning IR)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn lower_to_fxcore(
  surface: &SurfaceModule,
  diags: &mut Diagnostics,
) -> MeaningResult<FxCoreModule> {
  lower_to_fxcore_with_spec(surface, diags, &Spec::with_defaults())
}

/// Surface -> FxCore (Meaning IR) with spec validation
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn lower_to_fxcore_with_spec(
  surface: &SurfaceModule,
  _diags: &Diagnostics,
  spec: &Spec,
) -> MeaningResult<FxCoreModule> {
  trace!(
    surface_inputs = surface.inputs.len(),
    surface_nodes = surface.nodes.len(),
    surface_edges = surface.edges.len(),
    "lowering surface to fxcore"
  );
  // Spec 기반 검증: 타입 확인
  // 사용자 정의 타입은 허용 (surface.types에 선언된 타입은 사용자 정의)
  // spec에 있는 타입은 builtin/stdlib 타입이므로, 사용자 정의 타입과 충돌하지 않음
  // 실제로는 builtin/morphism 사용 시에만 spec 검증 수행

  // Stage-2: 입력 변환
  // 입력 타입은 사용자 정의 타입일 수 있으므로 spec 검증에서 제외
  let inputs: Vec<FxInput> = surface
    .inputs
    .iter()
    .map(|i| FxInput {
      name: i.name.clone(),
      ty: i.ty.clone(),
    })
    .collect();

  let mut morphisms = Vec::new();

  for d in &surface.decls {
    match d {
      SurfaceDecl::Extern {
        name,
        input,
        output,
        inputs: port_inputs,
        outputs: port_outputs,
      } => {
        // 최소: extern은 기본적으로 Pure로 두되,
        // 이름 prefix로 World로 승격하는 규칙(예: "io." 또는 "world.")
        let effect = if name.starts_with("io.") || name.starts_with("world.") {
          Effect::World
        } else {
          Effect::Pure
        };

        // Stage-2: 포트 정보 변환
        let fx_inputs: Vec<FxPort> = port_inputs
          .iter()
          .map(|p| FxPort {
            name: p.name.clone(),
            ty: p.ty.clone(),
          })
          .collect();
        let fx_outputs: Vec<FxPort> = port_outputs
          .iter()
          .map(|p| FxPort {
            name: p.name.clone(),
            ty: p.ty.clone(),
          })
          .collect();

        morphisms.push(FxMorphism {
          name: name.clone(),
          input: input.clone(),
          output: output.clone(),
          inputs: fx_inputs,
          outputs: fx_outputs,
          effect,
        });
      }
    }
  }

  // Build scope policy lookup
  let scope_policies: HashMap<&str, ScopePolicy> = surface
    .scopes
    .iter()
    .map(|s| {
      let policy = match s.policy.as_str() {
        "failfast" => ScopePolicy::FailFast,
        "isolate" => ScopePolicy::Isolate,
        _ => ScopePolicy::BestEffort,
      };
      (s.name.as_str(), policy)
    })
    .collect();

  let node_scopes: HashMap<&str, &str> = surface
    .nodes
    .iter()
    .map(|n| (n.name.as_str(), n.scope.as_deref().unwrap_or("global")))
    .collect();

  let scope_policy = |scope_name: &str| {
    scope_policies
      .get(scope_name)
      .copied()
      .unwrap_or(ScopePolicy::BestEffort)
  };

  let cond_refs = |cond: &SurfaceEdgeCond| -> Vec<String> {
    match cond {
      SurfaceEdgeCond::When(name)
      | SurfaceEdgeCond::Unless(name)
      | SurfaceEdgeCond::OnFail(name) => vec![name.clone()],
      SurfaceEdgeCond::WhenUnless { when, unless } => vec![when.clone(), unless.clone()],
      SurfaceEdgeCond::AllWhen(names) | SurfaceEdgeCond::AllUnless(names) => names.clone(),
    }
  };

  let mut edges_for_lowering = surface.edges.clone();
  let node_names: HashSet<&str> = surface.nodes.iter().map(|n| n.name.as_str()).collect();
  let mut dependency_edges: HashSet<(String, String)> = edges_for_lowering
    .iter()
    .filter(|edge| edge.from_input.is_none())
    .filter(|edge| {
      edge
        .to_endpoint
        .as_ref()
        .and_then(|ep| ep.port.as_ref())
        .is_none()
    })
    .map(|edge| (edge.from.clone(), edge.to.clone()))
    .collect();

  for edge in &surface.edges {
    let Some(cond) = &edge.cond else {
      continue;
    };
    for gate in cond_refs(cond) {
      if !node_names.contains(gate.as_str()) {
        continue;
      }
      let key = (gate.clone(), edge.to.clone());
      if dependency_edges.insert(key.clone()) {
        edges_for_lowering.push(SurfaceEdge::simple(key.0, key.1));
      }
    }
  }

  // Stage-4.1: scope boundary validation (Isolate containment)
  for edge in &surface.edges {
    let to_scope = node_scopes.get(edge.to.as_str()).copied();

    if !edge.is_input_source() {
      if let (Some(from_scope), Some(to_scope)) =
        (node_scopes.get(edge.from.as_str()).copied(), to_scope)
      {
        if from_scope != to_scope && scope_policy(from_scope) == ScopePolicy::Isolate {
          return Err(MeaningError::ContractViolation(
            format!(
              "edge `{}` -> `{}` crosses isolate scope boundary `{}` -> `{}`",
              edge.from, edge.to, from_scope, to_scope
            ),
            None,
          ));
        }
      }
    }

    if let (Some(cond), Some(to_scope)) = (&edge.cond, to_scope) {
      for gate in cond_refs(cond) {
        let Some(gate_scope) = node_scopes.get(gate.as_str()).copied() else {
          continue;
        };
        if gate_scope != to_scope && scope_policy(gate_scope) == ScopePolicy::Isolate {
          return Err(MeaningError::ContractViolation(
            format!(
              "edge `{}` -> `{}` uses gate `{}` crossing isolate scope boundary `{}` -> `{}`",
              edge.from, edge.to, gate, gate_scope, to_scope
            ),
            None,
          ));
        }
      }
    }
  }

  // W04: builtin 노드 발견 시 spec에서 morphism 자동 주입
  // builtin 표현 방식: dotless 이름 (`add`, `mul` 등) - pnix-ir-adapter와 정합
  let mut used_builtins: HashSet<String> = HashSet::new();
  let mut normalized_uses: HashMap<String, String> = HashMap::new();
  for node in &surface.nodes {
    // extern이 아닌 경우 builtin인지 확인
    let is_extern = morphisms.iter().any(|m| m.name == node.uses);
    let effective_uses = if is_extern {
      node.uses.clone()
    } else if let Some(normalized) = normalize_builtin_uses(&node.uses, spec) {
      normalized
    } else {
      // unknown builtin: 명시적 에러
      // SurfaceModule에는 span 정보가 없으므로 None 전달
      return Err(crate::MeaningError::UnresolvedSymbol(
        format!(
          "S2: unknown morphism/builtin: {} (not in spec catalog)",
          node.uses
        ),
        None,
      ));
    };

    if !is_extern {
      used_builtins.insert(effective_uses.clone());
    }
    normalized_uses.insert(node.name.clone(), effective_uses);
  }

  // builtin morphism 주입
  for builtin_name in &used_builtins {
    if let Some(builtin_decl) = spec.builtins.get(builtin_name) {
      // 시그니처 파싱: "Num → Num → Num" 형식
      let sig_parts: Vec<&str> = builtin_decl.signature.split(" → ").collect();
      let input_ty = sig_parts
        .first()
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Unknown".to_string());
      let output_ty = sig_parts
        .last()
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

      let morphism = FxMorphism::simple(
        builtin_name.to_string(), // dotless builtin 이름
        input_ty,
        output_ty,
        builtin_decl.effect,
      );
      morphisms.push(morphism);
    }
  }

  // morphism_map 생성 (주입된 builtin 포함)
  let morphism_map: HashMap<&str, &FxMorphism> =
    morphisms.iter().map(|m| (m.name.as_str(), m)).collect();

  // Convert nodes with full properties and execution hints
  let nodes: Vec<FxNode> = surface
    .nodes
    .iter()
    .map(|n| {
      let uses = normalized_uses
        .get(&n.name)
        .cloned()
        .unwrap_or_else(|| n.uses.clone());
      // Spec 기반 검증: morphism 확인 (이제 morphism_map에 builtin도 포함됨)
      if !morphism_map.contains_key(uses.as_str()) {
        // SurfaceModule에는 span 정보가 없으므로 None 전달
        return Err(crate::MeaningError::UnresolvedSymbol(
          format!(
            "unknown morphism: {} (not found in morphisms or spec catalog)",
            uses
          ),
          None,
        ));
      }

      let kind = match n.kind.as_deref() {
        Some("gate") => NodeKind::Gate,
        _ => NodeKind::Normal,
      };

      let cost = match n.cost.as_deref() {
        Some("tiny") => CostHint::Tiny,
        Some("light") => CostHint::Light,
        Some("heavy") => CostHint::Heavy,
        Some("xheavy") => CostHint::XHeavy,
        _ => CostHint::Medium,
      };

      // Compute execution hints (core calculates, executor trusts)
      let required_inputs: Vec<String> = morphism_map
        .get(uses.as_str())
        .map(|m| m.inputs.iter().map(|p| p.name.clone()).collect())
        .unwrap_or_default();

      // may_skip: optional OR in isolate/best_effort scope
      let scope_policy = n
        .scope
        .as_ref()
        .and_then(|s| scope_policies.get(s.as_str()).copied())
        .unwrap_or(ScopePolicy::BestEffort);
      let may_skip =
        n.optional || matches!(scope_policy, ScopePolicy::Isolate | ScopePolicy::BestEffort);

      let skip_policy = if n.optional {
        SkipPolicy::Skip
      } else {
        SkipPolicy::Error
      };

      Ok(FxNode {
        name: n.name.clone(),
        uses,
        kind,
        optional: n.optional,
        scope: n.scope.clone().unwrap_or_else(|| "global".into()),
        cost,
        priority: n.priority.unwrap_or(0),
        contract: ExecutionContract {
          required_inputs,
          may_skip,
          skip_policy,
          replay: None,
        },

        meta: None,
      })
    })
    .collect::<Result<_, _>>()?;

  // Convert edges with conditions
  let edges: Vec<FxEdge> = edges_for_lowering
    .iter()
    .map(|e| {
      let mut edge = if e.is_input_source() {
        // Stage-2: 입력 엣지
        let to_port = e.to_endpoint.as_ref().and_then(|ep| ep.port.clone());
        FxEdge::from_input(
          e.from_input.clone().unwrap_or_default(),
          e.to.clone(),
          to_port,
        )
      } else {
        // Stage-2: 노드 간 포트 엣지
        let from_port = e.from_endpoint.as_ref().and_then(|ep| ep.port.clone());
        let to_port = e.to_endpoint.as_ref().and_then(|ep| ep.port.clone());
        FxEdge::ported(e.from.clone(), from_port, e.to.clone(), to_port)
      };

      // Stage-3.2: 조건 변환
      if let Some(c) = &e.cond {
        edge.cond = Some(match c {
          SurfaceEdgeCond::When(g) => EdgeCond::When(g.clone()),
          SurfaceEdgeCond::Unless(g) => EdgeCond::Unless(g.clone()),
          SurfaceEdgeCond::OnFail(n) => EdgeCond::OnFail(n.clone()),
          SurfaceEdgeCond::WhenUnless { when, unless } => EdgeCond::WhenUnless {
            when: when.clone(),
            unless: unless.clone(),
          },
          SurfaceEdgeCond::AllWhen(gates) => EdgeCond::AllWhen(gates.clone()),
          SurfaceEdgeCond::AllUnless(gates) => EdgeCond::AllUnless(gates.clone()),
        });
      }

      edge
    })
    .collect();

  // Convert scopes
  let scopes: Vec<FxScope> = surface
    .scopes
    .iter()
    .map(|s| {
      let policy = match s.policy.as_str() {
        "failfast" => ScopePolicy::FailFast,
        "isolate" => ScopePolicy::Isolate,
        _ => ScopePolicy::BestEffort,
      };

      // Collect nodes that belong to this scope
      let scope_nodes: Vec<String> = surface
        .nodes
        .iter()
        .filter(|n| n.scope.as_ref() == Some(&s.name))
        .map(|n| n.name.clone())
        .collect();

      FxScope {
        name: s.name.clone(),
        nodes: scope_nodes,
        policy,
      }
    })
    .collect();

  // Y09: Convert ADT types from Surface IR to FxCore IR
  let adt_types: Vec<FxAdtType> = surface
    .adt_types
    .iter()
    .map(|adt| FxAdtType {
      name: adt.name.clone(),
      params: adt.params.clone(),
      variants: adt
        .variants
        .iter()
        .map(|v| FxAdtVariant {
          name: v.name.clone(),
          fields: v.fields.clone(),
        })
        .collect(),
    })
    .collect();

  let module = FxCoreModule {
    meta: Default::default(), // Will be filled in emit.rs
    name: surface.name.clone(),
    types: surface.types.clone(),
    adt_types: adt_types.clone(),
    adttypes: adt_types, // Y09: compat alias
    inputs,
    morphisms,
    nodes,
    edges,
    scopes,
  };
  trace!(
    fxcore_inputs = module.inputs.len(),
    fxcore_nodes = module.nodes.len(),
    fxcore_edges = module.edges.len(),
    "fxcore module built"
  );
  Ok(module)
}

/// FxCore -> SSA
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn lower_to_ssa(fx: &FxCoreModule, _diags: &Diagnostics) -> MeaningResult<SsaModule> {
  let mut ops: Vec<(crate::ssa::SSAValue, SsaOp)> = Vec::new();
  let mut next_reg: usize = 0;
  let mut node_regs: HashMap<String, crate::ssa::SSAValue> = HashMap::new();
  let mut input_regs: HashMap<String, crate::ssa::SSAValue> = HashMap::new();

  let morph_map: HashMap<&str, &FxMorphism> =
    fx.morphisms.iter().map(|m| (m.name.as_str(), m)).collect();

  let node_order = topo_order_nodes(fx);

  for node_name in node_order {
    let node = match fx.nodes.iter().find(|n| n.name == node_name) {
      Some(node) => node,
      None => continue,
    };

    let mut can_lower = node.kind == NodeKind::Normal && !node.optional && node.scope == "global";
    let mut inputs: Vec<crate::ssa::SSAValue> = Vec::new();

    let edges: Vec<&FxEdge> = fx.edges.iter().filter(|e| e.to == node.name).collect();
    if edges.iter().any(|e| e.cond.is_some()) {
      can_lower = false;
    }

    let resolve_edge_value = |edge: &FxEdge,
                              node_regs: &HashMap<String, crate::ssa::SSAValue>,
                              input_regs: &mut HashMap<String, crate::ssa::SSAValue>,
                              ops: &mut Vec<(crate::ssa::SSAValue, SsaOp)>,
                              next_reg: &mut usize|
     -> Option<crate::ssa::SSAValue> {
      if edge.from == "input" {
        let input_name = edge.from_input.as_deref().filter(|name| !name.is_empty())?;
        if let Some(reg) = input_regs.get(input_name) {
          return Some(*reg);
        }
        let var_name = format!("input_{}", input_name);
        let reg = emit_ssa_op(ops, next_reg, SsaOp::LoadVar(var_name));
        input_regs.insert(input_name.to_string(), reg);
        return Some(reg);
      }

      if let Some(port) = edge.from_port.as_deref() {
        if port != "out" {
          return None;
        }
      }
      node_regs.get(&edge.from).copied()
    };

    if can_lower {
      if let Some(morph) = morph_map.get(node.uses.as_str()) {
        let ports: Vec<&str> = morph.inputs.iter().map(|p| p.name.as_str()).collect();
        if ports.is_empty() {
          can_lower = false;
        } else if ports.len() == 1 && edges.iter().all(|e| e.to_port.is_none()) && edges.len() == 1
        {
          if let Some(value) = resolve_edge_value(
            edges[0],
            &node_regs,
            &mut input_regs,
            &mut ops,
            &mut next_reg,
          ) {
            inputs.push(value);
          } else {
            can_lower = false;
          }
        } else {
          for port in ports {
            let edge = edges.iter().find(|e| e.to_port.as_deref() == Some(port));
            match edge {
              Some(edge) => {
                if let Some(value) =
                  resolve_edge_value(edge, &node_regs, &mut input_regs, &mut ops, &mut next_reg)
                {
                  inputs.push(value);
                } else {
                  can_lower = false;
                  break;
                }
              }
              None => {
                can_lower = false;
                break;
              }
            }
          }
        }
      } else {
        can_lower = false;
      }
    }

    let reg = if can_lower {
      if let Some(op) = ssa_op_from_morphism(&node.uses, &inputs) {
        emit_ssa_op(&mut ops, &mut next_reg, op)
      } else {
        let extern_args: Vec<String> = inputs.iter().map(|v| format!("r{}", v.index())).collect();
        emit_ssa_op(
          &mut ops,
          &mut next_reg,
          SsaOp::CallExtern {
            name: node.uses.clone(),
            args: extern_args,
          },
        )
      }
    } else {
      emit_ssa_op(
        &mut ops,
        &mut next_reg,
        SsaOp::CallExtern {
          name: node.uses.clone(),
          args: vec![],
        },
      )
    };

    node_regs.insert(node.name.clone(), reg);
  }

  // 노드가 없으면 morphism에서 생성 (Stage-0 호환)
  if ops.is_empty() {
    for m in &fx.morphisms {
      emit_ssa_op(
        &mut ops,
        &mut next_reg,
        SsaOp::CallExtern {
          name: m.name.clone(),
          args: vec![],
        },
      );
    }
  }

  if ops.is_empty() {
    return Err(MeaningError::Internal(
      "ssa lowering produced no ops: empty FxCore module".to_string(),
      None,
    ));
  }

  let ret = if let Some(reg) = node_regs.get("result") {
    *reg
  } else {
    let sinks = collect_sink_nodes(fx);
    if sinks.len() == 1 {
      node_regs.get(&sinks[0]).copied().unwrap_or(ops[0].0)
    } else {
      ops.last().map(|(reg, _)| *reg).unwrap_or(ops[0].0)
    }
  };

  Ok(SsaModule {
    name: fx.name.clone(),
    blocks: vec![SsaBlock {
      label: "entry".into(),
      ops,
      ret,
    }],
  })
}

fn topo_order_nodes(fx: &FxCoreModule) -> Vec<String> {
  let mut indeg: HashMap<String, usize> = fx.nodes.iter().map(|n| (n.name.clone(), 0)).collect();
  let mut adj: HashMap<String, Vec<String>> =
    fx.nodes.iter().map(|n| (n.name.clone(), vec![])).collect();

  for e in &fx.edges {
    if e.from == "input" {
      continue;
    }
    if let Some(targets) = adj.get_mut(&e.from) {
      targets.push(e.to.clone());
    }
    if let Some(deg) = indeg.get_mut(&e.to) {
      *deg += 1;
    }
  }

  for targets in adj.values_mut() {
    targets.sort();
  }

  let mut ready: BTreeSet<String> = indeg
    .iter()
    .filter_map(|(n, &d)| if d == 0 { Some(n.clone()) } else { None })
    .collect();
  let mut order = Vec::new();

  // 결정론 보장: BTreeSet의 pop_first()를 사용하여 항상 가장 작은 요소부터 처리
  while let Some(n) = ready.pop_first() {
    order.push(n.clone());
    if let Some(targets) = adj.get(&n) {
      for m in targets {
        if let Some(d) = indeg.get_mut(m) {
          *d = d.saturating_sub(1);
          if *d == 0 {
            ready.insert(m.clone());
          }
        }
      }
    }
  }

  order
}

fn emit_ssa_op(
  ops: &mut Vec<(crate::ssa::SSAValue, SsaOp)>,
  next_reg: &mut usize,
  op: SsaOp,
) -> crate::ssa::SSAValue {
  let reg = crate::ssa::SSAValue(*next_reg);
  *next_reg += 1;
  ops.push((reg, op));
  reg
}

fn collect_sink_nodes(fx: &FxCoreModule) -> Vec<String> {
  let mut has_outgoing = HashSet::new();
  for edge in &fx.edges {
    if edge.from != "input" {
      has_outgoing.insert(edge.from.clone());
    }
  }
  let mut sinks: Vec<String> = fx
    .nodes
    .iter()
    .map(|n| n.name.clone())
    .filter(|name| !has_outgoing.contains(name))
    .collect();
  sinks.sort();
  sinks
}

fn ssa_op_from_morphism(name: &str, inputs: &[crate::ssa::SSAValue]) -> Option<SsaOp> {
  let op_name = name.strip_prefix("builtins.").unwrap_or(name);
  match op_name {
    "add" | "+" if inputs.len() == 2 => Some(SsaOp::Add(inputs[0], inputs[1])),
    "sub" | "-" | "subtract" if inputs.len() == 2 => Some(SsaOp::Sub(inputs[0], inputs[1])),
    "mul" | "*" | "multiply" if inputs.len() == 2 => Some(SsaOp::Mul(inputs[0], inputs[1])),
    "div" | "/" | "divide" if inputs.len() == 2 => Some(SsaOp::Div(inputs[0], inputs[1])),
    "mod" | "%" if inputs.len() == 2 => Some(SsaOp::Mod(inputs[0], inputs[1])),
    "neg" if inputs.len() == 1 => Some(SsaOp::Neg(inputs[0])),
    "floor" if inputs.len() == 1 => Some(SsaOp::Floor(inputs[0])),
    "ceil" if inputs.len() == 1 => Some(SsaOp::Ceil(inputs[0])),
    "abs" if inputs.len() == 1 => Some(SsaOp::Abs(inputs[0])),
    "sqrt" if inputs.len() == 1 => Some(SsaOp::Sqrt(inputs[0])),
    "sin" if inputs.len() == 1 => Some(SsaOp::Sin(inputs[0])),
    "cos" if inputs.len() == 1 => Some(SsaOp::Cos(inputs[0])),
    "not" if inputs.len() == 1 => Some(SsaOp::Not(inputs[0])),
    "and" if inputs.len() == 2 => Some(SsaOp::And(inputs[0], inputs[1])),
    "or" if inputs.len() == 2 => Some(SsaOp::Or(inputs[0], inputs[1])),
    "lt" if inputs.len() == 2 => Some(SsaOp::Lt(inputs[0], inputs[1])),
    "gt" if inputs.len() == 2 => Some(SsaOp::Gt(inputs[0], inputs[1])),
    "le" if inputs.len() == 2 => Some(SsaOp::Le(inputs[0], inputs[1])),
    "ge" if inputs.len() == 2 => Some(SsaOp::Ge(inputs[0], inputs[1])),
    "eq" if inputs.len() == 2 => Some(SsaOp::Eq(inputs[0], inputs[1])),
    "ne" if inputs.len() == 2 => Some(SsaOp::Ne(inputs[0], inputs[1])),
    "if" | "select" if inputs.len() == 3 => Some(SsaOp::Select(inputs[0], inputs[1], inputs[2])),
    _ => None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_lower_to_ssa_empty_module_errors() {
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "empty".to_string(),
      types: Vec::new(),
      adt_types: Vec::new(),
      adttypes: Vec::new(),
      inputs: Vec::new(),
      morphisms: Vec::new(),
      nodes: Vec::new(),
      edges: Vec::new(),
      scopes: Vec::new(),
    };
    let diags = Diagnostics::default();
    let err = lower_to_ssa(&fx, &diags).unwrap_err();
    assert!(matches!(err, MeaningError::Internal(_, _)));
  }

  #[test]
  fn test_normalize_builtin_uses_accepts_builtins_alias_form() {
    let spec = Spec::with_defaults();
    assert_eq!(
      normalize_builtin_uses("builtins.Process.spawn", &spec),
      Some("processSpawn".to_string())
    );
    assert_eq!(
      normalize_builtin_uses("builtins.process.spawn", &spec),
      Some("processSpawn".to_string())
    );
  }

  #[test]
  fn test_normalize_builtin_uses_accepts_process_alias_form() {
    let spec = Spec::with_defaults();
    assert_eq!(
      normalize_builtin_uses("Process.spawn", &spec),
      Some("processSpawn".to_string())
    );
    assert_eq!(
      normalize_builtin_uses("process.spawn", &spec),
      Some("processSpawn".to_string())
    );
  }

  #[test]
  fn test_isolate_scope_boundary_rejects_outbound_edge() {
    let surface = SurfaceModule {
      name: "scope_boundary".to_string(),
      types: vec!["Num".to_string()],
      adt_types: Vec::new(),
      inputs: Vec::new(),
      decls: Vec::new(),
      nodes: vec![
        SurfaceNode {
          name: "a".to_string(),
          uses: "add".to_string(),
          kind: None,
          optional: false,
          scope: Some("s1".to_string()),
          cost: None,
          priority: None,
        },
        SurfaceNode {
          name: "b".to_string(),
          uses: "add".to_string(),
          kind: None,
          optional: false,
          scope: None,
          cost: None,
          priority: None,
        },
      ],
      edges: vec![SurfaceEdge::simple("a".to_string(), "b".to_string())],
      scopes: vec![SurfaceScope {
        name: "s1".to_string(),
        policy: "isolate".to_string(),
      }],
    };

    let diags = Diagnostics::default();
    let spec = Spec::with_defaults();
    let err = lower_to_fxcore_with_spec(&surface, &diags, &spec).unwrap_err();

    assert!(
      matches!(err, MeaningError::ContractViolation(_, _)),
      "expected contract violation, got: {err:?}"
    );
  }

  #[test]
  fn test_edge_cond_adds_dependency_edge() {
    let mut edge = SurfaceEdge::from_input("x".to_string(), "n1".to_string(), None);
    edge.cond = Some(SurfaceEdgeCond::When("gate1".to_string()));

    let surface = SurfaceModule {
      name: "cond_dependency".to_string(),
      types: vec!["Num".to_string()],
      adt_types: Vec::new(),
      inputs: vec![SurfaceInput {
        name: "x".to_string(),
        ty: "Num".to_string(),
      }],
      decls: Vec::new(),
      nodes: vec![
        SurfaceNode {
          name: "gate1".to_string(),
          uses: "add".to_string(),
          kind: Some("gate".to_string()),
          optional: false,
          scope: None,
          cost: None,
          priority: None,
        },
        SurfaceNode {
          name: "n1".to_string(),
          uses: "add".to_string(),
          kind: None,
          optional: false,
          scope: None,
          cost: None,
          priority: None,
        },
      ],
      edges: vec![edge],
      scopes: Vec::new(),
    };

    let diags = Diagnostics::default();
    let spec = Spec::with_defaults();
    let fx = lower_to_fxcore_with_spec(&surface, &diags, &spec).unwrap();

    let has_dependency = fx.edges.iter().any(|e| {
      e.from == "gate1"
        && e.to == "n1"
        && e.from_input.is_none()
        && e.from_port.is_none()
        && e.to_port.is_none()
        && e.cond.is_none()
    });

    assert!(
      has_dependency,
      "expected implicit dependency edge from gate1 to n1"
    );
  }
}
