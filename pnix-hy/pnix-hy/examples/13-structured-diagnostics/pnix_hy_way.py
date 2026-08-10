"""pnix-hy의 방식 — diagnose: 위치·단계·캐럿이 담긴 구조화 진단(데이터).

diagnose는 잘못된 pnix 소스에 대해 {ok, line, column, offset, phase, message, excerpt(캐럿)}를
'데이터로' 돌려준다 — 예외로 새지 않고, DSL 사용자에게 그대로 보여줄 수 있다. 순수 — Hy 불필요.
"""
import os
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph  # noqa: E402


d = ph.diagnose("let a = ")     # 미완성 소스
print("ok:", d["ok"], "| line:", d["line"], "| column:", d["column"], "| phase:", d["phase"])
print("message:", d["message"])
print("excerpt(캐럿):")
print(d["excerpt"])

assert d["ok"] is False
assert d["line"] == 1 and isinstance(d["offset"], int)
assert "^" in d["excerpt"]        # 문제 위치를 캐럿으로 가리킨다

# 정상 소스는 ok=True.
good = ph.diagnose("let a = 1; in a + 1")
print("\n정상 소스 ok:", good["ok"])
assert good["ok"] is True

print("\n결론: 위치·단계·캐럿이 담긴 구조화 진단을 데이터로 반환 -> DSL UX/툴링에 바로 쓴다.")
