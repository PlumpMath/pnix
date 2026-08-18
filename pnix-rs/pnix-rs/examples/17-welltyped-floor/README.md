# 17-welltyped-floor — meta-circular 플로어가 정타입성을 증명

## 쉽게 말하면 (비유)
"컴파일이 됐다"는 "`rustc`가 우연히 받아줬다"는 뜻일 뿐이다. welltyped-check는
**meta-circular 플로어(자기 자신을 해석하는 typeck)**가 같은 프로그램을
받아주는지를 검사해서, "정타입"이라는 주장을 신뢰된 밑바닥에서 인증한다.

## 무엇을
Jones-최적 residual(예제 16)을 플로어(자체) typeck에 다시 통과시켜
well-typed임을 확인 — "rustc가 받아줬다"가 아니라 "신뢰된 meta-circular
바닥에서 정타입임이 증명됨"이 된다. 그리고 **음성 사례**: 일부러 틀린 타입의
Rust 프로그램은 플로어 typeck가 거부한다(게이트에 이빨이 있음). 이 정적
보장은 Rust/정적타입 쪽의 고유한 엣지다 — 동적 Lisp meta-circular(clj/hy)는
이런 컴파일-타임 보장을 값싸게 제공할 수 없다.

## plain Rust의 한계 (`limit_rust.rs`)
`rustc`는 타입을 검사하지만, 그 판정을 **재검토 가능한 증거**로 남기지
않는다. "이 프로그램이 왜 정타입인가"를 독립된 신뢰 기반에서 재확인할
방법이 없다 — rustc 자신을 다시 믿는 수밖에 없다.

## pnix-rs의 방식 (`pnix_rs_way.sh`)
- `pnix-rs welltyped-check` — 플로어 typeck로 정타입성 재인증(양성) +
  일부러 틀린 프로그램 거부(음성/이빨)

## 어디에 쓰나
Rust 고유의 정적 보장을 meta-circular 증명 체인에 편입 — 컴파일 성공을
"주장"이 아니라 "재검사 가능한 증거"로 바꾼다.

## 실행
```sh
rustc -O examples/17-welltyped-floor/limit_rust.rs -o /tmp/limit_17-welltyped-floor && /tmp/limit_17-welltyped-floor
bash examples/17-welltyped-floor/pnix_rs_way.sh
```
