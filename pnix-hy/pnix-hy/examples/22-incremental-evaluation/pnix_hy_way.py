"""pnix-hy 방식: 의미(content) 기반 정체성 — 정의별 증분 평가 + realisation early cutoff (proposal 0023).

`incremental_eval`은 top-level `let` 정의마다 **의존성-치환 content hash**를 매긴다(형제 참조를
그 정의의 hash로 치환한 뒤 해시 → 이름은 메타데이터). 그래서
- 안 바뀐 정의는 재사용, 바뀐 정의(+의존자)만 재계산,
- alpha-rename(이름만 교체)에도 전부 hit(정체성은 의미이므로).
`realisation_record`는 Nix-CA 유사 store: 같은 IR 해시는 평가 없이 결과를 증명(early cutoff).
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph
import pnix_hy.incremental as inc

inc.clear_incremental_cache()

src = "let big = 1000 * 1000; dep = big + 1; other = 7; in dep + other"
cold = inc.incremental_eval(src)          # 3 정의 전부 처음 → misses=3
warm = inc.incremental_eval(src)          # 전부 재사용 → hits=3
print(f"cold: value={cold['value']} misses={cold['misses']} hits={cold['hits']}")
print(f"warm: value={warm['value']} misses={warm['misses']} hits={warm['hits']}")
assert cold["misses"] == 3 and warm["hits"] == 3 and warm["misses"] == 0

# other 하나만 7->8: dep/big은 그대로 재사용, other만 재계산
changed = inc.incremental_eval("let big = 1000 * 1000; dep = big + 1; other = 8; in dep + other")
print(f"1개 변경: hits={changed['hits']} misses={changed['misses']} value={changed['value']}")
assert changed["hits"] == 2 and changed["misses"] == 1

# alpha-rename: big -> huge (의미 동일) → 전부 hit (이름은 메타데이터)
renamed = inc.incremental_eval("let huge = 1000 * 1000; dep = huge + 1; other = 8; in dep + other")
print(f"alpha-rename: hits={renamed['hits']} misses={renamed['misses']} (의미 불변 → 전부 hit)")
assert renamed["hits"] == 3 and renamed["value"] == changed["value"]

# realisation early cutoff: 같은 IR은 평가 없이 결과 증명
r1 = inc.realisation_record("let a = 6; in a * 7")
r2 = inc.realisation_record("let a = 6; in a * 7")
print(f"realisation: first early_cutoff={r1['early_cutoff']} second={r2['early_cutoff']} value={r1['value']}")
assert r1["early_cutoff"] is False and r2["early_cutoff"] is True and r1["value"] == 42

assert inc.incremental_eval_report()["ready"]
print("→ 의미가 정체성: 부분 재사용 + alpha-rename 면역 + realisation cutoff.")
