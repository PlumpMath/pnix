//! Structural cue extractor.
//!
//! OWNER-LAW (2026-05-12): mirror of
//! `stdlib/lib/gate/algorithm-synthesis/structural-cue-registry.px`.
//! Extracts `structural:*` cues from code shape + file path — the
//! companion to NL-based cue extraction (`verb_cue_registry`,
//! temporal markers). Enables the *code-only input mode*: when the
//! operator dumps a snippet with no NL, the synthesis chain still
//! has cues to start from.
//!
//! Two cue sources:
//!
//!   1. Path-based — substring match on file path. Zero false
//!      positives. e.g. `tests/foo.rs` → `structural:test-file`.
//!   2. Code-shape — light per-language detector. Currently Python
//!      only: `import X` whose name doesn't appear in the body →
//!      `structural:unused-import`.
//!
//! The cue's job is to *fire the intent*, not produce a typed
//! transform request. Downstream parameter-resolution still demands
//! proper host-resolver evidence for the actual request shape.

/// Recognized structural cue names. Sync test asserts set parity
/// against `.px` `validStructuralCues`.
pub const STRUCTURAL_CUES: &[&str] = &[
  "structural:unused-import",
  "structural:test-file",
  // pnix `.px` — graph mode (algorithm/dataflow specimen):
  "structural:px-extern-decl",
  "structural:px-node-list",
  "structural:px-edge-list",
  "structural:px-types-decl",
  // pnix `.px` — expression mode (stdlib / owner-law / gate):
  "structural:px-let-binding",
  "structural:px-lambda",
  "structural:px-import-stmt",
  "structural:px-owner-law-shape",
  // Python — code-shape patterns inviting specific transforms:
  "structural:python-untyped-def",
  "structural:python-bare-except",
  "structural:python-mutable-default-arg",
  "structural:python-percent-format",
  "structural:python-async-no-await",
];

/// Path substrings that fire `structural:test-file`. Mirror of `.px`
/// `testFilePathPatterns`.
pub const TEST_FILE_PATH_PATTERNS: &[&str] = &[
  "tests/",
  "_test.go",
  ".spec.ts",
  ".spec.tsx",
  ".test.ts",
  ".test.tsx",
  ".spec.js",
  ".spec.jsx",
  ".test.js",
  ".test.jsx",
  "test_",
  "_test.py",
];

/// Languages with code-shape detectors. Adding a language = one new
/// row + one new detector function (and the matcher dispatch below).
pub const CODE_ANALYSIS_SUPPORTED_LANGUAGES: &[&str] = &["python", "pnix"];

/// File-extension → source-language mapping. Mirror of `.px`
/// `extensionToLanguage`; sync test asserts pair-set parity.
///
/// Used by `infer_language_from_path` to derive a canonical language
/// string (the one consumed by `extract_code_signals` dispatch and
/// by code_transform's `SUPPORTED_LANGUAGES`) from a target path's
/// extension. Adding a new extension = one new row here + one new
/// `.px` entry.
pub const EXTENSION_TO_LANGUAGE: &[(&str, &str)] = &[
  ("px", "pnix"),
  ("py", "python"),
  ("rs", "rust"),
  ("ts", "typescript"),
  ("tsx", "typescript"),
  ("js", "javascript"),
  ("jsx", "javascript"),
  ("mjs", "javascript"),
  ("cjs", "javascript"),
  ("go", "go"),
];

/// Infer the source language from a file path's extension. Returns
/// `None` when the extension is missing, the path is empty, or the
/// extension is not in `EXTENSION_TO_LANGUAGE`. Caller decides
/// whether to Hold on `missing-language` or skip code-shape cues.
///
/// Crude string-based — `std::path::Path` is intentionally avoided
/// (P0-1 path-independence convention shared with
/// `lexer/block_parser::detect_base_language`).
///
/// Hidden files like `.bashrc` yield `None` because there is no
/// extension after a leading dot.
pub fn infer_language_from_path(target_path: &str) -> Option<String> {
  if target_path.is_empty() {
    return None;
  }
  // Take just the basename so a dot inside a directory name (e.g.
  // `foo.dir/bar` or `pnix_algo/v3.1/file`) does not get misread as
  // an extension.
  let basename = match target_path.rfind('/') {
    Some(i) => &target_path[i + 1..],
    None => target_path,
  };
  // Hidden files like `.bashrc` (leading dot is the only dot)
  // intentionally yield None — they're not extension-bearing.
  let dot = basename.rfind('.')?;
  if dot == 0 {
    return None;
  }
  let ext = &basename[dot + 1..];
  if ext.is_empty() {
    return None;
  }
  EXTENSION_TO_LANGUAGE
    .iter()
    .find(|(e, _)| *e == ext)
    .map(|(_, l)| (*l).to_string())
}

/// Extract path-based structural cues. Empty input → empty result.
pub fn extract_path_signals(target_path: &str) -> Vec<String> {
  if target_path.is_empty() {
    return Vec::new();
  }
  let mut out: Vec<String> = Vec::new();
  if TEST_FILE_PATH_PATTERNS
    .iter()
    .any(|pat| target_path.contains(pat))
  {
    out.push("structural:test-file".to_string());
  }
  out
}

/// Extract code-shape structural cues. Dispatches by language; empty
/// for unsupported languages or empty input.
pub fn extract_code_signals(attached_code: &str, language: &str) -> Vec<String> {
  if attached_code.is_empty() {
    return Vec::new();
  }
  match language {
    "python" => {
      // Python composes multiple detectors: unused imports stay the
      // primary signal, plus per-shape detectors that surface
      // refactor / fix-bug candidates without requiring NL.
      let mut out = detect_python_unused_imports(attached_code);
      for cue in detect_python_shapes(attached_code) {
        if !out.iter().any(|s| s == &cue) {
          out.push(cue);
        }
      }
      out
    }
    "pnix" => detect_pnix_shapes(attached_code),
    _ => Vec::new(),
  }
}

/// Python shape detector. Composes high-precision per-pattern checks
/// that mirror the `ankh-language/python.px` risky-pattern lineage
/// (bare-except, mutable-default-argument, etc.) — but here the
/// purpose is *intent routing*, not safety blocking. Each fired cue
/// invites a specific refactor / fix-bug transform.
///
/// Fires (one or more, in registry order):
///   - `structural:python-untyped-def`      — `def <name>(...)` body
///                                            without trailing `->`
///   - `structural:python-bare-except`      — `except:` clause
///   - `structural:python-mutable-default-arg` — `def f(x=[])` /
///                                            `def f(x={})` pattern
///   - `structural:python-percent-format`   — `"..." % (...)` /
///                                            `"..." % x` pattern
///   - `structural:python-async-no-await`   — `async def` body that
///                                            contains no `await`
fn detect_python_shapes(code: &str) -> Vec<String> {
  let mut out: Vec<String> = Vec::new();

  if has_python_untyped_def(code) {
    out.push("structural:python-untyped-def".to_string());
  }
  if has_python_bare_except(code) {
    out.push("structural:python-bare-except".to_string());
  }
  if has_python_mutable_default(code) {
    out.push("structural:python-mutable-default-arg".to_string());
  }
  if has_python_percent_format(code) {
    out.push("structural:python-percent-format".to_string());
  }
  if has_python_async_no_await(code) {
    out.push("structural:python-async-no-await".to_string());
  }
  out
}

/// Detect at least one `def <name>(...)` that has no `->` return-type
/// annotation before its `:`. Crude scan — false positives on inner
/// strings/comments are acceptable for cue firing.
fn has_python_untyped_def(code: &str) -> bool {
  for line in code.lines() {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("def ") && !trimmed.starts_with("async def ") {
      continue;
    }
    // Find the closing `)` that ends the parameter list, then check
    // whether the suffix up to `:` contains a `->`. Multi-line defs
    // (signature spans multiple lines) are not detected here — that
    // would require a balanced paren scan.
    let after_def = match trimmed
      .strip_prefix("async def ")
      .or_else(|| trimmed.strip_prefix("def "))
    {
      Some(s) => s,
      None => continue,
    };
    let mut depth = 0i32;
    let mut close_paren_idx = None;
    for (i, c) in after_def.char_indices() {
      match c {
        '(' => depth += 1,
        ')' => {
          depth -= 1;
          if depth == 0 {
            close_paren_idx = Some(i);
            break;
          }
        }
        _ => {}
      }
    }
    let Some(close_idx) = close_paren_idx else {
      continue;
    };
    let after_paren = &after_def[close_idx + 1..];
    // Look at everything before the first `:` on this line.
    let signature_tail = match after_paren.find(':') {
      Some(i) => &after_paren[..i],
      None => after_paren,
    };
    if !signature_tail.contains("->") {
      return true;
    }
  }
  false
}

/// Detect `except:` (bare except) clause.
fn has_python_bare_except(code: &str) -> bool {
  for line in code.lines() {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix("except") {
      // Skip whitespace after `except`. If the next non-space char
      // is `:`, it's bare. Anything else (`except Foo:` or
      // `except (A, B):`) is typed and OK.
      let after = rest.trim_start();
      if after.starts_with(':') {
        return true;
      }
    }
  }
  false
}

/// Detect `def f(x=[])` / `def f(x={})` style mutable defaults.
/// Crude substring match — `def[ ]<ident>(<...>=[<...>` or
/// `def[ ]<ident>(<...>={<...>`.
fn has_python_mutable_default(code: &str) -> bool {
  for line in code.lines() {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("def ") && !trimmed.starts_with("async def ") {
      continue;
    }
    // Look for `=[` or `={` inside the parameter list. Excludes
    // `=None` / `=0` / `=""` / `=tuple()` etc.
    if line.contains("=[]")
      || line.contains("={}")
      || line.contains("= []")
      || line.contains("= {}")
    {
      return true;
    }
  }
  false
}

/// Detect old-style `"..." % (...)` percent-formatting.
fn has_python_percent_format(code: &str) -> bool {
  for line in code.lines() {
    // Look for `" %` or `' %` patterns where a string literal is
    // immediately followed by `%` formatting. The space disambiguates
    // from `"abc%def"` literal content.
    if line.contains("\" %") || line.contains("' %") || line.contains("\"%") {
      // Stricter check: `\"%` would mean adjacent. Look for typical
      // format-spec characters after `%`: s, d, f, r, x, etc.
      let bytes = line.as_bytes();
      for (i, _) in line.match_indices('%') {
        if i == 0 {
          continue;
        }
        // Need at least 1 char after `%` for the format spec.
        if i + 1 >= bytes.len() {
          continue;
        }
        let prev = bytes[i - 1];
        let next = bytes[i + 1];
        // Previous char should be `"`, `'`, or close them after space.
        // Next should look like format-spec start.
        let prev_looks_string = matches!(prev, b'"' | b'\'') || prev == b' ';
        let next_looks_spec = matches!(next, b's' | b'd' | b'f' | b'r' | b'x' | b'i' | b'(' | b'%');
        if prev_looks_string && next_looks_spec {
          return true;
        }
      }
    }
  }
  false
}

/// Detect `async def` body that contains no `await` keyword anywhere
/// in the function. Crude — does not delimit function bodies, so a
/// large file with one `async def` somewhere AND one stray `await`
/// somewhere else still misses the cue (intentional false negative).
fn has_python_async_no_await(code: &str) -> bool {
  let has_async_def = code.contains("async def ");
  if !has_async_def {
    return false;
  }
  // If the file has `async def` anywhere AND no `await` anywhere,
  // it's a sign that an async function is sync-bodied.
  let has_await = code.contains("await ")
    || code.contains("\tawait")
    || code.contains("(await")
    || code.contains(" await\n");
  has_async_def && !has_await
}

/// pnix `.px` shape detector. Inspects the snippet for graph-mode
/// markers (`externs/nodes/edges/types` attrset entries) and
/// expression-mode markers (`let`/lambda/`import ./*.px`/
/// `ownerName = "..."`). Crude substring + line scan — not a parser.
/// Downstream parameter-resolution still demands typed evidence for
/// the actual transform request.
///
/// Each pattern is high-precision; false positives mostly occur in
/// strings/comments and are acceptable for cue firing (intent
/// classifier weights them against other cues, and the 5-gate
/// firewall ultimately gates promotion).
///
/// Fires (one or more, in registry order):
///   - `structural:px-extern-decl`    — `externs = [` attrset entry
///   - `structural:px-node-list`      — `nodes = [`
///   - `structural:px-edge-list`      — `edges = [`
///   - `structural:px-types-decl`     — `types = [`
///   - `structural:px-let-binding`    — `let ... in` keyword pair
///   - `structural:px-lambda`         — `= <ident>:` lambda parameter
///   - `structural:px-import-stmt`    — `import ./...px` relative import
///   - `structural:px-owner-law-shape`— `ownerName = "..."` declaration
fn detect_pnix_shapes(code: &str) -> Vec<String> {
  let mut out: Vec<String> = Vec::new();

  // Graph-mode markers (algorithm/dataflow specimen, attrset shape).
  if code.contains("externs = [") || code.contains("externs= [") {
    out.push("structural:px-extern-decl".to_string());
  }
  if code.contains("nodes = [") {
    out.push("structural:px-node-list".to_string());
  }
  if code.contains("edges = [") {
    out.push("structural:px-edge-list".to_string());
  }
  if code.contains("types = [") {
    out.push("structural:px-types-decl".to_string());
  }

  // Expression-mode markers (owner-law / stdlib).
  if has_pnix_let_in(code) {
    out.push("structural:px-let-binding".to_string());
  }
  if has_pnix_lambda(code) {
    out.push("structural:px-lambda".to_string());
  }
  if code.contains("import ./") {
    out.push("structural:px-import-stmt".to_string());
  }
  if code.contains("ownerName = \"") || code.contains("ownerName=\"") {
    out.push("structural:px-owner-law-shape".to_string());
  }

  out
}

/// Detect pnix `let ... in` keyword pair. Both keywords must be
/// present at line boundaries (line start or after whitespace, not
/// inside an identifier like "applet" or "regret"). Crude — false
/// positives on strings/comments containing the keywords are
/// acceptable for cue firing.
fn has_pnix_let_in(code: &str) -> bool {
  let has_let_keyword = code.contains("\nlet\n")
    || code.contains("\nlet ")
    || code.starts_with("let\n")
    || code.starts_with("let ");
  let has_in_keyword = code.contains("\nin ")
    || code.contains("\nin\n")
    || code.contains(" in {")
    || code.contains(" in (")
    || code.contains(" in rec")
    || code.contains(") in ")
    || code.contains("; in ");
  has_let_keyword && has_in_keyword
}

/// Detect pnix lambda parameter declaration. Looks for the pattern
/// `= <ident>:` on any non-comment line — distinguishing lambda
/// (`name = arg: body`) from attrset entry (`name = value;`).
/// Skips `==`, `!=`, `>=`, `<=`, `::` to avoid operator false
/// positives, and skips `=` inside string literals via a simple
/// `"`-count check.
fn has_pnix_lambda(code: &str) -> bool {
  for line in code.lines() {
    let trimmed_line = line.trim_start();
    // Skip comment-only lines (pnix uses `#` for line comments).
    if trimmed_line.starts_with('#') {
      continue;
    }
    let bytes = line.as_bytes();
    for (i, _) in line.match_indices('=') {
      // Reject operator `=` forms: `==`, `=>`.
      if i + 1 < bytes.len() && matches!(bytes[i + 1], b'=' | b'>') {
        continue;
      }
      // Reject second char of `==`, `!=`, `>=`, `<=`, `:=`.
      if i > 0 && matches!(bytes[i - 1], b'=' | b'!' | b'>' | b'<' | b':') {
        continue;
      }
      // Reject `=` inside a string: odd number of `"` to the left of `=`.
      let quote_count = line[..i].matches('"').count();
      if quote_count % 2 == 1 {
        continue;
      }
      // Scan after `=`: skip whitespace, expect identifier, then `:`.
      let after = &line[i + 1..];
      let trimmed = after.trim_start();
      let first = trimmed.chars().next();
      let Some(first_char) = first else {
        continue;
      };
      if !(first_char.is_ascii_alphabetic() || first_char == '_') {
        continue;
      }
      let ident_end = trimmed
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(trimmed.len());
      let after_ident = trimmed[ident_end..].trim_start();
      // Lambda parameter colon — and not `::` (path) or `:=`.
      if after_ident.starts_with(':')
        && !after_ident.starts_with("::")
        && !after_ident.starts_with(":=")
      {
        return true;
      }
    }
  }
  false
}

/// Python unused-import detector. Crude by design — fires the cue
/// when AT LEAST ONE imported name is never referenced in the
/// non-import body. Parameter-resolution still demands real host-
/// resolver evidence for the actual transform request.
///
/// Recognized import forms:
///   - `import x`
///   - `import x as y` (the alias `y` is the referenced name)
///   - `from m import x`
///   - `from m import x, y, z`
///   - `from m import x as y` (the alias `y` is the referenced name)
fn detect_python_unused_imports(code: &str) -> Vec<String> {
  // First pass: collect (import_line_idx, referenced_names) per line.
  let mut imported_names: Vec<(usize, Vec<String>)> = Vec::new();
  let lines: Vec<&str> = code.lines().collect();
  for (i, line) in lines.iter().enumerate() {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix("from ") {
      // `from m import x[ as y][, ...]`
      let Some(import_idx) = rest.find(" import ") else {
        continue;
      };
      let names_part = &rest[import_idx + " import ".len()..];
      // Strip trailing inline comment.
      let names_part = match names_part.find('#') {
        Some(h) => &names_part[..h],
        None => names_part,
      };
      let mut names = Vec::new();
      for chunk in names_part.split(',') {
        let chunk = chunk.trim();
        if chunk.is_empty() || chunk == "*" {
          continue;
        }
        // `<orig> as <alias>` → alias is the referenced name.
        let referenced = if let Some(as_pos) = chunk.find(" as ") {
          chunk[as_pos + " as ".len()..].trim().to_string()
        } else {
          chunk.to_string()
        };
        if !referenced.is_empty() {
          names.push(referenced);
        }
      }
      if !names.is_empty() {
        imported_names.push((i, names));
      }
    } else if let Some(rest) = trimmed.strip_prefix("import ") {
      // `import x[, y][ as z]` — Python's `import a, b` is legal but
      // `import a as x, b as y` is also legal.
      let rest = match rest.find('#') {
        Some(h) => &rest[..h],
        None => rest,
      };
      let mut names = Vec::new();
      for chunk in rest.split(',') {
        let chunk = chunk.trim();
        if chunk.is_empty() {
          continue;
        }
        let referenced = if let Some(as_pos) = chunk.find(" as ") {
          chunk[as_pos + " as ".len()..].trim().to_string()
        } else {
          // `import os.path` — the referenced name is `os` (the top-
          // level binding). Take everything before the first `.`.
          let head = chunk.split('.').next().unwrap_or(chunk).trim();
          head.to_string()
        };
        if !referenced.is_empty() {
          names.push(referenced);
        }
      }
      if !names.is_empty() {
        imported_names.push((i, names));
      }
    }
  }

  if imported_names.is_empty() {
    return Vec::new();
  }

  // Second pass: build a body string (every line except the
  // import-declaration lines themselves) and check substring
  // presence. Crude — doesn't distinguish identifiers in strings /
  // comments. Acceptable for cue firing (the downstream symbol
  // resolver is the source of truth for the actual transform).
  let import_indices: std::collections::HashSet<usize> =
    imported_names.iter().map(|(i, _)| *i).collect();
  let body: String = lines
    .iter()
    .enumerate()
    .filter(|(i, _)| !import_indices.contains(i))
    .map(|(_, l)| *l)
    .collect::<Vec<_>>()
    .join("\n");

  let any_unused = imported_names
    .iter()
    .flat_map(|(_, names)| names.iter())
    .any(|name| !body.contains(name));
  if any_unused {
    vec!["structural:unused-import".to_string()]
  } else {
    Vec::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn empty_path_yields_no_signals() {
    assert!(extract_path_signals("").is_empty());
  }

  #[test]
  fn empty_code_yields_no_signals() {
    assert!(extract_code_signals("", "python").is_empty());
  }

  #[test]
  fn unsupported_language_yields_no_code_signals() {
    let code = "import os\nprint('x')\n";
    assert!(extract_code_signals(code, "ruby").is_empty());
  }

  // ─── path-based test-file detection ─────────────────────────────

  #[test]
  fn path_tests_dir_fires_test_file() {
    let s = extract_path_signals("crates/foo/tests/it.rs");
    assert!(s.contains(&"structural:test-file".to_string()));
  }

  #[test]
  fn path_go_test_fires_test_file() {
    let s = extract_path_signals("pkg/foo_test.go");
    assert!(s.contains(&"structural:test-file".to_string()));
  }

  #[test]
  fn path_spec_ts_fires_test_file() {
    let s = extract_path_signals("src/utils.spec.ts");
    assert!(s.contains(&"structural:test-file".to_string()));
  }

  #[test]
  fn path_python_test_prefix_fires_test_file() {
    let s = extract_path_signals("test_utils.py");
    assert!(s.contains(&"structural:test-file".to_string()));
  }

  #[test]
  fn path_python_test_suffix_fires_test_file() {
    let s = extract_path_signals("utils_test.py");
    assert!(s.contains(&"structural:test-file".to_string()));
  }

  #[test]
  fn ordinary_path_does_not_fire_test_file() {
    let s = extract_path_signals("src/main.rs");
    assert!(s.is_empty());
  }

  // ─── Python unused-import detection ─────────────────────────────

  #[test]
  fn python_unused_import_fires() {
    let code = "import os\nimport sys\nprint(sys.version)\n";
    // `os` is unused; `sys` is used.
    let s = extract_code_signals(code, "python");
    assert!(
      s.contains(&"structural:unused-import".to_string()),
      "got {s:?}"
    );
  }

  #[test]
  fn python_all_imports_used_does_not_fire() {
    let code = "import os\nprint(os.getcwd())\n";
    let s = extract_code_signals(code, "python");
    assert!(s.is_empty(), "got {s:?}");
  }

  #[test]
  fn python_from_import_unused_fires() {
    let code = "from collections import OrderedDict, deque\nprint(deque)\n";
    // OrderedDict is imported but never referenced.
    let s = extract_code_signals(code, "python");
    assert!(
      s.contains(&"structural:unused-import".to_string()),
      "got {s:?}"
    );
  }

  #[test]
  fn python_aliased_import_uses_alias_name() {
    let code = "import numpy as np\nprint(np.array([]))\n";
    // The alias `np` is used; the original `numpy` is NOT what we
    // check — the alias is. So this should be considered all-used.
    let s = extract_code_signals(code, "python");
    assert!(s.is_empty(), "got {s:?}");
  }

  #[test]
  fn python_aliased_import_unused_alias_fires() {
    let code = "import numpy as np\nprint('hello')\n";
    let s = extract_code_signals(code, "python");
    assert!(
      s.contains(&"structural:unused-import".to_string()),
      "got {s:?}"
    );
  }

  #[test]
  fn python_dotted_import_uses_head_name() {
    // `import os.path` binds `os` at top level; references are
    // `os.path.foo(...)`. The head name `os` is what we check.
    let code = "import os.path\nprint(os.path.join('a', 'b'))\n";
    let s = extract_code_signals(code, "python");
    assert!(s.is_empty(), "got {s:?}");
  }

  #[test]
  fn python_inline_comment_after_import_ignored() {
    let code = "import os  # for path utilities\nprint(os.getcwd())\n";
    let s = extract_code_signals(code, "python");
    assert!(s.is_empty(), "got {s:?}");
  }

  #[test]
  fn python_star_import_does_not_fire() {
    // `from x import *` is opaque — we can't decide what's used.
    // Don't fire (cue is for cleanup; a wildcard import is its own
    // separate code smell).
    let code = "from os.path import *\nprint('hi')\n";
    let s = extract_code_signals(code, "python");
    assert!(s.is_empty(), "got {s:?}");
  }

  #[test]
  fn python_no_imports_returns_empty() {
    let code = "print('hi')\n";
    let s = extract_code_signals(code, "python");
    assert!(s.is_empty());
  }

  // ─── pnix `.px` shape detection ─────────────────────────────────

  #[test]
  fn pnix_graph_mode_externs_fires() {
    let code = r#"{ name = "foo"; externs = [ { name = "py.add"; } ]; }"#;
    let s = extract_code_signals(code, "pnix");
    assert!(
      s.contains(&"structural:px-extern-decl".to_string()),
      "got {s:?}"
    );
  }

  #[test]
  fn pnix_graph_mode_nodes_fires() {
    let code = "{\n  name = \"x\";\n  nodes = [ { name = \"n1\"; } ];\n}";
    let s = extract_code_signals(code, "pnix");
    assert!(
      s.contains(&"structural:px-node-list".to_string()),
      "got {s:?}"
    );
  }

  #[test]
  fn pnix_graph_mode_edges_fires() {
    let code = "{\n  edges = [ { from = \"a\"; to = \"b\"; } ];\n}";
    let s = extract_code_signals(code, "pnix");
    assert!(
      s.contains(&"structural:px-edge-list".to_string()),
      "got {s:?}"
    );
  }

  #[test]
  fn pnix_graph_mode_types_fires() {
    let code = "{ types = [ \"Num\" \"Bool\" ]; }";
    let s = extract_code_signals(code, "pnix");
    assert!(
      s.contains(&"structural:px-types-decl".to_string()),
      "got {s:?}"
    );
  }

  #[test]
  fn pnix_graph_mode_full_specimen_fires_all_four() {
    // Realistic algorithm corpus shape (W44 / TinyLang style).
    let code = r#"{
  name = "hof_list_ops";
  types = [ "List" "Num" ];
  inputs = { xs = "List"; };
  externs = [ { name = "builtins.map"; } ];
  nodes = [ { name = "n1"; uses = "builtins.map"; } ];
  edges = [ { from = { input = "xs"; }; to = { node = "n1"; }; } ];
}"#;
    let s = extract_code_signals(code, "pnix");
    assert!(
      s.contains(&"structural:px-extern-decl".to_string()),
      "got {s:?}"
    );
    assert!(
      s.contains(&"structural:px-node-list".to_string()),
      "got {s:?}"
    );
    assert!(
      s.contains(&"structural:px-edge-list".to_string()),
      "got {s:?}"
    );
    assert!(
      s.contains(&"structural:px-types-decl".to_string()),
      "got {s:?}"
    );
  }

  #[test]
  fn pnix_expression_mode_let_in_fires() {
    let code = "let\n  x = 1;\n  y = 2;\nin { sum = x + y; }";
    let s = extract_code_signals(code, "pnix");
    assert!(
      s.contains(&"structural:px-let-binding".to_string()),
      "got {s:?}"
    );
  }

  #[test]
  fn pnix_expression_mode_lambda_fires() {
    // `mkSignalEntry = cue: intent: weight: { ... };` — common stdlib lambda shape.
    let code = "let\n  mkSignalEntry = cue: intent: weight: { inherit cue intent weight; };\nin { x = mkSignalEntry \"a\" \"b\" 0.5; }";
    let s = extract_code_signals(code, "pnix");
    assert!(s.contains(&"structural:px-lambda".to_string()), "got {s:?}");
  }

  #[test]
  fn pnix_expression_mode_import_stmt_fires() {
    let code = "let\n  other = import ./other.px;\nin { x = other; }";
    let s = extract_code_signals(code, "pnix");
    assert!(
      s.contains(&"structural:px-import-stmt".to_string()),
      "got {s:?}"
    );
  }

  #[test]
  fn pnix_expression_mode_owner_law_shape_fires() {
    let code = "let\n  ownerName = \"stdlib/lib/gate/foo.px\";\nin { inherit ownerName; }";
    let s = extract_code_signals(code, "pnix");
    assert!(
      s.contains(&"structural:px-owner-law-shape".to_string()),
      "got {s:?}"
    );
  }

  #[test]
  fn pnix_expression_mode_full_owner_fires_multiple() {
    let code = r#"let
  ownerName = "stdlib/lib/gate/algorithm-synthesis/example.px";
  other = import ./helper.px;
  mkEntry = a: b: { inherit a b; };
in {
  inherit ownerName mkEntry;
}"#;
    let s = extract_code_signals(code, "pnix");
    assert!(
      s.contains(&"structural:px-let-binding".to_string()),
      "got {s:?}"
    );
    assert!(s.contains(&"structural:px-lambda".to_string()), "got {s:?}");
    assert!(
      s.contains(&"structural:px-import-stmt".to_string()),
      "got {s:?}"
    );
    assert!(
      s.contains(&"structural:px-owner-law-shape".to_string()),
      "got {s:?}"
    );
  }

  #[test]
  fn pnix_attrset_only_does_not_fire_lambda() {
    // Pure attrset (no lambda): `name = value;` pattern, not `name = arg:`.
    let code = "{\n  name = \"foo\";\n  count = 3;\n}";
    let s = extract_code_signals(code, "pnix");
    assert!(
      !s.contains(&"structural:px-lambda".to_string()),
      "lambda false positive on pure attrset: got {s:?}"
    );
  }

  #[test]
  fn pnix_lambda_skips_equality_operators() {
    // `==` / `!=` / `>=` / `<=` must not be misread as `= ident:`.
    let code = "let\n  x = if a == b then 1 else 0;\nin x";
    let s = extract_code_signals(code, "pnix");
    assert!(
      !s.contains(&"structural:px-lambda".to_string()),
      "lambda false positive on equality op: got {s:?}"
    );
  }

  #[test]
  fn pnix_lambda_skips_string_literal_equals() {
    // `= "string: thing"` must not fire (the colon is inside a string).
    let code = "{ message = \"name: oops\"; }";
    let s = extract_code_signals(code, "pnix");
    assert!(
      !s.contains(&"structural:px-lambda".to_string()),
      "lambda false positive on string colon: got {s:?}"
    );
  }

  #[test]
  fn pnix_let_in_requires_both_keywords() {
    // `let` alone (no `in`) does not fire.
    let code = "{ description = \"a let-shaped phrase\"; }";
    let s = extract_code_signals(code, "pnix");
    assert!(
      !s.contains(&"structural:px-let-binding".to_string()),
      "let-binding false positive on prose: got {s:?}"
    );
  }

  #[test]
  fn pnix_empty_code_yields_empty() {
    assert!(extract_code_signals("", "pnix").is_empty());
  }

  #[test]
  fn pnix_unrelated_text_yields_empty() {
    // Plain prose with no .px markers.
    let code = "this is just some text with no pnix structure at all";
    let s = extract_code_signals(code, "pnix");
    assert!(s.is_empty(), "got {s:?}");
  }

  // ─── language inference from path ───────────────────────────────

  #[test]
  fn infer_px_extension_yields_pnix() {
    assert_eq!(
      infer_language_from_path("examples/pnix_algo/W44.px").as_deref(),
      Some("pnix")
    );
  }

  #[test]
  fn infer_py_extension_yields_python() {
    assert_eq!(
      infer_language_from_path("src/main.py").as_deref(),
      Some("python")
    );
  }

  #[test]
  fn infer_rs_extension_yields_rust() {
    assert_eq!(
      infer_language_from_path("crates/foo/src/lib.rs").as_deref(),
      Some("rust")
    );
  }

  #[test]
  fn infer_ts_and_tsx_yield_typescript() {
    assert_eq!(
      infer_language_from_path("src/x.ts").as_deref(),
      Some("typescript")
    );
    assert_eq!(
      infer_language_from_path("src/x.tsx").as_deref(),
      Some("typescript")
    );
  }

  #[test]
  fn infer_js_jsx_mjs_cjs_yield_javascript() {
    assert_eq!(
      infer_language_from_path("x.js").as_deref(),
      Some("javascript")
    );
    assert_eq!(
      infer_language_from_path("x.jsx").as_deref(),
      Some("javascript")
    );
    assert_eq!(
      infer_language_from_path("x.mjs").as_deref(),
      Some("javascript")
    );
    assert_eq!(
      infer_language_from_path("x.cjs").as_deref(),
      Some("javascript")
    );
  }

  #[test]
  fn infer_go_extension_yields_go() {
    assert_eq!(
      infer_language_from_path("cmd/server/main.go").as_deref(),
      Some("go")
    );
  }

  #[test]
  fn infer_empty_path_yields_none() {
    assert!(infer_language_from_path("").is_none());
  }

  #[test]
  fn infer_no_extension_yields_none() {
    assert!(infer_language_from_path("Makefile").is_none());
    assert!(infer_language_from_path("path/to/binary").is_none());
  }

  #[test]
  fn infer_unrecognized_extension_yields_none() {
    assert!(infer_language_from_path("Cargo.toml").is_none());
    assert!(infer_language_from_path("config.json").is_none());
    assert!(infer_language_from_path("doc.md").is_none());
  }

  #[test]
  fn infer_hidden_dotfile_yields_none() {
    // `.bashrc` and `.gitignore` — leading dot is the only dot,
    // no real extension.
    assert!(infer_language_from_path(".bashrc").is_none());
    assert!(infer_language_from_path(".gitignore").is_none());
    assert!(infer_language_from_path("path/.envrc").is_none());
  }

  #[test]
  fn infer_dot_in_directory_name_not_misread_as_extension() {
    // `pnix_algo/v3.1/file` — the `.1` in directory is not the
    // extension. The file has no extension → None.
    assert!(infer_language_from_path("pnix_algo/v3.1/file").is_none());
    // But `pnix_algo/v3.1/file.px` should still infer pnix.
    assert_eq!(
      infer_language_from_path("pnix_algo/v3.1/file.px").as_deref(),
      Some("pnix")
    );
  }

  #[test]
  fn infer_full_corpus_path_works() {
    // Realistic paths from the algorithm corpus.
    assert_eq!(
      infer_language_from_path("examples/pnix_algo/completed/mathematical/MATH-01-gcd_extended.px")
        .as_deref(),
      Some("pnix")
    );
    assert_eq!(
      infer_language_from_path("stdlib/lib/gate/algorithm-synthesis/intent-recognition.px")
        .as_deref(),
      Some("pnix")
    );
  }

  #[test]
  fn extension_to_language_map_has_no_duplicates() {
    // Each (extension, language) pair must be unique. The same
    // extension cannot map to two languages — that would make
    // inference non-deterministic.
    let mut seen_exts: Vec<&str> = Vec::new();
    for (ext, _) in EXTENSION_TO_LANGUAGE {
      assert!(
        !seen_exts.contains(ext),
        "duplicate extension `{ext}` in EXTENSION_TO_LANGUAGE"
      );
      seen_exts.push(ext);
    }
  }

  #[test]
  fn inferred_language_for_pnix_dispatches_to_pnix_detector() {
    // End-to-end: infer language from `.px` path, then use the
    // inferred language with extract_code_signals → pnix detector
    // fires on a graph-mode specimen.
    let path = "examples/pnix_algo/W44.px";
    let lang = infer_language_from_path(path).expect("should infer pnix");
    let code = r#"{ name = "x"; externs = [ { name = "py.add"; } ]; }"#;
    let s = extract_code_signals(code, &lang);
    assert!(
      s.contains(&"structural:px-extern-decl".to_string()),
      "inferred-language dispatch did not fire pnix detector: got {s:?}"
    );
  }

  // ─── Python shape detection — substrate-sharing N=5 ─────────────

  #[test]
  fn python_untyped_def_fires() {
    let code = "def add(a, b):\n    return a + b\n";
    let s = extract_code_signals(code, "python");
    assert!(
      s.contains(&"structural:python-untyped-def".to_string()),
      "got {s:?}"
    );
  }

  #[test]
  fn python_typed_def_does_not_fire_untyped() {
    let code = "def add(a: int, b: int) -> int:\n    return a + b\n";
    let s = extract_code_signals(code, "python");
    assert!(
      !s.contains(&"structural:python-untyped-def".to_string()),
      "false positive on typed def: got {s:?}"
    );
  }

  #[test]
  fn python_async_def_without_return_type_fires_untyped() {
    let code = "async def fetch(url):\n    return await client.get(url)\n";
    let s = extract_code_signals(code, "python");
    assert!(
      s.contains(&"structural:python-untyped-def".to_string()),
      "got {s:?}"
    );
  }

  #[test]
  fn python_bare_except_fires() {
    let code = "try:\n    risky()\nexcept:\n    pass\n";
    let s = extract_code_signals(code, "python");
    assert!(
      s.contains(&"structural:python-bare-except".to_string()),
      "got {s:?}"
    );
  }

  #[test]
  fn python_typed_except_does_not_fire_bare() {
    let code = "try:\n    risky()\nexcept ValueError:\n    pass\n";
    let s = extract_code_signals(code, "python");
    assert!(
      !s.contains(&"structural:python-bare-except".to_string()),
      "false positive on typed except: got {s:?}"
    );
  }

  #[test]
  fn python_mutable_default_list_fires() {
    let code = "def f(items=[]):\n    items.append(1)\n";
    let s = extract_code_signals(code, "python");
    assert!(
      s.contains(&"structural:python-mutable-default-arg".to_string()),
      "got {s:?}"
    );
  }

  #[test]
  fn python_mutable_default_dict_fires() {
    let code = "def f(cache = {}):\n    return cache\n";
    let s = extract_code_signals(code, "python");
    assert!(
      s.contains(&"structural:python-mutable-default-arg".to_string()),
      "got {s:?}"
    );
  }

  #[test]
  fn python_safe_default_does_not_fire_mutable() {
    let code = "def f(items=None):\n    return items or []\n";
    let s = extract_code_signals(code, "python");
    assert!(
      !s.contains(&"structural:python-mutable-default-arg".to_string()),
      "false positive on None default: got {s:?}"
    );
  }

  #[test]
  fn python_percent_format_fires() {
    let code = "msg = \"hello %s, age %d\" % (name, age)\n";
    let s = extract_code_signals(code, "python");
    assert!(
      s.contains(&"structural:python-percent-format".to_string()),
      "got {s:?}"
    );
  }

  #[test]
  fn python_fstring_does_not_fire_percent() {
    let code = "msg = f\"hello {name}, age {age}\"\n";
    let s = extract_code_signals(code, "python");
    assert!(
      !s.contains(&"structural:python-percent-format".to_string()),
      "false positive on f-string: got {s:?}"
    );
  }

  #[test]
  fn python_async_no_await_fires() {
    let code = "async def fetch(url):\n    return client.get(url)\n";
    let s = extract_code_signals(code, "python");
    assert!(
      s.contains(&"structural:python-async-no-await".to_string()),
      "got {s:?}"
    );
  }

  #[test]
  fn python_async_with_await_does_not_fire_no_await() {
    let code = "async def fetch(url):\n    return await client.get(url)\n";
    let s = extract_code_signals(code, "python");
    assert!(
      !s.contains(&"structural:python-async-no-await".to_string()),
      "false positive when await present: got {s:?}"
    );
  }

  #[test]
  fn python_realistic_legacy_code_fires_multiple_cues() {
    // Real legacy Python pattern: untyped + bare except + percent fmt
    // + mutable default — pnix should flag all of them.
    let code = r#"
def process(items=[]):
    try:
        for item in items:
            print("processing %s" % item)
    except:
        pass
"#;
    let s = extract_code_signals(code, "python");
    assert!(
      s.contains(&"structural:python-untyped-def".to_string()),
      "got {s:?}"
    );
    assert!(
      s.contains(&"structural:python-bare-except".to_string()),
      "got {s:?}"
    );
    assert!(
      s.contains(&"structural:python-mutable-default-arg".to_string()),
      "got {s:?}"
    );
    assert!(
      s.contains(&"structural:python-percent-format".to_string()),
      "got {s:?}"
    );
  }

  #[test]
  fn python_modern_code_fires_no_legacy_cues() {
    // Type-hinted, f-string, no mutable defaults, no bare except.
    let code = r#"
def process(items: list[int] = None) -> int:
    items = items or []
    try:
        for item in items:
            print(f"processing {item}")
    except (ValueError, TypeError):
        return 0
    return len(items)
"#;
    let s = extract_code_signals(code, "python");
    assert!(
      !s.contains(&"structural:python-untyped-def".to_string()),
      "got {s:?}"
    );
    assert!(
      !s.contains(&"structural:python-bare-except".to_string()),
      "got {s:?}"
    );
    assert!(
      !s.contains(&"structural:python-mutable-default-arg".to_string()),
      "got {s:?}"
    );
    assert!(
      !s.contains(&"structural:python-percent-format".to_string()),
      "got {s:?}"
    );
  }

  #[test]
  fn python_unused_import_still_works_alongside_new_detectors() {
    // Regression: adding the new detectors must not break the
    // original unused-import detector.
    let code = "import os\nimport sys\nprint(sys.version)\n";
    let s = extract_code_signals(code, "python");
    assert!(
      s.contains(&"structural:unused-import".to_string()),
      "got {s:?}"
    );
  }

  // ─── registry consistency ───────────────────────────────────────

  #[test]
  fn structural_cues_const_matches_what_extractors_can_fire() {
    // The extractors can only emit cues from `STRUCTURAL_CUES`.
    let pnix_specimen = r#"let
  ownerName = "stdlib/lib/gate/x.px";
  other = import ./helper.px;
  mk = a: b: { inherit a b; };
  graph = {
    types = [ "Num" ];
    externs = [ { name = "py.add"; } ];
    nodes = [ { name = "n1"; } ];
    edges = [ { from = "a"; to = "b"; } ];
  };
in graph"#;
    let fired = vec![
      extract_path_signals("tests/foo.rs"),
      extract_code_signals("import os\nprint('hi')\n", "python"),
      extract_code_signals(pnix_specimen, "pnix"),
    ]
    .into_iter()
    .flatten()
    .collect::<std::collections::HashSet<_>>();
    for c in fired {
      assert!(
        STRUCTURAL_CUES.iter().any(|s| *s == c),
        "extractor fired cue `{c}` not in STRUCTURAL_CUES"
      );
    }
  }
}
