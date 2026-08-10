"""pnix-hy의 방식 — cached_eval: 정본(canonical) 내용주소 캐시.

cached_eval은 소스를 정본 형태로 정규화한 키로 메모이즈한다. 공백/포맷이 달라도 '같은 의미'면
같은 캐시 항목에 적중한다 — 표현이 아니라 내용이 키다. 순수 — Hy 불필요.
"""
import os
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph  # noqa: E402


first = ph.cached_eval("1 + 2")
again = ph.cached_eval("1 +  2")   # 공백만 다름 -> 같은 정본 -> 캐시 적중
print("첫 계산 cached?:", first["cached"], "| 값:", first["value"])
print("포맷만 다른 재요청 cached?:", again["cached"], "| 값:", again["value"])
print("cache_key(정본):", first["cache_key"][:16], "==", again["cache_key"][:16], "?",
      first["cache_key"] == again["cache_key"])

assert first["cached"] is False and again["cached"] is True   # 두 번째는 캐시 적중
assert first["value"] == again["value"] == 3
assert first["cache_key"] == again["cache_key"]               # 같은 정본 = 같은 키

print("\n결론: 내용주소(정본) 캐시라, 같은 의미의 다른 표현도 재사용한다 -> 재현/증분에 유리.")
