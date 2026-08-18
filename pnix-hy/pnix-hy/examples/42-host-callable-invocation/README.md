# 42. 호스트 콜러블 호출 — pnix에서 Python 함수/메서드를 안전하게 부르기

## 무엇을
`04-host-interop-loss-effect`가 **값**의 host↔pnix 변환(loss/effect 기록)을
다룬다면, 이건 **함수/메서드 호출** 자체를 다룬다 — pnix 쪽에서 host의 함수를
capability-gated로 부르고, 모든 호출이 `InteropRecord`(effect-class,
capability, witness) 증거를 남긴다.

- `call_host`/`call_host_method` — 함수/메서드 직접 호출 + 증거 기록
- `try_call_host` — `tryEval` 모양(`{success, value|error}`)의 호출 래퍼
- `apply_host_method` — 메서드를 boundary 너머로 적용
- `host_callable_arity` — 호스트 콜러블 시그니처를 pnix `functionArgs` 모양으로
- `host_callable_to_pnix` — 호스트 함수를 pnix `NativeFunc`로 감싸 pnix 소스가
  builtin처럼 적용
- `host_module_to_pnix` — 호스트 모듈 전체를 pnix attrset으로(공개 속성만)
- `to_host_eval` — pnix 소스를 평가하고 그 결과를 한 번에 host로

## 왜
Python `getattr`/`func(*args)`로 호출은 되지만, "이 호출이 어떤 effect-class
였는지", "capability가 있었는지", "재현 가능한 witness가 남았는지"는 전혀
기록되지 않는다.

## 무엇을 게이트하나
| 함수 | 확인 |
|---|---|
| `call_host`/`call_host_method` | 결과값 + `effect_class="host-call"` 증거 record |
| `try_call_host` | 성공 시 `{success:true, value}`, 예외 시 `{success:false, error:{kind,type,message}}` |
| `host_callable_arity` | 파라미터별 기본값 유무를 pnix `functionArgs` 모양(`{name: bool}`)으로 |
| `host_module_to_pnix` | 모듈 공개 함수 전부를 opaque host-callable attrset으로 |

## 한 줄
> pnix 쪽에서 host 함수를 부르는 모든 경로(직접 호출/메서드/모듈 전체
> 노출/eval-and-cross)가 effect-class와 witness를 남긴다.

## 경계
- 값 변환 자체는 `04`가 다룬다. 여기는 **호출** 경로.
