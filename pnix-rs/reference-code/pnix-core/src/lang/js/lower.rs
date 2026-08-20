//! UnifiedExpr → FxCoreExpr 변환 (JavaScript/TypeScript 전용)
//!
//! pnix-old의 lang_js/unified.rs에서 마이그레이션.
//!
//! JavaScript/TypeScript에서 파싱된 UnifiedExpr를 FxCoreExpr로 변환
//! JS 특화 함수 매핑, 연산자 처리, 최적화를 포함
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 구조 변환만, 값 계산 없음

use super::error::JsError;
use crate::fx::core_expr::FxCoreExpr;
use crate::lang::pnix::lower::lower_to_fx_core_with_mode as pnix_lower_with_mode;
use crate::lang::pnix::unified::ExecutionMode;
use crate::lang::pnix::UnifiedExpr;

/// UnifiedExpr를 FxCoreExpr로 lowering (JS-specific)
///
/// JS-specific lowering은 다음을 처리합니다:
/// - Math.* 함수 매핑 (Math.sin, Math.cos, Math.floor 등)
/// - JS 특화 연산자 (===, !==, typeof)
/// - 문자열 템플릿 최적화
/// - 배열 메서드 매핑 (map, filter, reduce)
///
/// Y08a-9: resolve_signals 파이프라인 적용
/// 기본적으로 Realtime 모드를 사용 (JS는 런타임 환경)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn lower_to_fx_core(
  expr: &UnifiedExpr,
  mode: ExecutionMode,
  allowlist: &[&str],
) -> Result<FxCoreExpr, JsError> {
  match expr {
    // JS-specific function applications
    UnifiedExpr::Apply { func, args } => lower_js_apply(func, args, mode, allowlist),

    // Delegate other expressions to lang_pnix's generic lowering
    _ => pnix_lower_with_mode(expr, mode, allowlist)
      .map_err(|e| JsError::Lowering(format!("Lowering error: {:?}", e))),
  }
}

/// 기존 API 호환성을 위한 래퍼 (Realtime 모드 사용)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn lower_to_fx_core_default(expr: &UnifiedExpr) -> Result<FxCoreExpr, JsError> {
  lower_to_fx_core(expr, ExecutionMode::Realtime, &[])
}

/// JS-specific 함수 적용 lowering
fn lower_js_apply(
  func: &str,
  args: &[UnifiedExpr],
  mode: ExecutionMode,
  allowlist: &[&str],
) -> Result<FxCoreExpr, JsError> {
  // JS Math 객체 함수들
  match func {
    // === Math.* 함수들 ===
    "Math.sin" | "math.sin" if args.len() == 1 => {
      let arg = lower_to_fx_core(&args[0], mode, allowlist)?;
      Ok(FxCoreExpr::sin(arg))
    }
    "Math.cos" | "math.cos" if args.len() == 1 => {
      let arg = lower_to_fx_core(&args[0], mode, allowlist)?;
      Ok(FxCoreExpr::cos(arg))
    }
    "Math.tan" | "math.tan" if args.len() == 1 => {
      let arg = lower_to_fx_core(&args[0], mode, allowlist)?;
      // tan = sin / cos
      let sin_arg = FxCoreExpr::sin(arg.clone());
      let cos_arg = FxCoreExpr::cos(arg);
      Ok(FxCoreExpr::div(sin_arg, cos_arg))
    }
    "Math.exp" | "math.exp" if args.len() == 1 => {
      let arg = lower_to_fx_core(&args[0], mode, allowlist)?;
      Ok(FxCoreExpr::exp(arg))
    }
    "Math.log" | "math.log" if args.len() == 1 => {
      let arg = lower_to_fx_core(&args[0], mode, allowlist)?;
      Ok(FxCoreExpr::ln(arg))
    }
    "Math.floor" | "math.floor" if args.len() == 1 => {
      let arg = lower_to_fx_core(&args[0], mode, allowlist)?;
      Ok(FxCoreExpr::floor(arg))
    }
    "Math.ceil" | "math.ceil" if args.len() == 1 => {
      let arg = lower_to_fx_core(&args[0], mode, allowlist)?;
      Ok(FxCoreExpr::ceil(arg))
    }
    "Math.abs" | "math.abs" if args.len() == 1 => {
      let arg = lower_to_fx_core(&args[0], mode, allowlist)?;
      Ok(FxCoreExpr::abs(arg))
    }
    "Math.sqrt" | "math.sqrt" if args.len() == 1 => {
      let arg = lower_to_fx_core(&args[0], mode, allowlist)?;
      Ok(FxCoreExpr::sqrt(arg))
    }
    "Math.round" | "math.round" if args.len() == 1 => {
      // round(x) = floor(x + 0.5)
      let arg = lower_to_fx_core(&args[0], mode, allowlist)?;
      let half = FxCoreExpr::float(0.5);
      Ok(FxCoreExpr::floor(FxCoreExpr::add(arg, half)))
    }
    "Math.min" | "math.min" if args.len() == 2 => {
      let a = lower_to_fx_core(&args[0], mode, allowlist)?;
      let b = lower_to_fx_core(&args[1], mode, allowlist)?;
      Ok(FxCoreExpr::if_then_else(
        FxCoreExpr::lt(a.clone(), b.clone()),
        a,
        b,
      ))
    }
    "Math.max" | "math.max" if args.len() == 2 => {
      let a = lower_to_fx_core(&args[0], mode, allowlist)?;
      let b = lower_to_fx_core(&args[1], mode, allowlist)?;
      Ok(FxCoreExpr::if_then_else(
        FxCoreExpr::gt(a.clone(), b.clone()),
        a,
        b,
      ))
    }
    "Math.pow" | "math.pow" if args.len() == 2 => {
      let base = lower_to_fx_core(&args[0], mode, allowlist)?;
      let exp = lower_to_fx_core(&args[1], mode, allowlist)?;
      Ok(FxCoreExpr::pow(base, exp))
    }

    // === 기타: lang_pnix로 위임 ===
    _ => {
      // 알 수 없는 함수는 lang_pnix의 일반 lowering으로 위임
      let unified_apply = UnifiedExpr::Apply {
        func: func.to_string(),
        args: args.to_vec(),
      };
      pnix_lower_with_mode(&unified_apply, mode, allowlist)
        .map_err(|e| JsError::Lowering(format!("Lowering error: {:?}", e)))
    }
  }
}
