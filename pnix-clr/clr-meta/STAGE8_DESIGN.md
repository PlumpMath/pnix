# clr-meta Compiler Stage8 design

상태: **closed (live gate PASS)** 2026-08-12. Parent floor는 Stage7.

## 목표

`PersistedAssemblyBuilder`-based Compiler Stage1-7 family에 대한
reproducible assembly artifact closure: 이 codegen path가 실제로 만드는
non-deterministic PE field에 대한 explicit, empirically-grounded policy, 그리고
같은 frozen Stage6 parent에서 두 independent Stage7 build가 그 policy 아래
byte-identical임을 증명하는 게이트.

## 실제로 non-deterministic이었던 것 (가정 아닌 측정으로 발견)

같은 frozen source의 두 build를 몇 초 간격으로 돌리면 `CompilerStage7.dll`
output이 정확히 17 bytes, 정확히 두 region에서 달랐다 (fix 작성 전
`cmp -l` byte-offset diffing으로 확인):

1. **PE COFF header `TimeDateStamp`** (4 bytes at
   `PEHeaders.CoffHeaderStartOffset + 4`) — real build wall-clock time.
2. **Module `Mvid`** (`#GUID` metadata heap의 16 bytes) —
   `PersistedAssemblyBuilder`가 매 `Save()`마다 부여하는 fresh random GUID.

그 외는 변하지 않았다. 특히: 이 codegen path는 PDB를 쓰지 않고 debug
directory를 embed하지 않으므로, *이 artifact family*에서는 debug info와
embedded source path가 non-determinism source가 아니다 — 이는 `PeSink.cs`에
대한 checked fact이지, debug info를 emit하는 미래 codegen path에
일반화되는 가정이 아니다.

## Policy

`PersistedAssemblyBuilder.Save()`는 어느 field도 set하는 public API를 노출하지
않는다. `PeSink.Finish()`는 이제 `Save()` 이후, artifact가 final path로
move되기 전에 둘 다 canonicalize한다. 이 pipeline이 만드는 모든 artifact에
대해 (flag 뒤가 아님 — 어느 field도 `compile`/`invoke`/`describe`/`prepare`/
`publish-directory`에 semantically observable하지 않음):

- `TimeDateStamp` → `0`.
- `Mvid` → `00000000-0000-0000-0000-000000000000`.

timestamp는 `PEHeaders.CoffHeaderStartOffset`(real PE structural offset,
hardcoded byte position 아님)로 찾는다. MVID는
`MetadataReader.GetModuleDefinition().Mvid`가 보고하는 *실제* GUID를 읽은 뒤,
file 안에서 그 exact 16-byte sequence가 정확히 한 번 나오도록 require한 후
overwrite한다 — compiled program content에 따라 heap layout이 바뀌므로
hardcoded heap offset이 아니다. 전체 policy record는
`compiler-selfhost/stage8-contract.edn` 참조.

`Pnix.ClrMeta.CompilerSupport`의 새 `describe-determinism` verb가 finished
artifact에서 두 field를 writer와 독립적으로 다시 읽으므로, 게이트가
`Finish()`가 돌았다고만 믿지 않고 artifact 자체에서 claim을 re-derive한다.

## Non-claim

Stage9 (clean-process replay), compiler self-reproduction, IL fixed-point,
Trusting-Trust defense, ClojureCLR replacement, promotion. debug info를 emit
하거나 source path를 embed하는 미래 codegen path의 determinism은 다루지
않음 — 이 contract는 `PeSink.cs`가 실제로 변한다고 측정된 field만 다룬다.

## 명령

```sh
./clr-meta/scripts/clr-meta-compiler-selfhost-stage8-gate
dotnet runtime/Pnix.ClrMeta.CompilerSupport.dll describe-determinism ARTIFACT.dll
```

## Live receipt

`work/compiler-selfhost-stage8-gate.receipt.json` (gitignored),
`claims.stage8 = true`, `claims.raw_artifact_reproducibility = true`
(scope: `compiler_stage7_persisted_assembly_builder_output`),
`claims["promotion/allowed?"] = false`.
