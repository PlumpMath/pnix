//! Category Theory AST and Optimization
//!
//! pnix-old의 ctast.rs를 pnix-new 패러다임(그래프 기반)에 맞게 적응.
//!
//! ## 모듈 구조
//!
//! - `ctast`: Category Theory AST 정의
//! - `convert`: FxCoreModule ↔ CTAST ↔ CTDiagram 변환
//! - `optimize`: CT 법칙 기반 최적화 패스
//! - `tags`: CT 메타데이터 태그 (CtCategory, CtTag)
//!
//! ## pnix-old vs pnix-new
//!
//! | 측면 | pnix-old | pnix-new |
//! |-----|----------|----------|
//! | IR | FxCoreExpr (트리) | FxCoreModule (그래프) |
//! | Op | UnifiedMeaningOp (200+) | 단순화된 CT 원시형 |
//! | 법칙 검증 | CTAST 재귀 | 그래프 패턴 매칭 |

pub mod convert;
pub mod ctast;
pub mod optimize;
pub mod tags;

pub use convert::{
  ct_diagram_to_ctast, ctast_to_ct_diagram, ctast_to_fxcore_module, fxcore_to_ctast_with_stats,
  ConversionStats,
};
pub use ctast::{CTLawPreservation, CTLit, CTMorphismOp, CTNode, CTType, CTAST};
pub use optimize::{CTOptPass, CTOptResult, CTOptimizer};
pub use tags::{CtCategory, CtContext, CtTag};
