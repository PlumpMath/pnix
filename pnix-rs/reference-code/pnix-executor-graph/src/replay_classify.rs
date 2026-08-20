use crate::builtins;
use crate::model::FxNodeMeta;

pub fn classify_uses(
  uses: &str,
  node_meta: Option<&FxNodeMeta>,
  meta_override: Option<&FxNodeMeta>,
) -> (bool, Option<String>) {
  let meta = meta_override.or(node_meta);
  if let Some(meta) = meta {
    if let Some(replay_class) = meta.replay_class.as_ref() {
      return (true, Some(replay_class.clone()));
    }
    if meta.nondet.unwrap_or(false) {
      return (true, None);
    }
  }

  // Backend RPC morphisms are external-world effects by default.
  // Use backend_of() so aliases like py/cljs are classified consistently.
  let backend = crate::rpc::backend_of(uses);
  if !backend.is_empty() && backend != crate::builtins::BUILTIN_BACKEND && backend != "nix" {
    return (true, Some("external_world/backend".to_string()));
  }

  let resolved = builtins::resolve_builtin_name(uses)
    .map(|s| s.into_owned())
    .unwrap_or_else(|| uses.to_string());
  let observe_like = matches!(
    resolved.as_str(),
    "processObserveSample"
      | "processObserveSampleById"
      | "process.observe.sample"
      | "process.observe.sample.by_id"
      | "builtins.process.observe.sample"
      | "builtins.process.observe.sample.by_id"
  );
  if observe_like {
    return (true, Some("external_world/process_observe".to_string()));
  }

  let process_like = matches!(
    resolved.as_str(),
    "processSpawn"
      | "processEnsure"
      | "processStatus"
      | "processSignal"
      | "processWait"
      | "processLogsTail"
      | "processTerminate"
      | "process.spawn"
      | "process.ensure"
      | "process.status"
      | "process.signal"
      | "process.wait"
      | "process.logs.tail"
      | "process.terminate"
      | "builtins.process.spawn"
      | "builtins.process.ensure"
      | "builtins.process.status"
      | "builtins.process.signal"
      | "builtins.process.wait"
      | "builtins.process.logs.tail"
      | "builtins.process.terminate"
  );
  if process_like {
    return (true, Some("external_world/process".to_string()));
  }

  let runtime_call_like = matches!(
    resolved.as_str(),
    "runtimeCall" | "runtime.call" | "builtins.runtime.call"
  );
  if runtime_call_like {
    return (true, Some("external_world/backend".to_string()));
  }

  (false, None)
}

#[cfg(test)]
mod tests {
  use super::classify_uses;

  #[test]
  fn classify_backend_aliases_as_external_world_backend() {
    let (nondet_py, class_py) = classify_uses("py.numpy.add", None, None);
    assert!(nondet_py);
    assert_eq!(class_py.as_deref(), Some("external_world/backend"));

    let (nondet_cljs, class_cljs) = classify_uses("cljs.identity", None, None);
    assert!(nondet_cljs);
    assert_eq!(class_cljs.as_deref(), Some("external_world/backend"));

    let (nondet_deno, class_deno) = classify_uses("deno.render", None, None);
    assert!(nondet_deno);
    assert_eq!(class_deno.as_deref(), Some("external_world/backend"));
  }

  #[test]
  fn classify_runtime_call_as_external_world_backend() {
    let (nondet, class) = classify_uses("runtime.call", None, None);
    assert!(nondet);
    assert_eq!(class.as_deref(), Some("external_world/backend"));
  }
}
