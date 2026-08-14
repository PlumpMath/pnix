# clr-meta Compiler Stage11 design

상태: **closed (live gate PASS)** 2026-08-12. Parent floor는 Stage10.

## 목표

source, IR, compiler, runtime, compatibility surface 전역의 one accepted/failed
boundary (roadmap 자체 one-line Stage11 정의). 여기서는: clr-meta가 실제로 가진
모든 integration surface를 하나의 adapter policy table
(`proofs/adapter-schema.tsv`)에 선언하고, 모든 `DONE` adapter의 closure가
live-replayable해야 함으로 읽는다.

## Adapter (5 row)

- `local-clojureclr` (`DONE`) — `bin/clr-meta` CLI + pinned ClojureCLR NuGet
  package + dotnet toolchain. Stage9 게이트를 **한 번** 실행해 replay
  (두 번 아님 — 아래 "why replay-once" 참조).
- `compiler-selfhost-native` (`DONE`) — `PersistedAssemblyBuilder`-based
  compiler-selfhost artifact family. Stage8 자체 latest checked receipt
  (`work/compiler-selfhost-stage8-gate.receipt.json`)를 읽어 replay — Stage8
  게이트 재실행 아님.
- `github-actions` (`HELD`) — monorepo-level `hosts.yml` workflow가 매 PR
  remote로 이 host를 exercise하지만, outcome은 external이며 여기서
  fetch하거나 trust하지 않음.
- `external-nuget-feed` (`HELD`) — pinned/cached ClojureCLR package 너머
  package fetch는 local proof boundary 밖.
- `cross-implementation` (`HELD`) — Stage14로 연기.

## replay-once인 이유, replay-twice가 아닌 이유 (구축 중 수정)

이 게이트 초안은 Stage9 *entire* 게이트를 두 번 재실행했다 (Stage8-10의
"run twice, require identical" 패턴 모방). 여기서는 틀렸다: Stage9가 이미
자체 clean-process replay property를 내부에서 증명하므로, Stage11에서 두 번째
재실행은 증거를 더하지 않고 비용만 두 배로 만든다. 더 나쁘게, later stage
(`12`, `13`, `14`, `15`, `N`)마다 predecessor를 "두 번 replay"하고 그
predecessor 또한 자신의 predecessor를 "두 번 replay"하면 hop마다 비용이
두 배 — stage depth에 대해 quadratic이며 StageN에서 거의 작동 불능에 가깝다.
Stage11부터 모든 stage는 대신 referenced predecessor를 **한 번** 호출한다:
referenced property가 오늘 source에 대해 여전히 성립함을 확인하기에 충분하고,
그 stage가 자신에 대해 이미 증명한 property를 재증명하지 않는다.

## Non-claim

Stage12-15/N, compiler self-reproduction, IL fixed-point, ClojureCLR
replacement, promotion.

## 명령

```sh
./clr-meta/scripts/clr-meta-compiler-selfhost-stage11-gate
```

## Live receipt

`work/compiler-selfhost-stage11-gate.receipt.json` (gitignored),
`claims.stage11 = true`, `claims["promotion/allowed?"] = false`.
