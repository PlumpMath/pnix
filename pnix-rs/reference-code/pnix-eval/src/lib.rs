//! pnix-eval: lightweight pure pnix expression evaluator.
//!
//! meta-circular runtime core. pnix-core AST 를 기준으로 해석하고,
//! pure domain helpers (`pnix-xml-core`, `xml-x3d-core`, `pnix-mathml-core`,
//! `pnix-openmath-core`, `pnix-svg-core`) 는 direct path 로 재사용한다.
//!
//! PnixExpr (pnix-core AST) 를 tree-walking interpret 해서 Value 를 반환.

#[cfg(test)]
mod ast_slot_probe;
#[cfg(test)]
mod bytecode_probe;
#[cfg(test)]
mod explicit_control_probe;
mod fx_map;
pub mod interpret;
pub mod markup;
mod math_markup;
pub mod package_mount;
pub mod pxir;
pub mod schema;
#[cfg(test)]
mod staging_probe;
mod svg;
pub mod value;
mod x3d;
mod xml_format_schema;

pub use interpret::{
  build_pxir_record_for_path, clear_eval_caches_for_perf_observation, clear_import_stat_memo,
  disable_deep_force_perf_timing, disk_ast_cache_production_default_on, enable_import_stat_memo,
  import_value_cache_production_default_on, init_preparsed_imports, profile_import_graph_at_path,
  reset_eval_perf_stats, reset_eval_perf_stats_for_fixture_observation,
  set_import_stat_memo_persistent, take_eval_perf_stats,
  EvalPerfStats, ImportGraphHeavyNode, ImportGraphProfile, ENV_DISK_AST_CACHE,
  ENV_IMPORT_VALUE_CACHE,
};
pub use pxir::PxirArtifactRecord;
pub use value::{Env, Value};

const _: () = {
  fn assert_send<T: Send>() {}
  fn assert_sync<T: Sync>() {}
  fn _check() {
    assert_send::<Value>();
    assert_sync::<Value>();
    assert_send::<Env>();
    assert_sync::<Env>();
  }
};

/// `.px` expression 문자열을 evaluate 하고 Value 를 반환.
///
/// API 경계에서는 `deep_force` 하여 결과 안에 `Value::Thunk` 가 남지
/// 않도록 한다. 평가 자체는 lazy 이며, 보이지 않는 thunk 가 caller 에게
/// 새지 않게 한다.
pub fn eval_expr(source: &str) -> anyhow::Result<Value> {
  let expr = interpret::parse_expr_arc_with_inline_cache(source)?;
  let env = Env::new();
  let value = interpret::eval(expr.as_ref(), &env)?;
  interpret::deep_force(value)
}

/// `.px` 파일을 evaluate 하고 Value 를 반환.
pub fn eval_file(path: &std::path::Path) -> anyhow::Result<Value> {
  let value = interpret::eval_file_at_path(path)?;
  interpret::deep_force(value)
}

/// Populate the per-thread import-value cache for a `.px` owner file.
///
/// Host-only transport helper for registry prewarm (Perf P3). Semantics
/// match `import` of the same path; no extra deep-force beyond what
/// `eval_file_at_path` already performs for cacheable modules.
pub fn warm_file_import_cache(path: &std::path::Path) -> anyhow::Result<()> {
  interpret::eval_file_at_path(path)?;
  Ok(())
}

/// `.px` expression/file 을 evaluate 하고 JSON 문자열로 반환.
pub fn eval_to_json(input: &str, is_file: bool) -> anyhow::Result<String> {
  let value = if is_file {
    eval_file(std::path::Path::new(input))?
  } else {
    eval_expr(input)?
  };
  Ok(value.to_json())
}

/// Evaluate `.px` source in an env that already has caller-supplied
/// bindings. Used by pnixc-meta's daemon mode: the boot expression
/// (`let runtime = ...; ev = ...; in ev`) is evaluated once and `ev`
/// is bound here so each subsequent query just builds a tiny per-query
/// expression that references `ev`.
pub fn eval_to_json_with_bindings(
  source: &str,
  bindings: Vec<(String, Value)>,
) -> anyhow::Result<String> {
  let expr = interpret::parse_expr_arc_with_inline_cache(source)?;
  let mut env = Env::with_capacity(bindings.len());
  for (name, value) in bindings {
    env.bind(name, value);
  }
  let value = interpret::eval(expr.as_ref(), &env)?;
  let forced = interpret::deep_force(value)?;
  Ok(forced.to_json())
}

/// Evaluate `.px` source in an env with one caller-supplied binding.
/// Hot-path helper for pnixc-meta daemon queries: avoids allocating a
/// one-element Vec and loop when the boot value is the only binding.
pub fn eval_to_json_with_binding(
  source: &str,
  name: impl Into<String>,
  value: Value,
) -> anyhow::Result<String> {
  let expr = interpret::parse_expr_arc_with_inline_cache(source)?;
  let mut env = Env::with_capacity(1);
  env.bind(name.into(), value);
  let value = interpret::eval(expr.as_ref(), &env)?;
  let forced = interpret::deep_force(value)?;
  Ok(forced.to_json())
}

/// Evaluate `.px` source with three caller-supplied bindings.
/// Hot-path helper for pnixc-meta catch-all dispatch: avoids allocating
/// a short Vec just to seed the request path/method/body environment.
pub fn eval_to_json_with_three_bindings(
  source: &str,
  first: (&str, Value),
  second: (&str, Value),
  third: (&str, Value),
) -> anyhow::Result<String> {
  let expr = interpret::parse_expr_arc_with_inline_cache(source)?;
  let mut env = Env::with_capacity(3);
  env.bind(first.0.to_owned(), first.1);
  env.bind(second.0.to_owned(), second.1);
  env.bind(third.0.to_owned(), third.1);
  let value = interpret::eval(expr.as_ref(), &env)?;
  let forced = interpret::deep_force(value)?;
  Ok(forced.to_json())
}

/// Same as [`eval_expr`] but seeds the env with pre-evaluated bindings.
pub fn eval_expr_with_bindings(
  source: &str,
  bindings: Vec<(String, Value)>,
) -> anyhow::Result<Value> {
  let expr = interpret::parse_expr_arc_with_inline_cache(source)?;
  let mut env = Env::with_capacity(bindings.len());
  for (name, value) in bindings {
    env.bind(name, value);
  }
  let value = interpret::eval(expr.as_ref(), &env)?;
  interpret::deep_force(value)
}

/// Same as [`eval_expr_with_bindings`] for a single binding.
pub fn eval_expr_with_binding(
  source: &str,
  name: impl Into<String>,
  value: Value,
) -> anyhow::Result<Value> {
  let expr = interpret::parse_expr_arc_with_inline_cache(source)?;
  let mut env = Env::with_capacity(1);
  env.bind(name.into(), value);
  let value = interpret::eval(expr.as_ref(), &env)?;
  interpret::deep_force(value)
}

// batch 261 (2026-04-18): M1-8 legacy API aliases.
// pnix-runtime-legacy 호환 이름을 pnix-eval 안에 추가해서 downstream crate
// (pnixc, pnix-query-runtime 등) 가 `pnix_runtime_legacy::eval_and_format`
// 호출을 `pnix_eval::eval_and_format` 으로 단순 문자열 대치만으로 전환할
// 수 있게 한다. 이건 M1-11 전환의 mechanical prerequisite.
//
// pnix-runtime-legacy 자체를 수정하지 않는다 (scope 밖). 대신 pnix-eval 이
// legacy 이름을 지원함으로써 호출자가 dep 만 `pnix_runtime_legacy` →
// `pnix_eval` 로 바꿔도 작동하게 만든다.

/// 출력 형식. pnix-runtime-legacy `OutputFormat` 과 호환되는 subset.
/// 현재 pnix-eval 은 JSON 만 지원. 다른 형식은 M1 feature parity 단계에서
/// 확장.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
  Json,
  /// 다른 형식 (Nix / Raw / Pretty / Edn) 은 pnix-runtime-legacy 가 소유.
  /// pnix-eval 은 상위 host 가 필요로 할 때 확장 예정.
  Unsupported,
}

/// Legacy alias of [`eval_expr`]. pnix-runtime-legacy 호환 이름.
pub fn eval_pnix_expr(source: &str) -> anyhow::Result<Value> {
  eval_expr(source)
}

/// Legacy alias of [`eval_file`]. pnix-runtime-legacy 호환 이름.
pub fn eval_pnix_file(path: &std::path::Path) -> anyhow::Result<Value> {
  eval_file(path)
}

/// Legacy alias mirroring `pnix_runtime_legacy::eval_and_format`.
/// `format` 이 `OutputFormat::Json` 이면 JSON 문자열 반환. 그 외는
/// `Unsupported` 에러 — feature parity 확장 시 확대.
pub fn eval_and_format(input: &str, is_file: bool, format: OutputFormat) -> anyhow::Result<String> {
  match format {
    OutputFormat::Json => eval_to_json(input, is_file),
    OutputFormat::Unsupported => {
      anyhow::bail!("pnix-eval: OutputFormat::Unsupported — 아직 JSON 만 지원")
    }
  }
}

#[cfg(test)]
mod eval_perf_tests {
  use super::*;

  #[test]
  fn eval_expr_records_inline_parse_counter() {
    reset_eval_perf_stats();
    let value = eval_expr("65177713").expect("eval");
    assert_eq!(value.to_json(), "65177713");
    let stats = take_eval_perf_stats();
    assert_eq!(stats.parse_count + stats.inline_parse_cache_hit_count, 1);
    assert_eq!(stats.import_count, 0);
    assert_eq!(stats.ast_cache_lookup_count, 1);
  }

  #[test]
  fn eval_expr_reuses_inline_parse_cache_for_repeated_source() {
    reset_eval_perf_stats();
    let source = "65177714";
    assert_eq!(eval_expr(source).expect("eval").to_json(), source);
    assert_eq!(eval_expr(source).expect("eval").to_json(), source);
    let stats = take_eval_perf_stats();
    assert_eq!(stats.ast_cache_lookup_count, 2);
    assert!(stats.inline_parse_cache_hit_count >= 1);
    assert!(stats.parse_count <= 1);
  }

  #[test]
  fn eval_to_json_with_single_binding_matches_multi_binding_surface() {
    assert_eq!(
      eval_to_json_with_binding("x + 1", "x", Value::Int(41)).expect("single binding"),
      "42"
    );
    assert_eq!(
      eval_to_json_with_bindings("x + 1", vec![("x".to_string(), Value::Int(41))])
        .expect("multi binding"),
      "42"
    );
  }

  #[test]
  fn eval_to_json_with_three_bindings_matches_multi_binding_surface() {
    assert_eq!(
      eval_to_json_with_three_bindings(
        "x + y + z",
        ("x", Value::Int(11)),
        ("y", Value::Int(13)),
        ("z", Value::Int(17)),
      )
      .expect("three bindings"),
      "41"
    );
    assert_eq!(
      eval_to_json_with_bindings(
        "x + y + z",
        vec![
          ("x".to_string(), Value::Int(11)),
          ("y".to_string(), Value::Int(13)),
          ("z".to_string(), Value::Int(17)),
        ],
      )
      .expect("multi binding"),
      "41"
    );
  }

  #[test]
  fn eval_expr_with_single_binding_matches_multi_binding_surface() {
    assert_eq!(
      eval_expr_with_binding("x + 1", "x", Value::Int(41))
        .expect("single binding")
        .to_json(),
      "42"
    );
    assert_eq!(
      eval_expr_with_bindings("x + 1", vec![("x".to_string(), Value::Int(41))])
        .expect("multi binding")
        .to_json(),
      "42"
    );
  }

  #[test]
  fn scoped_import_reuses_path_ast_cache_on_repeated_load() {
    let unique = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .expect("time")
      .as_nanos();
    let dir = std::env::temp_dir().join(format!(
      "pnix-eval-scoped-import-cache-{}-{}",
      std::process::id(),
      unique
    ));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let file = dir.join("fixture.px");
    std::fs::write(&file, "x + 1").expect("write fixture");
    let source = format!("builtins.scopedImport {{ x = 41; }} {}", file.display());

    reset_eval_perf_stats();
    assert_eq!(eval_expr(&source).expect("first eval").to_json(), "42");
    let first = take_eval_perf_stats();
    assert_eq!(first.file_read_count, 1);
    assert!(first.parse_count >= 1);

    reset_eval_perf_stats();
    assert_eq!(eval_expr(&source).expect("second eval").to_json(), "42");
    let second = take_eval_perf_stats();
    assert_eq!(second.file_read_count, 0);
    assert!(second.preparsed_ast_hit_count >= 1);
    assert!(second.canonical_path_cache_hit_count >= 1);

    let _ = std::fs::remove_file(&file);
    let _ = std::fs::remove_dir(&dir);
  }

  #[test]
  fn read_file_reuses_metadata_checked_content_cache() {
    let unique = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .expect("time")
      .as_nanos();
    let dir = std::env::temp_dir().join(format!(
      "pnix-eval-read-file-cache-{}-{}",
      std::process::id(),
      unique
    ));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let file = dir.join("fixture.txt");
    std::fs::write(&file, "hello-cache").expect("write fixture");
    let source = format!("builtins.readFile {}", file.display());

    reset_eval_perf_stats();
    assert_eq!(
      eval_expr(&source).expect("first eval").to_json(),
      "\"hello-cache\""
    );
    let first = take_eval_perf_stats();
    assert_eq!(first.file_read_count, 1);
    assert_eq!(first.read_file_cache_miss_count, 1);

    reset_eval_perf_stats();
    assert_eq!(
      eval_expr(&source).expect("second eval").to_json(),
      "\"hello-cache\""
    );
    let second = take_eval_perf_stats();
    assert_eq!(second.file_read_count, 0);
    assert!(second.read_file_cache_hit_count >= 1);

    let _ = std::fs::remove_file(&file);
    let _ = std::fs::remove_dir(&dir);
  }

  #[test]
  fn read_dir_reuses_metadata_checked_entry_cache() {
    let unique = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .expect("time")
      .as_nanos();
    let dir = std::env::temp_dir().join(format!(
      "pnix-eval-read-dir-cache-{}-{}",
      std::process::id(),
      unique
    ));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let file = dir.join("fixture.txt");
    std::fs::write(&file, "hello-dir-cache").expect("write fixture");
    let source = format!("builtins.readDir {}", dir.display());

    reset_eval_perf_stats();
    let first_value = eval_expr(&source).expect("first eval");
    let first_json: serde_json::Value =
      serde_json::from_str(&first_value.to_json()).expect("first json");
    assert_eq!(first_json["fixture.txt"], "regular");
    let first = take_eval_perf_stats();
    assert_eq!(first.read_dir_cache_miss_count, 1);

    reset_eval_perf_stats();
    let second_value = eval_expr(&source).expect("second eval");
    let second_json: serde_json::Value =
      serde_json::from_str(&second_value.to_json()).expect("second json");
    assert_eq!(second_json["fixture.txt"], "regular");
    let second = take_eval_perf_stats();
    assert!(second.read_dir_cache_hit_count >= 1);

    let _ = std::fs::remove_file(&file);
    let _ = std::fs::remove_dir(&dir);
  }

  #[test]
  fn path_normalize_reuses_lexical_cache() {
    let unique = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .expect("time")
      .as_nanos();
    let path = std::env::temp_dir()
      .join(format!("pnix-eval-path-normalize-cache-{}", unique))
      .join("a")
      .join("..")
      .join("fixture.txt");
    let source = format!("builtins.toPath \"{}\"", path.display());

    reset_eval_perf_stats();
    let first = eval_expr(&source).expect("first eval").to_json();
    let first_stats = take_eval_perf_stats();
    assert!(first_stats.path_normalize_cache_miss_count >= 1);

    reset_eval_perf_stats();
    let second = eval_expr(&source).expect("second eval").to_json();
    let second_stats = take_eval_perf_stats();
    assert_eq!(second, first);
    assert!(second_stats.path_normalize_cache_hit_count >= 1);
  }

  #[test]
  fn match_reuses_compiled_regex_cache_for_repeated_pattern() {
    let source = r#"builtins.match "([a-z]+)([0-9]+)" "abc123""#;

    reset_eval_perf_stats();
    assert_eq!(
      eval_expr(source).expect("first eval").to_json(),
      "[\"abc\",\"123\"]"
    );
    assert_eq!(
      eval_expr(source).expect("second eval").to_json(),
      "[\"abc\",\"123\"]"
    );
    let stats = take_eval_perf_stats();
    assert_eq!(stats.regex_cache_miss_count, 1);
    assert!(stats.regex_cache_hit_count >= 1);
    assert!(stats.builtins_select_fast_path_count >= 2);
  }

  #[test]
  fn generic_closure_deduplicates_stable_key_signature() {
    let source = r#"
      builtins.length (builtins.genericClosure {
        startSet = [ { key = 0; } { key = 0; } ];
        operator = item:
          if item.key < 2
          then [ { key = item.key + 1; } ]
          else [ ];
      })
    "#;

    assert_eq!(eval_expr(source).expect("genericClosure").to_json(), "3");
  }

  #[test]
  fn builtins_has_attr_uses_fast_path_for_known_builtin() {
    reset_eval_perf_stats();
    assert_eq!(
      eval_expr("builtins ? scopedImport")
        .expect("has attr")
        .to_json(),
      "true"
    );
    let stats = take_eval_perf_stats();
    assert_eq!(stats.builtins_has_attr_fast_path_count, 1);
  }

  #[test]
  fn builtins_fast_path_covers_schema_builtin_family() {
    reset_eval_perf_stats();
    assert_eq!(
      eval_expr("builtins.schemaNormalize")
        .expect("select schema builtin")
        .to_json(),
      "\"<builtin:schemaNormalize>\""
    );
    assert_eq!(
      eval_expr("builtins ? schemaNormalize")
        .expect("has schema builtin")
        .to_json(),
      "true"
    );
    let stats = take_eval_perf_stats();
    assert_eq!(stats.builtins_select_fast_path_count, 1);
    assert_eq!(stats.builtins_has_attr_fast_path_count, 1);
  }

  #[test]
  fn take_eval_perf_stats_resets_snapshot() {
    reset_eval_perf_stats();
    let _ = eval_expr("true").expect("eval");
    let stats = take_eval_perf_stats();
    assert!(stats.parse_count + stats.inline_parse_cache_hit_count >= 1);
    assert_eq!(take_eval_perf_stats(), EvalPerfStats::default());
  }
}
