# 43. Opaque 호스트 참조 생명주기 — 값이 아니라 객체를 안전하게 넘기기

## 무엇을
`04`/`42`가 **값**과 **함수 호출**을 다룬다면, 이건 **호스트 객체 자체**를
pnix 쪽에 안전하게 노출하는 문제다 — SES(Secure ECMAScript)식 opaque
참조: pnix는 객체를 직접 못 만지고, 공개 메서드만 이름으로 호출할 수 있고,
빌림(lend)에는 스코프가 있고, 필요하면 표면을 얼릴(harden) 수 있다.

- `make_opaque_ref` — host 객체를 opaque 참조로 감싼다(내부 노출 없음)
- `opaque_allowed_methods` — **공개** 메서드만 나열(`_private`는 안 보임)
- `opaque_call_method` — 이름으로 메서드 호출(effect 증거 동반)
- `inspect_opaque` — 객체를 노출하지 않고 타입/시그니처만 조회
- `interop_context`/`lend_opaque`/`release_opaque` — 스코프 있는 빌림 +
  생명주기 집계(`opaque_lifecycle`)
- `harden_opaque`/`declare_opaque_invariants` — 표면 해시 동결 + 불변
  속성 선언

## 왜
plain Python에서 객체를 넘기면 **전권**이다 — 어떤 메서드든, `_private`
속성이든 다 접근 가능하다. "이 메서드만 허용", "빌린 뒤엔 돌려줘야 함",
"이 표면은 이후 안 바뀐다고 약속" 같은 걸 강제할 표준이 없다.

## 무엇을 게이트하나
| 함수 | 확인 |
|---|---|
| `opaque_allowed_methods` | `_secret` 같은 private 메서드는 목록에 안 나옴 |
| `opaque_lifecycle` | `lend_opaque` 스코프 안/밖에서 `lends_active` 카운트 변화 |
| `harden_opaque` | 공개 메서드 표면의 결정적 sha256 |
| `declare_opaque_invariants` | 선언한 속성이 "frozen"으로 기록 |

## 한 줄
> 객체를 넘겨도 `_secret`은 안 보이고, 빌림은 스코프가 있고, 표면은
> 필요하면 얼릴 수 있다 — plain 참조 전달과 다르다.

## 경계
- 함수/메서드 **호출** 자체의 effect 증거는 `42`가 다룬다. 여기는
  **참조(reference)** 자체의 안전한 노출/생명주기.
