#!/usr/bin/env python3
"""Focused native-style checks for the Hy-written kernel subset."""

from __future__ import annotations

import ast
import sys
import builtins
import warnings
from pathlib import Path
from types import ModuleType

from bootstrap import KERNEL_PATH, bootstrap_stage3_chain

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

_STAGE3_CHAIN: tuple[ModuleType, ModuleType, ModuleType] | None = None
_DEFAULT_KERNEL: ModuleType | None = None


def stage3_chain() -> tuple[ModuleType, ModuleType, ModuleType]:
    global _STAGE3_CHAIN
    if _STAGE3_CHAIN is None:
        _STAGE3_CHAIN = bootstrap_stage3_chain()
    return _STAGE3_CHAIN


def make_kernel(module_name: str = "hy_meta_native_subset.kernel"):
    global _DEFAULT_KERNEL
    if module_name == "hy_meta_native_subset.kernel" and _DEFAULT_KERNEL is not None:
        return _DEFAULT_KERNEL
    _stage2, _stage2_prime, stage3 = stage3_chain()
    kernel = stage3.load_hy_file(KERNEL_PATH, module_name)
    if module_name == "hy_meta_native_subset.kernel":
        _DEFAULT_KERNEL = kernel
    return kernel


def make_kernel_evaluator(module_name: str = "hy_meta_native_subset.kernel"):
    kernel = make_kernel(module_name)

    def evaluate(source: str):
        return kernel.eval_source(source, None, "<hy-meta:native-subset>")

    return evaluate


def eval_kernel(source: str):
    return make_kernel_evaluator()(source)


def eval_kernel_raises(
    source: str,
    exc_name: str,
    evaluate=eval_kernel,
) -> Exception:
    try:
        evaluate(source)
    except Exception as exc:
        assert exc.__class__.__name__ == exc_name
        return exc
    raise AssertionError(f"expected {exc_name}")


def test_compiler_ast_focused_cases() -> None:
    kernel = make_kernel("hy_meta_native_subset.compiler_ast")
    filename = "<hy-meta:native-compile>"

    def compile_kernel(source: str):
        tree = kernel.compile_source_to_module(source, filename)
        compile(tree, filename, "exec")
        return tree

    def cant_compile(source: str) -> Exception:
        try:
            compile_kernel(source)
        except Exception as exc:
            assert exc.__class__.__name__ in {
                "HyLanguageError",
                "HyMacroExpansionError",
                "LexException",
                "PrematureEndOfInput",
                "SyntaxError",
            }
            assert str(exc)
            return exc
        raise AssertionError(f"expected compile failure for {source!r}")

    for source in [
        "(.meth obj #* args az)",
        "(.meth obj #** kwargs az)",
        "(.meth #** kwargs obj)",
        "(if foo bar baz)",
        "(do)",
        "(do 1)",
        "(raise)",
        "(raise Exception)",
        "(raise Exception :from NameError)",
        "(try 1 (except []) (else 1))",
        "(try 1 (finally 1))",
        "(try 1 (except []) (finally 1))",
        "(try 1 (except [Exception]) (else 1) (finally 1))",
        "(try 1 (except* [Exception]) (else 1) (finally 1))",
        "(assert 1)",
        '(assert 1 "Assert label")',
        "(global a)",
        "(defclass A)",
        "(defclass A [])",
        "(defclass A [] None 42)",
        "(defclass A [:metaclass type])",
        "(defclass A [:b c])",
        "(fn [])",
        "(fn [] 1)",
        "(defmacro foo [] (try None (except [] None)) `())",
        "(import a-b.c)",
        "(import x [y])",
        "(import __future__ [annotations]) (import sys) (setv some [1 2])",
        "(pragma)",
        "(pragma :warn-on-core-shadow True)",
        '(pragma :hy "1")',
        '(pragma :hy "1.3.0")',
        "(pragma :bracketed-templates True)",
        '(py "1 + 1")',
        '(py "  1 + 1  ")',
        '(pys "x = 41\\nx += 1")',
        '(pys "if 1:\\n  2")',
        "(get x y)",
        "(cut x)",
        "(cut x y)",
        "(cut x y z)",
        "(cut x y z t)",
        "(for [])",
        "(while foo bar)",
        "(while foo bar (else baz))",
        "(for [a [1 2]] (print a))",
        "(while 1 (break))",
        "(while 1 (continue))",
        'f"hello {(+ 1 1) !r} world"',
        'f"hello {(+ 1 1) !s} world"',
        'f"hello {(+ 1 1) !a} world"',
    ]:
        compile_kernel(source)

    import_tree = compile_kernel("(import a-b.c)")
    import_nodes = [stmt for stmt in import_tree.body if isinstance(stmt, ast.Import)]
    assert any(
        alias.name == "a_b.c" and alias.asname is None
        for node in import_nodes
        for alias in node.names
    )

    future_tree = compile_kernel(
        "(import __future__ [annotations]) (import sys) (setv some [1 2])"
    )
    assert isinstance(future_tree.body[0], ast.ImportFrom)
    assert future_tree.body[0].module == "__future__"

    prelude_tree = kernel.compile_source_to_module("", filename, None, None, None, True)
    assert isinstance(prelude_tree.body[0], ast.Import)
    assert prelude_tree.body[0].names[0].name == "hy"

    no_prelude_tree = kernel.compile_source_to_module(
        "(+ 1 1)", filename, None, None, None, False
    )
    assert not any(
        isinstance(stmt, ast.Import)
        and any(alias.name == "hy" for alias in stmt.names)
        for stmt in no_prelude_tree.body
    )

    positioned_tree = kernel.compile_source_to_module(
        "\n(+ 1 1)", filename, None, None, None, False
    )
    assert isinstance(positioned_tree.body[-1], ast.Expr)
    assert positioned_tree.body[-1].lineno == 2

    macro_positioned_tree = kernel.compile_source_to_module(
        "\n(defmacro m [] '(do (raise)))\n(m)",
        filename,
        None,
        None,
        None,
        False,
    )
    assert isinstance(macro_positioned_tree.body[-1], ast.Raise)
    assert macro_positioned_tree.body[-1].lineno == 3

    bare_name_tree = kernel.compile_source_to_module(
        "(do a b c)", filename, None, None, None, False
    )
    assert [type(node).__name__ for node in bare_name_tree.body] == [
        "Expr",
        "Expr",
        "Expr",
    ]
    assert [node.value.id for node in bare_name_tree.body] == ["a", "b", "c"]

    generator_fn_tree = kernel.compile_source_to_module(
        "(fn [] (yield 2) (+ 1 1))", filename, None, None, None, False
    )
    generator_fn = generator_fn_tree.body[0]
    assert isinstance(generator_fn, ast.FunctionDef)
    assert isinstance(generator_fn.body[0], ast.Expr)
    assert isinstance(generator_fn.body[0].value, ast.Yield)
    assert isinstance(generator_fn.body[1], ast.Return)
    assert isinstance(generator_fn.body[1].value, ast.BinOp)

    py_tree = compile_kernel('(py "1 + 1")')
    assert ast.dump(py_tree.body[-1].value) == ast.dump(
        ast.parse("(1 + 1\n)", mode="eval").body
    )

    pys_tree = compile_kernel('(pys "x = 41\\nx += 1")')
    assert any(isinstance(stmt, ast.AugAssign) for stmt in pys_tree.body)

    bool_tree = compile_kernel('(and 1 2.0 True "hi" 5)')
    bool_value = bool_tree.body[-1].value
    assert isinstance(bool_value, ast.BoolOp)
    assert len(bool_value.values) == 5

    lambda_tree = compile_kernel("(fn [x] (* x x))")
    assert isinstance(lambda_tree.body[-1].value, ast.Lambda)

    for source in [
        "(.meth #* args az)",
        "(. foo #* bar baz)",
        "(. foo #** bar baz)",
        "(. foo [])",
        "(. foo [1 2])",
        "(. foo (1))",
        "(setv (. foo (bar)) 1)",
        "(if)",
        "(if foobar)",
        "(if 1 2 3 4 5)",
        "(while)",
        "(raise Exception Exception)",
        "(try (do) (else 1) (else 2))",
        "(try 1 (else 1) (except []))",
        "(try 1 (finally 1) (except []))",
        "(try 1 (except []) (finally 1) (else 1))",
        "(try 1 (except* [Exception]) (except [Exception]))",
        "(try 1 (except [Exception]) (except* [Exception]))",
        "(try 1 (except))",
        "(try 1 (except 1))",
        "(try 1 (except [1 3]))",
        "(try 1 (except [(f) [IOError ValueError]]))",
        "(try 1 (except [x [FooBar] BarBar]))",
        "(assert)",
        "(assert 1 2 3)",
        "(assert 1 [1 2] 3)",
        "(global (foo))",
        "(nonlocal (foo))",
        "(defclass)",
        "(defclass A None)",
        "(defclass A None None)",
        "(fn)",
        "(fn ())",
        "(fn (x) 1)",
        '(fn "foo")',
        "(unquote)",
        "(unquote-splice)",
        "(unquote_splice)",
        "(except)",
        "(except*)",
        "(hyx_exceptXasteriskX)",
        "(pragma :native-code True)",
        "(pragma :hy 1)",
        '(pragma :hy "1.a")',
        '(pragma :hy "1..0")',
        '(pragma :hy "-1")',
        '(pragma :hy "999.0")',
        "(py)",
        "(py a)",
        '(py "foo" a)',
        '(py "1 +")',
        '(py "if 1:\\n  2")',
        '(pys "if 1\\n  2")',
        "(yield 1 2)",
        "(import spam [foo.bar])",
        "(require spam [foo.bar])",
        "(get)",
        "(get 1)",
        "(cut)",
        "(cut 1 2 3 4 5)",
        "(for)",
        "(for [x])",
        "(with)",
        "(with [])",
        "(with [] (pass))",
        "(while 1 (break 1))",
        "(while 1 (continue 1))",
        'f"hello {(+ 1 1) !q} world"',
    ]:
        cant_compile(source)


def test_lambda_lists() -> None:
    assert (
        eval_kernel(
            """
            (import asyncio)
            (defn posonly [x / y] (+ x y))
            (defn kwonly [* required [bonus 3]] (+ required bonus))
            (defn annotated-pair [#^ tuple #(x y)] (+ x y))
            (defn kwonly-pair-default [* [#(x y) [20 22]]] (+ x y))
            (defn kwonly-pair-required [* #(x y)] (+ x y))
            (setv annotated-fn (fn [#^ tuple #(x y)] (+ x y)))
            (setv async-fn (fn :async [x]
                              (await (asyncio.sleep 0))
                              (+ x 2)))
            (setv posonly-rejected False)
            (try
              (posonly :x 10 :y 32)
              (except [TypeError]
                (setv posonly-rejected True)))
            (setv kwonly-rejected False)
            (try
              (kwonly 39)
              (except [TypeError]
                (setv kwonly-rejected True)))
            (and
              (= (posonly 10 :y 32) 42)
              (= (kwonly :required 39) 42)
              (= (annotated-pair [20 22]) 42)
              (= (kwonly-pair-default) 42)
              (= (kwonly-pair-required :__hy_meta_arg_0 [20 22]) 42)
              (= (annotated-fn [10 32]) 42)
              (= (asyncio.run (async-fn 40)) 42)
              posonly-rejected
              kwonly-rejected)
            """
        )
        is True
    )


def test_native_annotation_cases() -> None:
    assert (
        eval_kernel(
            """
            (import typing [List get-type-hints])
            (defclass AnnotationContainer []
              (setv #^ int x 1 y 2)
              (#^ bool z))
            (defn #^ int annotated [#^ (get List int) p1
                                    p2
                                    #^ str p3
                                    #^ str [o1 None]
                                    #^ int [o2 0]
                                    #^ str #* rest
                                    #^ str k1
                                    #^ int [k2 0]
                                    #^ bool #** kwargs]
              42)
            (setv class-hints (get-type-hints AnnotationContainer))
            (setv fn-hints (get-type-hints annotated))
            [(is (get class-hints "x") int)
             (is (get class-hints "z") bool)
             (= (get fn-hints "p1") (get List int))
             (is (get fn-hints "p3") str)
             (is (get fn-hints "o1") str)
             (is (get fn-hints "o2") int)
             (is (get fn-hints "rest") str)
             (is (get fn-hints "k1") str)
             (is (get fn-hints "k2") int)
             (is (get fn-hints "kwargs") bool)
             (is (get fn-hints "return") int)]
            """
        )
        == [True] * 11
    )


def test_native_type_parameter_version_gate_cases() -> None:
    evaluate = make_kernel_evaluator("hy_meta_native_subset.type_parameter_gate")
    for source in [
        "(defn :tp [T] foo [] 1)",
        "(defn :async :tp [T] foo [] 1)",
        "(setv foo (fn :tp [T] [x] x))",
        "(setv foo (fn :async :tp [T] [x] x))",
        "(defclass :tp [T] C)",
        "(deftype Foo int)",
        "(deftype :tp [T] Foo int)",
        "(setv alias (deftype Foo int))",
    ]:
        exc = eval_kernel_raises(source, "SyntaxError", evaluate)
        assert "type parameters are outside the current direct-kernel lane" in str(exc)


def test_native_illegal_binding_cases() -> None:
    forms = [
        "(setv (do 1 2) 1)",
        "(setv 1 1)",
        "(setv {1 2} 1)",
        "(del 1 1)",
        "(setv None 1)",
        "(setv False 1)",
        "(setv True 1)",
        "(defn None [] 1)",
        "(defn True [] 1)",
        "(defn f [True] 1)",
        "(for [True [1 2 3]] True)",
        "(lfor True [1 2 3] True)",
        "(lfor :setv True 1 True)",
        "(with [True 1] True)",
        "(try 1 (except [True AssertionError] 2))",
        "(defclass True [])",
    ]
    for form in forms:
        try:
            eval_kernel(form)
        except SyntaxError:
            continue
        raise AssertionError(f"illegal binding form compiled: {form}")


def test_statement_fn_closure() -> None:
    assert (
        eval_kernel(
            """
            (setv make-adder
                  (fn [base]
                    (fn [x]
                      (setv y (+ base x))
                      y)))
            (setv add30 (make-adder 30))
            (add30 12)
            """
        )
        == 42
    )


def test_module_docstring() -> None:
    assert (
        eval_kernel(
            """
            "native subset module doc"
            (setv x 42)
            [__doc__ x]
            """
        )
        == ["native subset module doc", 42]
    )
    assert (
        eval_kernel(
            """
            "native subset future doc"
            (import __future__ [annotations])
            [__doc__ 42]
            """
        )
        == ["native subset future doc", 42]
    )
    assert (
        eval_kernel(
            """
            (defn f [] "docstring" 5)
            (defn f3 [] "not a docstring")
            (defclass C []
              "class docstring"
              (setv value 42))
            [(. f __doc__) (f) (. f3 __doc__) (f3) (. C __doc__) (. C value)]
            """
        )
        == ["docstring", 5, None, "not a docstring", "class docstring", 42]
    )


def test_native_fstring_cases() -> None:
    assert (
        eval_kernel(
            """
            (setv p "xyzzy")
            (setv foo "bar")
            (setv p:9 "other")
            (setv !r "bar")
            (setv value 12.34)
            (setv width 10)
            (setv precision 4)
            (setv events [])
            (defclass C [object]
              (defn __format__ [self format-spec]
                (+ "C[" format-spec "]")))
            (setv pi 3.141593)
            (setv fill "_")
            (setv explicit-format-error False)
            (try
              f"{pi =!s:.3f}"
              (except [ValueError]
                (setv explicit-format-error True)))
            (setv nested-debug-error False)
            (try
              f"{pi =:{fill =}^{width =}.2f}"
              (except [ValueError]
                (setv nested-debug-error True)))
            [f"hello world"
             f"hello {(+ 1 1)} world"
             f"a{1}{2}b"
             f"ab{{cde"
             f"ab{{cde}}}}fg{{{{{{"
             f"ab{{{(+ 1 1)}}}"
             f"a{(.upper (+ "g" "k"))}z"
             f"h{p}j"
             f"a{(do (setv floop 4) (* floop 2))}z"
             f"a{p !r}"
             f"a{p :9}"
             f"a{p:9}"
             f"a{p !r :9}"
             f"a{!r}"
             f"a{!r !r}"
             f"{2 :{(+ 2 2)}}"
             f"result: {value :{width}.{precision}}"
             f"{foo =}"
             f"xyz{  foo = }"
             f"{ foo = !s}"
             #[f[a{p !r :9}]f]
             #[f-string[result: {value :{width}.{precision}}]f-string]
             #[f[{{escaped braces}} \n {"not escaped"}]f]
             #[f["{0}"]f]
             f"{(C) :  {(str (+ 1 1)) !r :x<5}}"
             f"{pi = :{fill}^8.2f}"
             f"{(do (.append events "value") 2) :{(do (.append events "spec") 4)}}"
             explicit-format-error
             nested-debug-error
             (len "ℵℵℵ♥♥♥\\t♥♥\\r\\n")
             floop
             events]
            """
        )
        == [
            "hello world",
            "hello 2 world",
            "a12b",
            "ab{cde",
            "ab{cde}}fg{{{",
            "ab{2}",
            "aGKz",
            "hxyzzyj",
            "a8z",
            "a'xyzzy'",
            "axyzzy    ",
            "aother",
            "a'xyzzy'  ",
            "abar",
            "a'bar'",
            "   2",
            "result:      12.34",
            "foo ='bar'",
            "xyz  foo = 'bar'",
            " foo = bar",
            "a'xyzzy'  ",
            "result:      12.34",
            "{escaped braces} \n not escaped",
            '"0"',
            "C[  '2'xx]",
            "pi = __3.14__",
            "   2",
            True,
            True,
            11,
            4,
            ["value", "spec"],
        ]
    )


def test_native_quoted_string_model_cases() -> None:
    assert (
        eval_kernel(
            """
            (setv bracketed '#[my delim[hello world]my delim])
            (setv empty-bracket '#[[squid]])
            (setv plain '"squid")
            (setv invalid-string-brackets False)
            (try
              (hy.models.String "]foo]" :brackets "foo")
              (except [ValueError]
                (setv invalid-string-brackets True)))
            (setv invalid-fstring-brackets False)
            (try
              (hy.models.FString [(hy.models.String "hello")
                                  (hy.models.String "world ]foo]")]
                                 :brackets "foo")
              (except [ValueError]
                (setv invalid-fstring-brackets True)))
            (setv quoted 'f"hello {world}")
            (setv missing-world False)
            (try
              (hy.eval quoted (globals) (globals) "hy")
              (except [NameError]
                (setv missing-world True)))
            (setv world "goodbye")
            (setv p "xyzzy")
            (setv quoted-spec 'f"a{p !r:9}")
            (setv repr-roundtrip [])
            (for [orig ['f"hello {(+ 1 1)} world"
                        'f"a{p !r:9}"
                        'f"{ foo = !s}"]]
              (setv new (eval (repr orig)))
              (.append repr-roundtrip
                       [(= (len new) (len orig))
                        (list (map (fn [item]
                                     (getattr item "conversion" None))
                                   new))
                        (= new orig)]))
            [(. bracketed brackets)
             (. empty-bracket brackets)
             (is (. plain brackets) None)
             (repr '"foo")
             (repr '#[[foo]])
             (repr '#[xx[foo]xx])
             (repr '#[xx[]xx])
             invalid-string-brackets
             invalid-fstring-brackets
             (isinstance quoted hy.models.FString)
             missing-world
             (hy.eval quoted (globals) (globals) "hy")
             (getattr (get quoted 1) "expression")
             (getattr (get quoted-spec 1) "conversion")
             (str (get (get quoted-spec 1) 1))
             (hy.eval quoted-spec (globals) (globals) "hy")
             repr-roundtrip]
            """
        )
        == [
            "my delim",
            "",
            True,
            "hy.models.String('foo')",
            "hy.models.String('foo', brackets='')",
            "hy.models.String('foo', brackets='xx')",
            "hy.models.String('', brackets='xx')",
            True,
            True,
            True,
            True,
            "hello goodbye",
            "world",
            "r",
            "9",
            "a'xyzzy'  ",
            [
                [True, [None, None, None], True],
                [True, [None, "r"], True],
                [True, [None, "s"], True],
            ],
        ]
    )


def test_native_reader_timed_pragma_cases() -> None:
    assert (
        eval_kernel(
            """
            (setv plain '#[t[hello {(+ 1 1)}]t])
            (pragma :bracketed-templates True)
            (setv templ '#[t[hello {(+ 1 1)}]t])
            [(isinstance plain hy.models.String)
             (getattr plain "brackets")
             (isinstance templ hy.models.FString)
             (getattr templ "brackets")
             (getattr templ "is_tstring")
             (getattr (get templ 1) "is_tstring")
             (getattr (get templ 1) "expression")]
            """
        )
        == [True, "t", True, "t", True, True, "(+ 1 1)"]
    )


def test_native_tstring_compile_gate_cases() -> None:
    evaluate = make_kernel_evaluator("hy_meta_native_subset.tstring_gate")
    sources = [
        """
        (pragma :bracketed-templates True)
        #[t[hello {(+ 1 1)}]t]
        """,
    ]
    if sys.version_info >= (3, 14):
        sources.append('t"hello{(+ 1 1)}"')
    for source in sources:
        exc = eval_kernel_raises(source, "SyntaxError", evaluate)
        assert "template strings are outside the current direct-kernel lane" in str(exc)


def test_native_supported_compile_error_messages() -> None:
    kernel = make_kernel("hy_meta_native_subset.error_messages")
    filename = "<hy-meta:native-errors>"

    def compile_error(source: str) -> str:
        try:
            tree = kernel.compile_source_to_module(source, filename)
            compile(tree, filename, "exec")
        except Exception as exc:
            return str(exc)
        raise AssertionError(f"expected compile failure for {source!r}")

    for source, expected in [
        ("(py)", "kernel py needs exactly one string"),
        ("(py a)", "kernel py needs a string"),
        ("(pragma :native-code True)", "Unknown pragma `:native-code`"),
        ('(pragma :hy "999.0")', "Hy version 999.0 or later required"),
        (
            "(.meth #* args az)",
            "kernel method call shortcut receiver cannot be an unpacking form",
        ),
        ("(. foo #* bar baz)", "kernel dot chain parts cannot be unpacking forms"),
        ("(unquote)", "`unquote` is not allowed here"),
        ("(except*)", "`except*` is not allowed here"),
    ]:
        assert expected in compile_error(source)


def test_native_hy_repr_and_model_cases() -> None:
    assert (
        eval_kernel(
            """
            (import collections datetime re)
            (setv roundtrip-results [])
            (for [original-val [':mykeyword
                                {"a" 1 "b" 2 "a" 3}
                                '{"a" 1 "b" 2 "a" 3}
                                'f"the answer is {(+ 2 2) = }"
                                'f"the answer is {(+ 2 2) = !r :4}"]]
              (setv evaled (hy.eval (hy.read (hy.repr original-val))))
              (.append roundtrip-results
                       [(= evaled original-val)
                        (is (type evaled) (type original-val))]))
            (setv orig `[a ~5.0])
            (setv reprd (hy.repr orig))
            (setv no-roundtrip-result (hy.eval (hy.read reprd)))
            (setv self-ref-list [1 2 3])
            (setv (get self-ref-list 1) self-ref-list)
            (setv self-ref-dict {1 2 3 [4 5] 6 7})
            (setv (get self-ref-dict 3 1) self-ref-dict)
            (defclass ReprC [object])
            (hy.repr-register ReprC (fn [x] "cuddles"))
            (defclass ReprD [ReprC])
            (defclass ReprContainer [object]
              (defn __init__ [self value]
                (setv (. self value) value)))
            (hy.repr-register ReprContainer
                              :placeholder "(ReprContainer ...)"
                              (fn [x]
                                (+ "(ReprContainer " (hy.repr (. x value)) ")")))
            (setv container (ReprContainer 5))
            (setv (. container value) container)
            (setv container-self-repr (hy.repr container))
            (setv (. container value) [1 container 3])
            (defclass FallbackRepr [object]
              (defn __repr__ [self]
                "fallback"))
            (setv model-self-ref [1 2 3])
            (setv (get model-self-ref 1) model-self-ref)
            (setv model-self-ref-error False)
            (try
              (hy.as-model model-self-ref)
              (except [err hy.errors.HyWrapperError]
                (setv model-self-ref-error
                      (in "Self-referential" (str err)))))
            [roundtrip-results
             reprd
             (is (type (get orig 1)) float)
             (is (type (get no-roundtrip-result 1)) hy.models.Float)
             (hy.repr (.keys {1 2}))
             (hy.repr (.values {1 2}))
             (hy.repr (.items {1 2}))
             (hy.repr (datetime.datetime 2009 1 15 15 27 5 123))
             (hy.repr (datetime.date 2015 11 3))
             (hy.repr (collections.Counter [15 15 15 15]))
             (hy.repr (hy.models.Integer 7))
             (hy.repr (hy.models.String "hello"))
             (hy.repr (hy.models.List [1 2 3]))
             (hy.repr (hy.models.Dict [1 2 3]))
             (hy.repr self-ref-list)
             (hy.repr self-ref-dict)
             (hy.repr (ReprC))
             (not-in "cuddles" (hy.repr (ReprD)))
             container-self-repr
             (hy.repr container)
             (hy.repr (FallbackRepr))
             (= (hy.as-model 0) (hy.models.Integer 0))
             (= (hy.as-model "foo") (hy.models.String "foo"))
             (= (+ (hy.models.List [1 2]) (hy.models.List [3]))
                (hy.models.List [1 2 3]))
             model-self-ref-error]
            """
        )
        == [
            [[True, True]] * 5,
            "'[a 5.0]",
            True,
            True,
            "(dict-keys [1])",
            "(dict-values [2])",
            "(dict-items [#(1 2)])",
            "(datetime.datetime 2009 1 15 15 27 5 123)",
            "(datetime.date 2015 11 3)",
            "(Counter {15 4})",
            "'7",
            "'\"hello\"",
            "'[1 2 3]",
            "'{1 2  3}",
            "[1 [...] 3]",
            "{1 2  3 [4 {...}]  6 7}",
            "cuddles",
            True,
            "(ReprContainer (ReprContainer ...))",
            "(ReprContainer [1 (ReprContainer ...) 3])",
            "fallback",
            True,
            True,
            True,
            True,
        ]
    )


def test_native_quasiquote_cases() -> None:
    assert (
        eval_kernel(
            """
            (setv q '(c d e))
            (setv S hy.models.Symbol)
            (setv side-effects [])
            (setv falsey-splice
                  `(a b ~@q f ~@q ~@0 ~@False ~@None g ~@(when False 1) h))
            (setv single-eval
                  `(x ~@(do (.append side-effects "once") [1 2]) y))
            (setv nested
                  (hy.as-model `(1 `~(+ 1 ~(+ 2 3) ~@None) 4)))
            (setv nested-expected '(1 `~(+ 1 5) 4))
            (setv nested-struct
                  (hy.as-model
                    `(try
                       ~@(lfor i [1 2 3]
                           `(setv ~(S (+ "x" (str i))) (+ "x" (str ~i))))
                       (finally
                         (print "done")))))
            [(= falsey-splice '(a b c d e f c d e g h))
             (len single-eval)
             (str (get single-eval 0))
             (get single-eval 1)
             (get single-eval 2)
             (str (get single-eval 3))
             side-effects
             (= nested nested-expected)
             (= (get nested 1) '`~(+ 1 5))
             (= nested-struct
                '(try
                   (setv x1 (+ "x" (str 1)))
                   (setv x2 (+ "x" (str 2)))
                   (setv x3 (+ "x" (str 3)))
                   (finally
                     (print "done"))))]
            """
        )
        == [True, 4, "x", 1, 2, "y", ["once"], True, True, True]
    )


def test_eval_source_hy_eval_module_context() -> None:
    _stage2, _stage2_prime, stage3 = stage3_chain()
    kernel = stage3.load_hy_file(
        KERNEL_PATH, "hy_meta_native_subset.kernel_eval_source"
    )
    module_name = "hy_meta_native_subset.eval_source_session"
    sys.modules.pop(module_name, None)
    module = ModuleType(module_name)
    try:
        assert (
            kernel.eval_source(
                """
                (setv a 1 b 1 c 1)
                (setv x ```[~a ~~b ~~~c])
                (setv a 2 b 2 c 2)
                (setv x (hy.eval x))
                (setv a 3 b 3 c 3)
                (setv x (hy.eval x))
                (= (hy.as-model x) '[3 2 1])
                """,
                module,
                "<hy-meta:native-subset:eval-source>",
            )
            is True
        )
        assert module_name not in sys.modules

        previous_module = ModuleType(module_name)
        sys.modules[module_name] = previous_module
        replacement_module = ModuleType(module_name)
        assert (
            kernel.eval_source(
                "(hy.eval '(+ 2 3))",
                replacement_module,
                "<hy-meta:native-subset:eval-source>",
            )
            == 5
        )
        assert sys.modules[module_name] is previous_module
    finally:
        sys.modules.pop(module_name, None)


def test_native_hy_eval_argument_cases() -> None:
    assert (
        eval_kernel(
            """
            (import re string)
            (setv x 2)
            (setv payload '(+ x 2))
            (setv outer "O")
            (setv globals-dict {"g1" 1 "g2" 2})
            (setv locals-dict {"l1" 1 "l2" 2})
            (setv eval-basic (= (hy.eval '(+ 1 1)) 2))
            (setv eval-before-set (= (hy.eval '(+ x 2)) 4))
            (setv eval-after-set
                  (do
                    (setv x 4)
                    (= (hy.eval payload) 6)))
            (hy.eval :globals globals-dict :locals locals-dict
                     '(do
                        (global g2 g3)
                        (setv g2 "newg" g3 3 l2 "newl" l3 4)))
            (del (get globals-dict "__builtins__"))
            (setv eval-outer-unchanged
                  (do
                    (hy.eval :globals {"outer" "I"}
                             '(do (global outer) (setv outer "O3")))
                    (= outer "O")))
            [eval-basic
             eval-before-set
             eval-after-set
             (= ((hy.eval '(fn [x] (+ 3 3 x))) 3) 9)
             (is (hy.eval 're) re)
             (is (hy.eval 'False) False)
             (is (hy.eval 'None) None)
             (= (hy.eval '0) 0)
             (= (hy.eval '"") "")
             (= (hy.eval 'b"") b"")
             (= (hy.eval ':) :)
             (= (hy.eval '[]) [])
             (= (hy.eval '#()) #())
             (= (hy.eval '{}) {})
             (= (hy.eval '#{}) #{})
             (= (hy.eval 'digits :module string) "0123456789")
             (= (hy.eval 'digits :module "string") "0123456789")
             (= (hy.eval 'digits :module string :globals {"digits" "boo"})
                "boo")
             eval-outer-unchanged
             (= globals-dict {"g1" 1 "g2" "newg" "g3" 3})
             (= locals-dict {"l1" 1 "l2" "newl" "l3" 4})]
            """
        )
        == [True] * 21
    )


def test_native_hy_eval_upstream_remaining_cases() -> None:
    assert (
        eval_kernel(
            """
            (import traceback)
            (defmacro test-macro []
              '(setv blah "test from here"))
            (defmacro cheese []
              "gorgonzola")
            (setv M "tests.resources.macros")
            (setv ab 15)
            (setv module-macro-local
                  (= (hy.eval '(do (test-macro) blah))
                     "test from here"))
            (setv module-macro-remote
                  (= (hy.eval '(do (test-macro) blah) :module M)
                     1))
            (hy.eval '(defmacro bilb-ono-native [] "creative consulting")
                     :module M)
            (setv module-created-macro
                  (= (hy.eval '(bilb-ono-native) :module M)
                     "creative consulting"))
            (setv module-created-macro-hidden False)
            (try
              (hy.eval '(bilb-ono-native))
              (except [NameError]
                (setv module-created-macro-hidden True)))
            (setv module-loses-current-macro False)
            (try
              (hy.eval '(cheese) :module M)
              (except [NameError]
                (setv module-loses-current-macro True)))
            (hy.eval '(require tests.resources.tlib [qplah]))
            (setv require-inside-eval (= (hy.eval '(qplah 1)) [8 1]))
            (setv extra-macro
                  (= (hy.eval '(chippy a b) :macros
                              {"chippy" (fn [arg1 arg2]
                                           (hy.models.Symbol
                                             (+ (str arg1) (str arg2))))})
                     15))
            (defn local-macro-hidden-probe []
              (setv ab 15)
              (defmacro oh-hungee-local [arg1 arg2]
                (hy.models.Symbol (+ (str arg1) (str arg2))))
              (setv hidden False)
              (try
                (hy.eval '(oh-hungee-local a b))
                (except [NameError]
                  (setv hidden True)))
              hidden)
            (setv local-macro-hidden (local-macro-hidden-probe))
            (defmacro oh-hungee [arg1 arg2]
              (hy.models.Symbol (+ (str arg1) (str arg2))))
            (setv local-macro-passed
                  (= (hy.eval '(oh-hungee a b) :macros _hy_macros)
                     15))
            (setv named-local-macro-passed
                  (= (hy.eval '(oh-hungee a b)
                              :macros {"oh_hungee" (get-macro oh-hungee)})
                     15))
            (setv shadow-global-macro
                  (= (hy.eval '(cheese) :macros {"cheese" (fn [] "cheddar")})
                     "cheddar"))
            (setv shadow-core-macro
                  (= (hy.eval '(+ 1 1) :macros
                              {(hy.mangle "+")
                               (fn [#* args]
                                 (.join "" (gfor x args (str (int x)))))})
                     "11"))
            (setv filename-model (hy.read "(/ 1 0)" :filename "bad_math.hy"))
            (setv filename-trace-ok False)
            (try
              (hy.eval filename-model)
              (except [err ZeroDivisionError]
                (setv filename-trace-ok
                      (in "bad_math.hy"
                          (get (traceback.format-tb err.__traceback__) -1)))))
            (defclass EvalFailureC)
            (setv failure-types [])
            (try
              (hy.eval '(hy.eval))
              (except [TypeError]
                (.append failure-types True)))
            (try
              (hy.eval (EvalFailureC))
              (except [TypeError]
                (.append failure-types True)))
            (try
              (hy.eval 'False [])
              (except [TypeError]
                (.append failure-types True)))
            (try
              (hy.eval 'False {} 1)
              (except [TypeError]
                (.append failure-types True)))
            (setv keep-hy-globals {})
            (exec "import hy" keep-hy-globals)
            (setv no-extra-hy-removal
                  (and (= (hy.eval '(hy.repr [1 2]) keep-hy-globals)
                          "[1 2]")
                       (in "hy" keep-hy-globals)))
            (setv local-d {"a" 1})
            (setv returned-fn (hy.eval '(fn [] (hy.repr "hello"))
                                      :locals local-d))
            (setv returned-code-keeps-hy
                  (and (= local-d {"a" 1})
                       (= (returned-fn) #[["hello"]])))
            (setv s1 (hy.gensym))
            (setv s2 (hy.gensym "xx"))
            (setv s3 (hy.gensym "xx"))
            (setv s4 (hy.gensym "•ab"))
            [module-macro-local
             module-macro-remote
             module-created-macro
             module-created-macro-hidden
             module-loses-current-macro
             (= (hy.eval '(cheese)) "gorgonzola")
             require-inside-eval
             extra-macro
             local-macro-hidden
             local-macro-passed
             named-local-macro-passed
             shadow-global-macro
             shadow-core-macro
             filename-trace-ok
             (= failure-types [True True True True])
             no-extra-hy-removal
             returned-code-keeps-hy
             (isinstance s1 hy.models.Symbol)
             (.startswith s1 "_hy_gensym__")
             (.startswith s2 "_hy_gensym_xx_")
             (!= s2 s3)
             (!= (str s2) (str s3))
             (.startswith s4 "_hy_gensym_XbulletXab_")]
            """
        )
        == [True] * 23
    )


def test_native_keyword_cases() -> None:
    assert (
        eval_kernel(
            """
            (import pickle)
            (defn kwtest [#** kwargs]
              kwargs)
            (defclass KeywordLookup []
              (defn __getitem__ [self key]
                key))
            (setv empty :)
            (setv key :test-keyword)
            (setv lookup :foo)
            [(= :foo :foo)
             (!= :foo :bar)
             (= (get {:foo "bar"} :foo) "bar")
             (= (get {:foo "bar" ":foo" "quux"} :foo) "bar")
             (= (get {:foo "bar" ":foo" "quux"} ":foo") "quux")
             (= empty ':)
             (= (. empty name) "")
             (< :a :b)
             (= (sorted [:b :a :c]) [:a :b :c])
             (= key (pickle.loads
                      (pickle.dumps key :protocol pickle.HIGHEST-PROTOCOL)))
             (= (:foo (dict :foo "test")) "test")
             (= (lookup (dict :foo "test")) "test")
             (= (:foo (dict :a 1) 3) 3)
             (= (:foo (dict :a 1 :foo 5) 3) 5)
             (= (:foo-bar (dict :foo-bar "baz")) "baz")
             (= (:foo-bar (KeywordLookup)) "foo_bar")
             (= (kwtest :key-with-dashes "value")
                {"key_with_dashes" "value"})]
            """
        )
        == [True] * 17
    )


def test_native_mangling_special_form_alias_cases() -> None:
    assert (
        eval_kernel(
            """
            (setv a-b 1
                  -a-_b- 2
                  -_- 3
                  _42 3
                  foo? "nachos"
                  $ "dosh"
                  ♥ "love"
                  ⚘ab "flower"
                  ⚘-⚘ "doubleflower"
                  😂 "emoji"
                  a<b "little"
                  X☠ "treasure"
                  𝔥𝔢𝔩𝔩𝔬 15
                  oﬃce "space"
                  ⅓ .3
                  not-in 5
                  is-not 6
                  left []
                  right [])
            (setv + 7)
            [(= [a-b a_b] [1 1])
             (= [-a-_b- -a--b- -a__b-] [2 2 2])
             (= [-_- -__] [3 3])
             (= [foo? hyx_fooXquestion_markX] ["nachos" "nachos"])
             (= [$ hyx_Xdollar_signX] ["dosh" "dosh"])
             (= [+ hyx_Xplus_signX] [7 7])
             (is (not-in 2 [1 2 3]) False)
             (is (not_in 2 [1 2 3]) False)
             (is (not-in 4 [1 2 3]) True)
             (is (not_in 4 [1 2 3]) True)
             (is (is-not left right) True)
             (is (is_not left right) True)
             (= (hy.mangle "-") "hyx_XhyphenHminusX")
             (= (hy.mangle "__dunder-name__") "__dunder_name__")
             (= (hy.unmangle "hyx_XhyphenHminusX") "-")
             (= (hy.unmangle "__dunder_name__") "__dunder-name__")
             (= _42 3)
             (!= _42 -42)
             (not (in "_hyx_42" (locals)))
             (= (+ ⚘ab ♥) "flowerlove")
             (= (+ hyx_XflowerXab hyx_Xblack_heart_suitX) "flowerlove")
             (= ⚘-⚘ "doubleflower")
             (= hyx_XflowerX_XflowerX "doubleflower")
             (= 😂 "emoji")
             (= hyx_Xface_with_tears_of_joyX "emoji")
             (= a<b "little")
             (= hyx_aXlessHthan_signXb "little")
             (= hyx_Xlatin_capital_letter_xXXskull_and_crossbonesX "treasure")
             (= 𝔥𝔢𝔩𝔩𝔬 15)
             (= hello 15)
             (= oﬃce "space")
             (= office "space")
             (= ⅓ .3)
             (= hyx_Xvulgar_fraction_one_thirdX .3)
             (= (hy.mangle "_﹏a") "__a")
             (= (hy.mangle "﹏a") "_a")
             (= (hy.mangle "foo﹖") "hyx_fooXsmall_question_markX")
             (= (hy.mangle "a－b") "hyx_aXfullwidth_hyphenHminusXb")
             (= (hy.mangle " ") "hyx_XspaceX")
             (= (hy.mangle "\n") "hyx_XUaX")]
            """
        )
        == [True] * 40
    )


def test_native_operator_edge_cases() -> None:
    assert (
        eval_kernel(
            """
            (defclass Posable [object]
              (defn __pos__ [self]
                "called __pos__"))
            (setv starred-pow-single-error False)
            (try
              (** #* [2])
              (except [TypeError]
                (setv starred-pow-single-error True)))
            [(/ 2)
             (/ 8 2 2 2)
             (** 5 4 3)
             (** #* [5 4 3])
             (|)
             (| #* [])
             (| 5)
             (& 5)
             (@ 5)
             (bnot 0b00101111)
             (+ (Posable))
             (+ #* [])
             (* #* [])
             (/ #* [2])
             starred-pow-single-error]
            """
        )
        == [
            0.5,
            1.0,
            542101086242752217003726400434970855712890625,
            542101086242752217003726400434970855712890625,
            0,
            0,
            5,
            5,
            5,
            -48,
            "called __pos__",
            0,
            1,
            0.5,
            True,
        ]
    )
    for source in ["(// 2)", "(% 5)", "(** 2)", "(<< 5)", "(>> 5)", "(^ 5)"]:
        try:
            eval_kernel(source)
        except SyntaxError:
            pass
        else:
            raise AssertionError(f"expected SyntaxError for {source}")


def test_native_operator_upstream_parity_cases() -> None:
    assert (
        eval_kernel(
            """
            (import hy.pyops *)
            (defclass Posable [object]
              (defn __pos__ [self] "called __pos__"))
            (defclass MatBox [object]
              (defn __init__ [self content]
                (setv (. self content) content))
              (defn __matmul__ [self other]
                (MatBox (+ (. self content) (. other content)))))
            (defclass CompareBox [object]
              (defn __init__ [self x]
                (setv (. self x) x))
              (defn __lt__ [self other]
                (. self x)))
            (setv ident (object))
            (setv other (object))
            (setv add-f + and-f and get-f get cut-f cut)
            [(+)
             (+ (Posable))
             (+ 1 2 3 4 5)
             (+ "a" "b" "c")
             (+ ["a"] ["b"] ["c"])
             [(- 1) (- 2 1) (- 2 1 1)]
             [(*) (* 3) (* 2 3 4) (* "ke" 4) (* [1 2 3] 2)]
             [(** 3 2) (= (** 5 4 3 2) (** 5 (** 4 (** 3 2))))]
             [(/ 2) (/ 8 2 2 2)]
             [(// 16 5) (// 8 2 2)]
             [(% 16 5) (% 8 2)
              (% "aa %s bb" 15)
              (% "aa %s bb %s cc" #("X" "Y"))]
             [(. (@ (MatBox "b") (MatBox "c")) content)
              (. (@ (MatBox "d") (MatBox "e") (MatBox "f")) content)]
             [(<< 0b101 2) (<< 0b101 2 3)
              (>> 0b101 2) (>> 0b101000010 2 3)]
             [(& 17) (& 0b0011 0b0101) (& 0b111 0b110 0b100)]
             [(|) (| 17) (| 0b0011 0b0101) (| 0b11100 0b11000 0b10010)]
             [(^ 0b0011 0b0101) (bnot 0b00101111)]
             [(< "hello") (< 1 2 3) (< 1 3 2)
              (< (CompareBox "a") (CompareBox "b") (CompareBox "c"))]
             [(> "hello") (> 3 2 1) (> 1 3 2)]
             [(<= "hello") (<= 1 1) (<= 1 2 2) (<= 3 2 1)]
             [(>= "hello") (>= 1 1) (>= 3 2 1) (>= 1 2 3)]
             [(is ident ident) (is ident other) (is-not ident other)
              (!= 0 1 0) (!= 0 0 1)]
             [(and) (and 17) (and 1 2 3) (and 1 0 3)
              (and "a" 1 True [1]) (and #* [1 2 3])]
             [(or) (or 17) (or 0 0 3) (or "" None 0 False [])]
             [(not "hello") (not 0) (not None)]
             [(in 3 [1 2]) (in 2 [1 2]) (in 2 [1 2] [[1 2] 3])]
             [(not-in 3 [1 2]) (not-in 2 [1 2]) (not-in 3 [1 2] [[2 2] 3])]
             [(get "hello" 1)
              (get [[1 2 3] [4 5 6] [7 8 9]] 1 2)
              (get {"x" {"y" {"z" 12}}} "x" "y" "z")]
             [(cut "abcdef") (cut "abcdef" 3) (cut "abcdef" -2)
              (cut "abcdef" 3 None) (cut "abcdef" -2 None)
              (cut "abcdef" 3 5) (cut "abcdef" 0 None 2)
              (list (cut (range 100) 20 80 13))]
             [(add-f) (add-f 1 2 3)
              (and-f) (and-f 1 0 3)
              (get-f "hello" 1)
              (cut-f "abcdef" 3 5)]]
            """
        )
        == [
            0,
            "called __pos__",
            15,
            "abc",
            ["a", "b", "c"],
            [-1, 1, 0],
            [1, 3, 24, "kekekeke", [1, 2, 3, 1, 2, 3]],
            [9, True],
            [0.5, 1.0],
            [3, 2],
            [1, 0, "aa 15 bb", "aa X bb Y cc"],
            ["bc", "def"],
            [20, 160, 1, 10],
            [17, 1, 4],
            [0, 17, 7, 30],
            [6, -48],
            [True, True, False, "b"],
            [True, True, False],
            [True, True, True, False],
            [True, True, True, False],
            [True, False, True, True, False],
            [True, 17, 3, 0, [1], 3],
            [False, 17, 3, []],
            [False, True, True],
            [False, True, True],
            [True, False, True],
            ["e", 6, 12],
            [
                "abcdef",
                "abc",
                "abcd",
                "def",
                "ef",
                "de",
                "ace",
                [20, 33, 46, 59, 72],
            ],
            [0, 6, True, 0, "e", "de"],
        ]
    )


def test_native_augassign_cases() -> None:
    assert (
        eval_kernel(
            """
            (setv b 2)
            (setv c 3)
            (setv d 4)
            (defclass MatmulBox [object]
              (defn __init__ [self content]
                (setv (. self content) content))
              (defn __matmul__ [self other]
                (MatmulBox (+ (. self content) (. other content)))))
            (setv a 4)
            (+= a b c d)
            (setv add-value a)
            (setv a 4)
            (-= a b c d)
            (setv sub-value a)
            (setv a 4)
            (*= a b c d)
            (setv mul-value a)
            (setv a 4)
            (**= a b c)
            (setv pow-value a)
            (setv a 4)
            (/= a b c d)
            (setv div-value a)
            (setv a 4)
            (//= a b c d)
            (setv floor-value a)
            (setv a 4)
            (<<= a b c d)
            (setv left-shift-value a)
            (setv a 4)
            (>>= a b c d)
            (setv right-shift-value a)
            (setv a 4)
            (&= a b c d)
            (setv and-value a)
            (setv a 4)
            (|= a b c d)
            (setv or-value a)
            (setv a 15)
            (%= a 9)
            (setv mod-value a)
            (setv a 0b1100)
            (^= a 0b1010)
            (setv xor-value a)
            (setv a (MatmulBox "a"))
            (setv b-box (MatmulBox "b"))
            (setv c-box (MatmulBox "c"))
            (setv d-box (MatmulBox "d"))
            (@= a b-box c-box d-box)
            [add-value sub-value mul-value pow-value div-value floor-value
             left-shift-value right-shift-value and-value or-value
             mod-value xor-value (. a content)]
            """
        )
        == [
            13,
            -5,
            96,
            65536,
            1 / 6,
            0,
            2048,
            0,
            0,
            7,
            6,
            6,
            "abcd",
        ]
    )


def test_native_comparison_edge_cases() -> None:
    assert (
        eval_kernel(
            """
            (setv p "a")
            (setv pending-result (= (do (setv p "b") "hello")))
            (setv starred-not-equal-error False)
            (try
              (!= #* [1])
              (except [TypeError]
                (setv starred-not-equal-error True)))
            (setv starred-in-error False)
            (try
              (in #* [1])
              (except [TypeError]
                (setv starred-in-error True)))
            [(= 1)
             (< 1)
             (<= 1)
             (> 1)
             (>= 1)
             (is None)
             pending-result
             p
             (= #* [1])
             (< #* [1])
             starred-not-equal-error
             starred-in-error]
            """
        )
        == [True, True, True, True, True, True, True, "b", True, True, True, True]
    )
    for source in ["(!= 1)", "(is-not None)", "(in 1)", "(not-in 1)", "(=)"]:
        try:
            eval_kernel(source)
        except SyntaxError:
            pass
        else:
            raise AssertionError(f"expected SyntaxError for {source}")


def test_native_chainc_cases() -> None:
    assert (
        eval_kernel(
            """
            (setv seen-false [])
            (setv seen-true [])
            [(chainc 2 = (+ 1 1) = (- 3 1))
             (chainc 2 = (+ 1 1) = (+ 3 1))
             (chainc 2 = 2 > 1)
             (chainc 1 in [1] in [[1] [2 3]] not-in [5])
             (chainc 1 in [1] not-in [[1] [2 3]] not-in [5])
             (chainc (do (.append seen-false "a") 1)
                     <
                     (do (.append seen-false "b") 0)
                     <
                     (do (.append seen-false "c") 3))
             seen-false
             (chainc (do (.append seen-true "a") 1)
                     <
                     (do (.append seen-true "b") 2)
                     <
                     (do (.append seen-true "c") 3))
             seen-true]
            """
        )
        == [
            True,
            False,
            True,
            True,
            False,
            False,
            ["a", "b"],
            True,
            ["a", "b", "c"],
        ]
    )
    for source in ["(chainc 1)", "(chainc 1 =)", "(chainc 1 + 2)"]:
        try:
            eval_kernel(source)
        except SyntaxError:
            pass
        else:
            raise AssertionError(f"expected SyntaxError for {source}")


def test_native_setx_scope_cases() -> None:
    assert (
        eval_kernel(
            """
            (setx y (+ (setx x (+ "a" "b")) "c"))
            (setv loop-values [])
            (for [value [1 2 3]]
              (when (>= (setx seen (+ value 8)) 10)
                (.append loop-values seen)))
            (setv items ["apple" None "banana"])
            (setv filtered
                  (lfor index (range (len items))
                        :if (is-not (setx kept (get items index)) None)
                        kept))
            (defn helper-existing []
              (setv outer 20)
              (lfor n (range 10)
                    :do outer
                    (setx outer n))
              outer)
            (defn helper-new []
              (setv outer 20)
              (lfor n (range 10)
                    :do outer
                    (setx created n))
              created)
            (defn helper-empty []
              (setv outer 2)
              (lfor n (range 0)
                    :do outer
                    (setx never n))
              never)
            (setv empty-error "")
            (try
              (helper-empty)
              (except [err UnboundLocalError]
                (setv empty-error (. err __class__ __name__))))
            [x y loop-values seen filtered kept
             (helper-existing) (helper-new) empty-error]
            """
        )
        == [
            "ab",
            "abc",
            [10, 11],
            11,
            ["apple", "banana"],
            "banana",
            9,
            9,
            "UnboundLocalError",
        ]
    )


def test_native_setv_setx_unpack_upstream_cases() -> None:
    assert (
        eval_kernel(
            """
            (import itertools)
            (setv y 1)
            (setv x y)
            (setv y 12)
            (setv x y)
            (setv y (fn [x] 9))
            (setv x y)
            (setv fn-values [(x y) (y x)])
            (setv perms-expected
                  (set [#(1 3 2) #(3 2 1) #(2 1 3)
                        #(3 1 2) #(1 2 3) #(2 3 1)]))
            (setv foopermutations (fn [x] (itertools.permutations x)))
            (setv permutations- itertools.permutations)
            (setv itertools.permutations (fn [x] 9))
            (setv shadow-values
                  [(itertools.permutations perms-expected)
                   (foopermutations foopermutations)])
            (setv itertools.permutations permutations-)
            (setv restored (= (set (foopermutations [2 3 1]))
                              perms-expected))
            (defn none-value? [x]
              (is x None))
            (import contextlib [nullcontext])
            (setv with-target None
                  try-target None
                  try-except-target None)
            (setv setv-values
                  [(none-value? (setv with-target
                                      (with [value (nullcontext 3)]
                                        value)))
                   with-target
                   (none-value? (setv try-target
                                      (try (/ 1 2)
                                           (except [ZeroDivisionError] "E1"))))
                   try-target
                   (none-value? (setv try-except-target
                                      (try (/ 1 0)
                                           (except [ZeroDivisionError] "E2"))))
                   try-except-target])
            (setv attr-log [])
            (defclass Foo [object]
              (defn __setattr__ [self attr val]
                (.append attr-log [attr val])))
            (setv foo (Foo))
            (setv attr-none (none-value? (setv foo.eggs "ham")))
            (defn fun [[x1 None] [x2 None] [x3 None] [x4 None]
                       [a None] [b None] [c None] [d None]
                       [e None] [f None]]
              [x1 x2 x3 x4 a b c d e f])
            (setv l [1 2 3]
                  d {"a" "x" "b" "y"})
            (setv unpack-call-values
                  [(fun 5 #* l)
                   (fun 5 #** d)
                   (fun 5 #* l #** d)])
            (setv d1 {"a" 1 "b" 2}
                  d2 {"c" 3 "d" 4})
            (setv kw-unpack (fun #** d1 :e "eee" #** d2))
            (setv let-setx
                  (let [x 40 y 13]
                    (setv y (setx x 2))
                    [x y]))
            [fn-values shadow-values restored setv-values
             attr-none attr-log unpack-call-values kw-unpack let-setx]
            """
        )
        == [
            [9, 9],
            [9, 9],
            True,
            [True, 3, True, 0.5, True, "E2"],
            True,
            [["eggs", "ham"]],
            [
                [5, 1, 2, 3, None, None, None, None, None, None],
                [5, None, None, None, "x", "y", None, None, None, None],
                [5, 1, 2, 3, "x", "y", None, None, None, None],
            ],
            [None, None, None, None, 1, 2, 3, 4, "eee", None],
            [2, 2],
        ]
    )


def test_native_assignment_failure_and_order_cases() -> None:
    for source in [
        "(setv 1 2)",
        "(setv [1] [2])",
        "(setv [a #* b #* c] [1 2 3])",
        "(del 1)",
        "(+=)",
        "(+= x)",
        "(+= 1 2)",
        "(+= [a b] [1 2])",
    ]:
        try:
            eval_kernel(source)
        except SyntaxError:
            pass
        else:
            raise AssertionError(f"expected SyntaxError for {source!r}")
    assert (
        eval_kernel(
            """
            (setv events [])
            (setv carrier [[1] [2]])
            (defn target []
              (.append events "target")
              carrier)
            (defn index []
              (.append events "index")
              1)
            (defn value []
              (.append events "value")
              [7])
            (setv (get (target) (index)) (value))
            (setv after-set (list events))
            (del (get (target) (index)))
            (setv after-del (list events))
            (setv numbers [10])
            (defn atarget []
              (.append events "atarget")
              numbers)
            (defn aindex []
              (.append events "aindex")
              0)
            (defn avalue []
              (.append events "avalue")
              5)
            (+= (get (atarget) (aindex)) (avalue))
            (setv [dup dup] [1 2])
            (setv :chain [chain-a chain-a] 5)
            [after-set after-del events carrier numbers dup chain-a]
            """
        )
        == [
            ["value", "target", "index"],
            ["value", "target", "index", "target", "index"],
            [
                "value",
                "target",
                "index",
                "target",
                "index",
                "atarget",
                "aindex",
                "avalue",
            ],
            [[1]],
            [15],
            2,
            5,
        ]
    )


def test_collection_pending_evaluation_order_cases() -> None:
    assert (
        eval_kernel(
            """
            (setv x 1)
            (setv list-order [x (do (setv x 2) x)])
            (setv x 1)
            (setv tuple-order #(x (do (setv x 2) x)))
            (setv x 1)
            (setv set-order #{x (do (setv x 2) x)})
            (setv x 1)
            (setv dict-key-order {(do (setv x 2) x) x})
            (setv x 1)
            (setv dict-value-order {x (do (setv x 2) x)})
            (setv x 1)
            (setv list-unpack-order [x #*(do (setv x 2) [x 3]) x])
            (setv x 1)
            (setv dict-unpack-order {x 10 #**(do (setv x 2) {x 20}) x 30})
            [list-order
             tuple-order
             (= set-order #{2})
             dict-key-order
             dict-value-order
             list-unpack-order
             dict-unpack-order]
            """
        )
        == [
            [2, 2],
            (2, 2),
            True,
            {2: 2},
            {2: 2},
            [2, 2, 3, 2],
            {2: 30},
        ]
    )


def test_native_call_argument_order_cases() -> None:
    assert (
        eval_kernel(
            """
            (setv events [] x 1)
            (defn capture [a b #* rest #** kw]
              [a b rest kw events])
            (capture :k (do (.append events "kw") x)
                     (do (.append events "pos") (setv x 2) x)
                     #*(do (.append events "star") [3])
                     :j (do (.append events "kw2") 4)
                     #**(do (.append events "kwpack") {"m" 5}))
            """
        )
        == [2, 3, (), {"k": 2, "j": 4, "m": 5}, ["kw", "pos", "star", "kw2", "kwpack"]]
    )


def test_comprehension_clauses() -> None:
    assert (
        eval_kernel(
            """
            (setv seen [])
            (setv values
                  (lfor x [1 2 3]
                        :do (.append seen x)
                        :setv y (* x 2)
                        y))
            (setv stopped (list (gfor x [1 2 3 4]
                                      :do (when (= x 4) (break))
                                      x)))
            (setv unpacked-list
                  (lfor xs [[1 2] [3 4] [5]]
                        #* xs))
            (setv unpacked-set
                  (sfor xs [[1 2] [2 3]]
                        #* xs))
            (setv unpacked-generator
                  (list (gfor xs [[1 2] [3 4] [5]]
                               #* xs)))
            (setv unpacked-seen [])
            (setv unpacked-side-effect-generator
                  (list (gfor xs [[1 2] [3 4] [5]]
                               :do (.append unpacked-seen (len xs))
                               #* xs)))
            (defn sub-generator []
              (setv received (yield "first"))
              (yield (+ "received: " (str received)))
              (yield "last"))
            (setv protocol-generator
                  (gfor factory [sub-generator]
                        #* (factory)))
            (setv pending-iter-source "")
            (setv pending-iter-values
                  (lfor x (do
                            (setv pending-iter-source "x")
                            "ab")
                        y (do
                            (+= pending-iter-source "y")
                            "def")
                        (+ x y pending-iter-source)))
            (setv pending-if-source [])
            (setv pending-if-values
                  (lfor x (range 3)
                        :if (do
                              (.append pending-if-source x)
                              (% x 2))
                        x))
            [(sum values)
             seen
             stopped
             unpacked-list
             unpacked-set
             unpacked-generator
             unpacked-side-effect-generator
             unpacked-seen
             [(next protocol-generator)
              (.send protocol-generator "hello")
              (next protocol-generator)]
             [pending-iter-values pending-iter-source]
             [pending-if-values pending-if-source]]
            """
        )
        == [
            12,
            [1, 2, 3],
            [1, 2, 3],
            [1, 2, 3, 4, 5],
            {1, 2, 3},
            [1, 2, 3, 4, 5],
            [1, 2, 3, 4, 5],
            [2, 2, 1],
            ["first", "received: None", "last"],
            [
                ["adxy", "aexy", "afxy", "bdxyy", "bexyy", "bfxyy"],
                "xyy",
            ],
            [[1], [0, 1, 2]],
        ]
    )


def test_async_comprehension_clauses() -> None:
    assert (
        eval_kernel(
            """
            (import asyncio)
            (defclass AValues []
              (defn __init__ [self values]
                (setv (. self values) (iter values)))
              (defn __aiter__ [self]
                self)
              (defn :async __anext__ [self]
                (try
                  (return (next (. self values)))
                  (except [StopIteration]
                    (raise StopAsyncIteration)))))
            (defn :async use []
              (setv seen [])
              (setv xs (lfor :async x (AValues [1 2 3])
                              :do (.append seen x)
                              :setv y (* x 2)
                              y))
              (setv ss (sfor :async x (AValues [1 1 2])
                              :setv y (+ x 10)
                              y))
              (setv dd (dfor :async x (AValues [1 2])
                              :setv y (* x x)
                              x y))
              (setv stopped [])
              (for [:async value (gfor :async x (AValues [1 2 3 4])
                                        :do (when (= x 4) (break))
                                        x)]
                (.append stopped value))
              [(sum xs) seen (len ss) (sum (.values dd)) stopped])
            (asyncio.run (use))
            """
        )
        == [12, [1, 2, 3], 2, 5, [1, 2, 3]]
    )


def test_native_comprehension_upstream_remaining_cases() -> None:
    evaluate = make_kernel_evaluator("hy_meta_native_subset.comprehension_remaining")
    assert (
        evaluate(
            """
            (import types asyncio)
            (setv type-results
                  [(is (type (lfor x "abc" x)) list)
                   (is (type (sfor x "abc" x)) set)
                   (is (type (dfor x "abc" x x)) dict)
                   (is (type (gfor x "abc" x)) types.GeneratorType)
                   (is (type (lfor x "abc" :do (setv y 1) x)) list)
                   (is (type (sfor x "abc" :do (setv y 1) x)) set)
                   (is (type (dfor x "abc" :do (setv y 1) x x)) dict)
                   (is (type (gfor x "abc" :do (setv y 1) x)) types.GeneratorType)])
            (setv for-no-loop [])
            (for [] (.append for-no-loop 1))
            (setv empty-results
                  [(lfor 1) (sfor 1) (list (gfor 1)) (dfor 1 2) for-no-loop])
            (setv dfor-side-seen [])
            (setv dfor-side
                  (dfor x "abc"
                        :do (.append dfor-side-seen x)
                        x (.upper x)))
            (setv break-out "")
            (for [x "abc" y "123"]
              (+= break-out x y)
              (when (= (+ x y) "b2")
                (break)))
            (setv continue-out "")
            (for [c "xyz" d "12"]
              (+= continue-out c d)
              (when (= (+ c d) "y1")
                (continue))
              (+= continue-out "-"))
            (setv else-x 0)
            (for [a [1 2]]
              (setv else-x (+ else-x a))
              (else
                (setv else-x (+ else-x 50))))
            (setv else-empty 0)
            (for [a [1 2]]
              (setv else-empty (+ else-empty a))
              (else))
            (setv real-x 0)
            (setv real-values (lfor real-x [1 2 3] (+ real-x 1)))
            (setv helper-x 0 helper-seen [])
            (setv helper-values
                  (lfor helper-x [4 5 6]
                        :do (.append helper-seen 1)
                        (+ helper-x 1)))
            (setv pending-side [])
            (setv pending-values
                  (lfor x [1 2]
                        :do (.append pending-side ["do" x])
                        :setv y (do
                                   (.append pending-side ["setv" x])
                                   (* x 10))
                        :if (do
                              (.append pending-side ["if" x y])
                              (> y 10))
                        (do
                          (.append pending-side ["value" x y])
                          (+ x y))))
            (defn sub-generator []
              (setv received (yield "first"))
              (yield (+ "received: " (str received)))
              (yield "last"))
            (setv plain-g (gfor factory [sub-generator] #* (factory)))
            (setv stmt-g (gfor factory [sub-generator]
                                :do None
                                #* (factory)))
            (setv protocol-plain [(next plain-g) (.send plain-g "hello") (next plain-g)])
            (setv protocol-stmt [(next stmt-g) (.send stmt-g "hello") (next stmt-g)])
            (setv global_lfor_x 2)
            (defn global-foo []
              (lfor i (range 20)
                    (do
                      (global global_lfor_x)
                      (setv global_lfor_x i))))
            (global-foo)
            (defn nonlocal-bar []
              (setv x 2)
              (defn inner []
                (lfor i (range 20)
                      (do
                        (nonlocal x)
                        (setv x i))))
              (inner)
              x)
            (defn :async numbers []
              (for [i [1 2]]
                (yield i)))
            (defn :async async-use []
              (setv total 0)
              (for [:async value (numbers)]
                (setv total (+ total value))
                (else
                  (setv total (+ total 50))))
              total)
            [type-results
             empty-results
             dfor-side
             dfor-side-seen
             break-out
             continue-out
             else-x
             else-empty
             [real-values real-x]
             [helper-values helper-x helper-seen]
             [pending-values pending-side]
             [protocol-plain protocol-stmt]
             [global_lfor_x (nonlocal-bar)]
             (asyncio.run (async-use))]
            """
        )
        == [
            [True] * 8,
            [[], set(), [], {}, []],
            {"a": "A", "b": "B", "c": "C"},
            ["a", "b", "c"],
            "a1a2a3b1b2c1c2c3",
            "x1-x2-y1y2-z1-z2-",
            53,
            3,
            [[2, 3, 4], 0],
            [[5, 6, 7], 0, [1, 1, 1]],
            [
                [22],
                [
                    ["do", 1],
                    ["setv", 1],
                    ["if", 1, 10],
                    ["do", 2],
                    ["setv", 2],
                    ["if", 2, 20],
                    ["value", 2, 20],
                ],
            ],
            [
                ["first", "received: None", "last"],
                ["first", "received: None", "last"],
            ],
            [19, 19],
            53,
        ]
    )
    for source, needle in [
        (
            """
            (lfor i (range 20)
                  (do
                    (nonlocal x)
                    i))
            """,
            "no binding for nonlocal 'x'",
        ),
        (
            """
            (lfor i (range 20)
                  (do
                    (nonlocal i)
                    (setv i 2)))
            """,
            "assigned to before nonlocal declaration",
        ),
        (
            """
            (lfor i (range 20)
                  (do
                    (global i)
                    (setv i 2)))
            """,
            "assigned to before global declaration",
        ),
    ]:
        exc = eval_kernel_raises(source, "SyntaxError", evaluate)
        assert needle in str(exc)


def test_async_generator_functions() -> None:
    assert (
        eval_kernel(
            """
            (import asyncio)
            (defn :async named []
              (yield 20)
              (yield 22))
            (defn :async collect [agen]
              (setv total 0)
              (for [:async value (agen)]
                (setv total (+ total value)))
              total)
            (setv anonymous (fn :async []
                              (yield 19)
                              (yield 23)))
            [(asyncio.run (collect named))
             (asyncio.run (collect anonymous))]
            """
        )
        == [42, 42]
    )


def test_native_async_function_upstream_remaining_cases() -> None:
    assert (
        eval_kernel(
            """
            (import asyncio)
            (setv events [])
            (defn :async named-coro []
              (await (asyncio.sleep 0))
              (.append events "named-coro")
              [1 2 3])
            (setv anonymous-coro
                  (fn :async []
                    (await (asyncio.sleep 0))
                    (.append events "anonymous-coro")
                    (+ 20 22)))
            (defn :async collect [agen]
              (setv values [])
              (for [:async value (agen)]
                (.append values value)
                (else
                  (.append values "else")))
              values)
            (defn :async named-agen []
              (yield "nope"))
            (setv anonymous-agen
                  (fn :async []
                    (yield "dope!")))
            [(asyncio.run (named-coro))
             (asyncio.run (anonymous-coro))
             (asyncio.run (collect named-agen))
             (asyncio.run (collect anonymous-agen))
             events]
            """
        )
        == [
            [1, 2, 3],
            42,
            ["nope", "else"],
            ["dope!", "else"],
            ["named-coro", "anonymous-coro"],
        ]
    )


def test_yield_from_cases() -> None:
    assert (
        eval_kernel(
            """
            (defn delegated []
              (yield 10)
              (yield :from [20 12]))
            (defn broken []
              (yield 1)
              (yield 2)
              (/ 1 0))
            (defn handled []
              (try
                (yield :from (broken))
                (except [ZeroDivisionError]
                  (yield 39))))
            (defn keyword-values []
              (yield :from)
              (yield :from))
            (defn yield-expression-values []
              (setv received (yield "first"))
              (yield (+ "received: " received))
              (yield "last"))
            (defn yield-from-return-values []
              (setv delegated (yield :from (yield-from-return-subgenerator)))
              (yield delegated))
            (defn yield-from-return-subgenerator []
              (yield 10)
              (return 32))
            (setv expression-generator (yield-expression-values))
            [(list (delegated))
             (sum (handled))
             (list (map str (keyword-values)))
             [(next expression-generator)
              (.send expression-generator "hello")
              (next expression-generator)]
             (list (yield-from-return-values))]
            """
        )
        == [
            [10, 20, 12],
            42,
            [":from", ":from"],
            ["first", "received: hello", "last"],
            [10, 32],
        ]
    )


def test_generator_final_return_cases() -> None:
    assert (
        eval_kernel(
            """
            (defn implicit-return []
              (yield 3)
              "goodbye")
            (defn midtree-return []
              (yield)
              (+ 1 1))
            (defn for-return []
              (for [i (range 3)]
                (yield i))
              (+ 1 2))
            (defn while-return []
              (setv i 0)
              (while (< i 3)
                (yield i)
                (setv i (+ i 1)))
              (+ 2 3))
            (setv hit-finally False)
            (defn try-finally-yield []
              (setv x 1)
              (try
                (yield x)
                (finally
                  (nonlocal hit-finally)
                  (setv hit-finally True))))
            (defn drain [gen]
              (setv values [])
              (setv stop-value None)
              (try
                (while True
                  (.append values (next gen)))
                (except [e StopIteration]
                  (setv stop-value e.value)))
              [values stop-value])
            [(drain (implicit-return))
             (drain (midtree-return))
             (drain (for-return))
             (drain (while-return))
             [(list (try-finally-yield)) hit-finally]]
            """
        )
        == [
            [[3], "goodbye"],
            [[None], 2],
            [[0, 1, 2], 3],
            [[0, 1, 2], 5],
            [[1], True],
        ]
    )


def test_native_function_name_and_lambda_list_edge_cases() -> None:
    assert (
        eval_kernel(
            """
            (defn &hy [] 1)
            (defn phooey [x] (+ x 1))
            (defn mooey [x] (+= x 1) x)
            (defn kwonly [* foo] foo)
            (defn function-of-various-args [a b #* args foo #** kwargs]
              #(a b args foo kwargs))
            [(&hy)
             phooey.__name__
             mooey.__name__
             (try
               (setv x [#* spam] y 1)
               (except [NameError] 42))
             (try
               (kwonly)
               (except [e TypeError]
                 (in "missing 1 required keyword-only argument"
                     (get e.args 0))))
             (function-of-various-args 1 2 3 4 :foo 5 :bar 6 :quux 7)]
            """
        )
        == [
            1,
            "phooey",
            "mooey",
            42,
            True,
            (1, 2, (3, 4), 5, {"bar": 6, "quux": 7}),
        ]
    )


def test_native_function_syntax_error_cases() -> None:
    for source in [
        "(defn f [] (return 1 2)) (f)",
        "(defn f [] (yield 1 2)) (list (f))",
        "(defn f [] (yield :ploopy [1 2])) (list (f))",
        "(import asyncio) (defn :async f [] (yield 1) (return 2)) f",
        "(import asyncio) (setv f (fn :async [] (yield 1) (return 2))) f",
        "(defn bad [x [y 1] z] z)",
        "(fn (x) x)",
    ]:
        try:
            eval_kernel(source)
        except SyntaxError:
            pass
        else:
            raise AssertionError(f"expected SyntaxError for {source!r}")


def test_native_import_cases() -> None:
    assert (
        eval_kernel(
            """
            (import os.path [exists isdir isfile])
            (import sys :as systest)
            (import sys os)
            (import os.path [basename])
            (import os.path :as p)
            (import os.path [basename :as bn])
            (import sys
                    os.path [dirname]
                    os.path :as op
                    os.path [dirname :as dn])
            (import math *)
            (setv star-a (sqrt 1764))
            (import math [*])
            [(exists ".")
             (isdir ".")
             (not (isfile "."))
             (> (len systest.path) 0)
             (= (basename "/some/path") "path")
             (= p.basename basename)
             (= bn basename)
             (= (dirname "/some/path") "/some")
             (= op.dirname dirname)
             (= dn dirname)
             star-a
             (ceil 41.2)]
            """
        )
        == [True] * 10 + [42.0, 42]
    )
    assert (
        eval_kernel(
            """
            (import tests.resources.exports *)
            [(jan)
             (♥)
             (try
               (wayne)
               (except [NameError] "missing-unexported"))]
            """
        )
        == [21, 23, "missing-unexported"]
    )
    eval_kernel_raises("(import :lazy math)", "SyntaxError")
    eval_kernel_raises('(import "sys")', "SyntaxError")


def test_native_require_macro_cases() -> None:
    before_builtins_macros = dict(getattr(builtins, "_hy_macros", {}))
    assert (
        eval_kernel(
            """
            (require builtins)
            (builtins.defn also-not-async [] "ok")
            (require tests.resources.tlib
                     tests.resources.tlib :as TL
                     tests.resources.tlib [qplah]
                     tests.resources.tlib [parald :as pal])
            (require tests.resources.more-test-macros *)
            (require tests.resources.exports *)
            (require tests.resources [macros :as TM exports-none])
            (TM.test-macro)
            [(also-not-async)
             (tests.resources.tlib.qplah 1 2 3)
             (TL.parald 1 2 3)
             (qplah 1 2 3)
             (pal 1 2 3)
             (bairn 1 2 3)
             (cairn 1 2 3)
             (try
               (_dairn)
               (except [NameError] "missing-private"))
             (casey 1 2 3)
             (try
               (brother 1 2 3)
               (except [NameError] "missing-unexported"))
             blah
             (exports-none.cinco 1 2 3)]
            """
        )
        == [
            "ok",
            [8, 1, 2, 3],
            [9, 1, 2, 3],
            [8, 1, 2, 3],
            [9, 1, 2, 3],
            [14, 1, 2, 3],
            [15, 1, 2, 3],
            "missing-private",
            [11, 1, 2, 3],
            "missing-unexported",
            1,
            [5, 1, 2, 3],
        ]
    )
    assert dict(getattr(builtins, "_hy_macros", {})) == before_builtins_macros
    assert (
        eval_kernel(
            """
            (try
              (qplah "x")
              (except [NameError] "not-bound"))
            """
        )
        == "not-bound"
    )


def test_native_recursive_require_star_cases() -> None:
    assert (
        eval_kernel(
            """
            (require tests.resources.macro-with-require *)
            (test-macro)
            blah
            """
        )
        == 1
    )


def test_native_macro_namespace_shadowing_cases() -> None:
    source_result = (
        "This macro was created in tests.resources.macros, "
        "expanded in kernel.module and passed the value "
    )
    assert (
        eval_kernel(
            """
            (defmacro remote-test-macro [x]
              "home")
            (require tests.resources.macro-with-require *)
            (defmacro home-test-macro [x]
              (.format "home {}" (int x)))
            (setv module-name-var "kernel.module")
            [(remote-test-macro 9)
             (test-module-macro 2)
             (test-module-macro-2 3)]
            """
        )
        == [source_result + "9.", source_result + "2.", "home 3"]
    )
    assert (
        eval_kernel(
            """
            (require tests.resources.macro-with-require *)
            (defmacro remote-test-macro [x]
              "after")
            (defmacro home-test-macro [x]
              `(remote-test-macro ~x))
            (setv module-name-var "kernel.module")
            [(remote-test-macro 4)
             (test-module-macro 2)
             (test-module-macro-2 3)]
            """
        )
        == ["after", "after", "after"]
    )


def test_native_first_class_global_macro_cases() -> None:
    assert (
        eval_kernel(
            """
            (import builtins)
            (defmacro global1 []
              "global1 docstring"
              "from global1")
            (require tests.resources.tlib [qplah :as global2])
            (eval-and-compile
              (setv (get _hy_macros "global3")
                    (fn [] "from global3")))
            (eval-and-compile
              (setv (get _hy_macros (hy.mangle "global☘"))
                    (fn []
                      "global☘ docstring"
                      "from global☘")))
            [(is (get-macro when)
                 (get-macro "when")
                 (get builtins._hy_macros "when"))
             (not-in "when" (.keys _hy_macros))
             (= (global1) "from global1")
             (= (global2 1 2) [8 1 2])
             (= (global3) "from global3")
             (= (global☘) "from global☘")
             (= (. (get-macro global1) __doc__) "global1 docstring")
             (= (. (get-macro global☘) __doc__) "global☘ docstring")
             (= (. (get-macro hyx_globalXshamrockX) __doc__)
                "global☘ docstring")]
            """
        )
        == [True] * 9
    )
    assert (
        eval_kernel(
            """
            (defn global4 []
              "from global4 function")
            (setv global4-f1 (global4))
            (defmacro global4 []
              "from global4 macro")
            (setv global4-m (global4))
            (eval-when-compile
              (del (get-macro global4)))
            (setv global4-f2 (global4))
            [global4-f1 global4-m global4-f2 (global4)]
            """
        )
        == [
            "from global4 function",
            "from global4 macro",
            "from global4 function",
            "from global4 function",
        ]
    )
    assert (
        eval_kernel(
            """
            (pragma :warn-on-core-shadow False)
            (defmacro / [a b]
              f"{(int a)}/{(int b)}")
            (setv div1 (/ 1 2))
            (eval-when-compile
              (del (get-macro /)))
            (setv div2 (/ 1 2))
            [div1 div2]
            """
        )
        == ["1/2", 0.5]
    )


def test_native_first_class_local_macro_cases() -> None:
    assert (
        eval_kernel(
            """
            (defn test-local-get []
              (defmacro local1 []
                "local1 doc"
                1)
              (defmacro local2 []
                "local2 outer"
                2)
              (require tests.resources.local-req-example :as LRE)
              (setv outer
                    [(= (. (get-macro local1) __doc__) "local1 doc")
                     (= (. (get-macro local2) __doc__) "local2 outer")
                     (= (. (get-macro LRE.wiz) __doc__) "remote wiz doc")
                     (= (local1) 1)
                     (= (LRE.wiz) "remote wiz")])
              (defn inner []
                (defmacro local2 []
                  "local2 inner"
                  2)
                (defmacro local3 []
                  "local3 doc"
                  3)
                [(= (. (get-macro local2) __doc__) "local2 inner")
                 (= (. (get-macro local3) __doc__) "local3 doc")
                 (= (. (get-macro LRE.wiz) __doc__) "remote wiz doc")
                 (= (local2) 2)
                 (= (local3) 3)
                 (= (LRE.wiz) "remote wiz")])
              [outer (inner)])
            (test-local-get)
            """
        )
        == [[True] * 5, [True] * 6]
    )


def test_native_defmacro_lambda_list_and_docstring_cases() -> None:
    assert (
        eval_kernel(
            """
            (defn f [#* args]
              (+ "f:" (repr args)))
            (defmacro optional-mac [[x None]]
              `(f #* [~x]))
            (defmacro tuple-mac [#* xs]
              xs)
            (defmacro doc-mac []
              (setv mystring "hello world")
              `(fn [] ~mystring (+ 1 2)))
            (setv doc-fn (doc-mac))
            [(optional-mac)
             (tuple-mac 1 2 3)
             (doc-fn)
             doc-fn.__doc__]
            """
        )
        == ["f:(None,)", (1, 2, 3), 3, "hello world"]
    )
    assert (
        eval_kernel(
            """
            (defmacro identify-keywords [#* elts]
              `(lfor
                x ~elts
                (if (isinstance x hy.models.Keyword) "keyword" "other")))
            (identify-keywords 1 "bloo" :foo)
            """
        )
        == ["other", "other", "keyword"]
    )
    for source in [
        "(defmacro f [* a b] 1)",
        "(defmacro f [#** kw] 1)",
        "(defmacro f [a b #* body c] 1)",
        "(defmacro :kw [] 1)",
        "(defmacro foo.bar [] 1)",
    ]:
        try:
            eval_kernel(source)
        except SyntaxError:
            pass
        else:
            raise AssertionError(f"expected SyntaxError for {source!r}")


def test_native_macro_upstream_value_and_phase_cases() -> None:
    assert (
        eval_kernel(
            """
            (defmacro an-int [] 42)
            (defmacro a-true [] True)
            (defmacro a-false [] False)
            (defmacro a-float [] 42.0)
            (defmacro a-complex [] 42j)
            (defmacro a-string [] "foo")
            (defmacro a-bytes [] b"foo")
            (defmacro a-list [] [1 2])
            (defmacro a-tuple [#* xs] xs)
            (defmacro a-dict [] {1 2})
            (defmacro a-set [] #{1 2})

            (eval-when-compile
              (defn compile-adder [x y]
                `(+ ~x ~y)))
            (defmacro add-at-compile [x y]
              (compile-adder x y))

            (setv phase "load")
            (eval-when-compile
              (setv phase "compile"))
            (defmacro phase-when-compiling [] phase)

            (setv initialized False)
            (eval-and-compile
              (setv initialized True))
            (defmacro test-initialized [] initialized)

            (defmacro gensym-example []
              `(setv ~(hy.gensym) 1))
            (defclass GensymClass []
              (gensym-example)
              (gensym-example))

            [(an-int)
             (a-true)
             (a-false)
             (a-float)
             (a-complex)
             (a-string)
             (a-bytes)
             (a-list)
             (a-tuple 1 2)
             (a-dict)
             (a-set)
             (add-at-compile 20 22)
             phase
             (phase-when-compiling)
             initialized
             (test-initialized)
             (len (sfor name (dir GensymClass)
                    :if (not (.startswith name "__"))
                    name))]
            """
        )
        == [
            42,
            True,
            False,
            42.0,
            42j,
            "foo",
            b"foo",
            [1, 2],
            (1, 2),
            {1: 2},
            {1, 2},
            42,
            "load",
            "compile",
            True,
            True,
            2,
        ]
    )


def test_native_model_pattern_macro_cases() -> None:
    assert (
        eval_kernel(
            """
            (defmacro do-until [#* args]
              (import
                hy.model-patterns [whole FORM notpexpr dolike]
                funcparserlib.parser [many])
              (setv [body condition] (.parse
                (whole [(many (notpexpr "until")) (dolike "until")])
                args))
              (setv g (hy.gensym))
              `(do
                (setv ~g True)
                (while (or ~g (not (do ~@condition)))
                  ~@body
                  (setv ~g False))))

            (setv n 0 s "")
            (do-until
              (+= s "x")
              (until (+= n 1) (>= n 3)))
            (setv until-first s)
            (do-until
              (+= s "x")
              (until (+= n 1) (>= n 3)))

            (defmacro loop [#* args]
              (import
                hy.model-patterns [whole FORM sym SYM]
                funcparserlib.parser [many])
              (setv [loopers body] (.parse
                (whole [
                  (many (|
                    (>> (+ (sym "while") FORM) (fn [x] [x]))
                    (+ (sym "for") SYM (sym "in") FORM)
                    (+ (sym "for") SYM (sym "from") FORM (sym "to") FORM)))
                  (sym "do")
                  (many FORM)])
                args))
              (defn f [loopers]
                (setv head (if loopers (get loopers 0) None))
                (setv tail (cut loopers 1 None))
                (cond
                   (is head None)
                    `(do ~@body)
                   (= (len head) 1)
                    `(while ~@head ~(f tail))
                   (= (len head) 2)
                    `(for [~@head] ~(f tail))
                   True
                    (do
                      (setv [sym from to] head)
                      `(for [~sym (range ~from (+ ~to 1))] ~(f tail)))))
              (f loopers))

            (setv l [])
            (loop
               for x in "abc"
               do (.append l x))
            (setv loop-first l)
            (setv l [] k 2)
            (loop
               while (> k 0)
               for n from 1 to 3
               for p in [k n (* 10 n)]
               do (.append l p) (-= k 1))

            (import
              hy.model-patterns [whole FORM :as X]
              funcparserlib.parser [skip])
            (defn parse-whole [#* parsers]
              (.parse
                 (whole parsers)
                 ['1 '2 '3]))

            [[until-first s]
             [loop-first l]
             [(= (.parse (whole []) []) #())
              (= (.parse (whole [X]) ['1]) #('1))
              (= (.parse (whole [(skip X)]) ['1]) #())
              (= (parse-whole X X X) #('1 '2 '3))
              (= (parse-whole (skip X) X X) #('2 '3))
              (= (parse-whole (skip X) (skip X) X) #('3))
              (= (parse-whole (skip X) X (skip X)) #('2))
              (= (parse-whole (skip X) (skip X) (skip X)) #())]]
            """
        )
        == [
            ["xxx", "xxxx"],
            [["a", "b", "c"], [2, 1, 10, -1, 2, 20, -4, 3, 30]],
            [True, True, True, True, True, True, True, True],
        ]
    )


def test_native_defmacro_core_shadow_warning_cases() -> None:
    warning_text = "will shadow the core macro"
    for source in [
        "(defmacro when [] 1)",
        "(require tests.resources.tlib [qplah :as when])",
        "(defn f [] (defmacro when [] 1) 42)",
        "(defn f [] (require tests.resources.tlib [qplah :as when]) 42)",
    ]:
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            eval_kernel(source)
            assert any(warning_text in str(item.message) for item in caught)
    for source in [
        "(pragma :warn-on-core-shadow False) (defmacro when [] 1)",
        "(pragma :warn-on-core-shadow False)"
        " (require tests.resources.tlib [qplah :as when])",
        "(defn f []"
        "  (pragma :warn-on-core-shadow False)"
        "  (defmacro when [] 1)"
        "  42)",
    ]:
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            eval_kernel(source)
            assert not caught
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        eval_kernel(
            """
            (defn f []
              (pragma :warn-on-core-shadow False)
              (defmacro when [] 1)
              42)
            (defmacro when [] 1)
            """
        )
        assert len(caught) == 1
        assert warning_text in str(caught[0].message)


def test_native_defmacro_error_wrapping_cases() -> None:
    for source, expected in [
        (
            "(defmacro blah [x] `(print ~@z)) (blah y)",
            "NameError",
        ),
        (
            "(defmacro wrap-error-test [] (fn [])) (wrap-error-test)",
            "HyWrapperError",
        ),
    ]:
        try:
            eval_kernel(source)
        except Exception as exc:
            assert exc.__class__.__name__ == "HyMacroExpansionError"
            assert expected in str(exc)
            assert "expanding macro" in str(exc)
        else:
            raise AssertionError(f"expected HyMacroExpansionError for {source!r}")


def test_native_hy_macroexpand_cases() -> None:
    assert (
        eval_kernel(
            """
            (defmacro mac [x expr]
              `(~@expr ~x))
            (defmacro m-with-named-import []
              (import math [pow])
              (pow 2 3))
            (import tests.resources [tlib :as TL])
            (defn f [])
            (setv non-call '(wmbatt 1 2))
            (setv result-macro '(+ 1 1))
            [(= (hy.macroexpand '(mac (a b) (x y)))
                '(x y (a b)))
             (= (hy.macroexpand '(mac (a b) (mac 5)))
                '(a b 5))
             (= (hy.macroexpand-1 '(mac (a b) (mac 5)))
                '(mac 5 (a b)))
             (= (hy.macroexpand '(qplah "phooey")
                                :module "tests.resources.tlib")
                '[8 "phooey"])
             (= (hy.macroexpand '(qplah "phooey") :module TL)
                '[8 "phooey"])
             (= (hy.macroexpand '(chippy 1)
                                :macros {"chippy" (fn [x] `[~x ~x])})
                '[1 1])
             (is (hy.macroexpand f) f)
             (is (hy.macroexpand non-call) non-call)
             (is (hy.macroexpand result-macro) result-macro)
             (= (hy.macroexpand '(m-with-named-import))
                (hy.models.Float (** 2 3)))]
            """
        )
        == [True] * 10
    )


def test_native_reader_macro_delegation_cases() -> None:
    evaluate = make_kernel_evaluator("hy_meta_native_subset.kernel_reader_macros")
    assert evaluate("(defreader foo '1) #foo") == 1
    assert evaluate('(defreader foo "doc" \'1) [#foo (. (get-macro :reader foo) __doc__)]') == [
        1,
        "doc",
    ]
    assert evaluate('(defreader ^foo \'1) #^foo') == 1
    assert evaluate('(defreader foo \'1) (in "foo" _hy_reader_macros)') is True
    assert evaluate('(defreader rm---x \'1) (not-in "rm___x" _hy_reader_macros)')
    assert (
        evaluate(
            """
            (defreader rm---
              (setv form (.parse-one-form &reader))
              `(do (+= ~form "a") ~form))
            (defreader rm___
              (setv form (.parse-one-form &reader))
              `(do (+= ~form "b") ~form))
            (setv x "")
            [#rm--- x #rm___ x]
            """
        )
        == ["ab", "ab"]
    )
    assert (
        evaluate(
            """
            (import hy)
            (defreader lower
              (hy.models.String (.lower (&reader.parse-one-form))))
            #lower "HeLLO, WoRLd!"
            """
        )
        == "hello, world!"
    )
    assert (
        evaluate('(defreader foo None) #(1 #foo 2)')
        == (1, 2)
    )
    assert (
        evaluate('(defreader foo \'None) #(1 #foo 2)')
        == (1, None, 2)
    )
    assert (
        evaluate('(do (defreader foo "foo") (defreader bar "bar")) [#foo #bar]')
        == ["foo", "bar"]
    )
    assert (
        evaluate(
            """
            (defreader rm1 11)
            (defreader rm☘ 22)
            [(is (get-macro :reader rm1) (get _hy_reader_macros "rm1"))
             (is (get-macro :reader rm☘) (get _hy_reader_macros "rm☘"))]
            """
        )
        == [True, True]
    )
    assert evaluate('(require tests.resources.tlib :readers [upper!]) #upper! "hi"') == "HI"
    assert (
        evaluate(
            '(require tests.resources.tlib :readers *)'
            ' [#upper! "eVeRy" #lower "ReAdEr"]'
        )
        == ["EVERY", "reader"]
    )
    assert (
        evaluate(
            '(require tests.resources.tlib [qplah] :readers [upper!])'
            ' [(qplah 1) #upper! "hi"]'
        )
        == [[8, 1], "HI"]
    )
    for require_spec in [
        "[qplah] :readers [upper!]",
        ":readers [upper!] [qplah]",
        ":macros [qplah] :readers [upper!]",
        ":readers [upper!] :macros [qplah]",
    ]:
        assert (
            evaluate(
                f"(require tests.resources.tlib {require_spec})"
                ' [(qplah 1) #upper! "hello"]'
            )
            == [[8, 1], "HELLO"]
        )
    eval_kernel_raises("(defreader :a-key '1)", "HyMacroExpansionError", evaluate)
    eval_kernel_raises("# _ 3", "PrematureEndOfInput", evaluate)
    eval_kernel_raises(
        "(require tests.resources.tlib [taggart] [upper!])",
        "HySyntaxError",
        evaluate,
    )
    eval_kernel_raises(
        "(require tests.resources.tlib :readers [taggart] :readers [upper!])",
        "HySyntaxError",
        evaluate,
    )
    eval_kernel_raises(
        "(require tests.resources.tlib :readers [not-a-real-reader])",
        "HyRequireError",
        evaluate,
    )
    assert (
        evaluate(
            r'''
            (import hy types)
            (setv module (types.ModuleType "<reader-eval>"))
            (hy.eval (hy.read "(defreader r 5)") :module module)
            (hy.eval '(defreader test-read 4) :module module)
            (hy.eval '(require tests.resources.tlib :readers [upper!])
                     :module module)
            (setv current-reader-clean True)
            (for [tag ["#r" "#test-read" "#upper!"]]
              (try
                (hy.read tag)
                (setv current-reader-clean False)
                (except [Exception]
                  None)))
            (hy.eval '(setv reader (hy.HyReader :use-current-readers True))
                     :module module)
            (setv reader module.reader)
            (setv explicit-module-reader-values
                  [(hy.eval (hy.read "#r" :reader reader) :module module)
                   (hy.eval (hy.read "#test-read" :reader reader) :module module)
                   (hy.eval (hy.read "#upper! \"hi there\"" :reader reader)
                            :module module)])
            (setv module2 (types.ModuleType "<reader-explicit>"))
            (setv reader2 (hy.HyReader))
            (defn eval1 [s]
              (hy.eval (hy.read s :reader reader2) :module module2))
            (eval1 "(defreader fbaz 32)")
            (eval1 "(require tests.resources.tlib :readers [upper!])")
            [current-reader-clean
             explicit-module-reader-values
             (eval1 "#fbaz")
             (eval1 "#upper! \"hello\"")]
            '''
        )
        == [True, [5, 4, "HI THERE"], 32, "HELLO"]
    )
    assert (
        evaluate(
            """
            (import hy types)
            (setv module1 (types.ModuleType "<one>"))
            (setv module2 (types.ModuleType "<two>"))
            (setv stream1
                  (hy.read-many
                    "(do (defreader foo \\"foo1\\") (defreader bar \\"bar1\\")) #foo #bar"))
            (setv stream2
                  (hy.read-many
                    "(do (defreader foo \\"foo2\\") (defreader bar \\"bar2\\")) #foo #bar"))
            (setv results [])
            (for [expected [[None None] ["foo1" "foo2"] ["bar1" "bar2"]]]
              (setv pair [(hy.eval (next stream1) :module module1)
                          (hy.eval (next stream2) :module module2)])
              (.append results (= pair expected)))
            results
            """
        )
        == [True, True, True]
    )
    try:
        evaluate("#foo")
    except Exception as exc:
        assert exc.__class__.__name__ == "LexException"
    else:
        raise AssertionError("reader macro leaked into a fresh eval")


def test_native_reader_behavior_delegation_cases() -> None:
    evaluate = make_kernel_evaluator("hy_meta_native_subset.kernel_reader_behavior")
    assert (
        evaluate(
            r'''
            (import hy traceback)
            (defn tokenize [s]
              (list (hy.read-many s)))
            (defn first-form [s]
              (get (tokenize s) 0))
            (defn read-value [s]
              (hy.eval (first-form s)))
            (setv bracket (first-form "#[delim[hello world]delim]"))
            (setv empty-bracket (first-form "#[[squid]]"))
            (setv entry (first-form "(foo (one two))"))
            (setv symbol (get entry 0))
            (setv inner (get entry 1))
            (setv string-and-symbol (tokenize "\"apple\nblueberry\" abc"))
            (setv multiline (tokenize "\n(foo (one two))\n(foo bar)\n"))
            (setv dotted-error "")
            (try
              (tokenize "1.foo")
              (except [e Exception]
                (setv dotted-error
                      (.join "" (traceback.format-exception-only (type e) e)))))
            (setv escape-error "")
            (try
              (tokenize "\"\\x8\"")
              (except [e Exception]
                (setv escape-error
                      (.join "" (traceback.format-exception-only (type e) e)))))
            (setv bad-prefix False)
            (try
              (first-form "z\"hello\"")
              (except [Exception]
                (setv bad-prefix True)))
            (setv bad-shebang False)
            (try
              (list (hy.read-many "#!/usr/bin/env hy\n5"))
              (except [Exception]
                (setv bad-shebang True)))
            [(= (do
                  ; full-line and trailing comments are ignored
                  42 ; trailing
                  )
                42)
             (= [1 #_ (/ 1 0) 2] [1 2])
             (= [#_ 0 #_ 1 2] [2])
             (= {1 #_ (/ 1 0) 2} {1 2})
             (= (str bracket) "hello world")
             (= bracket.brackets "delim")
             (= (str empty-bracket) "squid")
             (= empty-bracket.brackets "")
             (= (read-value "b\"hello\"") b"hello")
             (= (read-value "rb\"foo\\x5a\"") b"foo\\x5a")
             (= (read-value "r\"foo\\x5a\"") "foo\\x5a")
             (= (read-value "\"foo\\x5a\"") "fooZ")
             bad-prefix
             (= (read-value "42") 42)
             (= (read-value "0x_af") 175)
             (= (read-value "1_000,000") 1000000)
             (= (read-value "1_2._3,4") 12.34)
             (= (read-value "1,0_00j,") 1000j)
             (= (str (first-form "_42")) "_42")
             (= (tokenize "foo.bar") (tokenize "(. foo bar)"))
             (= (tokenize ".foo.bar") (tokenize "(. None foo bar)"))
             (in "The parts of a dotted identifier must be symbols" dotted-error)
             (= entry.start-line 1)
             (= entry.start-column 1)
             (= entry.end-column 15)
             (= symbol.start-column 2)
             (= inner.start-column 6)
             (= (. (get string-and-symbol 0) end-line) 2)
             (= (. (get string-and-symbol 1) start-line) 2)
             (= (. (get multiline 0) start-line) 2)
             (= (. (get multiline 1) start-line) 3)
             bad-shebang
             (= (hy.eval
                  (get (list (hy.read-many "#!/usr/bin/env hy\n5"
                                           :skip-shebang True))
                       0))
                5)
             (in "unicodeescape" escape-error)
             (in "line 1" escape-error)]
            '''
        )
        == [True] * 35
    )


def test_native_relative_require_cases() -> None:
    _stage2, _stage2_prime, stage3 = stage3_chain()
    kernel = stage3.load_hy_file(
        KERNEL_PATH, "hy_meta_native_subset.kernel_relative_require"
    )
    module = ModuleType("tests.native_tests.relative_require_probe")
    module.__package__ = "tests.native_tests"
    assert kernel.eval_source(
        "(require .beside [xyzzy]) (xyzzy)",
        module,
        "<hy-meta:native-subset:relative-require>",
    ) == 1
    assert kernel.eval_source(
        "(require ..resources.macros [test-macro-2]) (test-macro-2) qup",
        module,
        "<hy-meta:native-subset:relative-require>",
    ) == 2
    assert kernel.eval_source(
        "(require . [beside :as BS]) (BS.xyzzy)",
        module,
        "<hy-meta:native-subset:relative-require>",
    ) == 1
    assert kernel.eval_source(
        """
        (import ..resources [tlib in-init])
        (import .. [resources])
        [tlib.SECRET-MESSAGE in-init resources.in-init]
        """,
        module,
        "<hy-meta:native-subset:relative-import>",
    ) == ["Hello World", "chippy", "chippy"]


def test_native_hy_R_one_shot_require_cases() -> None:
    assert eval_kernel('(hy.R.tests/resources/tlib.qplah "x")') == [8, "x"]
    assert eval_kernel('(hy.R.tests/resources/tlib.✈ "x")') == "plane x"
    assert (
        eval_kernel(
            """
            (hy.R.tests/resources/tlib.qplah "x")
            (try
              (qplah "x")
              (except [NameError] "not-bound"))
            """
        )
        == "not-bound"
    )
    assert (
        eval_kernel(
            """
            (hy.R.tests/resources/tlib.qplah "x")
            (try
              (tests.resources.tlib.qplah "x")
              (except [NameError] "not-bound"))
            """
        )
        == "not-bound"
    )
    for source in [
        '(hy.R.tests/resources/tlib.nonexistent-macro "x")',
        '(hy.R.nonexistent-module.qplah "x")',
    ]:
        try:
            eval_kernel(source)
        except Exception as exc:
            assert exc.__class__.__name__ == "HyRequireError"
        else:
            raise AssertionError(f"expected HyRequireError for {source!r}")


def test_native_hy_I_importer_cases() -> None:
    assert (
        eval_kernel(
            """
            (setv math (type "Dummy" #() {"sqrt" "hello"}))
            (defmacro frac [a b]
              `(hy.I.fractions.Fraction ~a ~b))
            [(hy.I.math.sqrt 4)
             (.sqrt (hy.I "math") 4)
             math.sqrt
             (= (* 6 (frac 1 3)) 2)
             (hy.I.os/path.basename "foo/bar")
             (try
               math.missing
               (except [AttributeError] "local-still-local"))
             (try
               sqrt
               (except [NameError] "sqrt-not-bound"))
             (try
               fractions
               (except [NameError] "fractions-not-bound"))
             (try
               os
               (except [NameError] "os-not-bound"))]
            """
        )
        == [
            2.0,
            2.0,
            "hello",
            True,
            "bar",
            "local-still-local",
            "sqrt-not-bound",
            "fractions-not-bound",
            "os-not-bound",
        ]
    )


def test_native_inspect_metadata_cases() -> None:
    import importlib
    import inspect
    import os.path
    import pydoc

    from bootstrap import install_kernel_import_hook

    _stage2, _stage2_prime, stage3 = stage3_chain()
    kernel = stage3.load_hy_file(
        KERNEL_PATH, "hy_meta_native_subset.kernel_inspect_metadata"
    )
    resources_root = ROOT / "tests" / "resources" / "hy_inspect"
    module_names = [
        "fodder_1",
        "fodder_2",
    ]
    for name in module_names:
        sys.modules.pop(name, None)

    hook = install_kernel_import_hook(kernel, [resources_root])
    try:
        with hook:
            importlib.invalidate_caches()
            fodder_1 = importlib.import_module("fodder_1")
            fodder_2 = importlib.import_module("fodder_2")
            modfile = os.path.normcase(fodder_1.__file__)
            if modfile.endswith(("c", "o")):
                modfile = modfile[:-1]
            git = fodder_1.StupidGit

            assert inspect.getdoc(fodder_1) == "A module docstring."
            assert (
                inspect.getdoc(git)
                == "A longer,\nindented\n\n   docstring."
            )
            assert (
                inspect.getdoc(git.abuse)
                == "Another\n\n    docstring\n\n containing\n    \n\ntabs\n\n "
            )
            rendered_doc = pydoc.render_doc(git, "Help on %s")
            assert "StupidGit" in rendered_doc
            assert "A longer" in rendered_doc
            assert inspect.getdoc(fodder_1.FesteringGob) == inspect.getdoc(git)
            assert inspect.getdoc(fodder_1.ChildNoDoc.foo) is None
            assert inspect.getcomments(fodder_1) == "; first line of the file\n"
            assert os.path.normcase(inspect.getsourcefile(fodder_1.spam)) == modfile
            source_1 = inspect.getsource(fodder_1)
            source_2 = inspect.getsource(fodder_2)
            assert "(defn abuse [self a b c]" in source_1
            assert "(defn :async lobbest [grenade])" in source_1
            assert ") (setv after_closing (fn [] 1))" in source_1
            assert '(defn f-with-reader [] (setv #m "x was assigned") x)' in source_2
            assert "(defn multiform-reader-macro [#* xs]" in source_2
            assert source_2.strip().splitlines()[-1] == (
                '#do-twice (setv x "x is assigned twice")'
            )
            assert fodder_2.__loader__.get_source("fodder_2") == source_2
    finally:
        for name in module_names:
            sys.modules.pop(name, None)


def test_native_dot_chain_cases() -> None:
    assert (
        eval_kernel(
            """
            (import os)
            (defclass C []
              (defn __init__ [self]
                (setv (. self xs) [10 20 12])))
            (defclass X [object])
            (defclass M [object]
              (defn meth [self #* args #** kwargs]
                (.join " " (+ #("meth") args
                  (tuple (map (fn [k] (get kwargs k))
                              (sorted (.keys kwargs))))))))
            (setv c (C))
            (setv m (M))
            (setv boxes [m])
            (setv x (X))
            (setv x.p m)
            (setv x.a (X))
            (setv x.a.b m)
            (defclass DotClass [object])
            (setv foo [(DotClass) (DotClass) (DotClass)])
            (setv bar (DotClass))
            (setv (. foo [1]) bar)
            (setv (. foo [1] test) "hello")
            (setv a 1 b 2 d 3)
            (defn .. [#* args]
              (.join "~" (map str args)))
            (defmacro .... [#* args]
              (.join "@" (map str args)))
            ((. defn) dotted-root-function [] "ok")
            (setv (. c xs [1]) 40)
            [(. [10 20 12] [1])
             (. "ab hello" (strip "ab ") (upper))
             (. "hElLO\twoRld" (expandtabs :tabsize 4) (lower))
             (+ (get (. c xs) 1) 2)
             (. "abc" __class__ __name__ [0])
             (.meth m)
             (.meth m "foo" "bar")
             (.meth :b "1" :a "2" m "foo" "bar")
             (.meth m #* ["foo" "bar"])
             (.p.meth x)
             (.p.meth x "foo" "bar")
             (.p.meth :b "1" :a "2" x "foo" "bar")
             (.p.meth x #* ["foo" "bar"])
             (.a.b.meth x)
             (.a.b.meth x "foo" "bar")
             (.a.b.meth :b "1" :a "2" x "foo" "bar")
             (.a.b.meth x #* ["foo" "bar"])
             (.__str__ :foo)
             (is (. boxes) boxes)
             ((. "" join) ["aa" "bb"])
             ((. "" join) ["aa" "bb" "cc"])
             (is (. foo [0]) (get foo 0))
             (is (. foo [0] __class__) DotClass)
             (is (. foo [(+ 1 1)] __class__) DotClass)
             (. foo [(+ 1 1)] __class__ __name__ [0])
             (. os (getcwd) (isalpha) __class__ __name__ [0])
             (is bar (get foo 1))
             (getattr (. foo [1]) "test")
             ..a.b.d
             ....uno.dos.tres
             (dotted-root-function)]
            """
        )
        == [
            20,
            "HELLO",
            "hello   world",
            42,
            "s",
            "meth",
            "meth foo bar",
            "meth foo bar 2 1",
            "meth foo bar",
            "meth",
            "meth foo bar",
            "meth foo bar 2 1",
            "meth foo bar",
            "meth",
            "meth foo bar",
            "meth foo bar 2 1",
            "meth foo bar",
            ":foo",
            True,
            "aabb",
            "aabbcc",
            True,
            True,
            True,
            "D",
            "b",
            True,
            "hello",
            "None~1~2~3",
            "None@uno@dos@tres",
            "ok",
        ]
    )


def test_native_defclass_cases() -> None:
    assert (
        eval_kernel(
            """
            (defclass BareNativeClass)
            (defclass NativeMeta [type])
            (defclass NativeWithMeta [:metaclass NativeMeta])
            (defclass InheritA [])
            (defclass InheritB [InheritA])
            (defclass InheritC [object])
            (defclass InheritD [InheritB InheritC])
            (defclass Attrs []
              (setv x 42))
            (defclass MethodAttrs []
              (setv x 42)
              (setv y (fn [self value]
                        (+ (. self x) value))))
            (setv method-attrs-instance (MethodAttrs))
            (setv MethodAttrs.x 0)
            (defclass DocSet []
              (setv __doc__ "doc string")
              (setv x 1))
            (defclass DocLiteral []
              "doc string"
              (setv x 1))
            (defclass MultiLineDoc []
              "begin a very long multi-line string to make
               sure that it comes out the way we hope
               and can span 3 lines end."
              (setv x 1))
            (defmacro DefClassSetter []
              `(defn set-x [self value] (setv (. self _x) value)))
            (defclass MacroExpanded []
              (DefClassSetter))
            (setv macro-expanded-instance (MacroExpanded))
            (.set-x macro-expanded-instance 9)
            (setv class_body_foo 1)
            (defclass SyntaxClass []
              (setv x 1)
              (setv y 2)
              (global class_body_foo)
              (setv class_body_foo 2)
              (defn greet [self]
                "Greet the caller"
                "hello!"))
            (defclass NativeBase []
              (defn __init-subclass__ [cls swallow #** kwargs]
                (setv (. cls swallow) swallow)))
            (defclass NativeChild [NativeBase :swallow "african"])
            (defclass NativePreparedDict [dict]
              (defn __setitem__ [self key value]
                (dict.__setitem__ self (+ "prepared_" key) value)))
            (defclass NativePreparedMeta [type]
              (defn [classmethod] __prepare__ [metacls name bases]
                (NativePreparedDict)))
            (defclass NativePrepared [:metaclass NativePreparedMeta]
              (defn [classmethod] method [cls] 7))
            (defclass DynamicBase [((fn [] (if True list dict)))]
              (setv x 42))
            (defclass DynamicDict [((fn [] (if False list dict)))]
              (setv x 42))
            (defclass NoLeak []
              (setv x (fn [] 1)))
            (setv no-leak False)
            (try
              (x)
              (except [NameError]
                (setv no-leak True)))
            (setv class-side-effect False)
            (defn set-class-side-effect []
              (global class-side-effect)
              (setv class-side-effect True))
            (defclass SideEffect []
              (set-class-side-effect))
            [(. BareNativeClass __name__)
             (is (type NativeWithMeta) NativeMeta)
             (isinstance (InheritD) InheritA)
             (isinstance (InheritD) InheritB)
             (isinstance (InheritD) InheritC)
             (not (isinstance (InheritA) InheritD))
             (= Attrs.x 42)
             (= (getattr (Attrs) "x") 42)
             (= (.y method-attrs-instance 1) 1)
             (= DocSet.__doc__ "doc string")
             (= DocLiteral.__doc__ "doc string")
             (and (in "begin" MultiLineDoc.__doc__)
                  (in "end" MultiLineDoc.__doc__))
             (= (. macro-expanded-instance _x) 9)
             (= SyntaxClass.x 1)
             (= SyntaxClass.y 2)
             (= class_body_foo 2)
             (= (.greet (SyntaxClass)) "hello!")
             (. (NativeChild) swallow)
             (NativePrepared.prepared-method)
             (isinstance (DynamicBase) list)
             (isinstance (DynamicDict) dict)
             no-leak
             class-side-effect]
            """
        )
        == [
            "BareNativeClass",
            True,
            True,
            True,
            True,
            True,
            True,
            True,
            True,
            True,
            True,
            True,
            True,
            True,
            True,
            True,
            True,
            "african",
            7,
            True,
            True,
            True,
            True,
        ]
    )


def test_native_decorator_cases() -> None:
    assert (
        eval_kernel(
            """
            (import asyncio)
            (defn foodec [func]
              (fn [] (+ (func) 1)))
            (defn [foodec] one-line []
              (* 2 2))
            (defn bazdec [func]
              (fn [] (+ (func) "x")))
            (defn [bazdec] multiline []
              (setv intermediate "i")
              (+ intermediate "b"))
            (defn classdec [cls]
              (setv (. cls bonus) 456)
              cls)
            (defclass [classdec] DecoratedClass []
              (setv attr 123))
            (defn dec1 [f] (fn [] (+ (f) "a")))
            (defn dec2 [f] (fn [] (+ (f) "b")))
            (defn [dec1 dec2] stacked [] "c")
            (setv events [])
            (defn order-decorator [f]
              (.append events "decorator")
              (fn []
                (.append events "wrapper")
                (f)))
            (defn
              [(do (.append events "decorator-list") order-decorator)]
              ordered
              [[arg (do (.append events "default") 1)]]
              (.append events "body")
              arg)
            (.append events (ordered))
            (defn async-decorator [func]
              (fn :async [] (/ (await (func)) 2)))
            (defn :async [async-decorator] coro-test []
              (await (asyncio.sleep 0))
              42)
            [(one-line)
             (multiline)
             (. DecoratedClass attr)
             (. DecoratedClass bonus)
             (stacked)
             events
             (asyncio.run (coro-test))]
            """
        )
        == [
            5,
            "ibx",
            123,
            456,
            "cba",
            ["decorator-list", "default", "decorator", "wrapper", "body", 1],
            21.0,
        ]
    )


def test_ellipsis_constant() -> None:
    assert (
        eval_kernel(
            """
            (setv e Ellipsis)
            (setv Ellipsis 14)
            (and (= Ellipsis 14)
                 (!= ... 14)
                 (is ... e))
            """
        )
        is True
    )


def test_match_patterns() -> None:
    assert (
        eval_kernel(
            """
            (defclass P []
              (setv __match_args__ (tuple ["x" "y"]))
              (defn __init__ [self x y]
                (setv (. self x) x)
                (setv (. self y) y)))
            (setv star 0)
            (match [1 2 3 4]
              [1 #* middle 4] (setv star (sum middle))
              _ (setv star 0))
            (setv rest-score 0)
            (match {"keep" 7 "a" 10 "b" 25}
              {"keep" keep #** rest}
              (setv rest-score (+ keep (get rest "a") (get rest "b")))
              _ (setv rest-score 0))
            (setv class-score 0)
            (match (P 11 31)
              (P :y y :x x) (setv class-score (+ x y))
              _ (setv class-score 0))
            [star rest-score class-score]
            """
        )
        == [5, 42, 42]
    )


def test_native_match_expression_syntax() -> None:
    assert (
        eval_kernel(
            """
            (defclass A []
              (setv B 0))
            (defclass Point []
              (setv __match_args__ (tuple ["x" "y"]))
              (defn __init__ [self x y]
                (setv (. self x) x)
                (setv (. self y) y)))
            (setv events [])
            (setv z
                  (match 0
                         x :if x 0
                         _ :as y :if (and (= y x) y) 1
                         A.B 2
                         (. A B) 3))
            (match 1
                   1 :if (do (.append events 1) False)
                   (.append events 2)
                   1 :if False
                   (.append events 3)
                   _ :if (do (.append events 4) True)
                   (.append events 5))
            (setv side-x 0)
            (defn side-foo []
              (nonlocal side-x)
              (+= side-x 1)
              side-x)
            (match (side-foo)
                   n (setv side-first n))
            (match (do (setv side-y side-x) (side-foo))
                   n (setv side-second n))
            (match (do (side-foo) (side-foo))
                   n (setv side-third n))
            (match (do (side-foo) (side-foo) side-x)
                   n (setv side-fourth n))
            [(match 0
                    0 :if False False
                    0 :if True True)
             (match 4
                    (| 0 1 2 3) True)
             (match 1)
             z
             (= A.B 0)
             (match #(0 1 2)
                    [#* xs] [0 xs])
             (match [0 1 2]
                    [0 #* xs] xs)
             (match :hello
                    :hello "keyword"
                    any-binding "missing")
             (= (match 1
                       1 :if True ':as)
                ':as)
             (match [0 1 2]
                    [0 #* xs] :as whole
                    :if (do
                          (setv guard-len (len whole))
                          (= guard-len 3))
                    (sum xs))
             (match (Point 1 0)
                    (Point 1 :y var) var)
             events
             [side-first side-y side-second side-third side-fourth side-x]]
            """
        )
        == [
            True,
            None,
            None,
            2,
            True,
            [0, [0, 1, 2]],
            [1, 2],
            "keyword",
            True,
            3,
            0,
            [1, 4, 5],
            [1, 1, 2, 4, 6, 6],
        ]
    )


def test_native_match_failure_cases() -> None:
    evaluate = make_kernel_evaluator("hy_meta_native_subset.match_failures")
    for source in [
        "(match {} {x 1} 1)",
        "(match [1 2] [x x] x)",
        "(match [1 2 3] [#* a #* b] a)",
        "(match [1] [#* 1] 1)",
        "(match [1] [1 :as] 1)",
        "(match [1] [1 :as 1] 1)",
        "(match [1] [#* x :as y] 1)",
        "(match {1 2} {1 x #** 1} x)",
        '(match {"a" 1} {"a" x "a" y} x)',
        "(match 1 1)",
        "(match 1 (1 2) 3)",
    ]:
        try:
            evaluate(source)
        except SyntaxError:
            pass
        else:
            raise AssertionError(f"expected SyntaxError for {source!r}")


def test_native_match_side_effect_order_cases() -> None:
    assert (
        eval_kernel(
            """
            (setv events [])
            (defclass PatternBox []
              (defn __getattr__ [self name]
                (.append events (+ "pattern-" name))
                (if (= name "target")
                    (return 2)
                    (raise (AttributeError name)))))
            (setv box (PatternBox))
            (setv pattern-result
                  (match 1
                         (. box target)
                           (do (.append events "bad-pattern-hit") "bad")
                         1 :if (do (.append events "guard-false") False)
                           (do (.append events "bad-guard-hit") "bad")
                         1 :if (do (.append events "guard-true") True)
                           (do (.append events "body-hit") "hit")
                         _
                           (do (.append events "bad-wildcard") "bad")))
            (setv nested-result
                  (let [x [1 2 3]
                        y {"a" 1 "b" 2 "c" 3}
                        b 1
                        a 1]
                    (match x
                           [1 #* x] None)
                    [(match x
                            [_ 3 :as a] :as b :if (= a 3)
                            [a b x])
                     (match y
                            {"b" b #** a}
                            [b a])
                     b]))
            [events pattern-result nested-result]
            """
        )
        == [
            ["pattern-target", "guard-false", "guard-true", "body-hit"],
            "hit",
            [[3, [2, 3], [2, 3]], [2, {"a": 1, "c": 3}], 2],
        ]
    )


def test_native_match_upstream_runtime_cases() -> None:
    assert (
        eval_kernel(
            """
            (defclass Point []
              (setv __match_args__ (tuple ["x" "y"]))
              (defn __init__ [self x y]
                (setv (. self x) x)
                (setv (. self y) y)))
            (defclass C [Point]
              (setv C Point))
            (defn whereis [points]
              (match points
                     [] "No points"
                     [(Point 0 0)] "The origin"
                     [(Point x y)] f"Single point {x}, {y}"
                     [(Point 0 y1) (Point 0 y2)]
                       f"Two on the Y axis at {y1}, {y2}"
                     _ "Something else"))
            (setv x #{0})
            (setv set-list-miss
                  (match x
                         [0] 0))
            (setv set-class
                  (match x
                         (set z) z))
            (setv nested-or
                  (match {0 0}
                         {0 [1 2 {}]} 0
                         (| {0 (| [1 2 {}] False)}
                            {1 [[]]}
                            {0 [1 2 {}]}
                            []
                            "X"
                            {}) 1
                         [] 2))
            (setv nested-map
                  (match {"something" {"important" 42}
                          "some list" [[1 2 3]]}
                         {"something" {"important" a}
                          "some list" [b]}
                           [a b]))
            (setv tuple-as
                  (match [(Point -1 0) (Point 1 2)]
                         #((Point x1 y1) (Point x2 y2) :as p2) :as whole
                         [x1 y1 x2 y2 (. p2 x) (. p2 y) (len whole)]))
            (setv let-pattern
                  (do
                    (setv [outer-x outer-y] [1 2]
                          p (Point 5 6))
                    [(let [x 3 y 4]
                       (match p
                              (Point x y) [x y]
                              _ False))
                     [outer-x outer-y]
                     (let [x (Point 3 6)
                           y 9]
                       (match p
                              (Point :y (. x y)) :as y
                                [(. y y) (. x y)]
                              _ False))
                     (let [x 3 y 4]
                       (match (Point 3 4)
                              (Point :y m :x n)
                                :if (= [n m] [x y])
                                True
                              _ False))]))
            (setv type-error False)
            (try
              (match []
                     (print 1 1) 1)
              (except [TypeError]
                (setv type-error True)))
            [(whereis [])
             (whereis [(Point 0 0)])
             (whereis [(Point 0 1)])
             (whereis [(Point 0 0) (Point 0 0)])
             (whereis [(Point 0 1) (Point 0 1)])
             (whereis [(Point 0 1) (Point 1 0)])
             (whereis 42)
             (= (match #() [] 0) 0)
             (match [0 0]
                    (| [0 1] [1 0]) 0)
             set-list-miss
             (= set-class x)
             nested-or
             nested-map
             tuple-as
             (match (Point 1 2)
                    (C.C 1 2) "ok")
             (match [1 2 3]
                    whole whole)
             let-pattern
             type-error]
            """
        )
        == [
            "No points",
            "The origin",
            "Single point 0, 1",
            "Two on the Y axis at 0, 0",
            "Two on the Y axis at 1, 1",
            "Something else",
            "Something else",
            True,
            None,
            None,
            True,
            1,
            [42, [1, 2, 3]],
            [-1, 0, 1, 2, 1, 2, 2],
            "ok",
            [1, 2, 3],
            [[5, 6], [1, 2], [6, 6], True],
            True,
        ]
    )


def test_native_control_flow_cases() -> None:
    assert (
        eval_kernel(
            """
            (setv counter 0)
            (if False
              (assert False)
              (do
                (+= counter 1)
                (+= counter 1)
                (+= counter 1)))
            (setv x 1)
            (setv label "")
            (defn tick [factor result]
              (global x)
              (*= x factor)
              (return result))
            (cond
              (tick 2 True) (do (*= x 3) (setv label "first"))
              (tick 5 True) (do (*= x 7) (setv label "second")))
            (setv s "")
            (setv n 2)
            (defn keep-going []
              (global s n)
              (+= s "a")
              (return n))
            (while (keep-going)
              (+= s "b")
              (-= n 1)
              (else
                (+= s "z")))
            (setv for-pending-source "")
            (setv for-pending-values [])
            (for [outer "ab"
                  inner (do
                          (+= for-pending-source "y")
                          "de")]
              (.append for-pending-values
                       (+ outer inner for-pending-source)))
            (setv do-x "a")
            (setv do-y (do
                         (setv do-x "b")
                         "c"))
            (setv when-do 0)
            (when (do
                    (setv when-do 1)
                    True)
              (setv when-do (+ when-do 41)))
            (setv cond-do 0)
            (cond
              (do
                (setv cond-do 1)
                False)
              (setv cond-do 0)
              (do
                (setv cond-do (+ cond-do 1))
                True)
              (setv cond-do (+ cond-do 40)))
            (setv while-do-s "")
            (setv while-do-x 2)
            (while (do
                     (+= while-do-s "a")
                     while-do-x)
              (+= while-do-s "b")
              (-= while-do-x 1)
              (else
                (+= while-do-s "z")))
            (setv while-continue-s "")
            (setv while-continue-x 2)
            (setv continued False)
            (while (do
                     (+= while-continue-s "a")
                     while-continue-x)
              (+= while-continue-s "b")
              (when (and (= while-continue-x 1) (not continued))
                (+= while-continue-s "c")
                (setv continued True)
                (continue))
              (-= while-continue-x 1)
              (else
                (+= while-continue-s "z")))
            (setv while-break-s "")
            (for [outer "123"]
              (+= while-break-s outer)
              (setv inner 0)
              (while (do
                       (when (and (= outer "2") (= inner 1))
                         (break))
                       (< inner 3))
                (+= while-break-s "y")
                (+= inner 1)))
            (setv while-last-out [])
            (setv while-last-x 0)
            (setv while-last-a [1 1])
            (while (do
                     (.append while-last-out 2)
                     (setv while-last-x (and while-last-a
                                             (.pop while-last-a)))
                     while-last-x)
              (setv while-last-x 0)
              (.append while-last-out while-last-x))
            (defn break-value []
              (for [break-x (range 10)]
                (when (= break-x 5)
                  (break)))
              break-x)
            (setv continue-values [])
            (for [continue-x (range 10)]
              (when (!= continue-x 5)
                (continue))
              (.append continue-values continue-x))
            [counter x label s do-x do-y when-do cond-do
             while-do-s while-continue-s while-break-s
             [while-last-out (is while-last-x while-last-a)]
             for-pending-values for-pending-source
             (break-value) continue-values (do) (do 1 2 3)]
            """
        )
        == [
            3,
            6,
            "first",
            "ababaz",
            "b",
            "c",
            42,
            42,
            "ababaz",
            "ababcabaz",
            "1yyy2y3yyy",
            [[2, 0, 2, 0, 2], True],
            ["ady", "aey", "bdyy", "beyy"],
            "yy",
            5,
            [5],
            None,
            3,
        ]
    )


def test_native_nonlocal_cases() -> None:
    assert (
        eval_kernel(
            """
            (setv home "earth")
            (defn blastoff []
              (nonlocal home)
              (setv home "saturn"))
            (blastoff)
            (defn local-score []
              (setv score 4)
              (defn inner []
                (nonlocal score)
                (setv score 10))
              (inner)
              score)
            (defn make-ration-log [days intensity]
              (setv health 20
                    ration-log
                    (list (map (fn [_]
                                 (nonlocal rations health)
                                 (-= rations intensity)
                                 (+= health (* 0.5 intensity))
                                 rations)
                               (range days))))
              health)
            (setv rations 100)
            [home (local-score) (make-ration-log 43 1.5) rations]
            """
        )
        == ["saturn", 10, 52.25, 35.5]
    )
    try:
        eval_kernel(
            """
            (defn make-ration-log [days intensity]
              (list (map (fn [_]
                           (nonlocal rations)
                           (-= rations intensity)
                           rations)
                         (range days))))
            (make-ration-log 43 1.5)
            """
        )
    except SyntaxError as exc:
        assert "no binding for nonlocal 'rations'" in str(exc)
    else:
        raise AssertionError("nonlocal without an enclosing/module binding compiled")


def test_native_logic_short_circuit_cases() -> None:
    assert (
        eval_kernel(
            """
            (setv and-del-list ["a" "b"])
            (setv and-del-skip (and 0 (del (get and-del-list 1))))
            (setv and-del-run (and 15 (del (get and-del-list 1))))
            (setv or-del-list ["a" "b"])
            (setv or-del-skip (or 15 (del (get or-del-list 1))))
            (setv or-del-run (or 0 (del (get or-del-list 1))))
            (setv and-for-list [])
            (setv and-for-skip
                  (and 0 (for [n [1 2]] (.append and-for-list n))))
            (setv and-for-run
                  (and 15 (for [n [1 2]] (.append and-for-list n))))
            (setv or-for-list [])
            (setv or-for-skip
                  (or 15 (for [n [1 2]] (.append or-for-list n))))
            (setv or-for-run
                  (or 0 (for [n [1 2]] (.append or-for-list n))))
            (setv setv-and-x 0)
            (setv setv-and-result (and (setv setv-and-x 1) 2))
            (setv setv-or-x 0)
            (setv setv-or-result (or (setv setv-or-x 3) 4))
            [and-del-skip and-del-run and-del-list
             or-del-skip or-del-run or-del-list
             and-for-skip and-for-run and-for-list
             or-for-skip or-for-run or-for-list
             [setv-and-result setv-and-x]
             [setv-or-result setv-or-x]]
            """
        )
        == [
            0,
            None,
            ["a"],
            15,
            None,
            ["a"],
            0,
            None,
            [1, 2],
            15,
            None,
            [1, 2],
            [None, 1],
            [4, 3],
        ]
    )


def test_native_compiler_boolop_shape_cases() -> None:
    kernel = make_kernel("hy_meta_native_subset.kernel_compiler_boolop_shape")

    py = kernel.python_source(
        "(and 1 2 3 (do (setv x 4) x) 5 6)",
        "<hy-meta:native-subset:compiler-boolop>",
    )
    assert py.count("if") == 1

    py = kernel.python_source(
        "(or 1 2 3 (do (setv x 4) x) 5 6 (do (setv y 7)))",
        "<hy-meta:native-subset:compiler-boolop>",
    )
    assert py.count("if") == 2

    py = kernel.python_source(
        "(or (and 1 2) (and 3 4))",
        "<hy-meta:native-subset:compiler-boolop>",
    )
    assert py.count(" and ") == 2
    assert py.count(" or ") == 1

    py = kernel.python_source(
        "(and (do (setv x 4) (or x 3)) 5 6)",
        "<hy-meta:native-subset:compiler-boolop>",
    )
    assert py.count("x = 4") == 1
    assert py.count("x or 3") == 1
    assert py.count(" and ") == 2


def test_native_conditional_expression_statement_cases() -> None:
    assert (
        eval_kernel(
            """
            (setv if-x 0)
            (setv if-false [(if False (setv if-x 1) 2) if-x])
            (setv if-y 0)
            (setv if-true [(if True (setv if-y 1) 2) if-y])
            (setv when-x 0)
            (setv when-false [(when False (setv when-x 1)) when-x])
            (setv when-y 0)
            (setv when-true [(when True (setv when-y 1)) when-y])
            (setv cond-x 0)
            (setv cond-false [(cond False (setv cond-x 1) True 2) cond-x])
            (setv cond-y 0)
            (setv cond-true [(cond False 2 True (setv cond-y 1)) cond-y])
            (setv cond-a 0)
            (setv cond-b 0)
            (setv cond-z 0)
            (setv cond-test-pending
                  [(cond (do (setv cond-a 1) False) (setv cond-z 1)
                         (do (setv cond-b 2) True) (setv cond-z 3))
                   cond-a cond-b cond-z])
            (defn final-if []
              (if True
                  (do (setv final-if-x 1) 7)
                  0))
            (defn final-when []
              (when True
                (setv final-when-x 1)
                11))
            (defn final-cond []
              (cond False 0
                    True (do (setv final-cond-x 1) 13)))
            (defn final-do []
              (do (setv final-do-x 1) 17))
            [if-false if-true when-false when-true
             cond-false cond-true cond-test-pending
             [(final-if) (final-when) (final-cond) (final-do)]]
            """
        )
        == [
            [2, 0],
            [None, 1],
            [None, 0],
            [None, 1],
            [2, 0],
            [None, 1],
            [None, 1, 2, 3],
            [7, 11, 13, 17],
        ]
    )


def test_native_with_cases() -> None:
    assert (
        eval_kernel(
            """
            (setv exits [])
            (setv async-exits [])
            (defclass WithTest [object]
              (defn __init__ [self value]
                (setv (. self value) value))
              (defn __enter__ [self]
                (. self value))
              (defn __exit__ [self exc-type exc-value traceback]
                (.append exits (. self value))))
            (defclass SuppressZDE [object]
              (defn __enter__ [self]
                self)
              (defn __exit__ [self exc-type exc-value traceback]
                (and (is-not exc-type None)
                     (issubclass exc-type ZeroDivisionError))))
            (defclass AsyncWithTest []
              (defn __init__ [self value]
                (setv (. self value) value))
              (defn :async __aenter__ [self]
                (. self value))
              (defn :async __aexit__ [self exc-type exc-value traceback]
                (.append async-exits (. self value))))
            (defclass QuietWithTest []
              (defn __init__ [self value]
                (setv (. self value) value))
              (defn __enter__ [self]
                (. self value))
              (defn __exit__ [self exc-type exc-value traceback]))
            (with [a (WithTest 1) b (WithTest 2) _ (WithTest 3)]
              (setv out [a b]))
            (with [(QuietWithTest "unnamed")]
              (setv unnamed-ran True))
            (with [_ (QuietWithTest 14)
                   [destructure-b destructure-d] (QuietWithTest (range 2 5 2))
                   _ (QuietWithTest 15)]
              (setv destructured [destructure-b destructure-d])
              (setv underscore-hidden False)
              (try
                _
                (except [NameError]
                  (setv underscore-hidden True))))
            (defn with-return []
              (with [value (QuietWithTest 42)]
                value))
            (setv pending-manager-calls 0)
            (with [pending-manager (do
                                      (+= pending-manager-calls 1)
                                      (WithTest 4))]
              (setv pending-manager-out pending-manager))
            (with [first-manager (WithTest 5)
                   second-manager (do
                                    (setv saw-first-manager first-manager)
                                    (WithTest 6))]
              (setv manager-order-out [first-manager
                                        second-manager
                                        saw-first-manager]))
            (import asyncio)
            (defn :async async-with-statement []
              (setv async-out [])
              (with [:async async-a (AsyncWithTest 7)
                     :async async-b (AsyncWithTest 8)
                     :async _ (AsyncWithTest 9)]
                (.extend async-out [async-a async-b]))
              (with [:async async-c (AsyncWithTest 10)
                     sync-d (WithTest 11)
                     :async async-e (AsyncWithTest 12)
                     _ (WithTest 13)]
                (.extend async-out [async-c sync-d async-e]))
              async-out)
            (setv async-out (asyncio.run (async-with-statement)))
            (setv suppress-normal (with [(SuppressZDE)] 5))
            (setv suppress-error (with [(SuppressZDE)] (/ 1 0)))
            (setv suppress-error-final (with [(SuppressZDE)] (/ 1 0) 5))
            (defn suppress-function [] (with [(SuppressZDE)] (/ 1 0)))
            (setv suppress-function-value (suppress-function))
            (setv suppress-side-w 7
                  suppress-side-l [])
            (setv suppress-side-w
                  (with [(SuppressZDE)]
                    (.append suppress-side-l suppress-side-w)
                    (/ 1 0)
                    5))
            [out
             unnamed-ran
             destructured
             underscore-hidden
             (with-return)
             pending-manager-calls
             pending-manager-out
             manager-order-out
             async-out
             async-exits
             exits
             suppress-normal
             suppress-error
             suppress-error-final
             suppress-function-value
             suppress-side-w
             suppress-side-l]
            """
        )
        == [
            [1, 2],
            True,
            [2, 4],
            True,
            42,
            1,
            4,
            [5, 6, 5],
            [7, 8, 10, 11, 12],
            [9, 8, 7, 12, 10],
            [3, 2, 1, 4, 6, 5, 13, 11],
            5,
            None,
            None,
            None,
            None,
            [7],
        ]
    )


def test_native_try_cases() -> None:
    assert (
        eval_kernel(
            """
            (setv events [])
            (try
              (.append events "body")
              (raise (KeyError "payload"))
              (except [ValueError e]
                (.append events "value"))
              (except [KeyError e]
                (.append events (get (getattr e "args") 0)))
              (else
                (.append events "else"))
              (finally
                (.append events "finally")))
            (try
              (.append events "clean")
              (except [RuntimeError e]
                (.append events "runtime"))
              (else
                (.append events "else2"))
              (finally
                (.append events "finally2")))
            (setv missing-parts [(try)
                                  (try (except []))
                                  (try (finally))
                                  (try (else))
                                  (try 1)
                                  (try 1 (finally 2))
                                  (try 1 (else 2))])
            (setv multi-value 0)
            (try
              (+= multi-value 1)
              (+= multi-value 2)
              (except [IOError])
              (except []))
            (setv multi-exprs [])
            (defn add-expr [] (.append multi-exprs 1))
            (try
              (add-expr)
              (add-expr)
              (add-expr)
              (except [IOError])
              (else
                (add-expr)))
            (setv raise-reraised False)
            (try
              (try
                (raise IndexError)
                (except [IndexError]
                  (raise)))
              (except [IndexError]
                (setv raise-reraised True)))
            (setv raise-without-active False)
            (try
              (raise)
              (except [RuntimeError]
                (setv raise-without-active True)))
            (setv uncaught-finally "")
            (try
              (try
                (+= uncaught-finally "a")
                (/ 1 0)
                (+= uncaught-finally "b")
                (finally
                  (+= uncaught-finally "c")))
              (except [ZeroDivisionError]))
            (setv raise-from-cause
                  (. (try
                       (raise ValueError :from NameError)
                       (except [e ValueError]
                         e))
                     __cause__))
            (setv except-scope
                  (try
                    (/ 1 0)
                    (except [caught ZeroDivisionError]
                      caught)))
            (setv nonsyntax-call-x 0)
            (try
              (+= nonsyntax-call-x 1)
              ("except" [IOError] (+= nonsyntax-call-x 1))
              (except []))
            (setv nonsyntax-list-x 0)
            (try
              (+= nonsyntax-list-x 1)
              [except [IOError] (+= nonsyntax-list-x 1)]
              (except []))
            [events
             missing-parts
             multi-value
             multi-exprs
             raise-reraised
             raise-without-active
             uncaught-finally
             (is (type raise-from-cause) NameError)
             (isinstance except-scope ZeroDivisionError)
             [nonsyntax-call-x nonsyntax-list-x]]
            """
        )
        == [
            ["body", "payload", "finally", "clean", "else2", "finally2"],
            [None, None, None, None, 1, 1, 2],
            3,
            [1, 1, 1, 1],
            True,
            True,
            "ac",
            True,
            True,
            [2, 2],
        ]
    )


def test_top_level_expression_valued_statement_cases() -> None:
    assert eval_kernel("(try 1)") == 1
    assert eval_kernel("(try (/ 1 0) (except [ZeroDivisionError] 42))") == 42
    assert eval_kernel("(try 1) 2") == 2
    assert (
        eval_kernel(
            """
            (import contextlib)
            (with [value (contextlib.nullcontext 41)]
              (+ value 1))
            """
        )
        == 42
    )
    assert (
        eval_kernel(
            """
            (import contextlib)
            (with [value (contextlib.nullcontext 1)]
              value)
            2
            """
        )
        == 2
    )
    assert eval_kernel("(match 1 1 42)") == 42
    assert (
        eval_kernel(
            """
            (defn function-try-last []
              (try
                (/ 1 0)
                (except [ZeroDivisionError]
                  42)))
            (function-try-last)
            """
        )
        == 42
    )
    assert (
        eval_kernel(
            """
            (import contextlib)
            (defn function-with-last []
              (with [value (contextlib.nullcontext 41)]
                (+ value 1)))
            (function-with-last)
            """
        )
        == 42
    )
    assert eval_kernel("((fn [] (try 1)))") == 1


def test_native_do_del_cases() -> None:
    assert (
        eval_kernel(
            """
            (setv do-empty (is (do) None))
            (setv if-empty (is (if True (do) (do)) None))
            (setv x "a")
            (setv do-value (do (setv x "b") "c"))
            (setv foo 42)
            (del foo)
            (setv del-name-missing False)
            (try
              foo
              (except [NameError]
                (setv del-name-missing True)))
            (setv items (list (range 5)))
            (del (get items 4))
            (del (get items 2))
            [do-empty if-empty do-value x del-name-missing items (del)]
            """
        )
        == [True, True, "c", "b", True, [0, 1, 3], None]
    )


def test_native_expression_result_cases() -> None:
    assert (
        eval_kernel(
            """
            (import asyncio)
            (import contextlib [nullcontext])
            (defclass ACtx []
              (defn __init__ [self value]
                (setv (. self value) value))
              (defn :async __aenter__ [self]
                (. self value))
              (defn :async __aexit__ [self exc-type exc-value traceback]
                False))
            (defn :async async-with-value []
              (return (with [:async value (ACtx 40)]
                        (+ value 2))))
            (defn :async async-with-effect []
              (setv y 0)
              (setv out (with [:async value (ACtx 40)]
                          (setv y 2)
                          (+ value y)))
              [out y])
            (defn :async async-with-order []
              (setv y 0
                    out (with [:async value (ACtx 40)]
                          (setv y 2)
                          (+ value y)))
              [out y])
            (defn :async async-identity [value]
              value)
            (defn :async async-try-value []
              (return
                (+ (try
                     (await (async-identity 10))
                     (except [ValueError e]
                       0))
                   (try
                     (raise (ValueError "bad"))
                     (except [ValueError e]
                       (await (async-identity 11))))
                   (try
                     (raise (ExceptionGroup "bad" [(ValueError "v")]))
                     (except* [ValueError e]
                       (await (async-identity 21)))))))
            (defn :async async-try-effects []
              (setv else-y 0)
              (setv else-out
                    (try
                      (await (async-identity 40))
                      (else
                        (setv else-y 2)
                        (+ 40 else-y))))
              (setv except-y 0)
              (setv except-out
                    (try
                      (raise (ValueError "bad"))
                      (except [ValueError e]
                        (setv except-y 42)
                        (await (async-identity "ok")))))
              (setv star-y 0)
              (setv star-out
                    (try
                      (raise (ExceptionGroup "bad" [(ValueError "v")]))
                      (except* [ValueError e]
                        (setv star-y 2)
                        (await (async-identity 40)))))
              [[else-out else-y] [except-out except-y] [star-out star-y]])
            (setv events [])
            (setv star-events [])
            (setv try-values
                  [(try)
                   (try 1)
                   (try 1 (else 2))
                   (try
                     (raise (NameError "missing"))
                     (except [NameError e]
                       (+ 20 22)))
                   (try 4
                     (finally
                       (.append events "done")))])
            (setv try-star-value
                  (try
                    (raise (ExceptionGroup "bad" [(KeyError "k") (ValueError "v")]))
                    (except* [KeyError e]
                      (.append star-events "key")
                      "key")
                    (except* [ValueError e]
                      (.append star-events "value")
                      "value")
                    (finally
                      (.append star-events "finally"))))
            (setv native-except-values
                  [(try
                     (get "foo" 5)
                     (except [[IndexError NameError]]
                       "type-list")
                     (except []
                       "fallback"))
                   (try
                     (abs "hi")
                     (except [e TypeError]
                       (is (type e) TypeError)))
                   (try
                     (get {1 2} 3)
                     (except [e [KeyError AttributeError]]
                       [(is (type e) KeyError) "name-list"]))
                   (try
                     (raise ValueError)
                     (except [[]]
                       "empty-list-caught")
                     (except []
                       "fallback"))
                   (try
                     (raise (ValueError "type-first"))
                     (except [ValueError err]
                       (get (getattr err "args") 0)))
                   (try
                     (raise (ExceptionGroup "bad" [(ValueError "v")]))
                     (except* [e ValueError]
                       "star-name-first"))])
            (setv outer-x 1)
            (setv outer-y 0)
            (setv outer-x
                  (try
                    (+ "G" "H")
                    (except [NameError]
                      (+ "I" "J"))
                    (else
                      (setv outer-y 1)
                      (assert (= outer-x 1))
                      (+ "K" "L"))))
            (setv except-y 0
                  except-out
                  (try
                    (raise (ValueError "bad"))
                    (except [ValueError err]
                      (setv except-y 42)
                      "ok")))
            (setv pending-order [])
            (setv pending-a 1
                  pending-b (try
                              (.append pending-order pending-a)
                              (setv pending-a 2)
                              3))
            (setv with-value
                  (with [value (nullcontext 41)]
                    (+ value 1)))
            (setv with-effect-y 0)
            (setv with-effect-out
                  (with [value (nullcontext 40)]
                    (setv with-effect-y 2)
                    (+ value with-effect-y)))
            (setv with-order-y 0
                  with-order-out
                  (with [value (nullcontext 40)]
                    (setv with-order-y 2)
                    (+ value with-order-y)))
            [try-values events try-star-value star-events
             native-except-values [outer-x outer-y] [except-out except-y]
             [pending-a pending-b pending-order]
             with-value [with-effect-out with-effect-y]
             [with-order-out with-order-y]
             (asyncio.run (async-with-value))
             (asyncio.run (async-with-effect))
             (asyncio.run (async-with-order))
             (asyncio.run (async-try-value))
             (asyncio.run (async-try-effects))]
            """
        )
        == [
            [None, 1, 2, 42, 4],
            ["done"],
            "value",
            ["key", "value", "finally"],
            [
                "type-list",
                True,
                [True, "name-list"],
                "fallback",
                "type-first",
                "star-name-first",
            ],
            ["KL", 1],
            ["ok", 42],
            [2, 3, [1]],
            42,
            [42, 2],
            [42, 2],
            42,
            [42, 2],
            [42, 2],
            42,
            [[42, 2], ["ok", 42], [40, 2]],
        ]
    )


def test_native_helper_scope_and_pending_deep_cases() -> None:
    assert (
        eval_kernel(
            """
            (import contextlib [nullcontext])
            (setv log [])
            (defn mark [label value]
              (.append log label)
              value)

            (setv if-x 0
                  if-value
                  (if (mark "if-test" True)
                      (do
                        (setv if-x (mark "if-body" 1))
                        (+ if-x 1))
                      (do
                        (setv if-x 99)
                        0)))
            (setv when-x 0
                  when-value
                  (when (mark "when-test" True)
                    (setv when-x (mark "when-body" 2))
                    (+ when-x 1)))
            (setv cond-x 0
                  cond-value
                  (cond
                    (mark "cond-skip" False) (setv cond-x 99)
                    (mark "cond-take" True)
                      (do
                        (setv cond-x (mark "cond-body" 3))
                        (+ cond-x 1))
                    True -1))
            (setv do-x 0
                  do-value
                  (do
                    (setv do-x (mark "do-body" 4))
                    (+ do-x 1)))
            (setv and-x 0
                  and-value
                  (and (mark "and-a" 5)
                       (do
                         (setv and-x (mark "and-b" 6))
                         (+ and-x 1))))
            (setv and-skip-x 0
                  and-skip-value
                  (and (mark "and-skip-a" 0)
                       (setv and-skip-x 1)))
            (setv or-x 0
                  or-value
                  (or (mark "or-a" 0)
                      (do
                        (setv or-x (mark "or-b" 7))
                        (+ or-x 1))))
            (setv or-skip-x 0
                  or-skip-value
                  (or (mark "or-skip-a" 8)
                      (setv or-skip-x 1)))
            (setv let-body-leak "outer"
                  let-value
                  (let [inner (mark "let-bind" 8)]
                    (setv let-body-leak (mark "let-body" inner))
                    (+ inner 1)))
            (setv for-seen []
                  for-value
                  (for [i (range 3)]
                    (.append for-seen (mark "for-body" i))
                    (else
                      (mark "for-else" None))))
            (setv while-i 0
                  while-seen []
                  while-value
                  (while (< while-i 2)
                    (.append while-seen (mark "while-body" while-i))
                    (+= while-i 1)))
            (setv try-x 0
                  try-value
                  (try
                    (raise (ValueError "bad"))
                    (except [ValueError e]
                      (setv try-x (mark "try-except" 11))
                      (+ try-x 1))))
            (setv with-x 0
                  with-value
                  (with [v (nullcontext (mark "with-enter" 10))]
                    (setv with-x (mark "with-body" v))
                    (+ with-x 1)))
            (setv match-x 0
                  match-value
                  (match (mark "match-subject" [12])
                         [a]
                           (do
                             (setv match-x (mark "match-body" a))
                             (+ match-x 1))))
            (setv comp-x 0
                  comp-value
                  (lfor n (range 3)
                        :do (setv comp-x (mark "comp-do" n))
                        :if (> comp-x 0)
                        (+ comp-x 1)))
            (setv nested-x 0
                  nested-value
                  (if True
                      (try
                        (with [v (nullcontext 13)]
                          (let [offset 1]
                            (setv nested-x (mark "nested-body" (+ v offset)))
                            (while (< nested-x 16)
                              (+= nested-x 1))
                            nested-x)))
                      0))

            (defn non-final-effects []
              (setv seen [])
              (if True
                  (.append seen "if")
                  (.append seen "bad-if"))
              (when True
                (.append seen "when")
                99)
              (cond
                True (.append seen "cond"))
              (do
                (.append seen "do")
                100)
              (try
                (.append seen "try")
                101)
              (with [v (nullcontext "with")]
                (.append seen v)
                102)
              (match 1
                     1 (.append seen "match"))
              (for [i [1]]
                (.append seen "for"))
              (while False
                (.append seen "bad-while"))
              seen)

            (setv pending [])
            (setv f-result f"{(do (.append pending "f-string") 42)}")
            (setv cmp-result
                  (< (do (.append pending "cmp-a") 1)
                     (do (.append pending "cmp-b") 2)
                     (do (.append pending "cmp-c") 3)))
            (setv comp-iter-result
                  (lfor x (do
                            (.append pending "iterable")
                            [1 2])
                        (do
                          (.append pending (+ "item-" (str x)))
                          x)))
            (setv guard-result
                  (match 5
                         x :if (do
                                  (.append pending "guard")
                                  (> x 3))
                           (+ x 1)))

            [[if-value if-x]
             [when-value when-x]
             [cond-value cond-x]
             [do-value do-x]
             [and-value and-x]
             [and-skip-value and-skip-x]
             [or-value or-x]
             [or-skip-value or-skip-x]
             [let-value let-body-leak]
             [for-value for-seen]
             [while-value while-seen while-i]
             [try-value try-x]
             [with-value with-x]
             [match-value match-x]
             [comp-value comp-x]
             [nested-value nested-x]
             (non-final-effects)
             [f-result cmp-result comp-iter-result guard-result pending]
             log]
            """
        )
        == [
            [2, 1],
            [3, 2],
            [4, 3],
            [5, 4],
            [7, 6],
            [0, 0],
            [8, 7],
            [8, 0],
            [9, 8],
            [None, [0, 1, 2]],
            [None, [0, 1], 2],
            [12, 11],
            [11, 10],
            [13, 12],
            [[2, 3], 2],
            [16, 16],
            ["if", "when", "cond", "do", "try", "with", "match", "for"],
            [
                "42",
                True,
                [1, 2],
                6,
                ["f-string", "cmp-a", "cmp-b", "cmp-c", "iterable", "item-1", "item-2", "guard"],
            ],
            [
                "if-test",
                "if-body",
                "when-test",
                "when-body",
                "cond-skip",
                "cond-take",
                "cond-body",
                "do-body",
                "and-a",
                "and-b",
                "and-skip-a",
                "or-a",
                "or-b",
                "or-skip-a",
                "let-bind",
                "let-body",
                "for-body",
                "for-body",
                "for-body",
                "for-else",
                "while-body",
                "while-body",
                "try-except",
                "with-enter",
                "with-body",
                "match-subject",
                "match-body",
                "comp-do",
                "comp-do",
                "comp-do",
                "nested-body",
            ],
        ]
    )


def test_native_setv_cases() -> None:
    assert (
        eval_kernel(
            """
            (setv :chain [a b c] 3)
            (setv v1 1
                  :chain [v2 v3] 2
                  v4 4
                  :chain [v5 v6 v7] 5)
            (setv :chain [[y #* z w] x [aa bb cc dd]]
                  "abcd")
            (setv l (* [0] 5))
            (setv calls [])
            (defn f [i]
              (.append calls [i (list l)])
              i)
            (setv :chain [(get l (f 1)) (get l (f 2)) (get l (f 3))]
                  (f 9))
            (defn none-value? [value]
              (is value None))
            (setv setv-x 1)
            (setv setv-arg-result (none-value? (setv setv-x 2)))
            (setv setv-p (setv setv-q 12))
            (setv setv-empty-result (none-value? (setv)))
            (setv setv-chain-result
                  (none-value? (setv :chain [setv-chain-a setv-chain-b] 3)))
            (setv statement-defn-result
                  (none-value? (setv statement-defn-value
                                     (defn statement-function [] 7))))
            (setv statement-defclass-result
                  (none-value? (setv statement-defclass-value
                                     (defclass StatementClass))))
            (setv statement-for-seen [])
            (setv statement-for-result
                  (none-value? (setv statement-for-value
                                     (for [i (range 3)]
                                       (.append statement-for-seen i)))))
            (setv statement-assert-result
                  (none-value? (setv statement-assert-value
                                     (assert True))))
            (setv statement-pass-result
                  (none-value? (pass)))
            [[a b c]
             [v1 v2 v3 v4 v5 v6 v7]
             [y z w x aa bb cc dd]
             calls
             l
             [setv-arg-result setv-x]
             [setv-p setv-q]
             setv-empty-result
             [setv-chain-result setv-chain-a setv-chain-b]
             [statement-defn-result statement-defn-value (statement-function)]
             [statement-defclass-result statement-defclass-value (. StatementClass __name__)]
             [statement-for-result statement-for-value statement-for-seen]
             [statement-assert-result statement-assert-value statement-pass-result]]
            """
        )
        == [
            [3, 3, 3],
            [1, 2, 2, 4, 5, 5, 5],
            ["a", ["b", "c"], "d", "abcd", "a", "b", "c", "d"],
            [
                [9, [0, 0, 0, 0, 0]],
                [1, [0, 0, 0, 0, 0]],
                [2, [0, 9, 0, 0, 0]],
                [3, [0, 9, 9, 0, 0]],
            ],
            [0, 9, 9, 9, 0],
            [True, 2],
            [None, 12],
            True,
            [True, 3, 3],
            [True, None, 7],
            [True, None, "StatementClass"],
            [True, None, [0, 1, 2]],
            [True, None, True],
        ]
    )


def test_native_unpacking_cases() -> None:
    assert (
        eval_kernel(
            """
            (setv l [1 2 3])
            (setv p [4 5])
            (setv d1 {"a" 1 "b" 2})
            (setv d2 {"c" 3 "d" 4})
            (setv [head #* tail] [10 20 12])
            (setv slice-target [0 1 2 3])
            (setv (cut slice-target 1 3) [20 22])
            (setv slice-delete [0 1 2 3])
            (del (cut slice-delete 1 3))
            (setv multi-target [[1 2] [3 4]])
            (setv (get multi-target 1 0) 9)
            (setv whole-slice [1 2 3])
            (setv (cut whole-slice) [4 5])
            (setv prefix-slice [1 2 3])
            (setv (cut prefix-slice 2) [9])
            (setv whole-delete [1 2 3])
            (del (cut whole-delete))
            [(= ["x" #* l "y" #* p] ["x" 1 2 3 "y" 4 5])
             (= #("x" #* l) #("x" 1 2 3))
             (= #{"x" #* l #* p} #{"x" 1 2 3 4 5})
             (= {"z" 0 #** d1 #** d2}
                {"z" 0 "a" 1 "b" 2 "c" 3 "d" 4})
             head
             tail
             slice-target
             slice-delete
             (get [[1 2 3] [4 5 6] [7 8 9]] 1 2)
             (get {"x" {"y" {"z" 12}}} "x" "y" "z")
             multi-target
             (cut "abcdef")
             (cut "abcdef" 3)
             (cut "abcdef" -2)
             (cut "abcdef" 3 None)
             (cut "abcdef" 3 5)
             (cut "abcdef" 0 None 2)
             whole-slice
             prefix-slice
             whole-delete
             (dfor pair [["a" 1] ["b" 2]]
                   #** {(get pair 0) (get pair 1)})
             (+ #* l #* p)
             (* #* [2 3 7])
             (* #* [])
             (and #* l)
             (or #* [False 0 42])
             (< #* l)
             (= #* [1 1 1])
             (= #* [1 2 1])
             (+ #* [[1] [2]])]
            """
        )
        == [
            True,
            True,
            True,
            True,
            10,
            [20, 12],
            [0, 20, 22, 3],
            [0, 3],
            6,
            12,
            [[1, 2], [9, 4]],
            "abcdef",
            "abc",
            "abcd",
            "def",
            "de",
            "ace",
            [4, 5],
            [9, 3],
            [],
            {"a": 1, "b": 2},
            15,
            42,
            1,
            3,
            42,
            True,
            True,
            False,
            [1, 2],
        ]
    )


def test_native_let_cases() -> None:
    assert (
        eval_kernel(
            """
            [(let [(annotate x int) 42] x)
             (let [[(annotate left int) right] [20 22]]
               (+ left right))
             (let [value 40]
               (setv bumped (+ value 2))
               bumped)
             (let [x 3 y 4]
               (match x
                      y (= x y)))
             (let [x 1 y 2]
               (match [5 6]
                      [x y] [x y]))
             (let [x "foo"
                   y "bar"
                   x (+ x y)
                   y (+ y x)
                   x (+ x x)]
               [x y])
             (let [a "a"
                   b (+ a "b")
                   c (+ b "c")]
               c)
             (let [[a b] [1 2]
                   [lhead #* ltail] (range 3)
                   #(thead #* ttail) (range 3)
                   [nhead #* #(c #* nrest)] [0 1 2]]
               [a b lhead ltail thead ttail nhead c nrest])
             (let [[a b] ["foo" "bar"]
                   [a #* c] (range 3)
                   [head #* tail] [a b c]]
               [a b c head tail])]
            """
        )
        == [
            42,
            42,
            42,
            True,
            [5, 6],
            ["foobarfoobar", "barfoobar"],
            "abc",
            [1, 2, 0, [1, 2], 0, [1, 2], 0, 1, [2]],
            [0, "bar", [1, 2], 0, ["bar", [1, 2]]],
        ]
    )


def test_native_let_scope_leakage_cases() -> None:
    assert (
        eval_kernel(
            """
            (import contextlib [nullcontext])
            (let [hidden-setv 1]
              (setv hidden-setv 2))
            (setv hidden-setv-missing False)
            (try
              hidden-setv
              (except [NameError]
                (setv hidden-setv-missing True)))
            (let [x 1] (setv leaked-setv 2))
            (let [x 1] (defn leaked-function [] 42))
            (let [x 1] (defclass LeakedClass []))
            (let [types 6] (import types))
            (let [sqrt 6] (import math [sqrt]))
            (let [x 1] (for [leaked-for [42]] None))
            (let [x 1] (with [leaked-with (nullcontext 42)] None))
            (let [x 1] (match [20 22] [leaked-left leaked-right] None))
            (let [hidden-for 1]
              (for [hidden-for [42]] None))
            (setv hidden-for-missing False)
            (try
              hidden-for
              (except [NameError]
                (setv hidden-for-missing True)))
            [hidden-setv-missing
             leaked-setv
             (leaked-function)
             (. LeakedClass __name__)
             (. (type types) __name__)
             (sqrt 1764)
             leaked-for
             leaked-with
             (+ leaked-left leaked-right)
             hidden-for-missing]
            """
        )
        == [True, 2, 42, "LeakedClass", "module", 42.0, 42, 42, 42, True]
    )


def test_native_let_upstream_scope_boundary_cases() -> None:
    assert (
        eval_kernel(
            """
            (import contextlib [nullcontext])
            (import types)
            (import hy)

            (setv x 100)
            (setv let-comprehensions
                  [(let [x 10]
                     [(lfor x (range 5) :if (> x 1) x) x])
                   (let [x 15]
                     [(lfor y (range 3) :setv x (* y 2) (+ y x)) x])
                   (let [x 20]
                     [(lfor z "abc" :do (setv x (.upper z)) (+ z x)) x])
                   (let [x 25
                         l []]
                     (for [x (range 5) :if (> x 1)]
                       (.append l x))
                     [l x])
                   x])

            (setv a-symbol 'a)
            (setv let-quasiquote
                  (let [a "x"]
                    [(= 'a a-symbol)
                     (= `a a-symbol)
                     (= (hy.as-model `(foo ~a))
                        '(foo "x"))
                     (= (hy.as-model `(foo ~@[a]))
                        '(foo "x"))]))

            (setv let-except None
                  let-except-after None)
            (let [foo 42
                  bar 33]
              (try
                (/ 1 0)
                (except [foo Exception]
                  (setv let-except [(isinstance foo Exception) bar])))
              (setv let-except-after foo))

            (setv let-with None
                  let-with-after None)
            (let [foo 42]
              (with [foo (nullcontext 99)]
                (setv let-with foo))
              (setv let-with-after foo))

            (setv foo 42
                  mutation-error False
                  mutation-inside None)
            (let [foo 12
                  bar 13]
              (setv foo 14)
              (del foo)
              (try
                foo
                (except [UnboundLocalError]
                  (setv mutation-error True)))
              (setv foo 16)
              (setv [foo bar baz] [1 2 3])
              (setv mutation-inside [foo bar baz]))
            (setv let-mutation [mutation-error foo baz mutation-inside])

            (for [break-x (range 3)]
              (let [done (% break-x 2)]
                (when done
                  (break))))
            (setv let-break break-x)

            (setv let-continue [])
            (for [continue-x (range 10)]
              (let [odd (% continue-x 2)]
                (when odd
                  (continue))
                (.append let-continue continue-x)))

            (defn grind []
              (yield 0)
              (let [a 1
                    b 2]
                (yield a)
                (yield b)))
            (setv let-yield (tuple (grind)))

            (defn get-answer []
              (let [answer 42]
                (return answer)))
            (setv let-return (get-answer))

            (let [math 6]
              (import math)
              (setv let-import [(in "math" (vars))
                                (isinstance math types.ModuleType)]))

            (let [captured 40
                  Base object]
              (defn let-fn [x]
                (+ captured x))
              (defclass LetClass [Base]
                (setv label captured)
                (defn value [self]
                  LetClass.label)))
            (setv let-nested [(let-fn 2)
                              LetClass.label
                              (.value (LetClass))])

            (setv dot-foo (fn [])
                  dot-foo.a 42)
            (let [a 1
                  b []
                  bar (fn [])]
              (setv bar.a 13)
              (setv (. bar a) 14)
              (.append b 2)
              (setv let-dot [bar.a a b dot-foo.a (. dot-foo a)
                             (. [1 2 3] [a])]))

            (setv arg-calls [])
            (defn default-maker []
              (.append arg-calls 1))
            (let [a 1]
              (defn defaulted [[b (default-maker)]]
                5))
            (setv let-arg-eval [(defaulted) arg-calls])

            (let [fuel 50]
              (defn propulse [distance]
                (nonlocal fuel)
                (-= fuel distance))
              (defn check-fuel []
                fuel))
            (setv fuel-before (check-fuel))
            (propulse 3)
            (setv let-nonlocal [fuel-before (check-fuel)])

            (setv hound 43)
            (defn bay []
              (let [unrelated 99]
                (global hound)
                (setv hound 2))
              (setv hound 3))
            (bay)
            (setv let-global hound)

            (defmacro triple [a]
              (setv g!a (hy.gensym a))
              `(do
                 (setv ~g!a ~a)
                 (+ ~g!a ~g!a ~g!a)))
            (defmacro ap-triple []
              '(+ a a a))
            (setv let-macros
                  (let [a 1
                        b (triple a)
                        c (ap-triple)]
                    [(triple a) (ap-triple) b c]))

            (setv hidden-missing False)
            (let [hidden 1]
              (setv hidden 2))
            (try
              hidden
              (except [NameError]
                (setv hidden-missing True)))
            (let [x 1]
              (setv leaked-setv 2))

            [let-comprehensions
             let-quasiquote
             let-except
             let-except-after
             let-with
             let-with-after
             let-mutation
             let-break
             let-continue
             let-yield
             let-return
             let-import
             let-nested
             let-dot
             let-arg-eval
             let-nonlocal
             let-global
             let-macros
             hidden-missing
             leaked-setv]
            """
        )
        == [
            [
                [[2, 3, 4], 10],
                [[0, 3, 6], 15],
                [["aA", "bB", "cC"], "C"],
                [[2, 3, 4], 4],
                100,
            ],
            [True, True, True, True],
            [True, 33],
            42,
            99,
            99,
            [True, 42, 3, [1, 2, 3]],
            1,
            [0, 2, 4, 6, 8],
            (0, 1, 2),
            42,
            [True, True],
            [42, 40, 40],
            [14, 1, [2], 42, 42, 2],
            [5, [1]],
            [50, 47],
            3,
            [3, 3, 3, 3],
            True,
            2,
        ]
    )


def main() -> None:
    test_compiler_ast_focused_cases()
    test_lambda_lists()
    test_native_annotation_cases()
    test_native_type_parameter_version_gate_cases()
    test_native_illegal_binding_cases()
    test_statement_fn_closure()
    test_module_docstring()
    test_native_fstring_cases()
    test_native_quoted_string_model_cases()
    test_native_reader_timed_pragma_cases()
    test_native_tstring_compile_gate_cases()
    test_native_supported_compile_error_messages()
    test_native_hy_repr_and_model_cases()
    test_native_quasiquote_cases()
    test_eval_source_hy_eval_module_context()
    test_native_hy_eval_argument_cases()
    test_native_hy_eval_upstream_remaining_cases()
    test_native_keyword_cases()
    test_native_mangling_special_form_alias_cases()
    test_native_operator_edge_cases()
    test_native_operator_upstream_parity_cases()
    test_native_augassign_cases()
    test_native_comparison_edge_cases()
    test_native_chainc_cases()
    test_native_setx_scope_cases()
    test_native_setv_setx_unpack_upstream_cases()
    test_native_assignment_failure_and_order_cases()
    test_collection_pending_evaluation_order_cases()
    test_native_call_argument_order_cases()
    test_comprehension_clauses()
    test_async_comprehension_clauses()
    test_native_comprehension_upstream_remaining_cases()
    test_async_generator_functions()
    test_native_async_function_upstream_remaining_cases()
    test_yield_from_cases()
    test_generator_final_return_cases()
    test_native_function_name_and_lambda_list_edge_cases()
    test_native_function_syntax_error_cases()
    test_native_import_cases()
    test_native_require_macro_cases()
    test_native_recursive_require_star_cases()
    test_native_macro_namespace_shadowing_cases()
    test_native_first_class_global_macro_cases()
    test_native_first_class_local_macro_cases()
    test_native_defmacro_lambda_list_and_docstring_cases()
    test_native_macro_upstream_value_and_phase_cases()
    test_native_model_pattern_macro_cases()
    test_native_defmacro_core_shadow_warning_cases()
    test_native_defmacro_error_wrapping_cases()
    test_native_hy_macroexpand_cases()
    test_native_reader_macro_delegation_cases()
    test_native_reader_behavior_delegation_cases()
    test_native_relative_require_cases()
    test_native_hy_R_one_shot_require_cases()
    test_native_hy_I_importer_cases()
    test_native_inspect_metadata_cases()
    test_native_dot_chain_cases()
    test_native_defclass_cases()
    test_native_decorator_cases()
    test_ellipsis_constant()
    test_match_patterns()
    test_native_match_expression_syntax()
    test_native_match_failure_cases()
    test_native_match_side_effect_order_cases()
    test_native_match_upstream_runtime_cases()
    test_native_control_flow_cases()
    test_native_nonlocal_cases()
    test_native_logic_short_circuit_cases()
    test_native_compiler_boolop_shape_cases()
    test_native_conditional_expression_statement_cases()
    test_native_with_cases()
    test_native_try_cases()
    test_top_level_expression_valued_statement_cases()
    test_native_do_del_cases()
    test_native_expression_result_cases()
    test_native_helper_scope_and_pending_deep_cases()
    test_native_setv_cases()
    test_native_unpacking_cases()
    test_native_let_cases()
    test_native_let_scope_leakage_cases()
    test_native_let_upstream_scope_boundary_cases()
    print("native_subset: ok")


if __name__ == "__main__":
    main()
