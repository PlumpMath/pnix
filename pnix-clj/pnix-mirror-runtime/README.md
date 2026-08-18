# pnix-mirror-runtime

> **`.px` 프로젝트, pnixc-meta로 빌드.** Rust 크레이트 아님.
>
> 2026-06-02 용어 가드: 이 디렉터리는 pnixc-meta mirror 프리미티브 `.px` 표면입니다.
> 폐기된 제품 라벨로 설명하지 마세요. 호스트는 bootstrap/transport만 담당하고,
> 의미 프리미티브 법은 `.px`에 남습니다.

## 이것의 정체

`.px`로 재구현한 기판 실행 계층입니다. 2026-05-13 walking 실험
(R10/R7/R8/R9)이 현재 기판에 없다고 확인한 4개의 mirror-spawn 프리미티브를
호스팅합니다.

- **P1 mirror-identity-registry** — mirror spawn마다 typed·replay-stable 정체성
- **P2 ontology-extension-event** — mirror 탄생 시 명시적 온톨로지 delta
- **P3 boundary-projection** — in-mirror live vs out-of-mirror reference shadow
- **P4 typed-receipt-stream** — 모든 프리미티브 이벤트 위 append-only 감사 체인

## 이것이 Rust `pnix-query-runtime`을 대체하는 이유

OWNER-LAW CONSTITUTION에 따르면:
> semantic / ontology / learning / floor / coverage / provenance /
> lineage law 는 Pnix/.px owner 로만 구현한다.

4개 프리미티브는 *기판 의미 법*입니다. Rust 코드로 살 수 없고, 메타순환 기판이
자신을 추론할 수 있도록 `.px`에 살아야 합니다. Rust 크레이트
`pnix-query-runtime`은 이 `.px` 프로젝트가 `pnixc-meta` end-to-end로 빌드
가능해질 때까지 과도기 bootstrap 호스트로 남습니다.

## 빌드 경로

```
pnixc (Rust stage 0)
  │ builds pnixc-pnix/*.px → stage1
  ▼
pnixc-meta (stage 2, self-hosted)
  │ builds this directory →
  ▼
pnix-mirror-runtime artifact
  │ host calls (analogous to `turn-exec interpreter.px`)
  ▼
mirror sandbox active — P1-P4 enforce primitives, the 4 axes
(in-mirror vs out / depth / sibling) become observable via P4
receipt stream
```

## 레이아웃

```
pnix-mirror-runtime/
  ├── README.md
  ├── project.px                          # pnixc-meta entry point
  └── primitives/
      ├── p1-mirror-identity-registry.px
      ├── p2-ontology-extension-event.px
      ├── p3-boundary-projection.px
      └── p4-typed-receipt-stream.px
```

## Live-Coding 평가 규칙

Mirror 프리미티브 개발은 코딩하면서 플랫폼을 실행해야 합니다. 정본
interpreter 대상은 `pnixc-meta`입니다. 이 프로젝트는 mirror 런타임 표면을
최종 Rust helper API가 아니라 pnixc-meta로 빌드된 `.px` 프로젝트로 평가하기
위해 존재합니다. 프리미티브가 새 unknown / Held / trace / verdict / boundary
형태를 드러내면, 그 관찰은 슬라이스가 닫히기 전에 테스트, harness 검사,
inventory 행, gate, 또는 명시적 Held 경로가 되어야 합니다.

직접 `pnix-query-px-eval` / Rust `eval_to_json` 호출은 bridge-debt 관찰
경로입니다. 일반 mirror dispatch 연산
(`applyMirrorPlateWithLensDispatch`)에 대해 pnixc-meta는 이미 정본 대상이며,
새 증명은 pnix-eval을 정본으로 다루지 말아야 합니다. 남은 직접 호출자는
`pnix-zero` 이전 저장소의 project-wiki에 있는
`maps/non-mirror-px-pnixc-meta-migration-plan.md`의 2026-05-17 수렴 계획이
추적하던 caller-cleanup debt입니다 (그 저장소는 이 자기완결 트리의
일부가 아님).

## Ontology-Example 업그레이드 기준

프리미티브 export가 평가된다고 해서 런타임이 완성된 것은 아닙니다. 다음
효과 기준은 `ontology-examples.md`의 결정적 온톨로지 베이스라인을 mirror
계산으로 재연하는 것입니다.

```text
meaning atom -> mirror plate -> runtime function -> receipt / trace -> next turn
```

그 경로는 `pnix-zero` 이전 저장소의 project-wiki에 있는
`maps/ontology-examples-to-mirror-meta-interpreter-map.md`에서 추적했습니다.

## 교차 참조

아래는 모두 `pnix-zero` 이전 저장소의 project-wiki에 있던 문서이며, 이
자기완결 트리의 일부가 아닙니다 (역사적 설계 근거로만 참고):

- `maps/ankh-macro-mirror-turn-axiom.md` — 어휘
- `maps/mirror-spawn-substrate-4-primitives-design.md` — 프리미티브 설계 초안
- `maps/mirror-upgrade-experiment-routes.md` — 이 재작성을 촉발한 walking 결과
- `maps/ontology-examples-to-mirror-meta-interpreter-map.md` — ontology examples를 넘어서는 효과 기준
