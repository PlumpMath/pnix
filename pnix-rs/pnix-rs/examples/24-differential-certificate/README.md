# 24-differential-certificate — 배터리 차분 동등성 인증서

## 쉽게 말하면 (비유)
`16-jones-optimality`/`18-cogen-second-futamura`는 한두 개 입력으로
residual이 정확함을 보인다. certify는 그걸 **입력 배터리 전체**로
돌려서, "source와 residual이 이 배터리의 모든 입력에서 일치한다"는
**재검토 가능한 인증서**(입력→출력 표의 내용해시)를 발급한다 — 증명이
아니라 **체크 가능한 차분 테스트**다.

## 무엇을
(1) 12-입력 배터리에서 source ≡ residual, 인증서 발급, (2) 인증서
해시는 **결정적**(재검증해도 동일), (3) **이빨**: 일부러 틀린 residual을
넣으면 배터리가 그 불일치를 잡아내 인증서 발급을 거부한다.

## plain Rust의 한계 (`limit_rust.rs`)
Rust 테스트(`#[test]`)는 개별 assertion을 통과/실패로만 보고한다.
"이 배터리 전체에 대한 동등성"을 하나의 **재검토 가능한 내용해시
인증서**로 묶어, 나중에 "이 인증서가 이 배터리에 대해 유효한가"를
독립적으로 재확인할 수 있는 표준 포맷은 없다.

## pnix-rs의 방식 (`pnix_rs_way.sh`)
- `pnix-rs certify-check` — 배터리 동등성 인증서 발급 + 결정성 +
  틀린 residual 거부(이빨)

## 어디에 쓰나
"이 최적화/특화가 이 회귀 배터리에 대해 여전히 유효하다"를 CI에서
재확인 가능한 인증서로 저장·전달.

## 실행
```sh
rustc -O examples/24-differential-certificate/limit_rust.rs -o /tmp/limit_24-differential-certificate && /tmp/limit_24-differential-certificate
bash examples/24-differential-certificate/pnix_rs_way.sh
```
