# 08-compartment-isolation — 격리(compartment) — 권한 최소화

## 쉽게 말하면 (비유)
한 프로세스에서 신뢰도가 다른 코드를 섞어 돌리면 서로의 상태를 오염시킬 수 있다.
pnix-rs의 compartment는 **자기 env/모듈**을 갖되 intrinsic만 공유하는 SES식 격리를 준다.

## 무엇을
**Compartment**: 자기 env·모듈 소유, builtins만 런타임 폴백으로 공유, lazy materialization.

## plain Rust의 한계 (`limit_rust.rs`)
Rust의 모듈/가시성은 컴파일-타임 캡슐화지, '런타임에 권한을 감쇠·격리한 실행 환경'을
동적으로 만들 표준 수단은 아니다.

## pnix-rs의 방식 (`pnix_rs_way.sh`) — Rust ↔ pnix
- `pnix-rs compartment-check` — 자기 env/모듈, 공유 intrinsic, 격리 확인

## 어디에 쓰나
플러그인/샌드박스, 멀티테넌시, 신뢰 경계 분리.

## 실행
```sh
# plain Rust의 한계 (직접 컴파일·실행)
rustc -O examples/08-compartment-isolation/limit_rust.rs -o /tmp/limit_08-compartment-isolation && /tmp/limit_08-compartment-isolation

# pnix-rs 방식 (flake로 설치했거나 PATH에 pnix-rs가 있으면 그대로,
#              아니면 PNIX_RS=/path/to/pnix-rs 로 지정)
bash examples/08-compartment-isolation/pnix_rs_way.sh
```
