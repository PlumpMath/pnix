//! Parametric synthesis pipeline (data-only IR + deterministic transforms).
//!
//! - IR: constraint spec from WYSIWYG UI
//! - Synthesis: strict, deterministic solve_for (no heuristics)
//! - Emission: UnifiedExpr / FxSurfaceExpr / Pnix string

pub mod codec;
pub mod const_eval;
pub mod emit;
pub mod error;
pub mod ir;
pub mod pipeline;
pub mod policy;
pub mod symbolic_bridge;
pub mod synth;
pub mod validate;

// Reserved prefix for internal symbolic signal variables.
const SIGNAL_SYMBOL_PREFIX: &str = "__pnix_signal__";

pub use codec::{
  canonicalize_policy, canonicalize_spec, policy_from_json_str, policy_to_json_string,
  spec_from_json_str, spec_to_json_string,
};
pub use emit::{
  emit_constraint_pnix, emit_constraint_surface, emit_constraint_unified, emit_fx_surface,
  emit_pnix_string, emit_synthesis_form_pnix, emit_synthesis_form_surface,
  emit_synthesis_form_unified, emit_synthesis_result_pnix, emit_synthesis_result_surface,
  emit_synthesis_result_unified, emit_unified,
};
pub use error::{ParametricError, ParametricResult};
pub use ir::{
  Constraint, ConstraintExpr, ContextMode, Fixture, ParamExpr, ParamExprKind, ParamRole,
  ParamUnaryOp, ParamValue, ParamVar, ParametricSpec, ProvenanceTag, SignalRef, TargetVar, Unit,
  UnitScale,
};
pub use pipeline::{synthesize_to_pnix, synthesize_to_surface, synthesize_to_unified};
pub use policy::{CallPolicy, CallSpec};
pub use symbolic_bridge::{constraint_to_symexprs, param_expr_to_symexpr};
pub use synth::{synthesize, synthesize_with_policy, SynthesisForm, SynthesisResult};
pub use validate::{validate_spec, validate_spec_with_policy};
