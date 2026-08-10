"""pnix-hy 방식: Futamura 사다리 전체를 하나의 산출물로 (proposal 0026 M7, 0029).

`futamura_ladder()`는 하나의 인터프리터에서 세 사영을 전부 파생해 보여준다:
  1차: 인터프리터를 프로그램에 특화 → 해석 계층이 사라진 잔여(interpreter-free),
  2차: 특화기를 인터프리터에 특화 → 독립 컴파일러(compiler(prog) = target),
  3차: 특화기를 자기적용 → cogen(생성기), specializer로 실행.
CLI로는 `pnix-hy-project --futamura`.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph
import pnix_hy.pnix_runtime as rt

lad = ph.futamura_ladder()
f, s, t = lad["first_projection"], lad["second_projection"], lad["third_projection"]

# 1차: 인터프리터 붕괴 → 잔여가 interpreter-free
print("1차 사영:", f["residual"], "| interpreter-free:", f["interpreter_free"])
assert f["residual"] == "((input * 3) + 4)" and f["interpreter_free"] is True

# 2차: 특화기 → 컴파일러, 그 컴파일러가 프로그램을 target으로
print(f"2차 사영: {s['compiler_spec_points']} spec-point 컴파일러 → compiler(prog) = {s['target']}")
assert s["target"] == "((input * 3) + 4)"
# 파생된 target이 실제로 맞나
assert all(rt.eval_source(f"let input = {i}; in {s['target']}") == i * 3 + 4 for i in (0, 5, 9))

# 3차: cogen(자기적용) → specializer로 실행
print(f"3차 사영: cogen({t['cogen_run_input']}) = {t['cogen_run_residual']}")
assert t["cogen_run_residual"] == "(6 * b)"

# 효율적 3차(0029): 인터프리터로부터 컴파일러를 파생(hand-written cogen approach)
compiler = ph.compiler_from_interpreter(
    'let int = prog: env: if prog.tag == "num" then prog.value '
    'else if prog.tag == "arg" then env '
    'else if prog.tag == "add" then (int prog.l env) + (int prog.r env) else 0; in int prog input')
tgt = compiler({"tag": "add", "l": {"tag": "arg"}, "r": {"tag": "num", "value": 5}})
assert all(rt.eval_source(f"let input = {i}; in {tgt}") == i + 5 for i in (0, 7))
print("cogen approach: compiler(x+5) →", tgt[:48], "…  (인터프리터에서 파생)")
print("→ 하나의 인터프리터에서 해석·컴파일·생성이 파생된다 — Futamura 사다리.")
