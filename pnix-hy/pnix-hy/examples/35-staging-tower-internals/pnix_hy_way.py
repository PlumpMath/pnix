"""pnix-hy 방식: staging tower 내부 기계 (proposal 0026 M2/M3).

pnix는 평가를 데이터로 다루는 기계들을 노출한다:
  · CEK 기계 — 계산을 (control, env, kont) 상태로 두고 한 스텝씩; 중간에 멈춰 **reify**(해시+증거)하고
    나중에 **resume**한다.
  · stage-polymorphic mini 평가기 — 같은 소스를 interpret(값) 또는 compile(잔여 코드)로.
  · offline BTA — 특화 전에 각 부분을 정적(S)/동적(D)으로 분류.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph
import pnix_hy.tower as tw

# (1) CEK: 도중에 멈추고 → reify(해시·witness) → 재개, 값은 통째 실행과 동일
paused = tw.cek_run("(2 + 3) * 4", pause_at=4)
reified = paused["reified"]                      # 상태를 데이터로 꺼냄
print("CEK pause@4:", paused["status"], "| reified sha256/witness 있음:",
      bool(reified["state_sha256"]) and bool(reified["witness_id"]))
resumed = tw.cek_resume(reified)                 # 그 지점부터 재개
full = tw.cek_run("(2 + 3) * 4")
assert paused["status"] == "paused"
assert resumed["value"] == full["value"] == 20   # 멈췄다 재개해도 같은 값
print("CEK resume →", resumed["value"], "(= 통째 실행", full["value"], ")")

# (2) stage-polymorphic: 같은 소스, 두 역할
val = tw.stage_poly_interpret("(input + 1) * 3", {"input": 5})   # 값
res = tw.stage_poly_compile("(input + 1) * 3", ("input",))        # 잔여 코드
print("stage-poly: interpret(input=5) =", val, "| compile =", res)
assert val == 18 and res == "((input + 1) * 3)"

# (3) offline BTA: 특화 전에 정적/동적 분류
bta = tw.binding_time_analysis("a * x + b", ("x",))
print("BTA(a*x+b, x dynamic): division =", bta["division"], "| counts =", bta["counts"])
assert bta["division"]["x"] == "D"               # x는 동적

print("→ 평가를 데이터로: 멈춤/reify/재개 + interpret/compile 겸용 + 정적/동적 분류.")
