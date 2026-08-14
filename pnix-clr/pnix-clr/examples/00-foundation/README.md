# 00 — 파운데이션 (clr seed)

**코어 00–06** 경로의 첫 단계 ([FOUNDATION_PATH.md](../FOUNDATION_PATH.md)).

바깥 `pnix-clr/` 에서:

```sh
./bin/pnix-clr pnix-clr/examples/00-foundation/program.px
./bin/pnix-clr -e '(-7) * (-6)'
./bin/pnix-clr --pnix-meta-smoke
```

첫 명령은 CLR 네이티브 seed 를 직접 돌린다. 둘째는 소스 기원 Int64 산술 경로.
smoke 는 형제 `pnix-meta` 트리의 정규 케이스
`bool-01`, `builtin-dead-import-01`, `hasattr-apply-precedence-01`,
`production-checked-i64-01` 를 로드해 핀 JSON 과 맞춘다.
float/BigInt/일반 수치 승격이나 primitive-manifest 강제를 주장하지 않는다.

카탈로그 색인: [../README.md](../README.md).  
호스트 간 균형: 모노레포 `examples/EXAMPLES_BALANCE.md`.
