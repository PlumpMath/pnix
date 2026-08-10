# Proposal 후보 — Hy(Python) ↔ pnix 언어기능 interop

> 상태: **후보 카탈로그**(수락·구현 안 됨). `/SCOPE_LOCK.md` §7 기준, 새 기능은 여기서 proposal로
> 시작하지 `todo.md [ ]`가 아니다. 2026-07-01 수립.
>
> 근거: 두 독립 scout 수렴 — Claude 다중-에이전트 적대적 scout(11 agents, 5 dimensions, ~0.79M
> tokens)과 codex 분석(P1–P10). **필수 미구현 기능은 없다**(작성 시 `--check` 44/44; 0001-0007
> shipped 후 54/54; stub-hunt = genuine stub 0). 아래는 전부 사람이 결정할 **선택적** 언어-interop
> 개선이다. pnix-쪽 매크로는 어느 것도 제안하지 않는다.

## 요약

interop 코어는 구현됨(InteropRecord / to_host / from_host / opaque ref / call_host(_method) /
module bridge / capability gate / SR5 host adapter). 제안 가능한 영역은 셋으로 뭉친다:
**(A) value/opaque fidelity** — 일부 host 타입이 손실을 조용히 잘못 표기; **(B) callable/module
reach** — pnix 소스가 아직 host callable을 못 부르고, `call_host`가 kwargs를 버리고, host arity
투영이 없음; **(C) macro/quote interop 방향** — SCOPE_LOCK가 허용한 두 방향(Hy 매크로를 pnix-투영
폼 위에; pnix 값을 Hy quasiquote 구멍에)이 커버리지 0. 두 scout가 일치; Claude scout는 구체적
loss-marking 결함을, codex scout는 ABI 프레이밍(versioned correspondence table, no-mirror 불변식,
role matrix)을 추가.

## 후보 카탈로그 (별도 표기 없으면 in-scope-optional) — 통합

Cluster A — value / opaque fidelity (전부 `pnix_hy/interop.py`, thin):

| # | 후보 | 가치 | 무엇을 추가 | 재사용 | codex |
|---|---|---|---|---|---|
| A1 | `roundtrip_host_value` fidelity 헬퍼 + loss 리포트 | high | probe 집합에 `to_host(force(from_host(v)))` → equality/loss 표; A-losses를 한 곳에 노출 | `from_host` L315, `to_host` L288, `rt.force_value` L1401 | — |
| A2 | `from_host` tuple→list를 lossy로(현재 `lossless` L330지만 `stable_data`가 붕괴) | med | 표기 뒤집기 | `LOSS_STATUSES` L32 | — |
| A3 | `from_host` 비-str dict 키 lossy/충돌(`{1:'a','1':'b'}` 충돌, lossless 주장) | med | 비-str 키 감지 → lossy | dict branch L332 | ~P1 |
| A4 | `from_host` bytes/bytearray 커버(현재 → opaque) | med | bytes→string/octets, lossy | list branch L322, `_pnix_kind` L262 | — |
| A5 | `from_host` set/frozenset 커버(현재 → opaque) | med | set→list, lossy | list branch L322 | — |
| A6 | `to_host`가 `PnixPath`/`PnixString` provenance 표기(`realize_value`가 str로 벗김) | med | stable_data 전 raw 검사; `output_kind`/lossy 표기. `realize_value`는 손대지 말 것 | `PnixPath` L101/`PnixString` L108 | — |
| A7 | `from_host`에서 기존 opaque ref 통과(re-crossing) | low | `from_host` 상단 `is_opaque_ref` guard | `is_opaque_ref` L192 | — |

Cluster B — callable / module reach:

| # | 후보 | 가치 | 무엇을 추가 | 재사용 | codex |
|---|---|---|---|---|---|
| B1 | `host_callable_to_pnix` — pnix eval 안에서 host callable 적용 | high | pnix env에 넣을 수 있는 `rt.NativeFunc(force_arg=True)`로 래핑 | `rt.NativeFunc` L66/`apply_pnix` L3733/`eval_source_raw` env-merge L4336, `check_capability` L354 | — |
| B2 | `call_host` kwargs 통과 | med | `kwargs` 전달(local + host-adapter 경로), `call_host_method`와 맞춤 | `call_host_method` L401/L443 | P4 |
| B3 | host-callable arity → `functionArgs` 투영 | med | signature를 pnix `{name: has_default}`(+var*) attrset으로 | `rt.function_args_value` L2329, `inspect_object` L60 | P4 |
| B4 | 모든 crossing에 균일한 per-call witness | med | `call_host` local, `call_host_method` scalar, `wrap_pnix_callable`/`pnix_callable`에 witness 찍기 | `interop._local_witness` L169, `gate.make_witness` | P2/P4 |
| B5 | `host_module_to_pnix(wrap_callables=True)` → 적용 가능 멤버 attrset (B1 의존) | med | opaque 대신 래핑하는 플래그 | `host_module_to_pnix` L486 | P3 |
| B6 | module export **manifest + hash** (export 수, denied/private, drift-stable) | med | module 투영을 `dir()`-동적이 아닌 재현 가능하게 manifest 첨부 | `host_module_to_pnix` L486, `rt` hashing | **P3 (codex-added)** |

Cluster C — meta-circular ↔ meta-circular / macro-quote interop:

| # | 후보 | 가치 | 무엇을 추가 | 재사용 | codex |
|---|---|---|---|---|---|
| C1 | Hy 매크로를 pnix-투영 폼 위에 적용 | high | pnix→`pnix_to_hy_form`→named Hy macro/macroexpand 실행→(pnix 재합성) | `pnix_to_hy_form` L1410, `hy_macroexpand_projection` L804 | P2 |
| C2 | pnix 값을 Hy quasiquote 구멍에 주입 | high | pnix-value→`hy.models` 변환기(`model_to_json` 역), `~`/`~@` 구멍에 splice | `hy_quasiquote_projection` L1067/`find_holes` L997, `_value_to_hy` L2280 | — |
| C3 | `quasiquote` ↔ `specialize_pnix` 실행 가능 대응 | med-high | 산문 비유를 검사 가능하게(정적 골격↔folded, 구멍↔dynamic vars) | `hy_quasiquote_projection` L1067, `specialize_pnix` L2420 | — |
| C4 | Hy reader-macro가 read-time에 pnix 임베드(`#px "..."`) | med | `defreader`가 임베드 pnix 파싱 → compile 전 `hy.models` | `hy_reader_macro_projection` L1369, `rt.parse` | — |
| C5 | 더 풍부한 투영-drift 분류기 `classify_drift`(`#_pnix-gap`/`gaps`/`differs`를 관찰; 채우면 안 됨) | high | `correspondence_table` differs 행에 매인 안정 enum으로 재분류 | `pnix_to_hy_form`/`synthesize_pnix_from_hy`, `_roundtrip_status` L2865, `correspondence_table` L814 | **P1** |
| C6 | 명시적 roundtrip API + 공유 status-vocab 전역화 | high | 두 closure + `hy_to_pnix_value_roundtrip` + `specialization_roundtrip`를 `_roundtrip_status` 경유 | `_roundtrip_status` L2865, `ROUNDTRIP_STATUS_VOCAB` L24 | — |
| C7 | `reify_pnix`와 대칭인 `reify_hy`(재-envelope, 2번째 projector 없음) | med | 하나의 `{reified:{source,form,ast,ir,effect,value,witness}}` envelope | `reify_pnix` L2872, `hy_pnix_projection` L1077 | — |
| C8 | `interop_no_mirror_report` 불변식 게이트(mirror OFF에서 interop 동작) | med | `mirror_event` 의존 없이 to_host/from_host/call_host/opaque 동작 단언 | interop core | **P7 (codex-added)** |
| C9 | `stage7_projection_report` — typed Hy-stage7-seam↔pnix 투영 리포트 | med | Hy 구성 → stage7 커널 → pnix 투영 → 역투영 → witness | `hy_mirror` stage7 seam (`mirror_full_introspection`/`introspection_parity`) | **P8 (codex-added)** |

Boundary-ABI (proposal + 양-레인 변경 + gate drift-guard 필요):

| # | 후보 | 노트 | codex |
|---|---|---|---|
| D1 | 대칭 cross-boundary 에러/예외 계약 | host exc → pnix `throw`/`tryEval {success,value}`; host-facing 래퍼에서 `PnixError` 잡기(현재 `{'exception':str}` L393는 attrset과 구별 불가; raw `PnixError` L466 누출) | — |
| D2 | opaque-ref lifecycle 정책 | `_OPAQUE`의 lane-local weakref/finalizer는 in-scope; 공유 ref shape의 refcount는 ABI-조율 | P5 |
| D3 | `correspondence_table` → versioned 언어-interop ABI | 29-row taxonomy를 versioned artifact로 승격(source_node/lang/pnix_tag/value_type/loss/supported)해 pnix-hs/pnix-rs가 vocabulary 공유 | **P3/P7 (codex-added)** |
| D4 | interop role matrix 문서 | feature×owner×status×proposal 표로 에이전트가 의도적 gap을 다시 안 열게 | **P10 (codex-added)** |

## By-design gap — 구현 금지 (SCOPE_LOCK §3/§4)
- pnix 매크로 / quasiquote / reader-macro / `require` — pnix는 비동형; pnix 위에 작용하는 Hy-쪽
  매크로(C1/C2/C3/C4)만 정당하고, pnix-쪽 매크로는 절대 아님.
- `#_pnix-gap` 마커, derivation store hashing, trace/warn stderr 부작용 — C5는 관찰하지 채우지 않음.
- attrset/closure의 `to_host`(stable_data→sorted dict; Closure→opaque) — 현재대로 맞음.
- PnixPath/PnixString의 `realize_value` str-정규화 — canonical-data; `to_host` 표기(A6)만 제안
  가능, 정규화 자체는 아님.
- `wrap_pnix_callable`의 kwargs(pnix 함수는 curried unary), `explain_hy`(C7 reify_hy와 중복),
  named cache/sandbox(ops-ceremony) — triage에서 drop.

## Out of scope (별도 레인 / 더 큰 scope)
- codex P9 `HostArtifactInteropRecord`(Python AST/code/pyc/marshal) — 이건 **hy-meta** 호스트
  레인(SCOPE_LOCK §5/§6); pnix-hy interop이 아니라 hy-meta proposal.
- `reify_host`/`explain_host`/host cache/host `sandbox_run` — hy-meta 레인.
- Host bytecode/marshal/pyc drift(`classify_stage8_drift`) — 투영 drift와 다른 축.
- Cross-repo InteropRecord/refcount ABI 통일 — D1/D2/D3의 shared-shape 절반.

## 추천 다음 proposal (수락 시 `NNNN-*.md`로 작성)
1. ✅ **SHIPPED 2026-07-01** — `0001-roundtrip-host-value-and-loss-fidelity.md`. A1–A6 완료
   (`interop.roundtrip_host_value` + tuple/set/bytes/non-str-key/collision/path loss-marking);
   `roundtrip_report` 등록 → `--check` 45/45. C6(source-level roundtrip vocab) 여전히 후보.
   원 노트: 진단 하나 + 얇은 loss-marking 교정; 가치/노력비 최고, 전부 local, ABI 위험 없음.
2. ✅ **SHIPPED 2026-07-01** — `0002-host-callable-into-pnix-eval.md`. B1(`host_callable_to_pnix`,
   pnix 소스가 host callable 적용, capability-gated + curried) + B2(`call_host` kwargs) +
   B3(`host_callable_arity`) + B5(`host_module_to_pnix(wrap_callables=)`) + B4(call_host/
   call_host_method witness). `host_bridge_report` → `--check` 46/46. 원 노트: callable/module
   언어 도달범위의 핵심.
3. ✅ **SHIPPED 2026-07-01** — `0003-hy-macro-quasiquote-over-pnix.md`. C1(`hy_macro_over_pnix`,
   Hy 매크로를 pnix-투영 폼 위에, 확장을 pnix로 재합성) + C2(`hy_quasiquote_over_pnix`, pnix 값이
   Hy quasiquote 구멍 채움) + C3(`quasiquote_specialize_correspondence`, 실행 가능 staging 대응) +
   primitive `hy_mirror.hy_eval_form`. `hy_macro_quasiquote_over_pnix_report` → `--check` 47/47.
   원 노트: SCOPE_LOCK가 명명한 두 방향; 이전 커버리지 0, 정확히 on-mission.

4. ✅ **SHIPPED 2026-07-01** — `0004-interop-diagnostics-and-invariants.md`. C5(`classify_drift`,
   투영 gap을 안정 enum으로 재분류) + C7(`reify_hy`, reify_pnix와 대칭인 Hy-쪽 reification) +
   C8(`interop.no_mirror_report`, mirror-OFF에서 interop 동작 불변식). `--check` 50/50
   (classify_drift, reify_hy, interop_no_mirror).

5. ✅ **SHIPPED 2026-07-01** — `0005-hy-reader-macro-embeds-pnix.md`. C4: Hy `#px "..."` reader
   macro(`hy_mirror.hy_read_with_pnix_reader`)가 READ 시점에 pnix 임베드; pnix-hy가 의미 부여
   (`pnix_mirror.hy_reader_embed_pnix`). `--check` 51/51.
   **C9 DECLINED** — ceremony: `pnix_meta_circular_projection`이 이미 Hy-stage7 ↔ pnix seam(4
   substrate)을 커버; 별도 리포트는 중복.

6. ✅ **SHIPPED 2026-07-01** — `0006-interop-error-contract-and-role-matrix.md`. D1
   (`is_interop_error`/`try_call_host`/`InteropError` — 애매성 없는 cross-boundary 에러, pnix-쪽
   전용, 공유 ABI 무변경) + D4(`docs/INTEROP_ROLE_MATRIX.md`). `--check` 52/52.

7. ✅ **SHIPPED 2026-07-01** — `0007-opaque-lifecycle-and-correspondence-abi.md`. D2 in-scope
   (`interop.opaque_lifecycle` lane-local lifecycle + leak count, ref shape 불변) + D3 in-scope
   (`pnix_mirror.correspondence_abi` content-hashed versioned artifact). `--check` 54/54.

**모든 in-scope + doc-only 카탈로그 항목 shipped(0001–0007).** 남은 것은 진짜 cross-lane ABI
잔여물뿐: **D2 공유 ref shape의 refcount**와 **D3 cross-repo vocabulary 통일**은 hy-meta + pnix-hy +
gate drift-guard를 *함께* 바꿔야 하며(현실적으로 pnix-hs/pnix-rs가 먼저 존재해야). A7(opaque-ref
passthrough)는 low-value. 전용 양-레인 proposal만.

## 교차대조 판정 (Claude scout ↔ codex P1–P10)
둘 다 수렴: 필수 미구현 없음; interop 실재; 남은 것 = 선택적 언어-interop 품질. 일치: 매크로-투영/
witness(C1/C4/B4≈P2), gap-ledger(C5≈P1), callable witness(B3/B4≈P4), module manifest(B6≈P3),
opaque lifecycle(D2≈P5), no-mirror gate(C8≈P7), correspondence ABI(D3≈P3/P7), stage7 report(C9≈P8),
role matrix(D4≈P10). Claude scout는 추가로 구체적 loss-marking 결함(A2/A3/A4/A5)과 codex가 "witness
binding"으로만 프레이밍한 두 매크로 DIRECTION(C1/C2)을 짚음. codex P9(host artifact envelope)는
hy-meta-레인(pnix-hy interop scope 밖)으로 올바로 재분류됨.
