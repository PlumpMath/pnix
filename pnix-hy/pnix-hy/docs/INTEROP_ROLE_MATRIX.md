# Interop 역할 매트릭스 (D4)

기능 × 소유자 × 상태 × proposal — 에이전트가 의도적 gap을 다시 열지 않도록. 소유 레인은
`/SCOPE_LOCK.md` §5/§6 기준: **hy-meta** = 호스트 자기컴파일/평가/재현 proof 레인; **pnix-hy** =
pnix 런타임 + Hy↔pnix 투영; **interop** = 명시적 경계(pnix-hy 소유).

| 기능 | 소유 | 상태 | Proposal / 심볼 |
|---|---|---|---|
| Hy 매크로 확장 기계 | hy-meta 호스트 레인 (Hy) | 존재 / 증명됨 | `hy_mirror.hy_macroexpand_projection` |
| Hy 매크로 투영 → pnix | pnix-hy | 존재 | `hy_mirror.hy_macro_step_trace` (투영; witness 필드 없음) |
| **pnix-쪽 매크로 / quasiquote / reader-macro** | — | **의도적 GAP — 구현 금지** | SCOPE_LOCK §3/§4; `hy_mirror._QUASIQUOTE_PNIX_NOTE` |
| Hy 매크로를 pnix-투영 폼 위에 | pnix-hy | shipped 0003 | `pnix_mirror.hy_macro_over_pnix` (C1) |
| pnix 값을 Hy quasiquote 구멍에 | pnix-hy | shipped 0003 | `pnix_mirror.hy_quasiquote_over_pnix` (C2) |
| quasiquote ↔ specialize staging 대응 | pnix-hy | shipped 0003 | `pnix_mirror.quasiquote_specialize_correspondence` (C3) |
| Hy `#px` reader macro가 pnix 임베드 (read-time) | pnix-hy | shipped 0005 | `hy_mirror.hy_read_with_pnix_reader` (C4) |
| Python AST / code-object / pyc / marshal artifact | hy-meta 호스트 레인 | 존재 | `hy-meta/host_exec.py`, `host_introspect.py` |
| Python AST 투영 → pnix | pnix-hy | 존재 | `pnix_mirror.synthesize_pnix_from_hy` |
| 값 interop (to_host / from_host) | interop | 존재 + loss-fidelity 0001 | `interop.to_host/from_host`, `roundtrip_host_value` (A1–A6) |
| pnix 소스에서 host callable 호출 | interop | shipped 0002 | `interop.host_callable_to_pnix` (B1, host-call 게이트) |
| host callable / method 호출 | interop | 존재 + kwargs 0002 | `interop.call_host(kwargs=)`, `call_host_method` |
| host-callable arity → functionArgs | interop | shipped 0002 | `interop.host_callable_arity` (B3) |
| pnix callable 래퍼 (host-facing) | interop | 존재 + typed error 0006 | `interop.wrap_pnix_callable` → `InteropError` |
| module 투영 (+ callable) | interop | 존재 + wrap_callables 0002 | `interop.host_module_to_pnix(wrap_callables=)` (B5) |
| opaque-ref 레지스트리 + method-level | interop | 존재 | `interop.make_opaque_ref/resolve_opaque/call_host_method` |
| capability / effect 게이트 | pnix-hy | 존재 | `gate.gate_check`, `interop.check_capability` |
| 결정적 witness | 공유 §14 envelope | 존재 (drift-guarded) | `gate.make_witness`, `gate.gate_report:witness_schema_ok` |
| 투영-drift 분류기 | pnix-hy | shipped 0004 | `pnix_mirror.classify_drift` (C5) |
| Hy-쪽 reification (대칭) | pnix-hy | shipped 0004 | `pnix_mirror.reify_hy` (C7) |
| mirror OFF에서도 interop 동작 | interop | shipped 0004 (불변식) | `interop.no_mirror_report` (C8) |
| cross-boundary 에러 계약 | interop | shipped 0006 | `interop.is_interop_error/try_call_host` (D1) |
| **공유 shape의 opaque-ref lifecycle (D2)** | 공유 envelope | 후보 — 양-레인 + drift-guard | `0000` |
| **versioned correspondence ABI (D3)** | 공유 envelope | 후보 — 양-레인 + drift-guard | `0000` |
| host artifact interop envelope (codex P9) | hy-meta 호스트 레인 | pnix-hy interop scope 밖 | 자체 hy-meta proposal |

규칙: **의도적 GAP** 또는 **scope 밖**으로 표시된 행은 "미구현 작업"이 아니다. 새 행은
`docs/proposals/NNNN-*.md`(SCOPE_LOCK §7)로 들어오지, 절대 `todo.md [ ]`로 들어오지 않는다.
