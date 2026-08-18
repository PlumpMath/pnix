# pnix-clj / clj-meta 스코프 잠금

pnix-clj는 Clojure 호스팅 pnix 런타임 및 메타원형 증인 substrate다.

clj-meta는 호스트 언어 증명 레인이다.

## 범위 안

- pnix 소스
- tokenizer / parser
- pnix AST
- canonical form / lowering
- content hash / CAS
- store / snapshot
- eval-source / eval-from-ast
- mirror / mirror-chain
- purity / determinism
- tower / stage closure
- witness / receipt / replay
- clj-meta host reflection / compiler proof lane

## 범위 밖

다음 레인은 pnix-clj 코어에 들어오면 안 된다:

- Hangul codec
- MSV / meaning sentence variants
- Korean dictionary / Korean mirror
- domain token matching
- graph-gate / gate-graph
- multi-language emit registry
- behavior-atom coding-agent emit
- puck-cli executor bridge
- autonomous tick runner
- redb ingest brain
- NL corpus / meaning graph / answer composer

## 규칙

기능이 메타원형 Clojure 호스팅 pnix 증명에 속하지 않으면 코어 게이트에 추가하면 안 된다.


---

## 소유자 수정 2026-07-08 — 공유 common-.px 코어 로딩은 범위 안 (B6)

명확화 (이 잠금은 공유 코어를 막아 둔 적이 없다 — "범위 밖" 목록은
`clj-msv` 크램: MSV / gate-graph / coding-agent / NL). **공유 common-`.px`
코어**는 기존 범위 안 `eval-source` / `import` / `tower` / `mirror` 레인의
직접 확장으로서 여기 범위 안이다:

- 외부 `../pnix-meta` 루트에서 common `.px` 로딩 (승인된 external-root
  loader, blocker B2 — 원래 `pnix-zero` 이전 저장소의 project-wiki에서
  추적되던 항목이며, 이 자기완결 트리에는 그 이전 저장소가 없다);
- 공유 정규 결과 + held reason 방출 (B1);
- 실제 호스트 IO로의 effect/capability 브리지 (B3).

위 범위 밖 울타리 (Hangul/MSV/gate-graph/coding-agent/…)는 **변경 없음** —
이 잠금이 막으려는 크램 그대로다. 헌법 구속: meta-first, 비회귀
(`bin/pnix-clj-gate` green 유지), 자동 승격 없음.
