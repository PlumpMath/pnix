"""pnix-hy의 방식 — 정본 내용주소 해시 + 결정성 + drift 분류.

pnix는 소스를 정규화된 AST/IR로 낮추고, 그 IR에 안정적 sha256을 부여한다. 같은 소스는 항상
같은 해시(결정성)이고, 프로젝션 gap 같은 차이는 안정적 카테고리로 '분류(drift)'된다.
"""
import os
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph  # noqa: E402


# 1) 정본 IR 해시: 같은 소스 -> 항상 같은 sha256 (프로세스와 무관, 재현 가능).
src = "let a = 1; in a + 2"
h1 = ph.ir_of(src)["ir_sha256"]
h2 = ph.ir_of(src)["ir_sha256"]
print("IR 해시:", h1[:16], "...", "| 결정적:", h1 == h2, "| 길이:", len(h1))
assert h1 == h2 and len(h1) == 64

# 2) 공백/포맷이 달라도 '정규화 IR'이라 같은 정본으로 수렴한다.
h3 = ph.ir_of("let  a=1 ;  in   a+2")["ir_sha256"]
print("포맷만 다른 소스도 같은 정본 해시:", h1 == h3)
assert h1 == h3

# 3) drift 분류: pnix->Hy 프로젝션의 차이를 안정적 카테고리로 분류한다.
clean = ph.classify_drift("(x: x + 1)")           # 깨끗한 프로젝션
gapped = ph.classify_drift("with m; b")            # Hy에 직접 형태가 없는 구성
print("clean drift:", clean["drift_count"], "| gapped 분류:", gapped["categories"])
assert clean["drift_count"] == 0 and "no-projection-construct" in gapped["categories"]

print("\n결론: 재현 가능한 정본 해시 + 차이의 원인 분류로 감사/캐시/회귀검출이 가능하다.")
