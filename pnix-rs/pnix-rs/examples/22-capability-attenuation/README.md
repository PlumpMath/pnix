# 22-capability-attenuation — capability 감쇠(비가역적 축소)

## 쉽게 말하면 (비유)
`08-compartment-isolation`이 "격리된 실행 환경"을 준다면, attenuation은
그 환경에 넘겨줄 **권한을 미리 깎는** 조작이다. 한 번 깎은 권한은
되돌려서 다시 넓힐 수 없다 — 자식은 항상 부모의 부분집합이다.

## 무엇을
(1) `file-write`를 제거한 grant는 그 효과를 거부하고 `file-read`는
유지, (2) 감쇠는 **비가역적**(감쇠된 grant로는 제거된 효과를 되찾을
방법이 없음 — 재확대 불가), (3) 회수(revoke, 빈 grant)는 모든 효과를
거부, (4) 연쇄 감쇠(grant를 여러 단계로 계속 깎기)는 각 단계마다 더
좁아짐.

## plain Rust의 한계 (`limit_rust.rs`)
Rust의 타입 시스템은 메모리 안전을 보장하지만, "이 하위 컴포넌트에게
넘길 권한 집합을 명시적으로 깎고, 그 축소가 되돌릴 수 없음을 보장"하는
capability 모델은 표준에 없다 — 소유권/borrow는 권한(effect) 모델이
아니다.

## pnix-rs의 방식 (`pnix_rs_way.sh`)
- `pnix-rs attenuate-check` — 감쇠(제거된 효과 거부) + 비가역성 + 회수
  + 연쇄 감쇠

## 어디에 쓰나
신뢰할 수 없는 하위 모듈/플러그인에 최소 권한만 넘기는 sandboxing
정책(POLA — principle of least authority)의 근거.

## 실행
```sh
rustc -O examples/22-capability-attenuation/limit_rust.rs -o /tmp/limit_22-capability-attenuation && /tmp/limit_22-capability-attenuation
bash examples/22-capability-attenuation/pnix_rs_way.sh
```
