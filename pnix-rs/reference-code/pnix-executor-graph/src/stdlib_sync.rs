//! 표준 라이브러리 동기화: 표준 라이브러리 모듈 로드 및 검증

#[cfg(test)]
mod tests {
  use std::collections::HashSet;
  use std::path::{Path, PathBuf};

  use pnix_core::ast::AstItem;
  use pnix_core::spec::builtin::BuiltinCatalog;
  use pnix_core::spec::stdlib::StdlibCatalog;

  fn stdlib_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
      .parent()
      .and_then(|p| p.parent())
      .expect("repo root");
    repo_root.join("stdlib")
  }

  fn load_stdlib_externs(rel_path: &str) -> Vec<String> {
    let path = stdlib_root().join(rel_path);
    // Fix: Return error instead of panicking - allows test to fail gracefully
    let ast = crate::module_loader::load_pnix_module_from_path(&path).unwrap_or_else(|err| {
      panic!(
        "load stdlib module {} failed: {}. \
           This test requires stdlib to be present. \
           Ensure stdlib files exist at {}",
        path.display(),
        err,
        stdlib_root().display()
      );
    });
    ast
      .items
      .iter()
      .filter_map(|item| match item {
        AstItem::ExternDecl { name, .. } => Some(name.clone()),
        _ => None,
      })
      .collect()
  }

  fn collect_stdlib_externs() -> Vec<String> {
    let mut externs = Vec::new();
    for module in ["lib/lists.px", "lib/strings.px", "lib/attrsets.px"] {
      externs.extend(load_stdlib_externs(module));
    }
    externs
  }

  fn is_runtime_stdlib_alias(name: &str) -> bool {
    name.starts_with("String.") || name.starts_with("List.") || name.starts_with("AttrSet.")
  }

  #[test]
  fn stdlib_aliases_match_spec_catalog() {
    let mut expected: Vec<String> = StdlibCatalog::with_defaults()
      .functions
      .values()
      .filter_map(|decl| decl.module_path.clone())
      .filter(|name| is_runtime_stdlib_alias(name))
      .collect();
    expected.sort();

    let mut found: Vec<String> = collect_stdlib_externs()
      .into_iter()
      .filter(|name| is_runtime_stdlib_alias(name))
      .collect();
    found.sort();

    assert_eq!(found, expected);
  }

  #[test]
  fn stdlib_aliases_map_to_builtins_catalog() {
    let builtin_catalog = BuiltinCatalog::with_defaults();
    for decl in StdlibCatalog::with_defaults().functions.values() {
      let Some(alias) = decl.module_path.as_deref() else {
        continue;
      };
      if !is_runtime_stdlib_alias(alias) {
        continue;
      }
      // Fix: Provide clearer error message for missing alias mapping
      let builtin = crate::builtins::resolve_builtin_name(alias).unwrap_or_else(|| {
        panic!(
          "missing builtin alias mapping for '{}'. \
             This indicates a mismatch between stdlib catalog and builtin resolver. \
             Check that STDLIB_ALIAS_MAP in pnix-core/src/spec/builtin.rs includes this alias.",
          alias
        );
      });
      assert!(
        builtin_catalog.contains(builtin.as_ref()),
        "builtin '{}' missing from spec catalog (alias: {})",
        builtin,
        alias
      );
    }
  }

  #[test]
  fn stdlib_builtin_externs_cover_aliases() {
    let externs = collect_stdlib_externs();
    let builtin_externs: HashSet<String> = externs
      .into_iter()
      .filter(|name| name.starts_with("builtins."))
      .collect();

    for decl in StdlibCatalog::with_defaults().functions.values() {
      let Some(alias) = decl.module_path.as_deref() else {
        continue;
      };
      if !is_runtime_stdlib_alias(alias) {
        continue;
      }
      // Fix: Provide clearer error message for missing alias mapping
      let builtin = crate::builtins::resolve_builtin_name(alias).unwrap_or_else(|| {
        panic!(
          "missing builtin alias mapping for '{}'. \
             This indicates a mismatch between stdlib catalog and builtin resolver. \
             Check that STDLIB_ALIAS_MAP in pnix-core/src/spec/builtin.rs includes this alias.",
          alias
        );
      });
      let builtin_name = format!("builtins.{}", builtin);
      assert!(
        builtin_externs.contains(&builtin_name),
        "stdlib externs missing {} (alias: {})",
        builtin_name,
        alias
      );
    }
  }
}
