//! pnix-core: Semantic compiler core
//!
//! pnix-core는 실행하지 않는다.
//! pnix-core는 의미를 해석하여 IR을 생성·검증하는 크로스플랫폼 컴파일러 코어다.

/// PNIX Core 버전: Cargo에서 가져온 크레이트 버전
pub const PNIX_CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[macro_use]
mod log_macros;

pub mod algorithm_synthesis;
pub mod ast;
pub mod build_ir;
pub mod cache;
pub mod code_transform;
pub mod codegen;
pub mod config;
pub mod contracts;
pub mod core;
pub mod ct;
pub mod debug;
pub mod diagnostics;
pub mod effects;
pub mod eval;
pub mod event_bus;
pub mod frp;
pub mod fx;
pub mod graphics;
pub mod hardware;
pub mod index;
pub mod io;
pub mod ir;
pub mod judgement_protocol;
pub mod judgment;
pub mod lang;
pub mod lexer;
// OWNER-LAW (2026-05-11):
//   `pub mod llm` is feature-gated behind `legacy-llm-names` (default OFF).
//   pnix substrate has no LLM seat (CLAUDE.md OWNER-LAW CONSTITUTION). The
//   legacy folder contains pure type definitions migrated from pnix-old's
//   `pnix_llm` (no LLM API calls). Default `cargo build` does NOT export
//   `pnix_core::llm::*`. Downstream callers that still need legacy names
//   must enable `--features legacy-llm-names` and migrate to
//   `language_meta` / `intent_meta` / `text_template` namespaces.
#[cfg(feature = "legacy-llm-names")]
#[deprecated(
  note = "OWNER-LAW: pnix has no LLM seat. Module is legacy pnix-old migration. Migrate to language_meta/intent_meta and disable the `legacy-llm-names` feature."
)]
pub mod llm;
// Internal-only mount for the legacy folder so this crate's own modules
// (nlp/mod.rs, runtime/machines_heart.rs, etc.) keep compiling while the
// rename refactor is in progress. This is NOT part of the public API —
// downstream crates must NOT use `pnix_core::__legacy_llm::*`. Use
// `language_meta` / `intent_meta` (forthcoming) instead.
#[cfg(not(feature = "legacy-llm-names"))]
#[path = "llm/mod.rs"]
#[doc(hidden)]
mod __legacy_llm;
#[cfg(not(feature = "legacy-llm-names"))]
#[doc(hidden)]
pub(crate) use __legacy_llm as llm;
pub mod mcp;
pub mod meta;
pub mod morphism;
pub mod nlp;
pub mod ontology;
pub mod ontology_demo;
pub mod parametric;
pub mod passes;
pub mod physics;
pub mod platform;
pub mod provenance;
pub mod puncheetah_consent_compile_cache;
pub mod render;
pub mod runtime;
pub mod runtime_plan;
pub mod spec;
pub mod ssa;
pub mod surface;
pub mod symbolic;
pub mod text_width;
pub mod tool_action;
pub mod types;
pub mod utils;
pub mod verify_action;

use build_ir::{Arch, Os};
use diagnostics::Diagnostics;
use thiserror::Error;

/// Meaning Error - 의미가 닫히지 못했을 때 발생
///
/// # Example
/// ```rust
/// use pnix_core::{MeaningError, diagnostics::Span};
/// let err = MeaningError::Parse("unexpected token".to_string(), None);
/// assert!(matches!(err, MeaningError::Parse(_, _)));
/// ```
#[derive(Debug, Error)]
pub enum MeaningError {
  /// 파싱 에러: 문법 분석 실패
  #[error("parse error: {0}")]
  Parse(String, Option<diagnostics::Span>),

  /// 해결되지 않은 심볼 에러: 정의되지 않은 심볼 참조
  #[error("unresolved symbol: {0}")]
  UnresolvedSymbol(String, Option<diagnostics::Span>),

  /// 타입 에러: 타입 검사 실패
  #[error("type error: {0}")]
  TypeError(String, Option<diagnostics::Span>),

  /// Lowering 에러: IR 변환 실패
  #[error("lowering error: {message}")]
  Lowering {
    /// 에러 메시지
    message: String,
    /// 소스 위치 (선택적)
    span: Option<diagnostics::Span>,
  },

  /// 계약 위반 에러: Meaning Closure 검증 실패
  #[error("contract violation: {0}")]
  ContractViolation(String, Option<diagnostics::Span>),

  /// 내부 에러: 컴파일러 내부 오류
  #[error("internal: {0}")]
  Internal(String, Option<diagnostics::Span>),
}

/// Meaning 결과 타입: Meaning 작업 결과
pub type MeaningResult<T> = Result<T, MeaningError>;

const REASON_CORE_PARSE_UNSUPPORTED: &str = "CORE_KERNEL_PARSE_UNSUPPORTED";
const REASON_CORE_LOWERING_UNSUPPORTED: &str = "CORE_KERNEL_LOWERING_UNSUPPORTED";

fn has_blocked_reason_prefix(message: &str) -> bool {
  message.contains("blocked(reason_code=")
}

fn looks_like_unsupported(message: &str) -> bool {
  message.to_ascii_lowercase().contains("unsupported")
}

fn blocked_reason_message(reason_code: &str, message: &str) -> String {
  format!("blocked(reason_code={reason_code}): {message}")
}

fn normalize_meaning_error(err: MeaningError) -> MeaningError {
  match err {
    MeaningError::Parse(message, span) => {
      if has_blocked_reason_prefix(&message) || !looks_like_unsupported(&message) {
        MeaningError::Parse(message, span)
      } else {
        MeaningError::Parse(
          blocked_reason_message(REASON_CORE_PARSE_UNSUPPORTED, &message),
          span,
        )
      }
    }
    MeaningError::Lowering { message, span } => {
      if has_blocked_reason_prefix(&message) || !looks_like_unsupported(&message) {
        MeaningError::Lowering { message, span }
      } else {
        MeaningError::Lowering {
          message: blocked_reason_message(REASON_CORE_LOWERING_UNSUPPORTED, &message),
          span,
        }
      }
    }
    other => other,
  }
}

/// 컴파일 옵션 (target 주입 포함)
///
/// ## 결정적 산출물 보장
///
/// `Default::default()`는 **고정 기본값** (Linux/x86_64)을 사용합니다.
/// 이는 동일한 소스 코드가 어떤 호스트에서 컴파일되든 동일한 산출물을 생성하도록 보장합니다.
///
/// ## 호스트 플랫폼 타겟팅
///
/// 호스트 플랫폼을 타겟으로 사용하려면 **executor**에서 host를 감지하여
/// `CompileOptions::with_target()`으로 주입해야 합니다.
/// pnix-core는 cfg!() 매크로를 사용하지 않아 host cfg에 의존하지 않습니다.
/// (pnix-core는 host-independent; 타겟 감지는 executor/호출자 책임)
#[derive(Debug, Clone)]
pub struct CompileOptions {
  /// 결정적 컴파일 여부
  pub deterministic: bool,
  /// 타겟 운영체제
  pub target_os: Os,
  /// 타겟 아키텍처
  pub target_arch: Arch,
  /// FxCore 그래프 크기 리소스 제한 (DoS 방지)
  pub resource_limits: contracts::ResourceLimits,
}

impl Default for CompileOptions {
  /// 결정적 기본값: Linux/x86_64
  ///
  /// 호스트 플랫폼을 사용하려면 executor에서 host를 감지하여
  /// `CompileOptions::with_target()`으로 주입하세요.
  fn default() -> Self {
    Self {
      deterministic: true,
      target_os: Os::Linux,      // 고정 기본값 (결정적)
      target_arch: Arch::X86_64, // 고정 기본값 (결정적)
      resource_limits: contracts::ResourceLimits::default(),
    }
  }
}

impl CompileOptions {
  /// 명시적 target 지정으로 옵션 생성
  ///
  /// 호스트 플랫폼을 타겟으로 사용하려면 executor에서
  /// `detect_host_os()`/`detect_host_arch()`를 호출하여 주입하세요.
  /// (pnix-core는 host-independent; 타겟 감지는 executor/호출자 책임)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn with_target(target_os: Os, target_arch: Arch) -> Self {
    Self {
      deterministic: true,
      target_os,
      target_arch,
      resource_limits: contracts::ResourceLimits::default(),
    }
  }
}

// NOTE: detect_host_os(), detect_host_arch(), for_host() 함수는
// cfg!() 매크로를 사용하여 host cfg에 의존하므로 pnix-core에서 제거됨.
// executor/호출자 계층에서 구현하여 사용.

/// 소스 단위: 컴파일할 소스 코드 단위
#[derive(Debug, Clone)]
pub struct SourceUnit {
  /// 소스 이름 (파일명 등)
  pub name: String,
  /// 소스 텍스트
  pub text: String,
}

/// Parse 출력: 파싱 결과
#[derive(Debug, Clone)]
pub struct ParseOutput {
  /// AST 모듈
  pub ast: ast::AstModule,
  /// 진단 정보
  pub diags: Diagnostics,
}

/// Check 출력: Meaning Closure 판정 결과
#[derive(Debug, Clone)]
pub struct CheckOutput {
  /// FxCore 모듈
  pub fxcore: core::FxCoreModule,
  /// 검증 리포트
  pub report: contracts::VerificationReport,
  /// 진단 정보
  pub diags: Diagnostics,
}

/// Compile 출력: 컴파일 결과
#[derive(Debug, Clone)]
pub struct CompileOutput {
  /// FxCore 모듈
  pub fxcore: core::FxCoreModule,
  /// SSA 모듈
  pub ssa: ssa::SsaModule,
  /// Build IR
  pub build_ir: build_ir::BuildIr,
  /// 생성된 아티팩트
  pub artifacts: codegen::Artifacts,
  /// 검증 리포트
  pub report: contracts::VerificationReport,
  /// 진단 정보
  pub diags: Diagnostics,
}

/// parse: 문법 검증 + AST 생성
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 파싱만, 값 계산 없음
pub fn parse(src: &SourceUnit) -> MeaningResult<ParseOutput> {
  debug!(source = %src.name, bytes = src.text.len(), "parse start");
  let mut diags = Diagnostics::default();
  let ast = ast::parse_module(&src.text, &src.name, &mut diags)
    .map_err(|e| normalize_meaning_error(MeaningError::Parse(e, None)))?;
  debug!(
    source = %src.name,
    diags = diags.items.len(),
    "parse done"
  );
  Ok(ParseOutput { ast, diags })
}

/// parse (Pnix module): Pnix data-only attrset → AST module
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 파싱만, 값 계산 없음
pub fn parse_pnix_module(src: &SourceUnit) -> MeaningResult<ParseOutput> {
  debug!(source = %src.name, bytes = src.text.len(), "parse pnix start");
  let diags = Diagnostics::default();
  let ast =
    lang::pnix::parse_pnix_module(&src.text, &src.name, Some(&src.name)).map_err(|err| {
      let span = match &err {
        lang::pnix::PnixError::Parse { span, .. } => Some(span.clone()),
        lang::pnix::PnixError::Lowering {
          span: Some(span), ..
        } => Some(span.clone()),
        _ => None,
      };
      normalize_meaning_error(MeaningError::Parse(err.to_string(), span))
    })?;
  debug!(
    source = %src.name,
    diags = diags.items.len(),
    "parse pnix done"
  );
  Ok(ParseOutput { ast, diags })
}

/// check: 의미 해석 + 계약 검증 (Meaning Closure 판정)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 분석만, 값 계산 없음
pub fn check(src: &SourceUnit, opts: &CompileOptions) -> MeaningResult<CheckOutput> {
  let ParseOutput { ast, diags } = parse(src)?;
  check_from_ast(ast, diags, opts)
}

/// check (Pnix module): Pnix data-only attrset → FxCore + contracts verify
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 분석만, 값 계산 없음
pub fn check_pnix_module(src: &SourceUnit, opts: &CompileOptions) -> MeaningResult<CheckOutput> {
  let ParseOutput { ast, diags } = parse_pnix_module(src)?;
  check_from_ast(ast, diags, opts)
}

fn check_from_ast(
  ast: ast::AstModule,
  mut diags: Diagnostics,
  opts: &CompileOptions,
) -> MeaningResult<CheckOutput> {
  debug!(module = %ast.name, items = ast.items.len(), "check start");
  // Spec 카탈로그 로드 (기본값)
  let spec = spec::Spec::with_defaults();

  // AST -> Surface
  let surface =
    passes::lowering::lower_to_surface(&ast, &mut diags).map_err(normalize_meaning_error)?;
  trace!(
    surface_inputs = surface.inputs.len(),
    surface_nodes = surface.nodes.len(),
    surface_edges = surface.edges.len(),
    "lowered to surface"
  );

  // Surface -> FxCore (spec 기반 검증)
  let fxcore = passes::lowering::lower_to_fxcore_with_spec(&surface, &diags, &spec)
    .map_err(normalize_meaning_error)?;
  trace!(
    fxcore_inputs = fxcore.inputs.len(),
    fxcore_nodes = fxcore.nodes.len(),
    fxcore_edges = fxcore.edges.len(),
    "lowered to fxcore"
  );

  // Resource limits (DoS guardrails)
  contracts::verify::verify_resource_limits(&fxcore, &opts.resource_limits)
    .map_err(normalize_meaning_error)?;

  // Contracts verify (S2-S5 Meaning Closure, spec 기반 검증)
  let report = contracts::verify::verify_fxcore_with_spec(&fxcore, &mut diags, &spec)
    .map_err(normalize_meaning_error)?;
  debug!(
    ok = report.ok,
    diags = diags.items.len(),
    "contracts verified"
  );

  if !report.ok {
    return Err(MeaningError::ContractViolation(
      format!(
        "meaning closure failed: S2={}, S3={}, S4={}, S5={}",
        report.closure.s2_reference_closure,
        report.closure.s3_contracts,
        report.closure.s4_dependency_closure,
        report.closure.s5_deterministic_artifacts,
      ),
      None,
    ));
  }

  Ok(CheckOutput {
    fxcore,
    report,
    diags,
  })
}

/// compile: IR/SSA/BuildIR 생성 + 텍스트 산출물 생성
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 구조 변환 및 텍스트 생성만, 파일 I/O 없음
pub fn compile(src: &SourceUnit, opts: &CompileOptions) -> MeaningResult<CompileOutput> {
  let CheckOutput {
    fxcore,
    report,
    diags,
  } = check(src, opts)?;
  compile_from_checked(fxcore, report, diags, opts)
}

/// compile (Pnix module): Pnix data-only attrset → dist artifacts
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 구조 변환 및 텍스트 생성만, 파일 I/O 없음
pub fn compile_pnix_module(
  src: &SourceUnit,
  opts: &CompileOptions,
) -> MeaningResult<CompileOutput> {
  let CheckOutput {
    fxcore,
    report,
    diags,
  } = check_pnix_module(src, opts)?;
  compile_from_checked(fxcore, report, diags, opts)
}

/// check (Pnix module AST): 이미 파싱된 AST로 검증
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 분석만, 값 계산 없음
pub fn check_pnix_module_ast(
  ast: ast::AstModule,
  opts: &CompileOptions,
) -> MeaningResult<CheckOutput> {
  let diags = Diagnostics::default();
  check_from_ast(ast, diags, opts)
}

/// compile (Pnix module AST): 이미 파싱된 AST로 컴파일
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 구조 변환 및 텍스트 생성만, 파일 I/O 없음
pub fn compile_pnix_module_ast(
  ast: ast::AstModule,
  opts: &CompileOptions,
) -> MeaningResult<CompileOutput> {
  let CheckOutput {
    fxcore,
    report,
    diags,
  } = check_pnix_module_ast(ast, opts)?;
  compile_from_checked(fxcore, report, diags, opts)
}

fn compile_from_checked(
  fxcore: core::FxCoreModule,
  report: contracts::VerificationReport,
  mut diags: Diagnostics,
  opts: &CompileOptions,
) -> MeaningResult<CompileOutput> {
  debug!(
    fxcore_nodes = fxcore.nodes.len(),
    fxcore_edges = fxcore.edges.len(),
    "compile start"
  );
  // FxCore optimization (헌법 P0-1 준수: 구조 변환만)
  let fxcore = passes::optimize::optimize_fxcore(&fxcore);
  trace!(
    fxcore_nodes = fxcore.nodes.len(),
    fxcore_edges = fxcore.edges.len(),
    "optimized fxcore"
  );

  // FxCore -> SSA
  let ssa = passes::lowering::lower_to_ssa(&fxcore, &diags).map_err(normalize_meaning_error)?;
  trace!(ssa_blocks = ssa.blocks.len(), "lowered to ssa");

  // SSA optimization (헌법 P0-1 준수: 구조 변환만)
  let ssa = passes::ssa_opt::optimize_ssa(&ssa);

  // Build IR 생성 (target은 opts에서 주입)
  let build_ir =
    build_ir::BuildIr::from_fxcore(&fxcore, opts.target_os.clone(), opts.target_arch.clone());
  trace!(
    target_os = ?opts.target_os,
    target_arch = ?opts.target_arch,
    "build ir created"
  );

  // Spec 카탈로그 로드 (기본값)
  let spec = spec::Spec::with_defaults();

  // Codegen은 텍스트 생성만 (툴체인 호출 금지, spec 포함)
  let artifacts = codegen::emit::emit_all_with_spec(&fxcore, &ssa, &build_ir, &mut diags, &spec)
    .map_err(normalize_meaning_error)?;
  debug!(
    replay_hash = %artifacts.replay_hash,
    diags = diags.items.len(),
    "artifacts generated"
  );

  Ok(CompileOutput {
    fxcore,
    ssa,
    build_ir,
    artifacts,
    report,
    diags,
  })
}

/// inspect: 실행 없이 구조를 JSON으로 조회
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O 없음
pub fn inspect_ir_json(src: &SourceUnit, opts: &CompileOptions) -> MeaningResult<String> {
  let out = compile(src, opts)?;
  Ok(out.artifacts.fxcore_json)
}

/// inspect_all: 전체 IR을 JSON으로 조회 (replay_hash 포함)
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O 없음
pub fn inspect_all_json(src: &SourceUnit, opts: &CompileOptions) -> MeaningResult<String> {
  let out = compile(src, opts)?;
  Ok(format!(
    "{{\n  \"replay_hash\": \"{}\",\n  \"fxcore\": {},\n  \"ssa\": {},\n  \"build_ir\": {}\n}}",
    out.artifacts.replay_hash,
    out.artifacts.fxcore_json,
    out.artifacts.ssa_json,
    out.artifacts.build_ir_json
  ))
}

// emit() 함수 제거됨 - fs I/O는 헌법 P0-1 위반
// dist 디렉토리 내보내기는 executor에서 구현
// 상세: `docs/executor.md` + `crates/pnix-executor-graph/src/cli.rs`

/// generate_code: FxCore graph를 타겟 언어로 생성
///
/// 헌법 C1 준수: 텍스트 생성만, 컴파일러/링커 호출 금지
pub fn generate_code(
  fxcore: &core::FxCoreModule,
  lang: codegen::TargetLanguage,
) -> Result<codegen::GeneratedCode, codegen::CodeGenError> {
  codegen::generate(fxcore, lang)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_compile_options_default_is_deterministic() {
    // Default는 항상 Linux/x86_64 (결정적 기본값)
    let opts = CompileOptions::default();
    assert!(matches!(opts.target_os, Os::Linux));
    assert!(matches!(opts.target_arch, Arch::X86_64));
    assert!(opts.deterministic);
  }

  #[test]
  fn test_compile_options_with_target() {
    let opts = CompileOptions::with_target(Os::Darwin, Arch::Aarch64);
    assert!(matches!(opts.target_os, Os::Darwin));
    assert!(matches!(opts.target_arch, Arch::Aarch64));
  }

  #[test]
  fn normalize_parse_unsupported_to_blocked_reason_code() {
    let err = normalize_meaning_error(MeaningError::Parse(
      "Unsupported syntax: string interpolation".to_string(),
      None,
    ));
    match err {
      MeaningError::Parse(message, _) => {
        assert!(
          message.starts_with("blocked(reason_code=CORE_KERNEL_PARSE_UNSUPPORTED):"),
          "unexpected message: {message}"
        );
      }
      other => panic!("expected parse error, got {other:?}"),
    }
  }

  #[test]
  fn normalize_lowering_unsupported_to_blocked_reason_code() {
    let err = normalize_meaning_error(MeaningError::Lowering {
      message: "unsupported lowering op".to_string(),
      span: None,
    });
    match err {
      MeaningError::Lowering { message, .. } => {
        assert!(
          message.starts_with("blocked(reason_code=CORE_KERNEL_LOWERING_UNSUPPORTED):"),
          "unexpected message: {message}"
        );
      }
      other => panic!("expected lowering error, got {other:?}"),
    }
  }

  #[test]
  fn normalize_preserves_existing_blocked_message() {
    let original = "blocked(reason_code=EVAL_TARGET_INVALID): bad target".to_string();
    let err = normalize_meaning_error(MeaningError::Parse(original.clone(), None));
    match err {
      MeaningError::Parse(message, _) => assert_eq!(message, original),
      other => panic!("expected parse error, got {other:?}"),
    }
  }

  // NOTE: test_compile_options_for_host, test_detect_host_functions 테스트는
  // cfg!() 매크로 의존 함수들이 제거되어 삭제됨.
  // executor에서 host 타겟 주입 테스트를 담당.
}
