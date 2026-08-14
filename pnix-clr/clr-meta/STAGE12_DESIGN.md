# clr-meta Compiler Stage12 design

상태: **closed (live gate PASS)** 2026-08-12. Parent floor는 Stage11.

## 목표

Compiler change는 replay와 gate admission까지 quarantine 유지 (roadmap 자체
one-line Stage12 정의): promotion-policy table
(`proofs/quarantine-policy.tsv`)과 두 `DONE` 게이트의 live replay.

## 게이트 (6 row)

- `local-verification` (`DONE`) — 모든 accepted slice가 Stage11 adapter
  closure를 통과해야 함. Stage11 게이트를 한 번 실행해 replay.
- `candidate-intake` (`DONE`) — 새 작업은 어떤 promotion 전에도
  `proofs/stage-manifest.tsv` row로 진입. `manifest-check`를 한 번 실행해
  replay.
- `remote-ci` (`HELD`) — monorepo workflow가 change를 promote할 수 없음.
- `manual-promotion`, `self-modification`, `external-evidence` (`HELD`) —
  아직 존재하지 않는 explicit receipt를 모두 요구.

## 명령

```sh
./clr-meta/scripts/clr-meta-compiler-selfhost-stage12-gate
```

## Live receipt

`work/compiler-selfhost-stage12-gate.receipt.json` (gitignored),
`claims.stage12 = true`, `claims["promotion/allowed?"] = false`.
