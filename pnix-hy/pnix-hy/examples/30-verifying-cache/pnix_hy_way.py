"""pnix-hy 방식: 상태-해시로 키를 매기는 verifying-trace 캐시 (proposal 0019).

`package_state_hash()`는 패키지 소스 상태의 해시. `cached_run(name, fn, state_hash=)`은
- **같은 상태**면 검사를 다시 돌리지 않고 캐시된 결과를 replay하며 `cached=True` 마커를 남기고,
- **상태가 바뀌면**(해시가 다르면) 자동으로 재계산한다.
캐시는 디스크에 영속하므로, 계약은 '같은 상태→replay / 바뀐 상태→recompute'로 관찰한다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy.check_cache as cc

runs = {"n": 0}
def check():
    runs["n"] += 1
    return {"ready": True, "available": True}

h = cc.package_state_hash()
print("package_state_hash:", h[:16], "…")

r1 = cc.cached_run("ex30", check, state_hash=h)              # 이 상태로 채움
r2 = cc.cached_run("ex30", check, state_hash=h)             # 같은 상태 → replay
r3 = cc.cached_run("ex30", check, state_hash="ex30-other")  # 다른 상태 → recompute

print(f"same-state replay: cached={r2.get('cached')} | changed-state: cached={r3.get('cached')}")
assert r2.get("cached") is True            # 같은 상태 → 재사용(verifying trace 마커)
assert r3.get("cached") is not True        # 상태 바뀜 → 재계산(자동 무효화)
assert runs["n"] >= 1                       # r2는 계산 안 함(replay), r1·r3는 계산

assert cc.check_cache_report()["ready"]
print("→ 같은 상태는 replay(cached=True), 바뀐 상태는 자동 재계산 — 검사에 상태-해시 키.")
