//! 언어별 AST 및 에러 타입 정의
//!
//! pnix-old의 lang_python, lang_pnix, lang_js에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의 및 파싱/변환만, 실행 로직 제외
//!
//! - parsing은 "값 계산"이 아니라 구조 생성(frontend)이므로 core에 둘 수 있음
//! - evaluation/IO/side-effect는 runtime/executor에서만 허용

// Clojure (타입 정의, lowering만 - 시간/스레드 없음)
// 스레드/시간 관련 코드는 pnix-runtime-legacy/src/clojure_thread.rs에 위치
pub mod clojure;
pub mod clojure_error;

pub mod js;
pub mod ko;
pub mod layer;
pub mod module_resolver;
pub mod pnix;
pub mod python;
pub mod python_types;

// Clojure exports
pub use clojure::{lower_clj_to_fx_core, parse_clj_expr, UnifiedExpr as ClojureUnifiedExpr};
pub use clojure_error::ClojureError;

pub use js::{lower_to_fx_core as lower_js_to_fx_core, JsError, UnifiedExpr as JsUnifiedExpr};
pub use ko::{
  analyze_korean_text, decompose_hangul_syllable, HangulSyllable, KoreanParticleHit,
  KoreanParticleKind, KoreanSentenceMood, KoreanTextAnalysis,
};
pub use layer::{
  build_capability_map, capability_map_path_for_package, load_laws_from_layer_pnix,
  load_layer_grammar_for_package_with, load_layer_laws_for_package_with,
  load_layer_meaning_for_package_with, load_layer_schema_pnix,
  load_layer_verification_report_for_package_with, load_verification_report_pnix,
  render_capability_map, update_capability_map_from_schema_with,
  write_capability_map_for_package_configured_with, write_capability_map_for_package_with,
  write_capability_map_with, LayerCapabilityMap, LayerEngine, LayerError, LayerLawSet,
  LayerPackage, LayerParseError, LayerPipeline, LayerPrepared, LayerRegistry, LayerReport,
  LayerVerificationReport,
};
pub use module_resolver::{ModuleResolution, ModuleResolutionError, ModuleResolver};
pub use pnix::{
  lower_to_fx_core as lower_pnix_to_fx_core, parse_expr as parse_pnix_expr,
  parse_let_bindings as parse_pnix_let_bindings, parse_minimal_expr as parse_pnix_minimal_expr,
  parse_pnix_module, validate_minimal_expr as validate_pnix_minimal_expr, PnixAttrItem, PnixError,
  PnixExpr, PnixLetBinding, PnixListPattern, PnixMatchAttrField, PnixMatchListPattern,
  PnixParamPattern, PnixPatternField, UnifiedExpr as PnixUnifiedExpr,
};
pub use python::convert::{convert_python_to_fx_core, PythonConvertError};
pub use python::unified::{python_node_to_unified, PythonUnifiedError};
pub use python_types::{BinOperator, PythonConstant, PythonNode, UnaryOperator};
