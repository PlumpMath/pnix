# 04-rust-pnix-projection — Rust ↔ px 사영 — 이 도구의 심장

## 쉽게 말하면 (비유)
pnix-rs의 핵심은 **Rust와 px를 서로 사영**하는 것이다.
px 값을 **Rust 프로그램으로 사영**해 실제 substrate(rs-meta 인터프리터·rustc)에서 돌려
**3-way 동등**을 증명하고, 거꾸로 Rust 소스를 **px 트리로 물화**해 다시 Rust로 **재구성**하면
rs-meta 자신이 판정하는 **AST 동일성 + rustc 정합**까지 확인한다.

## 무엇을
① px 값 → 네이티브 Rust 리터럴 프로그램 → (rs-meta interp == rustc == px canonical) 3-way
② Rust 소스 → ast-canonical → px 트리 → Rust 재구성(ast-canonical 동일 + rustc 정합).

## plain Rust의 한계 (`limit_rust.rs`)
Rust와 다른 표현(px) 사이의 값/AST 사영을 **substrate로 증명**하는 표준 수단이 plain Rust엔 없다.
serde도 '왕복이 의미를 보존하고, 두 실행기가 합치하는가'까지는 증명하지 않는다.

## pnix-rs의 방식 (`pnix_rs_way.sh`) — Rust ↔ pnix
- `pnix-rs rust-mirror -c '<px>'` — px 값을 Rust로 사영해 substrate 3-way lossless
- `pnix-rs rust-mirror-check` — 값 축 3-way + **AST 축**(Rust→px→Rust 재구성, AST 동일성)

## 어디에 쓰나
언어 간 이식(Rust↔px), 컴파일러/인터프리터 합치 검증, 신뢰 가능한 코드 생성.

## 경계 — 왜 "opaque host 참조"가 없나
pnix-hy(Python)는 host 객체를 opaque 참조로 감싸 pnix 쪽에 노출하는 SES식
생명주기(`make_opaque_ref`/`lend_opaque`/`harden_opaque`)를 갖는다. pnix-rs는
**의도적으로** 그게 없다 — `interop.rs`의 불변식: "`PxVal`은 host-object
variant를 갖지 않는다. host 결과는 항상 이 경계를 **순수 데이터(문자열)**로
건넌다. opaque host handle이 필요해 보이면 그건 새 variant가 아니라
**held boundary이자 제안**이다." 살아있는 host 객체 참조를 pnix 쪽에 노출하지
않는 게 이 lane의 설계 경계이지, 빠진 기능이 아니다.

## 실행
```sh
# plain Rust의 한계 (직접 컴파일·실행)
rustc -O examples/04-rust-pnix-projection/limit_rust.rs -o /tmp/limit_04-rust-pnix-projection && /tmp/limit_04-rust-pnix-projection

# pnix-rs 방식 (flake로 설치했거나 PATH에 pnix-rs가 있으면 그대로,
#              아니면 PNIX_RS=/path/to/pnix-rs 로 지정)
bash examples/04-rust-pnix-projection/pnix_rs_way.sh
```
