//! Clojure/Python 브릿지 예제: Clojure 및 Python 표현식을 FxCore로 변환하여 커널에서 실행
//!
//! Clojure와 Python 표현식을 각각 UnifiedExpr로 파싱한 후 FxCore로 변환하고,
//! 커널 스케줄러를 통해 실행하는 예제입니다.

use pnix_core::lang::python_types::{BinOperator, PythonConstant, PythonNode};
use pnix_core::lang::{
  lower_clj_to_fx_core, lower_pnix_to_fx_core, parse_clj_expr, python_node_to_unified,
};
use pnix_core::render::to_plain;
use pnix_runtime_kernel::{EffectEvent, EffectZone, Kernel, KernelConfig};

/// 메인 함수: Clojure/Python 브릿지 데모 실행
fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Clojure subset -> UnifiedExpr -> FxCore
  let clj_source = "(+ param/system-time 1)";
  let clj_unified = parse_clj_expr(clj_source)?;
  let clj_fx = lower_clj_to_fx_core(&clj_unified)?;
  let clj_render = to_plain(&clj_fx);

  // Python subset -> UnifiedExpr -> FxCore
  let py_node = PythonNode::BinOp {
    left: Box::new(PythonNode::Constant {
      value: PythonConstant::Int(40),
    }),
    op: BinOperator::Add,
    right: Box::new(PythonNode::Constant {
      value: PythonConstant::Int(2),
    }),
  };
  let py_unified = python_node_to_unified(&py_node)?;
  let py_fx = lower_pnix_to_fx_core(&py_unified)?;
  let py_render = to_plain(&py_fx);

  // Feed both through the kernel scheduler.
  let mut kernel = Kernel::new(KernelConfig::deterministic_defaults())?;

  kernel.schedule("clj", move |kernel| {
    kernel.emit_effect(EffectEvent::new(EffectZone::Pure, "clj", clj_render));
    kernel.schedule("clj-followup", move |kernel| {
      kernel.emit_effect(EffectEvent::new(EffectZone::Pure, "clj-followup", "ok"));
      Ok(())
    });
    kernel.tick();
    Ok(())
  });

  kernel.schedule("py", move |kernel| {
    kernel.emit_effect(EffectEvent::new(EffectZone::Pure, "py", py_render));
    kernel.schedule("py-followup", move |kernel| {
      kernel.emit_effect(EffectEvent::new(EffectZone::Pure, "py-followup", "ok"));
      Ok(())
    });
    kernel.tick();
    Ok(())
  });

  kernel.run_all()?;

  for event in kernel.effects().snapshot() {
    println!("[{:?}] {} -> {}", event.zone, event.name, event.detail);
  }

  Ok(())
}
