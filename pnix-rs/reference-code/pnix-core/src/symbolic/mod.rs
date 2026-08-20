//! Symbolic Math AST
//!
//! 심볼릭 수학 표현을 위한 AST 타입들.
//! pnix-old의 symbolic_core에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 AST 구조 정의, 값 계산 없음
//!
//! ## 사용 목적
//!
//! - 심볼릭 수식 표현 (컴퓨터 대수)
//! - 미분/적분 구조
//! - 텐서 연산 구조
//! - LaTeX 렌더링용 AST

pub mod api;
pub mod bridge;
pub mod ct;
pub mod engine;
pub mod expr;
pub mod ipc;
pub mod ipc_server;
pub mod latex;
pub mod lowering;
pub mod mcp_server;
pub mod nrepl_client;
pub mod number;
pub mod passes;
pub mod provenance;
pub mod rewrite;
pub mod serialize;
pub mod zone;

pub use api::{
  ApiError, CtContextSpec, DiffRequest, DiffResponse, ExpandRequest, ExpandResponse,
  NormalizeRequest, NormalizeResponse, ResourceLimitError, ResourceLimits, SimParams,
  SimplifyRequest, SimplifyResponse, SimulateRequest, SimulateResponse, SubstituteRequest,
  SubstituteResponse, TensorContextSpec, TensorContractRequest, TensorContractResponse,
};
pub use bridge::{fxcore_to_symexpr, symexpr_to_fxcore, SymbolicBridgeError};
pub use ct::{
  analyze_indices, check_ct, contains_tensor_indices, free_indices_of_expr, CtCategory, CtContext,
  CtError, CtTag, IndexUsage,
};
pub use engine::SymbolicEngine;
pub use expr::{IndexPosition, Span, SymExpr, SymKind, Symmetry, TensorIndex, TensorSymbol, Zone};
pub use ipc::{IpcOp, IpcRequest, IpcResponse, IpcStatus};
pub use ipc_server::{IpcServer, SessionState};
pub use latex::to_latex;
pub use lowering::{
  contains_tensor, lower_to_ir, BinOpKind, IrInst, IrLowering, IrProgram, LowerError, UnaryOpKind,
};
pub use mcp_server::McpServer;
pub use number::NumberValue;
pub use passes::{
  apply_identities, apply_identities_simple, differentiate, eval_numeric, find_contracted_indices,
  normalize, tensor_normalize,
};
pub use provenance::{
  analyze_differentiability, estimate_ir_cost, select_adaptive_tier, AdaptiveSimplifyResult,
  ApproxPoint, BudgetTier, CachedSubexprInfo, CtValidationResult, DifferentiabilityAnalysis,
  DifferentiabilityReason, FrpCacheStatsRecord, NonDifferentiableOp, ProvenanceBuilder,
  SimplifyStats, SymbolicProvenance, TemporalDecision,
};
pub use serialize::{
  from_json, from_json_value, full_type_summary, hover_info, to_json, to_json_pretty,
  to_json_value, type_summary, SerializeError,
};
