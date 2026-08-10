"""pnix-hy 방식: hygiene 자가검사 — 포획 탐지 + fresh binder (proposal 0017).

`hygiene_report`는 (1) 심어둔 충돌(planted collision)에서 **capture를 실제로 탐지**하고, (2) fresh
binder(gensym)가 깨끗한지, (3) 매크로 확장이 도입한 심볼이 사용자 자유변수를 오염시키지 않는지 게이트한다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy.pnix_mirror as pm

r = pm.hygiene_report()
print(f"capture_detected={r['capture_detected']} fresh_binder_clean={r['fresh_binder_clean']} "
      f"macro_expansion_ok={r['macro_expansion_ok']}")
# 심어둔 충돌에서 capture가 '탐지'되어야 한다(탐지기가 작동한다는 증거)
assert r["ready"]
assert r["capture_detected"] is True          # planted collision → 탐지됨
assert r["fresh_binder_clean"] is True        # gensym binder는 깨끗
assert r["macro_expansion_ok"] is True
print("→ 포획을 탐지하고, fresh binder는 오염되지 않음 — hygiene 게이트.")
