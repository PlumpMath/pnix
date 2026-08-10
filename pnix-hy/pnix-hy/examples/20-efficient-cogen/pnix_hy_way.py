"""pnix-hy 방식: efficient cogen = "cogen approach" (proposal 0029).

딥리서치가 확정한 정답(Birkedal&Welinder / Thiemann "Cogen in Six Lines" / Glück&Jørgensen / Leuschel):
cogen을 self-application으로 만들지 말고, 컴파일러 생성기를 **BTA/특화기 위의 얇은 층으로 hand-write**한다
(인터프리터를 끌고 들어가지 않으므로 비대해지지 않는다). pnix는 그 hand-written 생성기를 이미 가짐:
native 폴리variant 특화기 `tower.poly_specialize`. `pnix_hy.cogen`이 이를 cogen API로 노출한다.

핵심: 같은 3차 사영을, self-application(비대·>150초) 대신 cogen approach(0.003초)로.
"""
import os
import sys
import time

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph  # noqa: E402
import pnix_hy.pnix_runtime as rt  # noqa: E402

# 작은 산술 인터프리터: prog는 정적(컴파일 시), input은 동적(런타임)
INT = ('let int = prog: env: if prog.tag == "num" then prog.value '
       'else if prog.tag == "arg" then env '
       'else if prog.tag == "add" then (int prog.l env) + (int prog.r env) '
       'else if prog.tag == "mul" then (int prog.l env) * (int prog.r env) '
       'else 0; in int prog input')

# (1) 인터프리터의 생성확장 = 컴파일러 (host 형태), 밀리초 단위
compiler = ph.compiler_from_interpreter(INT)
P = {"tag": "add", "l": {"tag": "mul", "l": {"tag": "arg"}, "r": {"tag": "num", "value": 3}},
     "r": {"tag": "num", "value": 4}}
t0 = time.perf_counter()
target = compiler(P)                      # 특화기 자기적용 없이, native 생성기로 컴파일
dt = time.perf_counter() - t0
print(f"cogen approach: compiler((x*3)+4) 생성 {dt:.3f}s (naive는 >150s)")
assert all(rt.eval_source(f"let input = {i}; in {target}") == i * 3 + 4 for i in (0, 5, 9))
print(f"  target = {target[:60]}...  parity OK")

# (2) 같은 생성확장을 STANDALONE PNIX 소스로도 (이식 가능한 pnix 컴파일러)
csrc = ph.compiler_source(INT)            # 순수 pnix 컴파일러 소스 (문자열)
tgt2 = ph.compile_with(csrc, P)           # 순수 pnix 평가로 컴파일
assert tgt2 == target or all(rt.eval_source(f"let input = {i}; in {tgt2}") == i * 3 + 4 for i in (0, 5, 9))
print(f"  standalone pnix 컴파일러: {len(csrc)}B 소스 → 순수 pnix 평가로 동일 target")

# (3) 일반 생성확장: 정적 파라미터를 baked-in 한 재사용 컴파일러
gex = ph.generating_extension("(a * x) + b", ("x",))
assert rt.eval_source("let x = 5; in " + gex({"a": 3, "b": 4})) == 19
print("  generating_extension((a*x)+b) with a=3,b=4 → ((3 * x) + 4)")
print("→ 3차 Futamura 사영을 self-application 없이 실용 속도로 (research-backed).")
