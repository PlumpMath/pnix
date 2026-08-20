//! 내장 함수 해결: 표준 라이브러리 별칭 및 내장 함수 이름 해결

/// 내장 함수 백엔드 이름
pub const BUILTIN_BACKEND: &str = "builtins";

pub use pnix_core::spec::builtin::{is_builtin_uses, resolve_builtin_name};

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn resolve_builtin_name_handles_builtins_prefix() {
    assert_eq!(
      resolve_builtin_name("builtins.concat").as_deref(),
      Some("concat")
    );
    assert_eq!(
      resolve_builtin_name("builtins.String.length").as_deref(),
      Some("stringLength")
    );
    assert_eq!(
      resolve_builtin_name("builtins.Process.spawn").as_deref(),
      Some("processSpawn")
    );
  }

  #[test]
  fn resolve_builtin_name_handles_stdlib_aliases() {
    assert_eq!(
      resolve_builtin_name("String.length").as_deref(),
      Some("stringLength")
    );
    assert_eq!(resolve_builtin_name("List.fold").as_deref(), Some("fold"));
    assert_eq!(
      resolve_builtin_name("AttrSet.get").as_deref(),
      Some("mapGet")
    );
  }

  #[test]
  fn resolve_builtin_name_rejects_unknown() {
    assert!(resolve_builtin_name("py.numpy.add").is_none());
  }
}
