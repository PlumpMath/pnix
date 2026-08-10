#!/usr/bin/env python3
"""Developer CLI for the Hy/Python <-> pnix meta-circular projection toolkit.

pnix-hy's purpose is to project LANGUAGE EXPRESSIVENESS between the Python-language
ecosystem (Hy, Python) meta-circular and pnix's meta-circular, for a developer to study.
This is the hands-on entry point: run the projection facilities on a snippet or file.

Examples:
    bin/pnix-hy-project --hy '(defn f [x] (+ x 1))'
    bin/pnix-hy-project --pnix 'let inc = x: x + 1; in inc 41'
    bin/pnix-hy-project --hy '(when c x)' --json
    bin/pnix-hy-project --correspondence
    bin/pnix-hy-project --check        # run every toolkit self-check
"""

from __future__ import annotations

import argparse
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from pnix_hy import hy_mirror as hm  # noqa: E402
from pnix_hy import pnix_mirror as pm  # noqa: E402


def _reader_line(forms: list, indent: int = 0) -> list[str]:
    out: list[str] = []
    for f in forms or []:
        model = f.get("model")
        label = model
        if model in ("Symbol", "Keyword"):
            label = f"{model}({f.get('name')})"
        elif "value" in f:
            label = f"{model}({f.get('value')!r})"
        out.append("  " * indent + label)
        out.extend(_reader_line(f.get("items", []), indent + 1))
    return out


def _render_align_tree(nodes: list, indent: int = 0) -> list[str]:
    out: list[str] = []
    for n in nodes:
        star = "*" if n.get("differs") else ""
        out.append("  " * indent + f"{n['python_node']} -> {n['pnix_tag']}{star}")
        out.extend(_render_align_tree(n.get("children", []), indent + 1))
    return out


def cmd_hy(source: str, as_json: bool) -> None:
    proj = pm.hy_pnix_projection(source)
    if as_json:
        print(json.dumps(proj, indent=2, ensure_ascii=False))
        return
    print("== Hy source ==")
    print(" ", source)
    print("\n== reader forms ==")
    for line in _reader_line(proj.get("reader_forms") or []):
        print(" ", line)
    macro = proj.get("macro") or {}
    print(f"\n== macro layer ==  is_macro={macro.get('is_macro')}")
    if macro.get("is_macro"):
        print("  fully-expanded ->", (macro.get("fully_expanded_python") or "").replace("import hy\n", "").strip())
    print("\n== lowered Python ==")
    print("  " + (proj.get("python_source") or "").replace("\n", "\n  "))
    tree = pm.align_python_to_pnix_tree(proj.get("python_source") or "")
    print("\n== pnix correspondence (tree; * = differs) ==")
    for line in _render_align_tree(tree.get("tree") or []):
        print(" ", line)


def cmd_pnix(source: str, as_json: bool) -> None:
    proj = pm.pnix_form_projection(source)
    rev = pm.pnix_to_hy_form(source)
    if as_json:
        print(json.dumps({"projection": proj, "reverse_hy": rev}, indent=2, ensure_ascii=False))
        return
    print("== pnix source ==")
    print(" ", source)
    print(f"\n== pnix AST ==  root tag = {proj.get('ast_root_tag')}")
    print("  " + json.dumps(proj.get("ast"), ensure_ascii=False))
    print("\n== value ==")
    print(f"  interp={proj.get('value')!r}  json={proj.get('value_json')!r}  lanes_agree={proj.get('lanes_agree')}")
    print(f"  canonical emit: {proj.get('emitted_source')!r}")
    print("\n== reverse projection (synthesized Hy) ==")
    print("  " + rev.get("hy_source", ""))
    if rev.get("gaps"):
        print("  gaps:", ", ".join(f"{g['tag']}({g['reason']})" for g in rev["gaps"]))


def cmd_pnix_file(path: str, as_json: bool) -> None:
    """Project a file with relative imports rooted at that file's directory."""
    absolute = os.path.abspath(path)
    with open(absolute, encoding="utf-8") as fh:
        source = fh.read()
    previous = os.getcwd()
    try:
        os.chdir(os.path.dirname(absolute) or previous)
        cmd_pnix(source, as_json)
    finally:
        os.chdir(previous)


def cmd_hy_module(source: str, as_json: bool) -> None:
    proj = pm.project_hy_module(source)
    if as_json:
        print(json.dumps(proj, indent=2, ensure_ascii=False))
        return
    print(f"== Hy module projection ({proj.get('form_count')} top-level forms) ==")
    for i, f in enumerate(proj.get("forms") or []):
        al = f.get("pnix_alignment") or {}
        macro = " (macro)" if f.get("is_macro") else ""
        print(f"\n  [{i}] ({f.get('head')}){macro}")
        py = (f.get("python_source") or "").replace("import hy\n", "").strip()
        print("      -> " + py.replace("\n", "\n         "))
        print("      pnix: " + ", ".join(al.get("pnix_tags_hit", [])))


def cmd_import(source: str, as_json: bool) -> None:
    proj = hm.hy_import_projection(source)
    if as_json:
        print(json.dumps(proj, indent=2, ensure_ascii=False))
        return
    print(f"== Hy import/require projection ({proj.get('entry_count')} forms) ==")
    for e in proj.get("entries") or []:
        print(f"\n  ({e.get('kind')} ...)  [{e.get('stage')}]")
        if e.get("python_source"):
            print("    -> " + e["python_source"])
        if e.get("error"):
            print(f"    (not loaded: {e['error']})")
    pc = proj.get("pnix_correspondence") or {}
    print("\n  pnix import :", pc.get("pnix_import"))
    print("  pnix require:", pc.get("pnix_require_reason"))


def cmd_reader_macro(source: str, as_json: bool) -> None:
    proj = hm.hy_reader_macro_projection(source)
    if as_json:
        print(json.dumps(proj, indent=2, ensure_ascii=False))
        return
    print("== Hy reader-macro projection (the READ-TIME stage of the tower) ==")
    if proj.get("error"):
        print("  error:", proj["error"])
    for dr in proj.get("defreaders") or []:
        print(f"\n  (defreader {dr.get('name')} ...)  registers={dr.get('registers')}")
        py = (dr.get("python_source") or "").replace("import hy\n", "").strip()
        print("    read-time transformer (reader -> model) + registration:")
        print("      " + py.replace("\n", "\n      "))
    forms = proj.get("produced_forms") or []
    uses = [f for f in forms if not f.get("is_defreader")]
    if uses:
        print("\n  read-time expansion (#name consumed by the reader -> model):")
        for f in uses:
            print(f"    -> {json.dumps(f.get('form'), ensure_ascii=False)}")
    print("\n  user reader macros:", proj.get("user_reader_macros"))
    print("  builtin dispatch  :", proj.get("builtin_dispatch"))
    pc = proj.get("pnix_correspondence") or {}
    print("\n  pnix :", pc.get("reason"))
    print("  tower:", pc.get("staging_tower"))


def cmd_defmacro(source: str, as_json: bool) -> None:
    proj = hm.hy_defmacro_projection(source)
    if as_json:
        print(json.dumps(proj, indent=2, ensure_ascii=False))
        return
    print(f"== Hy defmacro projection ({proj.get('defmacro_count')} macro definitions) ==")
    for dm in proj.get("defmacros") or []:
        qq = " (quasiquote body)" if dm.get("body_is_quasiquote") else ""
        print(f"\n  (defmacro {dm.get('name')} [{' '.join(dm.get('params') or [])}] ...){qq}")
        py = (dm.get("python_source") or "").replace("import hy\n", "").strip()
        print("    compile-time transformer + registration:")
        print("      " + py.replace("\n", "\n      "))
    exps = proj.get("expansions") or []
    if exps:
        print("\n  form -> form rewrites:")
        for e in exps:
            if e.get("expand_error"):
                print(f"    ({e.get('call_head')} ...) -> error: {e['expand_error']}")
            else:
                head = ((e.get("expanded") or {}).get("items") or [{}])[0].get("name")
                print(f"    ({e.get('call_head')} ...) -> expands to ({head} ...)  changed={e.get('changed')}")
    pc = proj.get("pnix_correspondence") or {}
    print("\n  pnix   :", pc.get("reason"))
    print("  staging:", pc.get("staging_analogue"))


def cmd_quasiquote(source: str, as_json: bool) -> None:
    proj = hm.hy_quasiquote_projection(source)
    if as_json:
        print(json.dumps(proj, indent=2, ensure_ascii=False))
        return
    print("== Hy quasiquote projection (code-as-data; the staging view) ==")
    print(f"  source       : {source}")
    print(f"  template kind: {proj.get('template_kind')}  "
          f"(skeleton nodes {proj.get('static_skeleton_nodes')}, holes {proj.get('hole_count')})")
    print("  builds tree  : " + (proj.get("python_source") or "").replace("import hy\n", "").strip())
    for h in proj.get("holes") or []:
        print(f"    hole [{h['kind']}] -> {h.get('repr')}")
    pc = proj.get("pnix_correspondence") or {}
    print("  pnix         :", pc.get("reason"))
    print("  staging      :", pc.get("binding_time_analogue"))


def cmd_macro_steps(source: str, as_json: bool) -> None:
    tr = hm.hy_macro_step_trace(source)
    if as_json:
        print(json.dumps(tr, indent=2, ensure_ascii=False))
        return
    print(f"== Hy macro step trace ({tr.get('form_count')} top-level forms) ==")
    for f in tr.get("forms") or []:
        macro = f"macro, {f.get('step_count')} steps" if f.get("is_macro") else "not a macro"
        print(f"\n  ({f.get('head')})  [{macro}]")
        chain = " -> ".join(f"({s.get('head')})" if s.get("head") else "<atom>"
                            for s in f.get("steps") or [])
        print("    tower:", chain)
        last = (f.get("steps") or [{}])[-1]
        py = (last.get("python_source") or "").replace("import hy\n", "").strip()
        if py:
            print("    final Python:", py.replace("\n", "\n      "))


def cmd_pnix_meta(source: str, as_json: bool) -> None:
    proj = pm.pnix_meta_circular_projection(source)
    if as_json:
        print(json.dumps(proj, indent=2, ensure_ascii=False))
        return
    print("== pnix meta-circular (one form across all four substrates) ==")
    print(" ", source)
    print(f"\n  converged = {proj['converged']}")
    for lane, desc in proj["substrates"].items():
        val = proj["lanes"].get(lane, proj["errors"].get(lane, "?"))
        print(f"    {lane:16} ({desc:28}) -> {val!r}")
    if proj["errors"]:
        print("  errors:", proj["errors"])


def cmd_roundtrip(source: str, as_json: bool) -> None:
    r = pm.projection_value_roundtrip(source)
    if as_json:
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return
    print("== projection value-roundtrip (does meaning survive pnix -> Hy?) ==")
    print(" ", source)
    if not r.get("comparable"):
        print(f"  not comparable: {r.get('reason')}")
        return
    print(f"  pnix value : {r.get('pnix_value')!r}")
    print(f"  synth Hy   : {r.get('hy_source')}")
    print(f"  Hy value   : {r.get('hy_value')!r}   (stage7 kernel)")
    print(f"  meaning preserved: {r.get('meaning_preserved')}")
    if r.get("gaps"):
        print("  semantic notes:", ", ".join(g["reason"] for g in r["gaps"]))


def cmd_specialize(spec: str, as_json: bool) -> None:
    # "SRC" or "SRC ;; x,y" to mark x,y dynamic
    src, dyn = _split_capability_spec(spec)
    if dyn is None:
        dyn = ()
    try:
        r = pm.specialize_pnix(src, dyn)
    except Exception as exc:  # noqa: BLE001 - convert parser/runtime regressions to diagnosis
        if as_json:
            print(json.dumps({
                "schema": "pnix-hy.cli.specialize.error.v0",
                "ok": False,
                "source": src,
                "phase": "specialize",
                "error": f"{type(exc).__name__}: {exc}",
            }, indent=2, ensure_ascii=False))
            return
        print(f"error: {type(exc).__name__}: {exc}")
        return
    if as_json:
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return
    print("== Futamura 1st-projection (partial-evaluate the pnix interpreter) ==")
    print(f"  pnix    : {src}")
    print(f"  dynamic : {list(dyn)}")
    if r["fully_static"]:
        print(f"  -> fully static (no interpretive residue): {r['residual_hy']}  (value {r.get('value')!r})")
    else:
        print(f"  -> residual Hy (statics folded, dynamics kept): {r['residual_hy']}")
    if r.get("gaps"):
        print("  notes:", ", ".join(g.get("reason", g.get("tag", "")) for g in r["gaps"]))


def cmd_trace(source: str, as_json: bool) -> None:
    r = pm.pnix_evaluation_trace(source)
    if as_json:
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return
    print("== live meta-circular trace (Python bytecode the pnix evaluator runs) ==")
    print(f"  pnix   : {source}")
    if r.get("error"):
        print(f"  error  : {r['error']}")
        return
    print(f"  value  : {r.get('value')!r}")
    print(f"  runtime: {r['runtime_functions_distinct']} fns, "
          f"{r['runtime_calls_total']} calls, {r['runtime_opcodes_total']} opcodes executed")
    print("  top functions:", ", ".join(f"{f['name']}({f['calls']})" for f in r["top_functions"][:8]))
    print("  top opcodes  :", ", ".join(f"{o['opname']}({o['count']})" for o in r["top_opcodes"][:8]))


def cmd_hy_trace(source: str, as_json: bool) -> None:
    try:
        r = hm.execution_trace(source, "eval")
    except Exception as exc:  # noqa: BLE001 - avoid raw tracebacks for CLI diagnostics
        if as_json:
            print(json.dumps({
                "schema": "pnix-hy.cli.hy-trace.error.v0",
                "ok": False,
                "source": source,
                "phase": "hy-trace",
                "error": f"{type(exc).__name__}: {exc}",
            }, indent=2, ensure_ascii=False))
            return
        print(f"error: {type(exc).__name__}: {exc}")
        return
    if as_json:
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return
    print("== live execution trace (opcodes a Python/Hy-lowered fragment runs) ==")
    print(f"  source : {source}")
    if r.get("error"):
        print(f"  error  : {r['error']}")
    print(f"  result : {r.get('result_repr')}")
    print(f"  executed {r['executed_opcode_count']} opcodes "
          f"({r['executed_distinct']} distinct); static listing {r['static_opcode_count']}")
    if r.get("static_only_opcodes"):
        print("  present-but-not-run:", ", ".join(r["static_only_opcodes"]))
    print("  sequence:", " ".join(e["opname"] for e in r["event_sequence"][:16]))


def cmd_synth_pnix(source: str, as_json: bool) -> None:
    r = pm.synthesize_pnix_from_hy(source)
    if as_json:
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return
    print("== Hy -> pnix synthesis (reverse of pnix_to_hy_form) ==")
    print(f"  Hy     : {source}")
    print("  Python : " + (r.get("python_source") or "").replace("import hy\n", "").strip())
    print(f"  pnix   : {r.get('pnix_source')}   (synthesizable={r.get('synthesizable')})")
    for g in r.get("gaps") or []:
        print(f"    gap [{g.get('node')}] {g.get('reason')}")


def cmd_hy_roundtrip(source: str, as_json: bool) -> None:
    r = pm.hy_to_pnix_value_roundtrip(source)
    if as_json:
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return
    print("== Hy -> pnix value roundtrip (does the forward projection preserve meaning?) ==")
    print(f"  Hy     : {source}")
    print(f"  pnix   : {r.get('pnix_source')}")
    if not r.get("comparable"):
        print(f"  not comparable: {r.get('reason')}")
        return
    print(f"  Hy value  : {r.get('hy_value')!r}   (host Python lowering)")
    print(f"  pnix value: {r.get('pnix_value')!r}   (host pnix)")
    print(f"  meaning preserved: {r.get('meaning_preserved')}")


def cmd_closure(source: str, as_json: bool) -> None:
    r = pm.pnix_projection_closure(source)
    if as_json:
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return
    print("== pnix projection closure (pnix -> Hy -> pnix; involution by value) ==")
    print(f"  pnix   : {source}")
    print(f"  -> Hy  : {r.get('hy_source')}")
    print(f"  -> pnix: {r.get('pnix_source_2')}")
    if not r.get("comparable"):
        print(f"  not comparable: {r.get('reason')}")
        return
    print(f"  value  : {r.get('value')!r}   closed (meaning preserved both ways): {r.get('closed')}")


def cmd_hy_closure(source: str, as_json: bool) -> None:
    r = pm.hy_projection_closure(source)
    if as_json:
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return
    print("== Hy projection closure (Hy -> pnix -> Hy; involution by value) ==")
    print(f"  Hy     : {source}")
    print(f"  -> pnix: {r.get('pnix_source')}")
    print(f"  -> Hy  : {r.get('hy_source_2')}")
    if not r.get("comparable"):
        print(f"  not comparable: {r.get('reason')}")
        return
    print(f"  closed (meaning preserved both ways): {r.get('closed')}")


def cmd_ir(source: str, as_json: bool) -> None:
    from pnix_hy import ir as ir_mod  # noqa: PLC0415
    b = ir_mod.ir_of(source)
    rt_ = ir_mod.ir_roundtrip(source)
    if as_json:
        print(json.dumps({"ir": b, "roundtrip": rt_}, indent=2, ensure_ascii=False))
        return
    print("== pnix IR (canonical runtime representation; AST -> IR) ==")
    print(f"  pnix      : {source}")
    print(f"  root tag  : {b.get('root_tag')}")
    print(f"  ir_sha256 : {b.get('ir_sha256')}")
    print(f"  evaluable : {rt_.get('ir_evaluable')}   meaning preserved: {rt_.get('meaning_preserved')}"
          f"   hash stable: {rt_.get('hash_stable')}   value: {rt_.get('value')!r}")


def cmd_gate_check(spec: str, as_json: bool) -> None:
    from pnix_hy import gate as gt  # noqa: PLC0415
    src, granted = _split_capability_spec(spec)
    if granted is None:
        granted = ()
    r = gt.gate_check(src, granted=granted)
    if as_json:
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return
    print("== capability gate (admit only if required effects are granted) ==")
    print(f"  pnix    : {src}")
    print(f"  required: {r.get('required_effects')}   granted: {r.get('granted')}")
    for d in r.get("denials") or []:
        print(f"    DENIED: {d.get('name')} needs {d.get('effect')}")
    print(f"  ADMIT   : {r.get('allowed')}")


def _split_capability_spec(spec: str) -> tuple[str, tuple[str, ...] | None]:
    if ";;" not in spec:
        return spec, None
    quote: str | None = None
    escape = False
    for i in range(len(spec) - 1):
        ch = spec[i]
        if escape:
            escape = False
            continue
        if ch == "\\":
            escape = True
            continue
        if ch in "\"'":
            if quote is None:
                quote = ch
            elif quote == ch:
                quote = None
            continue
        if quote is None and ch == ";" and spec[i + 1] == ";":
            src = spec[:i]
            raw = spec[i + 2:]
            granted = tuple(c.strip() for c in raw.split(",") if c.strip())
            return src.strip(), granted
    return spec, None


def cmd_reify(source: str, as_json: bool) -> None:
    r = pm.reify_pnix(source)
    if as_json:
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return
    print("== pnix reification (source/form/AST/IR/value/effect/witness) ==")
    print(f"  pnix    : {source}")
    if not r.get("ok"):
        print(f"  {r.get('phase')} error: {r.get('error')}")
        return
    reified = r.get("reified") or {}
    print(f"  ast     : {(reified.get('ast') or {}).get('root_tag')} "
          f"{(reified.get('ast') or {}).get('sha256', '')[:16]}")
    print(f"  ir      : {(reified.get('ir') or {}).get('sha256', '')[:16]}")
    print(f"  value   : {(reified.get('value') or {}).get('data')!r}")
    print(f"  effect  : pure={(reified.get('effect') or {}).get('pure')}")
    print(f"  witness : {(reified.get('witness') or {}).get('result_sha256', '')[:16]}")


def cmd_roundtrip_status(source: str, as_json: bool) -> None:
    r = pm.roundtrip_status(source)
    if as_json:
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return
    print("== pnix roundtrip status vocabulary ==")
    print(f"  pnix      : {source}")
    print(f"  vocabulary: {', '.join(r.get('vocabulary') or [])}")
    for name, status in (r.get("statuses") or {}).items():
        print(f"    {name}: {status}")
    print(f"  lossless  : {r.get('lossless')}")


def cmd_explain(spec: str, as_json: bool) -> None:
    src, granted = _split_capability_spec(spec)
    r = pm.explain_pnix(src, granted=granted)
    if as_json:
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return
    print("== pnix explain (diagnose + purity + gate + mirror + safe-eval) ==")
    print(f"  pnix  : {src}")
    print(f"  phase : {r.get('phase')}   ok={r.get('ok')}")
    if r.get("gate"):
        g = r["gate"]
        print(f"  gate  : required={g.get('required_effects')} granted={g.get('granted')} allowed={g.get('allowed')}")
    d = r.get("diagnostic") or {}
    if not d.get("ok"):
        print(f"  error : {d.get('phase')} {d.get('message')}")
        if d.get("excerpt"):
            print("    " + d["excerpt"].replace("\n", "\n    "))
        return
    mirror = r.get("mirror") or {}
    safe = r.get("safe_eval") or {}
    print(f"  mirror: run_id={mirror.get('run_id', '')[:16]} facets={mirror.get('facet_count')}")
    print(f"  value : {safe.get('value')!r}")


def cmd_action_check(spec: str, as_json: bool) -> None:
    from pnix_hy import action as act  # noqa: PLC0415
    src, granted = _split_capability_spec(spec)
    r = act.check_action(src, granted=granted or ())
    if as_json:
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return
    print("== pnix action check (gate + witness + explain checkpoint) ==")
    print(f"  pnix    : {src}")
    print(f"  status  : {r.get('status')}   phase={r.get('phase')}")
    print(f"  effects : {r.get('effects')}   granted={r.get('granted')}")
    print(f"  source  : {(r.get('source_hash') or '')[:16]}")
    print(f"  ir      : {(r.get('ir_hash') or '')[:16]}")
    print(f"  value   : {(r.get('value_hash') or '')[:16]}")
    print(f"  witness : {r.get('witness_id')}")


def cmd_action_explain(spec: str, as_json: bool) -> None:
    from pnix_hy import action as act  # noqa: PLC0415
    src, granted = _split_capability_spec(spec)
    r = act.verify_action(src, intent="cli-action", before_snapshot={"cli": "hash-ref-only"},
                          granted=granted or ())
    if as_json:
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return
    verdict = r.get("verdict") or {}
    action = r.get("action") or {}
    print("== pnix action explain (semantic checkpoint) ==")
    print(f"  action  : {action.get('action_id')}")
    print(f"  rollback: {action.get('rollback_ref')}")
    print(f"  status  : {verdict.get('status')}   phase={verdict.get('phase')}")
    print(f"  effects : {verdict.get('effects')}   granted={verdict.get('granted')}")
    print(f"  witness : {verdict.get('witness_id')}")
    gate = verdict.get("gate") or {}
    if gate.get("denials"):
        for denial in gate.get("denials") or []:
            print(f"    HELD: {denial.get('name')} needs {denial.get('effect')}")


def cmd_stage_ladder(source: str, as_json: bool) -> None:
    from pnix_hy import stage as st  # noqa: PLC0415
    r = st.pnix_stage_ladder(source)
    if as_json:
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return
    print("== pnix runtime stage ladder (distinct from hy-meta stage8/9) ==")
    print(f"  pnix : {source}")
    for s in r.get("stages", []):
        print(f"    [{'PASS' if s['ok'] else 'FAIL'}] {s['stage']}: {s['detail']}")
    print(f"  value: {r.get('value')!r}   LADDER: {'PASS' if r.get('passed') else 'FAIL'}")


def cmd_perf(source: str, as_json: bool) -> None:
    r = pm.performance_report(source)
    if as_json:
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return
    timings = r.get("timings") or {}
    values = r.get("values") or {}
    print("== pnix runtime performance ==")
    print(f"  ready: {r.get('ready')}   values_agree: {values.get('agree')}")
    print(f"  generated_python_bytes: {r.get('generated_python_bytes')}   bytecode_ops: {r.get('bytecode_op_count')}")
    for name in (
        "parse",
        "canonical_emit",
        "compiler_emit",
        "python_compile",
        "interpreter_eval",
        "compiler_compile_exec",
        "compiler_exec_many",
    ):
        t = timings.get(name) or {}
        print(f"    {name}: {t.get('per_iter_ns')} ns/iter ({t.get('iterations')} iters)")


def cmd_mirror(source: str, as_json: bool) -> None:
    from pnix_hy import mirror as mr  # noqa: PLC0415
    r = mr.mirror_run(source, trace=True)
    if as_json:
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return
    print("== mirror_run (the SINGLETON pnix runtime mirror; one run, many facets) ==")
    print(f"  pnix   : {source}")
    print(f"  run_id : {r.get('run_id', '')[:16]}   ok={r.get('ok')}")
    for f in r.get("facets", []):
        kind = f["facet"]
        rest = {k: v for k, v in f.items() if k != "facet"}
        print(f"    {kind:18} {rest}")
    if r.get("ok"):
        print(f"  value  : {r.get('value')!r}   result_sha256={r.get('result_sha256','')[:16]}")


def cmd_interop(source: str, as_json: bool) -> None:
    from pnix_hy import interop as ip  # noqa: PLC0415
    value, rec = ip.to_host_eval(source)
    if as_json:
        print(json.dumps({"host_value": value if not ip.is_opaque_ref(value) else value,
                          "record": rec.to_dict()}, indent=2, ensure_ascii=False))
        return
    print("== interop: pnix value -> host (explicit boundary) ==")
    print(f"  pnix     : {source}")
    if ip.is_opaque_ref(value):
        print(f"  -> host  : opaque ref {value.get('kind')} ({value.get('repr')})")
    else:
        print(f"  -> host  : {value!r}")
    d = rec.to_dict()
    print(f"  record   : {d['direction']} {d['input_kind']}->{d['output_kind']} "
          f"loss={d['loss_status']} effect={d['effect_class']} cap={d['capability_required']}")


def cmd_receipt(source: str, as_json: bool) -> None:
    r = pm.eval_receipt(source)
    if as_json:
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return
    print("== eval receipt (reproducibility / audit bundle) ==")
    print(f"  source       : {source}")
    print(f"  canonical    : {r.get('canonical_emit')}")
    print(f"  source sha256: {r.get('source_sha256')}")
    if not r.get("ok"):
        print(f"  {r.get('phase')} error: {r.get('error')}")
        return
    print(f"  value        : {r.get('value')!r}")
    print(f"  value sha256 : {r.get('value_sha256')}")
    print(f"  pure         : {r.get('pure')}")
    conv = r.get("convergence") or {}
    print(f"  4-lane converged: {conv.get('converged')}  lanes={conv.get('lanes')}")
    tr = r.get("trace") or {}
    print(f"  trace        : {tr.get('runtime_functions_distinct')} fns, "
          f"{tr.get('runtime_calls_total')} calls, {tr.get('runtime_opcodes_total')} opcodes")


def cmd_diagnose(source: str, as_json: bool) -> None:
    r = pm.diagnose(source)
    if as_json:
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return
    print("== diagnose (structured eval error with source position) ==")
    if r.get("ok"):
        print(f"  ok: value {r.get('value')!r}")
        return
    where = f" (line {r['line']}, col {r['column']})" if r.get("line") else ""
    print(f"  {r.get('phase')} error{where}: {r.get('message')}")
    if r.get("excerpt"):
        print("    " + r["excerpt"].replace("\n", "\n    "))


def cmd_purity(source: str, as_json: bool) -> None:
    r = pm.static_purity_check(source)
    if as_json:
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return
    print("== static purity check (does this pnix touch the outside world?) ==")
    print(f"  pnix : {source}")
    print(f"  pure : {r.get('pure')}")
    for u in r.get("impure_uses") or []:
        print(f"    impure: {u.get('kind')} {u.get('name')}")
    for u in r.get("uncertain") or []:
        print(f"    uncertain: {u.get('kind')} -- {u.get('reason')}")
    if r.get("parse_error"):
        print(f"  parse error: {r['parse_error']}")


def cmd_safe_eval(source: str, as_json: bool) -> None:
    r = pm.safe_eval(source)
    if as_json:
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return
    print("== safe eval (untrusted pnix under resource bounds) ==")
    print(f"  pnix    : {source}")
    print(f"  steps   : {r.get('steps')}   elapsed: {r.get('elapsed_s')}s")
    if r.get("ok"):
        print(f"  value   : {r.get('value')!r}")
        if r.get("note"):
            print(f"  note    : {r['note']}")
    elif r.get("limit_exceeded"):
        print(f"  BLOCKED : limit exceeded -> {r['limit_exceeded']}")
    else:
        print(f"  error   : {r.get('error')}")


def cmd_tower(source: str, as_json: bool) -> None:
    t = pm.meta_circular_tower(source)
    if as_json:
        print(json.dumps(t, indent=2, ensure_ascii=False))
        return
    s = t.get("stages") or {}
    read, comp, run = s.get("read") or {}, s.get("compile") or {}, s.get("run") or {}
    pnix, collapse = s.get("pnix") or {}, s.get("collapse") or {}
    print("== meta-circular tower (the whole journey for one Hy fragment) ==")
    print(f"  Hy        : {source}")
    print(f"  [read]    : {read.get('reader_form_count')} reader form(s)")
    for mt in comp.get("macro_towers") or []:
        chain = " -> ".join(h if h else "<atom>" for h in mt.get("tower") or [])
        print(f"  [compile] : macro tower ({mt.get('head')}): {chain}")
    print("  [compile] : Python -> " + (comp.get("python_source") or "").replace("import hy\n", "").strip())
    if run.get("traced"):
        tops = ", ".join(f"{op}({n})" for op, n in run.get("top_opcodes") or [])
        print(f"  [run]     : {run.get('opcodes_executed')} opcodes executed (top: {tops}) -> {run.get('result')}")
    else:
        print(f"  [run]     : (not traced: {run.get('reason') or run.get('error')})")
    print(f"  [pnix]    : synth {pnix.get('synth_pnix')}  | value preserved={pnix.get('value_preserved')} closed={pnix.get('closed')}")
    if collapse:
        kind = "fully static (no interpretive residue)" if collapse.get("fully_static") else "residual"
        print(f"  [collapse]: specialize -> {collapse.get('residual_hy')}  ({kind}; value {collapse.get('value')})")


def cmd_correspondence(as_json: bool) -> None:
    table = pm.correspondence_table()
    if as_json:
        print(json.dumps(table, indent=2, ensure_ascii=False))
        return
    print(f"Python/Hy AST  <->  pnix AST/value  ({table['row_count']} rows; "
          f"{table['clean_1to1']} clean 1:1, {table['differs']} differ*)\n")
    for r in table["rows"]:
        star = "*" if r["differs"] else " "
        print(f" {star} {r['category']:26} | {r['python_ast']:28} | {r['hy_form']:22} "
              f"| {r['pnix_tag']:10} | {r['pnix_value_type']}")


def _toolkit_reports() -> dict:
    return {
        "hy_form_projection": hm.hy_form_projection_report,
        "hy_meta_circular_projection": hm.hy_meta_circular_projection_report,
        "hy_macroexpand_projection": hm.hy_macroexpand_projection_report,
        "hy_macro_step_trace": hm.hy_macro_step_trace_report,
        "hy_quasiquote_projection": hm.hy_quasiquote_projection_report,
        "hy_defmacro_projection": hm.hy_defmacro_projection_report,
        "hy_reader_macro_projection": hm.hy_reader_macro_projection_report,
        "hy_import_projection": hm.hy_import_projection_report,
        "hy_meta_host_api": hm.hy_meta_host_api_report,
        "pnix_form_projection": pm.pnix_form_projection_report,
        "correspondence": pm.correspondence_report,
        "alignment": pm.alignment_report,
        "alignment_tree": pm.alignment_tree_report,
        "hy_pnix_projection": pm.hy_pnix_projection_report,
        "pnix_to_hy_form": pm.pnix_to_hy_form_report,
        "pnix_meta_circular_projection": pm.pnix_meta_circular_projection_report,
        "compiler_emit_shape": pm.compiler_emit_shape_report,
        "projection_value_roundtrip": pm.projection_value_roundtrip_report,
        "synthesize_pnix_from_hy": pm.synthesize_pnix_from_hy_report,
        "hy_to_pnix_value_roundtrip": pm.hy_to_pnix_value_roundtrip_report,
        "pnix_projection_closure": pm.pnix_projection_closure_report,
        "hy_projection_closure": pm.hy_projection_closure_report,
        "meta_circular_tower": pm.meta_circular_tower_report,
        "safe_eval": pm.safe_eval_report,
        "reify_pnix": pm.reify_pnix_report,
        "roundtrip_status": pm.roundtrip_status_report,
        "explain_pnix": pm.explain_pnix_report,
        "static_purity_check": pm.static_purity_check_report,
        "cached_eval": pm.cached_eval_report,
        "performance_report": pm.performance_report_check,
        "fixture_report": _fixture_report,
        "diagnose": pm.diagnose_report,
        "eval_receipt": pm.eval_receipt_report,
        "hy_module_projection": hm.hy_module_projection_report,
        "project_hy_module": pm.project_hy_module_report,
        "specialize": pm.specialize_report,
        "execution_trace": hm.execution_trace_report,
        "pnix_evaluation_trace": pm.pnix_evaluation_trace_report,
        "interop": _interop_report,
        "interop_roundtrip": _interop_roundtrip_report,
        "interop_host_bridge": _interop_host_bridge_report,
        "interop_hy_macro_bridge": pm.hy_macro_quasiquote_over_pnix_report,
        "interop_no_mirror": _interop_no_mirror_report,
        "interop_error_contract": _interop_error_contract_report,
        "interop_opaque_lifecycle": _interop_opaque_lifecycle_report,
        "classify_drift": pm.classify_drift_report,
        "reify_hy": pm.reify_hy_report,
        "correspondence_abi": pm.correspondence_abi_report,
        "hy_reader_embed_pnix": pm.hy_reader_embed_pnix_report,
        "repl": _repl_report,
        "action": _action_report,
        "pnix_import_hook": _pnix_import_hook_report,
        "mirror_run": _mirror_run_report,
        "pnix_stage_ladder": _stage_ladder_report,
        "gate_check": _gate_report,
        "ir": _ir_report,
        "docs_drift": _docs_drift_report,
        "jones_optimality": _jones_optimality_report,
        "hygiene": _hygiene_report,
        "ir_diff": _ir_diff_report,
        "check_cache": _check_cache_report,
        "interop_hardening": _interop_hardening_report,
        "compartment": _compartment_report,
        "phase_separation": _phase_separation_report,
        "incremental_eval": _incremental_eval_report,
        "typed_witness": _typed_witness_report,
        "pe_annotations": _pe_annotations_report,
        "tower_ladder": _tower_ladder_report,
        "compiled_runtime": _compiled_runtime_report,
        "compiled_differential": _compiled_differential_report,
        "evaluate": _evaluate_report,
        "cogen": _cogen_report,
        "pe_size": _pe_size_report,
    }


def _cogen_report() -> dict:
    from pnix_hy import cogen_report  # noqa: PLC0415 - exported fn (module `cogen` is shadowed)
    return cogen_report()


def _pe_size_report() -> dict:
    from pnix_hy import pe_size_report  # noqa: PLC0415
    return pe_size_report()


def _evaluate_report() -> dict:
    from pnix_hy import compiled as _c  # noqa: PLC0415
    return _c.evaluate_report()


def _compiled_runtime_report() -> dict:
    from pnix_hy import compiled as _c  # noqa: PLC0415
    return _c.compiled_runtime_report()


def _compiled_differential_report() -> dict:
    from pnix_hy import compiled as _c  # noqa: PLC0415
    return _c.compiled_differential_report()


def _pe_annotations_report() -> dict:
    return pm.pe_annotations_report()


def _tower_ladder_report() -> dict:
    from pnix_hy import tower as tw  # noqa: PLC0415
    return tw.tower_ladder_report()


def _interop_hardening_report() -> dict:
    from pnix_hy import interop as ip  # noqa: PLC0415
    return ip.interop_hardening_report()


def _compartment_report() -> dict:
    from pnix_hy import compartment as cp  # noqa: PLC0415
    return cp.compartment_report()


def _phase_separation_report() -> dict:
    from pnix_hy import phase as phz  # noqa: PLC0415
    return phz.phase_separation_report()


def _incremental_eval_report() -> dict:
    from pnix_hy import incremental as inc  # noqa: PLC0415
    return inc.incremental_eval_report()


def _typed_witness_report() -> dict:
    from pnix_hy import gate as gt  # noqa: PLC0415
    return gt.typed_witness_report()


def _jones_optimality_report() -> dict:
    return pm.jones_optimality_report()


def _hygiene_report() -> dict:
    return pm.hygiene_report()


def _ir_diff_report() -> dict:
    from pnix_hy import ir as ir_mod  # noqa: PLC0415
    return ir_mod.ir_diff_report()


def _check_cache_report() -> dict:
    from pnix_hy import check_cache as cc  # noqa: PLC0415
    return cc.check_cache_report()


def _docs_drift_report() -> dict:
    from pnix_hy import capabilities as cap  # noqa: PLC0415
    return cap.docs_drift_report()


def _gate_report() -> dict:
    from pnix_hy import gate as gt  # noqa: PLC0415
    return gt.gate_report()


def _ir_report() -> dict:
    from pnix_hy import ir as ir_mod  # noqa: PLC0415
    return ir_mod.ir_report()


def _fixture_report() -> dict:
    from pnix_hy import pnix_runtime as rt  # noqa: PLC0415
    return rt.fixture_report()


def _interop_report() -> dict:
    from pnix_hy import interop as ip  # noqa: PLC0415
    return ip.interop_report()


def _interop_roundtrip_report() -> dict:
    from pnix_hy import interop as ip  # noqa: PLC0415
    return ip.roundtrip_report()


def _interop_host_bridge_report() -> dict:
    from pnix_hy import interop as ip  # noqa: PLC0415
    return ip.host_bridge_report()


def _interop_no_mirror_report() -> dict:
    from pnix_hy import interop as ip  # noqa: PLC0415
    return ip.no_mirror_report()


def _interop_error_contract_report() -> dict:
    from pnix_hy import interop as ip  # noqa: PLC0415
    return ip.error_contract_report()


def _interop_opaque_lifecycle_report() -> dict:
    from pnix_hy import interop as ip  # noqa: PLC0415
    return ip.opaque_lifecycle_report()


def _action_report() -> dict:
    from pnix_hy import action as act  # noqa: PLC0415
    return act.action_report()


def _pnix_import_hook_report() -> dict:
    from pnix_hy import interop as ip  # noqa: PLC0415
    return ip.pnix_import_hook_report()


def _repl_report() -> dict:
    from pnix_hy import repl as rp  # noqa: PLC0415
    return rp.repl_report()


def _mirror_run_report() -> dict:
    from pnix_hy import mirror as mr  # noqa: PLC0415
    return mr.mirror_run_report()


def _stage_ladder_report() -> dict:
    from pnix_hy import stage as st  # noqa: PLC0415
    return st.pnix_stage_ladder_report()


def _safe_report(fn) -> dict:
    """Run a report/lane, converting ANY exception (incl. subprocess TimeoutExpired) into a FAIL
    row -- so one slow/raising report can never abort the whole --check/--gate audit."""
    try:
        return fn()
    except Exception as exc:  # noqa: BLE001
        return {"ready": False, "available": False, "error": f"{type(exc).__name__}: {exc}"}


def cmd_check(as_json: bool, cached: bool = False) -> int:
    reports = _toolkit_reports()
    if cached:  # 0019: verifying-trace cache, opt-in; failures are never cached
        from pnix_hy import check_cache as cc  # noqa: PLC0415
        state = cc.package_state_hash()
        results = {name: cc.cached_run(name, (lambda fn=fn: _safe_report(fn)), state_hash=state)
                   for name, fn in reports.items()}
    else:
        results = {name: _safe_report(fn) for name, fn in reports.items()}
    all_ready = all(r.get("ready") for r in results.values())
    n_cached = sum(1 for r in results.values() if r.get("cached"))
    if as_json:
        print(json.dumps({"all_ready": all_ready, "cached": n_cached, "reports": results},
                         indent=2, ensure_ascii=False))
    else:
        for name, r in results.items():
            mark = "OK " if r.get("ready") else "FAIL"
            suffix = "  (cached)" if r.get("cached") else ""
            print(f"  [{mark}] {name}{suffix}")
        extra = f"  ({n_cached} cached)" if cached else ""
        print(f"\nall_ready: {all_ready}{extra}")
    return 0 if all_ready else 1


def cmd_gate(as_json: bool) -> int:
    """The full production ship-gate: the foundational SACRED gates (runtime self-test,
    rust corpus, the 4-lane mirror + closure) PLUS every toolkit self-check, as one
    pass/fail. Heavyweight (builds the stage7 kernel for the mirror)."""
    from pnix_hy import pnix_runtime as rt  # noqa: PLC0415

    gates: dict[str, dict] = {}

    st = _safe_report(rt.self_test_report)  # sacred lane isolated: a crash -> FAIL row, not abort
    st_pass = sum(1 for c in st.get("cases", []) if c.get("ok"))
    gates["runtime_self_test"] = {"ok": bool(st.get("ready")) and st_pass == len(st.get("cases", [])),
                                  "detail": f"{st_pass}/{len(st.get('cases', []))}"}

    rc = _safe_report(rt.rust_corpus_report)
    gates["rust_corpus"] = {"ok": bool(rc.get("ready")),
                            "detail": f"{rc.get('agree')}/{rc.get('count')}"}

    mirror = _safe_report(lambda: pm.self_test_report(include_hy_host=True))
    lane_ok = True
    lane_detail = {}
    for lane in ("runtime_parity", "source_parity", "compiler_parity", "compiler_source_parity"):
        s = mirror.get(lane) or {}
        lane_ok = lane_ok and bool(s.get("ready"))
        lane_detail[lane] = s.get("count")
    closure = (mirror.get("closure") or {}).get("status") or {}
    closure_ok = bool((mirror.get("closure") or {}).get("ready"))
    gates["four_lane_mirror"] = {"ok": bool(mirror.get("ready")) and lane_ok,
                                 "detail": lane_detail}
    gates["closure"] = {"ok": closure_ok, "detail": closure}

    toolkit = {name: _safe_report(fn) for name, fn in _toolkit_reports().items()}
    toolkit_ok = all(r.get("ready") for r in toolkit.values())
    gates["toolkit_self_checks"] = {"ok": toolkit_ok,
                                    "detail": f"{sum(1 for r in toolkit.values() if r.get('ready'))}/{len(toolkit)}"}

    passed = all(g["ok"] for g in gates.values())
    if as_json:
        print(json.dumps({"passed": passed, "gates": gates}, indent=2, ensure_ascii=False))
    else:
        print("== pnix-hy production ship-gate ==")
        for name, g in gates.items():
            print(f"  [{'PASS' if g['ok'] else 'FAIL'}] {name}: {g['detail']}")
        print(f"\nGATE: {'PASS' if passed else 'FAIL'}")
    return 0 if passed else 1


def main(argv: list[str] | None = None) -> int:
    if argv is None:  # console_scripts entry point calls main() with no args
        argv = sys.argv[1:]
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    g = parser.add_mutually_exclusive_group(required=True)
    g.add_argument("--hy", metavar="SRC", help="project a Hy fragment (Hy -> pnix)")
    g.add_argument("--hy-file", metavar="PATH", help="project a Hy file (single fragment)")
    g.add_argument("--hy-module", metavar="PATH", help="project EVERY top-level form of a .hy file (per-form breakdown)")
    g.add_argument("--macro-steps", metavar="SRC", help="trace the macro tower unfolding (macroexpand_1 to fixpoint) for every form")
    g.add_argument("--quasiquote", metavar="SRC", help="project a quoted/quasiquoted form as code-as-data (holes + staging view)")
    g.add_argument("--defmacro", metavar="SRC", help="project user macro definitions (transformer + registration + form->form rewrite)")
    g.add_argument("--reader-macro", metavar="SRC", help="project read-time reader macros (defreader + #name expansion); completes the staging tower")
    g.add_argument("--import", metavar="SRC", dest="import_", help="project import/require forms (run-time module vs compile-time macro scope)")
    g.add_argument("--pnix", metavar="SRC", help="project a pnix fragment (pnix + reverse Hy)")
    g.add_argument("--pnix-file", metavar="PATH", help="project a pnix file")
    g.add_argument("--pnix-meta", metavar="SRC", help="evaluate a pnix form on all four meta-circular substrates (slow)")
    g.add_argument("--roundtrip", metavar="SRC", help="check meaning is preserved pnix -> synthesized Hy -> value (slow)")
    g.add_argument("--synth-pnix", metavar="SRC", help="synthesize pnix source from a Hy fragment (reverse of --pnix's Hy synth)")
    g.add_argument("--hy-roundtrip", metavar="SRC", help="check meaning is preserved Hy -> synthesized pnix -> value")
    g.add_argument("--closure", metavar="SRC", help="involution check: pnix -> Hy -> pnix preserves value")
    g.add_argument("--hy-closure", metavar="SRC", help="involution check: Hy -> pnix -> Hy preserves value")
    g.add_argument("--tower", metavar="SRC", help="the whole meta-circular journey of a Hy fragment: read->compile->run->pnix->collapse")
    g.add_argument("--safe-eval", metavar="SRC", help="evaluate untrusted pnix under resource bounds (timeout/steps/output cap)")
    g.add_argument("--purity", metavar="SRC", help="static purity check: does this pnix touch filesystem/env/import?")
    g.add_argument("--diagnose", metavar="SRC", help="structured eval error with source position (line/col + caret excerpt)")
    g.add_argument("--receipt", metavar="SRC", help="reproducibility/audit receipt: hashes + value + 4-lane convergence + trace")
    g.add_argument("--interop", metavar="SRC", help="convert a pnix value to the host across the explicit interop boundary (loss/effect/capability record)")
    g.add_argument("--mirror", metavar="SRC", help="run the SINGLETON pnix runtime mirror (one run, faceted trace + witness)")
    g.add_argument("--stage-ladder", metavar="SRC", help="run the pnix runtime stage ladder (direct/normalized/store/roundtrip/mirror/replay/closure)")
    g.add_argument("--perf", metavar="SRC", help="benchmark pnix runtime lanes (parse/emit/compile/interp/compiler exec-many)")
    g.add_argument("--gate-check", metavar="SRC", help="capability gate: admit only if required effects are granted ('SRC ;; file-read,import')")
    g.add_argument("--ir", metavar="SRC", help="lower pnix to its canonical IR (normalized AST) + content hash + evaluability proof")
    g.add_argument("--reify", metavar="SRC", help="reify pnix as source/form/AST/IR/value/effect/witness")
    g.add_argument("--roundtrip-status", metavar="SRC", help="classify pnix roundtrips with the shared status vocabulary")
    g.add_argument("--explain", metavar="SRC", help="unified pnix explain surface ('SRC ;; file-read,import' for granted caps)")
    g.add_argument("--action-check", metavar="SRC", help="check one pnix action step ('SRC ;; file-read,import' for grants)")
    g.add_argument("--action-explain", metavar="SRC", help="verify one pnix action step with hash refs + witness")
    g.add_argument("--specialize", metavar="SRC", help="partial-evaluate a pnix program (Futamura 1st projection); mark dynamic vars as 'SRC ;; x,y'")
    g.add_argument("--trace", metavar="SRC", help="live meta-circular trace: the Python bytecode the pnix evaluator runs for SRC")
    g.add_argument("--hy-trace", metavar="SRC", help="live execution trace: opcodes a Python/Hy-lowered expression actually runs")
    g.add_argument("--correspondence", action="store_true", help="print the correspondence table")
    g.add_argument("--check", action="store_true", help="run every toolkit self-check")
    g.add_argument("--gate", action="store_true", help="full production ship-gate: sacred gates (runtime/corpus/4-lane mirror/closure) + toolkit self-checks")
    g.add_argument("--repl", metavar="MODE", choices=["python", "hy", "pnix"], help="start a context-retaining REPL: python | hy | pnix (warm process)")
    g.add_argument("--deployment", action="store_true", help="report install location + which capability tiers (core/projection/full) work here")
    g.add_argument("--capabilities", action="store_true", help="code-derived capability index (public API + reports + proposals); use before building to avoid duplicate work")
    g.add_argument("--futamura", action="store_true", help="run the whole Futamura ladder (1st/2nd/3rd projection) as one inspectable artifact")
    g.add_argument("--compiled-bench", action="store_true", help="benchmark the compiled core-subset runtime vs the tree-walker")
    g.add_argument("--ceval", metavar="SRC", help="evaluate pnix via the fast compiled runtime (auto-fallback to canonical for out-of-subset)")
    parser.add_argument("--json", action="store_true", help="emit raw JSON")
    parser.add_argument("--cached", action="store_true",
                        help="with --check: replay reports whose inputs' content hashes are unchanged (0019; failures always re-run)")
    args = parser.parse_args(argv)

    if args.check:
        return cmd_check(args.json, cached=args.cached)
    if args.gate:
        return cmd_gate(args.json)
    if args.repl is not None:
        from pnix_hy import repl as _repl  # noqa: PLC0415
        return _repl.run_repl(args.repl)
    if getattr(args, "ceval", None) is not None:
        from pnix_hy import compiled as _c  # noqa: PLC0415
        res = _c.evaluate(args.ceval)
        if args.json:
            print(json.dumps(res, indent=2, ensure_ascii=False))
        else:
            print(f"value  : {res['value']!r}")
            print(f"backend: {res['backend']}  (compiled = fast core-subset path; canonical = pnix_runtime)")
        return 0
    if getattr(args, "compiled_bench", False):
        import time as _time  # noqa: PLC0415
        from pnix_hy import compiled as _c  # noqa: PLC0415
        b = _c.compiled_bench(_time.perf_counter)
        if args.json:
            print(json.dumps(b, indent=2, ensure_ascii=False))
        else:
            print("== compiled runtime vs tree-walker (core subset) ==")
            for r in b["rows"]:
                print(f"  {r['case']:20} tree={r['tree_walker_s']:8.3f}s  "
                      f"compiled={r['compiled_s']:8.3f}s  speedup={r['speedup']}x  agree={r['agree']}")
            print(f"  all_agree: {b['all_agree']}  (canonical evaluator = pnix_runtime; compiled is a contrast lane)")
        return 0
    if args.futamura:
        from pnix_hy import tower as _tower  # noqa: PLC0415
        result = _tower.futamura_ladder()
        if args.json:
            print(json.dumps(result, indent=2, ensure_ascii=False))
        else:
            f, s, th = result["first_projection"], result["second_projection"], result["third_projection"]
            print("== Futamura ladder (pnix) ==")
            print(f"  1st projection  interpreter specialized to a program:")
            print(f"     residual = {f['residual']}   (interpreter-free: {f['interpreter_free']})")
            print(f"  2nd projection  specializer specialized to the interpreter = a compiler:")
            print(f"     {s['compiler_spec_points']} spec points, {s['compiler_size']}B; compiler(prog) = {s['target']}")
            print(f"  3rd projection  self-applied specializer = cogen ({th['cogen_size']}B), executed:")
            print(f"     cogen({th['cogen_run_input']}) = {th['cogen_run_residual']}")
            print(f"  note: {result['note']}")
        return 0
    if args.capabilities:
        from pnix_hy import capabilities as _cap  # noqa: PLC0415
        if args.json:
            print(json.dumps(_cap.capability_index(), indent=2, ensure_ascii=False))
        else:
            sys.stdout.write(_cap.render_capabilities_md())
        return 0
    if args.deployment:
        from pnix_hy import deploy as _deploy  # noqa: PLC0415
        info = _deploy.deployment_info()
        if args.json:
            print(json.dumps(info, indent=2, ensure_ascii=False))
        else:
            print("== pnix-hy deployment ==")
            print(f"  package    : {info['package_path']}")
            print(f"  HY_ROOT    : {info['hy_root']}  (PNIX_HY_HOME={info['pnix_hy_home_env']})")
            print(f"  hy-meta    : {'found' if info['hy_meta_found'] else 'MISSING'}"
                  f"  | vendored hy: {'found' if info['vendored_hy_found'] else 'MISSING'}"
                  f"  | proof python: {info['proof_python'] or 'MISSING'}")
            print(f"  tiers      : core={info['tiers']['core']}  "
                  f"projection={info['tiers']['projection']}  full_gate={info['tiers']['full_gate']}")
            if info["hint"]:
                print(f"  hint       : {info['hint']}")
        return 0
    if args.correspondence:
        cmd_correspondence(args.json)
        return 0
    if args.hy is not None:
        cmd_hy(args.hy, args.json)
        return 0
    if args.hy_file is not None:
        with open(args.hy_file, encoding="utf-8") as fh:
            cmd_hy(fh.read(), args.json)
        return 0
    if args.hy_module is not None:
        with open(args.hy_module, encoding="utf-8") as fh:
            cmd_hy_module(fh.read(), args.json)
        return 0
    if args.macro_steps is not None:
        cmd_macro_steps(args.macro_steps, args.json)
        return 0
    if args.quasiquote is not None:
        cmd_quasiquote(args.quasiquote, args.json)
        return 0
    if args.defmacro is not None:
        cmd_defmacro(args.defmacro, args.json)
        return 0
    if args.reader_macro is not None:
        cmd_reader_macro(args.reader_macro, args.json)
        return 0
    if args.import_ is not None:
        cmd_import(args.import_, args.json)
        return 0
    if args.pnix is not None:
        cmd_pnix(args.pnix, args.json)
        return 0
    if args.pnix_file is not None:
        cmd_pnix_file(args.pnix_file, args.json)
        return 0
    if args.pnix_meta is not None:
        cmd_pnix_meta(args.pnix_meta, args.json)
        return 0
    if args.roundtrip is not None:
        cmd_roundtrip(args.roundtrip, args.json)
        return 0
    if args.synth_pnix is not None:
        cmd_synth_pnix(args.synth_pnix, args.json)
        return 0
    if args.hy_roundtrip is not None:
        cmd_hy_roundtrip(args.hy_roundtrip, args.json)
        return 0
    if args.closure is not None:
        cmd_closure(args.closure, args.json)
        return 0
    if args.hy_closure is not None:
        cmd_hy_closure(args.hy_closure, args.json)
        return 0
    if args.tower is not None:
        cmd_tower(args.tower, args.json)
        return 0
    if args.safe_eval is not None:
        cmd_safe_eval(args.safe_eval, args.json)
        return 0
    if args.action_check is not None:
        cmd_action_check(args.action_check, args.json)
        return 0
    if args.action_explain is not None:
        cmd_action_explain(args.action_explain, args.json)
        return 0
    if args.purity is not None:
        cmd_purity(args.purity, args.json)
        return 0
    if args.diagnose is not None:
        cmd_diagnose(args.diagnose, args.json)
        return 0
    if args.receipt is not None:
        cmd_receipt(args.receipt, args.json)
        return 0
    if args.interop is not None:
        cmd_interop(args.interop, args.json)
        return 0
    if args.mirror is not None:
        cmd_mirror(args.mirror, args.json)
        return 0
    if args.stage_ladder is not None:
        cmd_stage_ladder(args.stage_ladder, args.json)
        return 0
    if args.perf is not None:
        cmd_perf(args.perf, args.json)
        return 0
    if args.gate_check is not None:
        cmd_gate_check(args.gate_check, args.json)
        return 0
    if args.ir is not None:
        cmd_ir(args.ir, args.json)
        return 0
    if args.reify is not None:
        cmd_reify(args.reify, args.json)
        return 0
    if args.roundtrip_status is not None:
        cmd_roundtrip_status(args.roundtrip_status, args.json)
        return 0
    if args.explain is not None:
        cmd_explain(args.explain, args.json)
        return 0
    if args.specialize is not None:
        cmd_specialize(args.specialize, args.json)
        return 0
    if args.trace is not None:
        cmd_trace(args.trace, args.json)
        return 0
    if args.hy_trace is not None:
        cmd_hy_trace(args.hy_trace, args.json)
        return 0
    return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
