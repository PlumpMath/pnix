"""한계: 형식 없는 witness/로그는 무엇에 대한 것인지, 유효한지 구분이 없다.

plain하게 증거(witness)를 dict/로그로 남기면
- 이게 eval-증거인지 compile-증거인지 payload만 봐선 알 수 없고,
- 어떤 필드가 필수인지 스키마가 없어 오타/누락이 조용히 통과하며,
- 예전 형식(deprecated)인지 최신인지 판별할 수단이 없다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))

# 형식 없는 witness — 아무 payload나 들어간다
w1 = {"kind": "eval", "in": "h1", "out": "h2", "status": "ok"}
w2 = {"kind": "eval", "inn": "h1"}                # 오타(inn)·필드 누락 — 아무도 안 잡음
print("형식 없는 witness:", w1)
print("오타/누락도 통과:", w2)
# 이게 어떤 predicate에 대한 증거인가? — 명시가 없다
print("predicate:", w1.get("predicate", "<없음>"))
print("한계: 무엇에 대한 증거인지·유효한지·최신 형식인지 판별 불가.")
