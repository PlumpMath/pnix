# 19-typed-attestation — 타입 있는 증명서(in-toto/SLSA 스타일)

## 쉽게 말하면 (비유)
`05-witness-and-gate`의 witness는 "무슨 일이 있었는지"를 13필드로 남긴다.
그런데 그 증거가 **무엇을 주장**하는지, 그 주장이 **무엇에 대한** 것인지는
타입이 없으면 헷갈릴 수 있다. attestation은 witness에 "이 claim(predicate)이
이 subject(내용해시)에 대해 성립"이라는 타입을 붙인다 — 공급망 보안의
in-toto/SLSA 표준과 같은 모양이다.

## 무엇을
eval witness를 `pnix-rs/attest/eval-purity.v0` 같은 타입 있는 attestation으로
승격 + 검증. **이빨 있는 음성 사례 2개**: (1) mirror-roundtrip 증명을 eval
witness에 잘못 붙이면(predicate와 실제 evidence 불일치) 거부, (2) subject를
위조(올바른 predicate + 틀린 내용해시)하면 거부. attestation 해시는 결정적.

## plain Rust의 한계 (`limit_rust.rs`)
Rust 표준 도구는 "이 빌드 산출물에 대해 이런 주장이 성립한다"를 타입 있는
증명서로 만들지 않는다. CI 로그는 있어도, predicate/subject가 명시적으로
분리되고 mismatch가 거부되는 검증 가능한 attestation 포맷은 없다.

## pnix-rs의 방식 (`pnix_rs_way.sh`)
- `pnix-rs attest-check` — witness → 타입 있는 attestation 승격/검증 +
  predicate-subject mismatch 거부(이빨) + 해시 결정성

## 어디에 쓰나
공급망 증거(누가/무엇을/왜 믿는지)를 pnix-rs 자체 증거 체계 안에서
표준화된 형태로 재사용. `13-peer-engine`의 engine-attestation과 같은 계열.

## 실행
```sh
rustc -O examples/19-typed-attestation/limit_rust.rs -o /tmp/limit_19-typed-attestation && /tmp/limit_19-typed-attestation
bash examples/19-typed-attestation/pnix_rs_way.sh
```
