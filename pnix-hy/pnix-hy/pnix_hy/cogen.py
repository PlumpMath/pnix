"""pnix_hy.cogen -- the EFFICIENT cogen (3rd Futamura projection done right), proposal 0029.

The 2026-07-02 deep-research audit (docs/audits/2026-07-02-cogen-stagepoly-research.md) confirmed
what four experiments had shown: a cogen produced by SELF-APPLICATION (`tower.build_cogen` /
`run_cogen`) is pathologically bloated -- the self-applied specializer drags an embedded
interpreter + a universal value datatype + environment/tag manipulation into every generating
extension, so running it is intractable (>150s for even a 1-branch interpreter) regardless of
runtime. The canonical fix (Birkedal & Welinder PLILP'94; Thiemann "Cogen in Six Lines" ICFP'96;
Glueck & Joergensen; Leuschel logen) is the "cogen approach": do NOT self-apply -- hand-write the
compiler generator directly as a thin layer over a good binding-time analysis / polyvariant
specializer, so it manipulates only syntax trees and contains no interpreter.

pnix ALREADY has that hand-written generator: `tower.poly_specialize` (the native polyvariant
specializer). This module exposes the cogen approach as an API:

  * `generating_extension(source, dynamic_vars)` -> a program-specific generating extension: a
    reusable callable `gex(static_env) -> residual` that specializes `source` given static
    bindings, WITHOUT self-application. This is the "cog p" of the literature.
  * `compiler_from_interpreter(interp)` -> the generating extension of an interpreter IS a
    compiler: `compiler(program) -> target`. Fast (milliseconds), where self-applied cogen was
    intractable.

Additive, host/pnix specializer lane only. The SACRED 545x4 mirror and `pnix_runtime` are
untouched -- this introduces a new artifact, it does not modify the reference evaluator.
"""

from __future__ import annotations

from typing import Any, Callable

from . import pnix_runtime as rt
# Absolute import: bypasses the package's lazy __getattr__ (see the note in
# pnix_mirror.py).
import pnix_hy.tower as tower


def generating_extension(source: str, dynamic_vars: tuple[str, ...] = ()) -> Callable[..., str]:
    """The cogen approach: return a program-specific GENERATING EXTENSION for `source` -- a
    reusable callable `gex(static_env={}) -> residual_source` that, given static bindings, emits
    the residual by running the hand-written native polyvariant specializer (NO self-application,
    no interpreter dragged along). `static_env` maps a free variable name to a pnix-data value.
    """
    def gex(static_env: dict[str, Any] | None = None) -> str:
        static_env = static_env or {}
        prelude = "".join(f"{k} = {tower._pnix_literal(v)}; " for k, v in static_env.items())
        src = f"let {prelude}in {source}" if prelude else source
        return tower.poly_specialize(src, dynamic_vars)["residual"]

    return gex


def cogen(source: str, dynamic_vars: tuple[str, ...] = ()) -> dict[str, Any]:
    """Produce the generating extension of `source` (the efficient 3rd-projection artifact).
    Returns the callable plus metadata. Contrast `tower.build_cogen`/`run_cogen`, which take the
    research-confirmed-bad self-application route."""
    return {"schema": "pnix-hy.cogen.v1", "approach": "hand-written (no self-application)",
            "generating_extension": generating_extension(source, dynamic_vars),
            "dynamic_vars": list(dynamic_vars)}


def compiler_from_interpreter(interpreter_source: str, program_var: str = "prog",
                              dynamic_vars: tuple[str, ...] = ("input",)) -> Callable[[Any], str]:
    """The generating extension of an interpreter IS a compiler (2nd Futamura projection via the
    cogen approach): returns `compiler(program_data) -> target_source`, specializing the
    interpreter to a fixed program with the runtime input(s) dynamic. Fast where self-applied
    cogen was intractable."""
    gex = generating_extension(interpreter_source, dynamic_vars)
    return lambda program: gex({program_var: program})


def compiler_source(interpreter_source: str) -> str:
    """0029 P2: the generating extension of an interpreter AS STANDALONE PNIX SOURCE -- a portable
    pnix compiler (a string), not a host closure. Built by the PNIX-expressed specializer
    (`tower.poly_mix_in_pnix`, 0026 M5b): specialize the interpreter with the program STATIC via
    `senv` and the runtime input dynamic. Evaluate the returned source with
    `senv = { prog = <program-data>; }` to get the target (see `compile_with`)."""
    from . import tower  # noqa: PLC0415

    enc = tower._encode(rt.ast_stable(rt.parse(interpreter_source)))
    obj = tower.MIX_IN_PNIX.strip()[: -len("in mix")] + f"ast = {tower._pnix_literal(enc)}; in mix ast senv"
    return tower.poly_mix_in_pnix(obj, {})["residual"]


def compile_with(compiler_src: str, program: Any, program_var: str = "prog") -> str:
    """Run a standalone pnix `compiler_src` (from `compiler_source`) on a program, returning the
    target as pnix source. Pure pnix evaluation -- the compiler is a real pnix artifact."""
    from . import tower  # noqa: PLC0415

    senv = "{ " + program_var + " = " + tower._pnix_literal(tower._wrap_node(program)) + "; }"
    return tower._decode_full(rt.stable_data(rt.eval_source(f"let senv = {senv}; in ({compiler_src})")))


# a tiny arithmetic interpreter used by the report (program static, input dynamic)
_INT = ('let int = prog: env: if prog.tag == "num" then prog.value '
        'else if prog.tag == "arg" then env '
        'else if prog.tag == "add" then (int prog.l env) + (int prog.r env) '
        'else if prog.tag == "mul" then (int prog.l env) * (int prog.r env) '
        'else 0; in int prog input')


def cogen_report() -> dict[str, Any]:
    """Self-check (proposal 0029): the hand-written cogen approach (1) produces generating
    extensions that agree with direct specialization, and (2) yields a WORKING compiler from an
    interpreter FAST -- the case the self-applied cogen (`run_cogen`) could not do in-budget."""
    try:
        import time  # noqa: PLC0415 - local, only for the efficiency assertion

        # (1) generic generating extension agrees with direct specialization + is correct
        gex = generating_extension("(a * x) + b", ("x",))
        r_gex = gex({"a": 3, "b": 4})
        r_direct = tower.poly_specialize("let a = 3; b = 4; in (a * x) + b", ("x",))["residual"]
        generic_ok = (r_gex == r_direct
                      and all(rt.eval_source(f"let x = {i}; in {r_gex}") == 3 * i + 4 for i in (0, 5, 9)))

        # (2) the generating extension of the interpreter is a fast, correct compiler
        compiler = compiler_from_interpreter(_INT)
        p1 = {"tag": "add", "l": {"tag": "mul", "l": {"tag": "arg"},
                                  "r": {"tag": "num", "value": 3}}, "r": {"tag": "num", "value": 4}}
        p2 = {"tag": "mul", "l": {"tag": "add", "l": {"tag": "arg"},
                                  "r": {"tag": "num", "value": 1}}, "r": {"tag": "num", "value": 10}}
        t0 = time.perf_counter()
        t1 = compiler(p1)
        t2 = compiler(p2)
        elapsed = time.perf_counter() - t0
        compiler_ok = (
            all(rt.eval_source(f"let input = {i}; in {t1}") == i * 3 + 4 for i in (0, 5, 9))
            and all(rt.eval_source(f"let input = {i}; in {t2}") == (i + 1) * 10 for i in (0, 5, 9)))
        # generously in-budget: hand-written cogen is ~ms; self-applied cogen was >150s
        in_budget = elapsed < 30.0

        # (3) P2: the STANDALONE PNIX compiler source compiles programs by pure pnix evaluation
        csrc = compiler_source(_INT)
        ts1 = compile_with(csrc, p1)
        ts2 = compile_with(csrc, p2)
        standalone_ok = (
            isinstance(csrc, str) and len(csrc) > 0
            and all(rt.eval_source(f"let input = {i}; in {ts1}") == i * 3 + 4 for i in (0, 5, 9))
            and all(rt.eval_source(f"let input = {i}; in {ts2}") == (i + 1) * 10 for i in (0, 5, 9)))

        # (4) Q1-1 sharing-safe unfolding: a dynamic let-binding used non-affinely in the body is
        # residualized as a SHARED let (not inlined at each use, which would duplicate the dynamic
        # computation). Research: call-by-need loses no sharing this way (Brown & Palsberg POPL'18).
        share_res = tower.poly_specialize("let y = x * x + 7; in y + y", ("x",))["residual"]
        sharing_safe = (share_res.count("x * x") == 1
                        and all(rt.eval_source(f"let x = {i}; in {share_res}") == (i * i + 7) * 2
                                for i in (0, 3, 9)))

        # (5) Q1-2 "The Trick": a select over a dynamic `if` with attrset branches folds to a
        # select-free `if` (static fields recovered) instead of residualizing the whole attrset.
        trick_res = tower.poly_specialize("(if b then { v = 1; } else { v = 2; }).v", ("b",))["residual"]
        # and the LET-BOUND case: `let r = if b then {..} else {..}; in r.a + r.c` also folds
        # (the if is distributed over product structure, so both selects recover static fields).
        lb_res = tower.poly_specialize(
            "let r = if b then { a = 10; c = 20; } else { a = 30; c = 40; }; in r.a + r.c", ("b",))["residual"]
        eta_trick = ("{" not in trick_res and "{" not in lb_res
                     and rt.eval_source("let b = true; in " + trick_res) == 1
                     and rt.eval_source("let b = false; in " + trick_res) == 2
                     and rt.eval_source("let b = true; in " + lb_res) == 30
                     and rt.eval_source("let b = false; in " + lb_res) == 70)

        ready = bool(generic_ok and compiler_ok and in_budget and standalone_ok
                     and sharing_safe and eta_trick)
        return {"schema": "pnix-hy.cogen.report.v0", "ready": ready, "available": True,
                "generic_generating_extension": generic_ok, "compiler_from_interpreter": compiler_ok,
                "compile_seconds": round(elapsed, 4), "in_budget": in_budget,
                "standalone_pnix_compiler": standalone_ok, "sharing_safe_unfold": sharing_safe,
                "eta_expansion_trick": eta_trick,
                "approach": "hand-written (no self-application)",
                "note": "efficient 3rd-projection route; self-applied run_cogen is the bloated one"}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.cogen.report.v0", "ready": False, "available": False,
                "error": f"{type(exc).__name__}: {exc}"}


def pe_size_report() -> dict[str, Any]:
    """Self-check + MEASUREMENT (0029 / research backlog Q1-4): quantifies the size effect of the
    sharing-safe unfolding (Q1-1) and eta "The Trick" (Q1-2) wins. A shared dynamic subexpression
    used k times appears EXACTLY ONCE in the residual (naive inlining would duplicate it k times),
    so residual size is asymptotically flat in k; and a dynamic-if over product structure folds
    away the attrset. This is the closest measurable answer to Q1-4 for call-by-need pnix."""
    try:
        from . import tower  # noqa: PLC0415

        sizes: dict[int, dict[str, int]] = {}
        for k in (2, 4, 8, 16):
            body = " + ".join(["y"] * k)
            res = tower.poly_specialize(f"let y = x * x + 7; in {body}", ("x",))["residual"]
            sizes[k] = {"residual_len": len(res), "shared_subexpr_count": res.count("x * x")}
            # parity: shared residual still computes k*(x*x+7)
            assert all(rt.eval_source(f"let x = {i}; in {res}") == (i * i + 7) * k for i in (0, 3))
        # Q1-1: the dynamic subexpr is shared (count == 1) no matter how many uses -> size flat in k
        sharing_flat = all(v["shared_subexpr_count"] == 1 for v in sizes.values())
        naive_would_duplicate = {k: k for k in sizes}  # naive inlining count == k
        # Q1-2: dynamic-if over product/list folds (no attrset/list-of-attrs residualized)
        eta_res = tower.poly_specialize(
            "let r = if b then { a = 1; c = 2; } else { a = 3; c = 4; }; in r.a + r.c", ("b",))["residual"]
        eta_folds = "{" not in eta_res
        # I1 bounded static variation: a LARGE cond over a many-field structure must NOT be
        # duplicated into every field (distribution is skipped -> single residual if, cond once).
        big = ("let cc = (x + x + x + x + x + x + x + x + x + x + x + x + x + x + x + x); "
               "in (if cc > 100 then { a = 1; b = 2; c = 3; d = 4; e = 5; } "
               "else { a = 6; b = 7; c = 8; d = 9; e = 10; }).a")
        bsv_res = tower.poly_specialize(big, ("x",))["residual"]
        bounded_variation = (bsv_res.count("x + x") <= 2
                             and rt.eval_source("let x = 20; in " + bsv_res) == 1
                             and rt.eval_source("let x = 1; in " + bsv_res) == 6)
        # I4 let-insertion: a NON-TRIVIAL cond distributed across >=2 fields is HOISTED once
        # (shared), not duplicated -- so distribution folds the fields without copying the cond.
        li = tower.poly_specialize(
            "let r = if (x * x + x) > 5 then { a = 1; c = 2; } else { a = 3; c = 4; }; in r.a + r.c",
            ("x",))
        li_res = li["residual"]
        let_insertion = (li.get("hoisted_bindings", 0) >= 1 and li_res.count("x * x") == 1
                         and "{" not in li_res
                         and all(rt.eval_source(f"let x = {i}; in {li_res}") == (3 if (i * i + i) > 5 else 7)
                                 for i in (0, 1, 5)))
        # 0030 commuting conversion (Bondorf CPS effect): a binary op over a dynamic-if operand with
        # a STATIC scalar other operand folds per branch; a DYNAMIC other operand is NOT pushed.
        cc_res = tower.poly_specialize("(if d then 10 else 20) + 5", ("d",))["residual"]
        commuting = ("+" not in cc_res
                     and rt.eval_source("let d = true; in " + cc_res) == 15
                     and rt.eval_source("let d = false; in " + cc_res) == 25)
        nd_res = tower.poly_specialize("(if d then 10 else 20) + e", ("d", "e"))["residual"]
        commuting_bounded = "+" in nd_res  # dynamic other operand kept, not duplicated
        ready = bool(sharing_flat and eta_folds and bounded_variation and let_insertion
                     and commuting and commuting_bounded)
        return {"schema": "pnix-hy.pe-size.report.v0", "ready": ready, "available": True,
                "sharing_subexpr_count_constant": sharing_flat, "eta_folds": eta_folds,
                "bounded_static_variation": bounded_variation, "let_insertion": let_insertion,
                "commuting_conversion": commuting and commuting_bounded,
                "sizes_by_uses": sizes, "naive_would_duplicate_count": naive_would_duplicate,
                "note": "call-by-need sharing: residual flat in #uses; naive inlining would be linear"}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.pe-size.report.v0", "ready": False, "available": False,
                "error": f"{type(exc).__name__}: {exc}"}


__all__ = ["generating_extension", "cogen", "compiler_from_interpreter",
           "compiler_source", "compile_with", "cogen_report", "pe_size_report"]
