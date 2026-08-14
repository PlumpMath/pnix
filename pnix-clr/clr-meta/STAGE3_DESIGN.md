# clr-meta Compiler Stage3 design

상태: **implemented + gated** (2026-08-07). Live gate
`scripts/clr-meta-compiler-selfhost-stage3-gate` PASS; receipt는
`work/compiler-selfhost-stage3-gate.receipt.json` 아래. 가장 높은 closed
compiler floor는 이제 **Stage3** (C3 Stage2는 parent로 남음). Stage4+는 여전히
open (`STAGE15_N_ROADMAP.md`).

## 목표

**Stage3**는 C3 Stage2 이후 첫 **same-source recompile convergence** 단계다:

```text
Stage1 (C2 host-seeded) → Stage2 (C3 same-source) → Stage3 (Stage2 recompiles
same frozen kernel → semantic + structural convergence under fresh load)
```

Stage3는 **아님**:

- evaluator generation nesting (gen0–2는 분리 유지);
- Stage15/N open-world evidence;
- full ClojureCLR replacement;
- PE byte-identity fixed point (그건 Stage8 policy);
- tri/five-host full corpus로의 automatic host promotion.

## 전제조건 (이미 closed)

| Checkpoint | Claim |
|---|---|
| C0 | selfhost ABI/attack contract |
| C1 | frozen kernel의 recursive source admission |
| C2 | host-seeded executable Compiler Stage1 PE |
| C3 | Stage1→Stage2 same-source compile + source-hidden fresh-target replay |

Stage3가 hash로 pin해야 하는 입력:

- C3 Stage2 PE + support triplet + Stage2 manifest
- frozen `pnix.clr-meta.compiler-kernel.v1` source closure (C3와 byte-identical)
- C3 lineage의 profile / plan / toolchain snapshot digest

## Stage3 definition of done

Stage3 gate receipt (`compiler_stage3=true`)는 **모두** 성립할 때만:

1. **Parent bind** — Stage2 artifact와 parent lineage hash가 frozen C3
   receipt와 일치 (silent parent rewrite 없음).
2. **Same-source recompile** — Stage2 (Stage1 아님, host ClojureCLR 아님)가
   **exact** frozen kernel source를 Stage3 PE bundle로 compile.
3. **Fresh load** — Stage3 PE가 Stage3 + support만 있는 directory에서 load;
   Stage1/Stage2 PE, kernel source, ClojureCLR product load path 없음.
4. **Semantic agreement** — Stage3가 post-Stage2 nonce target family와
   arithmetic/equality/truthiness target을 compile·run; 같은 input에서
   Stage2와 observation 일치.
5. **Structural description equality** — method/field inventory와 normalized
   structural description이 Stage2와 equal (C3와 같은 policy: raw PE bytes는
   Stage8까지 다를 수 있음).
6. **Source-hidden replay** — kernel source가 compiler directory에 없는
   상태에서 recompile + target execution 성공 (C3 replay와 같은 honesty).
7. **No auto-promotion** — receipt가 기록:
   `compiler_self_reproduction=false`, `compiler_stage15_n=false`,
   `il_fixed_point=false`, `promotion/allowed?=false`.

## Stage3 explicit non-goal

```text
compiler_stage4_through_7_convergence   # later stages
compiler_self_reproduction              # Stage2 already emits Stage3; full
                                        # closed self-reproduction loop is
                                        # a later named gate
clr_il_fixed_point / raw PE equality    # Stage8
broad_clojureclr_compatibility
pnix_common_compiler_integration
cross_host_canonical_equivalence
```

## Work package (구현 순서)

### WP-A — Stage3 plan + receipt schema

- artifact/plan schema에 `compiler_stage3` boolean 확장 (default false).
- 새 receipt schema
  `pnix.clr-meta.compiler-stage3.receipt.v1` 필드:
  - parent Stage2 digests, source closure digests
  - Stage3 output digests, structural description digest
  - semantic target matrix (nonce + arithmetic family)
  - `source_hidden_replay: true|false`
  - `promotion/allowed?: false`

### WP-B — Stage3 builder

- 입력: verified C3 Stage2 bundle + frozen kernel source.
- 동작: kernel source에 대해 Stage2를 compiler로 호출 (host AOT 아님).
- 출력: Stage3 PE + support copy + Stage3 manifest.
- Stage2 누락, source hash drift, 또는 admitted form에 host fallback 관찰 시
  fail closed.

### WP-C — Stage3 gate script

- `scripts/selfhost-stage3-gate` (구현 시 이름 고정):
  1. parent C3 receipt 검증
  2. Stage3 build (또는 `--no-build`로 기존 소비)
  3. Stage2↔Stage3 structural compare
  4. Stage3 only source-hidden fresh-target replay
  5. receipt 기록; definition-of-done 성립 시에만 exit 0

### WP-D — Mutation / negative matrix

C2/C3 스타일 no-output failure 유지:

- Stage3 publication의 identity/metadata mutation
- Stage3 control path의 arithmetic lowering mutation
- missing support / wrong parent hash / source drift

### WP-E — 문서 honesty

- live green 게이트 후에만 `STATUS.md` 갱신.
- WP-C 통과 전까지 `STAGE15_N_ROADMAP.md` Open claims 유지.
- 별도 product-admission 결정 전까지 Stage3를 `pnix-clr` product artifact
  plan에 fold하지 말 것.

## 제안 acceptance 명령 (future)

```sh
# From pnix-clr/clr-meta/  (design only — script not landed yet)
./scripts/selfhost-stage3-gate --build
# expects: receipt compiler_stage3=true, promotion/allowed?=false
```

## product host와의 관계

Stage3는 **clr-meta** compiler self-hosting을 진전시킨다. 그 자체로 다음을
하지 **않는다**:

- `pnix-clr` language surface 확장;
- full tri-host corpus에 clr admit (먼저 five-host **common slice** 사용);
- Trusting-Trust defense claim.

## “Stage3 closed” exit criteria

Live machine 증거:

1. stage3 게이트 exit 0
2. STATUS에 receipt hash 기록
3. Open claims flip만:
   - `compiler_stage3` (narrow) → true  
   - Stage4–7 / self-reproduction / fixed-point는 각자 게이트까지 false
