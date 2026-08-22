# 예제 균형 (다섯 호스트)

각 호스트는 `pnix-<host>/pnix-<host>/examples/` 를 **자기 소유**로 둔다.
카탈로그는 호스트 바인딩이다(공유 multi-host 코퍼스가 아님).
호스트에 **실재하는 표면**이 있을 때만 같은 *테마*를 두고, 기둥(pillar)이 다르면
번호·제목은 호스트마다 달라도 된다.

## 규모 스냅샷

| 호스트 | 제품 예제 루트 | 카탈로그 깊이 (대략) | 비고 |
|--------|----------------|----------------------|------|
| **clj** | `pnix-clj/pnix-clj/examples/` | ~94개 슬라이스 | 가장 densest: spine, machine, oracle, AI gate, live oracle, mirror-pair corpus |
| **hy** | `pnix-hy/pnix-hy/examples/` | ~43 | specialize, cogen, compartment, Jones, stage-ladder, receipt, perf, Hy 구성체 프로젝션, meta-circular tower, 양방향 closure, 호스트 콜러블 호출, opaque 참조 생명주기 등 |
| **rs** | `pnix-rs/pnix-rs/examples/` | ~28 | 중간: gate, mirror, BTA, embed, Jones/welltyped/cogen/attest/verifying-cache/ir-diff/attenuate/tower-depth/project-health |
| **cljs** | `pnix-cljs/pnix-cljs/examples/` | 코어 00–17 + readiness | Node 라이브러리 import + Stage15 fixed-point 조합 |
| **clr** | `pnix-clr/pnix-clr/examples/` | 코어 00–17 + readiness | C# 라이브러리 + Compiler Stage15/N 조합 |

모노레포 host-import 스모크(제품 카탈로그 아님): `examples/host-import/`.

## 공유 테마 표 (의도만 공통, 구현은 호스트별)

테마는 **사람용 라벨**이다. 모든 호스트가 같은 API·패리티를 갖는다는 주장이 아니다.
빈 칸 = 아직 카탈로그 없음 **또는** 호스트가 그 표면을 인정하지 않음.

| 테마 | clj | hy | rs | cljs | clr |
|------|-----|----|----|------|-----|
| 파운데이션 (eval seed) | 00 | 00 | 00 | 00 | 00 |
| 순수 / fail-closed 평가 | 01 | 01 | 01 | 01 | 01 |
| 결정성 / 해시 / drift | 13–15, 21 | 02, 29 | 02, 21 | — | — |
| 호스트 라이브러리 import | host-import + 51 | 14 | 15 | **02** | **02** |
| 호스트↔pnix interop / embed | 04, 07–08 | 04, 07–08, 42–43 | 04, 15 | **04** | **04** |
| 결과 모양 / 영수증 정직 | 02, 05 | 05, 37 | 05, 19, 28 | **03, 05** | **03, 05** |
| Specialize / Futamura | 03, 33 | 03, 33 | 06, 16, 18, 23 | — | — |
| Self-host / meta 쌍 | 11, 35 | 11, 35, 36, 40, 41 | 11, 26, 27 | **06** | **06** |
| Compartment / capability | 23, 31 | 23, 31 | 08, 22 | — | — |
| Cache / incremental | 12, 30 | 12, 22, 30 | 07, 20 | — | — |
| Machine / abstract CEK | 61, 78–92 | 35 | 26, 27 | — | — |
| In-process 호스트 eval | — | — | — | — | **05** (opt-in) |
| builtins / 문법 seed | 다수 | 다수 | gate·eval | **07–09, 11–16** | **07–08, 11–16** |
| 파일·artifact 게이트 | eval-file 등 | import hook | embed | **08** | **09–10** |
| pnix-meta 착수 전 공통 floor | **production-readiness** | **production-readiness** | **production-readiness** | **production-readiness** | **production-readiness** |

## 균형 규칙

1. **개수를 맞추려고** clj/hy 연구 슬라이스를 cljs/clr/rs에 **복제하지 않는다**.
2. 모든 호스트에 읽을 수 있는 **00–0N 코어 경로**는 유지한다:  
   foundation → sandbox → host import → outcome → embed → honesty → meta 경계  
   → (가능하면) builtins / 파일·CLI / 호스트 전용 게이트.
3. 호스트 전용 깊은 카탈로그는 **실행 가능한 표면 + README**가 있을 때만 늘린다.
4. 연구 카탈로그의 깊이는 여전히 다르다. 그러나 다섯 호스트 모두
   `production-readiness`에서 direct runtime, 같은 `.px` import,
   PNIX-in-PNIX, host import, host-meta 조합이라는 공통 floor를 실행한다.
   이것을 다섯 호스트 전체 의미 패리티나 공통 callback ABI로 확대 해석하지 않는다.

## 호스트별 시작점

| 호스트 | 진입 |
|--------|------|
| clj | `pnix-clj/pnix-clj/examples/START_HERE.md` |
| hy | `pnix-hy/pnix-hy/examples/README.md` + `FOUNDATION_PATH.md` |
| rs | `pnix-rs/pnix-rs/examples/README.md` + `FOUNDATION_PATH.md` |
| cljs | `pnix-cljs/pnix-cljs/examples/README.md` + `FOUNDATION_PATH.md` |
| clr | `pnix-clr/pnix-clr/examples/README.md` + `FOUNDATION_PATH.md` |

2026-08-22 갱신: 다섯 호스트에 byte-identical `production-readiness` 예제를
추가했다. cljs-meta의 15-generation fixed point와 clr-meta의 Compiler
Stage15/N은 이제 별도 meta 증거로 닫혀 있으므로 두 호스트를 experimental
seed라고 부르거나 Stage15/N 자체를 비주장으로 두던 문구를 제거했다. 제품과
meta의 identity는 계속 분리하며, callback/opaque ABI 패리티는 주장하지 않는다.

이전 최종 갱신: 2026-08-18 (rs: `pnix-rs check`의 34개 등록 게이트 중 아직 예제
없던 16개 — jones/welltyped/cogen/attest/verifying-cache/ir-diff/attenuate/
phase+assumption/certify/cross-host/stage/reflect-tower/explain+
capabilities+registry — 를 16–28로 채움. hy: `pnix_stage_ladder`/
`eval_receipt`/`performance_report` 세 CLI 표면(`--stage-ladder`/
`--receipt`/`--perf`)에 예제가 없던 것을 36–38로 채움. clr/cljs: 각 제품
런타임의 클로저(커링+캡처 재사용)에 예제가 없던 것을 16으로 채움 —
당시 clr-meta/cljs-meta 내부(Stage15/N)는 제품 주장으로 승격하지 않았음. 규칙
1·3에 따라 clj/hy 슬라이스 복제가 아니라 각 host 자신의 실행 가능한
표면만 반영 — 5개 host 전부 이번 감사 완료).

2차 감사 (같은 날): rs `02`가 `ir` 명령으로 개념만 보여주고 실제
`ir-check` 게이트는 안 부르던 것을 보강(34개 게이트 완전 커버). hy:
CLI 플래그(`--defmacro`/`--import`/`--macro-steps`/`--quasiquote`/
`--reader-macro`/`--tower`/`--synth-pnix`/`--closure`/`--hy-closure`)를
뒷받침하는 함수 8개가 예제 전무였던 것을 39–41로 채움. clr/cljs: 이번
세션에 새로 생긴 `--repl`이 예제 없던 것을 17로 채움. clj: 생성
capabilities 인덱스(`docs/CAPABILITIES.md`)의 report-artifact kind
전수 대조로 `live-oracle`(실제 nix-instantiate와의 실시간 차분 비교)과
`mirror-pair`(199-소스 코퍼스 전체 4-레인 수렴 집계 — `72`는 단일
소스 디버그 뷰라 범위가 다름)가 예제 없던 것을 93–94로 채움; 나머지
kind는 전부 기존 예제가 namespace require로 실제 커버 중임을 확인.

3차 감사 (같은 날): hy `pnix_hy.__all__`(공개 interop API) 전수 대조로
"값 변환"(04)과는 다른 두 축 — 호스트 함수/메서드 **호출**
(`call_host`/`call_host_method`/`try_call_host`/`host_callable_arity`/
`host_module_to_pnix`/`to_host_eval`)과 SES식 **opaque 호스트 참조
생명주기**(`make_opaque_ref`/`opaque_allowed_methods`/`lend_opaque`/
`harden_opaque`/`declare_opaque_invariants`, private 메서드 미노출까지
검증)가 예제 전무였던 것을 42–43으로 채움(둘 다 Hy 불필요, bare
python3로 확인). hy의 "Self-check reports"(`--capabilities`) 73개
목록은 대부분 이미 커버된 capability의 내부 회귀 테스트 wrapper임을
확인하고 오탐으로 제외.
