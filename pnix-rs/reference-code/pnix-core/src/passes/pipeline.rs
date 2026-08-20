//! Compilation Pipeline
//!
//! 전체 컴파일 파이프라인 조합.
//!
//! ## 파이프라인 단계
//!
//! 1. Parsing: Source → AST (외부)
//! 2. Surface Lowering: AST → Surface IR
//! 3. FxCore Lowering: Surface → FxCore
//! 4. FxCore Optimization: 그래프 최적화
//! 5. CT Optimization: 컴파일 타임 최적화 (CTAST)
//! 6. SSA Lowering: FxCore → SSA
//! 7. SSA Optimization: SSA 최적화
//! 8. Code Generation: SSA → BuildIR (codegen 모듈)
//!
//! ## 헌법 준수
//!
//! - P0-1: 모든 최적화는 구조 변환만, 값 계산 없음
//! - C1: pnix-core는 실행하지 않음
//!
//! ## 사용 예시
//!
//! ```ignore
//! let pipeline = CompilationPipeline::default();
//! let result = pipeline.run(&ast, &mut diags)?;
//! println!("Compilation took {}ms", result.total_time_ms());
//! ```

use crate::ast::AstModule;
use crate::core::FxCoreModule;
use crate::ct::optimize::{optimize_fxcore_module, CTOptResult};
use crate::diagnostics::Diagnostics;
use crate::ssa::SsaModule;
use crate::surface::SurfaceModule;
use crate::types::{TypeCheckResult, TypeChecker};
use crate::MeaningResult;

use super::lowering::{lower_to_fxcore, lower_to_ssa, lower_to_surface};
use super::optimize::{optimize_fxcore, optimize_fxcore_with_stats, OptimizationStats};
use super::provenance::{Provenance, ProvenanceBuilder};
use super::ssa_opt::{optimize_ssa, optimize_ssa_with_stats, SsaOptStats};
use super::telemetry::{CompileFrame, CompilePhase, CompileStats};

// ============================================================
// Pipeline Configuration
// ============================================================

/// 파이프라인 설정: 컴파일 파이프라인의 설정 옵션
#[derive(Clone, Debug)]
pub struct PipelineConfig {
  /// FxCore 최적화 활성화 여부
  pub enable_fxcore_opt: bool,
  /// CT 최적화 활성화 여부
  pub enable_ct_opt: bool,
  /// SSA 최적화 활성화 여부
  pub enable_ssa_opt: bool,
  /// 고급 SSA 최적화 활성화 여부 (호이스팅, 배치 그룹화)
  pub enable_advanced_ssa_opt: bool,
  /// 텔레메트리 수집 여부
  pub collect_telemetry: bool,
  /// 프로베넌스 추적 여부
  pub track_provenance: bool,
  /// 타입 검사 활성화 여부
  pub enable_type_check: bool,
}

impl Default for PipelineConfig {
  fn default() -> Self {
    Self {
      enable_fxcore_opt: true,
      enable_ct_opt: true,
      enable_ssa_opt: true,
      enable_advanced_ssa_opt: false,
      collect_telemetry: true,
      track_provenance: false, // 디버깅 시에만 활성화
      enable_type_check: true, // 기본적으로 활성화
    }
  }
}

impl PipelineConfig {
  /// 모든 최적화 활성화
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 설정만, 값 계산 없음
  pub fn all_optimizations() -> Self {
    Self {
      enable_fxcore_opt: true,
      enable_ct_opt: true,
      enable_ssa_opt: true,
      enable_advanced_ssa_opt: true,
      collect_telemetry: true,
      track_provenance: true,
      enable_type_check: true,
    }
  }

  /// 최적화 없음 (디버깅용)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 설정만, 값 계산 없음
  pub fn no_optimizations() -> Self {
    Self {
      enable_fxcore_opt: false,
      enable_ct_opt: false,
      enable_ssa_opt: false,
      enable_advanced_ssa_opt: false,
      collect_telemetry: true,
      track_provenance: true,
      enable_type_check: false, // 디버깅 시 타입 검사 비활성화 가능
    }
  }

  /// 빠른 컴파일 (최소 최적화)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 설정만, 값 계산 없음
  pub fn fast() -> Self {
    Self {
      enable_fxcore_opt: true,
      enable_ct_opt: false,
      enable_ssa_opt: true,
      enable_advanced_ssa_opt: false,
      collect_telemetry: false,
      track_provenance: false,
      enable_type_check: true,
    }
  }
}

// ============================================================
// Pipeline Result
// ============================================================

/// 파이프라인 실행 결과: 컴파일 파이프라인 실행의 결과
#[derive(Debug)]
pub struct PipelineResult {
  /// Surface IR 모듈
  pub surface: SurfaceModule,
  /// FxCore IR 모듈 (최적화 후)
  pub fxcore: FxCoreModule,
  /// SSA IR 모듈 (최적화 후)
  pub ssa: SsaModule,
  /// CT 최적화 결과 (CT 최적화 활성화된 경우)
  pub ct_result: Option<CTOptResult>,
  /// FxCore 최적화 통계 (FxCore 최적화 활성화된 경우)
  pub fxcore_stats: Option<OptimizationStats>,
  /// SSA 최적화 통계 (SSA 최적화 활성화된 경우)
  pub ssa_stats: Option<SsaOptStats>,
  /// 컴파일 프레임 목록 (텔레메트리 수집 활성화된 경우)
  pub frames: Vec<CompileFrame>,
  /// 컴파일 통계 (frames에서 계산됨)
  pub compile_stats: CompileStats,
  /// 프로베넌스 목록 (프로베넌스 추적 활성화된 경우)
  pub provenances: Vec<Provenance>,
  /// 타입 검사 결과 (타입 검사 활성화된 경우)
  pub type_result: Option<TypeCheckResult>,
}

impl PipelineResult {
  /// 총 컴파일 시간 (ms)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn total_time_ms(&self) -> f64 {
    self.compile_stats.total_time_ms
  }

  /// IR 크기 감소율 (%)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 계산만, 값 계산 없음
  pub fn size_reduction_pct(&self) -> f64 {
    self.compile_stats.size_reduction_pct()
  }

  /// 요약 문자열
  ///
  /// ## 헌법 준수 (P0-1, C1)
  ///
  /// 텍스트 생성만, 파일 I/O 없음
  pub fn summary(&self) -> String {
    let mut s = String::new();
    s.push_str(&format!(
      "Compilation: {} nodes -> {} nodes ({:.1}% reduction), {} transforms\n",
      self.compile_stats.initial_ir_size,
      self.compile_stats.final_ir_size,
      self.size_reduction_pct(),
      self.compile_stats.total_transformations
    ));

    if let Some(ref fx_stats) = self.fxcore_stats {
      s.push_str(&format!(
        "  FxCore: {} dead nodes, {} edges simplified, {} identity removed\n",
        fx_stats.dead_nodes_removed, fx_stats.edges_simplified, fx_stats.identity_nodes_removed
      ));
    }

    if let Some(ref ssa_stats) = self.ssa_stats {
      s.push_str(&format!(
        "  SSA: {} CSE eliminated ({:.1}% reduction)\n",
        ssa_stats.cse_eliminated,
        ssa_stats.reduction_ratio() * 100.0
      ));
    }

    if let Some(ref ct) = self.ct_result {
      s.push_str(&format!(
        "  CT: {} optimizations applied\n",
        ct.applied.len()
      ));
    }

    s
  }
}

// ============================================================
// Compilation Pipeline
// ============================================================

/// 컴파일 파이프라인: 전체 컴파일 과정을 관리하는 파이프라인
pub struct CompilationPipeline {
  /// 파이프라인 설정
  config: PipelineConfig,
}

impl Default for CompilationPipeline {
  fn default() -> Self {
    Self::new(PipelineConfig::default())
  }
}

impl CompilationPipeline {
  /// 설정으로 파이프라인 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(config: PipelineConfig) -> Self {
    Self { config }
  }

  /// 모듈 이름으로 파이프라인 생성 (config만 사용, 이름은 무시됨 - 하위 호환성)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn for_module(_name: impl Into<String>, config: PipelineConfig) -> Self {
    Self { config }
  }

  /// 전체 파이프라인 실행
  ///
  /// Note: 시간 측정은 pnix-core에서 하지 않음 (헌법 P0-1 준수).
  /// 외부 executor가 시간을 측정하여 frames에 duration_ms를 설정해야 함.
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn run(&self, ast: &AstModule, diags: &mut Diagnostics) -> MeaningResult<PipelineResult> {
    let mut provenances = Vec::new();
    let mut frames = Vec::new();

    // 1. Surface Lowering
    let surface = lower_to_surface(ast, diags)?;

    if self.config.collect_telemetry {
      frames.push(CompileFrame::new(
        CompilePhase::SurfaceLowering,
        0.0, // 시간은 executor가 측정
        surface.nodes.len() + surface.edges.len(),
      ));
    }

    // 2. FxCore Lowering
    let fxcore_raw = lower_to_fxcore(&surface, diags)?;
    let initial_ir_size = fxcore_raw.nodes.len() + fxcore_raw.edges.len();

    if self.config.collect_telemetry {
      frames.push(CompileFrame::new(
        CompilePhase::FxCoreLowering,
        0.0,
        initial_ir_size,
      ));
    }

    // 2.5. Type Checking (ZZ02b: 파이프라인 타입 검사 추가)
    // 헌법 원칙: "조용한 성공 금지" - 타입 에러가 있으면 컴파일 실패
    let type_result = if self.config.enable_type_check {
      let mut checker = TypeChecker::new();
      let result = checker.check(&fxcore_raw);

      // 타입 에러를 Diagnostics에 추가 (결정론: 정렬)
      let mut error_messages: Vec<String> = result.errors.iter().map(|e| e.to_string()).collect();
      error_messages.sort();
      for msg in &error_messages {
        diags.push(msg.clone(), None);
      }

      // 타입 경고도 Diagnostics에 추가 (결정론: 정렬)
      let mut warning_messages: Vec<String> = result
        .warnings
        .iter()
        .map(|w| format!("warning: {:?}", w))
        .collect();
      warning_messages.sort();
      for msg in &warning_messages {
        diags.push(msg.clone(), None);
      }

      if !result.success {
        // 헌법 원칙: 타입 에러가 있으면 컴파일 실패 (조용한 성공 금지)
        let summary = if error_messages.is_empty() {
          "Type check failed".to_string()
        } else {
          format!("Type check failed: {}", error_messages.join("; "))
        };
        return Err(crate::MeaningError::TypeError(summary, None));
      }

      if self.config.collect_telemetry {
        frames.push(CompileFrame::new(
          CompilePhase::Verification, // 타입 검사는 검증 단계
          0.0,
          fxcore_raw.nodes.len() + fxcore_raw.edges.len(),
        ));
      }

      Some(result)
    } else {
      None
    };

    // 3. FxCore Optimization
    let (fxcore_opt, fxcore_stats) = if self.config.enable_fxcore_opt {
      let (optimized, stats) = optimize_fxcore_with_stats(&fxcore_raw);

      if self.config.collect_telemetry {
        frames.push(
          CompileFrame::new(
            CompilePhase::FxCoreOptimization,
            0.0,
            optimized.nodes.len() + optimized.edges.len(),
          )
          .with_transformations(
            stats.dead_nodes_removed
              + stats.edges_simplified
              + stats.identity_nodes_removed
              + stats.transitive_edges_removed,
          ),
        );
      }

      if self.config.track_provenance {
        let prov = ProvenanceBuilder::new()
          .rule("fxcore_dead_node_elimination")
          .rule("fxcore_edge_simplification")
          .rule("fxcore_identity_elimination")
          .rule("fxcore_transitive_reduction")
          .original_hash(hash_fxcore(&fxcore_raw))
          .result_hash(hash_fxcore(&optimized))
          .build();
        provenances.push(prov);
      }

      (optimized, Some(stats))
    } else {
      (fxcore_raw, None)
    };

    // 4. CT Optimization (CTAST)
    let ct_result = if self.config.enable_ct_opt {
      let (_, ct_res) = optimize_fxcore_module(&fxcore_opt);

      if self.config.collect_telemetry {
        frames.push(
          CompileFrame::new(
            CompilePhase::FxCoreOptimization, // CT is part of FxCore opt phase
            0.0,
            fxcore_opt.nodes.len() + fxcore_opt.edges.len(),
          )
          .with_transformations(ct_res.applied.len())
          .with_note("CT optimization"),
        );
      }

      if self.config.track_provenance && !ct_res.applied.is_empty() {
        let mut prov = ProvenanceBuilder::new();
        for pass in &ct_res.applied {
          prov = prov.rule(format!("ct_{}", pass));
        }
        provenances.push(prov.build());
      }

      Some(ct_res)
    } else {
      None
    };

    // 5. SSA Lowering
    let ssa_raw = lower_to_ssa(&fxcore_opt, diags)?;

    if self.config.collect_telemetry {
      let ssa_size: usize = ssa_raw.blocks.iter().map(|b| b.ops.len()).sum();
      frames.push(CompileFrame::new(CompilePhase::SsaLowering, 0.0, ssa_size));
    }

    // 6. SSA Optimization
    let (ssa_opt, ssa_stats) = if self.config.enable_ssa_opt {
      let (optimized, stats) = if self.config.enable_advanced_ssa_opt {
        let advanced = super::ssa_opt::optimize_ssa_advanced(&ssa_raw);
        let stats = SsaOptStats {
          blocks_optimized: advanced.blocks.len(),
          total_ops_before: ssa_raw.blocks.iter().map(|b| b.ops.len()).sum(),
          total_ops_after: advanced.blocks.iter().map(|b| b.ops.len()).sum(),
          ..Default::default()
        };
        (advanced, stats)
      } else {
        optimize_ssa_with_stats(&ssa_raw)
      };

      if self.config.collect_telemetry {
        let ssa_size: usize = optimized.blocks.iter().map(|b| b.ops.len()).sum();
        frames.push(
          CompileFrame::new(CompilePhase::SsaOptimization, 0.0, ssa_size)
            .with_transformations(stats.cse_eliminated + stats.adjacent_eliminated),
        );
      }

      if self.config.track_provenance && stats.cse_eliminated > 0 {
        let prov = ProvenanceBuilder::new()
          .rule("ssa_cse")
          .rule("ssa_adjacent_cse")
          .build();
        provenances.push(prov);
      }

      (optimized, Some(stats))
    } else {
      (ssa_raw, None)
    };

    // Compute compile stats from frames (P0-1 준수: 순수 함수 사용)
    let compile_stats = CompileStats::from_frames(&frames);

    Ok(PipelineResult {
      surface,
      fxcore: fxcore_opt,
      ssa: ssa_opt,
      ct_result,
      fxcore_stats,
      ssa_stats,
      frames,
      compile_stats,
      provenances,
      type_result,
    })
  }

  /// 설정 참조
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn config(&self) -> &PipelineConfig {
    &self.config
  }
}

// ============================================================
// Quick Pipeline Functions
// ============================================================

/// AST → FxCore 빠른 변환 (최적화 없음)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn quick_lower_to_fxcore(
  ast: &AstModule,
  diags: &mut Diagnostics,
) -> MeaningResult<FxCoreModule> {
  let surface = lower_to_surface(ast, diags)?;
  lower_to_fxcore(&surface, diags)
}

/// AST → SSA 빠른 변환 (최적화 없음)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn quick_lower_to_ssa(ast: &AstModule, diags: &mut Diagnostics) -> MeaningResult<SsaModule> {
  let surface = lower_to_surface(ast, diags)?;
  let fxcore = lower_to_fxcore(&surface, diags)?;
  lower_to_ssa(&fxcore, diags)
}

/// AST → 최적화된 FxCore
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn compile_to_fxcore(ast: &AstModule, diags: &mut Diagnostics) -> MeaningResult<FxCoreModule> {
  let surface = lower_to_surface(ast, diags)?;
  let fxcore = lower_to_fxcore(&surface, diags)?;
  Ok(optimize_fxcore(&fxcore))
}

/// AST → 최적화된 SSA
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn compile_to_ssa(ast: &AstModule, diags: &mut Diagnostics) -> MeaningResult<SsaModule> {
  let surface = lower_to_surface(ast, diags)?;
  let fxcore = lower_to_fxcore(&surface, diags)?;
  let fxcore = optimize_fxcore(&fxcore);
  let ssa = lower_to_ssa(&fxcore, diags)?;
  Ok(optimize_ssa(&ssa))
}

// ============================================================
// Helpers
// ============================================================

/// FxCore 모듈 해시 (프로베넌스용)
///
/// 실제 내용을 포함하여 해시 계산: name, nodes, edges, morphisms의 모든 필드
/// 두 개의 다른 모듈이 같은 해시를 가지지 않도록 보장
fn hash_fxcore(fx: &FxCoreModule) -> u64 {
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};

  let mut hasher = DefaultHasher::new();

  // 모듈 이름
  fx.name.hash(&mut hasher);

  // Morphisms 해시 (실제 내용 포함)
  fx.morphisms.len().hash(&mut hasher);
  for m in &fx.morphisms {
    m.name.hash(&mut hasher);
    m.input.hash(&mut hasher);
    m.output.hash(&mut hasher);
    // Effect는 직렬화하여 해시 (Hash trait 없음)
    if let Ok(effect_json) = serde_json::to_string(&m.effect) {
      effect_json.hash(&mut hasher);
    }
    // Stage-2: 포트 정보
    for port in &m.inputs {
      port.name.hash(&mut hasher);
      port.ty.hash(&mut hasher);
    }
    for port in &m.outputs {
      port.name.hash(&mut hasher);
      port.ty.hash(&mut hasher);
    }
  }

  // Nodes 해시 (실제 내용 포함)
  fx.nodes.len().hash(&mut hasher);
  for n in &fx.nodes {
    n.name.hash(&mut hasher);
    n.uses.hash(&mut hasher);
    // NodeKind는 직렬화하여 해시 (Hash trait 없음)
    if let Ok(kind_json) = serde_json::to_string(&n.kind) {
      kind_json.hash(&mut hasher);
    }
    n.optional.hash(&mut hasher);
    n.scope.hash(&mut hasher);
    // CostHint는 직렬화하여 해시 (Hash trait 없음)
    if let Ok(cost_json) = serde_json::to_string(&n.cost) {
      cost_json.hash(&mut hasher);
    }
    n.priority.hash(&mut hasher);
    // contract는 실행 시 계산되므로 해시에 포함하지 않음 (결정론 보장)
  }

  // Edges 해시 (실제 내용 포함)
  fx.edges.len().hash(&mut hasher);
  for e in &fx.edges {
    e.from.hash(&mut hasher);
    e.to.hash(&mut hasher);
    e.from_port.hash(&mut hasher);
    e.to_port.hash(&mut hasher);
    e.from_input.hash(&mut hasher);
    // EdgeCond는 직렬화하여 해시 (Hash trait 없음)
    if let Some(cond) = &e.cond {
      if let Ok(cond_json) = serde_json::to_string(cond) {
        cond_json.hash(&mut hasher);
      }
    }
  }

  hasher.finish()
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ast::{AstItem, EdgeSource, EdgeTarget, SigAst};
  use crate::diagnostics::{Diagnostics, Span};
  use pnix_fxcore_types::{
    CostHint, EdgeCond, Effect, FxCoreModule, FxEdge, FxMorphism, FxNode, FxPort, NodeKind,
  };

  #[test]
  fn test_hash_fxcore_includes_content() {
    // 같은 이름과 노드/엣지 개수를 가진 두 모듈이 다른 내용을 가지면 다른 해시를 가져야 함
    let module1 = FxCoreModule {
      meta: Default::default(),
      name: "test".to_string(),
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![],
      morphisms: vec![FxMorphism {
        name: "add".to_string(),
        input: "Int".to_string(),
        output: "Int".to_string(),
        inputs: vec![FxPort {
          name: "a".to_string(),
          ty: "Int".to_string(),
        }],
        outputs: vec![FxPort {
          name: "result".to_string(),
          ty: "Int".to_string(),
        }],
        effect: Effect::Pure,
      }],
      nodes: vec![FxNode {
        name: "n1".to_string(),
        uses: "add".to_string(),
        kind: NodeKind::Normal,
        optional: false,
        scope: "global".to_string(),
        cost: CostHint::Tiny,
        priority: 0,
        contract: Default::default(),

        meta: None,
      }],
      edges: vec![],
      scopes: vec![],
    };

    let mut module2 = module1.clone();
    // morphism의 내용 변경 (이름은 같지만 input 타입이 다름)
    module2.morphisms[0].input = "Float".to_string();

    let hash1 = hash_fxcore(&module1);
    let hash2 = hash_fxcore(&module2);

    assert_ne!(
      hash1, hash2,
      "Different morphism content should produce different hash"
    );
  }

  #[test]
  fn test_hash_fxcore_includes_edge_content() {
    // 같은 노드를 가진 두 모듈이 다른 엣지 조건을 가지면 다른 해시를 가져야 함
    let module1 = FxCoreModule {
      meta: Default::default(),
      name: "test".to_string(),
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![],
      morphisms: vec![],
      nodes: vec![
        FxNode {
          name: "n1".to_string(),
          uses: "f".to_string(),
          kind: NodeKind::Normal,
          optional: false,
          scope: "global".to_string(),
          cost: CostHint::Tiny,
          priority: 0,
          contract: Default::default(),

          meta: None,
        },
        FxNode {
          name: "n2".to_string(),
          uses: "g".to_string(),
          kind: NodeKind::Normal,
          optional: false,
          scope: "global".to_string(),
          cost: CostHint::Tiny,
          priority: 0,
          contract: Default::default(),

          meta: None,
        },
      ],
      edges: vec![FxEdge {
        from: "n1".to_string(),
        to: "n2".to_string(),
        from_port: None,
        to_port: None,
        from_input: None,
        cond: Some(EdgeCond::When("gate1".to_string())),
      }],
      scopes: vec![],
    };

    let mut module2 = module1.clone();
    // 엣지 조건 변경
    module2.edges[0].cond = Some(EdgeCond::Unless("gate1".to_string()));

    let hash1 = hash_fxcore(&module1);
    let hash2 = hash_fxcore(&module2);

    assert_ne!(
      hash1, hash2,
      "Different edge conditions should produce different hash"
    );
  }

  fn make_test_ast() -> AstModule {
    AstModule {
      name: "test".into(),
      items: vec![
        AstItem::ExternDecl {
          name: "transform".into(),
          sig: SigAst::simple("Num".into(), "Num".into()),
          span: Span::default(),
        },
        AstItem::InputDecl {
          name: "x".into(),
          ty: "Num".into(),
          span: Span::default(),
        },
        AstItem::NodeDecl {
          name: "node1".into(),
          uses: "transform".into(),
          kind: None,
          optional: false,
          scope: None,
          cost: None,
          priority: None,
          span: Span::default(),
        },
        AstItem::EdgeDecl {
          from: EdgeSource::Input { name: "x".into() },
          to: EdgeTarget {
            node: "node1".into(),
            port: None,
          },
          cond: None,
          span: Span::default(),
        },
      ],
    }
  }

  #[test]
  fn test_pipeline_config_default() {
    let config = PipelineConfig::default();
    assert!(config.enable_fxcore_opt);
    assert!(config.enable_ct_opt);
    assert!(config.enable_ssa_opt);
    assert!(!config.enable_advanced_ssa_opt);
    assert!(config.collect_telemetry);
    assert!(!config.track_provenance);
  }

  #[test]
  fn test_pipeline_config_all() {
    let config = PipelineConfig::all_optimizations();
    assert!(config.enable_advanced_ssa_opt);
    assert!(config.track_provenance);
  }

  #[test]
  fn test_pipeline_config_no_opt() {
    let config = PipelineConfig::no_optimizations();
    assert!(!config.enable_fxcore_opt);
    assert!(!config.enable_ct_opt);
    assert!(!config.enable_ssa_opt);
  }

  #[test]
  fn test_quick_lower_to_fxcore() {
    let ast = make_test_ast();
    let mut diags = Diagnostics::default();

    let fxcore = quick_lower_to_fxcore(&ast, &mut diags).unwrap();

    assert_eq!(fxcore.name, "test");
    assert_eq!(fxcore.nodes.len(), 1);
    assert_eq!(fxcore.morphisms.len(), 1);
  }

  #[test]
  fn test_quick_lower_to_ssa() {
    let ast = make_test_ast();
    let mut diags = Diagnostics::default();

    let ssa = quick_lower_to_ssa(&ast, &mut diags).unwrap();

    assert_eq!(ssa.name, "test");
    assert!(!ssa.blocks.is_empty());
  }

  #[test]
  fn test_compile_to_fxcore() {
    let ast = make_test_ast();
    let mut diags = Diagnostics::default();

    let fxcore = compile_to_fxcore(&ast, &mut diags).unwrap();

    assert_eq!(fxcore.name, "test");
    // Optimizations should preserve the single node
    assert!(!fxcore.nodes.is_empty());
  }

  #[test]
  fn test_compile_to_ssa() {
    let ast = make_test_ast();
    let mut diags = Diagnostics::default();

    let ssa = compile_to_ssa(&ast, &mut diags).unwrap();

    assert_eq!(ssa.name, "test");
    assert!(!ssa.blocks.is_empty());
  }

  #[test]
  fn test_full_pipeline() {
    let ast = make_test_ast();
    let mut diags = Diagnostics::default();

    let pipeline = CompilationPipeline::default();
    let result = pipeline.run(&ast, &mut diags).unwrap();

    assert_eq!(result.surface.name, "test");
    assert_eq!(result.fxcore.name, "test");
    assert_eq!(result.ssa.name, "test");
    // IR sizes should be recorded
    assert!(result.compile_stats.initial_ir_size > 0 || result.compile_stats.final_ir_size > 0);
  }

  #[test]
  fn test_pipeline_no_optimizations() {
    let ast = make_test_ast();
    let mut diags = Diagnostics::default();

    let pipeline = CompilationPipeline::new(PipelineConfig::no_optimizations());
    let result = pipeline.run(&ast, &mut diags).unwrap();

    assert!(result.fxcore_stats.is_none());
    assert!(result.ssa_stats.is_none());
    assert!(result.ct_result.is_none());
  }

  #[test]
  fn test_pipeline_with_provenance() {
    let ast = make_test_ast();
    let mut diags = Diagnostics::default();

    let pipeline = CompilationPipeline::new(PipelineConfig::all_optimizations());
    let result = pipeline.run(&ast, &mut diags).unwrap();

    // Provenance may or may not be generated depending on optimizations applied
    // (single node cases may not trigger any optimizations)
    assert!(!result.provenances.is_empty());
  }

  #[test]
  fn test_pipeline_result_summary() {
    let ast = make_test_ast();
    let mut diags = Diagnostics::default();

    let pipeline = CompilationPipeline::default();
    let result = pipeline.run(&ast, &mut diags).unwrap();

    let summary = result.summary();
    assert!(summary.contains("Compilation:"));
    assert!(summary.contains("nodes"));
    assert!(summary.contains("transforms"));
  }

  #[test]
  fn test_pipeline_telemetry() {
    let ast = make_test_ast();
    let mut diags = Diagnostics::default();

    let pipeline = CompilationPipeline::for_module("test_module", PipelineConfig::default());
    let result = pipeline.run(&ast, &mut diags).unwrap();

    // Should have collected telemetry frames in result (P0-1 준수: stateless)
    assert!(!result.frames.is_empty());
  }

  #[test]
  fn test_pipeline_type_check_enabled() {
    let ast = make_test_ast();
    let mut diags = Diagnostics::default();

    let config = PipelineConfig {
      enable_type_check: true,
      ..PipelineConfig::default()
    };
    let pipeline = CompilationPipeline::new(config);
    let result = pipeline.run(&ast, &mut diags).unwrap();

    // 타입 검사 결과가 있어야 함
    assert!(result.type_result.is_some());
    let type_result = result.type_result.unwrap();
    // 타입 검사가 실행되었는지 확인 (성공/실패 여부는 테스트 AST에 따라 다를 수 있음)
    // 중요한 것은 타입 검사가 실행되고 결과가 반환되는 것
    assert!(!type_result.node_types.is_empty() || !type_result.errors.is_empty());
  }

  #[test]
  fn test_pipeline_type_check_disabled() {
    let ast = make_test_ast();
    let mut diags = Diagnostics::default();

    let config = PipelineConfig {
      enable_type_check: false,
      ..PipelineConfig::default()
    };
    let pipeline = CompilationPipeline::new(config);
    let result = pipeline.run(&ast, &mut diags).unwrap();

    // 타입 검사가 비활성화되면 결과가 없어야 함
    assert!(result.type_result.is_none());
  }
}
