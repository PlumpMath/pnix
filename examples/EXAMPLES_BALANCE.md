# 예제 균형 (다섯 호스트)

각 호스트는 `pnix-<host>/pnix-<host>/examples/` 를 **자기 소유**로 둔다.
카탈로그는 호스트 바인딩이다(공유 multi-host 코퍼스가 아님).
호스트에 **실재하는 표면**이 있을 때만 같은 *테마*를 두고, 기둥(pillar)이 다르면
번호·제목은 호스트마다 달라도 된다.

## 규모 스냅샷

| 호스트 | 제품 예제 루트 | 카탈로그 깊이 (대략) | 비고 |
|--------|----------------|----------------------|------|
| **clj** | `pnix-clj/pnix-clj/examples/` | ~90개 슬라이스 | 가장  densest: spine, machine, oracle, AI gate |
| **hy** | `pnix-hy/pnix-hy/examples/` | ~35 | specialize, cogen, compartment, Jones 등 |
| **rs** | `pnix-rs/pnix-rs/examples/` | ~15 | 중간: gate, mirror, BTA, embed |
| **cljs** | `pnix-cljs/pnix-cljs/examples/` | 코어 00–06 | experimental seed; Node 라이브러리 import |
| **clr** | `pnix-clr/pnix-clr/examples/` | 코어 00–06 | experimental seed; C# 라이브러리 + in-process opt-in |

모노레포 host-import 스모크(제품 카탈로그 아님): `examples/host-import/`.

## 공유 테마 표 (의도만 공통, 구현은 호스트별)

테마는 **사람용 라벨**이다. 모든 호스트가 같은 API·패리티를 갖는다는 주장이 아니다.
빈 칸 = 아직 카탈로그 없음 **또는** 호스트가 그 표면을 인정하지 않음.

| 테마 | clj | hy | rs | cljs | clr |
|------|-----|----|----|------|-----|
| 파운데이션 (eval seed) | 00 | 00 | 00 | 00 | 00 |
| 순수 / fail-closed 평가 | 01 | 01 | 01 | 01 | 01 |
| 결정성 / 해시 / drift | 13–15, 21 | 02 | 02 | — | — |
| 호스트 라이브러리 import | host-import + 51 | 14 | 15 | **02** | **02** |
| 호스트↔pnix interop / embed | 04, 07–08 | 04, 07–08 | 04, 15 | **04** | **04** |
| 결과 모양 / 영수증 정직 | 02, 05 | 05 | 05 | **03, 05** | **03, 05** |
| Specialize / Futamura | 03, 33 | 03, 33 | 06 | — | — |
| Self-host / meta 쌍 | 11, 35 | 11, 35 | 11 | **06** | **06** |
| Compartment / capability | 23, 31 | 23, 31 | 08 | — | — |
| Cache / incremental | 12, 30 | 12, 22, 30 | 07 | — | — |
| Machine / abstract CEK | 61, 78–92 | 35 | — | — | — |
| In-process 호스트 eval | — | — | — | — | **05** (opt-in) |

## 균형 규칙

1. **개수를 맞추려고** clj/hy 연구 슬라이스를 cljs/clr/rs에 **복제하지 않는다**.
2. 모든 호스트에 읽을 수 있는 **00–0N 코어 경로**는 유지한다:  
   foundation → sandbox → host import → outcome → embed → honesty → meta 경계.
3. 호스트 전용 깊은 카탈로그는 **실행 가능한 표면 + README**가 있을 때만 늘린다.
4. 성숙도가 다르다: cljs/clr 예제는 **experimental** 을 밝히고, 인정하지 않은
   주장(Stage15/N, 다섯 호스트 의미 패리티 등)은 fail-closed로 적는다.

## 호스트별 시작점

| 호스트 | 진입 |
|--------|------|
| clj | `pnix-clj/pnix-clj/examples/START_HERE.md` |
| hy | `pnix-hy/pnix-hy/examples/README.md` + `FOUNDATION_PATH.md` |
| rs | `pnix-rs/pnix-rs/examples/README.md` + `FOUNDATION_PATH.md` |
| cljs | `pnix-cljs/pnix-cljs/examples/README.md` + `FOUNDATION_PATH.md` |
| clr | `pnix-clr/pnix-clr/examples/README.md` + `FOUNDATION_PATH.md` |

최종 갱신: 2026-08-14 (cljs/clr 코어 카탈로그 + 매트릭스, 본문 한글).
