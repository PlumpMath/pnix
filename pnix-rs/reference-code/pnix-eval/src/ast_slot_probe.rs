//! A-0 probe (AST inline-cache slot lane, 2026-06-12) — MEASUREMENT ONLY.
//!
//! Design owner: `project-wiki/maps/host-ast-inline-cache-design-map.md`.
//! Measures the per-visit ceiling for the builtins-Select fast path:
//! today every visit pays `fast_builtin_attr_value(attr)` = FxHash of a
//! short attr string + map probe + `Arc<str>` refcount bump. A per-node
//! slot would replace that with one `OnceLock` load + `dyn Any`
//! downcast + the same refcount bump. This probe compares those two
//! shapes in isolation (substrate-wide does ~2.6M such visits).
//!
//! Run with:
//!   cargo test -p pnix-eval --release --lib ast_slot_probe -- --ignored --nocapture

#[cfg(test)]
mod tests {
  use crate::value::Value;
  use std::any::Any;
  use std::sync::Arc;
  use std::sync::OnceLock;

  /// Mirror of the production lookup shape: hash a short &str, probe a
  /// static HashMap, clone the Arc payload out.
  fn map_lookup(
    map: &std::collections::HashMap<&'static str, Arc<str>>,
    attr: &str,
  ) -> Option<Value> {
    map.get(attr).map(|a| Value::BuiltinPartial {
      name: a.clone(),
      args: Vec::new(),
    })
  }

  /// Mirror of the slot shape: OnceLock load + Any downcast + clone.
  fn slot_lookup(slot: &OnceLock<Box<dyn Any + Send + Sync>>) -> Option<Value> {
    slot.get().and_then(|b| b.downcast_ref::<Value>()).cloned()
  }

  /// Shape C: the slot stores only a `u32` id; the payload lives in a
  /// process-global table indexed by it. No `dyn Any` pointer chase.
  fn id_slot_lookup(slot: &OnceLock<u32>, table: &[Value]) -> Option<Value> {
    slot.get().map(|&i| table[i as usize].clone())
  }

  #[test]
  #[ignore = "ast slot probe — run explicitly with --ignored --nocapture"]
  fn ast_slot_per_visit_ceiling_probe() {
    let attrs: Vec<&'static str> = vec![
      "map",
      "filter",
      "foldl'",
      "elemAt",
      "length",
      "attrNames",
      "getAttr",
      "hasAttr",
    ];
    let mut map = std::collections::HashMap::default();
    for &a in &attrs {
      map.insert(a, Arc::<str>::from(a));
    }
    let slots: Vec<OnceLock<Box<dyn Any + Send + Sync>>> = attrs
      .iter()
      .map(|&a| {
        let s: OnceLock<Box<dyn Any + Send + Sync>> = OnceLock::new();
        let _ = s.set(Box::new(Value::BuiltinPartial {
          name: Arc::<str>::from(a),
          args: Vec::new(),
        }));
        s
      })
      .collect();

    const ITERS: usize = 4_000_000;
    const ROUNDS: usize = 7;
    let mut map_times = Vec::new();
    let mut slot_times = Vec::new();
    for _ in 0..ROUNDS {
      let t0 = std::time::Instant::now();
      let mut sink = 0usize;
      for i in 0..ITERS {
        let v = map_lookup(&map, attrs[i & 7]).unwrap();
        if let Value::BuiltinPartial { name, .. } = &v {
          sink ^= name.len();
        }
      }
      std::hint::black_box(sink);
      map_times.push(t0.elapsed());

      let t1 = std::time::Instant::now();
      let mut sink = 0usize;
      for i in 0..ITERS {
        let v = slot_lookup(&slots[i & 7]).unwrap();
        if let Value::BuiltinPartial { name, .. } = &v {
          sink ^= name.len();
        }
      }
      std::hint::black_box(sink);
      slot_times.push(t1.elapsed());
    }
    let table: Vec<Value> = attrs
      .iter()
      .map(|&a| Value::BuiltinPartial {
        name: Arc::<str>::from(a),
        args: Vec::new(),
      })
      .collect();
    let id_slots: Vec<OnceLock<u32>> = (0..attrs.len() as u32)
      .map(|i| {
        let s: OnceLock<u32> = OnceLock::new();
        let _ = s.set(i);
        s
      })
      .collect();
    let mut id_times = Vec::new();
    for _ in 0..ROUNDS {
      let t2 = std::time::Instant::now();
      let mut sink = 0usize;
      for i in 0..ITERS {
        let v = id_slot_lookup(&id_slots[i & 7], &table).unwrap();
        if let Value::BuiltinPartial { name, .. } = &v {
          sink ^= name.len();
        }
      }
      std::hint::black_box(sink);
      id_times.push(t2.elapsed());
    }
    id_times.sort();
    let idm = id_times[ROUNDS / 2];
    map_times.sort();
    slot_times.sort();
    let m = map_times[ROUNDS / 2];
    let s = slot_times[ROUNDS / 2];
    eprintln!(
      "ast-slot probe: map-lookup={:?} slot-load={:?} ratio={:.3}x ({} iters, median of {} rounds; A-0 gate: <1.3x => builtins-Select slot alone not worth the variant surgery)",
      m,
      s,
      m.as_secs_f64() / s.as_secs_f64(),
      ITERS,
      ROUNDS
    );
    eprintln!(
      "ast-slot probe C: id-slot+table={:?} ratio-vs-map={:.3}x (u32 slot + Vec index, no dyn Any)",
      idm,
      map_times[ROUNDS / 2].as_secs_f64() / idm.as_secs_f64()
    );
  }
}
