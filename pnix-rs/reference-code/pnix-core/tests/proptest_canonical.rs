//! Proptest Canonical 테스트: 속성 기반 테스트를 사용한 정규화 기능 테스트
//!
//! Proptest를 사용하여 다양한 입력에 대한 정규화 기능을 검증합니다.

#![cfg(feature = "proptest")]

use std::collections::HashSet;

use proptest::prelude::*;
use serde_json::{Map, Number, Value};

use pnix_core::codegen::normalize::canonicalize;

fn proptest_config() -> ProptestConfig {
  ProptestConfig {
    failure_persistence: None,
    ..ProptestConfig::default()
  }
}

fn json_value() -> impl Strategy<Value = Value> {
  let leaf = prop_oneof![
    Just(Value::Null),
    any::<bool>().prop_map(Value::Bool),
    any::<i64>().prop_map(|value| Value::Number(value.into())),
    any::<f64>().prop_filter_map("finite f64", |value| {
      Number::from_f64(value).map(Value::Number)
    }),
    "[a-zA-Z0-9_]{0,16}".prop_map(Value::String),
  ];

  leaf.prop_recursive(4, 64, 8, |inner| {
    let array = prop::collection::vec(inner.clone(), 0..8).prop_map(Value::Array);
    let object = prop::collection::vec(("[a-zA-Z0-9_]{1,12}", inner), 0..8)
      .prop_filter("unique keys", |entries| {
        let mut seen = HashSet::new();
        entries.iter().all(|(key, _)| seen.insert(key.clone()))
      })
      .prop_map(|entries| Value::Object(map_from_entries(&entries)));
    prop_oneof![array, object]
  })
}

fn object_entries() -> impl Strategy<Value = Vec<(String, Value)>> {
  prop::collection::vec(("[a-zA-Z0-9_]{1,12}", json_value()), 0..8).prop_filter(
    "unique keys",
    |entries| {
      let mut seen = HashSet::new();
      entries.iter().all(|(key, _)| seen.insert(key.clone()))
    },
  )
}

fn map_from_entries(entries: &[(String, Value)]) -> Map<String, Value> {
  let mut map = Map::new();
  for (key, value) in entries {
    map.insert(key.clone(), value.clone());
  }
  map
}

proptest! {
  #![proptest_config(proptest_config())]

  #[test]
  fn canonicalize_is_idempotent(value in json_value()) {
    let normalized = canonicalize(value);
    prop_assert_eq!(canonicalize(normalized.clone()), normalized);
  }

  #[test]
  fn canonicalize_ignores_object_key_order(entries in object_entries()) {
    let mut reversed = entries.clone();
    reversed.reverse();
    let left = Value::Object(map_from_entries(&entries));
    let right = Value::Object(map_from_entries(&reversed));
    prop_assert_eq!(canonicalize(left), canonicalize(right));
  }
}
