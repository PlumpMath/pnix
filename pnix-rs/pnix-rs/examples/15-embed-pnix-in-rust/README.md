# 15 · embed pnix in Rust — 호스트 소스에 pnix 임베드(read-time 승격)

> 형제 프로젝트의 canonical **08**(`08-hy-reader-embed-pnix`, `08-clojure-reader-or-edn-embed-pnix`)에
> 대응. 거기선 Hy `#px`/Clojure EDN reader가 pnix를 read-time에 1급 폼으로 임베드한다.
> Rust엔 reader가 없고 `macro_rules!`도 rs-meta에서 held이므로, pnix-rs는 rs-meta의
> **Rust 코드생성**(`rust-mirror`)을 그 자리에 놓아 언어-수준 interop을 세운다.

## 쉽게 말하면 (비유)
글 속에 외국어 문장을 인용부호로 넣었을 때, plain Rust는 그걸 **그냥 문자열**로만 둔다
(읽어도 뜻이 없고, 오타가 있어도 통과). pnix-rs 방식은 그 문장을 **읽는 순간 번역·검증**한다:
평가해서 뜻을 주고(→42), Rust로 옮겨 두 실행기(interp==rustc)가 같은 뜻인지 게이트한다.

## 무엇을
호스트 **Rust 소스**에 pnix 스니펫(`let base = 6; in base * 7`)을 임베드한 채, 그 임베드된
pnix에 **의미**를 준다 — pnix는 비동형 그대로, 두 언어를 언어 수준에서 잇는다(pnix에
reader/macro를 만드는 것이 **아니다**).

## plain의 한계 (`limit_rust.rs`, `rustc`로 컴파일·실행됨)
- 임베드된 pnix는 `&str` = **죽은 텍스트**: read-time 폼 승격 없음.
- 컴파일 시점에 pnix 문법/타입이 **검사되지 않는다**(오타 `base * `도 Rust는 통과).
- std에 pnix 인터프리터가 없어 **평가할 수단이 없다**.
- Rust로의 사영이 **의미를 보존하는지 증명**할 방법이 없다.

## pnix-rs의 방식 (`pnix_rs_way.sh`, 실제 `pnix-rs` CLI)
1. `px-eval` — 임베드된 pnix를 **평가**(→ 42): read-time 승격 = 의미 부여.
2. `rust-mirror` — px 값을 **Rust print-program으로 사영**하고 substrate에서
   `interp == rustc == native` 3-way로 실행(lossless). rs-meta의 Rust 코드생성이
   "매크로 확장/reader" 역할 — Rust 고유 meta-circular(자기 자신이 Rust-in-Rust 컴파일러).
3. `rust-mirror` witness — 이 **언어-간 임베드가 의미를 보존**하는가에 대한 `.px` 증거
   (direction=rust-projection, source_lang=px, target_lang=rust, out_hash).
4. 오타 pnix는 **read-time에 정직하게 거부**(plain Rust는 &str이라 통과).

## 어디에 쓰나
- Rust 애플리케이션에 **순수 pnix 설정/DSL**을 임베드하되, 그 스니펫을 컴파일-verified
  Rust로 승격하고 싶을 때(설정을 코드로 사영 + 의미보존 게이트).

## 두 언어를 섞는 게 아니다
pnix-rs는 Rust(rs-meta)와 pnix(px.rs)를 **각각 온전히** 구현하고 **서로 interop**시킨다.
이 섹션은 새 하이브리드 언어를 만드는 게 아니라, 호스트 Rust가 손님 pnix를 언어 수준에서
**임베드·승격·검증**하는 interop을 보여준다(형제 08과 같은 기둥).
