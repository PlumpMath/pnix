# 0005 — 잘-타입된 residual 게이트 (Rust 정적 강점)

상태: **DONE (2026-07-03)** — welltyped-check 5/5, aggregate 19 reports.
rs-meta에 `typecheck -c|-f`(pnix-free 범용) 추가, pnix-rs RustReconRecord에
well_typed(플로어 인증) 필드 + welltyped-check 게이트(factorial/add3/struct/
generic 재구성이 플로어 typeck로 well-typed + ill-typed 거부). typeck-check가
"플로어 accepts iff rustc"를 보증하므로 native tier 없이 well-typed 인증.
근거: deep-research finding [6] (Brown & Palsberg, POPL'18), 
docs/research/2026-07-03-metacircular-frontier.md.

## 동기 (언어별 meta-circular 잠재력의 차이)
동적 Lisp(pnix-hy) meta-circular은 residual의 타입-정합을 싸게 보증 못 한다.
Rust 정적 기판(rs-meta에 typeck 존재)은 **모든 px→Rust residual이 구성상
타입-정합(rustc-typeable)임을 게이트**할 수 있다 — Brown&Palsberg가 "타입이
residual의 타입-정합성을 보장하는 최초의 (typed+Jones-optimal+self-applicable)
PE"라 한 성질의 pnix-rs판.

## 접근
P6 rust_mirror의 px→Rust 재구성 residual을 rustc에 넘기기 전에 **rs-meta의
자체 typeck로 검증** — "rustc가 우연히 받아준다"에서 "우리 typeck가 well-typed임을
증명한다"로 격상. 나아가 사영의 **type-preservation**(px 타입 → Rust 타입 보존)을
게이트.

## 모듈/게이트
typeck/rust_mirror. 게이트 = 모든 재구성 residual이 rs-meta typeck 통과(well-typed
residual) + px→Rust type-preservation. 동적 Lisp meta-circular이 못 주는 정적 보증.

## 위험
중간. rs-meta typeck를 pnix-rs에서 호출(ast-canonical류 CLI 필요할 수 있음) +
px 타입 개념 정립. 하지만 Rust 방식의 정수라 payoff 높음.
