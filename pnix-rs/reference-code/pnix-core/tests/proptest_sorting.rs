//! Proptest 정렬 테스트: 속성 기반 테스트를 사용한 정렬 기능 테스트
//!
//! Proptest를 사용하여 다양한 입력에 대한 정렬 기능을 검증합니다.

#![cfg(feature = "proptest")]

use proptest::prelude::*;

use pnix_core::diagnostics::{Diagnostic, Diagnostics, Span};

fn proptest_config() -> ProptestConfig {
  ProptestConfig {
    failure_persistence: None,
    ..ProptestConfig::default()
  }
}

fn diagnostics_for_file(file: &'static str) -> impl Strategy<Value = Vec<Diagnostic>> {
  prop::collection::btree_set(0usize..10_000, 0..8).prop_map(move |starts| {
    let mut items: Vec<Diagnostic> = starts
      .into_iter()
      .map(|start| Diagnostic {
        message: format!("m{}", start),
        span: Some(Span::with_file(start, start.saturating_add(1), file)),
        hint: None,
      })
      .collect();
    items.reverse();
    items
  })
}

fn diag_summary(items: &[Diagnostic]) -> Vec<(String, usize, String)> {
  items
    .iter()
    .map(|diag| {
      let span = diag.span.as_ref().expect("span required");
      (
        span.file.clone().unwrap_or_default(),
        span.start,
        diag.message.clone(),
      )
    })
    .collect()
}

proptest! {
  #![proptest_config(proptest_config())]

  #[test]
  fn sorted_concat_matches_concat_sorted(
    xs in diagnostics_for_file("a.px"),
    ys in diagnostics_for_file("b.px"),
  ) {
    let mut combined = xs.clone();
    combined.extend(ys.clone());

    let sorted_combined = Diagnostics { items: combined }.sorted();
    let mut expected = Diagnostics { items: xs }.sorted();
    expected.extend(Diagnostics { items: ys }.sorted());

    prop_assert_eq!(diag_summary(&sorted_combined), diag_summary(&expected));
  }
}
