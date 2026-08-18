# 27-reflect-tower-coherence — reify/reflect 2단계 정합 + 메타-레벨 투명성

## 쉽게 말하면 (비유)
`11-self-hosting-tower`가 "코드=데이터로 자기 자신을 해석할 수 있다"를
보인다면, 이건 그 위에 한 단을 더 쌓는다: 코드를 데이터로 올린(reify)
것을 **다시** reify하고 다시 reflect(내려)봐도 원래대로 돌아오는가
(레벨 2 정합), 그리고 그렇게 레벨을 오르내려도 **의미가 안 변하는가**
(메타-레벨 투명성)를 본다.

## 무엇을
(1) `reflect ∘ reify = id`(레벨 1 — 기본 왕복), (2) 인코딩 자체를 다시
reify/reflect 해도 왕복(레벨 2 정합 — 탑이 well-founded), (3) **메타-레벨
투명성**: reify된 프로그램을 px 자기해석기로 평가한 값이 네이티브
평가값과 같음(레벨을 옮겨도 뜻이 안 변함).

## plain Rust의 한계 (`limit_rust.rs`)
Rust 매크로/`syn`으로 AST를 데이터로 다룰 수는 있지만, "그 데이터를
다시 데이터로 올려도 정합적인가", "레벨을 옮겨 평가해도 원래 의미와
같은가"를 검사하는 탑(tower) 정합성 개념이 없다.

## pnix-rs의 방식 (`pnix_rs_way.sh`)
- `pnix-rs reflect-tower-check` — 레벨 1/2 reify-reflect 정합 +
  메타-레벨 투명성(레벨 이동이 의미보존)

## 어디에 쓰나
자기해석기/매크로 시스템이 여러 단(meta-level)을 오르내릴 때도 신뢰할
수 있음을 보이는 근거. `11-self-hosting-tower`의 심화판.

## 실행
```sh
rustc -O examples/27-reflect-tower-coherence/limit_rust.rs -o /tmp/limit_27-reflect-tower-coherence && /tmp/limit_27-reflect-tower-coherence
bash examples/27-reflect-tower-coherence/pnix_rs_way.sh
```
