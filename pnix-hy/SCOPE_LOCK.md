# SCOPE_LOCK — pnix-hy / hy-meta

> 권위 있는 경계 선언. `hy-meta/`(호스트 레인)와 `pnix-hy/`(pnix 런타임 레인) **둘 다** 관장한다.
> 무엇을 구현하기 전에 먼저 읽을 것. 2026-07-01 수립.

## 0. Source of truth — 닫힌 상태는 이 BRANCH에 있다, `main`이 아니라

"scope 안에서 닫힘" 주장은 scope-relative일 뿐 아니라 **branch/ref-relative**다.
- **RECONCILED 2026-07-01:** `main`이 `374a8e4`에서 `3f0e186`로 fast-forward됨(clean, 충돌 없음;
  `374a8e4..3f0e186 HEAD -> main`). **이제 `main`이 권위 있는 닫힌 상태**다 — 감사, 모든 follow-up
  종료, drift-guard(`314b89f`), stub-hunt verdict(`accad7b`), 이 scope lock을 포함한다. branch
  `codex/stage10-session-sandbox-closure`와 `main`은 같은 라인(ahead/behind 0/0).
- reconcile 이전 격차(main이 186 커밋 뒤, 오래된 open todo: stage8/9 CI wiring, drift/diff 도구,
  stage10-15 planning, canonical-json boundary-deferred 항목)는 이 fast-forward로 해소됨.

**운영 노트:** source-of-truth 정리됨 — `main` == 이 ref. 동기화 유지: 이후 scope-lock/문서 변경도
`main`을 함께 fast-forward(`git push origin HEAD:main`)해 `main`이 다시 권위 상태 뒤로 밀리지 않게 한다.

## 1. Status — scope-relative (이 정확한 표현을 쓸 것)

**맞음:**
> pnix-hy / hy-meta는 현재 정의된 meta-circular projection scope 안에서 open todo와
> genuinely unimplemented stub는 0으로 수렴했다.
>
> Complete **with respect to the stated meta-circular-projection scope.**

**틀림(쓰지 말 것):** "전체 AI가 완성됐다" / "Complete overall" / "the project is finished."
`미완 0`은 **항상** scope-relative다. 여기서 완성이란 오직: 선언된 Hy/Python ↔ pnix
meta-circular projection surface(§1–§24) 안에서 open todo가 없고 genuinely unimplemented stub가
없다는 뜻이다.

증거(2026-07-01): 두 todo `[ ]` = 0; `--check` 44/44 all_ready; working tree clean; origin-synced.
세 번의 독립 multi-agent adversarial sweep 수렴 — capability audit(23 agents), follow-up
re-verification(8 agents), stub-hunt(15 agents) — 모두: genuine unimplemented code 없음, 모든
placeholder는 의도적/문서화됨. `pnix-hy/docs/IMPLEMENTATION_AUDIT.md` 참고.

## 2. 대원칙

> **의도적 placeholder를 미구현으로 재해석해서 구현하지 말 것.**
> (No new implementation may reinterpret intentional placeholders as missing work.)

미래의 주된 위험은 "미구현"이 아니라 — LLM이 문서화된 placeholder를 gap으로 오인해 이 닫힌 scope를
다시 열어젖히는 것이다.

## 3. 의도적 placeholder — 고정(구현하지 말 것)

이것들은 설계상 그렇다. 각각 "미구현"이 아니라 "문서화된 이유로 부재"다:

| Placeholder | 위치 | 왜 부재로 두어야 하나 |
|---|---|---|
| derivation `outPath`/`drvPath` store addressing | `pnix_runtime.py:~2694` (`derivation_value`) | pnix는 Nix store hashing을 구현하지 않음 — 의도적; self-test로 단언됨 |
| `builtins.placeholder` | `pnix_runtime.py:~3885` | 이건 진짜 Nix builtin(`=placeholder!<name>`)이지 stub 아님 |
| `trace` / `warn` value-identity | `pnix_runtime.py:~4059` | 순수 mirror는 stderr 부작용을 의도적으로 생략 |
| §9 pnix macro / quasiquote / defmacro / reader-macro / require | `_QUASIQUOTE/_DEFMACRO/_READER_MACRO/_IMPORT_PNIX_NOTE` (hy_mirror.py) | pnix는 비동형; Hy 쪽은 투영으로 OBSERVE만 |
| `#_pnix-gap[...]` / `#_pnix-<tag>` 투영 마커 | `pnix_mirror.py` `_pnix_to_hy`/`_python_expr_to_pnix`/... | 의도적 "clean 투영 없음" 마커, 항상 `gaps.append`와 짝 |
| fail-closed stage16 peer-review (all-None record) | `hy-meta/bootstrap.py:~6437` | all-None 값이 헌법상 유일 허용값(non-None = DRIFT) |
| host standalone/optional fallback | `interop.py`, `host_exec.py`, `hy_mirror.py` worker, oracle | hy-meta/host 모듈 부재 시 standalone 실행 보존 |
| runtime "unsupported operand/algorithm/AST tag" | `pnix_runtime.py`, `host_exec.py` guards | 이건 ERROR MESSAGE이지 stub 아님 |

## 4. Forbidden implementations (scope 재개봉 요소)

- §3 placeholder를 gap으로 재해석해 "채우기".
- pnix macro / quasiquote / reader-macro(§9) 구현 — pnix 언어가 명시적으로 동형(homoicony)을
  채택하기로 결정하지 않는 한(proposal 수준 결정, §7).
- `derivation` outPath/drvPath용 Nix store hashing.
- fallback이나 error-message를 "기능"으로 바꾸기.
- OUT-of-scope(§5) 항목을 "미구현 작업"으로 이 저장소에 끌어들이기.
- 새 기능을 proposal 대신 `todo.md [ ]`로 시작(§7).

## 5. In scope vs OUT of scope

**IN scope**(이 lock이 관장): Hy/Python ↔ pnix meta-circular projection surface(§1–§24),
hy-meta = HOST 자기컴파일/평가/재현/inspect proof 레인; pnix-hy = 그 위 pnix 런타임; interop =
명시적 경계; SINGLETON pnix mirror; 별도 수렴 게이트로서의 4-lane parity.

**OUT of scope**(별도 문제 — 여기서 "미구현" 아님, 이 lock 아래 추가 금지): 더 큰 제품 scope ·
pnix 전체 언어 완성 · cross-repo ABI 통합 · nix-msv-template 레이어 · pnix-clj feature branch ·
rs-meta stageN · pnix-hs · 실서비스 런타임. 각각 자기 scope + proposal이 필요하며, pnix-hy /
hy-meta의 gap이 아니다.

## 6. ABI 경계 — 유일한 공유 envelope

두 레인 사이의 유일한 공유 계약:
- **§14 witness FIELD SCHEMA**(in_hash/out_hash/env_hash/status/loss + InteropRecord 필드명) —
  런타임 drift-guard(`gate.gate_report:witness_schema_ok`) 포함.
- **§18/§19 opaque-ref shape** — `__hy_meta_opaque__`(호스트) / `__pnix_opaque__`(pnix fallback).

그 외는 전부 레인-로컬. hy-meta는 호스트 Python/Hy artifact·import hook·clean replay·
introspection·실행 floor를 소유; pnix-hy는 pnix reader/parser/AST/IR/eval/value/builtins/mirror/
stage-ladder/gate를 소유. 공유 envelope 변경은 두 레인 + drift-guard를 함께 갱신하고 proposal에 기록.

## 7. 변경 프로세스 (신규 — ad-hoc todo 증식을 대체)

- 새 기능은 **proposal 문서**(`pnix-hy/docs/proposals/NNNN-<slug>.md`)로 시작, `todo.md [ ]`가
  아니라. `todo.md`는 in-scope·이미 합의된 작업만.
- proposal은 반드시 밝힌다: 어느 scope인가, 의도적 placeholder나 OUT-of-scope 항목을 건드리는가,
  그렇다면 이를 승인하는 명시적 human decision.
- proposal이 수락된 뒤에만 작업이 `todo.md`로 들어간다.

## 8. 한 줄 요약

> pnix-hy / hy-meta는 현재 scope 안에서 닫혀 있다. 다음 위험은 "미구현"이 아니라 — 에이전트가 닫힌
> scope를 다시 여는 것. 경계를 지켜라; 재해석하지 말고 proposal하라.


---

## OWNER AMENDMENT 2026-07-08 — shared common-.px core admitted IN scope (B6)

Owner-authorized proposal per §5 (which requires OUT-of-scope items to be
admitted by an explicit scope + proposal). The **shared common-`.px` core**
track is now IN scope for this repo:

- loading common `.px` from `../pnix-meta` through the pnix-hy runtime;
- the cross-repo canonical-result + effect/capability ABI (blockers B1–B3,
  originally tracked in the `pnix-zero` predecessor repo's project-wiki — this
  self-contained tree has no such sibling tree to load);
- the "full pnix language" growth **only as needed to run the shared corpus**.

Bound by the constitution (`./CLAUDE.md`):

1. **Non-regression** — the existing §1–§24 closed scope stays closed and its
   gate stays green; the shared-core track is ADDITIVE, never a rewrite.
2. **Meta-first / no cram** — grow through `hy-meta`; do not race the product
   surface ahead of the substrate (this repo is attempt #3, not `clj-msv`).
3. The §5 fence REMAINS for everything NOT part of this shared-core track.

This amendment lifts the §5 fence for the shared-core track only.
