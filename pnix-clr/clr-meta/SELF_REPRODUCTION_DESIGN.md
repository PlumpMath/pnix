# clr-meta compiler self-reproduction / B==C fixed point

상태: **closed (live gate PASS)** 2026-08-12. Stage8의 PE canonicalization에 의존.

## 목표

`todo.md`가 추적하는 open item: hy-meta와 rs-meta가 이미 닫는
"kernelB compiles kernelC, B==C" 패턴을 증명한다 — Stage3-7의
immediate parent에 대한 기존 structural-description equality만이 아니라,
genuine self-hosting fixed point.

## 발견된 것 (처음부터 만든 것 아님)

Stage8 자체 게이트 출력이 계획되지 않은 bonus observation으로, Stage3부터
Stage7까지 compiled `CompilerStageN.dll`이 하나의 sha256을 공유함을 이미
로깅했다. 이 검사는 그 발견을 형식화한다: Stage1부터 Stage7까지 fresh
build하고 **일곱** stage 모두 — adjacent pair만이 아니라 Stage1 자체
포함 — 정확히 같은 sha256
(`19872f28ed3576cbdf50001649fb9fd773023778fa0bb5c3d7aee45a61baecb7`,
검증 run 기준)을 공유함을 확인한다. 이는 Stage8 canonicalization
*때문에* 성립한다: 모든 stage가 같은 frozen `compiler_kernel.clj` source를
compile하여, 같은 `PersistedAssemblyBuilder`-based codegen path를 통해 같은
IL을 만든다 — non-deterministic PE field 두 개뿐인 `TimeDateStamp`, `Mvid`가
canonicalized되면 stage 사이에 다를 것이 남지 않으며, Stage1의 host-seeded
build도 (모든 later stage와 같은 `PeSink.Finish()` path를 거침) 포함된다.

공유 Stage7 artifact를 통한 unseen target (`unseen_add.clj`)의 live
compile+execute는 shared bytes가 vacuously identical-but-broken이 아님을
확인한다.

## Non-claim

이것은 Compiler Stage1-7 family의 `PersistedAssemblyBuilder` PE-artifact
output에 대한 fixed point 한정이다 — 일반 CLR IL format claim도,
ClojureCLR replacement도, promotion도 아니다.

## 명령

```sh
./clr-meta/scripts/clr-meta-compiler-self-reproduction-check
```

## Live receipt

`work/compiler-self-reproduction-check.receipt.json` (gitignored),
`claims.compiler_self_reproduction = true`, `claims.fixed_point = true`,
`claims["promotion/allowed?"] = false`.
