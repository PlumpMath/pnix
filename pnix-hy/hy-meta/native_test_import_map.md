# Native Test Import Map

This map records the upstream Hy native-test surface that is represented in
`hy-meta/native_subset_test.py` and related proof commands.

## Ported In The Current Kernel Lane

| Upstream file | Imported coverage |
| --- | --- |
| `tests/native_tests/comprehensions.hy` | Comprehension types, empty comprehensions, `for []`, `:do`/`:setv`, `:if`, `#*`/`#**`, `setx`, global/nonlocal behavior, multidimensional `for` break/continue, async comprehensions, async `for` with `else`, and generator send protocol for final `#*`. |
| `tests/native_tests/functions.hy` | Lambda lists, destructuring, annotations, named/anonymous async functions and async generators, yield/yield-from, final generator return values, return behavior, name/doc metadata, and syntax errors are covered for the current kernel lane. |
| `tests/native_tests/let.hy` | Sequential/rebinding/destructuring let bindings, f-strings, comprehensions, quasiquote, exceptions, with, mutation, break/continue/yield/return, imports, nested functions/classes, dotted targets, default-argument evaluation counts, nonlocal/global, macro definitions, and body-binding leakage/hiding are covered. |
| `tests/native_tests/defclass.hy` | Bare classes, inheritance, attributes, function attributes, dynamic bases, no function leakage, docstrings, macro-expanded class bodies, body side effects, metaclass `__prepare__`, and `__init_subclass__` keyword behavior. |
| `tests/native_tests/decorators.hy` | Function/class decorators, stacked decorators, decorator/default/body evaluation order, and async function decorators. |
| `tests/native_tests/operators.hy` | Arithmetic, bitwise, matrix multiply, comparisons, identity, membership, boolean operators, operator shadowing, arity errors, unpacked operands, and augmented assignment grouping. |
| `tests/native_tests/dots.hy` | Attribute chains, method shortcuts, keyword receivers, call/index chains, assignment through dot chains, multidot macro/function expansion, and malformed dot syntax. |
| `tests/native_tests/setv.hy`, `setx.hy`, `del.hy`, `unpack.hy` | Assignment returns, unpacking, `setx` scopes, `setv :chain`, `del`, invalid targets, target evaluation order, call/literal unpacking, and dfor mapping unpacking. |
| `tests/native_tests/match.hy` | Literal, capture, wildcard, value, OR, AS, sequence, mapping, class, keyword, dotted constructor, guard, failure, and side-effect-order cases. |
| `tests/native_tests/model_patterns.hy` | Macro model-pattern parsing for `do-until`, `loop`, and whole/skip parsers. |
| `tests/native_tests/strings.hy` | Docstrings, module docstrings, Unicode string length, f-string conversion/spec/debug/evaluation order, bracket f-strings, delayed quoted f-strings, string model bracket metadata, and f-string repr round trips. |
| `tests/native_tests/hy_repr.hy` | `hy.repr` round trips, non-round-trip model promotion, dict views, datetime/date, collections, model constructors, self references, custom repr registration, placeholders, and fallback repr. |
| `tests/native_tests/mangling.hy` | Hyphen/underscore behavior, special forms, operators, keywords, Unicode, non-ASCII mangle/unmangle, and PEP 3131/NFKC normalization cases. |
| `tests/test_models.py` | Symbol/keyword/model construction behavior relevant to the kernel lane, model equality, list operations, invalid bracket model construction, recursive model detection, and repr/eval behavior. |
| `tests/native_tests/hy_eval.hy` | `hy.eval` core values, globals/locals/module combinations, macros during eval, extra macro tables, explicit filenames, failure cleanup, and namespace cleanup. |
| `tests/native_tests/hy_misc.hy` | `hy.gensym`, `hy.read`, `hy.read-many`, `hy.I`, `hy.R`, macroexpand, and macroexpand-1 behavior. |
| `tests/native_tests/hy_inspect.hy` | Module source, loader source, comments, docstrings, source files, and pydoc rendering that are applicable to the current source-location granularity. |
| `tests/compilers/test_ast.py` | Focused compile-success and compile-failure coverage for the direct-kernel lane, including control forms, declarations, class/function syntax, imports/requires, dot chains, dot unpacking rejection, placeholder special-form rejection, `pragma` validation, reader-timed `:bracketed-templates`, inline `py`/`pys`, module prelude opt-out, f-string conversions, and future-import ordering. |
| `tests/compilers/test_compiler.py` | Bare-name branch preservation, generator final-return AST shape, and boolop generated-Python shape parity for compact short-circuit lowering and nested logic preservation. |
| `tests/test_positions.py` | Focused source-position coverage for direct-kernel top-level/generated statement roots, including macro-expanded statement forms. |
| Supported direct-kernel syntax errors | Representative compile-error message text for direct-kernel-owned syntax boundaries is locked in focused checks. |

## Partially Ported

| Upstream file | Current status |
| --- | --- |
| `tests/native_tests/macros.hy`, `macros_first_class.hy`, `macros_local.hy`, `reader_macros.hy` | Macro definitions, phase behavior, first-class macros, local/global macro tables, reader macro delegation, failure cleanup, and core-shadow warnings are covered. Broad upstream macro tooling remains delegated to upstream Hy. |
| `tests/native_tests/import.hy` | Direct imports, requires, relative import/require, `hy.I`, `hy.R`, import hook module cache behavior, failed import cleanup, star exports, runpy, and reload are covered. Full CLI import behavior is outside this lane. |
| `tests/native_tests/with.hy`, `try.hy`, `conditional.hy`, `break_continue.hy`, `nonlocal.hy`, `do.hy` | Native statement/expression lowering cases used by the kernel lane are covered. Remaining upstream tests that only exercise general upstream compiler behavior are not tracked as blockers here. |

## Skipped By Version

| Upstream file | Reason |
| --- | --- |
| `tests/native_tests/tstrings.hy` | Python 3.14+ template strings are deliberately out of scope for the current direct-kernel lane until `TemplateStr`/`Interpolation` lowering is implemented. |
| Python 3.12+ `:tp` cases in `functions.hy`, `defclass.hy`, and `deftype.hy` | Type-parameter syntax is explicitly gated with a kernel `SyntaxError`; 3.12/3.13 support remains outside the active 3.11/3.14 proof targets. |

## Skipped As Upstream Tooling

| Upstream file | Reason |
| --- | --- |
| `tests/native_tests/repl.hy` | REPL behavior is outside direct-kernel ownership. |
| `tests/test_bin.py` | Full `hy`, stdin, `-c`, interactive modes, shebangs, `hyc`, `hy2py`, startup files, output buffering, and full CLI traceback rendering are outside direct-kernel ownership. The owned `hy-meta/bootstrap.py run`, `py`, `kernel-run`, and `kernel-py` commands are covered by smoke. |

## Still Failing

No current native-test import-map entry is intentionally tracked as failing.
New failures should be added here with the exact upstream file/function and the
reason they are not yet ported.
