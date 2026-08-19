# pnix-clj

## 스코프 잠금

[`docs/IMPLEMENTATION.md`](docs/IMPLEMENTATION.md) §5(스코프 경계) 참고
(예전 `SCOPE_LOCK.md`의 후신, 2026-08-20 통합).

`pnix-clj` 코어는 Clojure 호스팅 pnix 메타원형 증명 레인으로 한정된다:
parser, lowering, evaluator, CAS, store, snapshot, mirror, purity, determinism,
tower, witness, receipt, replay, 그리고 `clj-meta` 호스트 증명 레인.

NL/MSV/Hangul, 한국어 mirror/사전, graph-gate/gate-graph, 다언어 emit,
coding-agent emit, puck-cli bridge, tick runner, redb ingest brain,
NL meaning graph 레인은 코어 범위 밖이다.

**호스트 임포트 (공개 API):** [docs/IMPLEMENTATION.md](docs/IMPLEMENTATION.md) §11  
**이중 축:** monorepo [`../../HOST_DEV_ENV.md`](../../HOST_DEV_ENV.md)
