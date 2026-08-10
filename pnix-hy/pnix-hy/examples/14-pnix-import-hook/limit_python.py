"""plain의 한계 — 다른 언어 파일을 모듈로 import할 수 없고, sys.modules 트랜잭션도 없다.

  1) Python importlib는 .py만 안다 — .px(pnix) 파일을 import하려면 직접 로더를 짜야 한다.
  2) import 도중 실패하면 sys.modules 등 전역 상태가 부분적으로 오염될 수 있고,
     '스냅샷 -> 시도 -> 실패 시 롤백'하는 트랜잭션 프리미티브가 표준에 없다.
"""
import importlib

# 1) .px는 import 대상이 아니다.
try:
    importlib.import_module("some_pnix_module")   # .px 로더가 없다
except ModuleNotFoundError as e:
    print("import .px:", type(e).__name__, "(Python importlib는 .px를 모른다)")

# 2) sys.modules 트랜잭션(스냅샷/롤백) 프리미티브: 표준에 없다.
print("sys.modules 스냅샷/롤백 트랜잭션?: 표준 없음 (직접 관리해야 한다)")

print("\n결론: 다른-언어 모듈 로딩 + 실패-시-롤백 트랜잭션이 기본 제공되지 않는다.")
