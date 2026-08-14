# clr-meta Compiler Stage10 design

상태: **closed (live gate PASS)** 2026-08-12. Parent floor는 Stage9.

## 목표

Isolated load-context, classpath, session, sandbox replay (roadmap 자체
one-line Stage10 정의). 두 부분: 관련 boundary마다 explicit stance를 선언하는
policy table (`proofs/session-sandbox.tsv`), 그리고 실제로 로컬 검사 가능한
두 속성의 live replay — assert 아님, 가정 아님.

## Live-replay되는 것 (선언만이 아닌 real check에 근거)

1. **Load-context shadow rejection.** `bin/clr-meta`는 이미 pinned ClojureCLR
   runtime root에 심어진 `pnix.clr-meta` namespace shadow를 무엇이든 실행하기
   전에 거부한다 (`bin/clr-meta` 약 119-126행) — Stage10 이전부터 존재했으나
   어떤 게이트도 테스트하지 않음. 게이트가 real shadow DLL을 plant하고,
   `env -i` 아래에서 `bin/clr-meta -e "(+ 1 1)"`을 두 번 실행하며, 두 run
   모두 exit 2와 byte-identical stderr로 실패해야 하고, shadow를 제거한 뒤
   *같은* 명령이 다시 성공함을 확인한다 (거부가 심은 파일 때문이지 unrelated
   breakage가 아님을 증명).
2. **Session replay.** 두 명령 session (`--gate` 다음 `-e "(+ 40 2)"`)을
   `bin/clr-meta`로 `env -i` 아래에서 두 번 실행하고, 두 run 간
   byte-identical combined stdout을 요구.

## Policy-only인 것 (선언됨, 여기서 독립 재증명하지 않음)

- **Classpath scoping**: tool invocation에 대해 `CLOJURE_LOAD_PATH`는 항상
  `clr-meta/src`만으로 set (`bin/clr-meta` source를 읽어 검증; 이 게이트의
  별도 live probe 아님 — 스크립트 자체 텍스트에 대한 static claim이며,
  Stage9 clean-process matrix가 이미 functionally exercise).
- **Sandbox env**: 모든 compiler-selfhost stage 게이트(1-9)가 이미
  `dotnet` subprocess 호출을 explicit `env -i` allowlist로 라우팅 —
  Stage1-9 자체 established practice이며, 여기서 두 번 재검증하지 않고
  Stage10 policy line으로 기록.
- **Remote CI** (`HELD`): monorepo-level GitHub Actions workflow
  (`.github/workflows/hosts.yml`, `clr-gate` job)가 `nix run .#gate`로 매 PR
  이 host를 exercise하지만, outcome은 external이며 어떤 local proof check도
  fetch하거나 trust하지 않음.
- **Network / external sandbox** (`HELD`): local proof 범위 밖; 어느 쪽
  adapter receipt도 없음.

## Non-claim

Stage11-15/N, compiler self-reproduction, IL fixed-point, ClojureCLR
replacement, promotion.

## 명령

```sh
./clr-meta/scripts/clr-meta-compiler-selfhost-stage10-gate
```

## Live receipt

`work/compiler-selfhost-stage10-gate.receipt.json` (gitignored),
`claims.stage10 = true`, `claims["promotion/allowed?"] = false`.
