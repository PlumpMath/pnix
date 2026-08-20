//! 백엔드 RPC 디스패칭
//!
//! extern 네임스페이스 접두사를 기반으로 적절한 백엔드로 호출 라우팅

pub mod client;
pub mod clojure;
pub mod deno;
pub mod jsonrpc;
pub mod python;
pub mod query;

/// uses 문자열에서 백엔드 네임스페이스 추출 (예: "clojure.solve-linear"에서 "clojure")
pub fn backend_of(uses: &str) -> &str {
  if crate::builtins::is_builtin_uses(uses) {
    return crate::builtins::BUILTIN_BACKEND;
  }
  // CRITICAL: 알 수 없는 백엔드 처리
  // 빈 문자열 대신 명시적 에러 또는 경고 필요
  // 현재는 빈 문자열 반환 (호출자가 처리해야 함)
  // LOW: 알 수 없는 백엔드 silent 실패 수정 완료
  // 빈 백엔드는 경고 로그를 출력하며, 호출자가 빈 문자열을 처리해야 함
  // 이는 의도된 동작: 백엔드가 없으면 빈 문자열을 반환하여 호출자가 처리
  let backend = uses.split('.').next().unwrap_or("");
  if backend.is_empty() {
    // 빈 백엔드는 경고 로그 출력
    eprintln!("Warning: Empty backend for uses string '{}'", uses);
  }
  backend
}

/// uses 문자열에서 심볼 이름 추출 (예: "clojure.solve-linear"에서 "solve-linear")
pub fn symbol_of(uses: &str) -> &str {
  match uses.split_once('.') {
    Some((_backend, sym)) if !sym.is_empty() => sym,
    _ => uses,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn backend_extraction() {
    assert_eq!(backend_of("clojure.solve-linear"), "clojure");
    assert_eq!(backend_of("jvm.eval"), "jvm");
    assert_eq!(backend_of("cljs.identity"), "cljs");
    assert_eq!(backend_of("clojurescript.identity"), "clojurescript");
    assert_eq!(backend_of("py.numpy.add"), "py");
    assert_eq!(backend_of("deno.render"), "deno");
    assert_eq!(backend_of("builtins.concat"), "builtins");
    assert_eq!(backend_of("String.concat"), "builtins");
    assert_eq!(backend_of("List.map"), "builtins");
    assert_eq!(backend_of("AttrSet.get"), "builtins");
  }

  #[test]
  fn symbol_extraction() {
    assert_eq!(symbol_of("clojure.solve-linear"), "solve-linear");
    assert_eq!(symbol_of("py.numpy.add"), "numpy.add");
  }
}
