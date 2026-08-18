"""pnix-hy 방식: 메타순환 타워 전체 여정 (`meta_circular_tower`).

read(reader-form) -> compile(Python+매크로 tower) -> run(추적 실행) ->
pnix(합성/보존/닫힘) -> collapse(특화 왕복)를 한 산출물로 묶는다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
from pnix_hy import hy_mirror as hm  # noqa: F401 - import order matters (circular init)
import pnix_hy as ph

r = ph.meta_circular_tower("(+ 1 2)")
print(f"read: reader_form_count={r['stages']['read']['reader_form_count']}")
print(f"compile: python={r['stages']['compile']['python_source']!r}")
print(f"run: result={r['stages']['run']['result']} traced={r['stages']['run']['traced']}")
print(f"pnix: synth={r['stages']['pnix']['synth_pnix']!r} value_preserved={r['stages']['pnix']['value_preserved']}")
print(f"collapse: {r['stages']['collapse']}")
assert r["stages"]["run"]["result"] == "3"
assert r["stages"]["pnix"]["value_preserved"] and r["stages"]["pnix"]["closed"]
print("→ 다섯 단계(read/compile/run/pnix/collapse) 전부 통과, 값(3) 유지.")
