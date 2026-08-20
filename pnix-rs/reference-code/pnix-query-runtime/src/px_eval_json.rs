//! C6 read-model bridge for `.px` query/runtime documents.
//!
//! This module intentionally wraps `pnix_eval` for loader and fixture
//! surfaces owned by pnix-query-runtime. It is not canonical mirror
//! primitive proof. Any user-visible substrate decision reached from
//! these documents should be routed through a registered mirror lens and
//! pnixc-meta `applyMirrorPlateWithLensDispatch`.

use anyhow::Result;
use std::path::Path;

/// pnix-eval library-backed JSON emitter.
///
/// `pnixc --expr --emit eval` 를 대체하는 얇은 helper layer.
pub fn eval_px_source_to_json(source: &str) -> Result<String> {
  let value = pnix_eval::eval_pnix_expr(source)?;
  Ok(value.to_json())
}

/// `.px` file 을 `pnix_eval` 로 직접 평가하고 JSON text 를 반환한다.
pub fn eval_px_file_to_json(path: &Path) -> Result<String> {
  let value = pnix_eval::eval_pnix_file(path)?;
  Ok(value.to_json())
}
