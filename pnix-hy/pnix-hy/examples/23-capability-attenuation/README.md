# 23. 능력 감쇠·회수 — 넘겨도 전권이 아니다 (proposal 0020/0016)

## 무엇을
effect-class에 대한 **런타임 회수 가능** 능력 핸들 `CapabilityHandle`:
`grant_capability` → `attenuate`(권한 뺀 자식) → `suspend`/`resume` → `revoke`(영구).

## 왜
plain Python은 객체를 넘기면 **전권**을 넘긴 것 — 받는 쪽이 모든 메서드를 쓸 수 있고, 준 쪽은 최소권한으로
줄이거나 나중에 회수할 수 없다. SES(least authority)는 능력을 **감쇠 가능·회수 가능**한 핸들로 다룬다.

## 쉽게 말하면 (비유)
```
Python 객체 = 마스터키를 통째로 빌려줌 (복사·회수 불가)
CapabilityHandle = 특정 문만 여는 열쇠 발급 + 언제든 무효화 가능
attenuate = 더 적은 문만 여는 열쇠로 다시 발급 (더 많은 문은 못 엶)
```

## 흐름 (`effective()` 로 관찰)
| 동작 | 유효 권한 |
|---|---|
| `grant("read","write","exec")` | {read, write, exec} |
| `.attenuate("exec")` | {read, write} |
| `.attenuate("read")` (자식을 더 감쇠) | {write} |
| `suspend()` / `resume()` | {} → 복구 |
| `revoke()` | {} (영구; 이후 `resume`은 `InteropError`) |

## 한 줄
> 능력을 **핸들**로 넘기면 최소권한으로 감쇠하고, 중단·복구하고, 영구 회수할 수 있다 — 객체를 넘기는
> 것과 달리 권한이 escalate되지 않는다.

## 경계
- effect-class(read/write/exec/host-call…) 수준의 권한 대수. 관련: opaque own/borrow(`lend_opaque`),
  하드닝(`harden_opaque`), blame 있는 `InteropError`. 정본 평가기·4-lane 미러 무접촉.
