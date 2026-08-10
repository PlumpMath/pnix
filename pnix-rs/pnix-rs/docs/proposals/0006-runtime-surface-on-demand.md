# 0006 — px runtime 표면 확장 (수요 기반)

상태: HELD (로드맵 rank 4). 수요(mirror/gate/corpus 유기체가 요구) 발생 시에만.

## 대상 (SCOPE_LOCK held 표면)
- int↔float 승격 (현재 float×float만 — Nix는 승격, divergence 기록)
- 중첩 attrset-키 보간, string `+`, bool `&& || !`, `?`, `rec`, `with`, paths
- 비유한 float canonical-print (inf/NaN — P1 성질의 예외, 감사 #2 기록)
- float tower 인코딩

## 원칙
수요 없이 확장 금지(SCOPE_LOCK §2 placeholder 비재해석). 각 확장은 corpus 가드 +
substrate-check(px.rs가 rs-meta subset 유지) 통과 필수. 승격은 cross-host
divergence 재검토 동반.

## 모듈/게이트
px. 게이트 = corpus 확장 + substrate 3-way.
