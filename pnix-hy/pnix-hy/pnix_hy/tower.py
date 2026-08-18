"""pnix_hy.tower -- tower-ladder MILESTONE-1 (proposal 0026; explicit phasing).

Verified literature behind each piece: Amin & Rompf POPL'18 (stage polymorphism / maybe-lift,
Pink, EM), Jones/Gomard/Sestoft (S = L, cogen self-generation), Jefferson & Friedman / 3-Lisp
(finite reify/reflect towers). HONEST SCOPE: this module ships milestone-1 -- a stage-polymorphic
mini evaluator, a specializer expressed IN pnix over a core subset (the S = L prerequisite,
demonstrated), the cogen self-generation ACCEPTANCE harness (a real cogen is future work), a
gated reify/reflect v0 into the stage7 lane, and EM v0 (meta-level IR transforms, dual-mode).
Full-language S = L, an actual cogen, and a stage-polymorphic stage7 rewrite remain future
milestones and are NOT claimed here. pnix stays non-homoiconic throughout (EM transforms live
at the META level; IR is data).
"""

from __future__ import annotations

import sys
from typing import Any, Callable

from . import pnix_runtime as rt

class _SPClosure:
    """A static closure value inside the stage-polymorphic evaluator (M2)."""

    def __init__(self, param: str, body: dict[str, Any], env: dict[str, Any]) -> None:
        self.param, self.body, self.env = param, body, env


_SP_UNFOLD_LIMIT = 4000
# I1 bounded static variation: max chars of `cond` code duplicated across an eta-distributed
# if (len(cond) * (nfields-1)); above this, keep a single residual `if` instead of distributing.
_ETA_DIST_BUDGET = 200


def _sp(node: dict[str, Any], env: dict[str, tuple[str, Any]], depth: int = 0) -> tuple[str, Any]:
    """Maybe-lift evaluation (M2 subset: literals, var, ALL runtime binary ops, if, recursive
    let, lambda/apply with UNFOLDING, static attrsets, select). Returns ('val', v) for static
    subtrees, ('code', pnix_src) for residual computation. The SAME function is the interpreter
    (all-'val' env) and the compiler (symbolic ('code', name) entries) -- stage polymorphism.
    Static-data-driven recursion unfolds away (the Jones interpreter collapse); unbounded
    dynamic recursion hits the unfold limit and is rejected as outside the subset."""
    if depth > _SP_UNFOLD_LIMIT:
        raise ValueError("unfold limit exceeded (dynamic recursion outside the subset)")
    tag = node.get("tag")
    if tag in ("int", "float", "bool", "string"):
        return ("val", node.get("value"))
    if tag == "var":
        entry = env.get(str(node.get("name")))
        if entry is None:
            raise ValueError(f"unbound: {node.get('name')}")
        return entry
    if tag == "binary":
        lhs, rhs = _sp(node["lhs"], env, depth + 1), _sp(node["rhs"], env, depth + 1)
        op = node["op"]
        if lhs[0] == "val" and rhs[0] == "val":
            return ("val", rt.stable_data(rt.apply_binary(op, lhs[1], rhs[1])))
        return ("code", f"({_sp_src(lhs)} {op} {_sp_src(rhs)})")
    if tag == "if":
        cond = _sp(node["cond"], env, depth + 1)
        if cond[0] == "val":
            if not isinstance(cond[1], bool):
                raise ValueError("if condition folded to a non-bool")
            return _sp(node["then"] if cond[1] else node["else"], env, depth + 1)
        return ("code", f"(if {_sp_src(cond)} then {_sp_src(_sp(node['then'], env, depth + 1))} "
                        f"else {_sp_src(_sp(node['else'], env, depth + 1))})")
    if tag == "lambda":
        return ("val", _SPClosure(str(node["param"]), node["body"], env))
    if tag == "apply":
        fn = _sp(node["func"], env, depth + 1)
        if not (fn[0] == "val" and isinstance(fn[1], _SPClosure)):
            raise ValueError("apply of a non-static function (outside the subset)")
        arg = _sp(node["arg"], env, depth + 1)  # static OR symbolic -- either unfolds the body
        env2 = dict(fn[1].env)
        env2[fn[1].param] = arg
        return _sp(fn[1].body, env2, depth + 1)
    if tag == "let":
        env2: dict[str, tuple[str, Any]] = dict(env)
        pending: list[tuple[str, dict[str, Any]]] = []
        for b in node.get("bindings", []):
            path = b.get("path", [])
            if len(path) != 1:
                raise ValueError("nested-path let binding (outside the subset)")
            name, value = str(path[0]), b["value"]
            if isinstance(value, dict) and value.get("tag") == "lambda":
                # shared env2 => the closure sees itself: recursive definitions work
                env2[name] = ("val", _SPClosure(str(value["param"]), value["body"], env2))
            else:
                pending.append((name, value))
        remaining = pending
        while remaining:
            progress, rest = False, []
            for name, value in remaining:
                try:
                    env2[name] = _sp(value, env2, depth + 1)
                    progress = True
                except ValueError:
                    rest.append((name, value))
            if not progress:
                raise ValueError("unresolvable let bindings (outside the subset)")
            remaining = rest
        return _sp(node["body"], env2, depth + 1)
    if tag == "attrset":
        if node.get("recursive"):
            raise ValueError("rec attrset (outside the subset)")
        out: dict[str, tuple[str, Any]] = {}
        for b in node.get("bindings", []):
            path = b.get("path", [])
            if len(path) != 1:
                raise ValueError("nested-path attrset (outside the subset)")
            out[str(path[0])] = _sp(b["value"], env, depth + 1)
        if all(e[0] == "val" and not isinstance(e[1], _SPClosure) for e in out.values()):
            return ("val", {k: e[1] for k, e in out.items()})
        return ("attrs", out)  # mixed static/symbolic fields: selectable, not emittable
    if tag == "select":
        base = _sp(node["base"], env, depth + 1)
        attr = str(node.get("attr"))
        if base[0] == "val" and isinstance(base[1], dict):
            if attr not in base[1]:
                raise ValueError(f"missing attr {attr!r}")
            return ("val", base[1][attr])
        if base[0] == "attrs":
            if attr not in base[1]:
                raise ValueError(f"missing attr {attr!r}")
            return base[1][attr]
        raise ValueError("select on a non-static base (outside the subset)")
    raise ValueError(f"outside the stage-poly subset: {tag}")


def _sp_src(entry: tuple[str, Any]) -> str:
    if entry[0] == "code":
        return str(entry[1])
    if entry[0] != "val" or isinstance(entry[1], _SPClosure):
        raise ValueError("cannot residualize a non-data value")
    return _pnix_literal(entry[1])


def stage_poly_interpret(source: str, values: dict[str, Any]) -> Any:
    env = {k: ("val", v) for k, v in values.items()}
    kind, out = _sp(rt.ast_stable(rt.parse(source)), env)
    if kind != "val":
        raise ValueError("interpret mode produced code")
    return out


def stage_poly_compile(source: str, dynamic: tuple[str, ...]) -> str:
    """The SAME evaluator, acting as a one-pass compiler: dynamic names residualize, all else
    folds. The residual contains only program material -- zero interpretive overhead."""
    env: dict[str, tuple[str, Any]] = {n: ("code", n) for n in dynamic}
    kind, out = _sp(rt.ast_stable(rt.parse(source)), env)
    return str(out) if kind == "code" else _sp_src(("val", out))


# --- T3m: the specializer expressed IN pnix (S = L over the core subset) ---

MIX_IN_PNIX = """
let
  fold = op: a: b:
    if op == "+" then a + b else if op == "*" then a * b else if op == "-" then a - b
    else if op == "==" then a == b else if op == "!=" then a != b
    else if op == "<" then a < b else if op == ">" then a > b
    else if op == "&&" then a && b else if op == "||" then a || b
    else a;
  wrap = v:
    if builtins.isInt v then { tag = "int"; value = v; }
    else if builtins.isBool v then { tag = "bool"; value = v; }
    else if builtins.isString v then { tag = "string"; value = v; }
    else { tag = "const"; value = v; };
  isData = n: n.tag == "int" || n.tag == "bool" || n.tag == "string" || n.tag == "const";
  bind1 = env: name: node: env // builtins.listToAttrs [ { name = name; value = node; } ];
  mixPairs = pairs: senv:
    if pairs == [ ] then [ ]
    else [ { name = (builtins.head pairs).name; value = mix (builtins.head pairs).value senv; } ]
         ++ (mixPairs (builtins.tail pairs) senv);
  allData = pairs:
    if pairs == [ ] then true
    else (isData (builtins.head pairs).value) && (allData (builtins.tail pairs));
  dataPairs = pairs:
    if pairs == [ ] then [ ]
    else [ { name = (builtins.head pairs).name; value = (builtins.head pairs).value.value; } ]
         ++ (dataPairs (builtins.tail pairs));
  letPairs = pairs: env2:
    if pairs == [ ] then [ ]
    else (let b = builtins.head pairs; in
      [ { name = b.name;
          value = if b.value.tag == "lambda"
                  then { tag = "closure"; param = b.value.param; body = b.value.body; env = env2; }
                  else mix b.value env2; } ]
      ++ (letPairs (builtins.tail pairs) env2));
  mix = ast: senv:
    if ast.tag == "int" || ast.tag == "bool" || ast.tag == "string" || ast.tag == "const"
       || ast.tag == "closure" then ast
    else if ast.tag == "var" then
      (if builtins.hasAttr ast.name senv then builtins.getAttr ast.name senv else ast)
    else if ast.tag == "binary" then
      (let l = mix ast.lhs senv; r = mix ast.rhs senv; in
       if (isData l) && (isData r)
       then wrap (fold ast.op l.value r.value)
       else { tag = "binary"; op = ast.op; lhs = l; rhs = r; })
    else if ast.tag == "if" then
      (let c = mix ast.cond senv; in
       if c.tag == "bool"
       then (if c.value then mix ast.t senv else mix ast.e senv)
       else { tag = "if"; cond = c; t = mix ast.t senv; e = mix ast.e senv; })
    else if ast.tag == "lambda" then
      { tag = "closure"; param = ast.param; body = ast.body; env = senv; }
    else if ast.tag == "apply" then
      (let f = mix ast.func senv; a = mix ast.arg senv; in
       if f.tag == "closure"
       then mix f.body (bind1 f.env f.param a)
       else { tag = "apply"; func = f; arg = a; })
    else if ast.tag == "select" then
      (let b = mix ast.base senv; in
       if b.tag == "const" then wrap (builtins.getAttr ast.attr b.value)
       else { tag = "select"; base = b; attr = ast.attr; })
    else if ast.tag == "attrset" then
      (let pairs = mixPairs ast.binds senv; in
       if allData pairs then { tag = "const"; value = builtins.listToAttrs (dataPairs pairs); }
       else { tag = "attrset"; binds = pairs; })
    else if ast.tag == "let" then
      (let env2 = senv // (builtins.listToAttrs (letPairs ast.binds env2)); in
       mix ast.body env2)
    else { tag = "unsupported"; reason = ast.tag; };
in mix
"""



def _encode(node: dict[str, Any]) -> dict[str, Any]:
    """AST -> pnix-data encoding ('then'/'else' are pnix keywords, so t/e). M3 subset:
    literals, var, binary, if, lambda, apply, select, attrset, let."""
    tag = node.get("tag")
    if tag in ("int", "bool", "string"):
        return {"tag": tag, "value": node["value"]}
    if tag == "var":
        return {"tag": "var", "name": node["name"]}
    if tag == "binary":
        return {"tag": "binary", "op": node["op"],
                "lhs": _encode(node["lhs"]), "rhs": _encode(node["rhs"])}
    if tag == "if":
        return {"tag": "if", "cond": _encode(node["cond"]),
                "t": _encode(node["then"]), "e": _encode(node["else"])}
    if tag == "lambda":
        return {"tag": "lambda", "param": str(node["param"]), "body": _encode(node["body"])}
    if tag == "apply":
        return {"tag": "apply", "func": _encode(node["func"]), "arg": _encode(node["arg"])}
    if tag == "list":
        return {"tag": "list", "items": [_encode(x) for x in node.get("items", [])]}
    if tag == "null":
        return {"tag": "null"}
    if tag == "select":
        base = node["base"]
        if isinstance(base, dict) and base.get("tag") == "var" and base.get("name") == "builtins":
            return {"tag": "builtin", "name": str(node["attr"])}
        return {"tag": "select", "base": _encode(node["base"]), "attr": str(node["attr"])}
    if tag in ("attrset", "let"):
        if node.get("recursive"):
            raise ValueError("rec attrset outside the M3 subset")
        pairs = []
        for b in node.get("bindings", []):
            path = b.get("path", [])
            if len(path) != 1:
                raise ValueError("nested-path binding outside the M3 subset")
            pairs.append({"name": str(path[0]), "value": _encode(b["value"])})
        if tag == "attrset":
            return {"tag": "attrset", "binds": pairs}
        return {"tag": "let", "binds": pairs, "body": _encode(node["body"])}
    raise ValueError(f"outside the M3 subset: {tag}")


def _decode(enc: dict[str, Any]) -> str:
    tag = enc["tag"]
    if tag == "int":
        return repr(enc["value"])
    if tag == "string":
        return _pnix_literal(enc["value"])
    if tag == "const":
        return _pnix_literal(enc["value"])
    if tag == "bool":
        return "true" if enc["value"] else "false"
    if tag == "var":
        return str(enc["name"])
    if tag == "binary":
        return f"({_decode(enc['lhs'])} {enc['op']} {_decode(enc['rhs'])})"
    if tag == "if":
        return f"(if {_decode(enc['cond'])} then {_decode(enc['t'])} else {_decode(enc['e'])})"
    raise ValueError(tag)


def _pnix_literal(x: Any) -> str:
    if x is None:
        return "null"
    if isinstance(x, bool):
        return "true" if x else "false"
    if isinstance(x, (int, float)):
        return repr(x)
    if isinstance(x, str):
        return '"' + x.replace("\\", "\\\\").replace('"', '\\"') + '"'
    if isinstance(x, dict):
        return "{ " + " ".join(f"{k} = {_pnix_literal(v)};" for k, v in sorted(x.items())) + " }"
    if isinstance(x, list):
        return "[ " + " ".join(f"({_pnix_literal(v)})" for v in x) + " ]"
    raise ValueError(type(x).__name__)


def mix_in_pnix(source: str, static_env: dict[str, int]) -> dict[str, Any]:
    """Run the pnix-EXPRESSED specializer on a core-subset program: pnix specializing pnix.
    Returns the residual (as an encoded AST + decoded source) computed BY pnix itself."""
    enc = _encode(rt.ast_stable(rt.parse(source)))
    senv_nodes = {k: _wrap_node(v) for k, v in static_env.items()}
    body = MIX_IN_PNIX.strip()
    assert body.startswith("let") and body.endswith("in mix")
    call = f"{body} ({_pnix_literal(enc)}) ({_pnix_literal(senv_nodes)})"
    residual_enc = rt.stable_data(rt.eval_source(call))
    return {"residual_ast": residual_enc, "residual_source": _decode(residual_enc)}


def _wrap_node(v: Any) -> dict[str, Any]:
    """A host value -> folded object-language node (the senv now carries NODES)."""
    if isinstance(v, bool):
        return {"tag": "bool", "value": v}
    if isinstance(v, int):
        return {"tag": "int", "value": v}
    if isinstance(v, str):
        return {"tag": "string", "value": v}
    return {"tag": "const", "value": v}


# --- T2h: cogen self-generation ACCEPTANCE harness (real cogen = future milestone) ---

def self_generation_witness(cogen_src: str, mix_src: str,
                            apply_fn: Callable[[str, str], str]) -> dict[str, Any]:
    """The acceptance criterion for any future third-projection attempt: cogen is accepted iff
    it is SELF-GENERATING -- applying it to mix reproduces cogen itself, checked as canonical
    IR content-hash equality and stamped as a witness (extends the B==C proof style)."""
    from . import gate  # noqa: PLC0415
    from . import ir as ir_mod  # noqa: PLC0415

    produced = apply_fn(cogen_src, mix_src)
    ha = ir_mod.ir_of(produced)["ir_sha256"]
    hb = ir_mod.ir_of(cogen_src)["ir_sha256"]
    witness = gate.make_witness("cogen-self-generation",
                                {"produced": ha, "cogen": hb, "equal": ha == hb})
    return {"equal": ha == hb, "hash_produced": ha, "hash_cogen": hb,
            "witness_id": witness["witness_id"]}


# --- T6: reify/reflect v0 (finite-tower level shift, capability-gated + witnessed) ---

def reify_computation(source: str, env: dict[str, Any] | None = None) -> dict[str, Any]:
    """Reify a pnix computation as DATA: (expression IR, environment snapshot, defunctionalized
    continuation -- v0 is the halt continuation) + a content witness. 3-Lisp's (expr, env, kont)
    triple, finite-tower style."""
    from . import gate  # noqa: PLC0415
    from . import ir as ir_mod  # noqa: PLC0415

    bundle = ir_mod.ir_of(source)
    reified = {"schema": "pnix-hy.reified-computation.v0", "source": source,
               "expr_ir": bundle["ir"], "ir_sha256": bundle["ir_sha256"],
               "env": dict(env or {}), "continuation": {"kind": "halt"}}
    reified["witness_id"] = gate.make_witness("reify-computation", {
        "ir": bundle["ir_sha256"], "env": reified["env"], "kont": "halt"})["witness_id"]
    return reified


def reflect_to_stage7(reified: dict[str, Any],
                      granted: tuple[str, ...] | list[str] = ()) -> dict[str, Any]:
    """Level-shift the reified computation UP into the Hy-written stage7 interpreter lane and
    run it there. Held without the 'reflect' capability; on success returns the stage7-lane
    value, host-lane parity, and a checkpoint witness."""
    from . import gate  # noqa: PLC0415
    from . import pnix_mirror as pm  # noqa: PLC0415

    if "reflect" not in set(granted if not isinstance(granted, str) else (granted,)):
        return {"status": "held", "reason": "reflect capability not granted"}
    src = reified["source"]
    if reified.get("env"):
        binds = "; ".join(f"{k} = {_pnix_literal(v)}" for k, v in sorted(reified["env"].items()))
        src = f"let {binds}; in {src}"
    host_value = rt.stable_data(rt.eval_source(src))
    roundtrip = pm.projection_value_roundtrip(src)
    parity = bool(roundtrip.get("meaning_preserved") or roundtrip.get("values_agree"))
    witness = gate.make_witness("reflect", {"ir": reified["ir_sha256"], "value": repr(host_value),
                                            "parity": parity})
    return {"status": "accepted", "value": host_value, "stage7_parity": parity,
            "witness_id": witness["witness_id"]}


# --- T8: EM v0 (execute-at-meta: meta-level IR transforms, dual interpreted/compiled mode) ---

def em(source: str, transform: Callable[[dict[str, Any]], dict[str, Any]],
       mode: str = "interpret") -> dict[str, Any]:
    """Pink's EM shape, kept meta-level so pnix stays non-homoiconic: `transform` is HOST code
    over the IR (data), applied before evaluation (interpreted mode: runtime reflection) or
    before emission (compiled mode: effectively a macro). Both modes are witnessed."""
    from . import gate  # noqa: PLC0415
    from . import ir as ir_mod  # noqa: PLC0415

    ir1 = ir_mod.lower_to_ir(source)
    ir2 = transform(ir1) or ir1
    witness = gate.make_witness("em", {"mode": mode, "before": rt.ast_hash(ir1),
                                       "after": rt.ast_hash(ir2)})
    if mode == "interpret":
        return {"mode": "interpret", "value": rt.stable_data(ir_mod.eval_ir(ir2)),
                "ir_changed": rt.ast_hash(ir1) != rt.ast_hash(ir2),
                "witness_id": witness["witness_id"]}
    return {"mode": "compile", "residual": rt.emit_source(ir2),
            "ir_changed": rt.ast_hash(ir1) != rt.ast_hash(ir2),
            "witness_id": witness["witness_id"]}


def _fold_zero_add(node: Any) -> Any:
    """Report transform: rewrite `0 + x` -> `x` (a meta-level simplification rule)."""
    if isinstance(node, dict):
        out = {k: _fold_zero_add(v) for k, v in node.items()}
        if (out.get("tag") == "binary" and out.get("op") == "+"
                and (out.get("lhs") or {}).get("tag") == "int" and out["lhs"].get("value") == 0):
            return out["rhs"]
        return out
    if isinstance(node, list):
        return [_fold_zero_add(v) for v in node]
    return node


_COGEN_CACHE: dict[str, str] = {}


def build_cogen() -> dict[str, Any]:
    """0026 M5c/M6: self-apply the polyvariant specializer (3rd Futamura projection) to obtain
    the cogen -- the generating extension of the specializer, as a pnix program. Cheap to
    produce (closure conversion cleared the wall); cached. NOTE (0029): this SELF-APPLICATION
    route yields a pathologically bloated cogen (research-confirmed; running it is intractable);
    for the EFFICIENT cogen approach use `pnix_hy.cogen` (hand-written generating extensions)."""
    if "src" not in _COGEN_CACHE:
        poly = POLY_MIX_IN_PNIX.strip()
        _COGEN_CACHE["src"] = poly_specialize(poly[: -len("in pmix")] + "in pmix ast senv st",
                                              ("ast", "senv", "st"))["residual"]
    src = _COGEN_CACHE["src"]
    return {"schema": "pnix-hy.cogen.v0", "residual": src, "size": len(src)}


def run_cogen(prog_source: str, static_env: dict[str, Any] | None = None) -> dict[str, Any]:
    """0026 M6: EXECUTE the cogen artifact as a specializer -- it takes a program AST + static
    env and returns the residual, exactly like poly_mix_in_pnix, proving the generated cogen is
    an operational specializer (not just a well-formed artifact). Small inputs run in ~0.1s;
    large specialization tasks (e.g. re-deriving a full compiler) remain host-tree-walker
    perf-bound (documented)."""
    static_env = static_env or {}
    cogen = build_cogen()["residual"]
    enc = _encode(rt.ast_stable(rt.parse(prog_source)))
    senv_nodes = {k: _wrap_node(v) for k, v in static_env.items()}
    call = (f"let ast = {_pnix_literal(enc)}; senv = {_pnix_literal(senv_nodes)}; "
            f"st = {{ specs = [ ]; ctr = 0; }}; "
            f"in (let __r = ({cogen}); in {{ n = __r.n; specs = __r.st.specs; }})")
    out = _eval_deep(call)
    main = _decode_full(out["n"])
    specs = out.get("specs") or []
    residual = (f"let {'; '.join(s['name'] + ' = ' + s['param'] + ': ' + _decode_full(s['body']) for s in specs)}; "
                f"in {main}") if specs else main
    return {"schema": "pnix-hy.run-cogen.v0", "residual": residual,
            "specialization_points": len(specs)}


def futamura_ladder(interpreter_src: str | None = None, program_lit: str | None = None) -> dict[str, Any]:
    """0026 M7 (consolidation): run the WHOLE Futamura ladder as one inspectable artifact, using
    the existing rungs only (no new machinery). Over a tiny arithmetic object language:
      * 1st projection -- specialize the interpreter to a fixed program: target(input)
      * 2nd projection -- specialize the specializer to the interpreter: a stand-alone compiler
      * 3rd projection -- self-apply the specializer: cogen (produced + executed as a specializer)
    Small-input executions are fast; deriving a full compiler by RUNNING cogen on the interpreter
    stays host-tree-walker perf-bound (documented), so the 3rd rung here shows cogen EXECUTING as
    a specializer rather than re-deriving the whole compiler."""
    interpreter_src = interpreter_src or (
        'let int = prog: env: if prog.tag == "num" then prog.value '
        'else if prog.tag == "arg" then env '
        'else if prog.tag == "add" then (int prog.l env) + (int prog.r env) '
        'else if prog.tag == "mul" then (int prog.l env) * (int prog.r env) '
        'else 0; in int prog input')
    program_lit = program_lit or (
        '{ tag = "add"; l = { tag = "mul"; l = { tag = "arg"; }; r = { tag = "num"; value = 3; }; }; '
        'r = { tag = "num"; value = 4; }; }')

    # 1st projection: interpreter specialized to a fixed program (interpretive layer collapses)
    first = collapse_interpreter(program_lit)
    # 2nd projection: the pnix specializer specialized to the interpreter = a compiler
    enc = _encode(rt.ast_stable(rt.parse(interpreter_src)))
    obj = MIX_IN_PNIX.strip()[: -len("in mix")] + f"ast = {_pnix_literal(enc)}; in mix ast senv"
    compiler = poly_mix_in_pnix(obj, {})
    prog_data = rt.stable_data(rt.eval_source(program_lit))
    target = _decode_full(rt.stable_data(rt.eval_source(
        "let senv = { prog = " + _pnix_literal(_wrap_node(prog_data)) + "; }; in ("
        + compiler["residual"] + ")")))
    # 3rd projection: cogen produced by self-application, then EXECUTED as a specializer
    cogen = build_cogen()
    cogen_run = run_cogen("a * b", {"a": 6})  # small task: cogen behaves as the specializer

    return {
        "schema": "pnix-hy.futamura-ladder.v0",
        "first_projection": {"residual": first["residual"], "interpreter_free": first["interpreter_free"]},
        "second_projection": {"compiler_spec_points": compiler["specialization_points"],
                              "compiler_size": len(compiler["residual"]), "target": target},
        "third_projection": {"cogen_size": cogen["size"],
                             "cogen_run_input": "a * b (a=6)",
                             "cogen_run_residual": cogen_run["residual"]},
        "note": "1st/2nd fully in pnix; 3rd = cogen produced+executed as a specializer. Deriving "
                "the full compiler by running cogen is host-tree-walker perf-bound (compiled "
                "runtime needed); stage-polymorphic stage7 rewrite is SACRED (SCOPE_LOCK).",
    }


def tower_ladder_report() -> dict[str, Any]:
    """Self-check (proposal 0026, milestone-1): stage polymorphism, mix-in-pnix parity, the
    cogen acceptance harness, gated reify/reflect with stage7 parity, and dual-mode EM."""
    # The M2-M5 checks below build/parse/specialize a small pnix-expressed interpreter
    # (INTERP_IN_PNIX -- a 4-branch if/else-if chain) whose recursive-descent parse tree
    # needs noticeably more than Python's conservative default 1000-frame recursion limit
    # (confirmed live: fails under the default limit, succeeds cleanly once raised -- this
    # is genuine, bounded deep recursion from real nested source, not an infinite loop).
    # Scoped to just this call, restored afterward either way.
    _prev_recursion_limit = sys.getrecursionlimit()
    sys.setrecursionlimit(max(_prev_recursion_limit, 4000))
    try:
        return _tower_ladder_report_body()
    finally:
        sys.setrecursionlimit(_prev_recursion_limit)


def _tower_ladder_report_body() -> dict[str, Any]:
    try:
        # T5m: one evaluator, two roles; compile-then-eval == interpret (mini 1st projection)
        prog, dynv = "x * 3 + if b then 1 else 2", ("x", "b")
        residual = stage_poly_compile(prog, dynv)
        agree = all(
            stage_poly_interpret(prog, {"x": x, "b": b})
            == rt.eval_source(f"let x = {x}; b = {'true' if b else 'false'}; in {residual}")
            == rt.eval_source(f"let x = {x}; b = {'true' if b else 'false'}; in {prog}")
            for x, b in ((2, True), (5, False))
        )
        jones_mini = stage_poly_compile("2 * 3 + 4", ()) == "10"  # zero interpretive overhead
        t5_ok = agree and jones_mini

        # T3m: the pnix-expressed specializer agrees with meaning (S = L over the core subset)
        m1 = mix_in_pnix("x + 2 * 3", {"x": 36})
        m2 = mix_in_pnix("x + y", {"x": 1})
        m3 = mix_in_pnix("if b then x + 1 else 0", {"x": 41})
        t3_ok = (m1["residual_ast"] == {"tag": "int", "value": 42}
                 and m2["residual_ast"]["tag"] == "binary"
                 and rt.eval_source(f"let y = 9; in {m2['residual_source']}") == 10
                 and rt.eval_source(f"let b = true; in {m3['residual_source']}") == 42)

        # T2h: the acceptance harness distinguishes a self-generating toy from a non-fixed-point
        good = self_generation_witness("x: x", "y: y", lambda cogen, _mix: cogen)
        bad = self_generation_witness("x: x", "y: y", lambda _cogen, mix: mix)
        t2_ok = good["equal"] is True and bad["equal"] is False and bool(good["witness_id"])

        # T8: EM dual mode -- same meta transform is reflection when interpreting, macro when compiling
        interp = em("0 + (a: a + 2) 40", _fold_zero_add, mode="interpret")
        comp = em("0 + z", _fold_zero_add, mode="compile")
        t8_ok = (interp["value"] == 42 and interp["ir_changed"]
                 and comp["ir_changed"] and "0" not in comp["residual"]
                 and bool(interp["witness_id"]))

        # T6: gated level shift into the stage7 lane (needs the Hy proof python)
        reified = reify_computation("v * 2", env={"v": 21})
        held = reflect_to_stage7(reified, granted=())
        shifted = reflect_to_stage7(reified, granted=("reflect",))
        t6_ok = (held["status"] == "held" and shifted["status"] == "accepted"
                 and shifted["value"] == 42 and shifted["stage7_parity"] is True
                 and bool(shifted["witness_id"]))

        # ---- MILESTONE-2 checks ----
        # T5 M2: the Jones interpreter collapse -- residual has ZERO interpreter artifacts
        prog_lit = ('{ tag = "add"; l = { tag = "mul"; l = { tag = "arg"; }; '
                    'r = { tag = "num"; value = 3; }; }; r = { tag = "num"; value = 4; }; }')
        col = collapse_interpreter(prog_lit)
        t5m2_ok = (col["interpreter_free"]
                   and all(rt.eval_source(f"let input = {i}; in {col['residual']}") == i * 3 + 4
                           for i in (0, 5, 11)))
        # T3 M2: the pnix-expressed mix folds comparison/boolean ops with typed results
        c1 = mix_in_pnix("if x == 3 then 1 else 2", {"x": 3})
        c2 = mix_in_pnix("x < y", {"x": 1})
        t3m2_ok = (c1["residual_ast"] == {"tag": "int", "value": 1}
                   and c2["residual_ast"]["tag"] == "binary"
                   and rt.eval_source(f"let y = 9; in {c2['residual_source']}") is True)
        # T6 M2: defunctionalized-continuation stepper -- pause, hash deterministically, resume
        src6 = "2 * 3 + if true then 4 else 9"
        full = cek_run(src6, {})
        pa, pb = cek_run(src6, {}, pause_at=5), cek_run(src6, {}, pause_at=5)
        resumed = cek_resume(pa["reified"])
        t6m2_ok = (full["value"] == resumed["value"] == rt.eval_source(src6)
                   and pa["reified"]["state_sha256"] == pb["reified"]["state_sha256"]
                   and bool(pa["reified"]["witness_id"]))
        # T8 M2: EM consulted DURING evaluation (per-step meta rules), value-preserving
        ems = em_stepwise("0 + (0 + 40) + 2", _fold_zero_add, {})
        t8m2_ok = ems["value"] == 42 and ems["rewrites"] >= 1 and bool(ems["witness_id"])

        m2_ok = bool(t5m2_ok and t3m2_ok and t6m2_ok and t8m2_ok)

        # ---- MILESTONE-3 checks ----
        int_src = ('let int = prog: env: if prog.tag == "num" then prog.value '
                   'else if prog.tag == "arg" then env '
                   'else if prog.tag == "add" then (int prog.l env) + (int prog.r env) '
                   'else if prog.tag == "mul" then (int prog.l env) * (int prog.r env) '
                   'else 0; in int prog input')
        prog_data = {"tag": "add", "l": {"tag": "mul", "l": {"tag": "arg"},
                                         "r": {"tag": "num", "value": 3}},
                     "r": {"tag": "num", "value": 4}}
        # M3a+b: the pnix-EXPRESSED specializer now handles let/lambda/apply/select/attrset --
        # enough to run the FIRST FUTAMURA PROJECTION entirely inside pnix (pnix specializes a
        # pnix interpreter; the interpretive layer vanishes)
        fp = mix_in_pnix(int_src, {"prog": prog_data})
        m3_first_projection = (fp["residual_source"] == "((input * 3) + 4)"
                               and rt.eval_source(f"let input = 7; in {fp['residual_source']}") == 25)
        # M3c: offline BTA -- static/dynamic division PREDICTS the collapse (every if-condition
        # static => no residual dispatch), cross-checked against the actual residual
        bta = binding_time_analysis(
            int_src.replace("in int prog input",
                            f"prog = {_pnix_literal(prog_data)}; in int prog input"), ("input",))
        bta_dyn = binding_time_analysis("if b then 1 else 2", ("b",))
        m3_bta = (bta["all_if_conditions_static"]
                  and ("(if" not in col["residual"]) == bta["all_if_conditions_static"]
                  and bta_dyn["if_conditions"] == ["D"] and bta_dyn["result"] == "D")
        m3_ok = bool(m3_first_projection and m3_bta)

        # ---- MILESTONE-4 checks ----
        # M4a: polyvariant specialization -- dynamic recursion becomes a NAMED residual
        # recursive function instead of an infinite unfold; static recursion still folds flat
        pv = poly_specialize("let f = n: if n == 0 then 0 else 1 + (f (n - 1)); in f x", ("x",))
        pv_static = poly_specialize("let f = n: if n == 0 then 0 else 1 + (f (n - 1)); in f 4", ())
        m4_polyvariant = (pv["specialization_points"] == 1
                          and rt.eval_source(f"let x = 6; in {pv['residual']}") == 6
                          and pv_static["residual"] == "4" and pv_static["specialization_points"] == 0)
        # M4b: the SECOND Futamura projection -- specialize the pnix-expressed specializer
        # against the interpreter: the residual is a stand-alone COMPILER
        enc_int = _encode(rt.ast_stable(rt.parse(int_src)))
        mix_binds = MIX_IN_PNIX.strip()[: -len("in mix")]
        comp = poly_specialize(mix_binds + f"ast = {_pnix_literal(enc_int)}; in mix ast senv",
                               ("senv",))
        p2 = {"tag": "mul", "l": {"tag": "add", "l": {"tag": "arg"},
                                  "r": {"tag": "num", "value": 1}},
              "r": {"tag": "num", "value": 10}}
        def _compile_with(compiler_src: str, prog: dict[str, Any]) -> str:
            senv_lit = "{ prog = " + _pnix_literal(_wrap_node(prog)) + "; }"
            return _decode(rt.stable_data(rt.eval_source(
                f"let senv = {senv_lit}; in ({compiler_src})")))
        t1 = _compile_with(comp["residual"], prog_data)
        t2_src = _compile_with(comp["residual"], p2)
        m4_second_projection = (comp["specialization_points"] > 0
                                and t1 == "((input * 3) + 4)"
                                and all(rt.eval_source(f"let input = {i}; in {t2_src}")
                                        == (i + 1) * 10 for i in (0, 7)))
        m4_ok = bool(m4_polyvariant and m4_second_projection)

        # ---- MILESTONE-5a: the OUTER specializer's S=L (core subset) ----
        # the POLYVARIANT mix expressed IN pnix (state-passing memo) emits the same named
        # residual recursion as the host version, and still folds static recursion flat
        rec_src = "let f = n: if n == 0 then 0 else 1 + (f (n - 1)); in f x"
        pp = poly_mix_in_pnix(rec_src, {})
        pp_static = poly_mix_in_pnix(
            "let f = n: if n == 0 then 0 else 1 + (f (n - 1)); in f 4", {})
        hh = poly_specialize(rec_src, ("x",))
        m5_poly_in_pnix = (pp["specialization_points"] == 1
                           and pp_static["residual"] == "4"
                           and all(rt.eval_source(f"let x = {i}; in {pp['residual']}")
                                   == rt.eval_source(f"let x = {i}; in {hh['residual']}") == i
                                   for i in (0, 6)))
        # M5b: the SECOND projection performed ENTIRELY IN PNIX -- the pnix-expressed
        # polyvariant specializer specializes the pnix interpreter into a stand-alone compiler
        enc_int_b = _encode(rt.ast_stable(rt.parse(int_src)))
        mix_binds_b = MIX_IN_PNIX.strip()[: -len("in mix")]
        comp_pnix = poly_mix_in_pnix(mix_binds_b + f"ast = {_pnix_literal(enc_int_b)}; in mix ast senv",
                                     {})
        p2b = {"tag": "mul", "l": {"tag": "add", "l": {"tag": "arg"},
                                   "r": {"tag": "num", "value": 1}},
               "r": {"tag": "num", "value": 10}}
        def _comp2(compiler_src: str, prog: dict[str, Any]) -> str:
            senv_lit = "{ prog = " + _pnix_literal(_wrap_node(prog)) + "; }"
            return _decode_full(rt.stable_data(rt.eval_source(
                f"let senv = {senv_lit}; in ({compiler_src})")))
        tb1 = _comp2(comp_pnix["residual"], prog_data)
        tb2 = _comp2(comp_pnix["residual"], p2b)
        m5b_second_projection_in_pnix = (
            comp_pnix["specialization_points"] > 0
            and tb1 == "((input * 3) + 4)"
            and all(rt.eval_source(f"let input = {i}; in {tb2}") == (i + 1) * 10 for i in (0, 7)))
        # M5c: cogen by SELF-APPLICATION now TERMINATES (closure conversion cleared the wall)
        # and yields a well-formed generating extension of the polyvariant specializer. NOTE:
        # producing the cogen artifact is cheap; EXECUTING it (to actually emit a compiler)
        # is impractical in the host tree-walker (depth/perf) -- an honest, recorded frontier.
        _poly_b = POLY_MIX_IN_PNIX.strip()
        cogen = poly_specialize(_poly_b[: -len("in pmix")] + "in pmix ast senv st",
                                ("ast", "senv", "st"))
        cogen_wellformed = (cogen["specialization_points"] > 0
                            and isinstance(rt.parse(cogen["residual"]), dict))
        harness = self_generation_witness("x: x", "y: y", lambda c, _m: c)
        m5c_cogen_produced = bool(cogen_wellformed and harness["equal"])

        m5_ok = bool(m5_poly_in_pnix and m5b_second_projection_in_pnix and m5c_cogen_produced)

        # ---- MILESTONE-6: the cogen artifact is EXECUTABLE and behaves as the specializer ----
        # running the generated cogen folds static computation and residualizes dynamic vars,
        # identically to poly_mix_in_pnix -- an operational validation of the 3rd-projection
        # artifact on the core subset (large tasks remain host-perf-bound; documented).
        rc_static = run_cogen("2 * 3 + 4", {})
        rc_env = run_cogen("a + 1", {"a": 41})
        rc_dyn = run_cogen("a * b", {"a": 6})
        m6_cogen_executes = (rc_static["residual"] == "10"
                             and rc_env["residual"] == "42"
                             and rt.eval_source(f"let b = 7; in {rc_dyn['residual']}") == 42
                             and rc_dyn["specialization_points"] == 0)
        # the executed cogen agrees with the direct specializer (poly_mix_in_pnix) it came from
        m6_cogen_matches_mix = run_cogen("a * b", {"a": 6})["residual"] == \
            poly_mix_in_pnix("a * b", {"a": 6})["residual"]
        m6_ok = bool(m6_cogen_executes and m6_cogen_matches_mix)

        ready = bool(t5_ok and t3_ok and t2_ok and t8_ok and t6_ok
                     and m2_ok and m3_ok and m4_ok and m5_ok and m6_ok)
        return {"schema": "pnix-hy.tower-ladder.report.v0", "ready": ready, "available": True,
                "t5_stage_polymorphism": t5_ok, "t3_mix_in_pnix": t3_ok,
                "t2_cogen_harness": t2_ok, "t6_reify_reflect": t6_ok, "t8_em_dual_mode": t8_ok,
                "m2_interpreter_collapse": t5m2_ok, "m2_mix_comparisons": t3m2_ok,
                "m2_cek_pause_resume": t6m2_ok, "m2_em_stepwise": t8m2_ok,
                "m3_first_projection_in_pnix": m3_first_projection, "m3_bta": m3_bta,
                "m4_polyvariant": m4_polyvariant, "m4_second_projection": m4_second_projection,
                "m5_poly_mix_in_pnix": m5_poly_in_pnix,
                "m5b_second_projection_in_pnix": m5b_second_projection_in_pnix,
                "m5c_cogen_produced": m5c_cogen_produced,
                "cogen_spec_points": cogen["specialization_points"],
                "m6_cogen_executes": m6_cogen_executes,
                "m6_cogen_matches_mix": m6_cogen_matches_mix,
                "milestone": 6,
                "future": "the cogen artifact executes as a specializer on the core subset; "
                          "re-deriving a FULL compiler by running cogen on the interpreter "
                          "remains host-tree-walker perf-bound (a compiled runtime / stage7-lane "
                          "execution would remove it). Remaining ladder rung: stage-polymorphic "
                          "stage7 rewrite -- SACRED 4-lane, needs a SCOPE_LOCK boundary decision "
                          "(0026 M7)"}
    except Exception as exc:  # noqa: BLE001 - Hy proof python unavailable degrades T6
        return {"schema": "pnix-hy.tower-ladder.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


__all__ = ["stage_poly_interpret", "stage_poly_compile", "mix_in_pnix", "MIX_IN_PNIX",
           "self_generation_witness", "reify_computation", "reflect_to_stage7", "em",
           "collapse_interpreter", "INTERP_IN_PNIX",
           "cek_inject", "cek_step", "cek_run", "cek_resume", "reify_cek_state", "em_stepwise",
           "binding_time_analysis", "poly_specialize", "poly_mix_in_pnix", "POLY_MIX_IN_PNIX",
           "build_cogen", "run_cogen", "futamura_ladder",
           "tower_ladder_report"]


# ============================ MILESTONE-2 (0026 M2) ============================

# --- T5 M2: the Jones interpreter collapse, run through the stage-polymorphic evaluator ---
INTERP_IN_PNIX = """let
  int = prog: env:
    if prog.tag == "num" then prog.value
    else if prog.tag == "arg" then env
    else if prog.tag == "add" then (int prog.l env) + (int prog.r env)
    else if prog.tag == "mul" then (int prog.l env) * (int prog.r env)
    else 0;
  prog = {PROG};
in int prog input"""


def collapse_interpreter(prog_literal: str) -> dict[str, Any]:
    """Specialize the pnix-expressed mini INTERPRETER to a static program with a dynamic
    `input`: static-data-driven recursion unfolds completely, so the residual is plain
    arithmetic over `input` with ZERO interpreter artifacts (no tag tests, no dispatch) --
    the classic 'compiling by specializing an interpreter' collapse, checkable."""
    src = INTERP_IN_PNIX.replace("{PROG}", prog_literal)
    residual = stage_poly_compile(src, ("input",))
    return {"residual": residual,
            "interpreter_free": all(m not in residual for m in ("tag", "prog", "int ", "if"))}


# --- T6 M2: CEK stepper with DEFUNCTIONALIZED continuations (reify mid-run, hash, resume) ---

def cek_inject(source: str, env: dict[str, Any] | None = None) -> dict[str, Any]:
    return {"control": {"expr": rt.ast_stable(rt.parse(source))},
            "env": dict(env or {}), "kont": {"kind": "halt"}}


def cek_step(state: dict[str, Any]) -> dict[str, Any] | None:
    """ONE small-step transition of the CEK machine (core subset: literals/var/binary/if).
    Continuations are DATA frames (defunctionalized), so any intermediate state is plain,
    canonically-hashable data. Returns None when the state is final."""
    control, env, kont = state["control"], state["env"], state["kont"]
    if "expr" in control:
        e = control["expr"]
        tag = e.get("tag")
        if tag in ("int", "float", "bool", "string"):
            return {"control": {"value": e.get("value")}, "env": env, "kont": kont}
        if tag == "var":
            name = str(e.get("name"))
            if name not in env:
                raise ValueError(f"unbound: {name}")
            return {"control": {"value": env[name]}, "env": env, "kont": kont}
        if tag == "binary":
            frame = {"kind": "bin-l", "op": e["op"], "rhs": e["rhs"], "env": env, "next": kont}
            return {"control": {"expr": e["lhs"]}, "env": env, "kont": frame}
        if tag == "if":
            frame = {"kind": "if", "then": e["then"], "else": e["else"], "env": env, "next": kont}
            return {"control": {"expr": e["cond"]}, "env": env, "kont": frame}
        raise ValueError(f"outside the CEK subset: {tag}")
    value = control["value"]
    kind = kont.get("kind")
    if kind == "halt":
        return None
    if kind == "bin-l":
        frame = {"kind": "bin-r", "op": kont["op"], "lval": value, "next": kont["next"]}
        return {"control": {"expr": kont["rhs"]}, "env": kont["env"], "kont": frame}
    if kind == "bin-r":
        out = rt.stable_data(rt.apply_binary(kont["op"], kont["lval"], value))
        return {"control": {"value": out}, "env": env, "kont": kont["next"]}
    if kind == "if":
        if not isinstance(value, bool):
            raise ValueError("if condition is not a bool")
        branch = kont["then"] if value else kont["else"]
        return {"control": {"expr": branch}, "env": kont["env"], "kont": kont["next"]}
    raise ValueError(f"unknown continuation frame: {kind}")


def reify_cek_state(state: dict[str, Any]) -> dict[str, Any]:
    """Content-hash a mid-computation machine state (expr/value + env + defunctionalized
    continuation stack) and stamp a witness -- tower meta-state as a deterministic artifact."""
    import hashlib as _hashlib  # noqa: PLC0415
    import json as _json  # noqa: PLC0415

    from . import gate  # noqa: PLC0415

    digest = _hashlib.sha256(_json.dumps(state, sort_keys=True, separators=(",", ":"),
                                         default=repr).encode("utf-8")).hexdigest()
    witness = gate.make_witness("cek-state", {"state_sha256": digest})
    return {"schema": "pnix-hy.cek-state.v0", "state": state, "state_sha256": digest,
            "witness_id": witness["witness_id"]}


def cek_run(source: str, env: dict[str, Any] | None = None, *,
            pause_at: int | None = None, max_steps: int = 100000) -> dict[str, Any]:
    """Run the CEK machine; optionally PAUSE after `pause_at` steps and return the reified
    state (resumable via `cek_resume`)."""
    state = cek_inject(source, env)
    steps = 0
    while steps < max_steps:
        if pause_at is not None and steps == pause_at:
            return {"status": "paused", "steps": steps, "reified": reify_cek_state(state)}
        nxt = cek_step(state)
        if nxt is None:
            return {"status": "done", "steps": steps, "value": state["control"]["value"]}
        state = nxt
        steps += 1
    raise ValueError("max_steps exceeded")


def cek_resume(reified: dict[str, Any], *, max_steps: int = 100000) -> dict[str, Any]:
    """Resume a reified machine state to completion (level-shift back down: reflect)."""
    state = reified["state"]
    steps = 0
    while steps < max_steps:
        nxt = cek_step(state)
        if nxt is None:
            return {"status": "done", "steps": steps, "value": state["control"]["value"]}
        state = nxt
        steps += 1
    raise ValueError("max_steps exceeded")


# --- T8 M2: EM DURING evaluation -- meta-level rules consulted at every machine step ---

def em_stepwise(source: str, transform: Callable[[dict[str, Any]], dict[str, Any]],
                env: dict[str, Any] | None = None, *, max_steps: int = 100000) -> dict[str, Any]:
    """Evaluation-time EM: before every expression step, the CEK machine hands the CURRENT
    control expression to a META-level transform (host code over IR data). pnix stays
    non-homoiconic -- the object program never manipulates itself; the meta level does."""
    from . import gate  # noqa: PLC0415

    state = cek_inject(source, env)
    rewrites = 0
    steps = 0
    while steps < max_steps:
        control = state["control"]
        if "expr" in control:
            new_expr = transform(control["expr"]) or control["expr"]
            if new_expr is not control["expr"] and new_expr != control["expr"]:
                rewrites += 1
                state = {"control": {"expr": new_expr}, "env": state["env"], "kont": state["kont"]}
        nxt = cek_step(state)
        if nxt is None:
            witness = gate.make_witness("em-stepwise", {"rewrites": rewrites, "steps": steps})
            return {"value": state["control"]["value"], "rewrites": rewrites, "steps": steps,
                    "witness_id": witness["witness_id"]}
        state = nxt
        steps += 1
    raise ValueError("max_steps exceeded")


# --- M3c: offline BINDING-TIME ANALYSIS v1 (the proven prerequisite for self-application) ---

class _BTFunc:
    """A function's binding-time value: analyzed at each application site (monovariant memo)."""

    def __init__(self, param: str, body: dict[str, Any], env: dict[str, Any]) -> None:
        self.param, self.body, self.env = param, body, env


def binding_time_analysis(source: str, dynamic_vars: tuple[str, ...] = ()) -> dict[str, Any]:
    """0026 M3c: OFFLINE BTA over the tower subset -- classify every node Static/Dynamic before
    specialization (Jones 1985: the decision that made self-application effective; the exact
    prerequisite naive unfolding lacks, since recursive closures over a dynamic environment
    residualize into unbounded self-referencing code). Monovariant, memoized fixpoint on
    (function-body, argument-bt); conservative: unknown constructs are Dynamic."""
    ast = rt.ast_stable(rt.parse(source))
    memo: dict[tuple[int, str], str] = {}
    in_progress: set[tuple[int, str]] = set()
    if_conds: list[str] = []
    counts = {"S": 0, "D": 0}

    def join(a: str, b: str) -> str:
        return "D" if "D" in (a, b) else "S"

    def bt(node: Any, env: dict[str, Any], depth: int = 0) -> Any:
        if depth > 500:
            return "D"  # conservative on runaway analysis
        tag = node.get("tag") if isinstance(node, dict) else None
        if tag in ("int", "float", "bool", "string", "null"):
            out: Any = "S"
        elif tag == "var":
            name = str(node.get("name"))
            out = env[name] if name in env else "D"  # unbound/declared-dynamic -> D
        elif tag == "binary":
            out = join(_bts(bt(node["lhs"], env, depth + 1)), _bts(bt(node["rhs"], env, depth + 1)))
        elif tag == "if":
            c = _bts(bt(node["cond"], env, depth + 1))
            if_conds.append(c)
            tb = _bts(bt(node["then"], env, depth + 1))
            eb = _bts(bt(node["else"], env, depth + 1))
            out = join(tb, eb) if c == "S" else "D"
        elif tag == "lambda":
            out = _BTFunc(str(node["param"]), node["body"], env)
        elif tag == "apply":
            fn = bt(node["func"], env, depth + 1)
            arg = bt(node["arg"], env, depth + 1)
            if isinstance(fn, _BTFunc):
                key = (id(fn.body), _bts(arg))
                if key in in_progress:  # recursive re-entry: answer with the current assumption
                    out = memo.get(key, "S")  # optimistic seed; outer fixpoint corrects it
                else:
                    in_progress.add(key)
                    env2 = dict(fn.env)
                    env2[fn.param] = arg
                    result = bt(fn.body, env2, depth + 1)
                    out = result if isinstance(result, _BTFunc) else _bts(result)
                    if not isinstance(out, _BTFunc):
                        memo[key] = out
                    in_progress.discard(key)
            else:
                out = "D"
        elif tag == "select":
            out = _bts(bt(node["base"], env, depth + 1))
        elif tag == "attrset":
            parts = [_bts(bt(b["value"], env, depth + 1)) for b in node.get("bindings", [])
                     if len(b.get("path", [])) == 1]
            out = "D" if "D" in parts else "S"
        elif tag == "let":
            env2: dict[str, Any] = dict(env)
            for b in node.get("bindings", []):
                path = b.get("path", [])
                if len(path) != 1:
                    return "D"
                value = b["value"]
                if isinstance(value, dict) and value.get("tag") == "lambda":
                    env2[str(path[0])] = _BTFunc(str(value["param"]), value["body"], env2)
            for b in node.get("bindings", []):
                path = b.get("path", [])
                value = b["value"]
                if not (isinstance(value, dict) and value.get("tag") == "lambda"):
                    env2[str(path[0])] = bt(value, env2, depth + 1)
            out = bt(node["body"], env2, depth + 1)
        else:
            out = "D"  # conservative
        if not isinstance(out, _BTFunc):
            counts[out] += 1
        return out

    def _bts(x: Any) -> str:
        return "S" if isinstance(x, _BTFunc) else x

    base_env: dict[str, Any] = {v: "D" for v in dynamic_vars}
    result = "D"
    for _ in range(5):  # memoized fixpoint: rerun until application memo stabilizes
        snapshot = dict(memo)
        if_conds.clear()
        counts["S"] = counts["D"] = 0
        result = _bts(bt(ast, dict(base_env)))
        if memo == snapshot:
            break
    return {"schema": "pnix-hy.bta.v0", "result": result,
            "division": {v: "D" for v in dynamic_vars},
            "if_conditions": list(if_conds),
            "all_if_conditions_static": bool(if_conds) and all(c == "S" for c in if_conds),
            "counts": dict(counts)}


# ============================ MILESTONE-4 (0026 M4) ============================
# Polyvariant specialization: the 1985-Mix technique that breaks the M3 wall. Instead of
# unfolding a recursive call on a dynamic argument forever (or emitting unbounded
# self-referencing code), each (function-body, static-signature) pair becomes a NAMED
# SPECIALIZATION POINT: a residual function definition, with recursive calls emitted as calls
# to that name. Self-referencing dynamic lets residualize as (legal) recursive pnix lets.

class _PSState:
    def __init__(self) -> None:
        self.specs: dict[tuple[int, str], dict[str, Any]] = {}
        self.counter = 0
        self.shared: dict[str, str] = {}  # I4 let-insertion: residual-code -> hoisted binding name

    def fresh(self, prefix: str) -> str:
        self.counter += 1
        return f"{prefix}{self.counter}"

    def hoist(self, code: str) -> str:
        """I4 let-insertion: name a residual expression once (dedup by code) so it is shared via a
        top-level `let` instead of duplicated at each use. Returns the binding name."""
        if code not in self.shared:
            self.shared[code] = self.fresh("__h")
        return self.shared[code]

    # closure conversion uses fresh() too; kept on the same counter for determinism


def _is_trivial_code(code: str) -> bool:
    """A residual expression not worth hoisting (a bare identifier/number/keyword): hoisting it
    would just add `__h = <name>` noise. Anything with structure (spaces, operators, calls) is
    worth sharing when duplicated."""
    return code.replace("_", "").isalnum()


def _scalar_lit_node(value: Any) -> dict[str, Any] | None:
    """Re-encode a static scalar as an AST literal node (for 0030 commuting conversions), or None
    for non-scalars (lists/attrs/closures are not cheaply duplicable into if branches)."""
    if isinstance(value, bool):
        return {"tag": "bool", "value": value}
    if isinstance(value, int):
        return {"tag": "int", "value": value}
    if isinstance(value, str):
        return {"tag": "string", "value": value}
    return None


def _commute_binary_if(node: dict[str, Any], env: dict[str, Any], st: "_PSState",
                       depth: int) -> dict[str, Any] | None:
    """0030: if one operand of a binary op is a dynamic `if` AST node and the OTHER operand
    specializes to a STATIC SCALAR, return the AST with the op pushed into both branches
    (`(if c then a else b) op R` -> `if c then (a op R) else (b op R)`), so each branch folds and
    the static operand is duplicated only as a small literal. Returns None when not applicable
    (keeps the normal path -- never pushes a dynamic operand, which would duplicate work)."""
    op, ln, rn = node["op"], node["lhs"], node["rhs"]
    if isinstance(ln, dict) and ln.get("tag") == "if":
        other = _ps(rn, env, st, depth + 1)
        lit = _scalar_lit_node(other[1]) if other[0] == "val" else None
        if lit is not None:
            return {"tag": "if", "cond": ln["cond"],
                    "then": {"tag": "binary", "op": op, "lhs": ln["then"], "rhs": lit},
                    "else": {"tag": "binary", "op": op, "lhs": ln["else"], "rhs": lit}}
    if isinstance(rn, dict) and rn.get("tag") == "if":
        other = _ps(ln, env, st, depth + 1)
        lit = _scalar_lit_node(other[1]) if other[0] == "val" else None
        if lit is not None:
            return {"tag": "if", "cond": rn["cond"],
                    "then": {"tag": "binary", "op": op, "lhs": lit, "rhs": rn["then"]},
                    "else": {"tag": "binary", "op": op, "lhs": lit, "rhs": rn["else"]}}
    return None


def _ps_sig(env: dict[str, Any]) -> str:
    """The STATIC signature of a closure environment: data values by canonical repr, closures
    by body identity. Two applications with the same signature share one specialization."""
    parts: list[str] = []
    for k in sorted(env):
        e = env[k]
        if e[0] == "val":
            v = e[1]
            parts.append(f"{k}=c{id(v.body)}" if isinstance(v, _SPClosure) else f"{k}={v!r}")
        elif e[0] == "attrs":
            parts.append(f"{k}=attrs{sorted(e[1])}")
        else:
            parts.append(f"{k}=D")
    return "|".join(parts)


_PS_BUILTINS: dict[str, Any] = {
    "builtins.head": lambda xs: xs[0],
    "builtins.tail": lambda xs: xs[1:],
    "builtins.listToAttrs": lambda pairs: {p["name"]: p["value"] for p in pairs},
    "builtins.hasAttr": lambda name, s: name in s,
    "builtins.getAttr": lambda name, s: s[name],
    "builtins.isInt": lambda v: isinstance(v, int) and not isinstance(v, bool),
    "builtins.isBool": lambda v: isinstance(v, bool),
    "builtins.isString": lambda v: isinstance(v, str),
}


def _occurs(node: Any, name: str) -> int:
    """Count (over-approximately) occurrences of variable `name` in an AST subtree. Used ONLY as
    the >=2 sharing test for Q1-1 sharing-safe let residualization; over-counting is safe (it just
    residualizes a binding that could have been inlined -- identical semantics in pure pnix, only
    sharing differs), under-counting would not be, so this deliberately ignores shadowing."""
    if isinstance(node, dict):
        if node.get("tag") == "var" and node.get("name") == name:
            return 1
        return sum(_occurs(v, name) for v in node.values())
    if isinstance(node, list):
        return sum(_occurs(v, name) for v in node)
    return 0


def _as_attrs(entry: tuple[str, Any]) -> dict[str, Any] | None:
    """View a _ps entry as an attrs dict {key -> entry}, or None if it is not attrset-shaped.
    Used by Q1-2 to distribute an `if` over product structure."""
    if entry[0] == "attrs":
        return entry[1]
    if entry[0] == "val" and isinstance(entry[1], dict):
        return {k: ("val", v) for k, v in entry[1].items()}
    return None


def _as_list(entry: tuple[str, Any]) -> list[Any] | None:
    """View a _ps entry as a list of entries, or None if it is not list-shaped (Q1-2 sum/list)."""
    if entry[0] == "list":
        return entry[1]
    if entry[0] == "val" and isinstance(entry[1], list):
        return [("val", v) for v in entry[1]]
    return None


def _ps(node: dict[str, Any], env: dict[str, Any], st: _PSState, depth: int = 0) -> tuple[str, Any]:
    """Polyvariant maybe-lift evaluation. Entries: ('val', v) | ('code', src) | ('attrs', d) |
    ('builtin', name, args) | ('list', entries). Recursive dynamic applications become
    specialization points instead of infinite unfolds."""
    if depth > _SP_UNFOLD_LIMIT:
        raise ValueError("unfold limit exceeded")
    tag = node.get("tag")
    if tag in ("int", "float", "bool", "string"):
        return ("val", node.get("value"))
    if tag == "null":
        return ("val", None)
    if tag == "builtin":  # a `builtins.<name>` reference captured at encode time
        return ("builtin", node.get("name"), [])
    if tag == "var":
        name = str(node.get("name"))
        if name in env:
            return env[name]
        if name == "builtins":
            return ("code", "builtins")
        raise ValueError(f"unbound: {name}")
    if tag == "list":
        items = [_ps(x, env, st, depth + 1) for x in node.get("items", [])]
        if all(e[0] == "val" for e in items):
            return ("val", [e[1] for e in items])
        return ("list", items)
    if tag == "binary":
        op = node["op"]
        # 0030 commuting conversion (the effect Bondorf's CPS specializer achieves, without a CPS
        # rewrite): push a binary op INTO a dynamic `if` operand WHEN the other operand is a static
        # scalar, so both branches fold and NOTHING is duplicated. Skipped when the other operand is
        # dynamic (pushing would duplicate it -> bloat) or for the lazy booleans.
        if op not in ("&&", "||"):
            pushed = _commute_binary_if(node, env, st, depth)
            if pushed is not None:
                return _ps(pushed, env, st, depth + 1)
        lhs = _ps(node["lhs"], env, st, depth + 1)
        if op in ("&&", "||"):  # short-circuit (apply_binary has no lazy boolean ops)
            if lhs[0] == "val":
                if not isinstance(lhs[1], bool):
                    raise ValueError(f"{op} on a non-bool")
                if op == "&&" and lhs[1] is False:
                    return ("val", False)
                if op == "||" and lhs[1] is True:
                    return ("val", True)
                return _ps(node["rhs"], env, st, depth + 1)
            rhs = _ps(node["rhs"], env, st, depth + 1)
            return ("code", f"({_ps_src(lhs)} {op} {_ps_src(rhs)})")
        rhs = _ps(node["rhs"], env, st, depth + 1)
        if lhs[0] == "val" and rhs[0] == "val":
            return ("val", rt.stable_data(rt.apply_binary(op, lhs[1], rhs[1])))
        if op == "++" and lhs[0] in ("val", "list") and rhs[0] in ("val", "list"):
            le = [("val", v) for v in lhs[1]] if lhs[0] == "val" else lhs[1]
            re_ = [("val", v) for v in rhs[1]] if rhs[0] == "val" else rhs[1]
            return ("list", le + re_)
        return ("code", f"({_force_code(lhs, st, depth + 1)} {op} {_force_code(rhs, st, depth + 1)})")
    if tag == "if":
        cond = _ps(node["cond"], env, st, depth + 1)
        if cond[0] == "val":
            if not isinstance(cond[1], bool):
                raise ValueError("if condition folded to a non-bool")
            return _ps(node["then"] if cond[1] else node["else"], env, st, depth + 1)
        then_e = _ps(node["then"], env, st, depth + 1)
        else_e = _ps(node["else"], env, st, depth + 1)
        cc = _force_code(cond, st, depth + 1)
        # Q1-2 general "The Trick" (eta at product/sum): when BOTH branches are structured with the
        # same shape, DISTRIBUTE the if into the structure so downstream selects/indexing fold to
        # the (now dynamic-but-per-field) value instead of residualizing whole attrsets/lists and
        # eliminating afterward. Reuses the existing attrs/list entry kinds (no new consumers).
        # I1 BOUNDED STATIC VARIATION (Danvy/Malmkjaer/Palsberg, JGS): distribution COPIES `cc` into
        # every field, so only distribute when that duplication is bounded (cc small or few fields);
        # otherwise keep a single residual `if` (correct, no cond blow-up).
        def _cref(n_entries: int) -> str | None:
            # I4 let-insertion: distributing copies the cond into every field. If there are >=2
            # fields and the cond is non-trivial AND CLOSED over the top-level scope, HOIST it to a
            # shared top-level binding (used once) so distribution never duplicates the cond. A cond
            # that mentions a locally-bound name (fresh `__`-prefixed spec/closure/eta param) is NOT
            # top-level-closed -- hoisting it would move it out of scope -- so it is inlined instead.
            if n_entries >= 2 and not _is_trivial_code(cc) and "__" not in cc:
                return st.hoist(cc)
            # trivial or locally-scoped cond: inline it, but bound the fan-out (I1 backstop).
            return cc if len(cc) * max(0, n_entries - 1) <= _ETA_DIST_BUDGET else None
        ta, ea = _as_attrs(then_e), _as_attrs(else_e)
        if ta is not None and ea is not None and set(ta) == set(ea):
            cra = _cref(len(ta))
            if cra is not None:
                return ("attrs", {k: ("code", f"(if {cra} then {_force_code(ta[k], st, depth + 1)} "
                                              f"else {_force_code(ea[k], st, depth + 1)})") for k in ta})
        tl, el = _as_list(then_e), _as_list(else_e)
        if tl is not None and el is not None and len(tl) == len(el):
            crl = _cref(len(tl))
            if crl is not None:
                return ("list", [("code", f"(if {crl} then {_force_code(tl[i], st, depth + 1)} "
                                          f"else {_force_code(el[i], st, depth + 1)})") for i in range(len(tl))])
        return ("code", f"(if {cc} then {_force_code(then_e, st, depth + 1)} "
                        f"else {_force_code(else_e, st, depth + 1)})")
    if tag == "lambda":
        return ("val", _SPClosure(str(node["param"]), node["body"], env))
    if tag == "apply":
        fn = _ps(node["func"], env, st, depth + 1)
        arg = _ps(node["arg"], env, st, depth + 1)
        if fn[0] == "builtin":
            args = [*fn[2], arg]
            impl = _PS_BUILTINS[fn[1]]
            import inspect as _inspect  # noqa: PLC0415
            arity = len(_inspect.signature(impl).parameters)
            if len(args) < arity:
                return ("builtin", fn[1], args)
            if all(a[0] == "val" for a in args):
                return ("val", impl(*[a[1] for a in args]))
            if fn[1] == "builtins.listToAttrs" and args[0][0] == "list":
                out: dict[str, Any] = {}
                for e in args[0][1]:  # entries: attrs/val dicts with name(val)/value(entry)
                    if e[0] == "val":
                        out[e[1]["name"]] = ("val", e[1]["value"])
                    elif e[0] == "attrs" and e[1].get("name", ("",))[0] == "val":
                        out[e[1]["name"][1]] = e[1]["value"]
                    else:
                        raise ValueError("listToAttrs: unsupported entry")
                return ("attrs", out)
            call = " ".join([fn[1], *[_force_code(a, st, depth + 1) for a in args]])
            return ("code", f"({call})")
        if fn[0] == "val" and isinstance(fn[1], _SPClosure):
            clo = fn[1]
            if arg[0] == "val" and not isinstance(arg[1], _SPClosure):
                env2 = dict(clo.env)
                env2[clo.param] = arg
                return _ps(clo.body, env2, st, depth + 1)  # static arg: plain unfold
            # dynamic argument -> SPECIALIZATION POINT (polyvariance)
            key = (id(clo.body), _ps_sig(clo.env))
            if key not in st.specs:
                name = st.fresh("__s")
                pname = st.fresh("__a")
                st.specs[key] = {"name": name, "param": pname, "body": None}
                env2 = dict(clo.env)
                env2[clo.param] = ("code", pname)
                body_entry = _ps(clo.body, env2, st, depth + 1)
                st.specs[key]["body"] = _force_code(body_entry, st, depth + 1)
            return ("code", f"({st.specs[key]['name']} {_force_code(arg, st, depth + 1)})")
        if fn[0] == "code":
            return ("code", f"({fn[1]} {_force_code(arg, st, depth + 1)})")
        raise ValueError("apply of a non-applicable value")
    if tag == "select":
        # Q1-2 "The Trick" (eta-expansion at product/sum): push a select INTO the branches of an
        # `if` so a static attrset branch folds to its field, instead of residualizing the whole
        # attrset in each branch and selecting afterward (Danvy/Malmkjaer/Palsberg TOPLAS'96).
        # Sound & terminating: same value in pure pnix; the AST shrinks toward non-if bases.
        base_node = node.get("base")
        if isinstance(base_node, dict) and base_node.get("tag") == "if":
            pushed = {"tag": "if", "cond": base_node["cond"],
                      "then": {"tag": "select", "base": base_node["then"], "attr": node["attr"]},
                      "else": {"tag": "select", "base": base_node["else"], "attr": node["attr"]}}
            return _ps(pushed, env, st, depth + 1)
        base = _ps(node["base"], env, st, depth + 1)
        attr = str(node.get("attr"))
        if base[0] == "val" and isinstance(base[1], dict):
            return ("val", base[1][attr])
        if base[0] == "attrs":
            if attr not in base[1]:
                raise ValueError(f"missing attr {attr!r}")
            return base[1][attr]
        if base[0] == "code":
            return ("code", f"({base[1]}).{attr}")
        raise ValueError("select on a non-selectable value")
    if tag == "attrset":
        if node.get("recursive"):
            raise ValueError("rec attrset outside the subset")
        out2: dict[str, Any] = {}
        for b in node.get("bindings", []):
            path = b.get("path", [])
            if len(path) != 1:
                raise ValueError("nested-path attrset")
            out2[str(path[0])] = _ps(b["value"], env, st, depth + 1)
        if all(e[0] == "val" and not isinstance(e[1], _SPClosure) for e in out2.values()):
            return ("val", {k: e[1] for k, e in out2.items()})
        return ("attrs", out2)
    if tag == "let":
        env2 = dict(env)
        pending: list[tuple[str, dict[str, Any]]] = []
        names = []
        for b in node.get("bindings", []):
            path = b.get("path", [])
            if len(path) != 1:
                raise ValueError("nested-path let")
            name, value = str(path[0]), b["value"]
            names.append(name)
            if isinstance(value, dict) and value.get("tag") == "lambda":
                env2[name] = ("val", _SPClosure(str(value["param"]), value["body"], env2))
            else:
                pending.append((name, value))
        remaining = pending
        while remaining:
            progress, rest = False, []
            for name, value in remaining:
                try:
                    env2[name] = _ps(value, env2, st, depth + 1)
                    progress = True
                except ValueError:
                    rest.append((name, value))
            if not progress:
                # SELF/mutually-referencing dynamic bindings -> residualize as a RECURSIVE let
                # (legal in pnix); each unresolved name becomes a code symbol first.
                for name, _v in rest:
                    env2[name] = ("code", name)
                lines = []
                for name, value in rest:
                    lines.append(f"{name} = {_ps_src(_ps(value, env2, st, depth + 1))}")
                body_src = _ps_src(_ps(node["body"], env2, st, depth + 1))
                return ("code", "let " + "; ".join(lines) + "; in " + body_src)
            remaining = rest
        # Q1-1 sharing-safe: a dynamic (`code`) binding used NON-AFFINELY in the body would, if
        # inlined at every use, duplicate the dynamic computation and bloat/slow the residual
        # (pnix is call-by-need; inlining discards that sharing). Residualize such bindings as a
        # shared `let` instead. Semantics are unchanged (pure pnix); only sharing is preserved.
        body = node["body"]
        shared: list[tuple[str, str]] = []
        for name in names:
            entry = env2.get(name)
            if entry is not None and entry[0] == "code" and _occurs(body, name) >= 2:
                shared.append((name, str(entry[1])))
                env2[name] = ("code", name)
        body_entry = _ps(body, env2, st, depth + 1)
        if shared:
            prelude = "; ".join(f"{n} = {src}" for n, src in shared)
            return ("code", "let " + prelude + "; in " + _force_code(body_entry, st, depth + 1))
        return body_entry
    raise ValueError(f"outside the polyvariant subset: {tag}")


def _force_code(entry: tuple[str, Any], st: "_PSState", depth: int = 0) -> str:
    """Residualize any entry to pnix source, LAMBDA-LIFTING closures (closure conversion): a
    closure that must appear in residual code is emitted as `(param: <body>)` with its body
    specialized under a symbolic parameter -- the technique higher-order self-application needs."""
    if entry[0] == "val" and isinstance(entry[1], _SPClosure):
        clo = entry[1]
        pn = st.fresh("__l")
        env2 = dict(clo.env)
        env2[clo.param] = ("code", pn)
        body = _ps(clo.body, env2, st, depth + 1)
        return f"({pn}: {_force_code(body, st, depth + 1)})"
    return _ps_src(entry)


def _ps_src(entry: tuple[str, Any]) -> str:
    kind = entry[0]
    if kind == "code":
        return str(entry[1])
    if kind == "val":
        if isinstance(entry[1], _SPClosure):
            raise ValueError("closure residualized outside _force_code")
        return _pnix_literal(entry[1])
    if kind == "attrs":
        fields = " ".join(f"{k} = {_ps_src(v)};" for k, v in sorted(entry[1].items()))
        return "{ " + fields + " }"
    if kind == "list":
        return "[ " + " ".join(f"({_ps_src(v)})" for v in entry[1]) + " ]"
    if kind == "builtin":
        raise ValueError("cannot residualize a partially-applied builtin")
    raise ValueError(f"cannot residualize: {kind}")


def poly_specialize(source: str, dynamic_vars: tuple[str, ...] = ()) -> dict[str, Any]:
    """0026 M4a: POLYVARIANT specialization. Recursion over dynamic data no longer explodes:
    every (function, static-signature) pair becomes one named residual definition, recursive
    calls become calls to that name, and self-referencing dynamic environments residualize as
    recursive pnix lets. Schema `pnix-hy.poly-specialize.v0`."""
    st = _PSState()
    env: dict[str, Any] = {n: ("code", n) for n in dynamic_vars}
    entry = _ps(rt.ast_stable(rt.parse(source)), env, st)
    main = _force_code(entry, st)
    # spec-point definitions + I4 hoisted shared bindings, both in one top-level (recursive) let
    defs = [f"{s['name']} = {s['param']}: {s['body']}" for s in st.specs.values()]
    defs += [f"{name} = {code}" for code, name in st.shared.items()]
    residual = f"let {'; '.join(defs)}; in {main}" if defs else main
    return {"schema": "pnix-hy.poly-specialize.v0", "residual": residual,
            "specialization_points": len(st.specs),
            "hoisted_bindings": len(st.shared)}


# ============================ MILESTONE-5a (0026 M5) ============================
# The OUTER specializer's S=L, core subset: the POLYVARIANT mix expressed IN pnix.
# pnix is pure, so the specialization-point memo is threaded STATE-PASSING style: every
# recursive call returns { n = <node>; st = { specs; ctr; }; }. Specialization points are
# looked up by STRUCTURAL key equality (body AST + sanitized static signature -- closures
# contribute only their body, so self-referencing environments never enter comparisons).
POLY_MIX_IN_PNIX = """
let
  fold = op: a: b:
    if op == "+" then a + b else if op == "*" then a * b else if op == "-" then a - b
    else if op == "==" then a == b else if op == "!=" then a != b
    else if op == "<" then a < b else if op == ">" then a > b
    else if op == "//" then a // b else if op == "++" then a ++ b
    else a;
  wrapv = v:
    if builtins.isInt v then { tag = "int"; value = v; }
    else if builtins.isBool v then { tag = "bool"; value = v; }
    else if builtins.isString v then { tag = "string"; value = v; }
    else { tag = "const"; value = v; };
  isData = n: n.tag == "int" || n.tag == "bool" || n.tag == "string" || n.tag == "const";
  sig = env: builtins.map
    (k: let n = builtins.getAttr k env; in
        { name = k;
          v = if isData n then n
              else if n.tag == "closure" then { c = n.body; }
              else "D"; })
    (builtins.attrNames env);
  findSpec = specs: key:
    if specs == [ ] then null
    else if (builtins.head specs).key == key then builtins.head specs
    else findSpec (builtins.tail specs) key;
  findBind = binds: attr:
    if binds == [ ] then null
    else if (builtins.head binds).name == attr then builtins.head binds
    else findBind (builtins.tail binds) attr;
  entryName = e:
    if e.tag == "const" then { ok = true; v = e.value.name; }
    else if e.tag == "attrset" then
      (let h = findBind e.binds "name"; in
       if h != null && (isData h.value) then { ok = true; v = h.value.value; }
       else { ok = false; v = ""; })
    else { ok = false; v = ""; };
  namesOk = items:
    if items == [ ] then true
    else (entryName (builtins.head items)).ok && (namesOk (builtins.tail items));
  bind1 = env: name: node: env // builtins.listToAttrs [ { name = name; value = node; } ];
  arity = name: if name == "hasAttr" || name == "getAttr" || name == "seq" || name == "map"
                then 2 else 1;
  bapply = name: args: st:
    (let a0 = builtins.head args; in
     if name == "head" then
       (if a0.tag == "const" then { n = wrapv (builtins.head a0.value); st = st; }
        else if a0.tag == "list" then { n = builtins.head a0.items; st = st; }
        else { n = { tag = "bapp"; name = name; args = args; }; st = st; })
     else if name == "tail" then
       (if a0.tag == "const" then { n = { tag = "const"; value = builtins.tail a0.value; }; st = st; }
        else if a0.tag == "list" then { n = { tag = "list"; items = builtins.tail a0.items; }; st = st; }
        else { n = { tag = "bapp"; name = name; args = args; }; st = st; })
     else if name == "listToAttrs" then
       (if a0.tag == "const" then { n = { tag = "const"; value = builtins.listToAttrs a0.value; }; st = st; }
        else if a0.tag == "list" && (namesOk a0.items) then
          { n = { tag = "attrset";
                  binds = builtins.map
                    (e: if e.tag == "const"
                        then { name = e.value.name; value = wrapv e.value.value; }
                        else { name = (entryName e).v;
                               value = (findBind e.binds "value").value; })
                    a0.items; };
            st = st; }
        else { n = { tag = "bapp"; name = name; args = args; }; st = st; })
     else if name == "isInt" then
       (if isData a0 then { n = { tag = "bool"; value = a0.tag == "int"; }; st = st; }
        else { n = { tag = "bapp"; name = name; args = args; }; st = st; })
     else if name == "isBool" then
       (if isData a0 then { n = { tag = "bool"; value = a0.tag == "bool"; }; st = st; }
        else { n = { tag = "bapp"; name = name; args = args; }; st = st; })
     else if name == "isString" then
       (if isData a0 then { n = { tag = "bool"; value = a0.tag == "string"; }; st = st; }
        else { n = { tag = "bapp"; name = name; args = args; }; st = st; })
     else if name == "hasAttr" then
       (let s = builtins.head (builtins.tail args); in
        if s.tag == "const" then { n = { tag = "bool"; value = builtins.hasAttr a0.value s.value; }; st = st; }
        else if s.tag == "attrset" then
          { n = { tag = "bool"; value = (findBind s.binds a0.value) != null; }; st = st; }
        else { n = { tag = "bapp"; name = name; args = args; }; st = st; })
     else if name == "getAttr" then
       (let s = builtins.head (builtins.tail args); in
        if s.tag == "const" then { n = wrapv (builtins.getAttr a0.value s.value); st = st; }
        else if s.tag == "attrset" then { n = (findBind s.binds a0.value).value; st = st; }
        else { n = { tag = "bapp"; name = name; args = args; }; st = st; })
     else if name == "attrNames" then
       (if a0.tag == "const" then { n = { tag = "const"; value = builtins.attrNames a0.value; }; st = st; }
        else if a0.tag == "attrset" then
          { n = { tag = "const"; value = builtins.map (b: b.name) a0.binds; }; st = st; }
        else { n = { tag = "bapp"; name = name; args = args; }; st = st; })
     else if name == "toString" then
       (if isData a0 then { n = { tag = "string"; value = builtins.toString a0.value; }; st = st; }
        else { n = { tag = "bapp"; name = name; args = args; }; st = st; })
     else if name == "length" then
       (if a0.tag == "const" then { n = { tag = "int"; value = builtins.length a0.value; }; st = st; }
        else if a0.tag == "list" then { n = { tag = "int"; value = builtins.length a0.items; }; st = st; }
        else { n = { tag = "bapp"; name = name; args = args; }; st = st; })
     else if name == "seq" then
       { n = builtins.head (builtins.tail args); st = st; }
     else if name == "map" then
       (let f = a0; xs = builtins.head (builtins.tail args); in
        if xs.tag == "const" then
          (let r = mapNodes f (builtins.map wrapv xs.value) st; in
           { n = { tag = "list"; items = r.nodes; }; st = r.st; })
        else if xs.tag == "list" then
          (let r = mapNodes f xs.items st; in
           { n = { tag = "list"; items = r.nodes; }; st = r.st; })
        else { n = { tag = "bapp"; name = name; args = args; }; st = st; })
     else { n = { tag = "bapp"; name = name; args = args; }; st = st; });
  mapNodes = f: items: st:
    if items == [ ] then { nodes = [ ]; st = st; }
    else (let h = pApply f (builtins.head items) st;
              r = mapNodes f (builtins.tail items) h.st; in
          { nodes = [ h.n ] ++ r.nodes; st = r.st; });
  fresh = st: prefix: {
    name = prefix + (builtins.toString st.ctr);
    st = { specs = st.specs; ctr = st.ctr + 1; };
  };
  etaBody = nb: stx:
    if nb.tag == "closure"
    then (let pn2 = fresh stx "__a";
              inner = pmix nb.body (bind1 nb.env nb.param { tag = "var"; name = pn2.name; }) pn2.st;
              rest = etaBody inner.n inner.st; in
          { n = { tag = "lambda"; param = pn2.name; body = rest.n; }; st = rest.st; })
    else { n = nb; st = stx; };
  pmixList = items: senv: st:
    if items == [ ] then { nodes = [ ]; st = st; }
    else (let h = pmix (builtins.head items) senv st;
              r = pmixList (builtins.tail items) senv h.st; in
          { nodes = [ h.n ] ++ r.nodes; st = r.st; });
  allConst = nodes:
    if nodes == [ ] then true
    else (isData (builtins.head nodes)) && (allConst (builtins.tail nodes));
  constVals = nodes:
    if nodes == [ ] then [ ]
    else [ (builtins.head nodes).value ] ++ (constVals (builtins.tail nodes));
  pmixLet = pairs: env2: senv: st:
    if pairs == [ ] then { env = senv; st = st; res = [ ]; }
    else (let b = builtins.head pairs; in
      if b.value.tag == "lambda"
      then pmixLet (builtins.tail pairs)
                   env2
                   (bind1 senv b.name
                      { tag = "closure"; param = b.value.param; body = b.value.body; env = env2; })
                   st
      else (let r = pmix b.value senv st; in
            if (isData r.n) || (r.n.tag == "closure") || (r.n.tag == "attrset")
            then pmixLet (builtins.tail pairs) env2 (bind1 senv b.name r.n) r.st
            else (let rest = pmixLet (builtins.tail pairs) env2
                               (bind1 senv b.name { tag = "var"; name = b.name; }) r.st; in
                  { env = rest.env; st = rest.st;
                    res = [ { name = b.name; value = r.n; } ] ++ rest.res; })));
  pApply = fn: an: st:
    if fn.tag == "builtin" then
      (let args2 = fn.args ++ [ an ]; in
       if (builtins.length args2) == (arity fn.name)
       then bapply fn.name args2 st
       else { n = { tag = "builtin"; name = fn.name; args = args2; }; st = st; })
    else if fn.tag == "closure" then
      (if (isData an) || (an.tag == "closure") || (an.tag == "attrset")
       then pmix fn.body (bind1 fn.env fn.param an) st
       else (let key = { body = fn.body; s = sig fn.env; };
                 hit = findSpec st.specs key; in
             if hit != null
             then { n = { tag = "apply";
                          func = { tag = "var"; name = hit.name; }; arg = an; };
                    st = st; }
             else (let nm = fresh st "__s";
                       pn = fresh nm.st "__a";
                       seeded = { specs = pn.st.specs ++ [ { key = key; name = nm.name;
                                                             param = pn.name;
                                                             body = { tag = "var"; name = "__pending"; }; } ];
                                  ctr = pn.st.ctr; };
                       sb = pmix fn.body
                                 (bind1 fn.env fn.param { tag = "var"; name = pn.name; })
                                 seeded;
                       sbE = etaBody sb.n sb.st;
                       fin = { specs = builtins.map
                                 (s: if s.key == key
                                     then { key = s.key; name = s.name; param = s.param; body = sbE.n; }
                                     else s)
                                 sbE.st.specs;
                               ctr = sbE.st.ctr; }; in
                   { n = { tag = "apply";
                           func = { tag = "var"; name = nm.name; }; arg = an; };
                     st = fin; })))
    else { n = { tag = "apply"; func = fn; arg = an; }; st = st; };
  pmix = ast: senv: st: builtins.seq st (
    if isData ast then { n = ast; st = st; }
    else if ast.tag == "null" then { n = ast; st = st; }
    else if ast.tag == "builtin" then { n = { tag = "builtin"; name = ast.name; args = [ ]; }; st = st; }
    else if ast.tag == "var" then
      { n = if builtins.hasAttr ast.name senv then builtins.getAttr ast.name senv else ast;
        st = st; }
    else if ast.tag == "list" then
      (let r = pmixList ast.items senv st; in
       if allConst r.nodes
       then { n = { tag = "const"; value = constVals r.nodes; }; st = r.st; }
       else { n = { tag = "list"; items = r.nodes; }; st = r.st; })
    else if ast.tag == "binary" then
      (let l = pmix ast.lhs senv st; in
       if (ast.op == "&&" || ast.op == "||") && l.n.tag == "bool"
       then (if ast.op == "&&" && l.n.value == false then { n = l.n; st = l.st; }
             else if ast.op == "||" && l.n.value == true then { n = l.n; st = l.st; }
             else pmix ast.rhs senv l.st)
       else (let r = pmix ast.rhs senv l.st; in
             if (isData l.n) && (isData r.n) && ast.op != "&&" && ast.op != "||"
             then { n = wrapv (fold ast.op l.n.value r.n.value); st = r.st; }
             else if ast.op == "++" && (l.n.tag == "list" || isData l.n)
                     && (r.n.tag == "list" || isData r.n)
             then (let li = if l.n.tag == "list" then l.n.items
                            else builtins.map wrapv l.n.value;
                       ri = if r.n.tag == "list" then r.n.items
                            else builtins.map wrapv r.n.value; in
                   { n = { tag = "list"; items = li ++ ri; }; st = r.st; })
             else { n = { tag = "binary"; op = ast.op; lhs = l.n; rhs = r.n; }; st = r.st; }))
    else if ast.tag == "if" then
      (let c = pmix ast.cond senv st; in
       if c.n.tag == "bool"
       then (if c.n.value then pmix ast.t senv c.st else pmix ast.e senv c.st)
       else (let tb = pmix ast.t senv c.st; eb = pmix ast.e senv tb.st; in
             { n = { tag = "if"; cond = c.n; t = tb.n; e = eb.n; }; st = eb.st; }))
    else if ast.tag == "lambda" then
      { n = { tag = "closure"; param = ast.param; body = ast.body; env = senv; }; st = st; }
    else if ast.tag == "select" then
      (let b = pmix ast.base senv st; in
       if b.n.tag == "const" then { n = wrapv (builtins.getAttr ast.attr b.n.value); st = b.st; }
       else if b.n.tag == "attrset" then
         (let hit = findBind b.n.binds ast.attr; in
          if hit != null then { n = hit.value; st = b.st; }
          else { n = { tag = "select"; base = b.n; attr = ast.attr; }; st = b.st; })
       else { n = { tag = "select"; base = b.n; attr = ast.attr; }; st = b.st; })
    else if ast.tag == "attrset" then
      (let r = pmixAttrs ast.binds senv st; in
       if allConst (builtins.map (p: p.value) r.pairs)
       then { n = { tag = "const";
                    value = builtins.listToAttrs
                      (builtins.map (p: { name = p.name; value = p.value.value; }) r.pairs); };
              st = r.st; }
       else { n = { tag = "attrset"; binds = r.pairs; }; st = r.st; })
    else if ast.tag == "apply" then
      (let f = pmix ast.func senv st; a = pmix ast.arg senv f.st; in
       pApply f.n a.n a.st)
    else if ast.tag == "let" then
      (let env2 = (pmixLet ast.binds env2 senv st).env;
           done = pmixLet ast.binds env2 senv st;
           body = pmix ast.body done.env done.st; in
       if done.res == [ ]
       then body
       else { n = { tag = "let"; binds = done.res; body = body.n; }; st = body.st; })
    else { n = { tag = "unsupported"; reason = ast.tag; }; st = st; });
  pmixAttrs = binds: senv: st:
    if binds == [ ] then { pairs = [ ]; st = st; }
    else (let b = builtins.head binds;
              r = pmix b.value senv st;
              rest = pmixAttrs (builtins.tail binds) senv r.st; in
          { pairs = [ { name = b.name; value = r.n; } ] ++ rest.pairs; st = rest.st; });
in pmix
"""



def _decode_full(enc: dict[str, Any]) -> str:
    """Decode a residual node INCLUDING lambda/apply/let/select/attrset (M5)."""
    tag = enc["tag"]
    if tag in ("int", "bool", "string", "const", "var"):
        return _decode(enc)
    if tag == "binary":
        return f"({_decode_full(enc['lhs'])} {enc['op']} {_decode_full(enc['rhs'])})"
    if tag == "if":
        return (f"(if {_decode_full(enc['cond'])} then {_decode_full(enc['t'])} "
                f"else {_decode_full(enc['e'])})")
    if tag == "lambda":
        return f"({enc['param']}: {_decode_full(enc['body'])})"
    if tag == "apply":
        return f"({_decode_full(enc['func'])} {_decode_full(enc['arg'])})"
    if tag == "select":
        return f"({_decode_full(enc['base'])}).{enc['attr']}"
    if tag == "attrset":
        fields = " ".join(f"{p['name']} = {_decode_full(p['value'])};" for p in enc["binds"])
        return "{ " + fields + " }"
    if tag == "let":
        binds = "; ".join(f"{p['name']} = {_decode_full(p['value'])}" for p in enc["binds"])
        return f"let {binds}; in {_decode_full(enc['body'])}"
    if tag == "list":
        return "[ " + " ".join(f"({_decode_full(x)})" for x in enc["items"]) + " ]"
    if tag == "null":
        return "null"
    if tag == "bapp":
        return "(builtins." + enc["name"] + " " + " ".join(_decode_full(a) for a in enc["args"]) + ")"
    if tag == "builtin":
        return "builtins." + enc["name"]
    raise ValueError(f"cannot decode: {tag}")


def _eval_deep(call: str) -> Any:
    """Evaluate a pnix source whose LAZY state-threading forces as one enormous recursion at
    realize time: run in a worker thread with a large stack + a high recursion limit (the host
    tree-walker's recursion IS the deferred computation)."""
    import sys  # noqa: PLC0415
    import threading  # noqa: PLC0415

    result: dict[str, Any] = {}

    def _run() -> None:
        limit = sys.getrecursionlimit()
        try:
            sys.setrecursionlimit(1_000_000)
            result["out"] = rt.stable_data(rt.eval_source(call))
        except BaseException as exc:  # noqa: BLE001 - surface in the caller thread
            result["err"] = exc
        finally:
            sys.setrecursionlimit(limit)

    old_stack = threading.stack_size()
    threading.stack_size(512 * 1024 * 1024)  # deep force chains need real stack headroom
    try:
        worker = threading.Thread(target=_run, name="pnix-deep-eval")
        worker.start()
        worker.join()
    finally:
        threading.stack_size(old_stack)
    if "err" in result:
        raise result["err"]
    return result["out"]


def poly_mix_in_pnix(source: str, static_env: dict[str, Any]) -> dict[str, Any]:
    """0026 M5a: run the pnix-EXPRESSED POLYVARIANT specializer on a core-subset program --
    the outer specializer itself now satisfies S=L on this subset. Dynamic recursion comes
    back as named residual recursive definitions (computed BY pnix, state-passing memo)."""
    enc = _encode(rt.ast_stable(rt.parse(source)))
    senv_nodes = {k: _wrap_node(v) for k, v in static_env.items()}
    body = POLY_MIX_IN_PNIX.strip()
    assert body.startswith("let") and body.endswith("in pmix")
    binds = body[: -len("in pmix")]
    call = (f"{binds}__r = pmix ({_pnix_literal(enc)}) ({_pnix_literal(senv_nodes)}) "
            f"({{ specs = [ ]; ctr = 0; }}); in {{ n = __r.n; specs = __r.st.specs; }}")
    out = _eval_deep(call)
    main = _decode_full(out["n"])
    specs = out.get("specs") or []
    if specs:
        defs = "; ".join(f"{s['name']} = {s['param']}: {_decode_full(s['body'])}" for s in specs)
        residual = f"let {defs}; in {main}"
    else:
        residual = main
    return {"schema": "pnix-hy.poly-mix-in-pnix.v0", "residual": residual,
            "specialization_points": len(specs)}
